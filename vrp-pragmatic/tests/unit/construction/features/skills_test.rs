use crate::constraints::JobSkills;
use crate::construction::enablers::create_typed_actor_groups;
use crate::construction::enablers::{JobTie, VehicleTie};
use crate::construction::features::skills::create_skills_feature;
use crate::helpers::*;
use hashbrown::HashSet;
use std::iter::FromIterator;
use std::sync::Arc;
use vrp_core::construction::features::{ConstraintViolation, ViolationCode};
use vrp_core::construction::heuristics::{MoveContext, RouteContext, RouteState};
use vrp_core::models::problem::{Fleet, Job, Vehicle};

const VIOLATION_CODE: ViolationCode = 1;

fn create_job_with_skills(all_of: Option<Vec<&str>>, one_of: Option<Vec<&str>>, none_of: Option<Vec<&str>>) -> Job {
    let mut single = create_single_with_location(None);
    single.dimens.set_job_skills(Some(JobSkills {
        all_of: all_of.map(|skills| skills.iter().map(|s| s.to_string()).collect()),
        one_of: one_of.map(|skills| skills.iter().map(|s| s.to_string()).collect()),
        none_of: none_of.map(|skills| skills.iter().map(|s| s.to_string()).collect()),
    }));

    Job::Single(Arc::new(single))
}

fn create_vehicle_with_skills(skills: Option<Vec<&str>>) -> Vehicle {
    let mut vehicle = test_vehicle("v1");

    if let Some(skills) = skills {
        vehicle.dimens.set_vehicle_skills(HashSet::<String>::from_iter(skills.iter().map(|s| s.to_string())));
    }

    vehicle
}

fn failure() -> Option<ConstraintViolation> {
    ConstraintViolation::fail(VIOLATION_CODE)
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
    expected: Option<ConstraintViolation>,
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
    let constraint = create_skills_feature(VIOLATION_CODE).unwrap().constraint.unwrap();

    let actual = constraint.evaluate(&MoveContext::route(
        &create_solution_context_for_fleet(&fleet),
        &route_ctx,
        &create_job_with_skills(all_of, one_of, none_of),
    ));

    assert_eq!(actual, expected)
}

parameterized_test! {can_merge_skills, (source, candidate, expected), {
    can_merge_skills_impl(source, candidate, expected);
}}

can_merge_skills! {
    case_01: (create_job_with_skills(None, None, None), create_job_with_skills(None, None, None), Ok(())),

    case_02: (create_job_with_skills(Some(vec!["skill"]), None, None), create_job_with_skills(None, None, None), Ok(())),
    case_03: (create_job_with_skills(None, Some(vec!["skill"]), None), create_job_with_skills(None, None, None), Ok(())),
    case_04: (create_job_with_skills(None, None, Some(vec!["skill"])), create_job_with_skills(None, None, None), Ok(())),

    case_05: (create_job_with_skills(None, None, None), create_job_with_skills(Some(vec!["skill"]), None, None), Err(1)),
    case_06: (create_job_with_skills(None, None, None), create_job_with_skills(None, Some(vec!["skill"]), None), Err(1)),
    case_07: (create_job_with_skills(None, None, None), create_job_with_skills(None, None, Some(vec!["skill"])), Err(1)),

    case_08: (create_job_with_skills(Some(vec!["skill"]), None, None), create_job_with_skills(Some(vec!["skill"]), None, None), Ok(())),
    case_09: (create_job_with_skills(Some(vec!["skill"]), None, None), create_job_with_skills(None, Some(vec!["skill"]), None), Err(1)),
    case_10: (create_job_with_skills(Some(vec!["skill1", "skill2"]), None, None), create_job_with_skills(Some(vec!["skill1"]), None, None), Ok(())),
    case_11: (create_job_with_skills(Some(vec!["skill1"]), None, None), create_job_with_skills(Some(vec!["skill1", "skill2"]), None, None), Err(1)),
}

fn can_merge_skills_impl(source: Job, candidate: Job, expected: Result<(), i32>) {
    let constraint = create_skills_feature(VIOLATION_CODE).unwrap().constraint.unwrap();

    let result = constraint.merge(source, candidate).map(|_| ());

    assert_eq!(result, expected);
}
