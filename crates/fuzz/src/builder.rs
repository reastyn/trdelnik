use anymap::{AnyMap, CloneAny, Map};
use rand::seq::SliceRandom;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
    panic,
    pin::Pin,
    sync::Arc,
};
use tokio::{
    runtime::Handle,
    sync::{Mutex, OwnedMutexGuard},
    task,
};
use trdelnik_client::*;

type MyBoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type SimpleHandler = Box<dyn Fn(OwnedMutexGuard<PassableState>) -> MyBoxFuture<()>>;

// type AnyState = HashMap<TypeId, Box<dyn Any + Send>>;

type CreateValidatorHandler = fn() -> Validator;

pub struct FuzzTestBuilder {
    flows: Vec<SimpleHandler>,
    invariants: Vec<SimpleHandler>,
    started: bool,
    validator_create_handler: Option<CreateValidatorHandler>,
    passable_state: PassableState,
}

pub struct PassableState {
    state: Map<dyn CloneAny + Send + Sync>,
    client: Option<Client>,
}

impl Clone for PassableState {
    fn clone(&self) -> Self {
        PassableState {
            state: self.state.clone(),
            client: self.client.clone(),
        }
    }
}

impl PassableState {
    fn client(&self) -> Client {
        self.client
            .as_ref()
            .expect("You probably forgot to call the `start` method before accessing the client.")
            .clone()
    }
}

impl FuzzTestBuilder {
    pub fn new() -> Self {
        let default_panic = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            println!("Fuzzing ended");
            println!("{}", info);
            default_panic(info);
        }));
        FuzzTestBuilder {
            started: false,
            flows: vec![],
            invariants: vec![],
            validator_create_handler: None,
            passable_state: PassableState {
                state: Map::<dyn CloneAny + Send + Sync>::new(),
                client: None,
            },
        }
    }

    pub fn add_flow<F, Args>(&mut self, flow: F) -> &mut Self
    where
        F: Handler<Args> + 'static,
    {
        if self.started {
            panic!("You cannot add flows after the `start` method was called.");
        }
        let boxed_flow: SimpleHandler =
            Box::new(move |passable_state: OwnedMutexGuard<PassableState>| {
                let f = flow.clone();
                Box::pin(async move {
                    f.call(passable_state).await;
                })
            });
        self.flows.push(boxed_flow);
        self
    }

    pub fn add_invariant<F, Args>(&mut self, invariant: F) -> &mut Self
    where
        F: Handler<Args> + 'static,
    {
        if self.started {
            panic!("You cannot add invariants after the `start` method was called.");
        }
        let boxed_invariant: SimpleHandler =
            Box::new(move |passable_state: OwnedMutexGuard<PassableState>| {
                let f = invariant.clone();
                Box::pin(async move {
                    f.call(passable_state).await;
                })
            });
        self.invariants.push(boxed_invariant);
        self
    }

    pub fn initialize_validator(&mut self, create_handler: CreateValidatorHandler) -> &mut Self {
        self.validator_create_handler = Some(create_handler);
        self
    }

    pub fn with_state<S: Send + Clone + 'static>(&mut self, state: S) -> &mut Self {
        if self.started {
            panic!("You cannot add state after the `start` method was called.");
        }
        // Just because the API of the builder would not be that nice when
        // you would need to await every call of `with_state` method and it is only
        // during the initialization of the builder.
        self.passable_state
            .state
            .insert(Arc::new(Mutex::new(state)));

        self
    }

    // async fn start_validator(&mut self) {
    //     let client = self.validator.start().await;
    //     let mut passable_state = self.passable_state.lock().await;
    //     passable_state.client = Some(client.clone());
    // }

    async fn run_rand_sequence(&self, passable_state: Arc<Mutex<PassableState>>) {
        {
            let owned_mg_passable_state = passable_state.clone().lock_owned().await;
            let flow = self
                .flows
                .choose(&mut rand::thread_rng())
                .expect("There are no flows to run, add them using the `add_flow` method.");
            println!("Started flow");
            flow(owned_mg_passable_state).await;
            println!("Stopped flow");
        }

        println!("Checking invariants...");
        for invariant in self.invariants.iter() {
            let owned_mg_passable_state = passable_state.clone().lock_owned().await;
            println!("Started invariant");
            invariant(owned_mg_passable_state).await;
            println!("Stopped invariant");
        }
    }

    pub async fn start(&mut self, n_seq: usize, n_flows: usize) {
        self.started = true;
        if self.validator_create_handler.is_none() {
            panic!("You need to specify the creator of the validator using the `initialize_validator` method.");
        }
        for i in 0..n_seq {
            println!("Running sequence {}/{}", i + 1, n_seq);
            let mut passable_state_new = self.passable_state.clone();

            let mut validator = self.validator_create_handler.unwrap()();
            passable_state_new.client = Some(validator.start().await);

            let thread_safe_passed_state = Arc::new(Mutex::new(passable_state_new));

            for _ in 0..n_flows {
                self.run_rand_sequence(thread_safe_passed_state.clone()).await;
            }
        }
    }
}

pub trait Handler<T>: Clone + Send + Sized + 'static {
    type Future: Future<Output = ()> + Send + 'static;

    fn call(self, builder: OwnedMutexGuard<PassableState>) -> Self::Future;
}

trait FromPassable {
    fn from_passable(builder: &OwnedMutexGuard<PassableState>) -> Self;
}
pub struct State<T: 'static + Send>(pub OwnedMutexGuard<T>);

impl<T: 'static + Send> FromPassable for State<T> {
    fn from_passable(builder: &OwnedMutexGuard<PassableState>) -> State<T> {
        let state = builder.state.get::<Arc<Mutex<T>>>().unwrap();
        let owned_lock = task::block_in_place(move || {
            Handle::current().block_on(async move { state.clone().lock_owned().await })
        });

        State(owned_lock)
    }
}

impl<F, A, Fut> Handler<A> for F
where
    F: FnOnce(A) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    A: FromPassable,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn call(self, fuzz_test_builder: OwnedMutexGuard<PassableState>) -> Self::Future {
        let a = A::from_passable(&fuzz_test_builder);
        (self)(a).boxed()
    }
}

impl<F, A, B, Fut> Handler<(A, B)> for F
where
    F: FnOnce(A, B) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    A: FromPassable,
    B: FromPassable,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn call(self, fuzz_test_builder: OwnedMutexGuard<PassableState>) -> Self::Future {
        let a = A::from_passable(&fuzz_test_builder);
        let b = B::from_passable(&fuzz_test_builder);
        (self)(a, b).boxed()
    }
}

impl FromPassable for Client {
    fn from_passable(builder: &OwnedMutexGuard<PassableState>) -> Self {
        builder.client()
    }
}
