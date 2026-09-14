use super::*;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

fn shift_with_job_times(earliest_first: Option<&str>, latest_last: Option<&str>) -> VehicleShift {
    VehicleShift {
        job_times: Some(JobTimeConstraints {
            earliest_first: earliest_first.map(|t| t.to_string()),
            latest_last: latest_last.map(|t| t.to_string()),
        }),
        ..create_default_vehicle_shift()
    }
}

#[test]
fn can_detect_job_started_before_earliest_first() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![shift_with_job_times(Some("2026-10-07T08:00:00Z"), None)],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .schedule_stamp(25_000., 27_700.)
                        .load(vec![0])
                        .distance(10)
                        .build_single("job1", "delivery"),
                ])
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_job_times(&ctx);

    assert_eq!(
        result,
        Err(vec![
            "job time bound violation: first job starts at 1970-01-01T06:56:40Z, earliest allowed is 2026-10-07T08:00:00Z, vehicle id 'my_vehicle_1', shift index: 0"
                .into()
        ])
    );
}

#[test]
fn can_accept_job_inside_bounds() {
    // same shape, bounds wide open on both sides
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![shift_with_job_times(Some("1970-01-01T00:00:00Z"), Some("2200-01-01T00:00:00Z"))],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .schedule_stamp(25_000., 27_700.)
                        .load(vec![0])
                        .distance(10)
                        .build_single("job1", "delivery"),
                ])
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(check_job_times(&ctx), Ok(()));
}

#[test]
fn can_detect_job_departed_after_latest_last() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![shift_with_job_times(None, Some("1970-01-01T00:00:00Z"))],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .schedule_stamp(25_000., 27_700.)
                        .load(vec![0])
                        .distance(10)
                        .build_single("job1", "delivery"),
                ])
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_job_times(&ctx);

    assert_eq!(
        result,
        Err(vec![
            "job time bound violation: last job departs at 1970-01-01T07:41:40Z, latest allowed is 1970-01-01T00:00:00Z, vehicle id 'my_vehicle_1', shift index: 0"
                .into()
        ])
    );
}

#[test]
fn can_ignore_break_only_stops_when_picking_first_and_last_job() {
    // A break-only stop precedes and follows the sole real job. The bounds sit tight around
    // that job's own timing: if a break stop were mistaken for a job stop on either end, its
    // arrival or departure would trip the bound instead.
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![shift_with_job_times(Some("1970-01-01T00:00:05Z"), Some("1970-01-01T00:00:06Z"))],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                    StopBuilder::default().schedule_stamp(2., 3.).load(vec![1]).build_single("break", "break"),
                    StopBuilder::default()
                        .schedule_stamp(5., 6.)
                        .load(vec![0])
                        .distance(5)
                        .build_single("job1", "delivery"),
                    StopBuilder::default().schedule_stamp(10., 12.).load(vec![0]).build_single("break", "break"),
                    StopBuilder::default().schedule_stamp(15., 15.).load(vec![0]).build_arrival(),
                ])
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(check_job_times(&ctx), Ok(()));
}

#[test]
fn can_accept_job_that_waits_for_its_own_window_before_starting() {
    // The vehicle physically arrives at 5, well before earliest_first (10), but the job's own
    // time window does not open until 10, so the vehicle waits and service starts at 10. That
    // is legal: the bound is on the service start, not on the raw arrival.
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![shift_with_job_times(Some("1970-01-01T00:00:10Z"), None)],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                    StopBuilder::default().schedule_stamp(5., 11.).load(vec![0]).distance(5).build_single_time(
                        "job1",
                        "delivery",
                        (10., 11.),
                    ),
                ])
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(check_job_times(&ctx), Ok(()));
}
