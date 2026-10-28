#!/usr/bin/env bash
#
# Regenerates the schemas and the typed models derived from them. See README.md for the pipeline and
# the tools it needs. DATAMODEL_CODEGEN and JSON2TS can point at locally installed binaries.

set -euo pipefail

bindings_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${bindings_dir}/../.." && pwd)"
cd "${repo_root}"

schema_dir="vrp-cli/bindings/schemas"
python_dir="vrp-cli/bindings/python/vrp_cli/models"
typescript_file="vrp-cli/bindings/typescript/format_types.d.ts"

codegen="${DATAMODEL_CODEGEN:-datamodel-codegen}"
local_json2ts="${bindings_dir}/typescript/node_modules/.bin/json2ts"
if [[ ! -x "${local_json2ts}" ]]; then
  local_json2ts="json2ts"
fi
json2ts="${JSON2TS:-${local_json2ts}}"

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: '$1' not found; $2" >&2
    exit 1
  fi
}
require cargo "install rust, see https://rustup.rs"
require python3 "install python 3, it merges the generated typescript declarations"
require "${codegen}" "install with: pip install -r vrp-cli/bindings/python/requirements-codegen.txt"
require "${json2ts}" "install with: npm ci --prefix vrp-cli/bindings/typescript"

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

generated_schema_dir="${work}/schemas"
typescript_schema_dir="${work}/typescript-schemas"
typescript_parts_dir="${work}/typescript-parts"
generated_python_dir="${work}/python"
generated_typescript_file="${work}/format_types.d.ts"
mkdir -p \
  "${generated_schema_dir}" \
  "${typescript_schema_dir}" \
  "${typescript_parts_dir}" \
  "${generated_python_dir}"

echo "==> json schemas, from the rust types"
# the `schema` feature is not enabled by default, so that a normal build does not pull in schemars
cargo run --quiet --locked -p vrp-cli --features schema --example schema_gen -- "${generated_schema_dir}"

# the generator decides which documents exist, so discover them from its output rather than keeping a
# second list here which could fall out of step with it
shopt -s nullglob
schemas=("${generated_schema_dir}"/*.schema.json)
shopt -u nullglob
if [ ${#schemas[@]} -eq 0 ]; then
  echo "error: no schemas were generated in ${generated_schema_dir}" >&2
  exit 1
fi

documents=()
for schema in "${schemas[@]}"; do
  documents+=("$(basename "${schema}" .schema.json)")
done
echo "documents: ${documents[*]}"

echo "==> typescript declarations, from the schemas"

# json-schema-to-typescript 16 does not understand the draft 2020-12 `prefixItems` keyword and
# otherwise emits fixed tuple members as `unknown`. Give only that tool an equivalent tuple shape
# using the older `items: []` spelling; the published schemas remain draft 2020-12.
python3 - "${generated_schema_dir}" "${typescript_schema_dir}" <<'PY'
import json
import sys
from pathlib import Path

source, destination = Path(sys.argv[1]), Path(sys.argv[2])


def normalize(value: object) -> None:
    if isinstance(value, dict):
        if "prefixItems" in value:
            if "items" in value:
                raise ValueError("schema contains both prefixItems and items")
            value["items"] = value.pop("prefixItems")
        for child in value.values():
            normalize(child)
    elif isinstance(value, list):
        for child in value:
            normalize(child)


for path in source.glob("*.schema.json"):
    schema = json.loads(path.read_text(encoding="utf-8"))
    normalize(schema)
    (destination / path.name).write_text(json.dumps(schema), encoding="utf-8")
PY

for name in "${documents[@]}"; do
  # `--additionalProperties false` keeps the declarations strict: the solver ignores unknown fields,
  # but a typo in a caller's object literal should still be a compile error
  "${json2ts}" \
    --input "${typescript_schema_dir}/${name}.schema.json" \
    --output "${typescript_parts_dir}/${name}.d.ts" \
    --bannerComment "" \
    --additionalProperties false \
    --unreachableDefinitions
done

python3 - "${typescript_parts_dir}" "${generated_typescript_file}" "${documents[@]}" <<'PY'
import re
import sys
from pathlib import Path

work, out_file, documents = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]

# a declaration is an optional leading jsdoc block followed by `export type|interface`
SPLIT = re.compile(r'^(?=(?:/\*\*(?:[^*]|\*(?!/))*\*/\s*)?export (?:type|interface) )', re.M)
NAME = re.compile(r'export (?:type|interface) (\w+)')

declarations: dict[str, str] = {}
conflicts: list[str] = []

# the documents are generated separately and share a few types, so merge them into one declaration
# file, failing if two documents disagree about a type of the same name rather than picking one
for name in documents:
    for block in SPLIT.split((work / f'{name}.d.ts').read_text(encoding='utf-8')):
        block = block.strip()
        match = NAME.search(block)
        if not block or not match:
            continue
        key = match.group(1)
        if key not in declarations:
            declarations[key] = block
        elif declarations[key] != block:
            conflicts.append(key)

if conflicts:
    sys.exit(f'error: the schemas disagree about these types: {sorted(set(conflicts))}')

header = """// Generated by vrp-cli/bindings/generate.sh from vrp-cli/bindings/schemas.
// Do not edit by hand: regenerate instead, see vrp-cli/bindings/README.md.
"""

out_file.parent.mkdir(parents=True, exist_ok=True)
# Keep output byte-identical on every host so generated CI artifacts and local builds are reproducible:
# the default text mode would use the locale encoding and translate '\n' to CRLF on windows
with open(out_file, 'w', encoding='utf-8', newline='\n') as handle:
    handle.write(header + '\n' + '\n\n'.join(declarations.values()) + '\n')
print(f'{out_file}: {len(declarations)} types')
PY

echo "==> pydantic models, from the schemas"

for name in "${documents[@]}"; do
  echo "${python_dir}/${name}.py"
  "${codegen}" \
    --input "${generated_schema_dir}/${name}.schema.json" \
    --input-file-type jsonschema \
    --output "${generated_python_dir}/${name}.py" \
    --output-model-type pydantic_v2.BaseModel \
    --base-class vrp_cli.models._base.Model \
    --target-python-version 3.10 \
    --use-annotated \
    --use-standard-collections \
    --use-union-operator \
    --use-schema-description \
    --collapse-root-models \
    --reuse-model \
    --infer-union-variant-names \
    --use-title-as-name \
    --formatters ruff-format \
    --disable-timestamp \
    --custom-file-header "\
# Generated by vrp-cli/bindings/generate.sh from vrp-cli/bindings/schemas/${name}.schema.json
# Do not edit by hand: regenerate instead, see vrp-cli/bindings/README.md."
done

# Replace generated output only after every stage has succeeded. This removes stale documents when
# a schema is renamed or deleted while preserving the hand-written package and base-model modules.
mkdir -p "${schema_dir}" "${python_dir}" "$(dirname "${typescript_file}")"
find "${schema_dir}" -maxdepth 1 -type f -name '*.schema.json' -delete
find "${python_dir}" -maxdepth 1 -type f -name '*.py' ! -name '__init__.py' ! -name '_base.py' -delete
cp "${generated_schema_dir}"/*.schema.json "${schema_dir}/"
cp "${generated_python_dir}"/*.py "${python_dir}/"
cp "${generated_typescript_file}" "${typescript_file}"

echo "done"
