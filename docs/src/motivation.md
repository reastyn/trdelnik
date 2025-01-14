# Motivation

The usual way of testing Anchor programs in the past was to use the testing framework baked into Anchor. Anchor utilizes JavaScript (TypeScript) testing library [Mocha](https://mochajs.org/) to test programs.

The tests are run on solana-test-validator, which is a Solana node running. This is a great way to test programs, but it has some drawbacks:

- All recommendations for writing tests say that they should be independent on each other. This is not possible with the current testing framework, because the state of the program is shared between tests.
- Tests are run sequentially, which is not optimal for larger programs with many tests as they can take a long time to run.
- The tests are written in JavaScript, which can be tedious to debug and write.

With that said the testing framework some really cool features baked in, such as the generated types and methods for interacting with the program. Therefore it is really easy to write tests for programs.

## How does Trdelnik solves those problems?

We wanted to provide similar fast and easy way of testing programs, but with the ability to run tests in parallel and with the ability to write them in Rust.

For each test we create a local validator and deploy the program to it. This way we can run tests in parallel and they are independent on each other. We also provide a console to interact with the program, which is really useful for debugging.

```rust,noplayground
use trdelnik_client::{anyhow::Result, *};

#[trdelnik_test]
async fn test_happy_path1() {
    let mut validator = Validator::default();

    validator.add_program("escrow", program_id.pubkey());
    let trdelnik_client = validator.start().await;
    trdelnik_client
        .create_token_mint(&keypair(1), system_keypair(1).pubkey(), None, 0)
        .await?;
}
```

Trdelnik also generates a instruction helper methods for each instruction in the program. This way you can call similarly to IDL generated methods for TypeScript. The methods are in the `.program_client` module. This simplyfies the code and makes it easier to write tests.

```rust,noplayground
use trdelnik_client::{anyhow::Result, *};
use program_client::escrow_instruction;
use escrow;


#[trdelnik_test]
async fn test_happy_path1() {
    let mut validator = Validator::default();
    validator.add_program("escrow", &escrow::id());
    let trdelnik_client = validator.start().await;
    let mut fixture = Fixture::new(trdelnik_client);

    // This is a helper method generated by Trdelnik
    escrow_instruction::initialize_escrow(
        &trdelnik_client,
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
}
```

