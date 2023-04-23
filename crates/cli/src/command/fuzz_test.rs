mod command;
pub use command::FuzzCommand;
use anyhow::Error;
use fehler::throws;

use trdelnik_fuzz::commander::Commander;

#[throws]
pub async fn fuzz_test(fuzz_test_command: FuzzCommand) {
    let commander = Commander::default();
    match fuzz_test_command {
        FuzzCommand::Run { test_name } => commander.run_fuzz_test(test_name).await?,
        FuzzCommand::New { test_name } => commander.new_fuzz_test(test_name).await?
    }
}
