//! Contains environment specific logic.

use crate::utils::{DefaultRandom, Float, Random, Timer};
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
    pub random: Arc<dyn Random>,

    /// A global execution quota.
    pub quota: Option<Arc<dyn Quota>>,

    /// Describes CPU resources available to the algorithm.
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
            quota: max_time.map::<Arc<dyn Quota>, _>(|time| Arc::new(TimeQuota::new(time as Float))),
            ..Self::default()
        }
    }

    /// Creates an instance of `Environment`.
    pub fn new(
        random: Arc<dyn Random>,
        quota: Option<Arc<dyn Quota>>,
        parallelism: Parallelism,
        logger: InfoLogger,
        is_experimental: bool,
    ) -> Self {
        Self { random, quota, parallelism, logger, is_experimental }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Environment::new(
            Arc::new(DefaultRandom::default()),
            None,
            Parallelism::default(),
            Arc::new(|msg| println!("{msg}")),
            false,
        )
    }
}

/// A time quota.
pub struct TimeQuota {
    start: Timer,
    limit_in_secs: Float,
}

impl TimeQuota {
    /// Creates a new instance of `TimeQuota`.
    pub fn new(limit_in_secs: Float) -> Self {
        Self { start: Timer::start(), limit_in_secs }
    }
}

impl Quota for TimeQuota {
    fn is_reached(&self) -> bool {
        self.start.elapsed_secs_as_float() > self.limit_in_secs
    }
}

/// Describes CPU resources available to the algorithm.
#[derive(Clone)]
pub struct Parallelism {
    available_cpus: usize,
}

impl Default for Parallelism {
    fn default() -> Self {
        Self { available_cpus: get_cpus() }
    }
}

impl Parallelism {
    /// Creates an instance of `Parallelism` using the given number of available CPUs.
    pub fn new_with_cpus(available_cpus: usize) -> Self {
        Self { available_cpus }
    }

    /// Amount of total available CPUs.
    pub fn available_cpus(&self) -> usize {
        self.available_cpus
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
