use super::*;
use crate::format::problem::*;
use crate::format::Location;
use crate::helpers::{create_empty_plan, create_empty_problem, create_job, solve_with_metaheuristic_and_iterations};

/// This is unstable test: it fails with capacity violation message from the solution checker.
/// It seems the reason is that pipeline wrongly omits extra state update which is fixed in
/// related change.
#[test]
fn can_handle_properly_invalid_break_removal() {
    for _ in 0..REPEAT_COUNT_MEDIUM {
        let problem = Problem {
            plan: Plan {
                jobs: vec![
                    Job {
                        deliveries: Some(vec![JobTask {
                            places: vec![JobPlace {
                                location: Location::Coordinate { lat: 52.437842517427846, lng: 13.3829646081322 },
                                duration: 1.0,
                                times: Some(vec![vec![
                                    "2020-07-04T09:00:00Z".to_string(),
                                    "2020-07-04T13:00:00Z".to_string(),
                                ]]),
                                tag: None,
                            }],
                            demand: Some(vec![1]),
                            order: None,
                        }]),
                        ..create_job("job1")
                    },
                    Job {
                        deliveries: Some(vec![JobTask {
                            places: vec![JobPlace {
                                location: Location::Coordinate { lat: 52.504574435265766, lng: 13.512204487216097 },
                                duration: 2.0,
                                times: Some(vec![vec![
                                    "2020-07-04T09:00:00Z".to_string(),
                                    "2020-07-04T11:00:00Z".to_string(),
                                ]]),
                                tag: None,
                            }],
                            demand: Some(vec![1]),
                            order: None,
                        }]),
                        ..create_job("job2")
                    },
                    Job {
                        pickups: Some(vec![JobTask {
                            places: vec![JobPlace {
                                location: Location::Coordinate { lat: 52.51627010959871, lng: 13.515165894434492 },
                                duration: 3.0,
                                times: Some(vec![
                                    vec!["2020-07-04T09:00:00Z".to_string(), "2020-07-04T13:00:00Z".to_string()],
                                    vec!["2020-07-04T14:00:00Z".to_string(), "2020-07-04T16:00:00Z".to_string()],
                                ]),
                                tag: None,
                            }],
                            demand: Some(vec![1]),
                            order: None,
                        }]),
                        ..create_job("job3")
                    },
                    Job {
                        pickups: Some(vec![JobTask {
                            places: vec![JobPlace {
                                location: Location::Coordinate { lat: 52.49739587223939, lng: 13.499267072502096 },
                                duration: 4.0,
                                times: Some(vec![vec![
                                    "2020-07-04T14:00:00Z".to_string(),
                                    "2020-07-04T16:00:00Z".to_string(),
                                ]]),
                                tag: None,
                            }],
                            demand: Some(vec![2]),
                            order: None,
                        }]),
                        ..create_job("job4")
                    },
                    Job {
                        deliveries: Some(vec![JobTask {
                            places: vec![JobPlace {
                                location: Location::Coordinate { lat: 52.47816437518683, lng: 13.480325156196248 },
                                duration: 5.0,
                                times: Some(vec![
                                    vec!["2020-07-04T09:00:00Z".to_string(), "2020-07-04T11:00:00Z".to_string()],
                                    vec!["2020-07-04T14:00:00Z".to_string(), "2020-07-04T16:00:00Z".to_string()],
                                ]),
                                tag: None,
                            }],
                            demand: Some(vec![3]),
                            order: None,
                        }]),
                        ..create_job("job5")
                    },
                    Job {
                        pickups: Some(vec![JobTask {
                            places: vec![JobPlace {
                                location: Location::Coordinate { lat: 52.44030727908021, lng: 13.433537947080476 },
                                duration: 6.0,
                                times: Some(vec![vec![
                                    "2020-07-04T14:00:00Z".to_string(),
                                    "2020-07-04T18:00:00Z".to_string(),
                                ]]),
                                tag: None,
                            }],
                            demand: Some(vec![1]),
                            order: None,
                        }]),
                        ..create_job("job6")
                    },
                ],
                ..create_empty_plan()
            },
            fleet: Fleet {
                vehicles: vec![VehicleType {
                    type_id: "vehicle1".to_string(),
                    vehicle_ids: vec!["vehicle1_1".to_string()],
                    profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                    costs: VehicleCosts { fixed: Some(20.), distance: 0.002, time: 0.003 },
                    shifts: vec![VehicleShift {
                        start: ShiftStart {
                            earliest: "2020-07-04T09:00:00Z".to_string(),
                            latest: None,
                            location: Location::Coordinate { lat: 52.44105158292253, lng: 13.424429791168873 },
                        },
                        end: Some(ShiftEnd {
                            earliest: None,
                            latest: "2020-07-04T18:00:00Z".to_string(),
                            location: Location::Coordinate { lat: 52.44105158292253, lng: 13.424429791168873 },
                        }),
                        dispatch: None,
                        breaks: Some(vec![VehicleBreak {
                            time: VehicleBreakTime::TimeWindow(vec![
                                "2020-07-04T12:00:00Z".to_string(),
                                "2020-07-04T14:00:00Z".to_string(),
                            ]),
                            places: vec![VehicleBreakPlace { duration: 3600.0, location: None, tag: None }],
                            policy: None,
                        }]),
                        reloads: None,
                    }],
                    capacity: vec![5],
                    skills: None,
                    limits: None,
                }],
                profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
            },
            ..create_empty_problem()
        };
        let matrices = create_approx_matrices(&problem);
        solve_with_metaheuristic_and_iterations(problem, Some(matrices), 1);
    }
}
