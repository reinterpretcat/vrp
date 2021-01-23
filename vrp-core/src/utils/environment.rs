//! Contains environment specific logic.

use crate::utils::{DefaultRandom, Random, ThreadPool};
use std::sync::Arc;

/// Keeps track of environment specific information which influences algorithm behavior.
#[derive(Clone)]
pub struct Environment {
    /// A wrapper on random generator.
    pub random: Arc<dyn Random + Send + Sync>,

    /// Keeps data parallelism settings.
    pub parallelism: Parallelism,
}

impl Environment {
    /// Creates an instance of `Environment`.
    pub fn new(random: Arc<dyn Random + Send + Sync>, parallelism: Parallelism) -> Self {
        Self { random, parallelism }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Environment::new(Arc::new(DefaultRandom::default()), Parallelism::default())
    }
}

/// Specifies data parallelism settings.
#[derive(Clone)]
pub struct Parallelism {
    /// Amount of total available CPUs.
    pub available_cpus: usize,

    /// Available thread pools.
    pub thread_pools: Option<Arc<Vec<ThreadPool>>>,
}

impl Default for Parallelism {
    fn default() -> Self {
        Self { available_cpus: get_cpus(), thread_pools: None }
    }
}

impl Parallelism {
    /// Creates an instance of `Parallelism`.
    pub fn new(num_thread_pools: usize, threads_per_pool: usize) -> Self {
        let thread_pools = (0..num_thread_pools).map(|_| ThreadPool::new(threads_per_pool)).collect();
        Self { available_cpus: get_cpus(), thread_pools: Some(Arc::new(thread_pools)) }
    }

    /// Executes operation on thread pool with given index. If there is no thread pool with such
    /// index, then executes it without using any of thread pools.
    pub fn thread_pool_execute<OP, R>(&self, idx: usize, op: OP) -> R
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        if let Some(thread_pool) = self.thread_pools.as_ref().and_then(|tps| tps.get(idx)) {
            thread_pool.execute(op)
        } else {
            op()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_cpus() -> usize {
    num_cpus::get()
}

#[cfg(target_arch = "wasm32")]
fn get_cpus() -> usize {
    1
}
