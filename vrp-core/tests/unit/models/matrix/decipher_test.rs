use super::*;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_simple_capacity;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::Schedule;
use crate::models::matrix::SparseMatrix;
use crate::models::problem::{Fleet, Jobs, SimpleActivityCost, VehicleDetail};
use crate::models::solution::{Activity, Registry};
use crate::refinement::objectives::PenalizeUnassigned;

use crate::models::solution::Place as ActivityPlace;

// TODO add tests:
//      constraint violation (e.g. capacity)
//      multi job in wrong order
//      incomplete multi job
//      locked job in wrong route

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
    assert_eq!(to_vvec(&adjacency_matrix), expected_matrix);

    let restored_solution = decipher.decode(&adjacency_matrix);

    // TODO improve comparison
    assert_eq!(restored_solution.required.len(), original_solution.required.len());
    assert_eq!(restored_solution.ignored.len(), original_solution.ignored.len());
    assert_eq!(restored_solution.locked.len(), original_solution.locked.len());
    assert_eq!(restored_solution.unassigned.len(), original_solution.unassigned.len());
    assert_eq!(restored_solution.routes.len(), original_solution.routes.len());

    let adjacency_matrix = decipher.encode::<SparseMatrix>(&restored_solution);
    assert_eq!(to_vvec(&adjacency_matrix), expected_matrix);
}

#[test]
fn can_handle_multi_job_in_wrong_order() {
    let decipher = AdjacencyMatrixDecipher::new(create_diverse_problem());
    // 0-5-8-1
    // 2-7-6 -> 7-6 is not allowed
    let adjacency_matrix = vec![
        vec![0., 0., 0., 0., 0., 1., 0., 0., 0.], //
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 2., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 1.],
        vec![0., 0., 0., 0., 0., 2., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
    ];

    let restored_solution = decipher.decode(&to_sparse(&adjacency_matrix));
    assert_eq!(restored_solution.routes.len(), 1);
    assert_eq!(restored_solution.required.len(), 1);

    let adjacency_matrix = decipher.encode::<SparseMatrix>(&restored_solution);
    assert_eq!(to_vvec(&adjacency_matrix), vec![
        vec![0., 0., 0., 0., 0., 1., 0., 0., 0.], //
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 1.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 0., 0., 0., 0., 0., 0., 0., 0.],
        vec![0., 1., 0., 0., 0., 0., 0., 0., 0.],
    ]);
}

fn create_diverse_problem() -> Arc<Problem> {
    let transport = Arc::new(TestTransportCost {});
    let fleet = Arc::new(Fleet::new(
        vec![test_driver()],
        vec![
            VehicleBuilder::new()
                .id("v1")
                .details(vec![VehicleDetail { start: Some(0), end: Some(0), time: Some(DEFAULT_ACTOR_TIME_WINDOW) }])
                .build(),
            VehicleBuilder::new()
                .id("v2")
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

fn get_job(problem: &Problem, index: usize, single_index: usize) -> Job {
    let job = problem.jobs.all().skip(index).next().unwrap().clone();

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
                        time: place.times.get(tw_idx).cloned().unwrap(),
                    },
                    schedule: Schedule::new(0., 0.),
                    job: Some(single),
                })
            })
            .collect(),
    )
}

fn to_vvec(matrix: &SparseMatrix) -> Vec<Vec<f64>> {
    let mut data = vec![vec![0.; matrix.size]; matrix.size];
    matrix.data.iter().for_each(|(row, cells)| {
        cells.iter().for_each(|&(col, value)| {
            data[*row][col] = value;
        });
    });

    data
}

fn to_sparse(matrix: &Vec<Vec<f64>>) -> SparseMatrix {
    let mut sparse = SparseMatrix::new(matrix.len());

    for (row_idx, cols) in matrix.iter().enumerate() {
        for (col_idx, v) in cols.iter().enumerate() {
           if *v != 0. {
               sparse.set_cell(row_idx, col_idx, *v)
           }
        }
    }

    sparse
}
