# Description

This example shows how to use `pragmatic` library from javascript or typescript in node.

# Usage

- Install the pinned generators and run `./vrp-cli/bindings/generate.sh` from the repository root (see
  `vrp-cli/bindings/README.md`).
- Build the WebAssembly package: `cd vrp-cli && wasm-pack build --target nodejs --out-dir pkg-node`.
- Run the example from the repository root: `node examples/js-interop/example.mjs`

Node 19 and newer expose the Web Crypto API as a global. On older releases the example installs it
from `node:crypto` before loading the solver. See the
[javascript section](https://reinterpretcat.github.io/vrp/examples/interop/javascript.html) of the docs
for typed usage, error handling and running in a browser.
