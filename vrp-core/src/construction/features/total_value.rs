//! Calculates a total value of the served jobs.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/total_value_test.rs"]
mod total_value_test;

use super::*;
use crate::models::problem::Actor;
use crate::utils::Either;

/// Specifies a job value function which takes into account actor and job.
pub type ActorValueFn = Arc<dyn Fn(&Actor, &Job) -> Float + Send + Sync>;
/// Specifies a job value function which takes into account only a job.
pub type SimpleValueFn = Arc<dyn Fn(&Job) -> Float + Send + Sync>;
/// Specifies a job value reader as a variant of two functions.
pub type JobReadValueFn = Either<SimpleValueFn, ActorValueFn>;
/// Specifies a job write value.
pub type JobWriteValueFn = Arc<dyn Fn(Job, Float) -> Job + Send + Sync>;
/// A job value estimation function.
type EstimateValueFn = Arc<dyn Fn(&RouteContext, &Job) -> Float + Send + Sync>;

custom_tour_state!(TotalValue typeof Cost);

/// Maximizes a total value of served jobs.
pub fn create_maximize_total_job_value_feature(
    name: &str,
    job_read_value_fn: JobReadValueFn,
    job_write_value_fn: JobWriteValueFn,
    merge_code: ViolationCode,
) -> Result<Feature, GenericError> {
    let estimate_value_fn: EstimateValueFn = Arc::new({
        let job_read_value_fn = job_read_value_fn.clone();
        let sign = -1.;
        move |route_ctx, job| {
            sign * match &job_read_value_fn {
                JobReadValueFn::Left(left_fn) => (left_fn)(job),
                JobReadValueFn::Right(right_fn) => (right_fn)(route_ctx.route().actor.as_ref(), job),
            }
        }
    });

    FeatureBuilder::default()
        .with_name(name)
        .with_objective(MaximizeTotalValueObjective { estimate_value_fn: estimate_value_fn.clone() })
        .with_constraint(MaximizeTotalValueConstraint { merge_code, job_read_value_fn, job_write_value_fn })
        .with_state(MaximizeTotalValueState { estimate_value_fn })
        .build()
}

struct MaximizeTotalValueObjective {
    estimate_value_fn: EstimateValueFn,
}

/// Sums the value a route serves over its jobs.
fn route_value(route_ctx: &RouteContext, estimate_value_fn: &EstimateValueFn) -> Cost {
    route_ctx.route().tour.jobs().map(|job| (estimate_value_fn)(route_ctx, job)).sum()
}

impl FeatureObjective for MaximizeTotalValueObjective {
    /// Reads the per-route sum `MaximizeTotalValueState` keeps rather than walking every job: the
    /// population compares stored solutions after every generation, each comparison starts at this
    /// layer, and on a problem whose jobs carry no value at all it ties every time. Summing the jobs
    /// there was the largest single cost of a generation's bookkeeping on run62 (1423 jobs). A route
    /// whose state was never accepted is summed directly.
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution.solution.routes.iter().fold(0., |acc, route_ctx| {
            acc + route_ctx
                .state()
                .get_total_value()
                .copied()
                .unwrap_or_else(|| route_value(route_ctx, &self.estimate_value_fn))
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => (self.estimate_value_fn)(route_ctx, job),
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct MaximizeTotalValueConstraint {
    merge_code: ViolationCode,
    job_read_value_fn: JobReadValueFn,
    job_write_value_fn: JobWriteValueFn,
}

impl FeatureConstraint for MaximizeTotalValueConstraint {
    fn evaluate(&self, _: &MoveContext<'_>) -> Option<ConstraintViolation> {
        None
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match &self.job_read_value_fn {
            JobReadValueFn::Left(left_fn) => {
                let source_value = (left_fn)(&source);
                let candidate_value = (left_fn)(&candidate);
                let new_value = source_value + candidate_value;

                Ok(if new_value != source_value { (self.job_write_value_fn)(source, new_value) } else { source })
            }
            JobReadValueFn::Right(_) => Err(self.merge_code),
        }
    }
}

/// Keeps the value a route serves in its state. The value depends on which jobs the route holds and
/// on its actor, never on the schedule, so it is exact whatever order the feature states run in.
struct MaximizeTotalValueState {
    estimate_value_fn: EstimateValueFn,
}

impl FeatureState for MaximizeTotalValueState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let value = route_value(route_ctx, &self.estimate_value_fn);
        route_ctx.state_mut().set_total_value(value);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx
            .routes
            .iter_mut()
            .filter(|route_ctx| route_ctx.is_stale())
            .for_each(|route_ctx| self.accept_route_state(route_ctx));
    }
}
