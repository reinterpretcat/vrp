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
///
/// Follows the canonical SISR insertion order: jobs are sorted once by the chosen criterion
/// and inserted in that order without reevaluating remaining jobs after each insertion.
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

        Self::new(
            vec![
                // --- Core Selectors (Section 5.3) ---
                // 1. Random (Weight: 4)
                (Box::<AllJobSelector>::default(), 4),
                // 2. Demand: Largest First (Weight: 4)
                (Box::new(DemandJobSelector::new(true)), 4),
                // 3. Far: Largest Distance First (Weight: 2)
                (Box::new(RankedJobSelector::new(true)), 2),
                // 4. Close: Smallest Distance First (Weight: 1)
                (Box::new(RankedJobSelector::new(false)), 1),
                // --- VRPTW Extensions (Table 13) ---
                // 5. TW Length: Increasing (Shortest/Hardest first) (Weight: 2)
                (Box::new(TimeWindowJobSelector::new(TimeWindowSelectionMode::LengthAscending)), 2),
                // 6. TW Start: Increasing (Earliest first) (Weight: 2)
                (Box::new(TimeWindowJobSelector::new(TimeWindowSelectionMode::StartAscending)), 2),
                // 7. TW End: Decreasing (Latest first) (Weight: 2)
                (Box::new(TimeWindowJobSelector::new(TimeWindowSelectionMode::EndDescending)), 2),
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
        // Canonical SISR drives one job at a time via the `process` override below.
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
            InsertionResult::choose_best_result,
        );

        match result {
            InsertionResult::Success(_) => result,
            InsertionResult::Failure(mut failure) => {
                // If we failed to insert, blame the specific job we were trying to insert —
                // otherwise the failure handler treats it as systemic (no routes) and may
                // drain all remaining jobs.
                if failure.job.is_none() {
                    failure.job = Some((*job).clone());
                }
                InsertionResult::Failure(failure)
            }
        }
    }

    /// Canonical SISR insertion order: sort `required` once via the chosen `JobSelector`,
    /// then insert jobs strictly in that order without re-sorting or reselecting.
    fn process(
        &self,
        mut insertion_ctx: InsertionContext,
        job_selector: &dyn JobSelector,
        route_selector: &dyn RouteSelector,
        leg_selection: &LegSelection,
        result_selector: &dyn ResultSelector,
    ) -> InsertionContext {
        prepare_insertion_ctx(&mut insertion_ctx);

        // Sort once.
        job_selector.prepare(&mut insertion_ctx);
        route_selector.prepare(&mut insertion_ctx);

        while !insertion_ctx.solution.required.is_empty()
            && !insertion_ctx.environment.quota.as_ref().is_some_and(|q| q.is_reached())
        {
            // Drain in sorted order: take the next job rather than reselecting from the full set.
            let job = insertion_ctx.solution.required[0].clone();
            let result = {
                let jobs = [&job];
                let routes = route_selector.select(&insertion_ctx, &jobs).collect::<Vec<_>>();
                self.evaluate_all(&insertion_ctx, &jobs, &routes, leg_selection, result_selector)
            };

            match result {
                InsertionResult::Success(success) => apply_insertion_success(&mut insertion_ctx, success),
                InsertionResult::Failure(failure) => {
                    // Single-job failure semantics: record this job as unassigned and move on.
                    // `evaluate_all` above guarantees `failure.job` is set to our target.
                    let failed_job = failure.job.unwrap_or_else(|| job.clone());
                    insertion_ctx
                        .solution
                        .unassigned
                        .insert(failed_job.clone(), UnassignmentInfo::Simple(failure.constraint));
                    insertion_ctx.solution.required.retain(|j| *j != failed_job);
                }
            }
        }

        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
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
