use rosomaxa::prelude::*;
use rosomaxa::utils::*;
use std::sync::Arc;

struct CompositeTimeQuota {
    inner: Arc<dyn Quota + Send + Sync>,
    limit: usize,
    timer: Timer,
}

impl Quota for CompositeTimeQuota {
    fn is_reached(&self) -> bool {
        self.timer.elapsed_millis() > self.limit as u128 || self.inner.is_reached()
    }
}

/// Creates a new environment with extra limit quota. Limit is specified in seconds.
pub fn create_environment_with_custom_quota(
    limit: Option<usize>,
    environment: &Environment<DefaultRandom>,
) -> DefaultEnvironment {
    Environment {
        quota: match (limit, environment.quota.clone()) {
            (Some(limit), None) => Some(Arc::new(TimeQuota::new(limit as f64 / 1000.))),
            (None, Some(quota)) => Some(quota),
            (Some(limit), Some(inner)) => Some(Arc::new(CompositeTimeQuota { inner, limit, timer: Timer::start() })),
            (None, None) => None,
        },
        random: environment.random.clone(),
        parallelism: environment.parallelism.clone(),
        logger: environment.logger.clone(),
        is_experimental: environment.is_experimental,
    }
}
