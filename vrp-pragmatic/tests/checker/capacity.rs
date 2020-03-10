use super::*;
use crate::extensions::MultiDimensionalCapacity as Capacity;
use std::iter::once;

/// Checks that vehicle load is assigned correctly. The following rules are checked:
/// * max vehicle's capacity is not violated
/// * load change is correct
pub fn check_vehicle_load(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each(|tour| {
        let capacity = Capacity::new(context.get_vehicle(tour.vehicle_id.as_str())?.capacity.clone());

        let legs = (0_usize..)
            .zip(tour.stops.windows(2))
            .map(|(idx, leg)| {
                (
                    idx,
                    match leg {
                        [from, to] => (from, to),
                        _ => panic!("Unexpected leg configuration"),
                    },
                )
            })
            .collect::<Vec<_>>();
        let intervals: Vec<Vec<(usize, (&Stop, &Stop))>> = legs
            .iter()
            .fold(Vec::<(usize, usize)>::default(), |mut acc, (idx, (_, to))| {
                let last_idx = legs.len() - 1;
                if is_reload_stop(context, to) || *idx == last_idx {
                    let start_idx = acc.last().map_or(0_usize, |item| item.1 + 2);
                    let end_idx = if *idx == last_idx { last_idx } else { *idx - 1 };

                    acc.push((start_idx, end_idx));
                }

                acc
            })
            .into_iter()
            .map(|(start_idx, end_idx)| {
                legs.iter().cloned().skip(start_idx).take(end_idx - start_idx + 1).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        intervals
            .iter()
            .try_fold::<_, _, Result<_, String>>(Capacity::default(), |acc, interval| {
                let (start_delivery, end_pickup) = interval
                    .iter()
                    .flat_map(|(_, (from, to))| once(from).chain(once(to)))
                    .zip(0..)
                    .filter_map(|(stop, idx)| if idx == 0 || idx % 2 == 1 { Some(stop) } else { None })
                    .flat_map(|stop| {
                        stop.clone()
                            .activities
                            .iter()
                            .map(move |activity| (activity.clone(), context.get_activity_type(tour, stop, activity)))
                    })
                    .try_fold::<_, _, Result<_, String>>(
                        (acc, Capacity::default()),
                        |acc, (activity, activity_type)| {
                            let activity_type = activity_type?;
                            let demand = get_demand(context, &activity, &activity_type)?;
                            Ok(match demand {
                                (DemandType::StaticDelivery, demand) => (acc.0 + demand, acc.1),
                                (DemandType::StaticPickup, demand) => (acc.0, acc.1 + demand),
                                (DemandType::StaticPickupDelivery, demand) => (acc.0 + demand.clone(), acc.1 + demand),
                                _ => acc,
                            })
                        },
                    )?;

                let end_capacity = interval.iter().try_fold(start_delivery, |acc, (idx, (from, to))| {
                    let from_load = Capacity::new(from.load.clone());
                    let to_load = Capacity::new(to.load.clone());

                    if from_load > capacity || to_load > capacity {
                        return Err(format!("Load exceeds capacity in tour '{}'", tour.vehicle_id));
                    }

                    let change = to.activities.iter().try_fold::<_, _, Result<_, String>>(
                        Capacity::default(),
                        |acc, activity| {
                            let activity_type = context.get_activity_type(tour, to, activity)?;
                            let (demand_type, demand) =
                                if activity.activity_type == "arrival" || activity.activity_type == "reload" {
                                    (DemandType::StaticDelivery, end_pickup)
                                } else {
                                    get_demand(context, &activity, &activity_type)?
                                };

                            Ok(match demand_type {
                                DemandType::StaticDelivery | DemandType::DynamicDelivery => acc - demand,
                                DemandType::StaticPickup | DemandType::DynamicPickup => acc + demand,
                                DemandType::None | DemandType::StaticPickupDelivery => acc,
                            })
                        },
                    )?;

                    let is_from_valid = from_load == acc;
                    let is_to_valid = to_load == from_load + change;

                    if is_from_valid && is_to_valid {
                        Ok(to_load)
                    } else {
                        let message = match (is_from_valid, is_to_valid) {
                            (true, false) => format!("at stop {}", idx + 1),
                            (false, true) => format!("at stop {}", idx),
                            _ => format!("at stops {}, {}", idx, idx + 1),
                        };

                        Err(format!("Load mismatch {} in tour '{}'", message, tour.vehicle_id))
                    }
                })?;

                Ok(end_capacity - end_pickup)
            })
            .map(|_| ())
    })
}

enum DemandType {
    None,
    StaticPickup,
    StaticDelivery,
    StaticPickupDelivery,
    DynamicPickup,
    DynamicDelivery,
}

fn get_demand(
    context: &CheckerContext,
    activity: &Activity,
    activity_type: &ActivityType,
) -> Result<(DemandType, Capacity), String> {
    let (is_dynamic, demand) = context.visit_job(
        activity,
        &activity_type,
        |job, task| {
            let is_dynamic = job.pickups.as_ref().map_or(false, |p| p.len() > 0)
                && job.deliveries.as_ref().map_or(false, |p| p.len() > 0);
            let demand = task.demand.clone().map_or_else(|| Capacity::default(), |d| Capacity::new(d));

            (is_dynamic, demand)
        },
        || (false, Capacity::default()),
    )?;

    let demand_type = match (is_dynamic, activity.activity_type.as_ref()) {
        (_, "replacement") => DemandType::StaticPickupDelivery,
        (true, "pickup") => DemandType::DynamicPickup,
        (true, "delivery") => DemandType::DynamicDelivery,
        (false, "pickup") => DemandType::StaticPickup,
        (false, "delivery") => DemandType::StaticDelivery,
        _ => DemandType::None,
    };

    Ok((demand_type, demand))
}

fn is_reload_stop(context: &CheckerContext, stop: &Stop) -> bool {
    context.get_stop_activity_types(stop).first().map_or(false, |a| a == "reload")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format_time;

    parameterized_test! {can_check_load, (stop_loads, expected_result), {
        can_check_load_impl(stop_loads, expected_result);
    }}

    can_check_load! {
        case00: ( vec![1, 1, 3, 1, 2, 1, 0], Ok(())),

        case01: ( vec![1, 2, 3, 1, 2, 1, 0], Err("Load mismatch at stop 1 in tour 'my_vehicle_1'".to_owned())),
        case02: ( vec![1, 1, 2, 1, 2, 1, 0], Err("Load mismatch at stops 2, 3 in tour 'my_vehicle_1'".to_owned())),
        case03: ( vec![1, 1, 3, 2, 2, 1, 0], Err("Load mismatch at stop 3 in tour 'my_vehicle_1'".to_owned())),
        case04: ( vec![1, 1, 3, 1, 1, 1, 0], Err("Load mismatch at stop 4 in tour 'my_vehicle_1'".to_owned())),
        case05: ( vec![1, 1, 3, 1, 2, 2, 0], Err("Load mismatch at stop 5 in tour 'my_vehicle_1'".to_owned())),

        case06_1: ( vec![10, 1, 3, 1, 2, 1, 0], Err("Load exceeds capacity in tour 'my_vehicle_1'".to_owned())),
        case06_2: ( vec![1, 1, 30, 1, 2, 1, 0], Err("Load exceeds capacity in tour 'my_vehicle_1'".to_owned())),
        case06_3: ( vec![1, 1, 3, 1, 20, 1, 0], Err("Load exceeds capacity in tour 'my_vehicle_1'".to_owned())),
    }

    fn can_check_load_impl(stop_loads: Vec<i32>, expected_result: Result<(), String>) {
        let problem = Problem {
            plan: Plan {
                jobs: vec![
                    create_delivery_job("job1", vec![1., 0.]),
                    create_delivery_job("job2", vec![2., 0.]),
                    create_delivery_job("job3", vec![3., 0.]),
                    create_pickup_job("job4", vec![4., 0.]),
                    create_pickup_delivery_job("job5", vec![1., 0.], vec![5., 0.]),
                ],
                relations: None,
            },
            fleet: Fleet {
                vehicles: vec![VehicleType {
                    shifts: vec![VehicleShift {
                        start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
                        end: Some(VehiclePlace {
                            time: format_time(1000.).to_string(),
                            location: vec![0., 0.].to_loc(),
                        }),
                        breaks: None,
                        reloads: Some(vec![VehicleReload {
                            times: None,
                            location: vec![0., 0.].to_loc(),
                            duration: 2.0,
                            tag: None,
                        }]),
                    }],
                    capacity: vec![5],
                    ..create_default_vehicle_type()
                }],
                profiles: create_default_profiles(),
            },
            config: None,
        };
        let matrix = create_matrix_from_problem(&problem);
        let solution = Solution {
            statistic: Statistic {
                cost: 13.,
                distance: 1,
                duration: 2,
                times: Timing { driving: 1, serving: 1, waiting: 0, break_time: 0 },
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
                        *stop_loads.get(0).unwrap(),
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    Stop {
                        location: vec![1., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:03Z".to_string(),
                            departure: "1970-01-01T00:00:05Z".to_string(),
                        },
                        distance: 1,
                        load: vec![*stop_loads.get(1).unwrap()],
                        activities: vec![
                            Activity {
                                job_id: "job1".to_string(),
                                activity_type: "delivery".to_string(),
                                location: None,
                                time: None,
                                job_tag: None,
                            },
                            Activity {
                                job_id: "job5".to_string(),
                                activity_type: "pickup".to_string(),
                                location: None,
                                time: None,
                                job_tag: None,
                            },
                        ],
                    },
                    Stop {
                        location: vec![0., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:03Z".to_string(),
                            departure: "1970-01-01T00:00:05Z".to_string(),
                        },
                        distance: 1,
                        load: vec![*stop_loads.get(2).unwrap()],
                        activities: vec![Activity {
                            job_id: "reload".to_string(),
                            activity_type: "reload".to_string(),
                            location: None,
                            time: None,
                            job_tag: None,
                        }],
                    },
                    Stop {
                        location: vec![2., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:07Z".to_string(),
                            departure: "1970-01-01T00:00:08Z".to_string(),
                        },
                        distance: 3,
                        load: vec![*stop_loads.get(3).unwrap()],
                        activities: vec![
                            Activity {
                                job_id: "job2".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![2., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:08Z".to_string(),
                                    end: "1970-01-01T00:00:09Z".to_string(),
                                }),
                                job_tag: None,
                            },
                            Activity {
                                job_id: "job3".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![3., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:09Z".to_string(),
                                    end: "1970-01-01T00:00:10Z".to_string(),
                                }),
                                job_tag: None,
                            },
                        ],
                    },
                    create_stop_with_activity(
                        "job4",
                        "pickup",
                        (4., 0.),
                        *stop_loads.get(4).unwrap(),
                        ("1970-01-01T00:00:11Z", "1970-01-01T00:00:12Z"),
                        5,
                    ),
                    create_stop_with_activity(
                        "job5",
                        "delivery",
                        (5., 0.),
                        *stop_loads.get(5).unwrap(),
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        6,
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        *stop_loads.get(6).unwrap(),
                        ("1970-01-01T00:00:19Z", "1970-01-01T00:00:19Z"),
                        11,
                    ),
                ],
                statistic: Statistic {
                    cost: 13.,
                    distance: 1,
                    duration: 2,
                    times: Timing { driving: 1, serving: 1, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        };

        let result = check_vehicle_load(&CheckerContext::new(problem, vec![matrix], solution));

        assert_eq!(result, expected_result);
    }
}
