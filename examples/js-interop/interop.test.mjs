import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import test from 'node:test';

const require = createRequire(import.meta.url);
const vrp = require('../../vrp-cli/pkg-node/vrp_cli.js');

function assertInteropError(callback, expectedCause) {
  assert.throws(callback, (error) => {
    const failures = JSON.parse(error.message);

    assert.equal(failures.length, 1);
    assert.equal(failures[0].code, 'E0006');
    assert.match(failures[0].cause, expectedCause);
    assert.equal(typeof failures[0].action, 'string');
    return true;
  });
}

test('reports a non-array matrices argument using the binding error contract', () => {
  assertInteropError(() => vrp.solve_pragmatic('{}', {}, undefined), /matrices argument/);
});

test('reports an unserializable object using the binding error contract', () => {
  const problem = {};
  problem.self = problem;

  assertInteropError(() => vrp.get_routing_locations(problem), /serialize problem argument/);
});
