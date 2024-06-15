use super::*;
use crate::construction::enablers::*;
use crate::models::Extras;

/// A trait to get or set reserved times index.
pub trait ReservedTimeExtras {
    /// Sets reserved times index.
    fn set_reserved_times(&mut self, reserved_time: ReservedTimesIndex) -> &mut Self;
    /// Gets reserved times index.
    fn get_reserved_times(&self) -> Option<&ReservedTimesIndex>;
}

struct ReservedTimeExtrasKey;
impl ReservedTimeExtras for Extras {
    fn set_reserved_times(&mut self, reserved_times: ReservedTimesIndex) -> &mut Self {
        self.set_value::<ReservedTimeExtrasKey, _>(reserved_times);
        self
    }

    fn get_reserved_times(&self) -> Option<&ReservedTimesIndex> {
        self.get_value::<ReservedTimeExtrasKey, _>()
    }
}

/// Reschedules reserved time duration in more optimal way:
/// - try to avoid it during travel by moving it to earlier time on point stop
/// - try to reduce waiting time by moving it
#[derive(Default)]
pub struct RescheduleReservedTime {}

impl HeuristicSolutionProcessing for RescheduleReservedTime {
    type Solution = InsertionContext;

    fn post_process(&self, mut solution: Self::Solution) -> Self::Solution {
        if let Some((reserved_times_idx, reserved_times_fn)) = get_reserved_times_index_and_fn(&solution) {
            solution
                .solution
                .routes
                .iter_mut()
                .filter(|route_ctx| reserved_times_idx.contains_key(&route_ctx.route().actor))
                .for_each(|route_ctx| {
                    optimize_reserved_times_schedule(route_ctx.route_mut(), &reserved_times_fn);
                    // NOTE: optimize_* method has to make sure that no time violation could happen and
                    //       rewrite schedules; calling accept_* methods will rewrite optimizations,
                    //       hence not desirable
                    route_ctx.mark_stale(false);
                });
            solution
        } else {
            solution
        }
    }
}

fn get_reserved_times_index_and_fn(insertion_ctx: &InsertionContext) -> Option<(ReservedTimesIndex, ReservedTimesFn)> {
    insertion_ctx.problem.extras.get_reserved_times().cloned().and_then(|reserved_times_idx| {
        create_reserved_times_fn(reserved_times_idx.clone())
            .ok()
            .map(|reserved_times_fn| (reserved_times_idx, reserved_times_fn))
    })
}
