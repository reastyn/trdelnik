use trdelnik_client::{trdelnik_fuzz, Client, FutureExt, Validator};
use trdelnik_fuzz::{FuzzTestBuilder, State};

#[derive(Clone, Debug)]
struct TestState {}

async fn flow_test(_client: Client, State(_test_state): State<TestState>) {
    // Implement flow
}

#[trdelnik_fuzz]
async fn main() {
    fn initialize_validator() -> Validator {
        let mut validator = Validator::default();
        // validator.add_program("d21", PROGRAM_ID);
        validator
    }

    FuzzTestBuilder::new()
        .initialize_validator(initialize_validator)
        .add_flow(flow_test)
        .with_state(TestState {})
        .start(2, 200)
        .await;
}
