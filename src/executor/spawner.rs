use std::{future::Future, pin::Pin};

use crate::executor::Executor;

pub trait ExecutorSpawner {
    fn spawn_executor(&self) -> Pin<Box<dyn Future<Output = Executor> + Send>>;

    fn terminate_executors(&self) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}
