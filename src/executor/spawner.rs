use crate::executor::Executor;

pub trait ExecutorSpawner {
    fn spawn_executor(&self) -> Executor;

    fn terminate_executor(&self, executor: Executor) {
        drop(executor);
    }
}
