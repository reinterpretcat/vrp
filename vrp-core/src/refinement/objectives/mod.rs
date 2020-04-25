//! Specifies objective functions.

use crate::construction::heuristics::InsertionContext;
use crate::models::common::Cost;
use crate::utils::VariationCoefficient;
use std::cmp::Ordering;

/// An *objective* defines a *total ordering relation* and a *distance metric* on a set of
/// `solutions`. Given any two solutions, an objective answers the following two questions:
///
/// - "which solution is the better one" (total order)
/// - "how similar are the two solutions" (distance metric)
pub trait Objective {
    /// The solution value type that we define the objective on.
    type Solution;

    /// An objective defines a total ordering between any two solution values.
    ///
    /// This answers the question, is solution `a` better, equal or worse than solution `b`,
    /// according to the objective.
    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering;

    /// An objective defines a distance metric between any two solution values.
    ///
    /// The distance metric answer the question, how similar the solutions `a` and `b` are,
    /// according to the objective. A zero value would mean, that both solutions are in fact the same,
    /// according to the objective. Larger magnitudes would mean "less similar".
    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64;

    /// An objective fitness value for given `solution`.
    fn fitness(&self, solution: &Self::Solution) -> f64;
}

mod composite_objectives;
pub use self::composite_objectives::*;

mod total_routes;
pub use self::total_routes::TotalRoutes;

mod total_transport_cost;
pub use self::total_transport_cost::TotalTransportCost;

mod total_unassigned_jobs;
pub use self::total_unassigned_jobs::TotalUnassignedJobs;

mod work_balance;
pub use self::work_balance::WorkBalance;
