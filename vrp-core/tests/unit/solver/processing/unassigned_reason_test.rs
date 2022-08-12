use crate::construction::heuristics::{InsertionContext, SolutionContext, UnassignedCode};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_transport;
use crate::helpers::models::domain::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::create_route_context_with_activities;
use crate::models::common::{IdDimension, TimeWindow};
use crate::models::problem::Job;
use crate::solver::processing::UnassignmentReason;
use rosomaxa::evolution::HeuristicSolutionProcessing;

const UNASSIGNMENT_CODE: i32 = 1;

fn create_test_insertion_ctx(unassigned: Vec<(Job, UnassignedCode)>) -> InsertionContext {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v2"))
        .build();
    let routes = vec![
        create_route_context_with_activities(&fleet, "v1", vec![]),
        create_route_context_with_activities(&fleet, "v2", vec![]),
    ];
    let mut insertion_ctx = InsertionContext {
        problem: create_problem_with_constraint_jobs_and_fleet(
            create_constraint_pipeline_with_transport(),
            unassigned.iter().map(|(job, _)| job.clone()).collect(),
            fleet,
        ),
        solution: SolutionContext {
            unassigned: unassigned.into_iter().collect(),
            routes,
            ..create_empty_solution_context()
        },
        ..create_empty_insertion_context()
    };
    insertion_ctx.problem.constraint.accept_solution_state(&mut insertion_ctx.solution);

    insertion_ctx
}

fn create_early_delivery(id: &str) -> Job {
    SingleBuilder::default().times(vec![TimeWindow::new(0., 0.)]).location(Some(10)).id(id).build_as_job_ref()
}

fn create_assignable_delivery(id: &str) -> Job {
    SingleBuilder::default().id(id).build_as_job_ref()
}

parameterized_test! {can_combine_vehicle_details, (unassigned, expected_details), {
    can_combine_vehicle_details_impl(unassigned, expected_details);
}}

can_combine_vehicle_details! {
    case_01: (
        vec![(create_early_delivery("job1"), UnassignedCode::Unknown)],
        vec![("job1", vec![("v1", UNASSIGNMENT_CODE), ("v2", UNASSIGNMENT_CODE)])]
    ),
    case_02: (
        vec![
            (create_early_delivery("job1"), UnassignedCode::Simple(UNASSIGNMENT_CODE)),
            (create_early_delivery("job2"), UnassignedCode::Unknown)
        ],
        vec![("job1", vec![("v1", UNASSIGNMENT_CODE), ("v2", UNASSIGNMENT_CODE)]),
             ("job2", vec![("v1", UNASSIGNMENT_CODE), ("v2", UNASSIGNMENT_CODE)])]
    ),
}

fn can_combine_vehicle_details_impl(
    unassigned: Vec<(Job, UnassignedCode)>,
    expected_details: Vec<(&str, Vec<(&str, i32)>)>,
) {
    let insertion_ctx = create_test_insertion_ctx(unassigned);

    let insertion_ctx = UnassignmentReason::default().post_process(insertion_ctx);

    assert_eq!(insertion_ctx.solution.unassigned.len(), expected_details.len());
    insertion_ctx.solution.unassigned.into_iter().zip(expected_details.into_iter()).for_each(
        |((job, code), (expected_job_id, expected_details))| {
            assert_eq!(job.to_single().dimens.get_id().unwrap(), expected_job_id);
            match code {
                UnassignedCode::Detailed(details) => {
                    let details = details
                        .iter()
                        .map(|(actor, code)| (actor.vehicle.dimens.get_id().unwrap().as_str(), *code))
                        .collect::<Vec<_>>();
                    assert_eq!(details, expected_details);
                }
                _ => unreachable!(),
            }
        },
    );
}

parameterized_test! {can_handle_assignable_job, code, {
    can_handle_assignable_job_impl(code);
}}

can_handle_assignable_job! {
    case_01: UnassignedCode::Unknown,
    case_02: UnassignedCode::Simple(UNASSIGNMENT_CODE),
    case_03: UnassignedCode::Simple(2),
}

fn can_handle_assignable_job_impl(code: UnassignedCode) {
    let expected = (create_assignable_delivery("job1"), code);
    let insertion_ctx = create_test_insertion_ctx(vec![expected.clone()]);

    let insertion_ctx = UnassignmentReason::default().post_process(insertion_ctx);

    assert_eq!(insertion_ctx.solution.unassigned.len(), 1);
    let (actual_job, actual_code) = insertion_ctx.solution.unassigned.into_iter().next().unwrap();

    let (expected_job, expected_code) = expected;
    assert!(actual_job == expected_job);
    match (actual_code, expected_code) {
        (UnassignedCode::Unknown, UnassignedCode::Unknown) => {}
        (UnassignedCode::Simple(actual_code), UnassignedCode::Simple(expected_code)) => {
            assert_eq!(actual_code, expected_code)
        }
        _ => unreachable!(),
    }
}
