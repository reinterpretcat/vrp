use crate::checker::jobs::*;
use crate::checker::models::{StopInfo, TourInfo, VehicleMeta};
use crate::helpers::*;
use crate::json::solution::{Extras, Solution, Statistic, Timing, Tour};
use std::sync::Arc;

#[test]
fn can_validate_job_presence() {
    let tour = Tour {
        vehicle_id: "my_vehicle_1".to_string(),
        type_id: "my_vehicle".to_string(),
        stops: vec![
            create_stop_with_activity_with_tag(
                "departure",
                "departure",
                (1., 0.),
                2,
                default_time_window(),
                &create_info_tag(&"single", 1, vec![1., 0.], vec![0], vec![vec![0, 1]], 0.),
            ),
            create_stop_with_activity_with_tag(
                "job1",
                "delivery",
                (1., 0.),
                2,
                default_time_window(),
                &create_info_tag(&"single", 1, vec![1., 0.], vec![0], vec![vec![0, 1]], 0.),
            ),
            create_stop_with_activity_with_tag(
                "job2",
                "pickup",
                (1., 0.),
                2,
                default_time_window(),
                &create_info_tag(&"single", 1, vec![1., 0.], vec![0], vec![vec![0, 1]], 0.),
            ),
        ],
        statistic: Default::default(),
    };
    let mut solution_info = create_test_solution_info(
        vec![create_default_vehicle("my_vehicle")],
        None,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Default::default(),
            tours: vec![tour],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        },
    );
    solution_info.jobs.insert("job3".to_string(), Arc::new(create_delivery_job("job3", vec![1., 0.])));

    let result = check_job_presence(&solution_info).err();

    assert_eq_option!(result, Some("Solution has less jobs than the problem: 2 < 3".to_string()));
}

parameterized_test! {can_validate_stop_demand, (loads, expected), {
    can_validate_stop_demand_impl(loads.into_iter().map(|(load, demand)| (load, vec![demand])).collect(), expected);
}}

can_validate_stop_demand! {
    case01: (vec![(4, 0), (2, 2), (4, 2), (2, 2), (0, 2)], None),
    case02: (vec![(4, 0), (4, 2), (4, 2), (2, 2), (0, 2)], Some("Stop load mismatch: result '[4]' != expected '[2]'".to_string())),
    case03: (vec![(4, 0), (2, 2), (4, 2), (3, 2), (0, 2)], Some("Stop load mismatch: result '[3]' != expected '[2]'".to_string())),
}

fn can_validate_stop_demand_impl(loads: Vec<(i32, Vec<i32>)>, expected: Option<String>) {
    let tour_info = create_test_tour_info(Tour {
        vehicle_id: "my_vehicle_1".to_string(),
        type_id: "my_vehicle".to_string(),
        stops: vec![
            create_stop_with_activity_with_tag(
                "departure",
                "departure",
                (1., 0.),
                loads.get(0).unwrap().0,
                default_time_window(),
                &create_info_tag(&"single", 1, vec![1., 0.], loads.get(0).unwrap().1.clone(), vec![vec![0, 1]], 0.),
            ),
            create_stop_with_activity_with_tag(
                "job1",
                "delivery",
                (1., 0.),
                loads.get(1).unwrap().0,
                default_time_window(),
                &create_info_tag(&"single", 1, vec![1., 0.], loads.get(1).unwrap().1.clone(), vec![vec![0, 1]], 0.),
            ),
            create_stop_with_activity_with_tag(
                "job2",
                "pickup",
                (1., 0.),
                loads.get(2).unwrap().0,
                default_time_window(),
                &create_info_tag(&"multi", 1, vec![1., 0.], loads.get(2).unwrap().1.clone(), vec![vec![0, 1]], 0.),
            ),
            create_stop_with_activity_with_tag(
                "job2",
                "delivery",
                (1., 0.),
                loads.get(3).unwrap().0,
                default_time_window(),
                &create_info_tag(&"multi", 2, vec![1., 0.], loads.get(3).unwrap().1.clone(), vec![vec![0, 1]], 0.),
            ),
            create_stop_with_activity_with_tag(
                "job2",
                "delivery",
                (1., 0.),
                loads.get(4).unwrap().0,
                default_time_window(),
                &create_info_tag(&"multi", 3, vec![1., 0.], loads.get(4).unwrap().1.clone(), vec![vec![0, 1]], 0.),
            ),
        ],
        statistic: Statistic::default(),
    });

    let result = check_stop_has_proper_demand_change(&tour_info).err();

    assert_eq_option!(result, expected);
}
