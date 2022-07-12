use super::*;
use crate::construction::constraints::*;
use crate::helpers::construction::constraints::{create_simple_demand, create_simple_dynamic_demand};
use crate::helpers::models::problem::*;
use crate::models::common::*;
use crate::models::problem::*;
use crate::models::solution::Place;
use crate::models::{Lock, LockDetail, LockOrder, LockPosition, Problem};
use rosomaxa::prelude::Environment;

type JobData = (Option<Location>, (f64, f64), Duration, i32);
type VehicleData = (i32, (Location, Option<f64>, Option<f64>), Option<(Location, Option<f64>, Option<f64>)>);
type ActivityData = (Location, Duration, (Timestamp, Timestamp), Arc<Single>);
type RouteData<'a> = Vec<(&'a str, Location, Duration, (Timestamp, Timestamp), usize)>;
type LockData<'a> = (&'a str, LockOrder, LockPosition, Vec<&'a str>);

fn create_test_problem(
    singles: Vec<(&str, JobData)>,
    multies: Vec<(&str, Vec<JobData>)>,
    vehicles: Vec<(&str, VehicleData)>,
    locks: Vec<LockData>,
) -> Problem {
    let create_single = |id: &str, (location, (tw_start, tw_end), duration, demand), is_multi| {
        SingleBuilder::default()
            .id(id)
            .location(location)
            .duration(duration)
            .times(vec![TimeWindow::new(tw_start, tw_end)])
            .demand(if is_multi { create_simple_dynamic_demand(demand) } else { create_simple_demand(demand) })
            .build()
    };

    let jobs = singles
        .into_iter()
        .map(|(id, data)| Job::Single(Arc::new(create_single(id, data, false))))
        .chain(multies.into_iter().map(|(id, singles)| {
            let singles = singles.into_iter().map(|data| Arc::new(create_single(id, data, true))).collect();
            Job::Multi(test_multi_with_id(id, singles))
        }))
        .collect::<Vec<_>>();

    let vehicles = vehicles
        .into_iter()
        .map(|(id, (capacity, (start_location, start_earliest, start_latest), end))| {
            VehicleBuilder::default()
                .id(id)
                .details(vec![VehicleDetail {
                    start: Some(VehiclePlace {
                        location: start_location,
                        time: TimeInterval { earliest: start_earliest, latest: start_latest },
                    }),
                    end: end.map(|(end_location, end_earliest, end_latest)| VehiclePlace {
                        location: end_location,
                        time: TimeInterval { earliest: end_earliest, latest: end_latest },
                    }),
                }])
                .capacity(capacity)
                .build()
        })
        .collect::<Vec<_>>();

    let fleet = Arc::new(FleetBuilder::default().add_driver(test_driver()).add_vehicles(vehicles).build());
    let transport = TestTransportCost::new_shared();
    let activity = Arc::new(SimpleActivityCost::default());

    let locks = locks
        .into_iter()
        .map(|(vehicle_id, order, position, job_ids)| {
            let vehicle_id = vehicle_id.to_string();
            Arc::new(Lock {
                condition: Arc::new(move |actor| *actor.vehicle.dimens.get_id().unwrap() == vehicle_id),
                details: vec![LockDetail {
                    order,
                    position,
                    jobs: job_ids.iter().map(|job_id| get_job_by_id(jobs.iter().cloned(), job_id)).collect(),
                }],
                is_lazy: false,
            })
        })
        .collect::<Vec<_>>();

    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(Arc::new(TransportConstraintModule::new(
        transport.clone(),
        activity.clone(),
        Arc::new(|_| (None, None)),
        1,
        2,
        3,
    )));
    constraint.add_module(Arc::new(StrictLockingModule::new(&fleet, locks.as_slice(), 4)));
    constraint.add_module(Arc::new(CapacityConstraintModule::<SingleDimLoad>::new(5)));

    Problem {
        fleet: fleet.clone(),
        jobs: Arc::new(Jobs::new(&fleet, jobs, &transport)),
        locks,
        constraint: Arc::new(constraint),
        activity,
        transport,
        objective: Arc::new(ProblemObjective::default()),
        extras: Arc::new(Default::default()),
    }
}

fn create_test_insertion_ctx(problem: Arc<Problem>) -> InsertionContext {
    InsertionContext::new_empty(problem, Arc::new(Environment::default()))
}

fn add_new_route(insertion_ctx: &mut InsertionContext, vehicle_id: &str, activities: Vec<ActivityData>) {
    let actor = get_test_actor_from_fleet(insertion_ctx.problem.fleet.as_ref(), vehicle_id);

    let mut route_ctx = RouteContext::new(actor);
    let tour = &mut route_ctx.route_mut().tour;

    activities.into_iter().for_each(|(location, duration, (tw_start, tw_end), single)| {
        tour.insert_last(Activity {
            place: Place { location, duration, time: TimeWindow::new(tw_start, tw_end) },
            schedule: Schedule::new(0., 0.),
            job: Some(single),
            commute: None,
        });
    });

    insertion_ctx.problem.constraint.accept_route_state(&mut route_ctx);

    insertion_ctx.solution.routes.push(route_ctx);
}

fn add_routes(insertion_ctx: &mut InsertionContext, routes: Vec<(&str, RouteData)>) {
    let problem = insertion_ctx.problem.clone();
    routes.into_iter().for_each(|(vehicle_id, activities)| {
        add_new_route(
            insertion_ctx,
            vehicle_id,
            activities
                .into_iter()
                .map(|(job_id, location, duration, (s, e), index)| {
                    (location, duration, (s, e), get_as_single(&problem, job_id, index))
                })
                .collect(),
        );
    });
}

fn get_job_by_id<T: Iterator<Item = Job>>(jobs: T, job_id: &str) -> Job {
    let mut jobs = jobs;
    jobs.find(|job| job.dimens().get_id().unwrap() == job_id).unwrap()
}

fn get_as_single(problem: &Problem, job_id: &str, index: usize) -> Arc<Single> {
    match get_job_by_id(problem.jobs.all(), job_id) {
        Job::Single(single) => {
            assert_eq!(index, 0);
            single
        }
        Job::Multi(multi) => multi.jobs.get(index).unwrap().clone(),
    }
}

fn get_routes(insertion_ctx: &InsertionContext) -> Vec<(&str, Vec<&str>)> {
    let mut routes = insertion_ctx
        .solution
        .routes
        .iter()
        .map(|route_ctx| {
            (
                route_ctx.route.actor.vehicle.dimens.get_id().unwrap().as_str(),
                route_ctx
                    .route
                    .tour
                    .all_activities()
                    .flat_map(|a| a.job.as_ref())
                    .map(|single| single.dimens.get_id().unwrap().as_str())
                    .collect(),
            )
        })
        .collect::<Vec<_>>();

    routes.sort_by(|(a, _), (b, _)| a.cmp(b));

    routes
}

parameterized_test! {can_restore_solution, (singles, mutlies, locks, vehicles, routes, expected), {
    can_restore_solution_impl(singles, mutlies, locks, vehicles, routes, expected);
}}

can_restore_solution! {
    case01_single_all_correct: (vec![("job1", (Some(1), (1., 3.), 1., 1)), ("job2", (Some(2), (2., 4.), 1., 1)), ("job3", (Some(3), (3., 5.), 1., 1))],
        vec![], vec![],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (1., 3.), 0), ("job2", 2, 1., (2., 4.), 0), ("job3", 3, 1., (3., 5.), 0)])],
        ((0, 0), vec![("v1", vec!["job1", "job2", "job3"])])),

    case02_single_invalid_second_job_tw: (vec![("job1", (Some(1), (1., 3.), 1., 1)), ("job2", (Some(2), (0., 1.), 1., 1)), ("job3", (Some(3), (3., 5.), 1., 1))],
        vec![], vec![],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (1., 3.), 0), ("job2", 2, 1., (0., 1.), 0), ("job3", 3, 1., (3., 5.), 0)])],
        ((1, 0), vec![("v1", vec!["job1", "job3"])])),

    case03_single_multi_assignment_in_one_route: (vec![("job1", (Some(1), (0., 10.), 1., 1)), ("job2", (Some(2), (0., 10.), 1., 1))],
        vec![], vec![],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (0., 10.), 0), ("job2", 2, 1., (0., 10.), 0), ("job2", 2, 1., (0., 10.), 0)])],
        ((0, 0), vec![("v1", vec!["job1", "job2"])])),

    case04_single_multi_assignment_in_different_routes: (vec![("job1", (Some(1), (0., 10.), 1., 1)), ("job2", (Some(2), (0., 10.), 1., 1))],
        vec![], vec![],
        vec![
            ("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.))))),
            ("v2", (10, (0, Some(0.), None), Some((0, None, Some(10.))))),
        ],
        vec![
            ("v1", vec![ ("job1", 1, 1., (0., 10.), 0), ("job2", 2, 1., (0., 10.), 0)]),
            ("v2", vec![ ("job1", 1, 1., (0., 10.), 0)]),
        ],
        ((0, 0), vec![("v1", vec!["job1", "job2"])])),

    case05_multi_all_correct: (vec![],
        vec![("job1", vec![(Some(1), (1., 3.), 1., 1),  (Some(2), (2., 4.), 1., 1), (Some(3), (3., 5.), 1., 1)])],
        vec![],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (1., 3.), 0), ("job1", 2, 1., (2., 4.), 1), ("job1", 3, 1., (3., 5.), 2)])],
        ((0, 0), vec![("v1", vec!["job1", "job1", "job1"])])),

    case06_multi_invalid_second_index: (vec![],
        vec![("job1", vec![(Some(1), (1., 3.), 1., 1),  (Some(2), (2., 4.), 1., 1), (Some(3), (3., 5.), 1., 1)])],
        vec![],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (1., 3.), 0), ("job1", 2, 1., (2., 4.), 0), ("job1", 3, 1., (3., 5.), 2)])],
        ((1, 0), vec![])),

    case07_multi_partial_assignment: (vec![],
        vec![("job1", vec![(Some(1), (1., 3.), 1., 1),  (Some(2), (2., 4.), 1., 1), (Some(3), (3., 5.), 1., 1)])],
        vec![],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (1., 3.), 0),  ("job1", 2, 1., (2., 4.), 1), ])],
        ((1, 0), vec![])),

    case08_multi_invalid_permutation: (vec![],
        vec![("job1", vec![(Some(1), (0., 10.), 1., 1), (Some(2), (0., 10.), 1., 1)])],
        vec![],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 2, 1., (0., 10.), 1), ("job1", 1, 1., (0., 10.), 0)])],
        ((1, 0), vec![])),

    case09_relation_all_correct: (vec![("job1", (Some(1), (1., 3.), 1., 1)), ("job2", (Some(2), (2., 4.), 1., 1)), ("job3", (Some(3), (3., 5.), 1., 1))],
        vec![],
        vec![("v1", LockOrder::Sequence, LockPosition::Departure, vec!["job1", "job2"])],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (1., 3.), 0), ("job2", 2, 1., (2., 4.), 0), ("job3", 3, 1., (3., 5.), 0)])],
        ((0, 0), vec![("v1", vec!["job1", "job2", "job3"])])),

    case10_relation_sequence_middle: (vec![("job1", (Some(1), (0., 10.), 1., 1)), ("job2", (Some(2), (0., 10.), 1., 1)), ("job3", (Some(3), (0., 10.), 1., 1))],
        vec![],
        vec![("v1", LockOrder::Sequence, LockPosition::Departure, vec!["job1", "job2"])],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (0., 10.), 0), ("job3", 3, 1., (0., 10.), 0), ("job2", 2, 1., (0., 10.), 0)])],
        ((0, 0), vec![("v1", vec!["job1", "job2", "job3"])])),

    case11_relation_strict_end: (vec![("job1", (Some(1), (0., 10.), 1., 1)), ("job2", (Some(2), (0., 10.), 1., 1)), ("job3", (Some(3), (0., 10.), 1., 1))],
        vec![],
        vec![("v1", LockOrder::Strict, LockPosition::Arrival, vec!["job1", "job2"])],
        vec![("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))],
        vec![("v1", vec![ ("job1", 1, 1., (0., 10.), 0), ("job3", 3, 1., (0., 10.), 0), ("job2", 2, 1., (0., 10.), 0)])],
        ((1, 0), vec![("v1", vec!["job1", "job2"])])),

    case12_relation_strict_another_route: (vec![("job1", (Some(1), (0., 10.), 1., 1)), ("job2", (Some(2), (0., 10.), 1., 1)), ("job3", (Some(3), (0., 10.), 1., 1))],
        vec![],
        vec![("v2", LockOrder::Strict, LockPosition::Arrival, vec!["job1", "job2"])],
        vec![
            ("v1", (10, (0, Some(0.), None), Some((0, None, Some(10.))))),
            ("v2", (10, (0, Some(0.), None), Some((0, None, Some(10.)))))
        ],
        vec![
            ("v1", vec![ ("job1", 1, 1., (0., 10.), 0), ("job3", 3, 1., (0., 10.), 0), ("job2", 2, 1., (0., 10.), 0)]),
            ("v2", vec![ ("job1", 1, 1., (0., 10.), 0), ("job2", 2, 1., (0., 10.), 0), ("job3", 3, 1., (0., 10.), 0)])
        ],
        ((0, 0), vec![("v1", vec!["job3"]), ("v2", vec!["job1", "job2"])])),

    case13_multi_job_early_rejection: (vec![],
        vec![
            ("job1", vec![(Some(1), (0., 100.), 0., 1), (Some(2), (0., 100.), 0., -1)]),
            ("job2", vec![(Some(3), (0., 100.), 0., 2), (Some(4), (0., 100.), 0., -2)]),
            ("job3", vec![(Some(5), (0., 100.), 0., 2), (Some(6), (0., 100.), 0., -2)])
        ],
        vec![],
        vec![("v1", (2, (0, Some(0.), None), Some((0, None, Some(100.)))))],
        vec![
            ("v1", vec![
                ("job1", 1, 0., (0., 100.), 0),
                ("job2", 3, 0., (0., 100.), 0),
                ("job2", 4, 0., (0., 100.), 1),
                ("job3", 5, 0., (0., 100.), 0),
                ("job3", 6, 0., (0., 100.), 1),
                ("job1", 2, 0., (0., 100.), 1)
            ]),
        ],
        ((2, 0), vec![("v1", vec!["job1", "job1"])]),
    ),
}

#[allow(clippy::type_complexity)]
fn can_restore_solution_impl(
    singles: Vec<(&str, JobData)>,
    multies: Vec<(&str, Vec<JobData>)>,
    locks: Vec<LockData>,
    vehicles: Vec<(&str, VehicleData)>,
    routes: Vec<(&str, RouteData)>,
    expected: ((usize, usize), Vec<(&str, Vec<&str>)>),
) {
    let problem = Arc::new(create_test_problem(singles, multies, vehicles, locks));
    let mut insertion_ctx = create_test_insertion_ctx(problem.clone());
    add_routes(&mut insertion_ctx, routes);
    problem.constraint.accept_solution_state(&mut insertion_ctx.solution);

    let result = repair_solution_from_unknown(&insertion_ctx, &|| {
        InsertionContext::new(insertion_ctx.problem.clone(), insertion_ctx.environment.clone())
    });

    let ((unassigned, required), routes) = expected;
    assert_eq!(get_routes(&result), routes);
    assert_eq!(result.solution.unassigned.len(), unassigned);
    assert_eq!(result.solution.required.len(), required);
}

// TODO:
//  check invalid jobs within locks:
//   - invalid order
//   - multi assignment?
//   - not all or no jobs
