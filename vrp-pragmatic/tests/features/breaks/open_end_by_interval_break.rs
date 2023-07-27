use crate::format::problem::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::common::{Duration, Timestamp};

fn create_optional_break(time_window: (Timestamp, Timestamp), duration: Duration) -> VehicleBreak {
    let (start, end) = time_window;
    VehicleBreak::Optional {
        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(start), format_time(end)]),
        places: vec![VehicleOptionalBreakPlace { duration, location: None, tag: None }],
        policy: None,
    }
}

fn create_required_break(earliest: Timestamp, latest: Timestamp, duration: Duration) -> VehicleBreak {
    VehicleBreak::Required { time: VehicleRequiredBreakTime::OffsetTime { earliest, latest }, duration }
}

fn create_vehicle_shift_with_breaks(breaks: Vec<VehicleBreak>) -> VehicleShift {
    VehicleShift {
        start: ShiftStart {
            earliest: format_time(0.),
            latest: Some(format_time(0.)),
            location: Location::Coordinate { lat: 0., lng: 0. },
        },
        end: None,
        breaks: Some(breaks),
        ..create_default_vehicle_shift()
    }
}

parameterized_test! {can_simulate_two_open_shifts_with_different_breaks, (jobs, breaks, expected), {
    let expected = expected.into_iter().map(to_strings).collect();
    can_simulate_two_open_shifts_with_different_breaks_impl(jobs, breaks, expected);
}}

can_simulate_two_open_shifts_with_different_breaks! {
    case01_with_time_windows: (
        vec![create_delivery_job_with_times("job1_1", (1., 0.), vec![(10, 10)], 1.),
             create_delivery_job_with_times("job1_2", (2., 0.), vec![(40, 40)], 1.),
             create_delivery_job_with_times("job2_1", (3., 0.), vec![(110, 110)], 1.),
             create_delivery_job_with_times("job2_2", (4., 0.), vec![(140, 140)], 1.),
        ],
        vec![create_optional_break((25., 30.), 5.),
             create_required_break(50., 50., 50.),
        ],
        vec![vec!["departure"], vec!["job1_1", "break"], vec!["job1_2"], vec!["break", "job2_1"], vec!["job2_2"]]
    ),

    case02_without_time_windows: (
        vec![create_delivery_job_with_duration("job1_1", (1., 0.), 20.),
             create_delivery_job_with_duration("job1_2", (2., 0.), 20.),
             create_delivery_job_with_duration("job2_1", (3., 0.), 20.),
             create_delivery_job_with_duration("job2_2", (4., 0.), 20.),
        ],
        vec![create_optional_break((25., 30.), 5.),
             create_required_break(50., 50., 50.),
        ],
        vec![vec!["departure"], vec!["job1_1", "break"], vec!["job1_2", "break"], vec!["job2_1"], vec!["job2_2"]]
    ),
}

fn can_simulate_two_open_shifts_with_different_breaks_impl(
    jobs: Vec<Job>,
    breaks: Vec<VehicleBreak>,
    expected: Vec<Vec<String>>,
) {
    let problem = Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_vehicle_shift_with_breaks(breaks)],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert!(solution.violations.is_none());
    assert_eq!(get_ids_from_tour(&solution.tours[0]), expected);
}
