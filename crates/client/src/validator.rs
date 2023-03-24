use std::{
    fs, io,
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crossbeam_channel::unbounded;
use solana_core::tower_storage::FileTowerStorage;
use solana_faucet::faucet::{self, run_local_faucet_with_port};
use solana_rpc::rpc::JsonRpcConfig;
use solana_sdk::{signer::Signer, system_program};
use solana_validator::{admin_rpc_service, test_validator::*};

use crate::{keypair, Client, TempClone};

pub struct Validator {
    genesis_validator: TestValidatorGenesis,
}

fn remove_directory_contents(ledger_path: &Path) -> Result<(), io::Error> {
    for entry in fs::read_dir(ledger_path)? {
        let entry = entry?;
        if entry.metadata()?.is_dir() {
            fs::remove_dir_all(entry.path())?
        } else {
            fs::remove_file(entry.path())?
        }
    }
    Ok(())
}

fn request_local_address() -> SocketAddr {
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("Error when requesting a local address with port");
    listener
        .local_addr()
        .expect("Error parsing the assigned address")
}

impl Validator {
    pub fn new() -> Self {
        let ledger_path = PathBuf::from("test-ledger");
        remove_directory_contents(&ledger_path).unwrap_or_else(|err| {
            println!("Error: Unable to remove {}: {}", ledger_path.display(), err);
            panic!();
        });
        let tower_storage = Arc::new(FileTowerStorage::new(ledger_path.clone()));

        let admin_service_post_init = Arc::new(RwLock::new(None));
        let faucet_keypair = keypair(7);
        let faucet_lamports = 1_000_000_000_000_000;
        let faucet_pubkey = faucet_keypair.pubkey();

        let faucet_addr = request_local_address();
        let (sender, receiver) = unbounded();
        run_local_faucet_with_port(
            faucet_keypair.clone(),
            sender,
            Some(faucet::TIME_SLICE),
            None,
            None,
            faucet_addr.port(),
        );
        let _ = receiver.recv().expect("run faucet").unwrap_or_else(|err| {
            println!("Error: failed to start faucet: {err}");
            panic!();
        });
        let rpc_addr = request_local_address();

        solana_logger::setup_with_default("solana_program_runtime=debug");
        let mut genesis = TestValidatorGenesis::default();
        genesis.max_genesis_archive_unpacked_size = Some(u64::MAX);
        genesis.max_ledger_shreds = Some(100_000);
        genesis.rpc_port(rpc_addr.port());

        admin_rpc_service::run(
            &ledger_path,
            admin_rpc_service::AdminRpcRequestMetadata {
                rpc_addr: Some(rpc_addr),
                start_progress: genesis.start_progress.clone(),
                start_time: std::time::SystemTime::now(),
                validator_exit: genesis.validator_exit.clone(),
                authorized_voter_keypairs: genesis.authorized_voter_keypairs.clone(),
                staked_nodes_overrides: genesis.staked_nodes_overrides.clone(),
                post_init: admin_service_post_init,
                tower_storage: tower_storage.clone(),
            },
        );

        genesis
            .ledger_path(&ledger_path)
            .tower_storage(tower_storage)
            .rpc_port(rpc_addr.port())
            .add_account(
                faucet_pubkey,
                solana_sdk::account::AccountSharedData::new(
                    faucet_lamports,
                    0,
                    &system_program::id(),
                ),
            );
        genesis.rpc_config(JsonRpcConfig {
            enable_rpc_transaction_history: true,
            enable_extended_tx_metadata_storage: true,
            faucet_addr: Some(faucet_addr),
            ..JsonRpcConfig::default_for_test()
        });

        Validator {
            genesis_validator: genesis,
        }
    }

    pub async fn start(&self) -> Client {
        let (test_validator, payer) = self.genesis_validator.start_async().await;

        let trdelnik_client = Client::new_with_test_validator(payer, test_validator);
        trdelnik_client
    }
}
