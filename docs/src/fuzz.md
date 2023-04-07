# Fuzz testing usage

Fuzz testing is a type of automated testing that involves providing invalid, unexpected, or random input to a software program in order to test its robustness and detect bugs. Trdelnik provides an easy to use interface for fuzz testing your Solana programs.

## Usage

To create a new fuzz test run the 

```shell
trdelnik fuzz new <fuzz_test_name>
```

command. This will create a new fuzz test in the `trdelnik-tests/fuzz-tests` directory. The fuzz test will be named `<fuzz_test_name>.rs`. The fuzz test will be automatically added to the `trdelnik-tests/Cargo.toml` for Rust to be able to execute it as binary.

In the trdelnik-tests also add the fuzz testing library using `cargo add trdelnik-fuzz`.

## Getting started with fuzz tests

For the purpose of explaining the fuzz testing, simple anchor-counter smart contract will be used. 

```rust
#[account]
pub struct Counter {
    pub count: u64,
}

#[program]
pub mod anchor_counter {
    use super::*;

    pub fn increment(ctx: Context<Update>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count += 1;
        msg!("Current count is {}", counter.count);
        Ok(())
    }

    pub fn decrement(ctx: Context<Update>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count -= 1;
        msg!("Current count is {}", counter.count);
        Ok(())
    }
}
```

The smart contract supports only incrementing and decrementing passed account. It saves the current count as a `u64` number. Fuzz testing should be able to find that decrementing the counter below 0 will cause an overflow.

### Starting the fuzz tests

The fuzz tests are just a simple binary rust programs, that have the `#[trdelnik_fuzz]` macro added to the main function. Then the FuzzTestBuilder struct is used to build the fuzz test. The builder allows to add flows, invariants and validators to the fuzz test. The builder also allows to set to set the number of sequences and the number of iterations per sequence.

```rust
#[trdelnik_fuzz]
async fn main() {
    FuzzTestBuilder::new()
        .initialize_validator(initialize_validator)
        .add_flow(flow_increment)
        .add_flow(flow_decrement)
        .with_state(TestState {
            counter_account: CopyableKeypair(Keypair::new()),
            count: 0,
        })
        .add_invariant(invariant_check_counter)
        .start(2, 250)
        .await;
}
```

When fuzz testing the smart contract, there should be some local state that will be just a simple data structure in Rust. The advantage of having such a simple state is that the developer does not need to think about accounts, just about what should be the resulting state after the flow is executed. This local state is then validated against the state on the blockchain.

The fuzz test starts by running the `initialize_validator` function. This function is used to initialize the validator with smart contract and possibly other accounts.

```rust
fn initialize_validator() -> Validator {
    let mut validator = Validator::default();
    validator.add_program("anchor_counter", PROGRAM_ID);
    validator
}
```

The start function ran specifies how many validators will be ran in parallel and how many flows should be executed per validator. As seen on line 12 of the FuzzTestBuilder.

### State

The developer can register his own state structs that will be automatically injected into the flows and invariants. The state gets registered using the `with_state` function. The state struct needs to implement the `Clone` and `Debug` trait.

```rust
#[derive(Clone, Debug)]
struct TestState {
    count: i128,
    counter_account: CopyableKeypair,
}
```

### Flows

Flows are functions mutating the state of the smart contract. The ran flow is randomly chosen from the array of registered flows. Flows are registered using the `add_flow` function. 

State was automatically injected into the flow function. The flow function can also take other arguments, but they need to be registered in the `with_state` function.

Flow functions first mutate the Smart Contract and then synchronize what they did in the local state.


```rust
async fn flow_increment(State(mut state): State<TestState>, client: Client) {
    anchor_counter_instruction::increment(
        &client,
        Increment {},
        Update {
            counter: state.counter_account.0.pubkey(),
            user: client.payer().pubkey(),
            system_program: System::id(),
        },
        vec![client.payer().clone(), state.counter_account.0.clone()],
    )
    .await
    .unwrap();
    state.count += 1;
}
```

This is how the decrement flow looks like.

```rust
async fn flow_decrement(State(mut state): State<TestState>, client: Client) {
    anchor_counter_instruction::decrement(
        &client,
        Decrement {},
        Update {
            counter: state.counter_account.0.pubkey(),
            user: client.payer().pubkey(),
            system_program: System::id(),
        },
        vec![client.payer().clone(), state.counter_account.0.clone()],
    )
    .await
    .unwrap();
    state.count -= 1;
}
```

### Invariants

Invariants are functions that check the state of the smart contract. They are ran after each flow. Invariants are registered using the `add_invariant` function.

Here the invariant just checks that the count in the local state is the same as the count in the smart contract.

```rust
async fn invariant_check_counter(State(state): State<TestState>, client: Client) {
    let counter_account = client
        .get_account(state.counter_account.0.pubkey())
        .await
        .unwrap()
        .unwrap();

    let counter_account = Counter::try_deserialize(&mut counter_account.data()).unwrap();
    assert_eq!(counter_account.count as i128, state.count);
}
```

### Running the fuzz tests

The test can be ran using the `trdelnik fuzz run <fuzz_test_name>` command. The fuzz test will be ran for the specified number of sequences and iterations per sequence. The fuzz test will be ran in a docker container, so the developer does not need to worry about the environment.