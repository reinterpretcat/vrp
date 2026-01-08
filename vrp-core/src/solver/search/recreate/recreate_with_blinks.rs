use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, JobSelector};
use crate::models::Problem;
use crate::models::problem::Job;
use crate::prelude::Cost;
use crate::solver::RefinementContext;
use crate::solver::search::recreate::Recreate;
use rosomaxa::prelude::*;
use rosomaxa::utils::fold_reduce;
use std::sync::Arc;

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
    pub fn new(selectors: Vec<(Box<dyn JobSelector>, usize)>, blink_ratio: f64, random: Arc<dyn Random>) -> Self {
        let weights = selectors.iter().map(|(_, weight)| *weight).collect();
        let evaluator = Box::new(BlinkInsertionEvaluator::new(blink_ratio, random.clone()));

        Self {
            job_selectors: selectors.into_iter().map(|(selector, _)| selector).collect(),
            route_selector: Box::<AllRouteSelector>::default(),
            leg_selection: LegSelection::Exhaustive,
            result_selector: Box::<BestResultSelector>::default(),
            insertion_heuristic: InsertionHeuristic::new(evaluator),
            weights,
        }
    }

    /// Creates a new instance with defaults compliant with SISR paper (Section 5.3 + Table 13).
    pub fn new_with_defaults(random: Arc<dyn Random>) -> Self {
        let blink_ratio = 0.01;

        // Helper to wrap selectors with SingletonJobSelector
        fn singleton<T: JobSelector + 'static>(selector: T) -> Box<dyn JobSelector> {
            Box::new(SingletonJobSelector(selector))
        }

        Self::new(
            vec![
                // --- Core Selectors (Section 5.3) ---
                // 1. Random (Weight: 4)
                (singleton(AllJobSelector::default()), 4),
                // 2. Demand: Largest First (Weight: 4)
                (singleton(DemandJobSelector::new(true)), 4),
                // 3. Far: Largest Distance First (Weight: 2)
                (singleton(RankedJobSelector::new(true)), 2),
                // 4. Close: Smallest Distance First (Weight: 1)
                (singleton(RankedJobSelector::new(false)), 1),
                // --- VRPTW Extensions (Table 13) ---
                // 5. TW Length: Increasing (Shortest/Hardest first) (Weight: 2)
                (singleton(TimeWindowJobSelector::new(TimeWindowSelectionMode::LengthAscending)), 2),
                // 6. TW Start: Increasing (Earliest first) (Weight: 2)
                (singleton(TimeWindowJobSelector::new(TimeWindowSelectionMode::StartAscending)), 2),
                // 7. TW End: Decreasing (Latest first) (Weight: 2)
                (singleton(TimeWindowJobSelector::new(TimeWindowSelectionMode::EndDescending)), 2),
            ],
            blink_ratio,
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

struct BlinkInsertionEvaluator {
    blink_ratio: f64,
    random: Arc<dyn Random>,
}

impl BlinkInsertionEvaluator {
    pub fn new(blink_ratio: f64, random: Arc<dyn Random>) -> Self {
        Self { blink_ratio, random }
    }
}

impl InsertionEvaluator for BlinkInsertionEvaluator {
    fn evaluate_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[&Job],
        routes: &[&RouteContext],
        _leg_selection: &LegSelection,
        result_selector: &dyn ResultSelector,
    ) -> InsertionResult {
        // With SingletonJobSelector, this is guaranteed to be the single target job.
        let job = match jobs.first() {
            Some(job) => job,
            None => return InsertionResult::make_failure(),
        };

        let eval_ctx = EvaluationContext {
            goal: &insertion_ctx.problem.goal,
            job,
            leg_selection: &LegSelection::Exhaustive,
            result_selector,
        };

        let result = fold_reduce(
            routes,
            InsertionResult::make_failure,
            |best_in_thread, route_ctx| {
                let mut best_in_route = best_in_thread;
                let tour = &route_ctx.route().tour;

                for leg_index in 0..tour.legs().count() {
                    // The Blink: "Each position is evaluated with a probability of 1 - gamma"
                    if self.random.is_hit(self.blink_ratio) {
                        continue;
                    }

                    // Evaluate specific position
                    let result = eval_job_insertion_in_route(
                        insertion_ctx,
                        &eval_ctx,
                        route_ctx,
                        InsertionPosition::Concrete(leg_index),
                        best_in_route,
                    );

                    best_in_route = result;
                }

                best_in_route
            },
            // Reduce: Merge results from threads (Greedy/Best selection)
            |a, b| InsertionResult::choose_best_result(a, b),
        );

        match result {
            InsertionResult::Success(_) => result,
            InsertionResult::Failure(mut failure) => {
                // If we failed to insert, we must blame the specific job we were trying
                // to insert. Otherwise, the heuristic thinks the issue is systemic (no routes)
                // and might abort entirely.
                if failure.job.is_none() {
                    failure.job = Some((*job).clone());
                }
                InsertionResult::Failure(failure)
            }
        }
    }
}

struct SingletonJobSelector<T>(T);

impl<T: JobSelector> JobSelector for SingletonJobSelector<T> {
    fn prepare(&self, ctx: &mut InsertionContext) {
        self.0.prepare(ctx);
    }

    fn select<'a>(&'a self, ctx: &'a InsertionContext) -> Box<dyn Iterator<Item = &'a Job> + 'a> {
        // Only yield the first job: this forces InsertionHeuristic to align with BlinkInsertionEvaluator.
        Box::new(self.0.select(ctx).take(1))
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

struct DemandJobSelector {
    asc_order: bool,
}

impl DemandJobSelector {
    pub fn new(asc_order: bool) -> Self {
        Self { asc_order }
    }
}

impl JobSelector for DemandJobSelector {
    fn prepare(&self, insertion_ctx: &mut InsertionContext) {
        insertion_ctx.solution.required.sort_by(|a, b| {
            use crate::construction::features::JobDemandDimension;
            use crate::models::common::{MultiDimLoad, SingleDimLoad};

            // Get total demand size (sum of all pickup and delivery components)
            // Try SingleDimLoad first, then MultiDimLoad (only one type per problem)
            let get_demand = |job: &Job| -> i32 {
                let dimens = job.dimens();

                if let Some(demand) = dimens.get_job_demand::<SingleDimLoad>() {
                    demand.pickup.0.value + demand.pickup.1.value + demand.delivery.0.value + demand.delivery.1.value
                } else if let Some(demand) = dimens.get_job_demand::<MultiDimLoad>() {
                    let sum_load = |load: &MultiDimLoad| load.load[..load.size].iter().sum::<i32>();
                    sum_load(&demand.pickup.0)
                        + sum_load(&demand.pickup.1)
                        + sum_load(&demand.delivery.0)
                        + sum_load(&demand.delivery.1)
                } else {
                    0
                }
            };

            get_demand(a).cmp(&get_demand(b))
        });

        if !self.asc_order {
            insertion_ctx.solution.required.reverse();
        }
    }
}

/// Specifies the sorting criterion for Time Windows (Table 13 of SISR paper).
pub enum TimeWindowSelectionMode {
    /// Sort by time window duration (End - Start).
    /// Paper: "Increasing time window length"
    LengthAscending,

    /// Sort by start time.
    /// Paper: "Increasing time window start"
    StartAscending,

    /// Sort by end time.
    /// Paper: "Decreasing time window end"
    EndDescending,
}

pub struct TimeWindowJobSelector {
    mode: TimeWindowSelectionMode,
}

impl TimeWindowJobSelector {
    pub fn new(mode: TimeWindowSelectionMode) -> Self {
        Self { mode }
    }

    /// Helper to extract the relevant time window metric from a job.
    /// Returns (min_start, max_end, min_length).
    /// Handles multiple time windows by finding the extreme boundaries.
    fn get_tw_metric(job: &Job) -> (f64, f64, f64) {
        job.places()
            .flat_map(|place| place.times.iter())
            // NOTE: ignore non-time window spans. This is the most easy way to handle
            // when multiple span types mixed.
            .filter_map(|time| time.as_time_window())
            .fold(None, |acc: Option<(f64, f64, f64)>, tw| {
                let length = tw.end - tw.start;
                Some(match acc {
                    None => (tw.start, tw.end, length),
                    Some((min_start, max_end, min_length)) => {
                        (min_start.min(tw.start), max_end.max(tw.end), min_length.min(length))
                    }
                })
            })
            .unwrap_or((0.0, f64::MAX, f64::MAX))
    }
}

impl JobSelector for TimeWindowJobSelector {
    fn prepare(&self, insertion_ctx: &mut InsertionContext) {
        insertion_ctx.solution.required.sort_by(|a, b| {
            let (start_a, end_a, len_a) = Self::get_tw_metric(a);
            let (start_b, end_b, len_b) = Self::get_tw_metric(b);

            match self.mode {
                // Increasing Length: Shortest windows first (hardest to fit)
                TimeWindowSelectionMode::LengthAscending => len_a.total_cmp(&len_b),

                // Increasing Start: Earliest start first (chronological)
                TimeWindowSelectionMode::StartAscending => start_a.total_cmp(&start_b),

                // Decreasing End: Latest end first (reverse chronological/urgency)
                TimeWindowSelectionMode::EndDescending => end_b.total_cmp(&end_a),
            }
        });
    }
}
