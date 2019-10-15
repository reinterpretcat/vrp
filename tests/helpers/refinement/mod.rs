use crate::helpers::construction::constraints::create_constraint_pipeline;
use crate::helpers::models::problem::{
    empty_costs, test_driver_with_costs, test_single_job_with_id_and_location, test_vehicle_with_id, TestActivityCost,
    TestTransportCost,
};
use crate::helpers::models::solution::{create_route_with_activities, test_tour_activity_with_job};
use crate::models::problem::{Fleet, Job, Jobs, MatrixTransportCost, Vehicle};
use crate::models::solution::{Actor, Registry, Route};
use crate::models::{Problem, Solution};
use std::sync::Arc;

/// Generates problem and solution which has routes distributed uniformly, e.g.:
/// r0 r1 r2 r3
/// -----------
/// 0  4   8 12
/// 1  5   9 13
/// 2  6  10 14
/// 3  7  11 15
pub fn generate_matrix_routes(rows: usize, cols: usize) -> (Problem, Solution) {
    let drivers = vec![test_driver_with_costs(empty_costs())];
    let vehicles: Vec<Vehicle> = (0..cols).map(|i| test_vehicle_with_id(i.to_string().as_str())).collect();
    let fleet = Arc::new(Fleet::new(drivers, vehicles));
    let registry = Registry::new(&fleet);

    //let actors: Vec<Arc<Actor>> = registry.all().collect();
    let mut routes: Vec<Route> = Default::default();
    let mut jobs: Vec<Arc<Job>> = Default::default();

    (0..cols).for_each(|i| {
        routes.push(create_route_with_activities(&fleet, i.to_string().as_str(), Default::default()));
        (0..rows).for_each(|j| {
            let index = i * rows + j;

            let single = Arc::new(test_single_job_with_id_and_location(index.to_string().as_str(), Some(index)));
            let mut route = routes.get_mut(i).unwrap();
            jobs.push(single.clone());
            route.tour.insert_last(test_tour_activity_with_job(single));
        });
    });

    let matrix = generate_matrix(rows, cols, 1000.);
    let transport = Arc::new(MatrixTransportCost::new(matrix.clone(), matrix));
    let jobs = Jobs::new(&fleet, jobs, transport.as_ref());

    let problem = Problem {
        fleet,
        jobs: Arc::new(jobs),
        locks: vec![],
        constraint: Arc::new(create_constraint_pipeline()),
        activity: Arc::new(TestActivityCost::new()),
        transport,
        extras: Arc::new(Default::default()),
    };

    let solution = Solution { registry, routes, unassigned: Default::default(), extras: Arc::new(Default::default()) };

    (problem, solution)
}

fn generate_matrix(rows: usize, cols: usize, scale: f64) -> Vec<Vec<f64>> {
    unimplemented!()
}
