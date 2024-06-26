//! This example shows how to model Capacitated Vehicle Routing Problem (CVRP) variant where multiple vehicles
//! of the same type are constrained only be their capacity.

use std::sync::Arc;
use vrp_core::prelude::*;

fn define_routing_data() -> GenericResult<impl TransportCost + Send + Sync> {
    // define distance/duration matrix (use same data for both)
    // as we have five locations (4 for jobs, 1 for vehicle), we need to define 5x5 matrix, flatten to 1 dimension:
    #[rustfmt::skip]
    let routing_data = vec![
     //  0     1     2     3     4
        0.,  500., 520., 530., 540.,  // 0
        500.,  0.,  30.,  40.,  50.,  // 1
        520., 30.,   0.,  20.,  25.,  // 2
        530., 40.,  20.,   0.,  15.,  // 3
        540., 50.,  25.,  15.,   0.   // 4
    ];
    let (durations, distances) = (routing_data.clone(), routing_data);

    SimpleTransportCost::new(durations, distances)
}

/// Specifies problem variant: 4 delivery jobs with demand=1 and 4 vehicles with capacity=2 in each.
fn define_problem(goal: GoalContext, transport: Arc<dyn TransportCost + Send + Sync>) -> GenericResult<Problem> {
    // create 4 jobs with location indices from 1 to 4
    let single_jobs = (1..=4)
        .map(|idx| {
            SingleBuilder::default()
                .id(format!("job_{idx}").as_str())
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

    // configure goal of optimization
    GoalContextBuilder::with_features(vec![minimize_unassigned, transport_feature, capacity_feature])?
        // the goal is split into global and local parts:
        // - on global level we prefer solutions where:
        //   1. minimum of unassigned jobs
        //   2. minimum distance traveled
        // - on local level, as all jobs have the same weight, we prefer jobs that introduce a minimal distance change
        .set_goal(&["min-unassigned", "min-distance"], &["min-distance"])?
        .build()
}

fn main() -> GenericResult<()> {
    // specify CVRP variant as problem definition and goal of optimization
    let transport = Arc::new(define_routing_data()?);
    let goal = define_goal(transport.clone())?;
    let problem = Arc::new(define_problem(goal, transport)?);

    // build solver config with predefined settings to run 5 secs or 10 generations
    let config = VrpConfigBuilder::new(problem.clone())
        .prebuild()?
        .with_max_time(Some(5))
        .with_max_generations(Some(10))
        .build()?;

    // run solver and get the best known solution.
    let solution = Solver::new(problem, config).solve()?;

    assert!(solution.unassigned.is_empty(), "has unassigned jobs, but all jobs must be assigned");
    assert_eq!(solution.routes.len(), 2, "two tours are expected");
    assert_eq!(solution.cost, 2135., "unexpected cost (total distance traveled)");

    Ok(())
}
