//! This example demonstrates how to add a custom hard constraint to a CVRP variant.
//!
//! Here, we introduce a hardware requirement of a job so that only vehicles with matching
//! hardware capabilities can be used to serve it.
//!
//! Key points of implementation:
//! - adding custom dimension (property) for a job and a vehicle with dedicated [custom_dimension] macro
//! - assigning hardware requirements to the jobs and vehicles by setting custom properties accordingly
//! - adding custom [FeatureConstraint] which reads custom properties from the job and the vehicle and checks whether they match
//!

#[path = "./common/routing.rs"]
mod common;
use crate::common::define_routing_data;

use std::collections::HashSet;
use std::iter::once;
use std::sync::Arc;
use vrp_core::prelude::*;

// Specify two custom properties: one for a job and one for a vehicle
custom_dimension!(JobHardware typeof String);
custom_dimension!(VehicleHardware typeof HashSet<String>);

/// Provides a way to put custom hard constraints on job/activity - vehicle assignment.
struct HardwareConstraint {
    code: ViolationCode,
}

impl FeatureConstraint for HardwareConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            // matching job to route is evaluated before activity matching, so use it to improve search procedure
            MoveContext::Route { route_ctx, job, .. } => {
                let hardware_vehicle = route_ctx.route().actor.vehicle.dimens.get_vehicle_hardware();
                let hardware_job = job.dimens().get_job_hardware();

                match (hardware_job, hardware_vehicle) {
                    (None, _) => None,
                    (Some(hw_job), Some(hw_vehicle)) if hw_vehicle.contains(hw_job) => None,
                    _ => ConstraintViolation::fail(self.code),
                }
            }
            // matching activity to route is called for every possible insertion point in the tour
            // we don't need it here as hard constraint can be validated on route-job level
            MoveContext::Activity { .. } => None,
        }
    }
}

/// Specifies a CVRP problem variant: 4 delivery jobs with demand=1 and 2 vehicles with capacity=2 in each.
fn define_problem(goal: GoalContext, transport: Arc<dyn TransportCost>) -> GenericResult<Problem> {
    // create 4 jobs when second and forth have fridge requirement
    let single_jobs = (1..=4)
        .map(|idx| {
            SingleBuilder::default()
                .id(format!("job{idx}").as_str())
                .demand(Demand::delivery(1))
                .dimension(|dimens| {
                    // all jobs have fridge requirements, but only one vehicle will be allowed to serve them
                    dimens.set_job_hardware("fridge".to_string());
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
                        // vehicle starts at location with index 0 in routing matrix
                        .set_start_location(0)
                        // vehicle should return to location with index 0
                        .set_end_location(0)
                        .build()?,
                )
                .dimension(|dimens| {
                    if idx % 2 == 0 {
                        // only one vehicle has a hardware requirement set to 'fridge'
                        dimens.set_vehicle_hardware(once("fridge".to_string()).collect());
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

/// Defines CVRP variant with a custom constraint as a goal of optimization.
fn define_goal(transport: Arc<dyn TransportCost>) -> GenericResult<GoalContext> {
    let minimize_unassigned = MinimizeUnassignedBuilder::new("min-unassigned").build()?;
    let capacity_feature = CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").build()?;
    let transport_feature = TransportFeatureBuilder::new("min-distance")
        .set_transport_cost(transport)
        .set_time_constrained(false)
        .build_minimize_distance()?;
    // create our custom feature
    let hardware_feature = FeatureBuilder::default()
        .with_name("hardware")
        .with_constraint(HardwareConstraint { code: ViolationCode::default() })
        .build()?;

    // configure goal of optimization
    GoalContextBuilder::with_features(&[minimize_unassigned, transport_feature, capacity_feature, hardware_feature])?
        .build()
}

fn main() -> GenericResult<()> {
    let transport = Arc::new(define_routing_data()?);

    let goal = define_goal(transport.clone())?;
    let problem = Arc::new(define_problem(goal, transport)?);

    let config = VrpConfigBuilder::new(problem.clone()).prebuild()?.with_max_generations(Some(10)).build()?;

    // run the VRP solver and get the best known solution
    let solution = Solver::new(problem, config).solve()?;

    assert_eq!(
        solution.unassigned.len(),
        2,
        "expected two assigned jobs due to hardware requirement and capacity constraints"
    );
    assert_eq!(solution.routes.len(), 1, "only one tour should be there: second vehicle cannot serve hardware jobs");
    assert_eq!(solution.cost, 1050., "unexpected cost - closest to depot jobs should be assigned");

    // simple way to explore the solution, more advanced are available too
    println!(
        "\nIn solution, locations are visited in the following order:\n{:?}\n",
        solution.get_locations().map(Iterator::collect::<Vec<_>>).collect::<Vec<_>>()
    );

    Ok(())
}
