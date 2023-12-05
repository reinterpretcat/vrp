use crate::construction::heuristics::{InsertionContext, SolutionContext, UnassignmentInfo};
use crate::helpers::construction::features::create_goal_ctx_with_transport;
use crate::helpers::models::domain::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{RouteBuilder, RouteContextBuilder};
use crate::models::common::{IdDimension, TimeWindow};
use crate::models::problem::Job;
use crate::solver::processing::UnassignmentReason;
use rosomaxa::evolution::HeuristicSolutionProcessing;

const UNASSIGNMENT_CODE: i32 = 1;

fn create_test_insertion_ctx(unassigned: Vec<(Job, UnassignmentInfo)>) -> InsertionContext {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v2"))
        .build();
    let routes = vec![
        RouteContextBuilder::default().with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build()).build(),
        RouteContextBuilder::default().with_route(RouteBuilder::default().with_vehicle(&fleet, "v2").build()).build(),
    ];
    let mut insertion_ctx = InsertionContext {
        problem: create_problem_with_goal_ctx_jobs_and_fleet(
            create_goal_ctx_with_transport(),
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
    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);

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
    case_01_single_job: (
        vec![(create_early_delivery("job1"), UnassignmentInfo::Unknown)],
        vec![("job1", vec![("v1", UNASSIGNMENT_CODE), ("v2", UNASSIGNMENT_CODE)])]
    ),
    case_02_two_jobs: (
        vec![
            (create_early_delivery("job1"), UnassignmentInfo::Simple(UNASSIGNMENT_CODE)),
            (create_early_delivery("job2"), UnassignmentInfo::Unknown)
        ],
        vec![("job1", vec![("v1", UNASSIGNMENT_CODE), ("v2", UNASSIGNMENT_CODE)]),
             ("job2", vec![("v1", UNASSIGNMENT_CODE), ("v2", UNASSIGNMENT_CODE)])]
    ),
}

fn can_combine_vehicle_details_impl(
    unassigned: Vec<(Job, UnassignmentInfo)>,
    expected_details: Vec<(&str, Vec<(&str, i32)>)>,
) {
    let insertion_ctx = create_test_insertion_ctx(unassigned);

    let insertion_ctx = UnassignmentReason::default().post_process(insertion_ctx);

    assert_eq!(insertion_ctx.solution.unassigned.len(), expected_details.len());
    let mut actual_details = insertion_ctx.solution.unassigned.into_iter().collect::<Vec<_>>();
    actual_details.sort_by(|(a, _), (b, _)| a.dimens().get_id().cmp(&b.dimens().get_id()));
    actual_details.into_iter().zip(expected_details).for_each(|((job, code), (expected_job_id, expected_details))| {
        assert_eq!(job.to_single().dimens.get_id().unwrap(), expected_job_id);
        match code {
            UnassignmentInfo::Detailed(details) => {
                let details = details
                    .iter()
                    .map(|(actor, code)| (actor.vehicle.dimens.get_id().unwrap().as_str(), *code))
                    .collect::<Vec<_>>();
                assert_eq!(details, expected_details);
            }
            _ => unreachable!(),
        }
    });
}

parameterized_test! {can_handle_assignable_job, code, {
    can_handle_assignable_job_impl(code);
}}

can_handle_assignable_job! {
    case_01_unknown_code: UnassignmentInfo::Unknown,
    case_02_same_code: UnassignmentInfo::Simple(UNASSIGNMENT_CODE),
    case_03_different_code: UnassignmentInfo::Simple(2),
}

fn can_handle_assignable_job_impl(code: UnassignmentInfo) {
    let expected = (create_assignable_delivery("job1"), code);
    let insertion_ctx = create_test_insertion_ctx(vec![expected.clone()]);

    let insertion_ctx = UnassignmentReason::default().post_process(insertion_ctx);

    assert_eq!(insertion_ctx.solution.unassigned.len(), 1);
    let (actual_job, actual_code) = insertion_ctx.solution.unassigned.into_iter().next().unwrap();

    let (expected_job, expected_code) = expected;
    assert!(actual_job == expected_job);
    match (actual_code, expected_code) {
        (UnassignmentInfo::Unknown, UnassignmentInfo::Unknown) => {}
        (UnassignmentInfo::Simple(actual_code), UnassignmentInfo::Simple(expected_code)) => {
            assert_eq!(actual_code, expected_code)
        }
        _ => unreachable!(),
    }
}
