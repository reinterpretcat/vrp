//! A feature to detect filter jobs based on their reachability.

use crate::construction::heuristics::MoveContext;
use crate::models::problem::{Job, TransportCost, TravelTime};
use crate::models::{ConstraintViolation, Feature, FeatureBuilder, FeatureConstraint, ViolationCode};
use rosomaxa::utils::GenericError;
use std::sync::Arc;

/// Creates a feature to check reachability of the jobs. It is a hard constraint.
pub fn create_reachable_feature(
    name: &str,
    transport: Arc<dyn TransportCost>,
    code: ViolationCode,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default().with_name(name).with_constraint(ReachableConstraint { transport, code }).build()
}

struct ReachableConstraint {
    transport: Arc<dyn TransportCost>,
    code: ViolationCode,
}

impl FeatureConstraint for ReachableConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { .. } => None,
            MoveContext::Activity { route_ctx, activity_ctx } => {
                let prev = activity_ctx.prev;
                let target = activity_ctx.target;
                let next = activity_ctx.next;

                let prev_to_target = self.transport.distance(
                    route_ctx.route(),
                    prev.place.location,
                    target.place.location,
                    TravelTime::Departure(prev.schedule.departure),
                );

                if prev_to_target < 0. {
                    return ConstraintViolation::skip(self.code);
                }

                if let Some(next) = next {
                    let target_to_next = self.transport.distance(
                        route_ctx.route(),
                        target.place.location,
                        next.place.location,
                        TravelTime::Departure(target.schedule.departure),
                    );
                    if target_to_next < 0. {
                        return ConstraintViolation::skip(self.code);
                    }
                }

                None
            }
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        // NOTE it is responsibility of the caller to check whether jobs are reachable
        Ok(source)
    }
}
