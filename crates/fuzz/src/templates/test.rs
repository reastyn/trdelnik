use trdelnik_client::{Client, Pubkey, Validator};
use trdelnik_fuzz::{FuzzTestBuilder, State};

#[derive(Clone, Debug)]
struct TestState {
    
}

#[trdelnik_fuzz_test]
async fn main() {
    fn initialize_validator() -> Validator {
        let mut validator = Validator::default();
        // validator.add_program("program_name", PROGRAM_ID);
        validator
    }

    FuzzTestBuilder::new()
        .initialize_validator(initialize_validator)
        .with_state(TestState {})
        .start(1, 200)
        .await;
}
