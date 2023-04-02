use anyhow::Error;
use clap::{Parser, Subcommand};
use fehler::throws;

// subcommand functions to call and nested subcommands
mod command;
// bring nested subcommand enums into scope
use command::ExplorerCommand;
use command::KeyPairCommand;
use trdelnik_client::RunTestOptions;

#[derive(Parser)]
#[clap(version, propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a `program_client` crate
    Build {
        /// Anchor project root
        #[clap(short, long, default_value = "./")]
        root: String,
    },
    /// Get information about a keypair
    KeyPair {
        #[clap(subcommand)]
        subcmd: KeyPairCommand,
    },
    /// Run program tests
    Test {
        /// Anchor project root
        #[clap(short, long, default_value = "./")]
        root: String,

        #[clap(long)]
        nocapture: bool,

        #[clap(long)]
        package: Option<String>,

        #[clap(short, long)]
        nextest: bool,

        #[clap()]
        test_name: Option<String>,
    },
    /// The Hacker's Explorer
    Explorer {
        #[clap(subcommand)]
        subcmd: ExplorerCommand,
    },
    /// Initialize test environment
    Init,
}

#[throws]
pub async fn start() {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { root } => command::build(root).await?,
        Command::KeyPair { subcmd } => command::keypair(subcmd)?,
        Command::Test {
            root,
            nocapture,
            package,
            nextest,
            test_name,
        } => {
            command::test(command::TestOptions::new(
                root,
                RunTestOptions {
                    nocapture,
                    package,
                    test_name,
                    nextest,
                },
            ))
            .await?
        }
        Command::Explorer { subcmd } => command::explorer(subcmd).await?,
        Command::Init => command::init().await?,
    }
}
