use crate::constraints::{JobSkills, SkillsModule};
use crate::extensions::create_typed_actor_groups;
use crate::helpers::*;
use hashbrown::HashSet;
use std::iter::FromIterator;
use std::sync::Arc;
use vrp_core::construction::constraints::{ConstraintPipeline, RouteConstraintViolation};
use vrp_core::construction::heuristics::{RouteContext, RouteState};
use vrp_core::models::common::ValueDimension;
use vrp_core::models::problem::{Fleet, Job, Vehicle};

fn create_job_with_skills(all_of: Option<Vec<&str>>, one_of: Option<Vec<&str>>, none_of: Option<Vec<&str>>) -> Job {
    let mut single = create_single_with_location(None);
    single.dimens.set_value(
        "skills",
        JobSkills {
            all_of: all_of.map(|skills| skills.iter().map(|s| s.to_string()).collect()),
            one_of: one_of.map(|skills| skills.iter().map(|s| s.to_string()).collect()),
            none_of: none_of.map(|skills| skills.iter().map(|s| s.to_string()).collect()),
        },
    );

    Job::Single(Arc::new(single))
}

fn create_vehicle_with_skills(skills: Option<Vec<&str>>) -> Vehicle {
    let mut vehicle = test_vehicle("v1");

    if let Some(skills) = skills {
        vehicle.dimens.set_value("skills", HashSet::<String>::from_iter(skills.iter().map(|s| s.to_string())));
    }

    vehicle
}

fn failure() -> Option<RouteConstraintViolation> {
    Some(RouteConstraintViolation { code: 0 })
}

parameterized_test! {can_check_skills, (all_of, one_of, none_of, vehicle_skills, expected), {
    can_check_skills_impl(all_of, one_of, none_of, vehicle_skills, expected);
}}

can_check_skills! {
    case01: (None, None, None, None, None),

    case_all_of_01: (Some(vec!["s1"]), None, None, None, failure()),
    case_all_of_02: (Some(vec![]), None, None, None, None),
    case_all_of_03: (Some(vec!["s1"]), None, None, Some(vec!["s1"]), None),
    case_all_of_04: (Some(vec!["s1"]), None, None, Some(vec!["s2"]), failure()),
    case_all_of_05: (Some(vec!["s1", "s2"]), None, None, Some(vec!["s2"]), failure()),
    case_all_of_06: (Some(vec!["s1"]), None, None, Some(vec!["s1", "s2"]), None),

    case_one_of_01: (None, Some(vec!["s1"]), None, None, failure()),
    case_one_of_02: (None, Some(vec![]), None, None, None),
    case_one_of_03: (None, Some(vec!["s1"]), None, Some(vec!["s1"]), None),
    case_one_of_04: (None, Some(vec!["s1"]), None, Some(vec!["s2"]), failure()),
    case_one_of_05: (None, Some(vec!["s1", "s2"]), None, Some(vec!["s2"]), None),
    case_one_of_06: (None, Some(vec!["s1"]), None, Some(vec!["s1", "s2"]), None),

    case_none_of_01: (None, None, Some(vec!["s1"]), None, None),
    case_none_of_02: (None, None, Some(vec![]), None, None),
    case_none_of_03: (None, None, Some(vec!["s1"]), Some(vec!["s1"]), failure()),
    case_none_of_04: (None, None, Some(vec!["s1"]), Some(vec!["s2"]), None),
    case_none_of_05: (None, None, Some(vec!["s1", "s2"]), Some(vec!["s2"]), failure()),
    case_none_of_06: (None, None, Some(vec!["s1"]), Some(vec!["s1", "s2"]), failure()),

    case_combine_01: (Some(vec!["s1"]), None, Some(vec!["s2"]), Some(vec!["s1", "s2"]), failure()),
    case_combine_02: (None, Some(vec!["s1"]), Some(vec!["s2"]), Some(vec!["s1", "s2"]), failure()),
    case_combine_03: (Some(vec!["s1"]), Some(vec!["s2"]), None, Some(vec!["s1", "s2"]), None),
    case_combine_04: (Some(vec!["s1"]), Some(vec!["s2", "s3"]), None, Some(vec!["s1", "s2"]), None),
    case_combine_05: (Some(vec!["s1", "s2"]), Some(vec!["s3"]), None, Some(vec!["s1", "s2", "s3"]), None),
    case_combine_06: (Some(vec!["s1", "s2"]), Some(vec!["s3"]), None, Some(vec!["s1", "s2"]), failure()),
    case_combine_07: (Some(vec!["s1"]), Some(vec!["s2"]), Some(vec!["s3"]), Some(vec!["s1", "s2", "s3"]), failure()),
}

fn can_check_skills_impl(
    all_of: Option<Vec<&str>>,
    one_of: Option<Vec<&str>>,
    none_of: Option<Vec<&str>>,
    vehicle_skills: Option<Vec<&str>>,
    expected: Option<RouteConstraintViolation>,
) {
    let fleet = Fleet::new(
        vec![Arc::new(test_driver())],
        vec![Arc::new(create_vehicle_with_skills(vehicle_skills))],
        Box::new(|actors| create_typed_actor_groups(actors)),
    );
    let route_ctx = RouteContext::new_with_state(
        Arc::new(create_route_with_activities(&fleet, "v1", vec![])),
        Arc::new(RouteState::default()),
    );

    let actual = ConstraintPipeline::default().add_module(Box::new(SkillsModule::new(0))).evaluate_hard_route(
        &create_solution_context_for_fleet(&fleet),
        &route_ctx,
        &create_job_with_skills(all_of, one_of, none_of),
    );

    assert_eq!(actual, expected)
}
