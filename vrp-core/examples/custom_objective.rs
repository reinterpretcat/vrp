//! This example demonstrates how to add a job priority feature which prioritized assignment of
//! some jobs over others. It is based on closed (no return to a depot) CVRP variant.
//!
//! Here, we assign a top priority to a few jobs, so that we expect them to be preferred for assignment.
//!
//! Key points here:
//! - how to define custom objective function with [FeatureObjective] and how to use it within the solver's framework
//! - how to use state management capabilities for custom fitness calculation optimization with [FeatureState]
//!

#[path = "./common/routing.rs"]
mod common;

use crate::common::define_routing_data;
use std::iter::once;
use std::sync::Arc;

use vrp_core::prelude::*;

// a property for the job priority: true if it is a high prio job
custom_dimension!(JobPriority typeof bool);
// a state property to keep track of solution fitness for the priority feature
custom_solution_state!(PriorityFitness typeof Cost);

/// Provides a way to guide the search to achieve a goal of optimization considering priority jobs
/// assignment as the most important aspect.
struct PriorityObjective;

impl FeatureObjective for PriorityObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        // estimate global objective fitness: get the number of jobs with top priority in the solution

        let solution_ctx = &solution.solution;
        // read it from state if present: this is a performance optimization as fitness function
        // is called frequently. You don't need the state if fitness can be calculated very fast
        solution_ctx.state.get_priority_fitness().copied().unwrap_or_else(|| calculate_solution_fitness(solution_ctx))
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        // estimate local objective fitness: map job priority to cost penalty
        match move_ctx {
            MoveContext::Route { job, .. } => estimate_job_cost(job),
            MoveContext::Activity { .. } => 0.,
        }
    }
}

/// Provides a way to cache some calculations to speed up overall search procedure.
struct PriorityState;

impl FeatureState for PriorityState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _job: &Job) {
        // normally, delegate state updating to accept_route_state
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
    }

    fn accept_route_state(&self, _route_ctx: &mut RouteContext) {
        // do nothing for this example
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        // once the solution state is fixed, calculate the feature's fitness and store it in state
        let fitness = calculate_solution_fitness(solution_ctx);
        solution_ctx.state.set_priority_fitness(fitness);
    }
}

/// Estimates an impact of a job in the solution:
/// As the solver tries to minimize each objective:
///  - return 1 if the job is not prioritized
///  - return 0 if the job is a top-prio
fn estimate_job_cost(job: &Job) -> Cost {
    job.dimens().get_job_priority().filter(|&is_high_prio| *is_high_prio).map_or(1., |_| 0.)
}

/// Estimates solution fitness: iterate over every inserted job in the solution,
/// estimate the cost of its insertion and sum it.
fn calculate_solution_fitness(solution_ctx: &SolutionContext) -> Cost {
    solution_ctx.routes.iter().flat_map(|route_ctx| route_ctx.route().tour.jobs()).map(estimate_job_cost).sum::<Cost>()
}

/// Specifies four delivery jobs with demand=1 (two of them are with top priority) and a single vehicle
/// with capacity=2 which doesn't need to return to the depot.
fn define_problem(goal: GoalContext, transport: Arc<dyn TransportCost + Send + Sync>) -> GenericResult<Problem> {
    // create 4 jobs where two are having top prio
    let single_jobs = (1..=4)
        .map(|idx| {
            SingleBuilder::default()
                .id(format!("job{idx}").as_str())
                .demand(Demand::delivery(1))
                .dimension(|dimens| {
                    // mark two jobs as top priority (2 and 4 locations)
                    dimens.set_job_priority(idx % 2 == 0);
                })
                .location(idx)?
                .build_as_job()
        })
        .collect::<Result<Vec<_>, _>>()?;

    // define a single vehicle with limited capacity which doesn't need to return back to the depot
    let vehicle = VehicleBuilder::default()
        .id("v1".to_string().as_str())
        .add_detail(VehicleDetailBuilder::default().set_start_location(0).build()?)
        // only two jobs can be served by the vehicle
        .capacity(SingleDimLoad::new(2))
        .build()?;

    ProblemBuilder::default()
        .add_jobs(single_jobs.into_iter())
        .add_vehicles(once(vehicle))
        .with_goal(goal)
        .with_transport_cost(transport)
        .build()
}

/// Defines optimization goal as CVRP variant with a priority objective function on top.
fn define_goal(transport: Arc<dyn TransportCost + Send + Sync>) -> GenericResult<GoalContext> {
    let minimize_unassigned = MinimizeUnassignedBuilder::new("min-unassigned").build()?;
    let capacity_feature = CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").build()?;
    let transport_feature = TransportFeatureBuilder::new("min-distance")
        .set_transport_cost(transport)
        .set_time_constrained(false)
        .build_minimize_distance()?;

    // create our custom feature that consists of an objective and a state
    let priority_feature = FeatureBuilder::default()
        .with_name("maximize-priority")
        .with_objective(PriorityObjective)
        .with_state(PriorityState)
        .build()?;

    // configure goal of optimization
    GoalContextBuilder::with_features(vec![priority_feature, minimize_unassigned, transport_feature, capacity_feature])?
        .set_goal(
            // on global level, prefer more maximized jobs over anything else
            &["maximize-priority", "min-unassigned", "min-distance"],
            // on local level, prefer it over min-distance as distance is less important
            &["maximize-priority", "min-distance"],
        )?
        .build()
}

fn main() -> GenericResult<()> {
    let transport = Arc::new(define_routing_data()?);

    let goal = define_goal(transport.clone())?;
    let problem = Arc::new(define_problem(goal, transport)?);

    let config = VrpConfigBuilder::new(problem.clone()).prebuild()?.with_max_generations(Some(10)).build()?;

    // run the VRP solver and get the best known solution
    let solution = Solver::new(problem, config).solve()?;

    assert_eq!(solution.unassigned.len(), 2, "expected two assigned jobs due to capacity constraint");
    assert_eq!(solution.routes.len(), 1, "only one tour should be there");
    assert_eq!(
        solution.get_locations().map(Iterator::collect::<Vec<_>>).collect::<Vec<_>>(),
        vec![vec![0, 2, 4]],
        "tour doesn't serve only top-prio jobs"
    );
    assert_eq!(solution.cost, 545., "unexpected cost - closest to depot jobs should be assigned");

    Ok(())
}
