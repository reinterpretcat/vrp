# Javascript

This is example how to call solver methods from **javascript**. You need to build `vrp-cli` library for the
`WebAssembly` target. To do this, you can use [wasm-pack](https://rustwasm.github.io/wasm-pack/installer), picking the
target that matches where the code runs:

```shell
pip install -r vrp-cli/bindings/python/requirements-codegen.txt
npm ci --prefix vrp-cli/bindings/typescript
./vrp-cli/bindings/generate.sh
cd vrp-cli
wasm-pack build --target web                        # browsers
wasm-pack build --target nodejs --out-dir pkg-node  # node
```

It should generate `wasm` build + some javascript files for you. If you want to have a smaller binary, you can try
to build without default features: `csv-format`, `scientific-format`.

Arguments accept plain javascript objects (or json strings, if you already have them) and results come back as objects.

## In the browser

Use the following index.html file:

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

        // the locations the matrix below has to describe, in this exact order
        const locations = get_routing_locations(pragmatic_problem);
        console.log('routing locations are:', locations);

        // NOTE let's assume we got routing matrix data for locations somehow
        // NOTE or just pass an empty array to use great-circle distance approximation
        const matrix_data= [
            {
                "profile": "normal_car",
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

        // the result is an object, so its fields can be read directly
        const solution = solve_pragmatic(pragmatic_problem, matrix_data, config);
        console.log(`cost is ${solution.statistic.cost}, tours: ${solution.tours.length}`);
    }

    run();
</script>
</body>
</html>
```


## In node

No `init()` call is needed with the `nodejs` target:

```js
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const vrp = require('./pkg-node/vrp_cli.js');

const solution = vrp.solve_pragmatic(problem, [matrix], config);
console.log(solution.statistic.cost);
```

A complete runnable example lives in
[examples/js-interop](https://github.com/reinterpretcat/vrp/tree/master/examples/js-interop).

Node 19 or newer is required: the solver seeds its random number generator from the Web Crypto API, which node only
exposes as a global from that version. On older releases install it before loading the module, otherwise solving fails
with `could not initialize ThreadRng: Unknown Error: 65546` followed by `RuntimeError: unreachable`:

```js
import { webcrypto } from 'node:crypto';
if (typeof globalThis.crypto === 'undefined') globalThis.crypto = webcrypto;
```


## Types

`wasm-pack` emits a `vrp_cli.d.ts` which describes the documents instead of using `any`. The declarations and their json
schemas are generated from the solver's rust types by `vrp-cli/bindings/generate.sh`, so they match what it accepts:

```ts
import { solve_pragmatic, type Problem, type Solution } from './pkg-node/vrp_cli.js';

const problem: Problem = { /* checked at compile time */ };
const solution: Solution = solve_pragmatic(problem, [], undefined);
const cost: number = solution.statistic.cost;
```


## Error handling

Failures throw an `Error` whose message is a json array, where each entry has a `code`, a `cause` and a suggested
`action`:

```js
try {
  vrp.solve_pragmatic(problem, [], config);
} catch (err) {
  for (const error of JSON.parse(err.message)) {
    console.error(error.code, error.cause, error.action);
  }
}
```

See the [error index](../../concepts/pragmatic/errors/index.md) for the meaning of each code. Note that
`solve_pragmatic` validates the problem before solving it.
