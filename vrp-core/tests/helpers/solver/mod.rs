use crate::algorithms::geometry::Point;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_transport;
use crate::helpers::models::domain::test_random;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{create_route_with_activities, test_activity_with_job};
use crate::models::common::Location;
use crate::models::problem::*;
use crate::models::solution::{Registry, Route};
use crate::models::{Problem, Solution};
use crate::solver::{create_elitism_population, RefinementContext};
use rosomaxa::prelude::Environment;
use std::sync::Arc;

mod mutation;
pub use self::mutation::*;

pub fn create_default_refinement_ctx(problem: Arc<Problem>) -> RefinementContext {
    let environment = Arc::new(Environment::default());
    RefinementContext::new(
        problem.clone(),
        create_elitism_population(problem.objective.clone(), environment.clone()),
        environment,
        None,
    )
}

/// Generates matrix routes. See `generate_matrix_routes`.
pub fn generate_matrix_routes_with_defaults(rows: usize, cols: usize, is_open_vrp: bool) -> (Problem, Solution) {
    generate_matrix_routes(
        rows,
        cols,
        is_open_vrp,
        |id, location| test_single_with_id_and_location(id, location),
        |v| v,
        |data| (data.clone(), data),
    )
}

/// Generates problem and solution which has routes distributed uniformly, e.g.:
/// r0 r1 r2 r3
/// -----------
/// 0  4   8 12
/// 1  5   9 13
/// 2  6  10 14
/// 3  7  11 15
pub fn generate_matrix_routes(
    rows: usize,
    cols: usize,
    is_open_vrp: bool,
    job_factory: impl Fn(&str, Option<Location>) -> Arc<Single>,
    vehicle_modify: impl Fn(Vehicle) -> Vehicle,
    matrix_modify: impl Fn(Vec<f64>) -> (Vec<f64>, Vec<f64>),
) -> (Problem, Solution) {
    let fleet = Arc::new(
        FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(
                (0..cols)
                    .map(|i| {
                        vehicle_modify(Vehicle {
                            details: vec![VehicleDetail {
                                end: if is_open_vrp { None } else { test_vehicle_detail().end },
                                ..test_vehicle_detail()
                            }],
                            ..test_vehicle_with_id(i.to_string().as_str())
                        })
                    })
                    .collect(),
            )
            .build(),
    );
    let registry = Registry::new(&fleet, test_random());

    let mut routes: Vec<Route> = Default::default();
    let mut jobs: Vec<Job> = Default::default();

    (0..cols).for_each(|i| {
        routes.push(create_route_with_activities(&fleet, i.to_string().as_str(), Default::default()));
        (0..rows).for_each(|j| {
            let index = i * rows + j;

            let single = job_factory(["c".to_string(), index.to_string()].concat().as_str(), Some(index));
            let route = routes.get_mut(i).unwrap();
            jobs.push(Job::Single(single.clone()));

            let mut activity = test_activity_with_job(single);
            activity.place.location = index;

            route.tour.insert_last(activity);
        });
    });

    let (durations, distances) = matrix_modify(generate_matrix_from_sizes(rows, cols));

    let matrix_data = MatrixData::new(0, None, durations, distances);
    let transport = create_matrix_transport_cost(vec![matrix_data]).unwrap();
    let jobs = Jobs::new(&fleet, jobs, &transport);

    let problem = Problem {
        fleet,
        jobs: Arc::new(jobs),
        locks: vec![],
        // TODO pass transport costs with constraint
        constraint: Arc::new(create_constraint_pipeline_with_transport()),
        activity: Arc::new(TestActivityCost::default()),
        transport,
        objective: Arc::new(ObjectiveCost::default()),
        extras: Arc::new(Default::default()),
    };

    let solution = Solution { registry, routes, unassigned: Default::default(), extras: Arc::new(Default::default()) };

    (problem, solution)
}

fn generate_matrix_from_sizes(rows: usize, cols: usize) -> Vec<f64> {
    let size = cols * rows;
    let mut data = vec![0.; size * size];

    (0..size).for_each(|i| {
        let (left1, right1) = (i / rows, i % rows);
        ((i + 1)..size).for_each(|j| {
            let (left2, right2) = (j / rows, j % rows);
            let left_delta = left1 as f64 - left2 as f64;
            let right_delta = right1 as f64 - right2 as f64;

            let value = (left_delta * left_delta + right_delta * right_delta).sqrt();

            let sym_j = (j as i32 + (j as i32 - i as i32) * (size as i32 - 1)) as usize;

            data[i * size + j] = value;
            data[i * size + sym_j] = value;
        });
    });

    data
}

pub fn generate_matrix_distances_from_points(points: &[Point]) -> Vec<f64> {
    points.iter().cloned().flat_map(|p_a| points.iter().map(move |p_b| p_a.distance_to_point(p_b))).collect()
}
