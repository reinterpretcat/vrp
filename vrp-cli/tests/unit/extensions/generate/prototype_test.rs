use super::*;
use crate::helpers::generate::*;
use vrp_pragmatic::format::problem::*;

#[test]
fn can_generate_jobs_with_time_windows() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_test_job(-1., 1.),
                create_test_job(1., 0.),
                create_test_job(3., 1.),
                create_test_job(1., 2.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_test_vehicle_type()], profiles: vec![create_test_vehicle_profile()] },
        objectives: None,
    };

    let result =
        generate_from_prototype(&problem, None, 10, 2, None).unwrap_or_else(|err| panic!("cannot generate: '{}'", err));

    assert_eq!(result.plan.jobs.len(), 10);
    assert_eq!(
        result
            .plan
            .jobs
            .first()
            .expect("No job")
            .pickups
            .as_ref()
            .expect("No delivery")
            .first()
            .expect("No job task")
            .places
            .first()
            .expect("No job place")
            .times,
        Some(vec![create_test_time_window()])
    )
}
