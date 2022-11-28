#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/total_transport_test.rs"]
mod total_transport_test;

use super::*;
use crate::construction::constraints::{TOTAL_AREA_KEY, TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::models::common::Cost;
use crate::models::problem::TargetObjective;
use rosomaxa::prelude::*;
use std::ops::Deref;
use std::sync::Arc;

/// An objective function for total cost minimization as a target.
pub struct TotalCost;

impl TotalCost {
    /// Creates an objective to minimize total cost.
    pub fn minimize() -> TargetObjective {
        Arc::new(TotalTransport { fitness: Arc::new(|insertion_ctx| insertion_ctx.solution.get_total_cost()) })
    }
}

/// An objective function for total distance minimization as a target.
pub struct TotalDistance;

impl TotalDistance {
    /// Creates an objective to minimize total distance.
    pub fn minimize() -> TargetObjective {
        new_with_route_state_key(TOTAL_DISTANCE_KEY)
    }
}

/// An objective function for total area minimization as a target.
pub struct TotalArea;

impl TotalArea {
    /// Creates an objective to minimize total distance.
    pub fn minimize() -> TargetObjective {
        new_with_route_state_key(TOTAL_AREA_KEY)
    }
}

/// An objective function for total duration minimization as a target.
pub struct TotalDuration;

impl TotalDuration {
    /// Creates an objective to minimize total duration.
    pub fn minimize() -> TargetObjective {
        new_with_route_state_key(TOTAL_DURATION_KEY)
    }
}

struct TotalTransport {
    fitness: Arc<dyn Fn(&InsertionContext) -> f64 + Send + Sync>,
}

impl Objective for TotalTransport {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        self.fitness.deref()(solution)
    }
}

fn new_with_route_state_key(key: i32) -> TargetObjective {
    Arc::new(TotalTransport {
        fitness: Arc::new(move |insertion_ctx| {
            insertion_ctx
                .solution
                .routes
                .iter()
                .fold(Cost::default(), move |acc, rc| acc + rc.state.get_route_state::<f64>(key).cloned().unwrap_or(0.))
        }),
    })
}
