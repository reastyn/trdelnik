use std::ops::Deref;

use program_client::turnstile_instruction::{self, PROGRAM_ID};
use trdelnik_client::{
    tokio, trdelnik_fuzz, Client, FutureExt, Id, Keypair, Signer, System, Validator,
};
use trdelnik_fuzz::{FuzzTestBuilder, State};
use turnstile::{accounts, instruction, State as AccountState};

#[derive(Debug)]
struct CloneableKeypair(Keypair);

impl Clone for CloneableKeypair {
    fn clone(&self) -> Self {
        Self(self.0.insecure_clone())
    }
}

impl Deref for CloneableKeypair {
    type Target = Keypair;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug)]
struct TurnstileExpectedState {
    account_state: CloneableKeypair,
    locked: bool,
    res: bool,
}

fn initialize_validator() -> Validator {
    let mut validator = Validator::default();
    validator.add_program("turnstile", PROGRAM_ID);
    validator
}

async fn flow_push(client: Client, State(mut turnstile_exp_state): State<TurnstileExpectedState>) {
    turnstile_instruction::push(
        &client,
        instruction::Push {},
        accounts::UpdateState {
            state: turnstile_exp_state.account_state.pubkey(),
        },
        None,
    )
    .await
    .expect("push failed");
    if turnstile_exp_state.locked {
        turnstile_exp_state.res = false;
    } else {
        turnstile_exp_state.locked = true;
        turnstile_exp_state.res = true;
    }
}

async fn flow_coin(client: Client, State(mut turnstile_exp_state): State<TurnstileExpectedState>) {
    turnstile_instruction::coin(
        &client,
        instruction::Coin {
            dummy_arg: "dummy_string".to_owned(),
        },
        accounts::UpdateState {
            state: turnstile_exp_state.account_state.pubkey(),
        },
        None,
    )
    .await
    .expect("coin failed");
    // Synchronize local state
    turnstile_exp_state.locked = false;
}

async fn init_handler(client: Client, State(turnstile_exp_state): State<TurnstileExpectedState>) {
    // init instruction call
    turnstile_instruction::initialize(
        &client,
        instruction::Initialize {},
        accounts::Initialize {
            state: turnstile_exp_state.account_state.pubkey(),
            user: client.payer().pubkey(),
            system_program: System::id(),
        },
        Some(turnstile_exp_state.account_state.insecure_clone()),
    )
    .await
    .expect("init failed");
}

async fn invariant(client: Client, State(turnstile_exp_state): State<TurnstileExpectedState>) {
    let state: AccountState = client
        .account_data(turnstile_exp_state.account_state.pubkey())
        .await
        .expect("get account data failed");

    // after pushing the turnstile should be locked
    assert_eq!(state.locked, turnstile_exp_state.locked);
    // the last push was successfull
    assert_eq!(state.res, turnstile_exp_state.res);
}

#[trdelnik_fuzz]
async fn main() {
    FuzzTestBuilder::new()
        .initialize_validator(initialize_validator)
        .add_init_handler(init_handler)
        .add_flow(flow_push)
        .add_flow(flow_coin)
        .with_state(TurnstileExpectedState {
            account_state: CloneableKeypair(Keypair::new()),
            locked: true,
            res: false,
        })
        .add_invariant(invariant)
        .start(1, 200)
        .await;
}
