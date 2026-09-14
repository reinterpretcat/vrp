use crate::format::problem::*;
use crate::format::solution::solution_writer::create_tour;
use crate::format::solution::*;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::construction::enablers::ReservedTimeSpan;
use vrp_core::models::common::{TimeSpan, TimeWindow};
use vrp_core::models::examples::create_example_problem;

use crate::format::{JobTypeDimension, VehicleTypeDimension};
use vrp_core::construction::enablers::create_typed_actor_groups;
use vrp_core::models::problem::{JobIdDimension, JobTimeConstraints, JobTimeConstraintsDimension, Single};

type DomainProblem = vrp_core::models::Problem;
type DomainFleet = vrp_core::models::problem::Fleet;
type DomainPlace = vrp_core::models::solution::Place;
type DomainActivity = vrp_core::models::solution::Activity;
type DomainCommute = vrp_core::models::solution::Commute;
type DomainCommuteInfo = vrp_core::models::solution::CommuteInfo;
type DomainSchedule = vrp_core::models::common::Schedule;

fn create_test_problem_and_coord_index() -> (DomainProblem, CoordIndex) {
    let problem = {
        let mut problem = Arc::try_unwrap(create_example_problem()).unwrap_or_else(|_| unreachable!());
        problem.fleet = Arc::new(test_fleet());
        problem
    };
    let mut coord_index = CoordIndex::new(&create_empty_problem());
    coord_index.add(&Location::Reference { index: 0 });

    (problem, coord_index)
}

#[test]
fn can_create_solution() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![1])
                            .distance(10)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(16., 17.)
                            .load(vec![0])
                            .distance(15)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(22., 22.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_merge_activities_with_same_location_in_one_stop() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (5., 0.))],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert_eq!(
        solution.statistic,
        Statistic {
            cost: 32.,
            distance: 10,
            duration: 12,
            times: Timing { driving: 10, serving: 2, ..Timing::default() },
        }
    );
    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.tours.first().unwrap().stops.len(), 3);
    assert_eq!(solution.tours.first().unwrap().stops.get(1).unwrap().activities().len(), 2);
}

parameterized_test! {can_merge_activities_with_commute_in_one_stop, (jobs_data, expected), {
    can_merge_activities_with_commute_in_one_stop_impl(jobs_data, expected);
}}

can_merge_activities_with_commute_in_one_stop! {
    case_01: (
        vec![(1, None), (1, None), (2, None)],
        vec![(1, vec![(Some(1), None), (Some(1), None)]), (2, vec![(None, None)])]
    ),
    case_02: (
        vec![(1, Some((0., 0.))), (2, Some((1., 1.))), (3, Some((1., 2.)))],
        vec![(1, vec![(Some(1), Some((0., 0.))), (Some(2), Some((1., 1.))), (Some(3), Some((1., 2.)))])]
    ),
    case_03: (
        vec![(1, Some((0., 0.))), (1, Some((0., 0.))), (2, Some((1., 1.)))],
        vec![(1, vec![(Some(1), Some((0., 0.))), (Some(1), Some((0., 0.))), (Some(2), Some((1., 1.)))])]
    ),
    case_04: (
        vec![(1, Some((0., 0.))), (2, Some((1., 1.))), (3, Some((0., 0.))), (4, Some((1., 1.)))],
        vec![
            (1, vec![(Some(1), Some((0., 0.))), (Some(2), Some((1., 1.)))]),
            (3, vec![(Some(3), Some((0., 0.))), (Some(4), Some((1., 1.)))]),
        ]
    ),
}

#[allow(clippy::type_complexity)]
fn can_merge_activities_with_commute_in_one_stop_impl(
    jobs_data: Vec<(usize, Option<(Float, Float)>)>,
    expected: Vec<(usize, Vec<(Option<usize>, Option<(Float, Float)>)>)>,
) {
    let (problem, mut coord_index) = create_test_problem_and_coord_index();
    let activities = jobs_data
        .into_iter()
        .map(|(index, commute)| {
            coord_index.add(&Location::Reference { index });
            let arrival = index as Float;
            let commute = commute.map(|(f, b)| DomainCommute {
                forward: DomainCommuteInfo { location: 0, distance: f, duration: f },
                backward: DomainCommuteInfo { location: 0, distance: b, duration: b },
            });
            let departure = arrival + commute.as_ref().map(|c| c.forward.duration + c.backward.duration).unwrap_or(0.);
            DomainActivity {
                schedule: DomainSchedule { arrival, departure },
                commute,
                ..create_activity_with_job_at_location(create_single(&format!("job{index}")), index)
            }
        })
        .collect();
    let route = create_route_with_activities(&problem.fleet, "v1", activities);

    let tour = create_tour(&problem, &route, &coord_index, &Default::default());

    assert_eq!(expected.len(), tour.stops.len() - 2);
    expected.iter().zip(tour.stops.iter().skip(1)).for_each(|((expected_stop_idx, expected_acts), actual_stop)| {
        assert_eq!(Some(*expected_stop_idx), coord_index.get_by_loc(&actual_stop.as_point().unwrap().location));

        assert_eq!(expected_acts.len(), actual_stop.activities().len());
        expected_acts.iter().zip(actual_stop.activities().iter()).for_each(|((location, commute), actual)| {
            assert_eq!(*location, actual.location.as_ref().and_then(|l| coord_index.get_by_loc(l)));

            match (commute, &actual.commute) {
                (Some(expected), Some(actual)) => {
                    let check_commute = |expected: Float, info: Option<&CommuteInfo>| {
                        if expected == 0. {
                            assert!(info.is_none())
                        } else {
                            assert_eq!(expected, info.unwrap().time.duration());
                        }
                    };

                    check_commute(expected.0, actual.forward.as_ref());
                    check_commute(expected.1, actual.backward.as_ref());
                }
                (Some(_), None) => unreachable!("expected to have commute"),
                (None, Some(_)) => unreachable!("unexpected commute"),
                (None, None) => {}
            }
        });
    });
}

#[test]
fn can_merge_required_break_on_stop_arrival_time_properly() {
    let (problem, mut coord_index) = create_test_problem_and_coord_index();
    coord_index.add(&Location::Reference { index: 1 });
    let activities = vec![DomainActivity {
        schedule: DomainSchedule { arrival: 4., departure: 5. },
        ..create_activity_with_job_at_location(create_single(&format!("job{}", 1)), 1)
    }];
    let mut route = create_route_with_activities(&problem.fleet, "v1", activities);
    route.tour.all_activities_mut().last().unwrap().schedule.arrival = 6.;
    let reserved_times_index = vec![(
        route.actor.clone(),
        vec![ReservedTimeSpan { time: TimeSpan::Window(TimeWindow::new(4., 4.)), duration: 1. }],
    )]
    .into_iter()
    .collect();

    let tour = create_tour(&problem, &route, &coord_index, &reserved_times_index);

    assert_eq!(tour.stops.len(), 3);
    assert_eq!(get_ids_from_tour(&tour).into_iter().flatten().filter(|id| id == "break").count(), 1);
}

fn create_break_single(id: &str) -> Arc<Single> {
    let mut single = create_single_with_location(Some(DEFAULT_JOB_LOCATION));
    single.dimens.set_job_id(id.to_string()).set_job_type("break".to_string());

    Arc::new(single)
}

fn create_fleet_with_appointment_bound(earliest_first: Float) -> DomainFleet {
    let mut vehicle = test_vehicle("v1");
    vehicle
        .dimens
        .set_job_time_constraints(JobTimeConstraints { earliest_first: Some(earliest_first), latest_last: None });

    DomainFleet::new(vec![Arc::new(test_driver())], vec![Arc::new(vehicle)], |actors| {
        create_typed_actor_groups(actors, |a| a.vehicle.dimens.get_vehicle_type().cloned().expect("no vehicle type"))
    })
}

#[test]
fn does_not_hold_a_break_back_to_the_appointment_bound() {
    let (mut problem, mut coord_index) = create_test_problem_and_coord_index();
    problem.fleet = Arc::new(create_fleet_with_appointment_bound(50.));
    coord_index.add(&Location::Reference { index: 1 });
    coord_index.add(&Location::Reference { index: 2 });

    let activities = vec![
        // a break served at 5, long before the bound: the solver did not hold it back and neither
        // may the writer, or the tour reports a wait of 45 that nobody took
        DomainActivity {
            place: DomainPlace { idx: 0, location: 1, duration: 2., time: TimeWindow::new(0., 1000.) },
            schedule: DomainSchedule { arrival: 5., departure: 7. },
            ..create_activity_with_job_at_location(create_break_single("break"), 1)
        },
        // an appointment arriving at 40 and held back to the bound by the clamp
        DomainActivity {
            place: DomainPlace { idx: 0, location: 2, duration: 1., time: TimeWindow::new(0., 1000.) },
            schedule: DomainSchedule { arrival: 40., departure: 51. },
            ..create_activity_with_job_at_location(create_single("job1"), 2)
        },
    ];
    let route = create_route_with_activities(&problem.fleet, "v1", activities);

    let tour = create_tour(&problem, &route, &coord_index, &Default::default());

    let reported_service = |stop: &Stop| {
        let activity = stop.activities().last().expect("a stop carries an activity");
        // read it the way the checker does: the interval when present, the stop schedule otherwise
        activity.time.as_ref().map_or_else(
            || (stop.schedule().arrival.clone(), stop.schedule().departure.clone()),
            |time| (time.start.clone(), time.end.clone()),
        )
    };

    assert_eq!(reported_service(tour.stops.get(1).unwrap()), (format_time(5.), format_time(7.)));
    assert_eq!(reported_service(tour.stops.get(2).unwrap()), (format_time(50.), format_time(51.)));
    assert_eq!(tour.statistic.times.waiting, 10, "only the appointment waited, from 40 to the bound at 50");
}
