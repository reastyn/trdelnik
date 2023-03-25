use std::{
    fs,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::{PathBuf},
    sync::{Arc, RwLock},
};

use crossbeam_channel::unbounded;
use rand::Rng;
use solana_core::tower_storage::FileTowerStorage;
use solana_faucet::faucet::{self, run_local_faucet_with_port};
use solana_rpc::rpc::JsonRpcConfig;
use solana_sdk::{
    native_token::sol_to_lamports, signature::Keypair, signer::Signer, system_program,
};
use solana_validator::{admin_rpc_service, test_validator::*};

use crate::{Client, TempClone};

pub struct Validator {
    genesis_validator: TestValidatorGenesis,
    ledger_path: PathBuf,
}

fn request_local_address_rpc() -> SocketAddr {
    // let listener =
    //     TcpListener::bind("127.0.0.1:0").expect("Error when requesting a local address with port");
    // listener
    //     .local_addr()
    //     .expect("Error parsing the assigned address")
    loop {
        let port: u16 = rand::thread_rng().gen_range(1024, 65535 - 1);
        if port_is_available(port) && port_is_available(port + 1) {
            return SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        }
    }
}

fn port_is_available(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
// The port for solana RPC needs to have 2 ports available, one for the RPC and one for the websocket
// They are right next to each other, so we need to check if both are available
fn request_local_address() -> SocketAddr {
    loop {
        let port: u16 = rand::thread_rng().gen_range(1024, 65535 - 1);
        if port_is_available(port) {
            return SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        }
    }
}

fn generate_temp_dir() -> PathBuf {
    let mut rng = rand::thread_rng();
    let ledger_num: u32 = rng.gen();
    let ledger_path = PathBuf::from(format!("../target/tmp/test-ledger-{ledger_num}/"));
    if !ledger_path.exists() {
        fs::create_dir_all(&ledger_path).unwrap_or_else(|err| {
            panic!(
                "Error: Unable to create directory {}: {}",
                ledger_path.display(),
                err
            );
        });
    }
    ledger_path
}

impl Validator {
    pub fn new() -> Self {
        let ledger_path = generate_temp_dir();

        let tower_storage = Arc::new(FileTowerStorage::new(ledger_path.clone()));

        let admin_service_post_init = Arc::new(RwLock::new(None));
        let faucet_lamports = sol_to_lamports(1_000_000.);
        let faucet_keypair = Keypair::new();
        let faucet_pubkey = faucet_keypair.pubkey();

        let faucet_addr = request_local_address();
        println!("Faucet address: {}", faucet_addr);
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
        let rpc_addr = request_local_address_rpc();
        println!("RPC address: {}", rpc_addr);

        solana_logger::setup_with_default("solana_program_runtime=debug");
        let mut genesis = TestValidatorGenesis::default();
        genesis.max_genesis_archive_unpacked_size = Some(u64::MAX);
        genesis.max_ledger_shreds = Some(10_000);

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
            ledger_path,
        }
    }

    pub async fn start(&self) -> Client {
        let (test_validator, payer) = self.genesis_validator.start_async().await;

        let trdelnik_client =
            Client::new_with_test_validator(payer, test_validator, self.ledger_path.clone());
        trdelnik_client
    }
}