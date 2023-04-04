use fehler::throws;
use program_client;
use trdelnik_client::{anyhow::Result, *};
// @todo: do not forget to import your program crate (also in the ../Cargo.toml)

// @todo: create and deploy your fixture
#[throws]
#[fixture]
async fn init_fixture() -> Fixture {
    // This method spins up a local validator (like solana-test-validator) and returns a client to it
    let mut validator = Validator::default();
    // @todo: here you can call your add your program
    // validator.add_program("name", PROGRAM_ID);
    let client = validator.start().await;

    let mut fixture = Fixture::new(client);
    fixture.deploy().await?;
    fixture
}

#[trdelnik_test]
async fn test_happy_path(#[future] init_fixture: Result<Fixture>) {
    // @todo: add your happy path test scenario and the other test cases
    let fixture = init_fixture.await?;
    let test_account = keypair(1);
    let test_account_client = fixture.client.clone_with_payer(test_account.clone());
    assert_eq!(test_account_client.payer().pubkey(), test_account.pubkey());
}

// @todo: design and implement all the logic you need for your fixture(s)
struct Fixture {
    client: Client,
    program: Keypair,
    state: Keypair,
}
impl Fixture {
    fn new(client: Client) -> Self {
        Fixture {
            client,
            program: program_keypair(1),
            state: keypair(42),
        }
    }

    #[throws]
    async fn deploy(&mut self) {
        self.client
            .airdrop(self.client.payer().pubkey(), 5_000_000_000)
            .await?;
    }
}
