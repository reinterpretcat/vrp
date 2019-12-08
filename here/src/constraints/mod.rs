use core::models::problem::{Job, Single};
use core::models::solution::Activity;

fn as_single_job<F>(activity: &Activity, condition: F) -> Option<Arc<Single>>
where
    F: Fn(&Arc<Single>) -> bool,
{
    activity.job.as_ref().and_then(|job| match job.as_ref() {
        Job::Single(job) => {
            if condition(job) {
                Some(job.clone())
            } else {
                None
            }
        }
        _ => None,
    })
}

mod breaks;
pub use self::breaks::BreakModule;

mod extra_costs;
pub use self::extra_costs::ExtraCostModule;

mod multi_tour_capacity;
pub use self::multi_tour_capacity::MultiTourCapacityConstraintModule;

mod reachable;
pub use self::reachable::ReachableModule;

mod skills;
pub use self::skills::SkillsModule;
use std::sync::Arc;
