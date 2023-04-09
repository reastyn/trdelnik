use anymap::{CloneAny, Map};
use futures::FutureExt;
use rand::seq::SliceRandom;
use std::fmt::Debug;
use std::process;
use std::{future::Future, panic, pin::Pin, sync::Arc};
use tokio::{
    runtime::Handle,
    sync::{Mutex, OwnedMutexGuard, RwLock},
    task,
};
use tracing::{debug, error, info, instrument, Level};
use tracing_subscriber::{
    filter::filter_fn,
    fmt::format,
    layer::{Layer, SubscriberExt},
    prelude::*,
};
use trdelnik_client::futures::future::select_all;
use trdelnik_client::{futures::future::join_all, *};

use crate::writer::MemoryWriter;

type MyBoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type SimpleHandler = Box<dyn Fn(OwnedMutexGuard<PassableState>) -> MyBoxFuture<()> + Send + Sync>;

type CreateValidatorHandler = fn() -> Validator;

pub struct FuzzTestBuilder {
    flows: Arc<RwLock<Vec<SimpleHandler>>>,
    invariants: Arc<RwLock<Vec<SimpleHandler>>>,
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

struct CustomArcMutex<T: Clone>(Arc<Mutex<T>>);

impl<T: Clone> CustomArcMutex<T> {
    fn clone_arc(&self) -> Arc<Mutex<T>> {
        self.0.clone()
    }

    fn new(t: T) -> Self {
        Self(Arc::new(Mutex::new(t)))
    }
}

impl<T: Clone> Clone for CustomArcMutex<T> {
    fn clone(&self) -> Self {
        let test = &self.0;
        // Unfortunately this version of tokio does not support blocking locks in async runtime.
        let lock = task::block_in_place(move || {
            Handle::current().block_on(async move { test.clone().lock_owned().await })
        });

        CustomArcMutex(Arc::new(Mutex::new(lock.clone())))
    }
}

impl FuzzTestBuilder {
    pub fn new() -> Self {
        FuzzTestBuilder {
            started: false,
            flows: Arc::new(RwLock::new(vec![])),
            invariants: Arc::new(RwLock::new(vec![])),
            validator_create_handler: None,
            passable_state: PassableState {
                state: Map::<dyn CloneAny + Send + Sync>::new(),
                client: None,
            },
        }
    }

    fn add_handler<F, Args>(
        &mut self,
        array: Arc<RwLock<Vec<SimpleHandler>>>,
        handler: F,
    ) -> &mut Self
    where
        F: Handler<Args> + 'static + Sync + Send,
    {
        let boxed_invariant: SimpleHandler =
            Box::new(move |passable_state: OwnedMutexGuard<PassableState>| {
                let f = handler.clone();
                Box::pin(async move {
                    f.call(passable_state).await;
                })
            });
        {
            task::block_in_place(move || {
                Handle::current().block_on(async move {
                    let mut locked_invariants = array.write().await;
                    locked_invariants.push(boxed_invariant);
                })
            });
        }
        self
    }

    pub fn add_flow<F, Args>(&mut self, flow: F) -> &mut Self
    where
        F: Handler<Args> + 'static + Sync + Send,
    {
        if self.started {
            panic!("You cannot add flows after the `start` method was called.");
        }
        self.add_handler(self.flows.clone(), flow);
        self
    }

    pub fn add_invariant<F, Args>(&mut self, invariant: F) -> &mut Self
    where
        F: Handler<Args> + 'static + Sync + Send,
    {
        if self.started {
            panic!("You cannot add invariants after the `start` method was called.");
        }
        self.add_handler(self.invariants.clone(), invariant);
        self
    }

    pub fn initialize_validator(&mut self, create_handler: CreateValidatorHandler) -> &mut Self {
        self.validator_create_handler = Some(create_handler);
        self
    }

    pub fn with_state<S: Send + Sync + Clone + 'static>(&mut self, state: S) -> &mut Self {
        if self.started {
            panic!("You cannot add state after the `start` method was called.");
        }

        self.passable_state.state.insert(CustomArcMutex::new(state));
        self
    }

    // async fn start_validator(&mut self) {
    //     let client = self.validator.start().await;
    //     let mut passable_state = self.passable_state.lock().await;
    //     passable_state.client = Some(client.clone());
    // }

    async fn run_rand_flow(
        passable_state: Arc<Mutex<PassableState>>,
        flows: Arc<RwLock<Vec<SimpleHandler>>>,
        invariants: Arc<RwLock<Vec<SimpleHandler>>>,
    ) {
        {
            let owned_mg_passable_state = passable_state.clone().lock_owned().await;
            let read_flows = flows.read().await;
            let flow = read_flows
                .choose(&mut rand::thread_rng())
                .expect("There are no flows to run, add them using the `add_flow` method.");
            debug!("Started flow");
            flow(owned_mg_passable_state).await;
            debug!("Stopped flow");
        }

        debug!("Checking invariants...");
        let invariants = invariants.read().await;
        for invariant in invariants.iter() {
            let owned_mg_passable_state = passable_state.clone().lock_owned().await;
            invariant(owned_mg_passable_state).await;
        }
        debug!("Invariants checked");
    }

    #[instrument(
        name = "Sequence::started",
        skip(thread_safe_passed_state, flows, invariants, n_flows, _curr_seq_n)
        fields(curr_sequence_number = %_curr_seq_n)
    )]
    async fn run_sequence(
        _curr_seq_n: usize,
        n_flows: usize,
        thread_safe_passed_state: Arc<Mutex<PassableState>>,
        flows: Arc<RwLock<Vec<SimpleHandler>>>,
        invariants: Arc<RwLock<Vec<SimpleHandler>>>,
    ) {
        for i in 0..n_flows {
            debug!("Running flow {}/{}", i + 1, n_flows);
            Self::run_rand_flow(
                thread_safe_passed_state.clone(),
                flows.clone(),
                invariants.clone(),
            )
            .await;
        }
    }

    pub async fn start(&mut self, n_seq: usize, n_flows: usize) {
        self.started = true;
        if self.validator_create_handler.is_none() {
            panic!("You need to specify the creator of the validator using the `initialize_validator` method.");
        }

        let writer = MemoryWriter::new();

        let mut futures = vec![];

        let format = format()
            .pretty()
            .with_thread_ids(true)
            .with_thread_names(true);
        let layer = tracing_subscriber::fmt::layer()
            .event_format(format)
            .with_writer(writer.clone())
            .with_filter(filter_fn(|metadata| {
                metadata.target() == "trdelnik_fuzz::builder"
            }));

        tracing_subscriber::registry().with(layer).init();

        // panic::set_hook(Box::new(move |info| {
        //     memory_writer.clone().print();
        //     println!("Fuzzing ended: {}", info);
        //     process::exit(1);
        // }));

        for i in 0..n_seq {
            debug!("Running sequence {}/{}", i + 1, n_seq);
            let mut passable_state_new = self.passable_state.clone();
            // println!("Passing state to validator: {:?}", passable_state_new.state);

            let mut validator = self.validator_create_handler.unwrap()();
            passable_state_new.client = Some(validator.start().await);

            let thread_safe_passed_state = Arc::new(Mutex::new(passable_state_new));
            let flows = self.flows.clone();
            let invariants = self.invariants.clone();
            let future = tokio::spawn(async move {
                Self::run_sequence(i, n_flows, thread_safe_passed_state, flows, invariants).await;
            });
            futures.push(future);
        }
        match select_all(futures).await {
            (Ok(_), _, _) => {}
            (Err(e), index, _) => {
                writer.print(index);
                panic!("Fuzzing ended: {}", e);
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

#[derive(Debug)]
pub struct State<T: 'static + Send + CloneAny + Sync + Clone + Debug>(pub OwnedMutexGuard<T>);

impl<T: 'static + Send + CloneAny + Sync + Clone + Debug> FromPassable for State<T> {
    fn from_passable(builder: &OwnedMutexGuard<PassableState>) -> State<T> {
        let state = builder.state.get::<CustomArcMutex<T>>().unwrap();

        let owned_lock = task::block_in_place(move || {
            Handle::current().block_on(async move { state.clone_arc().lock_owned().await })
        });

        State(owned_lock)
    }
}

impl<F, A, Fut> Handler<A> for F
where
    F: FnOnce(A) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    A: FromPassable + Debug,
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
    A: FromPassable + Debug,
    B: FromPassable + Debug,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn call(self, fuzz_test_builder: OwnedMutexGuard<PassableState>) -> Self::Future {
        let fn_name = std::any::type_name::<F>();
        let a = A::from_passable(&fuzz_test_builder);
        let b = B::from_passable(&fuzz_test_builder);
        debug!("Calling {fn_name}(\n\t{a:?},\n\t{b:?}\n)");
        (self)(a, b).boxed()
    }
}

impl FromPassable for Client {
    fn from_passable(builder: &OwnedMutexGuard<PassableState>) -> Self {
        builder.client()
    }
}
