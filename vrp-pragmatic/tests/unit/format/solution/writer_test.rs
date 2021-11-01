use crate::format::problem::*;
use crate::format::solution::writer::create_tour;
use crate::format::solution::*;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::models::examples::create_example_problem;
use vrp_core::utils::as_mut;

type DomainActivity = vrp_core::models::solution::Activity;
type DomainCommute = vrp_core::models::solution::Commute;
type DomainSchedule = vrp_core::models::common::Schedule;

#[test]
fn can_create_solution() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("my_vehicle")],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 52.,
                distance: 20,
                duration: 22,
                times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        10
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (5., 0.),
                        0,
                        ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z"),
                        15
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:22Z"),
                        20
                    )
                ],
                statistic: Statistic {
                    cost: 52.,
                    distance: 20,
                    duration: 22,
                    times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_merge_activities_with_same_location_in_one_stop() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![5., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("my_vehicle")],
            profiles: create_default_matrix_profiles(),
        },
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
            times: Timing { driving: 10, serving: 2, waiting: 0, break_time: 0 },
        }
    );
    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.tours.first().unwrap().stops.len(), 3);
    assert_eq!(solution.tours.first().unwrap().stops.get(1).unwrap().activities.len(), 2);
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

fn can_merge_activities_with_commute_in_one_stop_impl(
    jobs_data: Vec<(usize, Option<(f64, f64)>)>,
    expected: Vec<(usize, Vec<(Option<usize>, Option<(f64, f64)>)>)>,
) {
    let problem = {
        let mut problem = Arc::try_unwrap(create_example_problem()).unwrap_or_else(|_| unreachable!());
        problem.fleet = Arc::new(test_fleet());
        unsafe {
            as_mut(problem.extras.as_ref()).insert("capacity_type".to_string(), Arc::new("single".to_string()));
        }

        problem
    };
    let mut coord_index = CoordIndex::new(&create_empty_problem());
    coord_index.add(&Location::Reference { index: 0 });
    let activities = jobs_data
        .into_iter()
        .map(|(index, commute)| {
            coord_index.add(&Location::Reference { index });
            let arrival = index as f64;
            let commute = commute.map(|(f, b)| DomainCommute { forward: (0., f), backward: (0., b) });
            DomainActivity {
                schedule: DomainSchedule {
                    arrival,
                    departure: commute.as_ref().map(|c| c.forward.1 + c.backward.1).unwrap_or(0.),
                },
                commute,
                ..create_activity_with_job_at_location(create_single(&format!("job{}", index)), index)
            }
        })
        .collect();
    let route = create_route_with_activities(&problem.fleet, "v1", activities);

    let tour = create_tour(&problem, &route, &coord_index);
    assert_eq!(expected.len(), tour.stops.len() - 2);
    expected.iter().zip(tour.stops.iter().skip(1)).for_each(|((expected_stop_idx, expected_acts), actual_stop)| {
        assert_eq!(Some(*expected_stop_idx), coord_index.get_by_loc(&actual_stop.location));

        assert_eq!(expected_acts.len(), actual_stop.activities.len());
        expected_acts.iter().zip(actual_stop.activities.iter()).for_each(|((location, commute), actual)| {
            assert_eq!(*location, actual.location.as_ref().and_then(|l| coord_index.get_by_loc(l)));

            match (commute, &actual.commute) {
                (Some(expected), Some(actual)) => {
                    assert_eq!(expected.0, actual.forward_duration);
                    assert_eq!(expected.1, actual.backward_duration);
                }
                (Some(_), None) => unreachable!("expected to have commute"),
                (None, Some(_)) => unreachable!("unexpected commute"),
                (None, None) => {}
            }
        });
    });
}
