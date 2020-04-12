use crate::checker::solve_and_check;
use crate::format::problem::*;
use crate::generator::*;

use crate::format::Location;
use proptest::prelude::*;

fn get_breaks() -> impl Strategy<Value = Option<Vec<VehicleBreak>>> {
    prop::collection::vec(generate_break(get_break_locations(), generate_durations(10..100), get_break_times()), 1..2)
        .prop_map(|reloads| Some(reloads))
}

fn get_break_locations() -> impl Strategy<Value = Option<Vec<Location>>> {
    prop_oneof![
        Just(None),
        generate_location(&DEFAULT_BOUNDING_BOX).prop_map(|location| Some(vec![location])),
        prop::collection::vec(generate_location(&DEFAULT_BOUNDING_BOX), 1..5).prop_map(|locations| Some(locations))
    ]
}

fn job_prototype() -> impl Strategy<Value = Job> {
    delivery_job_prototype(
        job_task_prototype(
            job_place_prototype(
                generate_location(&DEFAULT_BOUNDING_BOX),
                generate_durations(1..10),
                generate_no_time_windows(),
            ),
            generate_simple_demand(1..5),
            generate_no_tags(),
        ),
        generate_no_priority(),
        generate_no_skills(),
    )
}

fn get_break_times() -> impl Strategy<Value = VehicleBreakTime> {
    prop_oneof![get_break_offset_time(), get_break_time_windows()]
}

prop_compose! {
    fn get_break_offset_time()
    (
     start in 100..500,
     length in 10..200
    ) -> VehicleBreakTime {
        VehicleBreakTime::TimeOffset(vec![start as f64, (start + length) as f64])
    }
}

pub fn get_break_time_windows() -> impl Strategy<Value = VehicleBreakTime> {
    generate_multiple_time_windows_fixed(
        START_DAY,
        vec![from_hours(11), from_hours(13)],
        vec![from_hours(2), from_hours(4)],
        1..2,
    )
    .prop_map(|tws| VehicleBreakTime::TimeWindow(tws.first().unwrap().clone()))
}

prop_compose! {
    fn get_vehicle_type_with_breaks()
    (
     vehicle in default_vehicle_type_prototype(),
     breaks in get_breaks()
    ) -> VehicleType {
        assert_eq!(vehicle.shifts.len(), 1);

        let mut vehicle = vehicle;
        vehicle.shifts.first_mut().unwrap().breaks = breaks;

        vehicle
    }
}

prop_compose! {
    fn get_problem_with_breaks()
    (
     plan in generate_plan(generate_jobs(job_prototype(), 1..256)),
     fleet in generate_fleet(generate_vehicles(get_vehicle_type_with_breaks(), 1..4), default_profiles())
    ) -> Problem {
        Problem {
            plan,
            fleet,
            objectives: None,
            config: None,
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_breaks(problem in get_problem_with_breaks()) {
        let result = solve_and_check(problem, None);

        assert_eq!(result, Ok(()));
    }
}
