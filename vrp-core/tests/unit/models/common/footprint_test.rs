use super::*;
use crate::algorithms::structures::BitVec;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::problem::{TestSingleBuilder, TestVehicleBuilder};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::models::problem::{MatrixData, create_matrix_transport_cost};
use crate::prelude::*;

fn create_test_problem() -> Problem {
    ProblemBuilder::default()
        .add_vehicle(TestVehicleBuilder::default().build())
        .add_jobs(
            vec![
                TestSingleBuilder::default().id("job1").build_as_job_ref(),
                TestSingleBuilder::default().id("job2").build_as_job_ref(),
            ]
            .into_iter(),
        )
        .with_transport_cost(
            create_matrix_transport_cost(vec![MatrixData::new(0, None, vec![0.; 9], vec![0.; 9])]).unwrap(),
        )
        .with_goal(TestGoalContextBuilder::default().build())
        .build()
        .unwrap()
}

#[test]
fn can_use_footprint_new() {
    let problem = create_test_problem();
    let footprint = Footprint::new(&problem);
    assert_eq!(footprint.dimension(), get_dimension(&problem));
    assert!(footprint.repr.iter().all(|&value| value == 0));
}

#[test]
fn can_use_footprint_add() {
    let problem = create_test_problem();
    let mut footprint = Footprint::new(&problem);
    let mut shadow = Shadow { repr: BitVec::new(footprint.dimension() * footprint.dimension()) };
    shadow.repr.set(0, true);
    shadow.repr.set(4, true);

    footprint.add(&shadow);

    assert_eq!(footprint.get(0, 0), 1);
    assert_eq!(footprint.get(1, 1), 1);
}

#[test]
fn can_use_footprint_union() {
    let problem = create_test_problem();
    let mut footprint1 = Footprint::new(&problem);
    let mut footprint2 = Footprint::new(&problem);
    footprint1.repr[0] = 1;
    footprint2.repr[1] = 1;

    footprint1.union(&footprint2);

    assert_eq!(footprint1.get(0, 0), 1);
    assert_eq!(footprint1.get(0, 1), 1);
}

#[test]
fn can_use_footprint_get() {
    let problem = create_test_problem();
    let mut footprint = Footprint::new(&problem);
    footprint.repr[0] = 2;

    assert_eq!(footprint.get(0, 0), 2);
}

#[test]
fn tcan_use_footprint_iter() {
    let problem = create_test_problem();
    let mut footprint = Footprint::new(&problem);
    footprint.repr[0] = 1;
    footprint.repr[1] = 2;

    #[rustfmt::skip]
    let expected = vec![
        ((0, 0), 1), ((0, 1), 2), ((0, 2), 0),
        ((1, 0), 0), ((1, 1), 0), ((1, 2), 0),
        ((2, 0), 0), ((2, 1), 0), ((2, 2), 0)
    ];

    let result: Vec<_> = footprint.iter().collect();
    assert_eq!(result, expected);
}

#[test]
fn can_use_footprint_forget() {
    let problem = create_test_problem();
    let mut footprint = Footprint::new(&problem);
    footprint.repr[0] = 16;
    footprint.repr[1] = 4;
    footprint.repr[2] = 0;

    footprint.forget();

    assert_eq!(footprint.get(0, 0), 16_f64.log2() as u8);
    assert_eq!(footprint.get(0, 1), 4_f64.log2() as u8);
    assert_eq!(footprint.get(0, 2), 0);
}

#[test]
fn can_use_footprint_estimate_solution() {
    let problem = create_test_problem();
    let mut footprint = Footprint::new(&problem);
    footprint.repr[1] = 1; // 0 -> 1
    footprint.repr[5] = 2; // 1 -> 2
    footprint.repr[6] = 3; // 2 -> 0
    let solution_ctx = TestInsertionContextBuilder::default()
        .with_problem(problem)
        .with_routes(vec![
            RouteContextBuilder::default()
                .with_route(
                    RouteBuilder::default()
                        .add_activity(ActivityBuilder::with_location(1).build())
                        .add_activity(ActivityBuilder::with_location(2).build())
                        .build(),
                )
                .build(),
        ])
        .build()
        .solution;

    let cost = footprint.estimate_solution(&solution_ctx);

    assert_eq!(cost, 6);
}

#[test]
fn can_use_footprint_estimate_edge() {
    let problem = create_test_problem();
    let mut footprint = Footprint::new(&problem);
    footprint.repr[0] = 3;

    let cost = footprint.estimate_edge(0, 0);

    assert_eq!(cost, 3);
}

#[test]
fn can_create_shadow_new() {
    let dim = 5;
    let shadow = Shadow { repr: BitVec::new(dim * dim) };
    assert_eq!(shadow.dimension(), dim);
}

#[test]
fn can_use_shadow_iter() {
    let dim = 3;
    let mut shadow = Shadow { repr: BitVec::new(dim * dim) };
    shadow.repr.set(0, true);
    shadow.repr.set(4, true);
    shadow.repr.set(8, true);

    #[rustfmt::skip]
    let expected = vec![
        ((0, 0), true),  ((0, 1), false), ((0, 2), false),
        ((1, 0), false), ((1, 1), true),  ((1, 2), false),
        ((2, 0), false), ((2, 1), false), ((2, 2), true),
    ];

    let result: Vec<_> = shadow.iter().collect();
    assert_eq!(result, expected);
}

#[test]
fn can_create_shadow_from_insertion_context() {
    let problem = create_test_problem();
    let insertion_ctx = TestInsertionContextBuilder::default().with_problem(problem).build();

    let shadow = Shadow::from(&insertion_ctx);
    assert_eq!(shadow.dimension(), 3);
}
