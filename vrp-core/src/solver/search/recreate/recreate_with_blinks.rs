use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::Problem;
use crate::models::common::*;
use crate::models::problem::Job;
use crate::solver::RefinementContext;
use crate::solver::search::recreate::Recreate;
use rosomaxa::prelude::*;
use std::sync::Arc;

struct ChunkJobSelector {
    size: usize,
}

impl ChunkJobSelector {
    pub fn new(size: usize) -> Self {
        Self { size }
    }
}

impl JobSelector for ChunkJobSelector {
    fn select<'a>(&'a self, insertion_ctx: &'a InsertionContext) -> Box<dyn Iterator<Item = &'a Job> + 'a> {
        Box::new(insertion_ctx.solution.required.iter().take(self.size))
    }
}

struct RankedJobSelector {
    asc_order: bool,
}

impl RankedJobSelector {
    pub fn new(asc_order: bool) -> Self {
        Self { asc_order }
    }

    pub fn rank_job(problem: &Arc<Problem>, job: &Job) -> Cost {
        problem
            .fleet
            .profiles
            .iter()
            .map(|profile| problem.jobs.rank(profile, job).unwrap_or(Cost::MAX))
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or_default()
    }
}

impl JobSelector for RankedJobSelector {
    fn prepare(&self, insertion_ctx: &mut InsertionContext) {
        let problem = &insertion_ctx.problem;

        insertion_ctx
            .solution
            .required
            .sort_by(|a, b| Self::rank_job(problem, a).total_cmp(&Self::rank_job(problem, b)));

        if self.asc_order {
            insertion_ctx.solution.required.reverse();
        }
    }
}

/// A recreate method as described in "Slack Induction by String Removals for
/// Vehicle Routing Problems" (aka SISR) paper by Jan Christiaens, Greet Vanden Berghe.
pub struct RecreateWithBlinks {
    job_selectors: Vec<Box<dyn JobSelector>>,
    route_selector: Box<dyn RouteSelector>,
    leg_selection: LegSelection,
    result_selector: Box<dyn ResultSelector>,
    insertion_heuristic: InsertionHeuristic,
    weights: Vec<usize>,
}

impl RecreateWithBlinks {
    /// Creates a new instance of `RecreateWithBlinks`.
    pub fn new(selectors: Vec<(Box<dyn JobSelector>, usize)>, random: Arc<dyn Random>) -> Self {
        let weights = selectors.iter().map(|(_, weight)| *weight).collect();
        Self {
            job_selectors: selectors.into_iter().map(|(selector, _)| selector).collect(),
            route_selector: Box::<AllRouteSelector>::default(),
            leg_selection: LegSelection::Stochastic(random.clone()),
            result_selector: Box::new(BlinkResultSelector::new_with_defaults(random)),
            insertion_heuristic: Default::default(),
            weights,
        }
    }

    /// Creates a new instance of `RecreateWithBlinks` with default prameters.
    pub fn new_with_defaults(random: Arc<dyn Random>) -> Self {
        Self::new(
            vec![
                (Box::<AllJobSelector>::default(), 10),
                (Box::new(ChunkJobSelector::new(8)), 10),
                (Box::new(RankedJobSelector::new(true)), 5),
                (Box::new(RankedJobSelector::new(false)), 1),
            ],
            random,
        )
    }
}

impl Recreate for RecreateWithBlinks {
    fn run(&self, _: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.environment.random.weighted(self.weights.as_slice());
        let job_selector = self.job_selectors.get(index).unwrap().as_ref();

        self.insertion_heuristic.process(
            insertion_ctx,
            job_selector,
            self.route_selector.as_ref(),
            &self.leg_selection,
            self.result_selector.as_ref(),
        )
    }
}
