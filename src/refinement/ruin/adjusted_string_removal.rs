use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::construction::states::InsertionContext;
use crate::models::problem::Job;
use crate::models::solution::Route;
use crate::models::Solution;
use crate::refinement::ruin::{create_insertion_context, RuinStrategy};
use crate::refinement::RefinementContext;

/// "Adjusted string removal" strategy based on "Slack Induction by String Removals for
/// Vehicle Routing Problems" (aka SISR) by Jan Christiaens, Greet Vanden Berghe.
/// Some definitions from the paper:
///     String is a sequence of consecutive nodes in a tour.
///     Cardinality is the number of customers included in a string or tour.
pub struct AdjustedStringRemoval {
    /// Specifies max removed string cardinality for specific tour.
    lmax: usize,
    /// Specifies average number of removed customers.
    cavg: usize,
    /// Preserved customers ratio.
    alpha: f64,
}

impl AdjustedStringRemoval {
    fn new(lmax: usize, cavg: usize, alpha: f64) -> Self {
        Self { lmax, cavg, alpha }
    }
}

impl Default for AdjustedStringRemoval {
    fn default() -> Self {
        Self::new(10, 10, 0.01)
    }
}

impl RuinStrategy for AdjustedStringRemoval {
    fn ruin_solution(refinement_ctx: &RefinementContext, solution: &Solution) -> InsertionContext {
        let jobs: HashSet<Arc<Job>> = HashSet::new();
        let routes: HashSet<Box<Route>> = HashSet::new();
        let insertion_cxt = create_insertion_context(refinement_ctx, solution);

        unimplemented!()
    }
}

/// Selects random job from existing solution
fn select_random_job<'a>(routes: &'a Vec<Route>) -> Option<(&'a Route, &Arc<Job>)> {
    if routes.is_empty() {
        return None;
    }

    //let route_index =

    unimplemented!()
}
