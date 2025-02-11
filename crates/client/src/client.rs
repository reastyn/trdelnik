use crate::{config::CONFIG, Reader, TempClone};
use anchor_client::{
    anchor_lang::{
        prelude::System, solana_program::program_pack::Pack, AccountDeserialize, Id,
        InstructionData, ToAccountMetas,
    },
    solana_client::rpc_config::RpcTransactionConfig,
    solana_sdk::{
        account::Account,
        bpf_loader,
        commitment_config::CommitmentConfig,
        instruction::Instruction,
        loader_instruction,
        pubkey::Pubkey,
        signer::{keypair::Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
    Client as AnchorClient, ClientError as Error, Program,
};

use borsh::BorshDeserialize;
use fehler::throws;
use futures::stream::{self, StreamExt};
use log::{debug, error};
use serde::de::DeserializeOwned;
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_cli_output::display::println_transaction;
use solana_client::nonblocking;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use solana_validator::test_validator::TestValidator;
use std::fmt::Debug;
// The deprecated `create_associated_token_account` function is used because of different versions
// of some crates are required in this `client` crate and `anchor-spl` crate
#[allow(deprecated)]
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use std::{fmt::Formatter, mem, path::PathBuf, sync::Arc};
use std::{thread::sleep, time::Duration};
use tokio::task;
use tokio::time;

// @TODO: Make compatible with the latest Anchor deps.
// https://github.com/project-serum/anchor/pull/1307#issuecomment-1022592683

const RETRY_LOCALNET_EVERY_MILLIS: u64 = 500;

type Payer = Arc<Keypair>;

/// `Client` allows you to send typed RPC requests to a Solana cluster.
pub struct Client {
    payer: Keypair,
    anchor_client: AnchorClient<Payer>,
    rpc_client: nonblocking::rpc_client::RpcClient,

    test_validator: Arc<TestValidator>,
    ledger_path: PathBuf,
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("payer", &self.payer.pubkey())
            .field("rpc_url", &self.rpc_client.url())
            .field("ledger_path", &self.ledger_path)
            .finish()
    }
}

impl Client {
    pub fn new(payer: Keypair, test_validator: Arc<TestValidator>, ledger_path: PathBuf) -> Self {
        Self {
            payer: payer.clone(),
            anchor_client: AnchorClient::new_with_options(
                test_validator.rpc_url().as_str().parse().unwrap(),
                Arc::new(payer),
                CommitmentConfig::confirmed(),
            ),
            rpc_client: nonblocking::rpc_client::RpcClient::new_with_commitment(
                test_validator.rpc_url(),
                CommitmentConfig::confirmed(),
            ),
            test_validator,
            ledger_path,
        }
    }

    pub fn clone_with_payer(&self, payer: Keypair) -> Self {
        Client::new(payer, self.test_validator.clone(), self.ledger_path.clone())
    }

    /// Gets client's payer.
    pub fn payer(&self) -> &Keypair {
        &self.payer
    }

    /// Gets the internal Anchor client to call Anchor client's methods directly.
    pub fn anchor_client(&self) -> &AnchorClient<Payer> {
        &self.anchor_client
    }

    /// Creates [Program] instance to communicate with the selected program.
    pub fn program(&self, program_id: Pubkey) -> Program<Payer> {
        self.anchor_client.program(program_id)
    }

    /// Finds out if the Solana localnet is running.
    ///
    /// Set `retry` to `true` when you want to wait for up to 15 seconds until
    /// the localnet is running (until 30 retries with 500ms delays are performed).
    pub async fn is_localnet_running(&self, retry: bool) -> bool {
        let dummy_pubkey = Pubkey::new_from_array([0; 32]);
        let rpc_client = self.anchor_client.program(dummy_pubkey).rpc();
        task::spawn_blocking(move || {
            for _ in 0..(if retry {
                CONFIG.test.validator_startup_timeout / RETRY_LOCALNET_EVERY_MILLIS
            } else {
                1
            }) {
                if rpc_client.get_health().is_ok() {
                    return true;
                }
                if retry {
                    sleep(Duration::from_millis(RETRY_LOCALNET_EVERY_MILLIS));
                }
            }
            false
        })
        .await
        .expect("is_localnet_running task failed")
    }

    /// Gets deserialized data from the chosen account serialized with Anchor
    ///
    /// # Errors
    ///
    /// It fails when:
    /// - the account does not exist.
    /// - the Solana cluster is not running.
    /// - deserialization failed.
    #[throws]
    pub async fn account_data<T>(&self, account: Pubkey) -> T
    where
        T: AccountDeserialize + Send + 'static,
    {
        let res = self.rpc_client.get_account_data(&account).await?;
        T::try_deserialize(&mut &res[..]).unwrap()
        // let cluster = self.test_validator.as_ref().unwrap().rpc_url();
        // task::spawn_blocking(move || {
        //     let dummy_keypair = Keypair::new();
        //     let dummy_program_id = Pubkey::new_from_array([0; 32]);
        //     let program =
        //         Client::new_with_cluster(dummy_keypair, cluster.as_str().parse().unwrap())
        //             .program(dummy_program_id);
        //     program.account::<T>(account)
        // })
        // .await
        // .expect("account_data task failed")?
    }

    /// Gets deserialized data from the chosen account serialized with Bincode
    ///
    /// # Errors
    ///
    /// It fails when:
    /// - the account does not exist.
    /// - the Solana cluster is not running.
    /// - deserialization failed.
    #[throws]
    pub async fn account_data_bincode<T>(&self, account: Pubkey) -> T
    where
        T: DeserializeOwned + Send + 'static,
    {
        let account = self
            .get_account(account)
            .await?
            .ok_or(Error::AccountNotFound)?;

        bincode::deserialize(&account.data)
            .map_err(|_| Error::LogParseError("Bincode deserialization failed".to_string()))?
    }

    /// Gets deserialized data from the chosen account serialized with Borsh
    ///
    /// # Errors
    ///
    /// It fails when:
    /// - the account does not exist.
    /// - the Solana cluster is not running.
    /// - deserialization failed.
    #[throws]
    pub async fn account_data_borsh<T>(&self, account: Pubkey) -> T
    where
        T: BorshDeserialize + Send + 'static,
    {
        let account = self
            .get_account(account)
            .await?
            .ok_or(Error::AccountNotFound)?;

        T::try_from_slice(&account.data)
            .map_err(|_| Error::LogParseError("Bincode deserialization failed".to_string()))?
    }

    /// Returns all information associated with the account of the provided [Pubkey].
    ///
    /// # Errors
    ///
    /// It fails when the Solana cluster is not running.
    #[throws]
    pub async fn get_account(&self, account: Pubkey) -> Option<Account> {
        self.rpc_client
            .get_account_with_commitment(&account, self.rpc_client.commitment())
            .await?
            .value
    }

    /// Sends the Anchor instruction with associated accounts and signers.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use trdelnik_client::*;
    ///
    /// pub async fn initialize(
    ///     client: &Client,
    ///     state: Pubkey,
    ///     user: Pubkey,
    ///     system_program: Pubkey,
    ///     signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    /// ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
    ///     Ok(client
    ///         .send_instruction(
    ///             PROGRAM_ID,
    ///             turnstile::instruction::Initialize {},
    ///             turnstile::accounts::Initialize {
    ///                 state: a_state,
    ///                 user: a_user,
    ///                 system_program: a_system_program,
    ///             },
    ///             signers,
    ///         )
    ///         .await?)
    /// }
    /// ```
    #[throws]
    pub async fn send_instruction(
        &self,
        program: Pubkey,
        instruction: impl InstructionData + Send + 'static,
        accounts: impl ToAccountMetas + Send + 'static,
        signers: impl IntoIterator<Item = Keypair> + Send + 'static,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        let anchor_program = self.program(program);

        let signature = task::spawn_blocking(move || {
            let mut request = anchor_program
                .request()
                .args(instruction)
                .accounts(accounts);
            let signers = signers.into_iter().collect::<Vec<_>>();
            for signer in &signers {
                request = request.signer(signer);
            }
            request.send()
        })
        .await
        .expect("send instruction task failed")?;

        self.rpc_client
            .get_transaction_with_config(
                &signature,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Binary),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: None,
                },
            )
            .await
            .unwrap()
    }

    /// Sends the transaction with associated instructions and signers.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[throws]
    /// pub async fn create_account(
    ///     &self,
    ///     keypair: &Keypair,
    ///     lamports: u64,
    ///     space: u64,
    ///     owner: &Pubkey,
    /// ) -> EncodedConfirmedTransaction {
    ///     self.send_transaction(
    ///         &[system_instruction::create_account(
    ///             &self.payer().pubkey(),
    ///             &keypair.pubkey(),
    ///             lamports,
    ///             space,
    ///             owner,
    ///         )],
    ///         [keypair],
    ///     )
    ///     .await?
    /// }
    /// ```
    #[throws]
    pub async fn send_transaction(
        &self,
        instructions: &[Instruction],
        signers: impl IntoIterator<Item = &Keypair> + Send,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        let mut signers = signers.into_iter().collect::<Vec<_>>();
        signers.push(self.payer());

        let tx = &Transaction::new_signed_with_payer(
            instructions,
            Some(&self.payer.pubkey()),
            &signers,
            self.rpc_client
                .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                .await
                .expect("Error while getting recent blockhash")
                .0,
        );
        debug!("Sending transaction: {:?}", tx);
        let signature = self.rpc_client.send_and_confirm_transaction(tx).await?;

        self.rpc_client
            .get_transaction_with_config(
                &signature,
                RpcTransactionConfig {
                    commitment: Some(CommitmentConfig::confirmed()),
                    encoding: Some(UiTransactionEncoding::JsonParsed),
                    ..RpcTransactionConfig::default()
                },
            ) // })
            .await
            .expect("get transaction task failed")
    }

    /// Airdrops lamports to the chosen account.
    #[throws]
    pub async fn airdrop(&self, address: Pubkey, lamports: u64) {
        let async_client = nonblocking::rpc_client::RpcClient::new_with_commitment(
            self.test_validator.rpc_url(),
            CommitmentConfig::confirmed(),
        );
        async_client
            .request_airdrop(&address, lamports)
            .await
            .unwrap_or_else(|_| panic!("Airdop to address {address} failed"));
        debug!("Airdropped {} lamports to {}", lamports, address);
        while async_client.get_balance(&address).await.unwrap() < lamports {
            time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get balance of an account
    #[throws]
    pub async fn get_balance(&self, address: Pubkey) -> u64 {
        let rpc_client = self.anchor_client.program(System::id()).rpc();
        task::spawn_blocking(move || rpc_client.get_balance(&address))
            .await
            .expect("get_balance task failed")?
    }

    /// Get token balance of an token account
    #[throws]
    pub async fn get_token_balance(&mut self, address: Pubkey) -> UiTokenAmount {
        let rpc_client = self.anchor_client.program(System::id()).rpc();
        task::spawn_blocking(move || rpc_client.get_token_account_balance(&address))
            .await
            .expect("get_token_balance task failed")?
    }

    /// Deploys a program based on it's name.
    /// This function wraps boilerplate code required for the successful deployment of a program,
    /// i.e. SOLs airdrop etc.
    ///
    /// # Arguments
    ///
    /// * `program_keypair` - [Keypair] used for the program
    /// * `program_name` - Name of the program to be deployed
    ///
    /// # Example:
    ///
    /// *Project structure*
    ///
    /// ```text
    /// project/
    /// - programs/
    ///   - awesome_contract/
    ///     - ...
    ///     - Cargo.toml
    ///   - turnstile/
    ///     - ...
    ///     - Cargo.toml
    /// - ...
    /// - Cargo.toml
    /// ```
    ///
    /// *Code*
    ///
    /// ```rust,ignore
    /// client.deploy_program(program_keypair(0), "awesome_contract");
    /// client.deploy_program(program_keypair(1), "turnstile");
    /// ```
    #[throws]
    pub async fn deploy_by_name(&self, program_keypair: &Keypair, program_name: &str) {
        debug!("reading program data");

        let reader = Reader::new();
        let mut program_data = reader
            .program_data(program_name)
            .await
            .expect("reading program data failed");

        debug!("airdropping the minimum balance required to deploy the program");
        // let system_program = self.anchor_client.program(System::id());
        // self.airdrop(system_program.payer(), 5_000_000_000).await?;
        // self.airdrop(program_keypair.pubkey(), 5_000_000_000).await?;

        // TODO: This will fail on devnet where airdrops are limited to 1 SOL
        self.airdrop(self.payer().pubkey(), 5_000_000_000)
            .await
            .expect("airdropping for deployment failed");

        debug!("deploying program");
        self.deploy(program_keypair.clone(), mem::take(&mut program_data))
            .await
            .expect("deploying program failed");
    }

    /// Deploys the program.
    #[throws]
    async fn deploy(&self, program_keypair: Keypair, program_data: Vec<u8>) {
        const PROGRAM_DATA_CHUNK_SIZE: usize = 900;

        let program_pubkey = program_keypair.pubkey();
        let system_program = self.anchor_client.program(System::id());

        let program_data_len = program_data.len();
        debug!("program_data_len: {}", program_data_len);

        debug!("create program account");

        let rpc_client = system_program.rpc();
        let min_balance_for_rent_exemption = task::spawn_blocking(move || {
            rpc_client.get_minimum_balance_for_rent_exemption(program_data_len)
        })
        .await
        .expect("crate program account task failed")?;
        debug!(
            "min_balance_for_rent_exemption: {}",
            min_balance_for_rent_exemption
        );

        let create_account_ix = system_instruction::create_account(
            &system_program.payer(),
            &program_pubkey,
            min_balance_for_rent_exemption,
            program_data_len as u64,
            &bpf_loader::id(),
        );
        {
            let program_keypair = Keypair::from_bytes(&program_keypair.to_bytes()).unwrap();
            let system_program = self.anchor_client.program(System::id());
            task::spawn_blocking(move || {
                system_program
                    .request()
                    .instruction(create_account_ix)
                    .signer(&program_keypair)
                    .send()
            })
            .await
            .expect("create program account task failed")?;
        }

        debug!("write program data");

        let mut offset = 0usize;
        let mut futures = Vec::new();
        for chunk in program_data.chunks(PROGRAM_DATA_CHUNK_SIZE) {
            let program_keypair = Keypair::from_bytes(&program_keypair.to_bytes()).unwrap();
            let loader_write_ix = loader_instruction::write(
                &program_pubkey,
                &bpf_loader::id(),
                offset as u32,
                chunk.to_vec(),
            );
            let system_program = self.program(System::id());

            futures.push(async move {
                task::spawn_blocking(move || {
                    system_program
                        .request()
                        .instruction(loader_write_ix)
                        .signer(&program_keypair)
                        .send()
                })
                .await
                .expect("write program data task failed")
            });
            offset += chunk.len();
        }
        stream::iter(futures)
            .buffer_unordered(100)
            .collect::<Vec<_>>()
            .await;

        debug!("finalize program");

        let loader_finalize_ix = loader_instruction::finalize(&program_pubkey, &bpf_loader::id());
        let system_program = self.program(System::id());
        task::spawn_blocking(move || {
            system_program
                .request()
                .instruction(loader_finalize_ix)
                .signer(&program_keypair)
                .send()
        })
        .await
        .expect("finalize program account task failed")?;

        debug!("program deployed");
    }

    /// Creates accounts.
    #[throws]
    pub async fn create_account(
        &self,
        keypair: &Keypair,
        lamports: u64,
        space: u64,
        owner: &Pubkey,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        self.send_transaction(
            &[system_instruction::create_account(
                &self.payer().pubkey(),
                &keypair.pubkey(),
                lamports,
                space,
                owner,
            )],
            [keypair],
        )
        .await?
    }

    /// Creates rent exempt account.
    #[throws]
    pub async fn create_account_rent_exempt(
        &mut self,
        keypair: &Keypair,
        space: u64,
        owner: &Pubkey,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        let rpc_client = self.anchor_client.program(System::id()).rpc();
        self.send_transaction(
            &[system_instruction::create_account(
                &self.payer().pubkey(),
                &keypair.pubkey(),
                rpc_client.get_minimum_balance_for_rent_exemption(space as usize)?,
                space,
                owner,
            )],
            [keypair],
        )
        .await?
    }

    /// Executes a transaction constructing a token mint.
    #[throws]
    pub async fn create_token_mint(
        &self,
        mint: &Keypair,
        authority: Pubkey,
        freeze_authority: Option<Pubkey>,
        decimals: u8,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        let rpc_client = self.test_validator.get_rpc_client();
        self.send_transaction(
            &[
                system_instruction::create_account(
                    &self.payer().pubkey(),
                    &mint.pubkey(),
                    rpc_client
                        .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)?,
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::ID,
                ),
                spl_token::instruction::initialize_mint(
                    &spl_token::ID,
                    &mint.pubkey(),
                    &authority,
                    freeze_authority.as_ref(),
                    decimals,
                )
                .unwrap(),
            ],
            [mint],
        )
        .await?
    }

    /// Executes a transaction that mints tokens from a mint to an account belonging to that mint.
    #[throws]
    pub async fn mint_tokens(
        &self,
        mint: Pubkey,
        authority: &Keypair,
        account: Pubkey,
        amount: u64,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        self.send_transaction(
            &[spl_token::instruction::mint_to(
                &spl_token::ID,
                &mint,
                &account,
                &authority.pubkey(),
                &[],
                amount,
            )
            .unwrap()],
            [authority],
        )
        .await?
    }

    /// Executes a transaction constructing a token account of the specified mint. The account needs to be empty and belong to system for this to work.
    /// Prefer to use [create_associated_token_account] if you don't need the provided account to contain the token account.
    #[throws]
    pub async fn create_token_account(
        &self,
        account: &Keypair,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        let rpc_client = self.anchor_client.program(System::id()).rpc();
        self.send_transaction(
            &[
                system_instruction::create_account(
                    &self.payer().pubkey(),
                    &account.pubkey(),
                    rpc_client
                        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?,
                    spl_token::state::Account::LEN as u64,
                    &spl_token::ID,
                ),
                spl_token::instruction::initialize_account(
                    &spl_token::ID,
                    &account.pubkey(),
                    mint,
                    owner,
                )
                .unwrap(),
            ],
            [account],
        )
        .await?
    }

    /// Executes a transaction constructing the associated token account of the specified mint belonging to the owner. This will fail if the account already exists.
    #[throws]
    pub async fn create_associated_token_account(&self, owner: &Keypair, mint: Pubkey) -> Pubkey {
        self.send_transaction(
            #[allow(deprecated)]
            &[create_associated_token_account(
                &self.payer().pubkey(),
                &owner.pubkey(),
                &mint,
            )],
            &[],
        )
        .await?;
        get_associated_token_address(&owner.pubkey(), &mint)
    }

    /// Executes a transaction creating and filling the given account with the given data.
    /// The account is required to be empty and will be owned by bpf_loader afterwards.
    #[throws]
    pub async fn create_account_with_data(&self, account: &Keypair, data: Vec<u8>) {
        const DATA_CHUNK_SIZE: usize = 900;

        let rpc_client = self.anchor_client.program(System::id()).rpc();
        self.send_transaction(
            &[system_instruction::create_account(
                &self.payer().pubkey(),
                &account.pubkey(),
                rpc_client.get_minimum_balance_for_rent_exemption(data.len())?,
                data.len() as u64,
                &bpf_loader::id(),
            )],
            [account],
        )
        .await?;

        let mut offset = 0usize;
        for chunk in data.chunks(DATA_CHUNK_SIZE) {
            debug!("writing bytes {} to {}", offset, offset + chunk.len());
            self.send_transaction(
                &[loader_instruction::write(
                    &account.pubkey(),
                    &bpf_loader::id(),
                    offset as u32,
                    chunk.to_vec(),
                )],
                [account],
            )
            .await?;
            offset += chunk.len();
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.ledger_path.exists().then(|| {
            std::fs::remove_dir_all(&self.ledger_path).unwrap_or_else(|err| {
                error!(
                    "Error removing validator ledger {}: {}",
                    self.ledger_path.display(),
                    err
                )
            });
        });
    }
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Client::new(
            self.payer().clone(),
            self.test_validator.clone(),
            self.ledger_path.clone(),
        )
    }
}

/// Utility trait for printing transaction results.
pub trait PrintableTransaction {
    /// Pretty print the transaction results, tagged with the given name for distinguishability.
    fn print_named(&self, name: &str);

    /// Pretty print the transaction results.
    fn print(&self) {
        self.print_named("");
    }
}

impl PrintableTransaction for EncodedConfirmedTransactionWithStatusMeta {
    fn print_named(&self, name: &str) {
        let tx = self.transaction.transaction.decode().unwrap();
        debug!("EXECUTE {} (slot {})", name, self.slot);
        match self.transaction.meta.clone() {
            Some(meta) => println_transaction(&tx, Some(&meta), "  ", None, None),
            _ => println_transaction(&tx, None, "  ", None, None),
        }
    }
}
