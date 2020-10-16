use super::*;
use crate::helpers::generate::*;
use vrp_pragmatic::format::problem::*;
use vrp_pragmatic::format::Location;

#[test]
fn can_import_hre_problem() {
    let hre_problem = r#"
{
  "plan": {
    "jobs": [
      {
        "id": "simpleJob",
        "places": {
          "delivery": {
            "location": {
              "lat": 52.56,
              "lng": 13.40
            },
            "duration": 180
          }
        },
        "demand": [
          1
        ]
      },
      {
        "id": "multiJob",
        "places": {
          "pickups": [
            {
              "location": {
                "lat": 52.5622,
                "lng": 13.4023
              },
              "duration": 180,
              "demand": [
                1
              ],
              "tag": "p1"
            },
            {
              "location": {
                "lat": 52.533,
                "lng": 13.397
              },
              "duration": 180,
              "demand": [
                1
              ],
              "tag": "p2"
            }
          ],
          "deliveries": [
            {
              "location": {
                "lat": 52.5252,
                "lng": 13.4188
              },
              "duration": 180,
              "demand": [
                2
              ],
              "tag": "d1"
            }
          ]
        }
      }
    ],
    "relations": [
      {
        "type": "flexible",
        "jobs": [
          "simpleJob"
        ],
        "vehicleId": "vehicle_1"
      }
    ]
  },
  "fleet": {
    "types": [
      {
        "id": "vehicle",
        "profile": "normal_car",
        "costs": {
          "distance": 0.0002,
          "time": 0.005,
          "fixed": 25
        },
        "shifts": [
          {
            "start": {
              "time": "2020-01-01T00:00:00Z",
              "location": {
                "lat": 52.466,
                "lng": 13.281
              }
            },
            "end": {
              "time": "2020-01-01T08:00:00Z",
              "location": {
                "lat": 52.466,
                "lng": 13.281
              }
            },
            "breaks": [
              {
                "times": [
                  [
                    "2020-01-01T06:00:00Z",
                    "2020-01-01T08:00:00Z"
                  ]
                ],
                "duration": 1800
              }
            ],
            "reloads": [
              {
                "times": [
                  [
                    "2020-01-01T09:00:00Z",
                    "2020-01-01T12:00:00Z"
                  ]
                ],
                "duration": 1800,
                "location": {
                  "lat": 52.466,
                  "lng": 13.281
                }
              }
            ],
            "depots": [
              {
                "times": [
                  [
                    "2020-01-01T01:00:00Z",
                    "2020-01-01T02:00:00Z"
                  ]
                ],
                "duration": 1800,
                "location": {
                  "lat": 52.466,
                  "lng": 13.281
                }
              }
            ]
          }
        ],
        "capacity": [
          10
        ],
        "amount": 1,
        "limits": {
          "maxDistance": 1000000,
          "shiftTime": 14400
        }
      }
    ],
    "profiles": [
      {
        "name": "normal_car",
        "type": "car"
      }
    ]
  }
}
    "#;

    let problem = deserialize_hre_problem(BufReader::new(hre_problem.as_bytes())).expect("Cannot read hre problem");

    assert_eq!(problem.plan.jobs.len(), 2);
    assert_eq!(problem.plan.relations.as_ref().unwrap().len(), 1);

    let simple_job = problem.plan.jobs.first().unwrap();
    assert_eq!(simple_job.id, "simpleJob");
    assert_eq!(simple_job.deliveries.as_ref().unwrap().len(), 1);
    assert!(simple_job.pickups.is_none());
    assert!(simple_job.services.is_none());
    assert!(simple_job.replacements.is_none());

    let multi_job = problem.plan.jobs.last().unwrap();
    assert_eq!(multi_job.id, "multiJob");
    assert_eq!(multi_job.deliveries.as_ref().unwrap().len(), 1);
    assert_eq!(multi_job.pickups.as_ref().unwrap().len(), 2);
    assert!(multi_job.services.is_none());
    assert!(multi_job.replacements.is_none());

    assert_eq!(problem.fleet.vehicles.len(), 1);
    let vehicle = problem.fleet.vehicles.first().unwrap();
    assert_eq!(vehicle.vehicle_ids, vec!["vehicle_1"]);
    assert_eq!(vehicle.capacity, vec![10]);
    assert_eq!(vehicle.profile, "normal_car");

    assert!(vehicle.limits.is_some());
    let limits = vehicle.limits.as_ref().unwrap();
    assert_eq!(limits.max_distance, Some(1000000.));
    assert_eq!(limits.shift_time, Some(14400.));
    assert!(limits.allowed_areas.is_none());

    assert_eq!(vehicle.shifts.len(), 1);
    let shift = vehicle.shifts.first().unwrap();
    assert!(shift.end.is_some());
    assert_eq!(shift.breaks.as_ref().unwrap().len(), 1);
    assert_eq!(shift.depots.as_ref().unwrap().len(), 1);
    assert_eq!(shift.reloads.as_ref().unwrap().len(), 1);
}

#[test]
fn can_write_hre_problem() {
    let pragmatic_problem = Problem {
        plan: Plan {
            jobs: vec![
                Job {
                    id: "job1".to_string(),
                    pickups: Some(vec![JobTask {
                        places: vec![JobPlace {
                            location: Location::Coordinate { lat: 1., lng: 0. },
                            times: Some(vec![create_test_time_window()]),
                            ..create_empty_job_place()
                        }],
                        demand: Some(vec![1]),
                        ..create_empty_job_task()
                    }]),
                    skills: Some(JobSkills { all_of: Some(vec!["skill1".to_string()]), one_of: None, none_of: None }),
                    ..create_empty_job()
                },
                Job {
                    id: "job2".to_string(),
                    deliveries: Some(vec![JobTask {
                        places: vec![JobPlace {
                            location: Location::Coordinate { lat: 2., lng: 0. },
                            times: Some(vec![create_test_time_window()]),
                            ..create_empty_job_place()
                        }],
                        demand: Some(vec![1]),
                        ..create_empty_job_task()
                    }]),
                    ..create_empty_job()
                },
                Job {
                    id: "job3".to_string(),
                    pickups: Some(vec![JobTask {
                        places: vec![JobPlace {
                            location: Location::Coordinate { lat: 3., lng: 0. },
                            times: Some(vec![create_test_time_window()]),
                            ..create_empty_job_place()
                        }],
                        demand: Some(vec![1]),
                        ..create_empty_job_task()
                    }]),
                    deliveries: Some(vec![JobTask {
                        places: vec![JobPlace {
                            location: Location::Coordinate { lat: 4., lng: 0. },
                            times: Some(vec![create_test_time_window()]),
                            ..create_empty_job_place()
                        }],
                        demand: Some(vec![1]),
                        ..create_empty_job_task()
                    }]),
                    ..create_empty_job()
                },
                Job {
                    id: "job4".to_string(),
                    pickups: Some(vec![
                        JobTask {
                            places: vec![JobPlace {
                                location: Location::Coordinate { lat: 5., lng: 0. },
                                times: Some(vec![create_test_time_window()]),
                                ..create_empty_job_place()
                            }],
                            demand: Some(vec![1]),
                            ..create_empty_job_task()
                        },
                        JobTask {
                            places: vec![JobPlace {
                                location: Location::Coordinate { lat: 6., lng: 0. },
                                times: Some(vec![create_test_time_window()]),
                                ..create_empty_job_place()
                            }],
                            demand: Some(vec![1]),
                            ..create_empty_job_task()
                        },
                    ]),
                    deliveries: Some(vec![JobTask {
                        places: vec![JobPlace {
                            location: Location::Coordinate { lat: 4., lng: 0. },
                            times: Some(vec![create_test_time_window()]),
                            ..create_empty_job_place()
                        }],
                        demand: Some(vec![1]),
                        ..create_empty_job_task()
                    }]),
                    ..create_empty_job()
                },
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Any,
                    jobs: vec!["job1".to_string()],
                    vehicle_id: "vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: vec!["job1".to_string()],
                    vehicle_id: "vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Strict,
                    jobs: vec!["job1".to_string()],
                    vehicle_id: "vehicle_1".to_string(),
                    shift_index: None,
                },
            ]),
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                type_id: "vehicle".to_string(),
                vehicle_ids: vec!["vehicle_1".to_string()],
                profile: "car".to_string(),
                costs: VehicleCosts { fixed: None, distance: 0.0, time: 0.0 },
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: "2020-05-01T09:00:00.00Z".to_string(),
                        latest: None,
                        location: Location::Coordinate { lat: 0.0, lng: 0.0 },
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: "2020-05-01T18:00:00.00Z".to_string(),
                        location: Location::Coordinate { lat: 0.0, lng: 0.0 },
                    }),
                    depots: Some(vec![VehicleCargoPlace {
                        location: Location::Coordinate { lat: 0.0, lng: 0.0 },
                        duration: 1800.,
                        times: Some(vec![create_test_time_window()]),
                        tag: Some("depot".to_string()),
                    }]),
                    breaks: Some(vec![VehicleBreak {
                        time: VehicleBreakTime::TimeWindow(vec![
                            "2020-05-01T12:00:00.00Z".to_string(),
                            "2020-05-01T13:00:00.00Z".to_string(),
                        ]),
                        duration: 1800.,
                        locations: None,
                    }]),
                    reloads: None,
                }],
                capacity: vec![10],
                skills: Some(vec!["skill1".to_string()]),
                limits: Some(VehicleLimits {
                    max_distance: Some(10000.),
                    shift_time: Some(14400.),
                    allowed_areas: None,
                }),
            }],
            profiles: vec![Profile { name: "normal_car".to_string(), profile_type: "car".to_string(), speed: None }],
        },
        objectives: None,
        config: None,
    };
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };

    serialize_hre_problem(writer, &pragmatic_problem).unwrap();

    let hre_problem: models::Problem =
        serde_json::from_reader(BufReader::new(buffer.as_bytes())).expect("cannot read hre problem");

    // TODO improve check
    assert_eq!(hre_problem.plan.jobs.len(), 4);
    assert_eq!(hre_problem.plan.relations.map_or(0, |relations| relations.len()), 3);
    assert_eq!(hre_problem.fleet.types.len(), 1);
    assert_eq!(hre_problem.fleet.profiles.len(), 1);
}
