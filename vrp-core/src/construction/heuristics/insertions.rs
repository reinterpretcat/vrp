#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/insertions_test.rs"]
mod insertions_test;

use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use crate::models::solution::Activity;
use rosomaxa::utils::unwrap_from_result;
use std::cmp::Ordering;
use std::ops::{Add, Sub};
use tinyvec::{tiny_vec, TinyVec, TinyVecIterator};

/// Specifies insertion result variant.
pub enum InsertionResult {
    /// Successful insertion result.
    Success(InsertionSuccess),
    /// Insertion failure.
    Failure(InsertionFailure),
}

/// Specifies insertion success result needed to insert job into tour.
pub struct InsertionSuccess {
    /// Specifies delta cost change for the insertion.
    pub cost: InsertionCost,

    /// Original job to be inserted.
    pub job: Job,

    /// Specifies activities within index where they have to be inserted.
    pub activities: Vec<(Activity, usize)>,

    /// Specifies route context where insertion happens.
    pub context: RouteContext,
}

/// Specifies insertion failure.
pub struct InsertionFailure {
    /// Failed constraint code.
    pub constraint: i32,
    /// A flag which signalizes that algorithm should stop trying to insert at next positions.
    pub stopped: bool,
    /// Original job failed to be inserted.
    pub job: Option<Job>,
}

type CostArray = [Cost; 8];

/// A hierarchical cost of job's insertion.
#[derive(Clone, Debug, Default)]
pub struct InsertionCost {
    data: TinyVec<CostArray>,
}

impl InsertionCost {
    /// Creates a new instance of `InsertionCost`.
    pub fn new(data: &[Cost]) -> Self {
        Self { data: data.into() }
    }

    /// Returns iterator over cost values.
    pub fn iter(&self) -> impl Iterator<Item = Cost> + '_ {
        self.data.iter().cloned()
    }

    /// Returns highest* possible insertion cost.
    pub fn max_value() -> Self {
        Self { data: tiny_vec!(Cost::MAX, Cost::MAX, Cost::MAX, Cost::MAX) }
    }

    fn operator(self, rhs: Self, op: fn(Cost, Cost) -> Cost) -> Self {
        let (mut left, right) =
            if self.data.len() < rhs.data.len() { (self.data, rhs.data) } else { (rhs.data, self.data) };

        left.iter_mut().enumerate().for_each(|(idx, value)| {
            *value = op(*value, right[idx]);
        });

        Self { data: left }
    }
}

impl FromIterator<Cost> for InsertionCost {
    fn from_iter<T: IntoIterator<Item = Cost>>(iter: T) -> Self {
        Self { data: TinyVec::<CostArray>::from_iter(iter) }
    }
}

impl IntoIterator for InsertionCost {
    type Item = Cost;
    type IntoIter = TinyVecIterator<CostArray>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl Eq for InsertionCost {}

impl PartialEq for InsertionCost {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl PartialOrd for InsertionCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InsertionCost {
    fn cmp(&self, other: &Self) -> Ordering {
        let size = self.data.len().max(other.data.len());
        unwrap_from_result((0..size).try_fold(Ordering::Equal, |acc, idx| {
            let left = self.data.get(idx).cloned().unwrap_or_default();
            let right = other.data.get(idx).cloned().unwrap_or_default();

            let result = left.total_cmp(&right);
            match result {
                Ordering::Equal => Ok(acc),
                _ => Err(result),
            }
        }))
    }
}

impl Add for InsertionCost {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.operator(rhs, |a, b| a + b)
    }
}

impl Sub for InsertionCost {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.operator(rhs, |a, b| a - b)
    }
}

/// Implements generalized insertion heuristic.
/// Using `JobSelector`, `RouteSelector`, and `ResultSelector` it tries to identify next job to
/// be inserted until there are no jobs left or it is not possible to insert due to constraint
/// limitations.
pub struct InsertionHeuristic {
    insertion_evaluator: Box<dyn InsertionEvaluator + Send + Sync>,
}

impl Default for InsertionHeuristic {
    fn default() -> Self {
        InsertionHeuristic::new(Box::new(PositionInsertionEvaluator::default()))
    }
}

impl InsertionHeuristic {
    /// Creates a new instance of `InsertionHeuristic`.
    pub fn new(insertion_evaluator: Box<dyn InsertionEvaluator + Send + Sync>) -> Self {
        Self { insertion_evaluator }
    }
}

impl InsertionHeuristic {
    /// Runs common insertion heuristic algorithm using given selector specializations.
    pub fn process(
        &self,
        insertion_ctx: InsertionContext,
        job_selector: &(dyn JobSelector + Send + Sync),
        route_selector: &(dyn RouteSelector + Send + Sync),
        leg_selector: &(dyn LegSelector + Send + Sync),
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        prepare_insertion_ctx(&mut insertion_ctx);

        while !insertion_ctx.solution.required.is_empty()
            && !insertion_ctx.environment.quota.as_ref().map_or(false, |q| q.is_reached())
        {
            let jobs = job_selector.select(&mut insertion_ctx).collect::<Vec<Job>>();
            let routes = route_selector.select(&mut insertion_ctx, jobs.as_slice()).collect::<Vec<RouteContext>>();

            let result = self.insertion_evaluator.evaluate_all(
                &insertion_ctx,
                jobs.as_slice(),
                routes.as_slice(),
                leg_selector,
                result_selector,
            );

            match result {
                InsertionResult::Success(success) => {
                    apply_insertion_success(&mut insertion_ctx, success);
                }
                InsertionResult::Failure(failure) => {
                    apply_insertion_failure(&mut insertion_ctx, jobs, routes, failure);
                }
            }
        }

        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
    }
}

impl InsertionResult {
    /// Creates result which represents insertion success.
    pub fn make_success(
        cost: InsertionCost,
        job: Job,
        activities: Vec<(Activity, usize)>,
        route_ctx: RouteContext,
    ) -> Self {
        Self::Success(InsertionSuccess { cost, job, activities, context: route_ctx })
    }

    /// Creates result which represents insertion failure.
    pub fn make_failure() -> Self {
        Self::make_failure_with_code(-1, false, None)
    }

    /// Creates result which represents insertion failure with given code.
    pub fn make_failure_with_code(code: i32, stopped: bool, job: Option<Job>) -> Self {
        Self::Failure(InsertionFailure { constraint: code, stopped, job })
    }

    /// Compares two insertion results and returns the cheapest by cost.
    pub fn choose_best_result(left: Self, right: Self) -> Self {
        match (&left, &right) {
            (Self::Success(_), Self::Failure(_)) => left,
            (Self::Failure(_), Self::Success(_)) => right,
            (Self::Success(lhs), Self::Success(rhs)) => {
                if lhs.cost > rhs.cost {
                    right
                } else {
                    left
                }
            }
            (Self::Failure(_), Self::Failure(rhs)) => {
                if rhs.constraint == -1 {
                    left
                } else {
                    right
                }
            }
        }
    }

    /// Returns insertion result as success.
    pub fn as_success(&self) -> Option<&InsertionSuccess> {
        match self {
            Self::Success(success) => Some(success),
            Self::Failure(_) => None,
        }
    }

    /// Returns insertion result as success.
    pub fn into_success(self) -> Option<InsertionSuccess> {
        match self {
            Self::Success(success) => Some(success),
            Self::Failure(_) => None,
        }
    }
}

pub(crate) fn prepare_insertion_ctx(insertion_ctx: &mut InsertionContext) {
    insertion_ctx.solution.required.extend(insertion_ctx.solution.unassigned.iter().map(|(job, _)| job.clone()));
    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
}

pub(crate) fn finalize_insertion_ctx(insertion_ctx: &mut InsertionContext) {
    finalize_unassigned(insertion_ctx, UnassignmentInfo::Unknown);

    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
}

pub(crate) fn apply_insertion_success(insertion_ctx: &mut InsertionContext, success: InsertionSuccess) {
    let is_new_route = insertion_ctx.solution.registry.use_route(&success.context);
    let route_index =
        insertion_ctx.solution.routes.iter().position(|ctx| ctx == &success.context).unwrap_or_else(|| {
            assert!(is_new_route);
            insertion_ctx.solution.routes.push(success.context.deep_copy());
            insertion_ctx.solution.routes.len() - 1
        });

    let route_ctx = insertion_ctx.solution.routes.get_mut(route_index).unwrap();
    let route = route_ctx.route_mut();
    success.activities.into_iter().for_each(|(a, index)| {
        route.tour.insert_at(a, index + 1);
    });

    let job = success.job;
    insertion_ctx.solution.required.retain(|j| *j != job);
    insertion_ctx.solution.unassigned.remove(&job);
    insertion_ctx.problem.goal.accept_insertion(&mut insertion_ctx.solution, route_index, &job);
}

fn apply_insertion_failure(
    insertion_ctx: &mut InsertionContext,
    jobs: Vec<Job>,
    routes: Vec<RouteContext>,
    failure: InsertionFailure,
) {
    // NOTE in most of the cases, it is not needed to reevaluate insertion for all other jobs
    let all_unassignable =
        jobs.len() == insertion_ctx.solution.required.len() && routes.len() == insertion_ctx.solution.routes.len();

    // NOTE this happens when evaluator fails to insert jobs due to lack of routes in registry
    // TODO remove from required only jobs from selected list
    let no_routes_available = failure.job.is_none();

    if let Some(job) = failure.job {
        insertion_ctx.solution.unassigned.insert(job.clone(), UnassignmentInfo::Simple(failure.constraint));
        insertion_ctx.solution.required.retain(|j| *j != job);
    }

    if all_unassignable || no_routes_available {
        let code =
            if all_unassignable { UnassignmentInfo::Unknown } else { UnassignmentInfo::Simple(failure.constraint) };
        finalize_unassigned(insertion_ctx, code);
    }
}

fn finalize_unassigned(insertion_ctx: &mut InsertionContext, code: UnassignmentInfo) {
    let unassigned = &insertion_ctx.solution.unassigned;
    insertion_ctx.solution.required.retain(|job| !unassigned.contains_key(job));
    insertion_ctx.solution.unassigned.extend(insertion_ctx.solution.required.drain(0..).map(|job| (job, code.clone())));
}
