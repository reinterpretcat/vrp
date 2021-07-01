use super::*;
use crate::helpers::construction::constraints::create_simple_demand;
use crate::helpers::models::problem::*;
use crate::models::common::*;
use crate::models::problem::{Jobs, ObjectiveCost, VehicleDetail, VehiclePlace};
use crate::models::solution::Place;
use crate::models::Problem;
use crate::utils::Environment;

type JobData = (Option<Location>, (f64, f64), Duration, i32);
type VehicleData = ((Location, Option<f64>, Option<f64>), Option<(Location, Option<f64>, Option<f64>)>);
type ActivityData = (Location, Duration, (Timestamp, Timestamp), Arc<Single>);

fn create_test_problem(
    singles: Vec<(&str, JobData)>,
    multies: Vec<(&str, Vec<JobData>)>,
    vehicles: Vec<(&str, VehicleData)>,
) -> Problem {
    let create_single = |id: &str, (location, (tw_start, tw_end), duration, demand)| {
        SingleBuilder::default()
            .id(id)
            .location(location)
            .duration(duration)
            .times(vec![TimeWindow::new(tw_start, tw_end)])
            .demand(create_simple_demand(demand))
            .build()
    };

    let jobs = singles
        .into_iter()
        .map(|(id, data)| Job::Single(Arc::new(create_single(id, data))))
        .chain(multies.into_iter().map(|(id, singles)| {
            let singles = singles.into_iter().map(|data| create_single(id, data)).collect();
            MultiBuilder::default().id(id).jobs(singles).build()
        }))
        .collect::<Vec<_>>();

    let vehicles = vehicles
        .into_iter()
        .map(|(id, ((start_location, start_earliest, start_latest), end))| {
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
                .build()
        })
        .collect::<Vec<_>>();

    let fleet = Arc::new(FleetBuilder::default().add_driver(test_driver()).add_vehicles(vehicles).build());

    let transport = TestTransportCost::new_shared();

    Problem {
        fleet: fleet.clone(),
        jobs: Arc::new(Jobs::new(&fleet, jobs, &transport)),
        locks: vec![],
        constraint: Arc::new(Default::default()),
        activity: Arc::new(TestActivityCost::default()),
        transport,
        objective: Arc::new(ObjectiveCost::default()),
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
        });
    });

    insertion_ctx.solution.routes.push(route_ctx);
}

fn get_job_by_id(problem: &Problem, job_id: &str) -> Job {
    problem.jobs.all().find(|job| job.dimens().get_id().unwrap() == job_id).clone().unwrap()
}

fn get_as_single(problem: &Problem, job_id: &str) -> Arc<Single> {
    get_job_by_id(&problem, job_id).to_single().clone()
}

#[test]
#[ignore]
fn can_restore_solution() {
    let problem = Arc::new(create_test_problem(
        vec![
            ("job1", (Some(1), (1., 3.), 1., 1)),
            ("job2", (Some(2), (2., 4.), 1., 1)),
            ("job3", (Some(3), (3., 5.), 1., 1)),
        ],
        vec![],
        vec![("v1", ((0, Some(0.), None), Some((0, None, Some(10.)))))],
    ));
    let mut insertion_ctx = create_test_insertion_ctx(problem.clone());
    add_new_route(
        &mut insertion_ctx,
        "v1",
        vec![
            (1, 1., (1., 3.), get_as_single(&problem, "job1")),
            (2, 1., (2., 4.), get_as_single(&problem, "job2")),
            (3, 1., (3., 5.), get_as_single(&problem, "job2")),
        ],
    );
    problem.constraint.accept_solution_state(&mut insertion_ctx.solution);

    let result = repair_solution_from_unknown(&insertion_ctx);

    assert_eq!(result.solution.unassigned.len(), 0);
}
