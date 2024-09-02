use crate::algorithms::geometry::Point;
use crate::construction::features::TransportFeatureBuilder;
use crate::construction::heuristics::MoveContext;
use crate::helpers::models::domain::{test_random, TestGoalContextBuilder};
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder};
use crate::models::common::{Cost, Location};
use crate::models::problem::*;
use crate::models::solution::{Activity, Registry, Route};
use crate::models::*;
use crate::models::{Problem, Solution};
use crate::solver::{create_elitism_population, RefinementContext};
use rosomaxa::evolution::TelemetryMode;
use rosomaxa::prelude::{Environment, Float};
use std::sync::Arc;

mod mutation;
pub use self::mutation::*;

pub fn create_default_refinement_ctx(problem: Arc<Problem>) -> RefinementContext {
    let environment = Arc::new(Environment::default());
    RefinementContext::new(
        problem.clone(),
        Box::new(create_elitism_population(problem.goal.clone(), environment.clone())),
        TelemetryMode::None,
        environment,
    )
}

/// Generates matrix routes. See `generate_matrix_routes`.
pub fn generate_matrix_routes_with_defaults(
    rows: usize,
    cols: usize,
    scale: Float,
    is_open_vrp: bool,
) -> (Problem, Solution) {
    generate_matrix_routes(
        rows,
        cols,
        is_open_vrp,
        |transport, activity, _| {
            TestGoalContextBuilder::default()
                .add_feature(
                    TransportFeatureBuilder::new("transport")
                        .set_violation_code(ViolationCode(1))
                        .set_transport_cost(transport)
                        .set_activity_cost(activity)
                        .build_minimize_cost()
                        .unwrap(),
                )
                .build()
        },
        |id, location| TestSingleBuilder::default().id(id).location(location).build_shared(),
        |v| v,
        |data| {
            let data = data.into_iter().map(|i| (i * scale).round() as i32).collect::<Vec<_>>();
            (data.clone(), data)
        },
    )
}

/// Generates matrix distances from points. Please note that the distances are rounded.
pub fn generate_matrix_distances_from_points(points: &[Point], scale: Float) -> Vec<i32> {
    points
        .iter()
        .cloned()
        .flat_map(|p_a| points.iter().map(move |p_b| (p_a.distance_to_point(p_b) * scale).round() as i32))
        .collect()
}

pub fn generate_matrix_routes_with_disallow_list(
    rows: usize,
    cols: usize,
    is_open_vrp: bool,
    disallowed_pairs: Vec<(&str, &str)>,
) -> (Problem, Solution) {
    let disallowed_pairs =
        disallowed_pairs.into_iter().map(|(prev, next)| (prev.to_string(), next.to_string())).collect();

    generate_matrix_routes(
        rows,
        cols,
        is_open_vrp,
        move |transport, activity, _| {
            TestGoalContextBuilder::empty()
                .add_feature(
                    TransportFeatureBuilder::new("transport")
                        .set_violation_code(ViolationCode(1))
                        .set_transport_cost(transport)
                        .set_activity_cost(activity)
                        .build_minimize_cost()
                        .unwrap(),
                )
                .add_feature(
                    FeatureBuilder::default()
                        .with_name("leg")
                        .with_constraint(LegFeatureConstraint { ignore: "cX".to_string(), disallowed_pairs })
                        .build()
                        .unwrap(),
                )
                .build()
        },
        |id, location| TestSingleBuilder::default().id(id).location(location).build_shared(),
        |v| v,
        |data| {
            let data = data.into_iter().map(|i| i.round() as i32).collect::<Vec<_>>();
            (data.clone(), data)
        },
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
    goal_factory: impl FnOnce(Arc<dyn TransportCost>, Arc<dyn ActivityCost>, &Extras) -> GoalContext,
    job_factory: impl Fn(&str, Option<Location>) -> Arc<Single>,
    vehicle_modify: impl Fn(Vehicle) -> Vehicle,
    matrix_modify: impl Fn(Vec<Float>) -> (Vec<i32>, Vec<i32>),
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

    let extras = Extras::default();

    (0..cols).for_each(|i| {
        routes.push(RouteBuilder::default().with_vehicle(fleet.as_ref(), i.to_string().as_str()).build());
        (0..rows).for_each(|j| {
            let index = i * rows + j;

            let single = job_factory(["c".to_string(), index.to_string()].concat().as_str(), Some(index));
            let route = routes.get_mut(i).unwrap();
            jobs.push(Job::Single(single.clone()));

            let mut activity = ActivityBuilder::default().job(Some(single)).build();
            activity.place.location = index;

            route.tour.insert_last(activity);
        });
    });

    let (durations, distances) = matrix_modify(generate_matrix_from_sizes(rows, cols));

    let matrix_data = MatrixData::new(0, None, durations, distances);
    let transport = create_matrix_transport_cost(vec![matrix_data]).unwrap();
    let activity = Arc::new(TestActivityCost::default());
    let jobs = Jobs::new(&fleet, jobs, transport.as_ref());

    let problem = Problem {
        fleet,
        jobs: Arc::new(jobs),
        locks: vec![],
        // TODO: we should pass the same transport costs, but the tests were written assuming default one
        goal: Arc::new(goal_factory(TestTransportCost::new_shared(), activity.clone(), &extras)),
        activity,
        transport,
        extras: Arc::new(extras),
    };

    let solution =
        Solution { cost: Cost::default(), registry, routes, unassigned: Default::default(), telemetry: None };

    (problem, solution)
}

fn generate_matrix_from_sizes(rows: usize, cols: usize) -> Vec<Float> {
    let size = cols * rows;
    let mut data = vec![0.; size * size];

    (0..size).for_each(|i| {
        let (left1, right1) = (i / rows, i % rows);
        ((i + 1)..size).for_each(|j| {
            let (left2, right2) = (j / rows, j % rows);
            let left_delta = left1 as Float - left2 as Float;
            let right_delta = right1 as Float - right2 as Float;

            let value = (left_delta * left_delta + right_delta * right_delta).sqrt();

            let sym_j = (j as i32 + (j as i32 - i as i32) * (size as i32 - 1)) as usize;

            data[i * size + j] = value;
            data[i * size + sym_j] = value;
        });
    });

    data
}

struct LegFeatureConstraint {
    ignore: String,
    disallowed_pairs: Vec<(String, String)>,
}

impl FeatureConstraint for LegFeatureConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { .. } => None,
            MoveContext::Activity { activity_ctx, .. } => {
                let retrieve_job_id = |activity: Option<&Activity>| {
                    activity.as_ref().and_then(|next| {
                        next.retrieve_job()
                            .and_then(|job| job.dimens().get_job_id().cloned())
                            .or_else(|| Some(self.ignore.clone()))
                    })
                };

                retrieve_job_id(Some(activity_ctx.prev)).zip(retrieve_job_id(activity_ctx.next)).and_then(
                    |(prev, next)| {
                        let is_disallowed = self.disallowed_pairs.iter().any(|(p_prev, p_next)| {
                            let is_left_match = p_prev == &prev || p_prev == &self.ignore;
                            let is_right_match = p_next == &next || p_next == &self.ignore;

                            is_left_match && is_right_match
                        });

                        if is_disallowed {
                            ConstraintViolation::skip(ViolationCode(7))
                        } else {
                            None
                        }
                    },
                )
            }
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}
