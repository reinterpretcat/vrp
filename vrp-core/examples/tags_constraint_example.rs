//! This example demonstrates how to use a custom tags constraint to restrict
//! job assignments based on vehicle capabilities.
//!
//! In this scenario:
//! - job1 and job2 require a "fragile" tag (can only be served by vehicles with "fragile" tag)
//! - job3 and job4 have no tag requirements (can be served by any vehicle)
//! - vehicle_1 has both "fragile" and "hazmat" tags
//! - vehicle_2 has no tags (so cannot serve fragile jobs)
//!
//! Key points of implementation:
//! - using custom dimension (property) for jobs and vehicles with dedicated custom_dimension macro
//! - assigning tags to the jobs and vehicles by setting custom properties accordingly
//! - using the provided create_tags_feature which validates job-vehicle tag compatibility

#[path = "./common/routing.rs"]
mod common;
use crate::common::define_routing_data;

use std::collections::HashSet;
use std::sync::Arc;
use vrp_core::prelude::*;
use vrp_core::construction::features::{
    CapacityFeatureBuilder, TransportFeatureBuilder, create_tags_feature,
    JobTagsDimension, VehicleTagsDimension,
};
use vrp_core::models::problem::{JobIdDimension, VehicleIdDimension};

/// Specifies a CVRP problem variant with tags constraint: 4 delivery jobs (2 with "fragile" tag) and 2 vehicles
fn define_problem(goal: GoalContext, transport: Arc<dyn TransportCost>) -> GenericResult<Problem> {
    // create 4 jobs: 2 with "fragile" tag, 2 without
    let single_jobs = (1..=4)
        .map(|idx| {
            SingleBuilder::default()
                .id(format!("job{idx}").as_str())
                .demand(Demand::delivery(1))
                .dimension(|dimens| {
                    // jobs 1 and 2 have fragile requirement
                    if idx <= 2 {
                        let mut tags = HashSet::new();
                        tags.insert("fragile".to_string());
                        dimens.set_job_tags(tags);
                    }
                })
                .location(idx)?
                .build_as_job()
        })
        .collect::<Result<Vec<_>, _>>()?;

    // create 2 vehicles
    let vehicles = (1..=2)
        .map(|idx| {
            VehicleBuilder::default()
                .id(format!("v{idx}").as_str())
                .add_detail(
                    VehicleDetailBuilder::default()
                        .set_start_location(0)
                        .set_end_location(0)
                        .build()?,
                )
                .dimension(|dimens| {
                    // only vehicle 1 has "fragile" tag capability
                    if idx == 1 {
                        let mut tags = HashSet::new();
                        tags.insert("fragile".to_string());
                        tags.insert("hazmat".to_string());
                        dimens.set_vehicle_tags(tags);
                    }
                })
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

/// Defines CVRP variant with tags constraint as a goal of optimization.
fn define_goal(transport: Arc<dyn TransportCost>) -> GenericResult<GoalContext> {
    let minimize_unassigned = MinimizeUnassignedBuilder::new("min-unassigned").build()?;
    let capacity_feature = CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").build()?;
    let transport_feature = TransportFeatureBuilder::new("min-distance")
        .set_transport_cost(transport)
        .set_time_constrained(false)
        .build_minimize_distance()?;
    // create our custom tags feature
    let tags_feature = create_tags_feature("tags", ViolationCode::from(2))?;

    // configure goal of optimization
    GoalContextBuilder::with_features(&[minimize_unassigned, transport_feature, capacity_feature, tags_feature])?
        .build()
}

fn main() -> GenericResult<()> {
    let transport = Arc::new(define_routing_data()?);

    let goal = define_goal(transport.clone())?;
    let problem = Arc::new(define_problem(goal, transport)?);

    let config = VrpConfigBuilder::new(problem.clone()).prebuild()?.with_max_generations(Some(10)).build()?;

    // run the VRP solver and get the best known solution
    let solution = Solver::new(problem, config).solve()?;

    println!("\n--- Tags Constraint Example Results ---");
    println!("Total routes: {}", solution.routes.len());
    println!("Assigned jobs: {}", solution.routes.iter().map(|r| r.tour.job_count()).sum::<usize>());
    println!("Unassigned jobs: {}", solution.unassigned.len());
    
    for (idx, route) in solution.routes.iter().enumerate() {
        println!("\nRoute {}:", idx + 1);
        println!("  Vehicle: {}", route.actor.vehicle.dimens.get_vehicle_id().unwrap_or(&"unknown".to_string()));
        println!("  Jobs:");
        for job in route.tour.jobs() {
            println!("    - {}", job.dimens().get_job_id().unwrap_or(&"unknown".to_string()));
        }
    }

    if !solution.unassigned.is_empty() {
        println!("\nUnassigned jobs (failed tag matching):");
        for (job, _) in solution.unassigned.iter() {
            println!("  - {}", job.dimens().get_job_id().unwrap_or(&"unknown".to_string()));
        }
    }

    println!("\nExpected behavior:");
    println!("  - job1 and job2 (fragile) should be assigned to vehicle_1 (has fragile tag)");
    println!("  - job3 and job4 (no tags) can be assigned to any vehicle");
    println!("  - vehicle_2 (no tags) cannot serve job1 or job2");

    Ok(())
}