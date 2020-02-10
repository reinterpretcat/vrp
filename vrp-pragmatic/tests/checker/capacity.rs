use crate::checker::CheckerContext;
use crate::extensions::MultiDimensionalCapacity;

/// Checks that vehicle load is assigned correctly. The following rules are checked:
/// * max vehicle's capacity is not violated
/// * load change is correct
pub fn check_vehicle_load(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each(|tour| {
        let capacity = MultiDimensionalCapacity::new(context.get_vehicle(tour.vehicle_id.as_str())?.capacity.clone());

        // TODO check load at departure stop
        (1..).zip(tour.stops.windows(2)).try_for_each(|(idx, leg)| {
            let (from, to) = match leg {
                [from, to] => (from, to),
                _ => return Err("Unexpected leg configuration".to_owned()),
            };

            let change = to.activities.iter().try_fold::<_, _, Result<_, String>>(
                MultiDimensionalCapacity::default(),
                |acc, activity| {
                    let activity_type = context.get_activity_type(tour, to, activity)?;
                    let demand = context.visit_job(
                        activity,
                        &activity_type,
                        |job| MultiDimensionalCapacity::new(job.demand.clone()),
                        |_, place| MultiDimensionalCapacity::new(place.demand.clone()),
                        || MultiDimensionalCapacity::default(),
                    )?;

                    Ok(if activity.activity_type == "pickup" { acc - demand } else { acc + demand })
                },
            )?;

            let old_load = MultiDimensionalCapacity::new(from.load.clone());
            let new_load = MultiDimensionalCapacity::new(to.load.clone());

            if old_load > capacity || new_load > capacity {
                return Err(format!("Load exceeds capacity in tour '{}'", tour.vehicle_id));
            }

            if new_load + change == old_load {
                Ok(())
            } else {
                Err(format!("Load mismatch at stop {} in tour '{}'", idx, tour.vehicle_id))
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::*;
    use crate::json::problem::*;
    use crate::json::solution::*;

    parameterized_test! {can_check_load, (stop_loads, expected_result), {
        can_check_load_impl(stop_loads, expected_result);
    }}

    can_check_load! {
        case01: ( vec![3, 2, 0, 1, 1], Ok(())),

        case02_1: ( vec![3, 1, 0, 1, 1], Err("Load mismatch at stop 1 in tour 'my_vehicle_1'".to_owned())),
        case02_2: ( vec![3, 3, 0, 1, 1], Err("Load mismatch at stop 1 in tour 'my_vehicle_1'".to_owned())),
        case03_1: ( vec![3, 2, 1, 1, 1], Err("Load mismatch at stop 2 in tour 'my_vehicle_1'".to_owned())),
        case04_1: ( vec![3, 2, 0, 0, 1], Err("Load mismatch at stop 3 in tour 'my_vehicle_1'".to_owned())),
        case04_2: ( vec![3, 2, 0, 2, 1], Err("Load mismatch at stop 3 in tour 'my_vehicle_1'".to_owned())),
        case05_1: ( vec![3, 2, 0, 1, 2], Err("Load mismatch at stop 4 in tour 'my_vehicle_1'".to_owned())),
        case05_2: ( vec![3, 2, 0, 1, 0], Err("Load mismatch at stop 4 in tour 'my_vehicle_1'".to_owned())),

        case06_1: ( vec![10, 2, 0, 1, 1], Err("Load exceeds capacity in tour 'my_vehicle_1'".to_owned())),
        case06_2: ( vec![3, 10, 0, 1, 1], Err("Load exceeds capacity in tour 'my_vehicle_1'".to_owned())),
    }

    fn can_check_load_impl(stop_loads: Vec<i32>, expected_result: Result<(), String>) {
        let problem = Problem {
            id: "my_problem".to_string(),
            plan: Plan {
                jobs: vec![
                    create_delivery_job("job1", vec![1., 0.]),
                    create_delivery_job("job2", vec![2., 0.]),
                    create_delivery_job("job3", vec![3., 0.]),
                    create_pickup_job("job4", vec![4., 0.]),
                ],
                relations: None,
            },
            fleet: Fleet {
                types: vec![VehicleType {
                    id: "my_vehicle".to_string(),
                    profile: "car".to_string(),
                    costs: create_default_vehicle_costs(),
                    shifts: vec![create_default_vehicle_shift()],
                    capacity: vec![5],
                    amount: 1,
                    skills: None,
                    limits: None,
                }],
                profiles: create_default_profiles(),
            },
            config: None,
        };
        let matrix = create_matrix_from_problem(&problem);
        let solution = Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 13.,
                distance: 1,
                duration: 2,
                times: Timing { driving: 1, serving: 1, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        *stop_loads.get(0).unwrap(),
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        *stop_loads.get(1).unwrap(),
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    ),
                    Stop {
                        location: vec![2., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:01:42Z".to_string(),
                            departure: "1970-01-01T00:01:45Z".to_string(),
                        },
                        load: vec![*stop_loads.get(2).unwrap()],
                        activities: vec![
                            Activity {
                                job_id: "job2".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![2., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:01:42Z".to_string(),
                                    end: "1970-01-01T00:01:43Z".to_string(),
                                }),
                                job_tag: None,
                            },
                            Activity {
                                job_id: "job3".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![3., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:01:43Z".to_string(),
                                    end: "1970-01-01T00:01:45Z".to_string(),
                                }),
                                job_tag: None,
                            },
                        ],
                    },
                    create_stop_with_activity(
                        "job4",
                        "pickup",
                        (4., 0.),
                        *stop_loads.get(3).unwrap(),
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        *stop_loads.get(4).unwrap(),
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
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
