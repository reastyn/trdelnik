use std::env;

use fehler::throw;
use thiserror::Error;
use tokio::process::Command;
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Run this command in the root of the workspace")]
    BadWorkspace,
}

pub struct Commander {}

impl Default for Commander {
    fn default() -> Self {
        Self {}
    }
}

impl Commander {
    pub async fn new_fuzz_test(&self, name: String) -> Result<(), Error> {
        let root_path = env::current_dir().expect("Unable to get current directory");
        let trdelnik_test_folder = root_path.join("trdelnik-tests");
        if !trdelnik_test_folder.exists() {
            throw!(Error::BadWorkspace)
        }
        let fuzz_test_folder = trdelnik_test_folder.join("fuzz-tests");
        if !fuzz_test_folder.exists() {
            fs::create_dir(fuzz_test_folder.clone())
                .await
                .expect("Unable to create fuzz-tests folder");
        }
        let name = if name.ends_with(".rs") {
            name
        } else {
            format!("{}.rs", name)
        };
        let test_path = fuzz_test_folder.join(name.clone());
        if test_path.exists() {
            panic!("Fuzz test with name {} already exists", name);
        }
        let test_content = include_str!("templates/test.rs");
        fs::write(test_path.clone(), &test_content)
            .await
            .expect(format!("Unable to create fuzz test in path {}", test_path.display()).as_str());

        OpenOptions::new()
            .write(true)
            .append(true)
            .open(trdelnik_test_folder.join("Cargo.toml"))
            .await
            .expect("Unable to open Cargo.toml")
            .write_all(
                format!(
                    "
[[bin]]
name = \"{name}\"
path = \"fuzz-tests/{name}\"
test = false
doc = false
            "
                )
                .as_bytes(),
            )
            .await
            .expect("Could not add fuzz test to Cargo.toml");

        return Ok(());
    }

    pub async fn run_fuzz_test(&self, name: String) -> Result<(), Error> {
        let success = Command::new("cargo")
            .current_dir("trdelnik-tests")
            .arg("run")
            .arg("--bin")
            .arg(name)
            .spawn()
            .expect("Unable to run fuzz test")
            .wait()
            .await
            .expect("Unable to start fuzz test")
            .success();
        if !success {
            println!("Fuzzing ended");
        }
        Ok(())
    }
}
