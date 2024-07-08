//! This example shows how to model Pickup and Delivery with Time Windows (PDPTW) variant where
//! each job is represented by pickup and delivery tasks and has time window constraints.
//!
//! The code below looks almost the same as CVPR example and differs only in jobs/vehicle construction
//! and time constraint configuration.
//!
//! Key points here:
//! - how to create PUDO (pickup and drop off) jobs
//! - how to set time window constraints on jobs and vehicles
//!

#[path = "./common/routing.rs"]
mod common;
use crate::common::define_routing_data;

use std::iter::once;
use std::sync::Arc;
use vrp_core::models::common::TimeWindow;
use vrp_core::prelude::*;

/// Specifies a PDPTW problem variant: two PUDO (pick up/drop off) jobs with demand=1 and 1 vehicle with capacity 1
fn define_problem(goal: GoalContext, transport: Arc<dyn TransportCost + Send + Sync>) -> GenericResult<Problem> {
    // build two PUDO (pick up/drop off) jobs with demand=1 and permissive time windows (just to show API usage)
    let pudos = (1..=2)
        .map(|idx| {
            let location_idx = if idx == 1 { 1 } else { 3 };
            MultiBuilder::default()
                .id(format!("pudo{idx}").as_str())
                .add_job(
                    SingleBuilder::default()
                        .demand(Demand::pudo_pickup(1))
                        .times(vec![TimeWindow::new(0., 1000.)])?
                        .duration(10.)?
                        .location(location_idx)?
                        .build()?,
                )
                .add_job(
                    SingleBuilder::default()
                        .demand(Demand::pudo_delivery(1))
                        .times(vec![TimeWindow::new(0., 1000.)])?
                        .duration(10.)?
                        .location(location_idx + 1)?
                        .build()?,
                )
                .build_as_job()
        })
        .collect::<Result<Vec<_>, _>>()?;

    // define a single vehicle with limited capacity
    let vehicle = VehicleBuilder::default()
        .id("v1".to_string().as_str())
        .add_detail(
            VehicleDetailBuilder::default()
                // vehicle starts at location with index 0 in routing matrix
                .set_start_location(0)
                .set_start_time(0.)
                // vehicle should return to location with index 0
                .set_end_location(0)
                .set_end_time(10000.)
                .build()?,
        )
        // the vehicle has capacity=1, so it is forced to do delivery after each pickup
        .capacity(SingleDimLoad::new(1))
        .build()?;

    ProblemBuilder::default()
        .add_jobs(pudos.into_iter())
        .add_vehicles(once(vehicle))
        .with_goal(goal)
        .with_transport_cost(transport)
        .build()
}

/// Defines PDPTW variant as a goal of optimization.
fn define_goal(transport: Arc<dyn TransportCost + Send + Sync>) -> GenericResult<GoalContext> {
    // configure features needed to model PDPTW
    let minimize_unassigned = MinimizeUnassignedBuilder::new("min-unassigned").build()?;
    let capacity_feature = CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").build()?;
    let transport_feature = TransportFeatureBuilder::new("min-distance")
        .set_transport_cost(transport)
        // enable time constraint (not necessary, default behavior, here for demonstration purpose only)
        .set_time_constrained(true)
        .build_minimize_distance()?;

    // configure goal of optimization
    GoalContextBuilder::with_features(&[minimize_unassigned, transport_feature, capacity_feature])?.build()
}

fn main() -> GenericResult<()> {
    // get routing data, see `./common/routing.rs` for details
    let transport = Arc::new(define_routing_data()?);

    // specify PDPTW variant as problem definition and the goal of optimization
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
    assert_eq!(solution.routes.len(), 1, "one tour should be there");
    assert_eq!(solution.cost, 1105., "unexpected cost (total distance traveled)");

    // simple way to explore the solution, more advanced are available too
    println!(
        "\nIn solution, locations are visited in the following order:\n{:?}\n",
        solution.get_locations().map(Iterator::collect::<Vec<_>>).collect::<Vec<_>>()
    );

    Ok(())
}
