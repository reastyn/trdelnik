use crate::commander::Error;
use fehler::throws;
use log::debug;
use std::borrow::Cow;

/// `Tester` is used primarily by [`#[trdelnik_test]`](trdelnik_test::trdelnik_test) macro.
///
/// There should be no need to use `Tester` directly.
#[derive(Default)]
pub struct Tester {
    _root: Cow<'static, str>,
}

impl Tester {
    // pub fn new() -> Self {
    //     Self {
    //         root: "../../".into(),
    //     }
    // }

    // pub fn with_root(root: impl Into<Cow<'static, str>>) -> Self {
    //     Self { root: root.into() }
    // }

    #[throws]
    pub async fn before(&mut self) {
        debug!("_____________________");
        debug!("____ BEFORE TEST ____");
        // solana_logger::setup_with_default("solana_program_runtime=debug");
        // let program_id = Pubkey::new_unique();
        // let (test_validator, _payer) = TestValidatorGenesis::default()
        //     .add_program("../target/deploy/escrow", program_id)
        //     .start_async()
        //     .await;
        // println!("test_validator: {:?}", test_validator.rpc_url());
        // let rpc_client = test_validator.get_async_rpc_client();
        // rpc_client
        // let commander = Commander::with_root(mem::take(&mut self.root));
        // commander.start_localnet().await?
    }

    #[throws]
    pub async fn after(&self) {
        debug!("____ AFTER TEST ____");
        // localnet_handle.stop_and_remove_ledger().await?;
        debug!("_____________________");
    }
}
