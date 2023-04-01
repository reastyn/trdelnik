use anyhow::Error;
use fehler::throws;
use trdelnik_client::*;

pub struct TestOptions {
    root: String,
    options: RunTestOptions,
}

impl TestOptions {
    pub fn new(root: String, options: RunTestOptions) -> Self {
        Self { root, options }
    }
}

#[throws]
pub async fn test(options: TestOptions) {
    let commander = Commander::with_root(options.root);
    commander.run_tests(options.options).await?;
}
