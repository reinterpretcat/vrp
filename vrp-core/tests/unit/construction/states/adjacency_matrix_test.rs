use super::*;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_simple_capacity;
use crate::helpers::models::common::DEFAULT_PROFILE;
use crate::helpers::models::problem::*;
use crate::models::problem::SimpleActivityCost;
use crate::models::problem::{Fleet, Jobs, VehicleDetail};
use crate::models::solution::Registry;
use crate::refinement::objectives::PenalizeUnassigned;

#[test]
fn can_create_adjacency_matrix_decipher() {
    let problem = create_diverse_problem();

    let decipher = AdjacencyMatrixDecipher::new(problem);

    assert_eq!(decipher.dimensions(), 8);
    assert_eq!(decipher.activity_direct_index.len(), 8);
    assert_eq!(decipher.activity_reverse_index.len(), 8);
    assert_eq!(decipher.actor_direct_index.len(), 2);
}

#[test]
fn can_encode_decode_valid_diverse_problem() {
    let problem = create_diverse_problem();
    let decipher = AdjacencyMatrixDecipher::new(problem.clone());
    let original_solution = SolutionContext {
        required: vec![],
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        routes: vec![
            create_route(
                problem.fleet.actors.first().unwrap().clone(),
                vec![
                    as_job_info((get_job(&problem, 0, 0), 0, 1, 1)), //
                    as_job_info((get_job(&problem, 2, 0), 0, 0, 0)),
                ],
            ),
            create_route(
                problem.fleet.actors.last().unwrap().clone(),
                vec![
                    as_job_info((get_job(&problem, 1, 0), 0, 0, 0)), //
                    as_job_info((get_job(&problem, 1, 1), 1, 0, 0)),
                ],
            ),
        ],
        registry: Registry::new(&problem.fleet),
    };
    // 0-4-7-0
    // 1-5-6
    let expected_matrix = vec![
        vec![0., 0., 0., 0., 1., 0., 0., 0.], //
        vec![0., 0., 0., 0., 0., 2., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 1.],
        vec![0., 0., 0., 0., 0., 0., 2., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0.],
        vec![1., 0., 0., 0., 0., 0., 0., 0.],
    ];

    let adjacency_matrix = decipher.encode::<VecMatrix>(&original_solution);
    assert_eq!(adjacency_matrix.data, expected_matrix);

    let restored_solution = decipher.decode(&adjacency_matrix);

    // TODO improve comparison
    assert_eq!(original_solution.required.len(), restored_solution.required.len());
    assert_eq!(original_solution.ignored.len(), restored_solution.ignored.len());
    assert_eq!(original_solution.locked.len(), restored_solution.locked.len());
    assert_eq!(original_solution.unassigned.len(), restored_solution.unassigned.len());
    assert_eq!(original_solution.routes.len(), restored_solution.routes.len());

    let adjacency_matrix = decipher.encode::<VecMatrix>(&restored_solution);
    assert_eq!(adjacency_matrix.data, expected_matrix);
}

fn create_diverse_problem() -> Arc<Problem> {
    let transport = Arc::new(TestTransportCost {});
    let fleet = Arc::new(Fleet::new(
        vec![test_driver()],
        vec![
            test_vehicle(DEFAULT_PROFILE),
            VehicleBuilder::new()
                .id("v1")
                .details(vec![VehicleDetail { start: Some(0), end: None, time: Some(DEFAULT_ACTOR_TIME_WINDOW) }])
                .build(),
        ],
    ));
    let jobs = Arc::new(Jobs::new(
        &fleet,
        vec![
            SingleBuilder::new()
                .id("job1")
                .places(vec![(Some(1), 1., vec![(0., 100.)]), (Some(2), 1., vec![(0., 10.), (20., 100.)])])
                .build_as_job_ref(),
            MultiBuilder::new()
                .id("job2")
                .job(SingleBuilder::new().id("s1").places(vec![(Some(2), 1., vec![(0., 100.)])]).build())
                .job(SingleBuilder::new().id("s2").places(vec![(Some(3), 1., vec![(10., 100.)])]).build())
                .build(),
            SingleBuilder::new().id("job3").places(vec![(Some(4), 1., vec![(0., 100.)])]).build_as_job_ref(),
        ],
        transport.as_ref(),
    ));

    Arc::new(Problem {
        fleet,
        jobs,
        locks: vec![],
        constraint: Arc::new(create_constraint_pipeline_with_simple_capacity()),
        activity: Arc::new(SimpleActivityCost::default()),
        transport,
        objective: Arc::new(PenalizeUnassigned::default()),
        extras: Arc::new(Default::default()),
    })
}

fn as_job_info(info: ActivityWithJob) -> ActivityInfo {
    ActivityInfo::Job(info)
}

fn get_job(problem: &Problem, index: usize, single_index: usize) -> Job {
    let job = problem.jobs.all().skip(index).next().unwrap().clone();

    job.as_multi().map_or_else(|| job.clone(), |m| Job::Single(m.jobs.get(single_index).unwrap().clone()))
}

fn create_route(actor: Arc<Actor>, activities: Vec<ActivityInfo>) -> RouteContext {
    let mut rc = RouteContext::new(actor);

    activities.iter().for_each(|a| {
        rc.route_mut().tour.insert_last(create_tour_activity(a, None));
    });

    rc
}
