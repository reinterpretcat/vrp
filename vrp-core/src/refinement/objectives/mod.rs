//! Specifies objective functions.

use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::RefinementContext;
use crate::utils::VariationCoefficient;
use std::any::Any;
use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};
use std::sync::Arc;

/// Specifies objective cost type.
pub trait ObjectiveCost {
    /// Returns absolute value of objective.
    fn value(&self) -> Cost;
    /// Compares objectives costs together, returns (`actual`, `relaxed`) ordering.
    fn cmp_relaxed(&self, other: &Box<dyn ObjectiveCost + Send + Sync>) -> (Ordering, Ordering);
    /// Clones objective cost.
    fn clone_box(&self) -> Box<dyn ObjectiveCost + Send + Sync>;
    /// Returns objective cost as `Any`.
    fn as_any(&self) -> &dyn Any;
}

/// A short alias for boxed `ObjectiveCost`.
pub type ObjectiveCostType = Box<dyn ObjectiveCost + Send + Sync>;
/// Specifies collection of objective costs.
pub type ObjectiveCosts = Vec<ObjectiveCostType>;
/// Specifies function which returns actual cost from multiple objective costs.
pub type ObjectiveCostValueFn = Arc<dyn Fn(&ObjectiveCosts, &ObjectiveCosts) -> f64 + Send + Sync>;

/// Encapsulates objective function behaviour.
pub trait Objective {
    /// Estimates cost for given problem and solution.
    fn estimate_cost(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> ObjectiveCostType;

    /// Checks whether given solution satisfies objective.
    /// Returns `None` if objective goal is not set.
    fn is_goal_satisfied(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<bool>;
}

/// An objective cost with measurable value.
pub struct MeasurableObjectiveCost {
    cost: Cost,
    tolerance: Option<f64>,
}

mod total_routes;
pub use self::total_routes::TotalRoutes;

mod total_transport_cost;
pub use self::total_transport_cost::TotalTransportCost;

mod total_unassigned_jobs;
pub use self::total_unassigned_jobs::TotalUnassignedJobs;

mod multi_objective;
pub use self::multi_objective::MultiObjective;
pub use self::multi_objective::MultiObjectiveCost;

impl ObjectiveCost for MeasurableObjectiveCost {
    fn value(&self) -> f64 {
        self.cost
    }

    fn cmp_relaxed(&self, other: &ObjectiveCostType) -> (Ordering, Ordering) {
        let actual = self.cost.partial_cmp(&other.value()).unwrap_or(Less);
        let relaxed = if let Some(tolerance) = self.tolerance {
            // NOTE we get actual ratio between two values
            let ratio = (other.value() - self.cost).abs() / self.cost;
            if ratio.is_normal() && ratio < tolerance {
                Equal
            } else {
                actual
            }
        } else {
            actual
        };

        (actual, relaxed)
    }

    fn clone_box(&self) -> ObjectiveCostType {
        Box::new(Self { cost: self.cost, tolerance: self.tolerance })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MeasurableObjectiveCost {
    pub fn new(cost: Cost) -> Self {
        Self { cost, tolerance: None }
    }

    pub fn new_with_tolerance(cost: Cost, tolerance: Option<f64>) -> Self {
        Self { cost, tolerance }
    }
}

fn check_value_variation_goals(
    refinement_ctx: &mut RefinementContext,
    actual_value: f64,
    value_goal: &Option<(f64, bool)>,
    variation_goal: &Option<VariationCoefficient>,
) -> Option<bool> {
    let variation =
        variation_goal.as_ref().map(|variation_goal| variation_goal.update_and_check(refinement_ctx, actual_value));
    let value = value_goal.as_ref().map(|&(desired_value, is_minimization)| {
        if is_minimization {
            actual_value <= desired_value
        } else {
            actual_value >= desired_value
        }
    });

    variation.map(|variation| variation || value.unwrap_or(false)).or(value)
}
