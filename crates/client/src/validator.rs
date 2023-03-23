use std::{
    fs, io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    system_program, signer::Signer,
};
use crossbeam_channel::unbounded;
use solana_core::tower_storage::FileTowerStorage;
use solana_faucet::faucet::{self, run_local_faucet_with_port};
use solana_rpc::rpc::JsonRpcConfig;
use solana_validator::{admin_rpc_service, test_validator::*};

use crate::{Client, TempClone, keypair};

pub struct Validator {
    faucet_keypair: Keypair,
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
        let faucet_keypair_file = ledger_path.join("faucet-keypair.json");
        if !faucet_keypair_file.exists() {
            write_keypair_file(&faucet_keypair, faucet_keypair_file.to_str().unwrap())
                .unwrap_or_else(|err| {
                    println!(
                        "Error: Failed to write {}: {}",
                        faucet_keypair_file.display(),
                        err
                    );
                    panic!();
                });
        }
        let faucet_pubkey = faucet_keypair.pubkey();

        let faucet_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1447);
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
            // panic!();
            faucet_addr
        });
        let rpc_port = 1337;

        solana_logger::setup_with_default("solana_program_runtime=debug");
        let mut genesis = TestValidatorGenesis::default();
        genesis.max_genesis_archive_unpacked_size = Some(u64::MAX);
        genesis.max_ledger_shreds = Some(100_000);
        genesis.rpc_port(rpc_port);

        admin_rpc_service::run(
            &ledger_path,
            admin_rpc_service::AdminRpcRequestMetadata {
                rpc_addr: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), rpc_port)),
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
            .rpc_port(rpc_port)
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
            faucet_keypair,
            genesis_validator: genesis,
        }
    }

    pub async fn start(&self) -> Client {
        let (test_validator, payer) = self.genesis_validator.start_async().await;

        let trdelnik_client = Client::new_with_test_validator(payer, test_validator);
        trdelnik_client
    }
}
