//! Contains environment specific logic.

use crate::utils::{Random, ThreadPool, Timer};
use std::sync::Arc;

/// A logger type which is called with various information.
pub type InfoLogger = Arc<dyn Fn(&str) + Send + Sync>;

/// Specifies a computational quota for executions. The main purpose is to allow to stop algorithm
/// in reaction to external events such as user cancellation, timer, etc.
pub trait Quota: Send + Sync {
    /// Returns true when computation should be stopped.
    fn is_reached(&self) -> bool;
}

/// Keeps track of environment specific information which influences algorithm behavior.
#[derive(Clone)]
pub struct Environment {
    /// A wrapper on random generator.
    pub random: Random,

    /// A global execution quota.
    pub quota: Option<Arc<dyn Quota + Send + Sync>>,

    /// Keeps data parallelism settings.
    pub parallelism: Parallelism,

    /// An information logger.
    pub logger: InfoLogger,

    /// A boolean flag which signalizes that experimental behavior is allowed.
    pub is_experimental: bool,
}

impl Environment {
    /// Creates an instance of `Environment` using optional time quota and defaults.
    pub fn new_with_time_quota(max_time: Option<usize>) -> Self {
        Self {
            quota: max_time.map::<Arc<dyn Quota + Send + Sync>, _>(|time| Arc::new(TimeQuota::new(time as f64))),
            ..Self::default()
        }
    }

    /// Creates an instance of `Environment`.
    pub fn new(
        random: Random,
        quota: Option<Arc<dyn Quota + Send + Sync>>,
        parallelism: Parallelism,
        logger: InfoLogger,
        is_experimental: bool,
    ) -> Self {
        Self { random, quota, parallelism, logger, is_experimental }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Environment::new(Random::default(), None, Parallelism::default(), Arc::new(|msg| println!("{msg}")), false)
    }
}

/// A time quota.
pub struct TimeQuota {
    start: Timer,
    limit_in_secs: f64,
}

impl TimeQuota {
    /// Creates a new instance of `TimeQuota`.
    pub fn new(limit_in_secs: f64) -> Self {
        Self { start: Timer::start(), limit_in_secs }
    }
}

impl Quota for TimeQuota {
    fn is_reached(&self) -> bool {
        self.start.elapsed_secs_as_f64() > self.limit_in_secs
    }
}

/// Specifies data parallelism settings.
#[derive(Clone)]
pub struct Parallelism {
    available_cpus: usize,
    thread_pools: Option<Arc<Vec<ThreadPool>>>,
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

    /// Creates an instance of `Parallelism` using available cpus as given.
    pub fn new_with_cpus(available_cpus: usize) -> Self {
        Self { available_cpus, ..Self::default() }
    }

    /// Amount of total available CPUs.
    pub fn available_cpus(&self) -> usize {
        self.available_cpus
    }

    /// Executes operation on thread pool with given index. If there is no thread pool with such
    /// index, then executes it without using any of thread pools.
    pub fn thread_pool_execute<OP, R>(&self, idx: usize, op: OP) -> R
    where
        OP: FnOnce() -> R + Send,
        R: Send,
    {
        if let Some(thread_pool) = self.thread_pools.as_ref().and_then(|tps| tps.get(idx % tps.len())) {
            thread_pool.execute(op)
        } else {
            op()
        }
    }

    /// Returns amount of thread pools used. Returns zero if default thread pool is used.
    pub fn thread_pool_size(&self) -> usize {
        self.thread_pools.as_ref().map_or(0, |tp| tp.len())
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
