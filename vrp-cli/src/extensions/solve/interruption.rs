//! Interruption handler.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vrp_core::utils::*;

/// Creates interruption quota.
pub fn create_interruption_quota(max_time: Option<usize>) -> Arc<dyn Quota + Send + Sync> {
    let inner = max_time.map::<Arc<dyn Quota + Send + Sync>, _>(|time| Arc::new(TimeQuota::new(time as f64)));
    let should_interrupt = Arc::new(AtomicBool::new(false));

    ctrlc::set_handler({
        let should_interrupt = should_interrupt.clone();
        move || {
            should_interrupt.store(true, Ordering::Relaxed);
        }
    })
    .expect("cannot set interruption handler");

    Arc::new(InterruptionQuota { inner, should_interrupt })
}

struct InterruptionQuota {
    inner: Option<Arc<dyn Quota + Send + Sync>>,
    should_interrupt: Arc<AtomicBool>,
}

impl Quota for InterruptionQuota {
    fn is_reached(&self) -> bool {
        self.inner.as_ref().map_or(false, |inner| inner.is_reached()) || self.should_interrupt.load(Ordering::Relaxed)
    }
}
