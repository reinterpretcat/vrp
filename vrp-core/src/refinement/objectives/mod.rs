//! Specifies objective functions.

use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::RefinementContext;
use crate::utils::VariationCoefficient;
use std::any::Any;
use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};
use std::ops::Deref;
use std::sync::Arc;

/// Specifies objective cost type.
pub trait ObjectiveCost {
    fn value(&self) -> Cost;
    fn cmp(&self, other: &Box<dyn ObjectiveCost + Send + Sync>) -> Ordering;

    fn clone_box(&self) -> Box<dyn ObjectiveCost + Send + Sync>;
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

/// A multi objective cost.
pub struct MultiObjectiveCost {
    primary_costs: ObjectiveCosts,
    secondary_costs: ObjectiveCosts,
    value_func: ObjectiveCostValueFn,
}

/// Encapsulates objective which has multiple objectives.
pub struct MultiObjective {
    /// List of primary objectives. Solution can be considered as improvement
    /// only if none of costs, returned by these objectives, is worse.
    primary_objectives: Vec<Box<dyn Objective + Send + Sync>>,
    /// List of secondary objectives. This list is evaluated only if primary objectives
    /// costs are considered as equal.
    secondary_objectives: Vec<Box<dyn Objective + Send + Sync>>,
    /// A function which extract actual cost from multiple objective costs.
    value_func: ObjectiveCostValueFn,
}

mod total_routes;
pub use self::total_routes::TotalRoutes;

mod total_transport_cost;
pub use self::total_transport_cost::TotalTransportCost;

mod total_unassigned_jobs;
pub use self::total_unassigned_jobs::TotalUnassignedJobs;

impl ObjectiveCost for MeasurableObjectiveCost {
    fn value(&self) -> f64 {
        self.cost
    }

    fn cmp(&self, other: &ObjectiveCostType) -> Ordering {
        self.cost.partial_cmp(&other.value()).unwrap_or(Less)
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

impl ObjectiveCost for MultiObjectiveCost {
    fn value(&self) -> f64 {
        self.value_func.deref()(&self.primary_costs, &self.secondary_costs)
    }

    fn cmp(&self, other: &ObjectiveCostType) -> Ordering {
        let (primary_costs, secondary_costs) = self.get_costs(other);

        match Self::analyze(&self.primary_costs, primary_costs) {
            Equal => Self::analyze(&self.secondary_costs, secondary_costs),
            primary @ _ => primary,
        }
    }

    fn clone_box(&self) -> ObjectiveCostType {
        Box::new(Self {
            primary_costs: self.primary_costs.iter().map(|c| c.clone_box()).collect(),
            secondary_costs: self.secondary_costs.iter().map(|c| c.clone_box()).collect(),
            value_func: self.value_func.clone(),
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MultiObjectiveCost {
    /// Creates a new instance of `MultiObjectiveCost`.
    pub fn new(
        primary_costs: ObjectiveCosts,
        secondary_costs: ObjectiveCosts,
        value_func: ObjectiveCostValueFn,
    ) -> Self {
        Self { primary_costs, secondary_costs, value_func }
    }

    fn get_costs<'a>(&self, other: &'a ObjectiveCostType) -> (&'a Vec<ObjectiveCostType>, &'a Vec<ObjectiveCostType>) {
        let other = other.as_any().downcast_ref::<MultiObjectiveCost>().expect("Expecting MultiObjectiveCost");

        let primary_costs = &other.primary_costs;
        assert_eq!(self.primary_costs.len(), primary_costs.len());

        let secondary_costs = &other.secondary_costs;
        assert_eq!(self.secondary_costs.len(), secondary_costs.len());

        (primary_costs, secondary_costs)
    }

    fn analyze(left: &Vec<ObjectiveCostType>, right: &Vec<ObjectiveCostType>) -> Ordering {
        left.iter().zip(right.iter()).fold(Equal, |acc, (a, b)| match (acc, a.cmp(b)) {
            (Equal, new @ _) => new,
            (Less, Greater) => Greater,
            (Less, _) => Less,
            (Greater, _) => Greater,
        })
    }
}

impl MultiObjective {
    /// Creates a new instance of `MultiObjective`.
    pub fn new(
        primary_objectives: Vec<Box<dyn Objective + Send + Sync>>,
        secondary_objectives: Vec<Box<dyn Objective + Send + Sync>>,
        value_func: ObjectiveCostValueFn,
    ) -> Self {
        assert!(!primary_objectives.is_empty() || !secondary_objectives.is_empty());
        Self { primary_objectives, secondary_objectives, value_func }
    }
}

impl Default for MultiObjective {
    fn default() -> Self {
        Self {
            primary_objectives: vec![Box::new(TotalRoutes::default()), Box::new(TotalUnassignedJobs::default())],
            secondary_objectives: vec![Box::new(TotalTransportCost::default())],
            value_func: Arc::new(|_, secondary| secondary.first().unwrap().value()),
        }
    }
}

impl Objective for MultiObjective {
    fn estimate_cost(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> ObjectiveCostType {
        let primary_costs =
            self.primary_objectives.iter().map(|o| o.estimate_cost(refinement_ctx, insertion_ctx)).collect::<Vec<_>>();
        let secondary_costs = self
            .secondary_objectives
            .iter()
            .map(|o| o.estimate_cost(refinement_ctx, insertion_ctx))
            .collect::<Vec<_>>();

        Box::new(MultiObjectiveCost::new(primary_costs, secondary_costs, self.value_func.clone()))
    }

    fn is_goal_satisfied(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<bool> {
        let mut get_satisfaction = |objectives: &Vec<Box<dyn Objective + Send + Sync>>| {
            objectives.iter().filter_map(|o| o.is_goal_satisfied(refinement_ctx, insertion_ctx)).collect::<Vec<_>>()
        };

        let mut results = get_satisfaction(&self.primary_objectives);

        if results.is_empty() {
            results.extend(get_satisfaction(&self.secondary_objectives).into_iter())
        }

        if results.is_empty() {
            None
        } else {
            Some(results.iter().all(|&goal_satisfied| goal_satisfied))
        }
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
