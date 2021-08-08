# Javascript

This is example how to call solver methods from **javascript** in browser. You need to build `vrp-cli` library for
`WebAssembly` target. To do this, you can use [wasm-pack](https://rustwasm.github.io/wasm-pack/installer):

    cd vrp-cli
    wasm-pack build --target web

It should generate `wasm` build + some javascript files for you. If you want to have a smaller binary, you can try
to build without default features: `csv-format`, `hre-format`, `scientific-format`.

To test it, use the following index.html file:

```html
<html>
<head>
    <meta content="text/html;charset=utf-8" http-equiv="Content-Type"/>
</head>
<body>
<script type="module">
    import init, { get_routing_locations, solve_pragmatic } from './pkg/vrp_cli.js';

    async function run() {
        await init();

        const pragmatic_problem = JSON.parse(`
{
  "plan": {
    "jobs": [
      {
        "id": "job1",
        "deliveries": [
          {
            "places": [
              {
                "location": {
                  "lat": 52.52599,
                  "lng": 13.45413
                },
                "duration": 300.0,
                "times": [
                  [
                    "2019-07-04T09:00:00Z",
                    "2019-07-04T18:00:00Z"
                  ],
                  [
                    "2019-07-05T09:00:00Z",
                    "2019-07-05T18:00:00Z"
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
        "id": "job2",
        "pickups": [
          {
            "places": [
              {
                "location": {
                  "lat": 52.5225,
                  "lng": 13.4095
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
        "id": "job3",
        "pickups": [
          {
            "places": [
              {
                "location": {
                  "lat": 52.5225,
                  "lng": 13.4095
                },
                "duration": 300.0
              }
            ],
            "demand": [
              1
            ],
            "tag": "p1"
          }
        ],
        "deliveries": [
          {
            "places": [
              {
                "location": {
                  "lat": 52.5165,
                  "lng": 13.3808
                },
                "duration": 300.0
              }
            ],
            "demand": [
              1
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
       "profile": {
          "matrix": "normal_car"
        },
        "costs": {
          "fixed": 22.0,
          "distance": 0.0002,
          "time": 0.004806
        },
        "shifts": [
          {
            "start": {
              "earliest": "2019-07-04T09:00:00Z",
              "location": {
                "lat": 52.5316,
                "lng": 13.3884
              }
            },
            "end": {
              "latest": "2019-07-04T18:00:00Z",
              "location": {
                "lat": 52.5316,
                "lng": 13.3884
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
        "name": "normal_car"
      }
    ]
  }
}
`);

        const locations = get_routing_locations(pragmatic_problem);
        console.log(`routing locations are:\n ${locations}`);

        // NOTE let's assume we got routing matrix data for locations somehow
        // NOTE or just pass an empty array to use great-circle distance approximation
        const matrix_data= [
            {
                "matrix": "normal_car",
                "travelTimes": [
                   0,    609, 981, 906,
                   813,  0,   371, 590,
                   1055, 514, 0,   439,
                   948,  511, 463,   0
                ],
                "distances": [
                   0,    3840,  5994,  5333,
                   4696, 0,     2154,  3226,
                   5763, 2674,  0,     2145,
                   5112, 2470,  2152,  0
                ]
            }
        ];

        // config provides the way to tweak algorithm behavior
        const config = {
            "termination": {
                 "maxTime": 10,
                 "maxGenerations": 1000
            }
        };

        const solution = solve_pragmatic(pragmatic_problem, matrix_data, config);
        console.log(`solution is:\n ${solution}`);
    }

    run();
</script>
</body>
</html>
```
