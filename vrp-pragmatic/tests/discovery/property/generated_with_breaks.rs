use crate::format::problem::*;
use crate::format::Location;
use crate::generator::*;
use crate::helpers::solve_with_metaheuristic_and_iterations;
use proptest::prelude::*;

fn disable_departure_time_optimization(mut vehicle: VehicleType) -> VehicleType {
    vehicle.shifts.iter_mut().for_each(|shift| {
        shift.start.latest = Some(shift.start.earliest.clone());
    });

    vehicle
}

mod optional {
    use super::*;

    fn get_optional_breaks() -> impl Strategy<Value = Option<Vec<VehicleBreak>>> {
        let places_proto = get_optional_break_places(
            prop_oneof![Just(None), generate_location(&DEFAULT_BOUNDING_BOX).prop_map(Some)],
            generate_durations(10..100),
        );
        let break_proto = generate_optional_break(
            prop::collection::vec(places_proto, 1..2),
            prop_oneof![get_optional_break_offset_time(), get_optional_break_time_windows()],
            Just(None),
        );

        prop::collection::vec(break_proto, 1..2).prop_map(Some)
    }

    prop_compose! {
        pub fn generate_optional_break(
          places_proto: impl Strategy<Value = Vec<VehicleOptionalBreakPlace>>,
          time_proto: impl Strategy<Value = VehicleOptionalBreakTime>,
          policy_proto: impl Strategy<Value = Option<VehicleOptionalBreakPolicy>>,
        )
        (
         places in places_proto,
         time in time_proto,
         policy in policy_proto,
        ) -> VehicleBreak {
            VehicleBreak::Optional {
                time,
                places,
                policy
            }
        }
    }

    prop_compose! {
        pub fn get_optional_break_places(
           locations: impl Strategy<Value = Option<Location>>,
           durations: impl Strategy<Value = f64>,
        )
        (
         location in locations,
         duration in durations
        ) -> VehicleOptionalBreakPlace {
            VehicleOptionalBreakPlace { location, duration, tag: None }
        }
    }

    prop_compose! {
        fn get_optional_break_offset_time()
        (
         start in 3600..14400,
         length in 600..1800
        ) -> VehicleOptionalBreakTime {
            VehicleOptionalBreakTime::TimeOffset(vec![start as f64, (start + length) as f64])
        }
    }

    prop_compose! {
        fn get_vehicle_type_with_optional_breaks()
        (
         vehicle in default_vehicle_type_prototype(),
         breaks in get_optional_breaks()
        ) -> VehicleType {
            with_breaks(disable_departure_time_optimization(vehicle), breaks)
        }
    }

    prop_compose! {
        pub(crate) fn get_problem_with_optional_breaks()
        (
         plan in generate_plan(generate_jobs(job_prototype(), 1..256)),
         fleet in generate_fleet(
            generate_vehicles(get_vehicle_type_with_optional_breaks(), 1..4),
            default_matrix_profiles())
        ) -> Problem {
            Problem { plan, fleet, objectives: None }
        }
    }

    fn get_optional_break_time_windows() -> impl Strategy<Value = VehicleOptionalBreakTime> {
        generate_multiple_time_windows_fixed(
            START_DAY,
            vec![from_hours(11), from_hours(13)],
            vec![from_hours(2), from_hours(4)],
            1..2,
        )
        .prop_map(|tws| VehicleOptionalBreakTime::TimeWindow(tws.first().unwrap().clone()))
    }
}

mod required {
    use super::*;
    use crate::{format_time, parse_time};

    fn from_hours_as_usize(hours: i32) -> i32 {
        parse_time(START_DAY) as i32 + from_hours(hours).as_secs() as i32
    }

    fn get_required_breaks() -> impl Strategy<Value = Option<Vec<VehicleBreak>>> {
        let break_proto = generate_required_break(
            prop_oneof![get_required_break_offset_time(), get_required_break_exact_time()],
            generate_durations(1..3600),
        );

        prop::collection::vec(break_proto, 1..2).prop_map(Some)
    }

    prop_compose! {
        pub fn generate_required_break(
          time_proto: impl Strategy<Value = VehicleRequiredBreakTime>,
          duration_proto: impl Strategy<Value = f64>,
        )
        (
         time in time_proto,
         duration in duration_proto,
        ) -> VehicleBreak {
            VehicleBreak::Required { time, duration }
        }
    }

    prop_compose! {
        fn get_required_break_offset_time()
        (
         time in 3600..14400,
        ) -> VehicleRequiredBreakTime {
            let time = time as f64;
            VehicleRequiredBreakTime::OffsetTime { earliest: time - 10., latest : time}
        }
    }

    prop_compose! {
        fn get_required_break_exact_time()
        (
         time in from_hours_as_usize(10)..from_hours_as_usize(13),
        ) -> VehicleRequiredBreakTime {
            let time = time as f64;
            VehicleRequiredBreakTime::ExactTime{ earliest: format_time(time - 1.), latest: format_time(time) }
        }
    }

    prop_compose! {
        fn get_vehicle_type_with_required_breaks()
        (
         vehicle in default_vehicle_type_prototype(),
         breaks in get_required_breaks()
        ) -> VehicleType {
           with_breaks(disable_departure_time_optimization(vehicle), breaks)
        }
    }

    prop_compose! {
        pub(crate) fn get_problem_with_required_breaks()
        (
         plan in generate_plan(generate_jobs(job_prototype(), 1..256)),
         fleet in generate_fleet(
            generate_vehicles(get_vehicle_type_with_required_breaks(), 1..4),
            default_matrix_profiles())
        ) -> Problem {
            Problem { plan, fleet, objectives: None }
        }
    }
}

fn with_breaks(vehicle: VehicleType, breaks: Option<Vec<VehicleBreak>>) -> VehicleType {
    assert_eq!(vehicle.shifts.len(), 1);

    let mut vehicle = vehicle;
    vehicle.shifts.first_mut().unwrap().breaks = breaks;

    vehicle
}

fn job_prototype() -> impl Strategy<Value = Job> {
    delivery_job_prototype(
        job_task_prototype(
            job_place_prototype(
                generate_location(&DEFAULT_BOUNDING_BOX),
                generate_durations(1..10),
                generate_no_time_windows(),
                generate_no_tags(),
            ),
            generate_simple_demand(1..5),
            generate_no_order(),
        ),
        generate_no_jobs_skills(),
        generate_no_jobs_value(),
        generate_no_jobs_group(),
        generate_no_jobs_compatibility(),
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_optional_breaks(problem in optional::get_problem_with_optional_breaks()) {
        solve_with_metaheuristic_and_iterations(problem, None, 10);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    #[ignore]
    fn can_solve_problem_with_required_breaks(problem in required::get_problem_with_required_breaks()) {
        solve_with_metaheuristic_and_iterations(problem, None, 10);
    }
}
