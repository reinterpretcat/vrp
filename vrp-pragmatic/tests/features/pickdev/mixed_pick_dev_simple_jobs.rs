use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_one_pickup_delivery_and_two_deliveries_with_one_vehicle() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_pickup_delivery_job("job2", (2., 0.), (3., 0.)),
                create_delivery_job("job3", (4., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "every job must be served");
    assert_eq!(solution.tours.len(), 1, "one vehicle must do all of it");

    // The jobs sit at 1, 2, 3 and 4 on a line out of a depot at 0, so the round trip costs the same
    // travelled in either direction and the solver may return either. What must hold is the total
    // and the order the pickup-delivery pair imposes.
    assert_eq!(solution.statistic.distance, 8);
    assert_eq!(solution.statistic.duration, 12);
    assert_eq!(solution.statistic.times.serving, 4);

    let activities = tour_activities(&solution.tours[0]);
    let position_of = |job_id: &str, activity_type: &str| {
        activities
            .iter()
            .position(|activity| activity.0 == job_id && activity.1 == activity_type)
            .unwrap_or_else(|| panic!("'{job_id}' is not served as '{activity_type}': {activities:?}"))
    };

    position_of("job1", "delivery");
    position_of("job3", "delivery");
    assert!(
        position_of("job2", "pickup") < position_of("job2", "delivery"),
        "job2 must be picked up before it is delivered: {activities:?}"
    );
    assert_eq!(activities.len(), 4, "nothing else may be served: {activities:?}");
}
