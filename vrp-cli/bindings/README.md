# Language bindings

Everything the Python and WebAssembly bindings are built from. The rust side of the bindings lives in
`../src/interop`.

The solver's rust types are the only place a format is defined, and everything here is generated from
them:

```
bindings/
├── generate.sh                   regenerates everything below
├── schemas/                      from the rust types
├── python/                       from the schemas, built into a wheel by maturin
│   └── requirements-codegen.txt  pinned generator for this directory
└── typescript/                   from the schemas, embedded by the wasm binding
    └── package.json              pinned generator for this directory
```

Each language directory pins the generator that writes it, so neither toolchain needs to be installed
to work on the other.

Generated output is deliberately not committed: the rust types are the only source of truth. CI and
release workflows run the generator once and pass its output to the jobs that build Python wheels,
WebAssembly packages and the published Rust crate.

Run the script locally before a task that consumes generated bindings, or after changing a format
type:

```shell
pip install -r vrp-cli/bindings/python/requirements-codegen.txt
npm ci --prefix vrp-cli/bindings/typescript
./vrp-cli/bindings/generate.sh
```

The first stage is a cargo example, `../examples/schema_gen.rs`, gated behind the crate's `schema`
feature so that a normal build does not pull in `schemars`.

Beyond the two generators the script needs `bash`, `python3` and `cargo` on the path. On windows run it
from WSL or Git Bash. `DATAMODEL_CODEGEN` and `JSON2TS` can point at binaries installed elsewhere.

## Schemas

Conforming to [JSON Schema 2020-12](https://json-schema.org/).

| file                   | describes                                                |
|------------------------|----------------------------------------------------------|
| `problem.schema.json`  | a problem definition in `pragmatic` format               |
| `matrix.schema.json`   | a routing matrix in `pragmatic` format                   |
| `config.schema.json`   | the solver configuration                                 |
| `solution.schema.json` | a solution as produced by the solver                     |
| `error.schema.json`    | one entry of a failure reported by any of the bindings   |

### Input versus output

`problem`, `matrix` and `config` describe documents the solver *reads*, while `solution` and `error`
describe documents it *writes*. The distinction matters because the two are not symmetric: a few
fields may be omitted on input but are always present on output, so they are optional in the input
schemas and required in the output ones. `statistic.times.commuting` is one such field.

As a consequence an older solution document is not necessarily valid against
`solution.schema.json`, even though the solver still reads it. The solver is deliberately lenient
about what it accepts and strict about what it produces.

## Python

`python/` is a mixed rust/python project: maturin builds the native extension as `vrp_cli._native`
and the package here re-exports it, which is what lets the generated models ship inside the wheel as
`vrp_cli.models`. Models serialize with the json field names by default, so `model_dump_json()`
produces a document the solver accepts without needing `by_alias`.

See `../../examples/python-interop` for usage.

## TypeScript

`typescript/format_types.d.ts` is embedded into the binding with `include_str!` so that `wasm-pack`
emits a fully typed `.d.ts` on its own, rather than one where every argument is `any`. Release jobs
generate it before packaging, and `vrp-cli/Cargo.toml` explicitly includes the ignored file in the
published crate so downstream `wasm32` builds remain self-contained.

See `../../examples/js-interop` for usage.
