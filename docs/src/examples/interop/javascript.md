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
    import init, { get_routing_locations, convert_to_pragmatic, solve_pragmatic } from './pkg/vrp_cli.js';

    async function run() {
        await init();

        const pragmatic_problem_str = convert_to_pragmatic(
            // another supported json format type
            // NOTE you can use simple csv format too
            'hre',
            // array of strings
            [`
{
  "plan": {
    "jobs": [
      {
        "id": "job",
        "places": {
          "delivery": {
            "location": {
              "lat": 52.52599,
              "lng": 13.45413
            },
            "duration": 300
          }
        },
        "demand": [1]
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
          "fixed": 30
        },
        "shifts": [{
          "start": {
            "time": "2020-04-07T00:00:00Z",
            "location": {
              "lat": 52.5225,
              "lng": 13.4095
            }
          },
          "end": {
            "time": "2020-04-07T08:00:00Z",
            "location": {
              "lat": 52.5225,
              "lng": 13.4095
            }
          }
        }],
        "capacity": [2],
        "amount": 1
      }
    ],
    "profiles": [
      {
        "name": "normal_car",
        "type": "car"
      }
    ]
  }
}`]);
        console.log(`pragmatic problem is:\n ${pragmatic_problem_str}`);

        const pragmatic_problem = JSON.parse(pragmatic_problem_str);

        const locations = get_routing_locations(pragmatic_problem);
        console.log(`routing locations are:\n ${locations}`);

        // NOTE let's assume we got routing matrix data for locations somehow
        // NOTE or just pass an empty array to use great-circle distance approximation
        const matrix_data= [
            {
                "profile": "normal_car",
                "travelTimes": [
                    0, 609,
                    580, 0
                ],
                "distances": [
                    0, 3840,
                    3610, 0
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
