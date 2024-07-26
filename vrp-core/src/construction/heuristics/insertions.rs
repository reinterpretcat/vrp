#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/insertions_test.rs"]
mod insertions_test;

use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::{Actor, Job, JobIdDimension};
use crate::models::solution::Activity;
use crate::models::ViolationCode;
use rosomaxa::prelude::*;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, ControlFlow, Index, Sub};
use std::sync::Arc;
use tinyvec::{TinyVec, TinyVecIterator};

/// Specifies insertion result variant.
#[derive(Debug)]
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

    /// Specifies actor to be used.
    pub actor: Arc<Actor>,
}

impl Debug for InsertionSuccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("cost", &self.cost)
            .field("job", &self.job)
            .field(
                "activities",
                &self
                    .activities
                    .iter()
                    .map(|(a, idx)| {
                        (
                            a.retrieve_job()
                                .and_then(|job| job.dimens().get_job_id().cloned())
                                .unwrap_or("undef".to_string()),
                            *idx,
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .field("actor", self.actor.as_ref())
            .finish()
    }
}

/// Specifies insertion failure.
#[derive(Debug)]
pub struct InsertionFailure {
    /// Failed constraint code.
    pub constraint: ViolationCode,
    /// A flag which signalizes that algorithm should stop trying to insert at next positions.
    pub stopped: bool,
    /// Original job failed to be inserted.
    pub job: Option<Job>,
}

/// Specifies a max size of stack allocated array to be used. If data size exceeds it,
/// then heap allocated vector is used which leads to performance impact.
const COST_DIMENSION: usize = 6;

/// A size of a cost array used by `InsertionCost`.
type CostArray = [Cost; COST_DIMENSION];

/// A lexicographical cost of job's insertion.
#[derive(Clone, Default)]
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
        Self::new(&[Cost::MAX])
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
        (0..size)
            .try_fold(Ordering::Equal, |acc, idx| {
                let left = self.data.get(idx).cloned().unwrap_or_default();
                let right = other.data.get(idx).cloned().unwrap_or_default();

                let result = left.total_cmp(&right);
                match result {
                    Ordering::Equal => ControlFlow::Continue(acc),
                    _ => ControlFlow::Break(result),
                }
            })
            .unwrap_value()
    }
}

impl<'a, B> Add<B> for &'a InsertionCost
where
    B: Borrow<InsertionCost>,
{
    type Output = InsertionCost;

    fn add(self, rhs: B) -> Self::Output {
        let rhs = rhs.borrow();
        let size = self.data.len().max(rhs.data.len());

        (0..size)
            .map(|idx| {
                self.data.get(idx).copied().unwrap_or(Cost::default())
                    + rhs.data.get(idx).copied().unwrap_or(Cost::default())
            })
            .collect()
    }
}

impl<B> Add<B> for InsertionCost
where
    B: Borrow<InsertionCost>,
{
    type Output = InsertionCost;

    fn add(self, rhs: B) -> Self::Output {
        &self + rhs
    }
}

impl<'a, B> Sub<B> for &'a InsertionCost
where
    B: Borrow<InsertionCost>,
{
    type Output = InsertionCost;

    fn sub(self, rhs: B) -> Self::Output {
        let rhs = rhs.borrow();
        let size = self.data.len().max(rhs.data.len());

        (0..size)
            .map(|idx| {
                self.data.get(idx).copied().unwrap_or(Cost::default())
                    - rhs.data.get(idx).copied().unwrap_or(Cost::default())
            })
            .collect()
    }
}

impl<B> Sub<B> for InsertionCost
where
    B: Borrow<InsertionCost>,
{
    type Output = InsertionCost;

    fn sub(self, rhs: B) -> Self::Output {
        &self - rhs
    }
}

impl Debug for InsertionCost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self.data).finish()
    }
}

impl Index<usize> for InsertionCost {
    type Output = Cost;

    fn index(&self, index: usize) -> &Self::Output {
        if index < self.data.len() {
            &self.data[index]
        } else {
            panic!("index out of range: {index}, size is {}", self.data.len())
        }
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
        InsertionHeuristic::new(Box::<PositionInsertionEvaluator>::default())
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
        mut insertion_ctx: InsertionContext,
        job_selector: &(dyn JobSelector + Send + Sync),
        route_selector: &(dyn RouteSelector + Send + Sync),
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionContext {
        prepare_insertion_ctx(&mut insertion_ctx);

        while !insertion_ctx.solution.required.is_empty()
            && !insertion_ctx.environment.quota.as_ref().map_or(false, |q| q.is_reached())
        {
            job_selector.prepare(&mut insertion_ctx);
            route_selector.prepare(&mut insertion_ctx);

            let jobs = job_selector.select(&insertion_ctx).collect::<Vec<_>>();
            let routes = route_selector.select(&insertion_ctx, jobs.as_slice()).collect::<Vec<_>>();

            let result =
                self.insertion_evaluator.evaluate_all(&insertion_ctx, &jobs, &routes, leg_selection, result_selector);

            match result {
                InsertionResult::Success(success) => {
                    apply_insertion_success(&mut insertion_ctx, success);
                }
                InsertionResult::Failure(failure) => {
                    // NOTE copy data to make borrow checker happy
                    let (route_indices, jobs) = copy_selection_data(&insertion_ctx, routes.as_slice(), jobs.as_slice());
                    apply_insertion_failure(&mut insertion_ctx, failure, &route_indices, &jobs);
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
        route_ctx: &RouteContext,
    ) -> Self {
        Self::Success(InsertionSuccess { cost, job, activities, actor: route_ctx.route().actor.clone() })
    }

    /// Creates result which represents insertion failure.
    pub fn make_failure() -> Self {
        Self::make_failure_with_code(ViolationCode::unknown(), false, None)
    }

    /// Creates result which represents insertion failure with given code.
    pub fn make_failure_with_code(code: ViolationCode, stopped: bool, job: Option<Job>) -> Self {
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
                if rhs.constraint == ViolationCode::unknown() {
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
}

impl TryFrom<InsertionResult> for InsertionSuccess {
    type Error = InsertionFailure;

    fn try_from(value: InsertionResult) -> Result<Self, Self::Error> {
        match value {
            InsertionResult::Success(success) => Ok(success),
            InsertionResult::Failure(failure) => Err(failure),
        }
    }
}

pub(crate) fn prepare_insertion_ctx(insertion_ctx: &mut InsertionContext) {
    insertion_ctx.solution.required.extend(insertion_ctx.solution.unassigned.keys().cloned());
    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
}

pub(crate) fn finalize_insertion_ctx(insertion_ctx: &mut InsertionContext) {
    finalize_unassigned(insertion_ctx, UnassignmentInfo::Unknown);

    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
}

pub(crate) fn apply_insertion_success(insertion_ctx: &mut InsertionContext, success: InsertionSuccess) {
    let route_index = if let Some(new_route_ctx) = insertion_ctx.solution.registry.get_route(&success.actor) {
        insertion_ctx.solution.routes.push(new_route_ctx);
        insertion_ctx.solution.routes.len() - 1
    } else {
        insertion_ctx
            .solution
            .routes
            .iter()
            .position(|route_ctx| route_ctx.route().actor == success.actor)
            .expect("registry is out of sync with used routes")
    };

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
    failure: InsertionFailure,
    route_indices: &[usize],
    jobs: &[Job],
) {
    let selected_routes = route_indices.len();
    let selected_jobs = jobs.len();

    // NOTE in most of the cases, it is not needed to reevaluate insertion for all other jobs
    let all_unassignable = selected_jobs == insertion_ctx.solution.required.len()
        && selected_routes == insertion_ctx.solution.routes.len();

    // give a change to promote special jobs which might unblock assignment for other jobs
    // TODO move this a bit up to avoid adding failed jobs to unassigned?
    let failure_handled = insertion_ctx.problem.goal.notify_failure(&mut insertion_ctx.solution, route_indices, jobs);
    if failure_handled {
        return;
    }

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

fn copy_selection_data(
    insertion_ctx: &InsertionContext,
    routes: &[&RouteContext],
    jobs: &[&Job],
) -> (Vec<usize>, Vec<Job>) {
    let route_indices = routes
        .iter()
        .filter_map(|route| insertion_ctx.solution.routes.iter().position(|r| r == *route))
        .collect::<Vec<_>>();
    let jobs = jobs.iter().map(|&job| job.clone()).collect::<Vec<_>>();

    (route_indices, jobs)
}
