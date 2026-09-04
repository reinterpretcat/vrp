use super::*;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

fn create_test_problem(job_skills: Option<JobSkills>, vehicle_skills: Option<Vec<String>>) -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![Job { skills: job_skills, ..create_delivery_job("job1", (1., 0.)) }],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["some_real_vehicle".to_string()],
                skills: vehicle_skills,
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

fn create_test_solution() -> Solution {
    SolutionBuilder::default()
        .tour(Tour {
            vehicle_id: "some_real_vehicle".to_string(),
            type_id: "my_vehicle".to_string(),
            shift_index: 0,
            stops: vec![
                StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                StopBuilder::default()
                    .coordinate((1., 0.))
                    .schedule_stamp(1., 1.)
                    .load(vec![0])
                    .distance(1)
                    .build_single("job1", "delivery"),
                StopBuilder::default()
                    .coordinate((0., 0.))
                    .schedule_stamp(2., 2.)
                    .load(vec![0])
                    .distance(2)
                    .build_arrival(),
            ],
            statistic: Statistic::default(),
        })
        .build()
}

fn skills(all_of: Option<Vec<&str>>, one_of: Option<Vec<&str>>, none_of: Option<Vec<&str>>) -> Option<JobSkills> {
    let map = |skills: Option<Vec<&str>>| skills.map(|s| s.into_iter().map(String::from).collect::<Vec<_>>());

    Some(JobSkills { all_of: map(all_of), one_of: map(one_of), none_of: map(none_of) })
}

fn held(skills: Vec<&str>) -> Option<Vec<String>> {
    Some(skills.into_iter().map(String::from).collect())
}

fn check(job_skills: Option<JobSkills>, vehicle_skills: Option<Vec<String>>) -> Result<(), Vec<GenericError>> {
    let ctx = CheckerContext::new(
        create_example_problem(),
        create_test_problem(job_skills, vehicle_skills),
        None,
        create_test_solution(),
    )
    .unwrap();

    check_skills(&ctx)
}

fn expected_violation() -> Result<(), Vec<GenericError>> {
    Err(vec![
        "job 'job1' requires skills its vehicle does not hold, vehicle id 'some_real_vehicle', shift index: 0".into(),
    ])
}

#[test]
fn can_pass_when_the_job_demands_nothing() {
    assert_eq!(check(None, held(vec!["a"])), Ok(()));
}

#[test]
fn can_check_all_of() {
    assert_eq!(check(skills(Some(vec!["a", "b"]), None, None), held(vec!["a", "b", "c"])), Ok(()));
    assert_eq!(check(skills(Some(vec!["a", "b"]), None, None), held(vec!["a"])), expected_violation());
}

// This is the shape the area gate rides on: a job carries one tag per area that may serve it and
// the vehicle carries the tags of the areas it holds, so a single match is the whole permission.
#[test]
fn can_check_one_of() {
    assert_eq!(check(skills(None, Some(vec!["a", "b"]), None), held(vec!["b"])), Ok(()));
    assert_eq!(check(skills(None, Some(vec!["a", "b"]), None), held(vec!["c"])), expected_violation());
}

#[test]
fn can_check_none_of() {
    assert_eq!(check(skills(None, None, Some(vec!["a"])), held(vec!["b"])), Ok(()));
    assert_eq!(check(skills(None, None, Some(vec!["a"])), held(vec!["a"])), expected_violation());
}

// A vehicle without skills holds nothing, so any non-empty demand is unmet — the same reading the
// solver's own constraint takes.
#[test]
fn can_treat_a_vehicle_without_skills_as_holding_none() {
    assert_eq!(check(skills(Some(vec!["a"]), None, None), None), expected_violation());
    assert_eq!(check(skills(None, Some(vec!["a"]), None), None), expected_violation());
    assert_eq!(check(skills(Some(vec![]), None, None), None), Ok(()));
    assert_eq!(check(skills(None, None, Some(vec!["a"])), None), Ok(()));
}

// Our emitter writes `"oneOf": []` on every job the area gate leaves unrestricted, and
// `JobSkills::new` drops an empty list before the constraint sees it. Reading it as "must hold one
// of nothing" would condemn every such job.
#[test]
fn can_read_an_empty_list_as_no_demand_at_all() {
    assert_eq!(check(skills(Some(vec![]), Some(vec![]), Some(vec![])), held(vec!["a"])), Ok(()));
    assert_eq!(check(skills(Some(vec!["a"]), Some(vec![]), Some(vec![])), held(vec!["a"])), Ok(()));
    assert_eq!(check(skills(Some(vec![]), Some(vec![]), Some(vec![])), None), Ok(()));
}

#[test]
fn can_fail_when_only_one_of_several_demands_is_unmet() {
    assert_eq!(check(skills(Some(vec!["a"]), Some(vec!["b"]), None), held(vec!["a", "b"])), Ok(()));
    assert_eq!(check(skills(Some(vec!["a"]), Some(vec!["b"]), None), held(vec!["a"])), expected_violation());
}
