use super::*;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_simple_capacity;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{IdDimension, Schedule};
use crate::models::matrix::SparseMatrix;
use crate::models::problem::{Fleet, Jobs, SimpleActivityCost, TransportCost, VehicleDetail};
use crate::models::solution::{Activity, Registry};

use crate::construction::constraints::Demand;
use crate::helpers::refinement::create_default_objective;
use crate::models::solution::Place as ActivityPlace;
use crate::models::{Lock, LockDetail, LockOrder, LockPosition};

#[test]
fn can_create_adjacency_matrix_decipher() {
    let problem = create_diverse_problem();

    let decipher = AdjacencyMatrixDecipher::new(problem);

    assert_eq!(decipher.dimensions(), 9);
    assert_eq!(decipher.activity_direct_index.len(), 9);
    assert_eq!(decipher.activity_reverse_index.len(), 9);
    assert_eq!(decipher.actor_direct_index.len(), 2);
}

#[test]
fn can_encode_decode_feasible_diverse_problem() {
    let problem = create_diverse_problem();
    let decipher = AdjacencyMatrixDecipher::new(problem.clone());
    let original_solution = SolutionContext {
        required: vec![],
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        routes: vec![
            create_route(
                &problem.fleet,
                "v1",
                vec![
                    (get_job(&problem, 0, 0), 0, 1, 1), //
                    (get_job(&problem, 2, 0), 0, 0, 0),
                ],
            ),
            create_route(
                &problem.fleet,
                "v2",
                vec![
                    (get_job(&problem, 1, 0), 0, 0, 0), //
                    (get_job(&problem, 1, 1), 1, 0, 0),
                ],
            ),
        ],
        registry: Registry::new(&problem.fleet),
    };
    // 0-5-8-1
    // 2-6-7
    let expected_matrix = vec![
        vec![0., 0., 0., 0., 0., 1., 0., 0., 0.], //
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 2., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 1.],
        vec![0., 0., 0., 0., 0., 0., 0., 2., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
    ];

    let adjacency_matrix = decipher.encode::<SparseMatrix>(&original_solution);
    assert_eq!(adjacency_matrix.to_vvec(), expected_matrix);

    let restored_solution = decipher.decode(&adjacency_matrix);

    // TODO improve comparison
    assert_eq!(restored_solution.required.len(), original_solution.required.len());
    assert_eq!(restored_solution.ignored.len(), original_solution.ignored.len());
    assert_eq!(restored_solution.locked.len(), original_solution.locked.len());
    assert_eq!(restored_solution.unassigned.len(), original_solution.unassigned.len());
    assert_eq!(restored_solution.routes.len(), original_solution.routes.len());

    let adjacency_matrix = decipher.encode::<SparseMatrix>(&restored_solution);
    assert_eq!(adjacency_matrix.to_vvec(), expected_matrix);
}

#[test]
fn can_handle_multi_job_in_wrong_order() {
    let decipher = AdjacencyMatrixDecipher::new(create_diverse_problem());
    // 0-5-8-1
    // 2-7-6 -> 7-6 is not allowed
    let adjacency_matrix = vec![
        vec![0., 0., 0., 0., 0., 1., 0., 0., 0.], //
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 2., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 1.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 1., 0., 0., 0., 0., 2., 0., 0.],
    ];

    let restored_solution = decipher.decode(&SparseMatrix::from_vvec(&adjacency_matrix));
    assert_eq!(restored_solution.routes.len(), 1);
    assert_eq!(restored_solution.required.len(), 1);

    let adjacency_matrix = decipher.encode::<SparseMatrix>(&restored_solution);
    assert_eq!(
        adjacency_matrix.to_vvec(),
        vec![
            vec![0., 0., 0., 0., 0., 1., 0., 0., 0.], //
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 1.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
        ]
    );
}

#[test]
fn can_handle_single_job_capacity_violation() {
    let decipher = AdjacencyMatrixDecipher::new(create_diverse_problem());
    // 0-8-1
    // 2-6-7 5-> 5 violates capacity
    let adjacency_matrix = vec![
        vec![0., 0., 0., 0., 0., 0., 0., 0., 1.], //
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 2., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 2., 0.],
        vec![0., 0., 0., 0., 0., 2., 0., 0., 0.],
        vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
    ];

    let restored_solution = decipher.decode(&SparseMatrix::from_vvec(&adjacency_matrix));
    assert_eq!(restored_solution.routes.len(), 2);
    assert_eq!(restored_solution.required.len(), 1);

    let adjacency_matrix = decipher.encode::<SparseMatrix>(&restored_solution);
    assert_eq!(
        adjacency_matrix.to_vvec(),
        vec![
            vec![0., 0., 0., 0., 0., 0., 0., 0., 1.], //
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 2., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 2., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
        ]
    );
}

#[test]
fn can_handle_multi_job_capacity_violation() {
    let decipher = AdjacencyMatrixDecipher::new(create_diverse_problem());
    // 0-8-6-5-7-1 violates capacity
    let adjacency_matrix = vec![
        vec![0., 0., 0., 0., 0., 0., 0., 0., 1.], //
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 1., 0.],
        vec![0., 0., 0., 0., 0., 1., 0., 0., 0.],
        vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 1., 0., 0.],
    ];

    let restored_solution = decipher.decode(&SparseMatrix::from_vvec(&adjacency_matrix));
    assert_eq!(restored_solution.routes.len(), 1);
    assert_eq!(restored_solution.required.len(), 1);

    // expect 0-8-5-1
    let adjacency_matrix = decipher.encode::<SparseMatrix>(&restored_solution);
    assert_eq!(
        adjacency_matrix.to_vvec(),
        vec![
            vec![0., 0., 0., 0., 0., 0., 0., 0., 1.], //
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 1., 0., 0., 0.],
        ]
    );
}

#[test]
fn can_handle_multi_job_incomplete_order() {
    let decipher = AdjacencyMatrixDecipher::new(create_diverse_problem());
    // 0-6-5-1
    // 2-8-7
    let adjacency_matrix = vec![
        vec![0., 0., 0., 0., 0., 0., 1., 0., 0.], //
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 2.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 1., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 2., 0.],
    ];

    let restored_solution = decipher.decode(&SparseMatrix::from_vvec(&adjacency_matrix));
    assert_eq!(restored_solution.routes.len(), 2);
    assert_eq!(restored_solution.required.len(), 1);

    // 0-5-1
    // 2-8
    let adjacency_matrix = decipher.encode::<SparseMatrix>(&restored_solution);
    assert_eq!(
        adjacency_matrix.to_vvec(),
        vec![
            vec![0., 0., 0., 0., 0., 1., 0., 0., 0.], //
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 2.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        ]
    );
}

parameterized_test! {can_handle_job_with_lock, lock_order, {
    can_handle_job_with_lock_impl(lock_order);
}}

can_handle_job_with_lock! {
    case1: LockOrder::Any,
    case2: LockOrder::Sequence,
    case3: LockOrder::Strict,
}

fn can_handle_job_with_lock_impl(lock_order: LockOrder) {
    let mut problem = create_diverse_problem_unwrapped();
    problem.locks.push(Arc::new(Lock {
        condition: Arc::new(|actor| {
            let id = actor.vehicle.dimens.get_id().unwrap();

            id == "v1"
        }),
        details: vec![LockDetail::new(lock_order, LockPosition::Any, vec![problem.jobs.all().last().unwrap()])],
    }));

    let decipher = AdjacencyMatrixDecipher::new(Arc::new(problem));
    // 0-5-1
    // 2-8
    let adjacency_matrix = vec![
        vec![0., 0., 0., 0., 0., 1., 0., 0., 0.], //
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 2.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
    ];

    let restored_solution = decipher.decode(&SparseMatrix::from_vvec(&adjacency_matrix));
    assert_eq!(restored_solution.routes.len(), 1);
    assert_eq!(restored_solution.required.len(), 1);

    // 0-8-5-1
    let adjacency_matrix = decipher.encode::<SparseMatrix>(&restored_solution);
    assert_eq!(
        adjacency_matrix.to_vvec(),
        vec![
            vec![0., 0., 0., 0., 0., 0., 0., 0., 1.], //
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
            vec![0., 0., 0., 0., 0., 1., 0., 0., 0.],
        ]
    );
}

fn create_diverse_problem() -> Arc<Problem> {
    Arc::new(create_diverse_problem_unwrapped())
}

fn create_diverse_problem_unwrapped() -> Problem {
    let transport: Arc<dyn TransportCost + Sync + Send> = Arc::new(TestTransportCost {});
    let fleet = Arc::new(
        FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![
                VehicleBuilder::default()
                    .id("v1")
                    .capacity(2)
                    .details(vec![VehicleDetail {
                        start: Some(0),
                        end: Some(0),
                        time: Some(DEFAULT_ACTOR_TIME_WINDOW),
                    }])
                    .build(),
                VehicleBuilder::default()
                    .id("v2")
                    .capacity(2)
                    .details(vec![VehicleDetail { start: Some(0), end: None, time: Some(DEFAULT_ACTOR_TIME_WINDOW) }])
                    .build(),
            ])
            .build(),
    );
    let demand = Demand { pickup: (0, 0), delivery: (1, 0) };
    let jobs = Arc::new(Jobs::new(
        &fleet,
        vec![
            SingleBuilder::default()
                .id("job1")
                .demand(demand.clone())
                .places(vec![(Some(1), 1., vec![(0., 100.)]), (Some(2), 1., vec![(0., 10.), (20., 100.)])])
                .build_as_job_ref(),
            MultiBuilder::default()
                .id("job2")
                .job(
                    SingleBuilder::default()
                        .id("s1")
                        .demand(demand.clone())
                        .places(vec![(Some(2), 1., vec![(0., 100.)])])
                        .build(),
                )
                .job(
                    SingleBuilder::default()
                        .id("s2")
                        .demand(demand.clone())
                        .places(vec![(Some(3), 1., vec![(10., 100.)])])
                        .build(),
                )
                .build(),
            SingleBuilder::default()
                .id("job3")
                .demand(demand)
                .places(vec![(Some(4), 1., vec![(0., 100.)])])
                .build_as_job_ref(),
        ],
        &transport,
    ));

    Problem {
        fleet,
        jobs,
        locks: vec![],
        constraint: Arc::new(create_constraint_pipeline_with_simple_capacity()),
        activity: Arc::new(SimpleActivityCost::default()),
        transport,
        objective: create_default_objective(),
        extras: Arc::new(Default::default()),
    }
}

fn get_job(problem: &Problem, index: usize, single_index: usize) -> Job {
    let job = problem.jobs.all().nth(index).unwrap();

    job.as_multi().map_or_else(|| job.clone(), |m| Job::Single(m.jobs.get(single_index).unwrap().clone()))
}

fn create_route(fleet: &Fleet, vehicle: &str, activities: Vec<ActivityWithJob>) -> RouteContext {
    create_route_context_with_activities(
        fleet,
        vehicle,
        activities
            .into_iter()
            .map(|(job, single_idx, place_idx, tw_idx)| {
                let single = match job {
                    Job::Single(single) => single,
                    Job::Multi(multi) => multi.jobs.get(single_idx).cloned().unwrap(),
                };

                let place = single.places.get(place_idx).unwrap();

                Box::new(Activity {
                    place: ActivityPlace {
                        location: place.location.unwrap(),
                        duration: place.duration,
                        time: place.times.get(tw_idx).unwrap().as_time_window().unwrap(),
                    },
                    schedule: Schedule::new(0., 0.),
                    job: Some(single),
                })
            })
            .collect(),
    )
}
