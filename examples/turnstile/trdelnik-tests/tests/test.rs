use fehler::throws;
use program_client::turnstile_instruction;
use trdelnik_client::{anyhow::Result, *};
use turnstile;

#[throws]
#[fixture]
async fn init_fixture() -> Fixture {
    let mut validator = Validator::new();
    validator.add_program("turnstile", turnstile::id());
    let client = validator.start().await;
    // create a test fixture
    let fixture = Fixture {
        client: client,
        state: keypair(42),
    };

    // init instruction call
    turnstile_instruction::initialize(
        &fixture.client,
        fixture.state.pubkey(),
        fixture.client.payer().pubkey(),
        System::id(),
        Some(fixture.state.clone()),
    )
    .await?;

    fixture
}

#[trdelnik_test]
async fn test_happy_path(#[future] init_fixture: Result<Fixture>) {
    let fixture = init_fixture.await?;

    // coin instruction call
    turnstile_instruction::coin(
        &fixture.client,
        "dummy_string".to_owned(),
        fixture.state.pubkey(),
        None,
    )
    .await?;
    // push instruction call
    turnstile_instruction::push(&fixture.client, fixture.state.pubkey(), None).await?;

    // check the test result
    let state = fixture.get_state().await?;

    // after pushing the turnstile should be locked
    assert_eq!(state.locked, true);
    // the last push was successfull
    assert_eq!(state.res, true);
}

#[trdelnik_test]
async fn test_unhappy_path(#[future] init_fixture: Result<Fixture>) {
    let fixture = init_fixture.await?;

    // pushing without prior coin insertion
    turnstile_instruction::push(&fixture.client, fixture.state.pubkey(), None).await?;

    // check the test result
    let state = fixture.get_state().await?;

    // after pushing the turnstile should be locked
    assert_eq!(state.locked, true);
    // the last push was successfull
    assert_eq!(state.res, false);
}

struct Fixture {
    client: Client,
    state: Keypair,
}

impl Fixture {
    #[throws]
    async fn get_state(&self) -> turnstile::State {
        self.client.account_data(self.state.pubkey()).await?
    }
}
