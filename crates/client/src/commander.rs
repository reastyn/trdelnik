use crate::{
    idl::{self, Idl},
    program_client_generator,
};
use cargo_metadata::{MetadataCommand, Package};
use fehler::{throw, throws};
use futures::future::try_join_all;
use log::debug;
use std::{borrow::Cow, io, iter, path::Path, process::Stdio, string::FromUtf8Error};
use thiserror::Error;
use tokio::{fs, io::AsyncWriteExt, process::Command};

pub static PROGRAM_CLIENT_DIRECTORY: &str = ".program_client";

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    Io(#[from] io::Error),
    #[error("{0:?}")]
    Utf8(#[from] FromUtf8Error),
    #[error("build programs failed")]
    BuildProgramsFailed,
    #[error("testing failed")]
    TestingFailed,
    #[error("read program code failed: '{0}'")]
    ReadProgramCodeFailed(String),
    #[error("{0:?}")]
    Idl(#[from] idl::Error),
    #[error("{0:?}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("parsing Cargo.toml dependencies failed")]
    ParsingCargoTomlDependenciesFailed,
}

/// `Commander` allows you to start localnet, build programs,
/// run tests and do other useful operations.
pub struct Commander {
    root: Cow<'static, str>,
}

pub struct RunTestOptions {
    pub nocapture: bool,
    pub nextest: bool,
    pub package: Option<String>,
    pub test_name: Option<String>,
}

impl Commander {
    /// Creates a new `Commander` instance with the default root `"../../"`.
    pub fn new() -> Self {
        Self {
            root: "../../".into(),
        }
    }

    /// Creates a new `Commander` instance with the provided `root`.
    pub fn with_root(root: impl Into<Cow<'static, str>>) -> Self {
        Self { root: root.into() }
    }

    /// Builds programs (smart contracts).
    #[throws]
    pub async fn build_programs(&self) {
        let success = Command::new("cargo")
            .arg("build-bpf")
            .arg("--")
            // prevent prevent dependency loop:
            // program tests -> program_client -> program
            .args(["-Z", "avoid-dev-deps"])
            .spawn()?
            .wait()
            .await?
            .success();
        if !success {
            throw!(Error::BuildProgramsFailed);
        }
    }

    /// Runs standard Rust tests.
    ///
    /// _Note_: The [--nocapture](https://doc.rust-lang.org/cargo/commands/cargo-test.html#display-options) argument is used
    /// to allow you read `println` outputs in your terminal window.
    #[throws]
    pub async fn run_tests(&self, options: RunTestOptions) {
        let mut command = Command::new("cargo");
        if options.nextest {
            command.arg("nextest").arg("run");
        } else {
            command.arg("test");
        }
        command.arg("--package").arg("trdelnik-tests");
        if let Some(package) = options.package {
            command.arg("--package").arg(package);
        }

        command.arg("--");
        if options.nocapture {
            command.arg("--nocapture");
        }
        let success = command.spawn()?.wait().await?.success();
        if !success {
            throw!(Error::TestingFailed);
        }
    }

    /// Creates the `program_client` crate.
    ///
    /// It's used internally by the [`#[trdelnik_test]`](trdelnik_test::trdelnik_test) macro.
    #[throws]
    pub async fn create_program_client_crate(&self) {
        let crate_path = Path::new(self.root.as_ref()).join(PROGRAM_CLIENT_DIRECTORY);
        if fs::metadata(&crate_path).await.is_ok() {
            return;
        }

        // @TODO Would it be better to:
        // zip the template folder -> embed the archive to the binary -> unzip to a given location?

        fs::create_dir(&crate_path).await?;

        let cargo_toml_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/program_client/Cargo.toml.tmpl"
        ));
        fs::write(crate_path.join("Cargo.toml"), &cargo_toml_content).await?;

        let src_path = crate_path.join("src");
        fs::create_dir(&src_path).await?;

        let lib_rs_content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/templates/program_client/lib.rs"
        ));
        fs::write(src_path.join("lib.rs"), &lib_rs_content).await?;

        debug!("program_client crate created")
    }

    /// Returns an [Iterator] of program [Package]s read from `Cargo.toml` files.
    pub fn program_packages(&self) -> impl Iterator<Item = Package> {
        let cargo_toml_data = MetadataCommand::new()
            .no_deps()
            .exec()
            .expect("Cargo.toml reading failed");

        cargo_toml_data.packages.into_iter().filter(|package| {
            // @TODO less error-prone test if the package is a _program_?
            if let Some("programs") = package.manifest_path.iter().nth_back(2) {
                return true;
            }
            false
        })
    }

    /// Updates the `program_client` dependencies.
    ///
    /// It's used internally by the [`#[trdelnik_test]`](trdelnik_test::trdelnik_test) macro.
    #[throws]
    pub async fn generate_program_client_deps(&self) {
        let trdelnik_dep = r#"trdelnik-client = "0.6.0""#.parse().unwrap();
        // @TODO replace the line above with the specific version or commit hash
        // when Trdelnik is released or when its repo is published.
        // Or use both variants - path for Trdelnik repo/dev and version/commit for users.
        // Some related snippets:
        //
        // println!("Trdelnik Version: {}", std::env!("VERGEN_BUILD_SEMVER"));
        // println!("Trdelnik Commit: {}", std::env!("VERGEN_GIT_SHA"));
        // https://docs.rs/vergen/latest/vergen/#environment-variables
        //
        // `trdelnik = "0.1.0"`
        // `trdelnik = { git = "https://github.com/Ackee-Blockchain/trdelnik.git", rev = "cf867aea87e67d7be029982baa39767f426e404d" }`

        let absolute_root = fs::canonicalize(self.root.as_ref()).await?;

        let program_deps = self.program_packages().map(|package| {
            let name = package.name;
            let path = package
                .manifest_path
                .parent()
                .unwrap()
                .strip_prefix(&absolute_root)
                .unwrap();
            format!(r#"{name} = {{ path = "../{path}", features = ["no-entrypoint"] }}"#)
                .parse()
                .unwrap()
        });

        let cargo_toml_path = Path::new(self.root.as_ref())
            .join(PROGRAM_CLIENT_DIRECTORY)
            .join("Cargo.toml");

        let mut cargo_toml_content: toml::Value =
            fs::read_to_string(&cargo_toml_path).await?.parse()?;

        let cargo_toml_deps = cargo_toml_content
            .get_mut("dependencies")
            .and_then(toml::Value::as_table_mut)
            .ok_or(Error::ParsingCargoTomlDependenciesFailed)?;

        for dep in iter::once(trdelnik_dep).chain(program_deps) {
            if let toml::Value::Table(table) = dep {
                let (name, value) = table.into_iter().next().unwrap();
                cargo_toml_deps.entry(name).or_insert(value);
            }
        }

        // @TODO remove renamed or deleted programs from deps?

        fs::write(cargo_toml_path, cargo_toml_content.to_string()).await?;
    }

    /// Updates the `program_client` `lib.rs`.
    ///
    /// It's used internally by the [`#[trdelnik_test]`](trdelnik_test::trdelnik_test) macro.
    #[throws]
    pub async fn generate_program_client_lib_rs(&self) {
        let idl_programs = self.program_packages().map(|package| async move {
            let name = package.name;
            let output = Command::new("cargo")
                .arg("+nightly")
                .arg("rustc")
                .args(["--package", &name])
                .arg("--profile=check")
                .arg("--")
                .arg("-Zunpretty=expanded")
                .output()
                .await?;
            if output.status.success() {
                let code = String::from_utf8(output.stdout)?;
                Ok(idl::parse_to_idl_program(name, &code).await?)
            } else {
                let error_text = String::from_utf8(output.stderr)?;
                Err(Error::ReadProgramCodeFailed(error_text))
            }
        });
        let idl = Idl {
            programs: try_join_all(idl_programs).await?,
        };
        let use_tokens = self.parse_program_client_imports().await?;
        let program_client = program_client_generator::generate_source_code(idl, &use_tokens);
        let program_client = Self::format_program_code(&program_client).await?;

        let rust_file_path = Path::new(self.root.as_ref())
            .join(PROGRAM_CLIENT_DIRECTORY)
            .join("src/lib.rs");
        fs::write(rust_file_path, &program_client).await?;
    }

    /// Formats program code.
    #[throws]
    pub async fn format_program_code(code: &str) -> String {
        let mut rustfmt = Command::new("rustfmt")
            .args(["--edition", "2018"])
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        if let Some(stdio) = &mut rustfmt.stdin {
            stdio.write_all(code.as_bytes()).await?;
        }
        let output = rustfmt.wait_with_output().await?;
        String::from_utf8(output.stdout)?
    }

    /// Returns `use` modules / statements
    /// The goal of this method is to find all `use` statements defined by the user in the `.program_client`
    /// crate. It solves the problem with regenerating the program client and removing imports defined by
    /// the user.
    #[throws]
    pub async fn parse_program_client_imports(&self) -> Vec<syn::ItemUse> {
        let output = Command::new("cargo")
            .arg("+nightly")
            .arg("rustc")
            .args(["--package", "program_client"])
            .arg("--profile=check")
            .arg("--")
            .arg("-Zunpretty=expanded")
            .output()
            .await?;
        let code = String::from_utf8(output.stdout)?;
        let mut use_modules: Vec<syn::ItemUse> = vec![];
        for item in syn::parse_file(code.as_str()).unwrap().items.into_iter() {
            if let syn::Item::Mod(module) = item {
                let modules = module
                    .content
                    .ok_or("account mod: empty content")
                    .unwrap()
                    .1
                    .into_iter();
                for module in modules {
                    if let syn::Item::Use(u) = module {
                        use_modules.push(u);
                    }
                }
            }
        }
        if use_modules.is_empty() {
            use_modules.push(syn::parse_quote! { use trdelnik_client::*; })
        }
        use_modules
    }
}

impl Default for Commander {
    /// Creates a new `Commander` instance with the default root `"../../"`.
    fn default() -> Self {
        Self::new()
    }
}
