//! This module provides functionailty to reject plans where
//! departures leave after a place's available times, instead
//! of merely having the restriction apply for arrival times
use std::sync::Arc;

use rosomaxa::utils::GenericError;

use crate::{
    models::{ConstraintViolation, Feature, FeatureBuilder, FeatureConstraint, ViolationCode},
    prelude::ActivityCost,
};

struct StrictDepartureConstraint {
    time_constraint_code: ViolationCode,
    activity: Arc<dyn ActivityCost>,
}

impl FeatureConstraint for StrictDepartureConstraint {
    fn evaluate(&self, move_ctx: &crate::prelude::MoveContext<'_>) -> Option<crate::prelude::ConstraintViolation> {
        match move_ctx {
            crate::prelude::MoveContext::Activity { activity_ctx, route_ctx, .. } => {
                let activity_departure = self.activity.estimate_departure(
                    route_ctx.route(),
                    activity_ctx.target,
                    activity_ctx.target.schedule.arrival,
                );
                let place_closing = activity_ctx.target.place.time.end;
                if activity_departure > place_closing {
                    ConstraintViolation::fail(self.time_constraint_code)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
pub fn create_strict_departure_feature(
    name: &str,
    activity: Arc<dyn ActivityCost>,
    time_constraint_code: ViolationCode,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(StrictDepartureConstraint { activity, time_constraint_code })
        .build()
}
