use super::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

fn window(earliest: f64, latest: f64, fallback: Option<&str>) -> VisitWindowJson {
    VisitWindowJson {
        earliest: format_time(earliest),
        latest: format_time(latest),
        fallback: fallback.map(str::to_string),
        bridge: false,
    }
}

/// One shift with an `own` window that falls back to a `fallback` window when `overflow` is on,
/// one job of 30 units tagged `tag`, and a tour that arrives at `arrival` and departs at `departure`.
fn check(
    tag: &str,
    own: (f64, f64),
    fallback: Option<(f64, f64)>,
    overflow: bool,
    arrival: f64,
    departure: f64,
) -> Result<(), Vec<GenericError>> {
    let problem = Problem {
        plan: Plan {
            jobs: vec![Job {
                visit_window: Some(tag.to_string()),
                ..create_delivery_job_with_duration("job1", (1., 0.), 30.)
            }],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    visit_windows: Some(
                        std::iter::once(("own".to_string(), window(own.0, own.1, overflow.then_some("fallback"))))
                            .chain(
                                fallback
                                    .map(|(earliest, latest)| ("fallback".to_string(), window(earliest, latest, None))),
                            )
                            .collect(),
                    ),
                    ..create_default_vehicle_shift()
                }],
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
                        .schedule_stamp(arrival, departure)
                        .load(vec![0])
                        .distance(1)
                        .build_single("job1", "delivery"),
                ])
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    check_visit_windows(&ctx)
}

#[test]
fn accepts_a_stop_inside_its_own_window() {
    assert!(check("own", (100., 400.), None, false, 150., 180.).is_ok());
}

#[test]
fn accepts_a_stop_that_arrived_early_and_waited_for_its_window() {
    assert!(check("own", (100., 400.), None, false, 50., 130.).is_ok());
}

#[test]
fn rejects_a_stop_ending_after_its_window() {
    assert!(check("own", (100., 400.), None, false, 390., 420.).is_err());
}

#[test]
fn rejects_a_stop_served_before_its_window() {
    assert!(check("own", (100., 400.), None, false, 50., 80.).is_err());
}

#[test]
fn accepts_a_stop_inside_its_fallback() {
    assert!(check("own", (100., 400.), Some((100., 800.)), true, 600., 630.).is_ok());
}

#[test]
fn rejects_a_stop_outside_its_chain() {
    assert!(check("fallback", (100., 400.), Some((100., 800.)), false, 790., 820.).is_err());
}
