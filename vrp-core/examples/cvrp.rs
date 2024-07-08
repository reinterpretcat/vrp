//! This example shows how to model Capacitated Vehicle Routing Problem (CVRP) variant where multiple vehicles
//! of the same type are constrained only be their capacity and job demand.
//!
//! Key points here:
//! - how to create jobs, vehicles and define [Problem]
//! - how to define a goal of optimization considering capacity/demand constraints and distance minimization
//! - how to define routing matrix
//! - how to compose all building blocks together
//! - how to run the solver
//!

#[path = "./common/routing.rs"]
mod common;
use crate::common::define_routing_data;

use std::sync::Arc;
use vrp_core::prelude::*;

/// Specifies a CVRP problem variant: 4 delivery jobs with demand=1 and 4 vehicles with capacity=2 in each.
fn define_problem(goal: GoalContext, transport: Arc<dyn TransportCost + Send + Sync>) -> GenericResult<Problem> {
    // create 4 jobs with location indices from 1 to 4
    let single_jobs = (1..=4)
        .map(|idx| {
            SingleBuilder::default()
                .id(format!("job{idx}").as_str())
                // each job is delivery job with demand=1
                .demand(Demand::delivery(1))
                // job has location, which is an index in routing matrix
                .location(idx)?
                .build_as_job()
        })
        .collect::<Result<Vec<_>, _>>()?;

    // create 4 vehicles
    let vehicles = (1..=4)
        .map(|idx| {
            VehicleBuilder::default()
                .id(format!("v{idx}").as_str())
                .add_detail(
                    VehicleDetailBuilder::default()
                        // vehicle starts at location with index 0 in routing matrix
                        .set_start_location(0)
                        // vehicle should return to location with index 0
                        .set_end_location(0)
                        .build()?,
                )
                // each vehicle has capacity=2, so it can serve at most 2 jobs
                .capacity(SingleDimLoad::new(2))
                .build()
        })
        .collect::<Result<Vec<_>, _>>()?;

    ProblemBuilder::default()
        .add_jobs(single_jobs.into_iter())
        .add_vehicles(vehicles.into_iter())
        .with_goal(goal)
        .with_transport_cost(transport)
        .build()
}

/// Defines CVRP variant as a goal of optimization.
fn define_goal(transport: Arc<dyn TransportCost + Send + Sync>) -> GenericResult<GoalContext> {
    // configure features needed to model CVRP
    let minimize_unassigned = MinimizeUnassignedBuilder::new("min-unassigned").build()?;
    let capacity_feature = CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").build()?;
    let transport_feature = TransportFeatureBuilder::new("min-distance")
        .set_transport_cost(transport)
        // explicitly opt-out from time constraint on vehicles/jobs
        .set_time_constrained(false)
        .build_minimize_distance()?;

    // configure goal of optimization: features with objectives are read from ordered feature list. Here we have:
    //   1. minimum of unassigned jobs as the main objective
    //   2. minimum distance traveled
    GoalContextBuilder::with_features(&[minimize_unassigned, transport_feature, capacity_feature])?.build()
}

fn main() -> GenericResult<()> {
    // get routing data, see `./common/routing.rs` for details
    let transport = Arc::new(define_routing_data()?);

    // specify CVRP variant as problem definition and the goal of optimization
    let goal = define_goal(transport.clone())?;
    let problem = Arc::new(define_problem(goal, transport)?);

    // build a solver config with the predefined settings to run 5 secs or 10 generations at most
    let config = VrpConfigBuilder::new(problem.clone())
        .prebuild()?
        .with_max_time(Some(5))
        .with_max_generations(Some(10))
        .build()?;

    // run the VRP solver and get the best known solution
    let solution = Solver::new(problem, config).solve()?;

    assert!(solution.unassigned.is_empty(), "has unassigned jobs, but all jobs must be assigned");
    assert_eq!(solution.routes.len(), 2, "two tours are expected");
    assert_eq!(solution.cost, 2135., "unexpected cost (total distance traveled)");

    // simple way to explore the solution, more advanced are available too
    println!(
        "\nIn solution, locations are visited in the following order:\n{:?}\n",
        solution.get_locations().map(Iterator::collect::<Vec<_>>).collect::<Vec<_>>()
    );

    Ok(())
}
