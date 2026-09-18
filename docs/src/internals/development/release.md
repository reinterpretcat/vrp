# Releasing

Everything is published by workflows triggered manually from the Actions tab. They are independent, so
a release can be done in stages.

## Before starting

Bump the version in the workspace `Cargo.toml`, and in `rosomaxa/Cargo.toml` if it changed, then update
the `[Unreleased]` section of `CHANGELOG.md`. Note that `rosomaxa` is versioned independently on a
`0.9.x` line while the other crates share the workspace version.

The bump is not optional. Publishing skips any crate already on crates.io at that version, so running
the workflow without bumping reports success while uploading nothing.

A few version strings are written out by hand and do not follow the manifest. Update them too:

* the `deps.rs` badge and the `docker run` tag in `README.md`
* the `docker run` tag in `docs/src/getting-started/installation.md`
* `version` and `date-released` in `CITATION.cff`

The cli reports `--version` from `CARGO_PKG_VERSION`, so that one needs nothing.

The crates must be released together whenever one of them starts using a new API or feature of
another. `vrp-cli` currently depends on the `schema` feature of `vrp-pragmatic`, so publishing it
against an older `vrp-pragmatic` cannot work.

## Crates

Run the `Publish packages` workflow. It takes the release tag (`vX.Y.Z`, matching the `vrp-cli`
version), a docker tag, and a `dry_run` flag which is worth using first.

The workflow verifies the tag against the manifest, runs the full gate, and packages every crate
before uploading anything. It generates the ignored language bindings from the rust types first and
passes exactly that output to the crate and WebAssembly release jobs. It then publishes in dependency
order:

    rosomaxa -> vrp-core -> vrp-scientific -> vrp-pragmatic -> vrp-cli

Publishing is idempotent and resumable: a crate already on crates.io at that version is skipped,
transient upload failures are retried, and the registry index is polled before a dependent crate
follows. A run which fails partway can simply be re-run.

The same workflow pushes the docker image to GHCR and attaches the WebAssembly package to the GitHub
release.

## Python

Run the `Publish to PyPI` workflow. It builds wheels for linux, windows and macos, then uploads them
using the `PYPI_API_TOKEN` secret.

Thanks to pyo3's `abi3-py310` feature each platform produces a single wheel which works on CPython
3.10 and later, so there is no per-interpreter matrix. The wheel contains the generated models, so it
depends on `pydantic` at runtime.

## WebAssembly

There is no npm package. The `--target web` build is attached to the GitHub release as
`vrp_cli_wasm.zip`, and anyone can build either target locally:

```shell
pip install -r vrp-cli/bindings/python/requirements-codegen.txt
npm ci --prefix vrp-cli/bindings/typescript
./vrp-cli/bindings/generate.sh
cd vrp-cli
wasm-pack build --target web                        # browsers
wasm-pack build --target nodejs --out-dir pkg-node  # node
```

Both include typescript declarations describing the documents, generated from the schemas. CI builds
and tests both targets, so publishing to npm would only need an extra step.

## Caveat when packaging locally

`cargo package` verifies a crate by building it from its packaged form, where sibling path
dependencies resolve to their registry versions. That produces a different dependency graph, and its
artifacts conflict with those of a normal workspace build, so a later `cargo test` can fail with
confusing type errors. Give it its own target directory, as the workflow does:

```shell
CARGO_TARGET_DIR=target/package-verify cargo package --locked --workspace \
  --exclude heuristic-research --exclude json-pragmatic
```

Run `./vrp-cli/bindings/generate.sh` first. The generated declarations are intentionally ignored by
git, but `vrp-cli/Cargo.toml` includes them in the published crate so downstream WebAssembly builds do
not need the generator toolchain.

If you hit it anyway, `cargo clean` restores a working state.
