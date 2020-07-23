pub const SIMPLE_PROBLEM: &str = r#"
{
  "plan": {
    "jobs": [
      {
        "id": "single_job",
        "deliveries": [
          {
            "places": [
              {
                "location": {
                  "lat": 52.5622847,
                  "lng": 13.4023099
                },
                "duration": 240.0,
                "times": [
                  [
                    "2019-07-04T10:00:00Z",
                    "2019-07-04T16:00:00Z"
                  ]
                ]
              }
            ],
            "demand": [
              1
            ]
          }
        ]
      },
      {
        "id": "multi_job",
        "pickups": [
          {
            "places": [
              {
                "location": {
                  "lat": 52.5622847,
                  "lng": 13.4023099
                },
                "duration": 240.0
              }
            ],
            "demand": [
              1
            ],
            "tag": "p1"
          },
          {
            "places": [
              {
                "location": {
                  "lat": 52.5330881,
                  "lng": 13.3973059
                },
                "duration": 240.0
              }
            ],
            "demand": [
              1
            ],
            "tag": "p2"
          }
        ],
        "deliveries": [
          {
            "places": [
              {
                "location": {
                  "lat": 52.5252832,
                  "lng": 13.4188422
                },
                "duration": 240.0
              }
            ],
            "demand": [
              2
            ],
            "tag": "d1"
          }
        ]
      }
    ]
  },
  "fleet": {
    "vehicles": [
      {
        "typeId": "vehicle",
        "vehicleIds": [
          "vehicle_1"
        ],
        "profile": "normal_car",
        "costs": {
          "fixed": 22.0,
          "distance": 0.0002,
          "time": 0.004806
        },
        "shifts": [
          {
            "start": {
              "earliest": "2019-07-04T09:00:00Z",
              "latest": "2019-07-04T09:30:00Z",
              "location": {
                "lat": 52.4664257,
                "lng": 13.2812488
              }
            },
            "end": {
              "earliest": "2019-07-04T17:30:00Z",
              "latest": "2019-07-04T18:00:00Z",
              "location": {
                "lat": 52.4664257,
                "lng": 13.2812488
              }
            }
          }
        ],
        "capacity": [
          10
        ]
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

pub const SIMPLE_MATRIX: &str = r#"
{
  "profile": "normal_car",
  "travelTimes": [
    0,
    939,
    1077,
    2251,
    1003,
    0,
    645,
    2220,
    1068,
    701,
    0,
    2385,
    2603,
    2420,
    2597,
    0
  ],
  "distances": [
    0,
    4870,
    5113,
    17309,
    4580,
    0,
    2078,
    16983,
    5306,
    2688,
    0,
    15180,
    19743,
    14154,
    14601,
    0
  ]
}
"#;
