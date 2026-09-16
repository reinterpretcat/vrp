// Solves a small Vehicle Routing Problem from node using the WebAssembly build.
//
// Build the package first, then run this file:
//
//     ./vrp-cli/bindings/generate.sh
//     cd vrp-cli && wasm-pack build --target nodejs --out-dir pkg-node
//     node examples/js-interop/example.mjs
//
// Arguments accept plain objects (or json strings) and results come back as objects. The generated
// `vrp_cli.d.ts` describes both, so typescript callers get full completion and type checking.

import { createRequire } from 'node:module';
import { webcrypto } from 'node:crypto';

// The solver seeds its random number generator from the Web Crypto API. Node exposes it as a global
// from v19 onwards; on older releases it has to be installed manually. Without it, solving fails
// with "could not initialize ThreadRng: Unknown Error: 65546".
if (typeof globalThis.crypto === 'undefined') {
  globalThis.crypto = webcrypto;
}

const require = createRequire(import.meta.url);
const vrp = require('../../vrp-cli/pkg-node/vrp_cli.js');

const problem = {
  plan: {
    jobs: [
      {
        id: 'delivery_job1',
        deliveries: [
          {
            places: [
              {
                location: { lat: 52.52599, lng: 13.45413 },
                duration: 300,
                times: [['2019-07-04T09:00:00Z', '2019-07-04T18:00:00Z']],
              },
            ],
            demand: [1],
          },
        ],
      },
      {
        id: 'pickup_job2',
        pickups: [
          {
            places: [{ location: { lat: 52.5225, lng: 13.4095 }, duration: 240 }],
            demand: [1],
          },
        ],
      },
      // moves goods between two places: the pickup is always served before the delivery, and both
      // are assigned to the same vehicle or neither is
      {
        id: 'pickup_delivery_job3',
        pickups: [
          {
            places: [{ location: { lat: 52.5225, lng: 13.4095 }, duration: 300, tag: 'p1' }],
            demand: [1],
          },
        ],
        deliveries: [
          {
            places: [{ location: { lat: 52.5165, lng: 13.3808 }, duration: 300, tag: 'd1' }],
            demand: [1],
          },
        ],
      },
    ],
  },
  fleet: {
    vehicles: [
      {
        typeId: 'vehicle',
        vehicleIds: ['vehicle_1'],
        profile: { matrix: 'normal_car' },
        costs: { fixed: 22, distance: 0.0002, time: 0.005 },
        shifts: [
          {
            start: { earliest: '2019-07-04T09:00:00Z', location: { lat: 52.5316, lng: 13.3884 } },
            end: { latest: '2019-07-04T18:00:00Z', location: { lat: 52.5316, lng: 13.3884 } },
          },
        ],
        capacity: [10],
      },
    ],
    profiles: [{ name: 'normal_car' }],
  },
};

// Routing information between every pair of locations, in the order `get_routing_locations` returns.
// Pass an empty array instead to let the solver approximate distances.
const matrix = {
  profile: 'normal_car',
  travelTimes: [0, 609, 981, 906, 813, 0, 371, 590, 1055, 514, 0, 439, 948, 511, 463, 0],
  distances: [0, 3840, 5994, 5333, 4696, 0, 2154, 3226, 5763, 2674, 0, 2145, 5112, 2470, 2152, 0],
};

const config = { termination: { maxTime: 5, maxGenerations: 1000 } };

// The locations the matrix above has to describe, in order.
const locations = vrp.get_routing_locations(problem);
console.log(`unique locations: ${locations.length}`);

// Any of these calls throws an Error whose message is a json array of problems, each carrying a
// code, a cause and a suggested action.
let solution;
try {
  solution = vrp.solve_pragmatic(problem, [matrix], config);
} catch (err) {
  for (const error of JSON.parse(err.message)) {
    console.error(`${error.code}: ${error.cause}\n  ${error.action}`);
  }
  process.exit(1);
}

console.log(`cost: ${solution.statistic.cost.toFixed(2)}`);
console.log(`distance: ${solution.statistic.distance} m, duration: ${solution.statistic.duration} s`);
console.log(`tours: ${solution.tours.length}, unassigned: ${(solution.unassigned ?? []).length}`);

for (const tour of solution.tours) {
  console.log(`\nvehicle ${tour.vehicleId} visits ${tour.stops.length} stops:`);
  for (const stop of tour.stops) {
    const served = stop.activities.map((activity) => activity.jobId).join(', ');
    console.log(`  ${stop.time.arrival} -> ${served}`);
  }
}
