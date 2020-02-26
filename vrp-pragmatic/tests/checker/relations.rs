use crate::checker::CheckerContext;
use crate::json::problem::*;
use crate::json::solution::*;
use std::collections::HashSet;

/// Checks relation rules.
pub fn check_relations(context: &CheckerContext) -> Result<(), String> {
    (0_usize..)
        .zip(context.problem.plan.relations.as_ref().map_or(vec![].iter(), |relations| relations.iter()))
        .try_for_each(|(idx, relation)| {
            let tour = get_tour_by_vehicle_id(&relation.vehicle_id, relation.shift_index, &context.solution);
            // NOTE tour can be absent for tour relation
            let tour = if tour.is_err() {
                return match relation.type_field {
                    RelationType::Tour => Ok(()),
                    _ => tour.map(|_| ()),
                };
            } else {
                tour.unwrap()
            };

            let activity_ids = get_activity_ids(&tour);

            let relation_ids = relation.jobs.iter().collect::<HashSet<_>>();
            if relation_ids.len() != relation.jobs.len() {
                return Err(format!("Relation {} contains duplicated ids: {:?}", idx, relation.jobs));
            }

            match relation.type_field {
                RelationType::Sequence => {
                    let common = intersection(activity_ids.clone(), relation.jobs.clone());
                    if common != relation.jobs {
                        Err(format!(
                            "Relation {} does not follow sequence rule: expected {:?}, got {:?}, common: {:?}",
                            idx, relation.jobs, activity_ids, common
                        ))
                    } else {
                        Ok(())
                    }
                }
                RelationType::Flexible => {
                    let ids = activity_ids.iter().filter(|id| relation_ids.contains(id)).cloned().collect::<Vec<_>>();
                    if ids != relation.jobs {
                        Err(format!(
                            "Relation {} does not follow flexible rule: expected {:?}, got {:?}, common: {:?}",
                            idx, relation.jobs, activity_ids, ids
                        ))
                    } else {
                        Ok(())
                    }
                }
                RelationType::Tour => {
                    let has_wrong_assignment = context
                        .solution
                        .tours
                        .iter()
                        .filter(|other| tour.vehicle_id != other.vehicle_id)
                        .any(|tour| get_activity_ids(tour).iter().any(|id| relation_ids.contains(id)));

                    if has_wrong_assignment {
                        Err(format!("Relation {} has jobs assigned to another tour", idx))
                    } else {
                        Ok(())
                    }
                }
            }
        })?;

    Ok(())
}

fn get_tour_by_vehicle_id(vehicle_id: &str, shift_index: Option<usize>, solution: &Solution) -> Result<Tour, String> {
    solution
        .tours
        .iter()
        .find(|tour| tour.vehicle_id == vehicle_id && tour.shift_index == shift_index.unwrap_or(0))
        .cloned()
        .ok_or_else(|| format!("Cannot find tour for '{}'", vehicle_id))
}

fn get_activity_ids(tour: &Tour) -> Vec<String> {
    tour.stops
        .iter()
        .flat_map(|stop| {
            // TODO consider job tags within multi jobs
            stop.activities.iter().map(|a| a.job_id.clone())
        })
        .collect()
}

fn intersection<T>(left: Vec<T>, right: Vec<T>) -> Vec<T>
where
    T: PartialEq,
{
    let mut common = Vec::new();
    let mut right = right;

    for e1 in left.into_iter() {
        if let Some(pos) = right.iter().position(|e2| e1 == *e2) {
            common.push(e1);
            right.remove(pos);
        } else {
            if !common.is_empty() {
                break;
            }
        }
    }

    common
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format_time;
    use crate::helpers::*;

    mod single {
        use super::*;
        use crate::json::solution::Tour as VehicleTour;
        use RelationType::{Flexible, Sequence, Tour};

        fn create_relation(job_ids: Vec<&str>, relation_type: RelationType) -> Relation {
            Relation {
                type_field: relation_type,
                jobs: job_ids.iter().map(|id| id.to_string()).collect(),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }
        }

        fn create_relation_with_wrong_id(vehicle_id: &str) -> Relation {
            Relation {
                type_field: Flexible,
                jobs: vec!["job1".to_string()],
                vehicle_id: vehicle_id.to_string(),
                shift_index: None,
            }
        }

        fn create_relation_with_wrong_shift() -> Relation {
            Relation {
                type_field: Flexible,
                jobs: vec!["job1".to_string()],
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: Some(1),
            }
        }

        parameterized_test! {can_check_relations, (relations, expected_result), {
            can_check_relations_impl(relations, expected_result);
        }}

        can_check_relations! {
            case_sequence_01: (Some(vec![create_relation(vec!["departure", "job1", "job2"], Sequence)]), Ok(())),
            case_sequence_02: (Some(vec![create_relation(vec!["job1", "job2"], Sequence)]), Ok(())),
            case_sequence_03: (Some(vec![create_relation(vec!["job1", "job2"], Sequence),
                                         create_relation(vec!["job4", "job5"], Sequence)]), Ok(())),
            case_sequence_04: (Some(vec![create_relation(vec!["departure", "job1"], Sequence),
                                         create_relation(vec!["job3", "reload"], Sequence)]), Ok(())),
            case_sequence_05: (Some(vec![create_relation(vec!["departure", "job2", "job1"], Sequence)]), Err(())),
            case_sequence_06: (Some(vec![create_relation(vec!["departure", "job1", "job1"], Sequence)]), Err(())),
            case_sequence_07: (Some(vec![create_relation(vec!["departure", "job1", "job3"], Sequence)]), Err(())),
            case_sequence_08: (Some(vec![create_relation(vec!["job1", "job2", "job7"], Sequence)]), Err(())),

            case_flexible_01: (Some(vec![create_relation(vec!["departure", "job1", "job3"], Flexible)]), Ok(())),
            case_flexible_02: (Some(vec![create_relation(vec!["job1", "job3"], Flexible)]), Ok(())),
            case_flexible_03: (Some(vec![create_relation(vec!["departure", "job2", "job1"], Flexible)]), Err(())),

            case_tour_01:     (Some(vec![create_relation(vec!["departure", "job1", "job3"], Tour)]), Ok(())),
            case_tour_02:     (Some(vec![create_relation(vec!["job1", "job2"], Tour)]), Ok(())),
            case_tour_03:     (Some(vec![create_relation(vec!["job2", "job3"], Tour)]), Ok(())),
            case_tour_04:     (Some(vec![create_relation(vec!["job2", "job6"], Tour)]), Ok(())),

            case_mixed_01:    (Some(vec![create_relation(vec!["departure", "job1"], Sequence),
                                         create_relation(vec!["job3", "job4"], Flexible)]), Ok(())),

            case_wrong_vehicle_01: (Some(vec![create_relation_with_wrong_id("my_vehicle_2")]), Err(())),
            case_wrong_vehicle_02: (Some(vec![create_relation_with_wrong_id("my_vehicle_x")]), Err(())),
            case_wrong_vehicle_03: (Some(vec![create_relation_with_wrong_shift()]), Err(())),
        }

        fn can_check_relations_impl(relations: Option<Vec<Relation>>, expected_result: Result<(), ()>) {
            let problem = Problem {
                id: "my_problem".to_string(),
                plan: Plan {
                    jobs: vec![
                        create_delivery_job("job1", vec![1., 0.]),
                        create_delivery_job("job2", vec![2., 0.]),
                        create_pickup_job("job3", vec![3., 0.]),
                        create_delivery_job("job4", vec![4., 0.]),
                        create_pickup_job("job5", vec![5., 0.]),
                    ],
                    relations,
                },
                fleet: Fleet {
                    types: vec![VehicleType {
                        id: "my_vehicle".to_string(),
                        profile: "car".to_string(),
                        costs: create_default_vehicle_costs(),
                        shifts: vec![VehicleShift {
                            start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
                            end: Some(VehiclePlace {
                                time: format_time(1000.).to_string(),
                                location: vec![0., 0.].to_loc(),
                            }),
                            breaks: Some(vec![VehicleBreak {
                                times: VehicleBreakTime::TimeWindows(vec![vec![format_time(0.), format_time(1000.)]]),
                                duration: 2.0,
                                location: None,
                            }]),
                            reloads: Some(vec![VehicleReload {
                                times: None,
                                location: vec![0., 0.].to_loc(),
                                duration: 2.0,
                                tag: None,
                            }]),
                        }],
                        capacity: vec![5],
                        amount: 2,
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
                    cost: 51.,
                    distance: 16,
                    duration: 25,
                    times: Timing { driving: 16, serving: 9, waiting: 0, break_time: 2 },
                },
                tours: vec![
                    VehicleTour {
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
                            ),
                            create_stop_with_activity(
                                "job1",
                                "delivery",
                                (1., 0.),
                                1,
                                ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                            ),
                            Stop {
                                location: vec![2., 0.].to_loc(),
                                time: Schedule {
                                    arrival: "1970-01-01T00:00:03Z".to_string(),
                                    departure: "1970-01-01T00:00:06Z".to_string(),
                                },
                                load: vec![0],
                                activities: vec![
                                    Activity {
                                        job_id: "job2".to_string(),
                                        activity_type: "delivery".to_string(),
                                        location: None,
                                        time: None,
                                        job_tag: None,
                                    },
                                    Activity {
                                        job_id: "break".to_string(),
                                        activity_type: "break".to_string(),
                                        location: None,
                                        time: None,
                                        job_tag: None,
                                    },
                                ],
                            },
                            create_stop_with_activity(
                                "job3",
                                "pickup",
                                (3., 0.),
                                1,
                                ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                            ),
                            create_stop_with_activity(
                                "reload",
                                "reload",
                                (0., 0.),
                                1,
                                ("1970-01-01T00:00:11Z", "1970-01-01T00:00:13Z"),
                            ),
                            create_stop_with_activity(
                                "job4",
                                "delivery",
                                (4., 0.),
                                0,
                                ("1970-01-01T00:00:17Z", "1970-01-01T00:00:18Z"),
                            ),
                            create_stop_with_activity(
                                "job5",
                                "pickup",
                                (5., 0.),
                                1,
                                ("1970-01-01T00:00:19Z", "1970-01-01T00:00:20Z"),
                            ),
                            create_stop_with_activity(
                                "arrival",
                                "arrival",
                                (0., 0.),
                                0,
                                ("1970-01-01T00:00:25Z", "1970-01-01T00:00:25Z"),
                            ),
                        ],
                        statistic: Statistic {
                            cost: 51.,
                            distance: 16,
                            duration: 25,
                            times: Timing { driving: 16, serving: 9, waiting: 0, break_time: 2 },
                        },
                    },
                    VehicleTour {
                        vehicle_id: "my_vehicle_2".to_string(),
                        type_id: "my_vehicle".to_string(),
                        shift_index: 0,
                        stops: vec![],
                        statistic: Default::default(),
                    },
                ],
                unassigned: vec![],
                extras: None,
            };

            let result = check_relations(&CheckerContext::new(problem, vec![matrix], solution)).map_err(|_| ());

            assert_eq!(result, expected_result);
        }
    }
}
