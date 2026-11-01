use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

fn get_job_activities(solution: &Solution, job_id: &str) -> Vec<(String, String)> {
    solution
        .tours
        .iter()
        .flat_map(|tour| tour.stops.iter())
        .flat_map(|stop| stop.activities())
        .filter(|activity| activity.job_id == job_id)
        .map(|activity| (activity.activity_type.clone(), activity.job_tag.clone().unwrap_or_default()))
        .collect()
}

fn create_activities(activities: &[(&str, &str)]) -> Vec<(String, String)> {
    activities.iter().map(|(activity_type, tag)| (activity_type.to_string(), tag.to_string())).collect()
}

#[test]
fn can_keep_pickups_in_fixed_order() {
    // NOTE without a fixed order, picking up p2 at (2, 0) before p1 at (4, 0) is cheaper
    let problem = Problem {
        plan: Plan {
            jobs: vec![Job {
                fixed_order: Some(true),
                ..create_multi_job(
                    "multi",
                    vec![((4., 0.), 1., vec![1]), ((2., 0.), 1., vec![1])],
                    vec![((6., 0.), 1., vec![2])],
                )
            }],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![0])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(4., 5.)
                            .load(vec![1])
                            .distance(4)
                            .build_single_tag("multi", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(7., 8.)
                            .load(vec![2])
                            .distance(6)
                            .build_single_tag("multi", "pickup", "p2"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(12., 13.)
                            .load(vec![0])
                            .distance(10)
                            .build_single_tag("multi", "delivery", "d1"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(19., 19.)
                            .load(vec![0])
                            .distance(16)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(16).serving(3).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_keep_pickups_and_deliveries_in_fixed_order() {
    // NOTE without a fixed order, the cheapest tour picks up p2 first and delivers d2 first
    let create_job = |fixed_order: Option<bool>| Job {
        fixed_order,
        ..create_multi_job(
            "multi",
            vec![((3., 0.), 1., vec![1]), ((1., 0.), 1., vec![1])],
            vec![((5., 5.), 1., vec![1]), ((5., 0.), 1., vec![1])],
        )
    };
    let solve = |fixed_order: Option<bool>| {
        let problem = Problem {
            plan: Plan { jobs: vec![create_job(fixed_order)], ..create_empty_plan() },
            fleet: Fleet {
                vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])],
                ..create_default_fleet()
            },
            ..create_empty_problem()
        };
        let matrix = create_matrix_from_problem(&problem);

        get_job_activities(&solve_with_metaheuristic(problem, Some(vec![matrix])), "multi")
    };

    assert_eq!(
        solve(None),
        create_activities(&[("pickup", "p2"), ("pickup", "p1"), ("delivery", "d2"), ("delivery", "d1")])
    );
    assert_eq!(
        solve(Some(true)),
        create_activities(&[("pickup", "p1"), ("pickup", "p2"), ("delivery", "d1"), ("delivery", "d2")])
    );
}
