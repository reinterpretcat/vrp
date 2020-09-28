use super::*;

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

    let problem = read_hre_problem(BufReader::new(hre_problem.as_bytes())).expect("Cannot read hre problem");

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
