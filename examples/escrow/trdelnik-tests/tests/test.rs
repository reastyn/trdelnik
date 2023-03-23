use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{PathBuf, Path},
    sync::{Arc, RwLock}, fs, io,
};

use anchor_spl::token;
use crossbeam_channel::unbounded;
use escrow;
use fehler::throws;
use program_client::escrow_instruction;
use rstest::fixture;
use solana_core::tower_storage::FileTowerStorage;
use solana_faucet::faucet::{run_local_faucet_with_port, self};
use solana_rpc::rpc::JsonRpcConfig;
use solana_validator::{test_validator::*, admin_rpc_service};
use trdelnik_client::{anyhow::Result, solana_sdk::{system_program, signature::write_keypair_file}, *};

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

#[throws]
#[fixture]
async fn init_fixture() -> Fixture {
    let alice_wallet = keypair(21);
    let payer = keypair(0);

    let program_id = program_keypair(1);
    let payer_pub = payer.pubkey().clone();
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
        write_keypair_file(&Keypair::new(), faucet_keypair_file.to_str().unwrap()).unwrap_or_else(
            |err| {
                println!(
                    "Error: Failed to write {}: {}",
                    faucet_keypair_file.display(),
                    err
                );
                panic!();
            },
        );
    }
    let faucet_pubkey = faucet_keypair.pubkey();

    let faucet_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1447);
    let (sender, receiver) = unbounded();
    run_local_faucet_with_port(faucet_keypair, sender, Some(faucet::TIME_SLICE), None, None, faucet_addr.port());
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
            solana_sdk::account::AccountSharedData::new(faucet_lamports, 0, &system_program::id()),
        );
    genesis.rpc_config(JsonRpcConfig {
        enable_rpc_transaction_history: true,
        enable_extended_tx_metadata_storage: true,
        faucet_addr: Some(faucet_addr),
        ..JsonRpcConfig::default_for_test()
    });

    let (test_validator, payer) = genesis.start_async().await;
    // .add_program("../target/deploy/escrow", program_id.pubkey())
    // .add_account(alice_wallet, account)
    // .start_async()
    // .await;
    // test_validator.get_async_rpc_client().request_airdrop(&alice_wallet.pubkey(), 5_000_000_000).await.unwrap();

    let trdelnik_client = Client::new_with_test_validator(payer, test_validator);

    let mut fixture = Fixture::new(trdelnik_client, program_id, alice_wallet);

    // Deploy
    fixture.deploy().await?;
    // Create a PDA authority
    fixture.pda = Pubkey::find_program_address(&[b"escrow"], &escrow::id()).0;
    // Creation of token mint A
    println!("works");
    fixture
        .client
        .create_token_mint(&fixture.mint_a, fixture.mint_authority.pubkey(), None, 0)
        .await?;
    // Creation of token mint B
    println!("works");
    fixture
        .client
        .create_token_mint(&fixture.mint_b, fixture.mint_authority.pubkey(), None, 0)
        .await?;
    println!("works");
    // Creation of alice's and bob's ATAs for token A
    fixture.alice_token_a_account = fixture
        .client
        .create_associated_token_account(&fixture.alice_wallet, fixture.mint_a.pubkey())
        .await?;
    fixture.bob_token_a_account = fixture
        .client
        .create_associated_token_account(&fixture.bob_wallet, fixture.mint_a.pubkey())
        .await?;
    // Creation of alice's and bob's ATAs for token B
    fixture.alice_token_b_account = fixture
        .client
        .create_associated_token_account(&fixture.alice_wallet, fixture.mint_b.pubkey())
        .await?;
    fixture.bob_token_b_account = fixture
        .client
        .create_associated_token_account(&fixture.bob_wallet, fixture.mint_b.pubkey())
        .await?;

    // Mint some tokens
    fixture
        .client
        .mint_tokens(
            fixture.mint_a.pubkey(),
            &fixture.mint_authority,
            fixture.alice_token_a_account,
            500,
        )
        .await?;
    fixture
        .client
        .mint_tokens(
            fixture.mint_b.pubkey(),
            &fixture.mint_authority,
            fixture.bob_token_b_account,
            1000,
        )
        .await?;

    fixture
}

#[trdelnik_test]
async fn test_happy_path1(#[future] init_fixture: Result<Fixture>) {
    let fixture = init_fixture.await?;
    // Initialize escrow
    escrow_instruction::initialize_escrow(
        &fixture.client,
        500,
        1000,
        fixture.alice_wallet.pubkey(),
        fixture.alice_token_a_account,
        fixture.alice_token_b_account,
        fixture.escrow_account.pubkey(),
        System::id(),
        token::ID,
        [fixture.alice_wallet.clone(), fixture.escrow_account.clone()],
    )
    .await?;

    let escrow = fixture.get_escrow().await?;
    let alice_token_a_account = fixture
        .get_token_account(fixture.alice_token_a_account)
        .await?;

    assert_eq!(alice_token_a_account.owner, fixture.pda);
    assert_eq!(escrow.initializer_key, fixture.alice_wallet.pubkey());
    assert_eq!(escrow.initializer_amount, 500);
    assert_eq!(escrow.taker_amount, 1000);
    assert_eq!(
        escrow.initializer_deposit_token_account,
        fixture.alice_token_a_account
    );
    assert_eq!(
        escrow.initializer_receive_token_account,
        fixture.alice_token_b_account
    );

    // Exchange
    escrow_instruction::exchange(
        &fixture.client,
        fixture.bob_wallet.pubkey(),
        fixture.bob_token_b_account,
        fixture.bob_token_a_account,
        fixture.alice_token_a_account,
        fixture.alice_token_b_account,
        fixture.alice_wallet.pubkey(),
        fixture.escrow_account.pubkey(),
        fixture.pda,
        token::ID,
        [fixture.bob_wallet.clone()],
    )
    .await?;

    let alice_token_a_account = fixture
        .get_token_account(fixture.alice_token_a_account)
        .await?;
    let alice_token_b_account = fixture
        .get_token_account(fixture.alice_token_b_account)
        .await?;
    let bob_token_a_account = fixture
        .get_token_account(fixture.bob_token_a_account)
        .await?;
    let bob_token_b_account = fixture
        .get_token_account(fixture.bob_token_b_account)
        .await?;

    assert_eq!(alice_token_a_account.owner, fixture.alice_wallet.pubkey());
    assert_eq!(bob_token_a_account.amount, 500);
    assert_eq!(alice_token_a_account.amount, 0);
    assert_eq!(alice_token_b_account.amount, 1000);
    assert_eq!(bob_token_b_account.amount, 0);
}

#[trdelnik_test]
async fn test_happy_path2(#[future] init_fixture: Result<Fixture>) {
    let fixture = init_fixture.await?;

    // Initialize escrow
    escrow_instruction::initialize_escrow(
        &fixture.client,
        500,
        1000,
        fixture.alice_wallet.pubkey(),
        fixture.alice_token_a_account,
        fixture.alice_token_b_account,
        fixture.escrow_account.pubkey(),
        System::id(),
        token::ID,
        [fixture.alice_wallet.clone(), fixture.escrow_account.clone()],
    )
    .await?;

    // Cancel
    escrow_instruction::cancel_escrow(
        &fixture.client,
        fixture.alice_wallet.pubkey(),
        fixture.alice_token_a_account,
        fixture.pda,
        fixture.escrow_account.pubkey(),
        token::ID,
        [],
    )
    .await?;

    let alice_token_a_account = fixture
        .get_token_account(fixture.alice_token_a_account)
        .await?;

    assert_eq!(alice_token_a_account.owner, fixture.alice_wallet.pubkey());
    assert_eq!(alice_token_a_account.amount, 500);
}

struct Fixture {
    client: Client,
    program: Keypair,
    // Mint stuff
    mint_a: Keypair,
    mint_b: Keypair,
    mint_authority: Keypair,
    // Escrow
    escrow_account: Keypair,
    // Participants
    alice_wallet: Keypair,
    bob_wallet: Keypair,
    // Token accounts
    alice_token_a_account: Pubkey,
    alice_token_b_account: Pubkey,
    bob_token_a_account: Pubkey,
    bob_token_b_account: Pubkey,
    // PDA authority of escrow
    pda: Pubkey,
}
impl Fixture {
    fn new(client: Client, program: Keypair, alice_wallet: Keypair) -> Self {
        Fixture {
            client,
            program,

            mint_a: keypair(1),
            mint_b: keypair(2),
            mint_authority: system_keypair(1),

            escrow_account: keypair(99),

            alice_wallet,
            bob_wallet: keypair(22),

            alice_token_a_account: Pubkey::default(),
            alice_token_b_account: Pubkey::default(),
            bob_token_a_account: Pubkey::default(),
            bob_token_b_account: Pubkey::default(),

            pda: Pubkey::default(),
        }
    }

    #[throws]
    async fn deploy(&mut self) {
        self.client
            .airdrop(self.alice_wallet.pubkey(), 5_000_000_000)
            .await?;
        self.client
            .deploy_by_name(&self.program.clone(), "escrow")
            .await?;
    }

    #[throws]
    async fn get_escrow(&self) -> escrow::EscrowAccount {
        self.client
            .account_data::<escrow::EscrowAccount>(self.escrow_account.pubkey())
            .await?
    }

    #[throws]
    async fn get_token_account(&self, key: Pubkey) -> token::TokenAccount {
        self.client.account_data::<token::TokenAccount>(key).await?
    }
}
