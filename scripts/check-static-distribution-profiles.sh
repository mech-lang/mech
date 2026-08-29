#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-static-profiles.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0

fail() {
  echo "static distribution profile contract failed: $*" >&2
  exit 1
}

for utility in cargo python3 rg
do
  command -v "$utility" >/dev/null 2>&1 || fail "required utility '$utility' is unavailable"
done

mode=${1:-all}
case "$mode" in
  all | engine | selected-runtime | full-runtime | full-source | full-compiler | wasm-source | static) ;;
  *)
    echo "usage: $0 [all|engine|selected-runtime|full-runtime|full-source|full-compiler|wasm-source|static]" >&2
    exit 2
    ;;
esac

cargo_nightly() {
  cargo +nightly-2026-03-03 "$@"
}

capture_workspace_profile() {
  package=$1
  features=$2
  output=$3
  cargo_nightly tree \
    --manifest-path "$repository_root/Cargo.toml" \
    -p "$package" \
    --no-default-features \
    --features "$features" \
    -e features,no-dev > "$output"
  cargo_nightly tree \
    --manifest-path "$repository_root/Cargo.toml" \
    -p "$package" \
    --no-default-features \
    --features "$features" \
    -e features,no-dev \
    -i "$package" >> "$output"
}

capture_fixture_profile() {
  manifest=$1
  output=$2
  cargo_nightly tree --manifest-path "$manifest" -e features,no-dev > "$output"
  for package in mech-engine mech-stdlib
  do
    cargo_nightly tree \
      --manifest-path "$manifest" \
      -e features,no-dev \
      -i "$package" >> "$output"
  done
}

check_graph() {
  graph=$1
  contract=$2
  shift 2
  python3 - "$graph" "$contract" "$@" <<'PY'
from pathlib import Path
import sys

graph_path = Path(sys.argv[1])
contract = sys.argv[2]
requirements = [argument.split("=", 1) for argument in sys.argv[3:]]
text = graph_path.read_text(encoding="utf-8")

for disposition, needle in requirements:
    present = needle in text
    if disposition == "require" and not present:
        raise SystemExit(
            f"static distribution profile contract failed: {contract} requires {needle!r}"
        )
    if disposition == "forbid" and present:
        raise SystemExit(
            f"static distribution profile contract failed: {contract} forbids {needle!r}"
        )
PY
}

machine_packages="mech-combinatorics mech-compare mech-logic mech-math mech-matrix mech-range mech-set mech-stats mech-string"

check_no_machine_packages() {
  graph=$1
  contract=$2
  for package in $machine_packages
  do
    check_graph "$graph" "$contract" "forbid=$package v"
  done
}

check_full_machine_layers() {
  graph=$1
  contract=$2
  expected=$3
  for package in $machine_packages
  do
    check_graph "$graph" "$contract" "require=$package feature \"runtime\""
    case "$expected" in
      runtime)
        check_graph "$graph" "$contract" \
          "forbid=$package feature \"source\"" \
          "forbid=$package feature \"compiler\""
        ;;
      source)
        check_graph "$graph" "$contract" \
          "require=$package feature \"source\"" \
          "forbid=$package feature \"compiler\""
        ;;
      compiler)
        check_graph "$graph" "$contract" \
          "require=$package feature \"source\"" \
          "require=$package feature \"compiler\""
        ;;
      *) fail "unknown machine layer expectation '$expected'" ;;
    esac
  done
  case "$expected" in
    runtime | source)
      check_graph "$graph" "$contract" 'forbid=mech-core feature "compiler"'
      ;;
    compiler)
      check_graph "$graph" "$contract" 'require=mech-core feature "compiler"'
      ;;
  esac
}

check_static_boundary() {
  python3 "$repository_root/scripts/check-compiler-planning-quarantine.py"
  python3 "$repository_root/scripts/check-production-resident-routing.py"
  python3 "$repository_root/scripts/check-rust-module-layout.py"
  python3 "$repository_root/scripts/check-source-catalog-entrypoints.py"

  cargo_nightly metadata \
    --manifest-path "$repository_root/Cargo.toml" \
    --format-version 1 \
    --no-deps > "$scratch/workspace-metadata.json"

  python3 - \
    "$repository_root/src/engine/Cargo.toml" \
    "$repository_root/src/engine/src" \
    "$repository_root/tests/architecture/distributions/profile-contracts.json" \
    "$repository_root/tests/architecture/function-system/runtime-factory-surface.json" \
    "$scratch/workspace-metadata.json" <<'PY'
from hashlib import sha256
import json
from pathlib import Path
import re
import sys

manifest_path, source_root, contracts_path, runtime_surface_path, metadata_path = map(
    Path, sys.argv[1:]
)
machine_packages = {
    "mech-math", "mech-compare", "mech-logic", "mech-range", "mech-matrix",
    "mech-set", "mech-string", "mech-stats", "mech-combinatorics",
}
machine_identifiers = tuple(name.replace("-", "_") for name in sorted(machine_packages))
obsolete = (
    "default_function_catalog", "linked_stdlib", "linked_math", "linked_compare",
    "linked_stats", "linked_string", "linked_combinatorics",
)

metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
engine_package = next(
    (package for package in metadata["packages"] if package["name"] == "mech-engine"),
    None,
)
if engine_package is None:
    raise SystemExit("static distribution profile contract failed: mech-engine metadata is missing")
dependencies = {dependency["name"] for dependency in engine_package["dependencies"]}
for forbidden in sorted(machine_packages | {"mech-stdlib"}):
    if forbidden in dependencies:
        raise SystemExit(
            f"static distribution profile contract failed: mech-engine depends on {forbidden}"
        )

stdlib_package = next(
    (package for package in metadata["packages"] if package["name"] == "mech-stdlib"),
    None,
)
if stdlib_package is None:
    raise SystemExit("static distribution profile contract failed: mech-stdlib metadata is missing")
stdlib_dependencies = {
    dependency["name"]: dependency
    for dependency in stdlib_package["dependencies"]
    if dependency.get("kind") is None
}
expected_stdlib_dependencies = machine_packages | {"mech-core", "mech-engine"}
if set(stdlib_dependencies) != expected_stdlib_dependencies:
    raise SystemExit(
        "static distribution profile contract failed: mech-stdlib normal dependencies are "
        f"{sorted(stdlib_dependencies)}, expected {sorted(expected_stdlib_dependencies)}"
    )
for dependency_name, dependency in stdlib_dependencies.items():
    expected_optional = dependency_name in machine_packages
    if dependency["optional"] != expected_optional:
        raise SystemExit(
            "static distribution profile contract failed: mech-stdlib dependency "
            f"{dependency_name} optional={dependency['optional']}, expected {expected_optional}"
        )

value_shape_features = (
    "bool", "string",
    "u8", "u16", "u32", "u64", "u128",
    "i8", "i16", "i32", "i64", "i128",
    "f32", "f64", "c64", "r64",
    "set", "map", "table", "tuple", "record", "atom", "enum",
    "matrix1", "matrix2", "matrix3", "matrix4", "matrix2x3", "matrix3x2",
    "row_vector2", "row_vector3", "row_vector4",
    "vector2", "vector3", "vector4",
    "row_vectord", "vectord", "matrixd",
)
stdlib_features = stdlib_package["features"]
for feature_name in value_shape_features:
    routes = stdlib_features.get(feature_name)
    if routes is None:
        raise SystemExit(
            f"static distribution profile contract failed: mech-stdlib omits {feature_name}"
        )
    for required_route in (f"mech-core/{feature_name}", f"mech-engine/{feature_name}"):
        if required_route not in routes:
            raise SystemExit(
                "static distribution profile contract failed: "
                f"mech-stdlib/{feature_name} omits {required_route}"
            )
    for route in routes:
        if route in (f"mech-core/{feature_name}", f"mech-engine/{feature_name}"):
            continue
        match = re.fullmatch(r"(mech-[a-z-]+)(\?/|/)([a-z0-9_]+)", route)
        if match is None or match.group(1) not in machine_packages:
            raise SystemExit(
                "static distribution profile contract failed: "
                f"mech-stdlib/{feature_name} has unexpected route {route!r}"
            )
        package, separator, forwarded_feature = match.groups()
        if separator != "?/" or forwarded_feature != feature_name:
            raise SystemExit(
                "static distribution profile contract failed: "
                f"mech-stdlib/{feature_name} must weak-forward its matching feature, got {route!r}"
            )

for path in sorted(source_root.rglob("*.rs")):
    text = path.read_text(encoding="utf-8")
    for identifier in machine_identifiers:
        if re.search(rf"\b{re.escape(identifier)}\b", text):
            raise SystemExit(
                f"static distribution profile contract failed: engine imports machine "
                f"identifier {identifier} in {path}"
            )
    for symbol in obsolete:
        if symbol in text:
            raise SystemExit(
                f"static distribution profile contract failed: obsolete symbol {symbol} in {path}"
            )

manifest_text = manifest_path.read_text(encoding="utf-8")
for symbol in obsolete:
    if symbol in manifest_text:
        raise SystemExit(
            f"static distribution profile contract failed: obsolete feature {symbol} in engine manifest"
        )

workspace_names = {
    package["name"]
    for package in metadata["packages"]
    if package["id"] in set(metadata["workspace_members"])
}
for required in ("mech-engine", "mech-stdlib"):
    if required not in workspace_names:
        raise SystemExit(
            f"static distribution profile contract failed: {required} is not a workspace member"
        )

contracts_text = contracts_path.read_text(encoding="utf-8")
contracts = json.loads(contracts_text)
if contracts.get("schema") != 1:
    raise SystemExit("static distribution profile contract failed: unsupported profile schema")
expected_profiles = {
    "selected-runtime", "full-runtime", "full-source", "full-compiler",
}
profiles = {profile["profile_name"]: profile for profile in contracts.get("profiles", [])}
if set(profiles) != expected_profiles:
    raise SystemExit(
        "static distribution profile contract failed: deterministic profile set mismatch"
    )
expected_digest = "605e2ea1b0b3cf8db9df3f21cc3d461e6c453c310936aba7cd30a5a15678affa"
selected_digest = "a006c5b25aa925939f4973273e2aea9cac2897fbcca32dc25edd6be74631445d"
runtime_surface = json.loads(runtime_surface_path.read_text(encoding="utf-8"))
runtime_factories = runtime_surface.get("runtime_factories")
if not isinstance(runtime_factories, list):
    raise SystemExit(
        "static distribution profile contract failed: frozen runtime factories are missing"
    )
canonical_runtime_surface = "".join(
    f"{entry['id_hex']}\t{entry['name']}\n"
    for entry in sorted(
        runtime_factories,
        key=lambda entry: (entry["id_hex"], entry["name"]),
    )
).encode("utf-8")
actual_digest = sha256(canonical_runtime_surface).hexdigest()
if actual_digest != expected_digest:
    raise SystemExit(
        f"static distribution profile contract failed: frozen runtime digest is {actual_digest}"
    )
expected_surface = {
    "selected-runtime": (3, 0, 0, 0, 0, selected_digest, "sha256-canonical-id-tab-name-lf-v1"),
    "full-runtime": (9033, 0, 0, 0, 0, expected_digest, "sha256-canonical-id-tab-name-lf-v1"),
    "full-source": (9034, 119, 10, 52, 50, "1c024944469c5f4372c7d47035ad18375c6a913030c15286ce2685350ee08a8f", "sha256-canonical-id-tab-name-lf-v1"),
    "full-compiler": (9034, 119, 10, 52, 50, "1c024944469c5f4372c7d47035ad18375c6a913030c15286ce2685350ee08a8f", "sha256-canonical-id-tab-name-lf-v1"),
}
surface_keys = (
    "catalog_factory_count", "source_specializer_count", "intrinsic_count",
    "prelude_count", "module_export_count", "runtime_surface_digest", "digest_algorithm",
)
for name, profile in profiles.items():
    actual_surface = tuple(profile.get(key) for key in surface_keys)
    if actual_surface != expected_surface[name]:
        raise SystemExit(
            f"static distribution profile contract failed: {name} deterministic surface mismatch"
        )
    for key in (
        "required_packages", "forbidden_packages",
        "required_feature_layers", "forbidden_feature_layers",
    ):
        if key not in profile:
            raise SystemExit(
                f"static distribution profile contract failed: {name} omits {key}"
            )
    for key in (
        "required_packages", "forbidden_packages", "required_feature_layers",
        "forbidden_feature_layers",
    ):
        if profile[key] != sorted(profile[key]):
            raise SystemExit(
                f"static distribution profile contract failed: {name} {key} is not sorted"
            )

machine_packages = sorted(machine_packages)
runtime_packages = ["mech-engine", *machine_packages, "mech-stdlib"]
compiler_packages = ["mech-core", *runtime_packages]
full_packages = sorted(["mech-core", *runtime_packages])

expected_packages = {
    "selected-runtime": {
        "required_packages": ["mech-core", "mech-engine", "mech-math", "mech-stdlib"],
        "forbidden_packages": [
            "mech-bytecode", "mech-combinatorics", "mech-compare", "mech-logic",
            "mech-matrix", "mech-range", "mech-set", "mech-stats", "mech-string",
            "mech-syntax",
        ],
    },
    "full-runtime": {
        "required_packages": full_packages,
        "forbidden_packages": ["mech-bytecode", "mech-syntax"],
    },
    "full-source": {
        "required_packages": sorted([*full_packages, "mech-syntax"]),
        "forbidden_packages": ["mech-bytecode"],
    },
    "full-compiler": {
        "required_packages": sorted([*full_packages, "mech-bytecode", "mech-syntax"]),
        "forbidden_packages": [],
    },
}
for profile_name, contract_packages in expected_packages.items():
    for key, expected in contract_packages.items():
        if profiles[profile_name][key] != expected:
            raise SystemExit(
                f"static distribution profile contract failed: "
                f"{profile_name} {key} mismatch"
            )

def layers(packages, names):
    return sorted(f"{package}/{name}" for package in packages for name in names)

expected_layers = {
    "selected-runtime": {
        "required_feature_layers": sorted([
            "mech-engine/runtime", "mech-math/runtime", "mech-stdlib/runtime",
        ]),
        "forbidden_feature_layers": sorted([
            "mech-core/compiler", "mech-engine/compiler", "mech-engine/source",
            "mech-math/compiler", "mech-math/source", "mech-stdlib/compiler",
            "mech-stdlib/source",
        ]),
    },
    "full-runtime": {
        "required_feature_layers": layers(runtime_packages, ["runtime"]),
        "forbidden_feature_layers": sorted(
            layers(compiler_packages, ["compiler"])
            + layers(runtime_packages, ["source"])
        ),
    },
    "full-source": {
        "required_feature_layers": layers(runtime_packages, ["runtime", "source"]),
        "forbidden_feature_layers": layers(compiler_packages, ["compiler"]),
    },
    "full-compiler": {
        "required_feature_layers": sorted(
            layers(runtime_packages, ["runtime", "source"])
            + layers(compiler_packages, ["compiler"])
        ),
        "forbidden_feature_layers": [],
    },
}
for profile_name, contract_layers in expected_layers.items():
    for key, expected in contract_layers.items():
        if profiles[profile_name][key] != expected:
            raise SystemExit(
                f"static distribution profile contract failed: "
                f"{profile_name} {key} mismatch"
            )
PY
}

check_engine() {
  capture_workspace_profile mech-engine runtime "$scratch/engine-runtime.tree"
  check_graph "$scratch/engine-runtime.tree" "engine runtime" \
    "require=mech-core v" \
    'require=mech-engine feature "runtime"' \
    "forbid=mech-syntax v" \
    "forbid=mech-bytecode v" \
    "forbid=mech-stdlib v"
  check_no_machine_packages "$scratch/engine-runtime.tree" "engine runtime"

  capture_workspace_profile mech-engine source "$scratch/engine-source.tree"
  check_graph "$scratch/engine-source.tree" "engine source" \
    "require=mech-syntax v" \
    'require=mech-engine feature "source"' \
    "forbid=mech-bytecode v" \
    "forbid=mech-stdlib v"
  check_no_machine_packages "$scratch/engine-source.tree" "engine source"

  capture_workspace_profile mech-engine compiler "$scratch/engine-compiler.tree"
  check_graph "$scratch/engine-compiler.tree" "engine compiler" \
    "require=mech-syntax v" \
    "require=mech-bytecode v" \
    'require=mech-engine feature "compiler"' \
    "forbid=mech-stdlib v"
  check_no_machine_packages "$scratch/engine-compiler.tree" "engine compiler"
}

check_selected_runtime() {
  manifest="$repository_root/tests/fixtures/bytecode-runtime-consumer/Cargo.toml"
  capture_fixture_profile "$manifest" "$scratch/selected-runtime.tree"
  check_graph "$scratch/selected-runtime.tree" "selected runtime" \
    "require=mech-core v" \
    "require=mech-engine v" \
    "require=mech-stdlib v" \
    "require=mech-math v" \
    "forbid=mech-syntax v" \
    "forbid=mech-bytecode v" \
    "forbid=mech-compare v" \
    "forbid=mech-logic v" \
    "forbid=mech-range v" \
    "forbid=mech-matrix v" \
    "forbid=mech-set v" \
    "forbid=mech-string v" \
    "forbid=mech-stats v" \
    "forbid=mech-combinatorics v" \
    'forbid=mech-engine feature "source"' \
    'forbid=mech-engine feature "compiler"' \
    'forbid=mech-stdlib feature "source"' \
    'forbid=mech-stdlib feature "compiler"' \
    'require=mech-engine feature "runtime"' \
    'require=mech-stdlib feature "runtime"' \
    'require=mech-math feature "runtime"' \
    'forbid=mech-math feature "source"' \
    'forbid=mech-math feature "compiler"' \
    'forbid=mech-core feature "compiler"'

  capture_workspace_profile mech-stdlib "runtime,f64" "$scratch/weak-f64.tree"
  check_no_machine_packages "$scratch/weak-f64.tree" "weak f64 forwarding"
  capture_workspace_profile mech-stdlib "runtime,matrixd" "$scratch/weak-matrixd.tree"
  check_no_machine_packages "$scratch/weak-matrixd.tree" "weak matrixd forwarding"

  MECH_EXPECT_RUNTIME_FACTORY_COUNT=3 \
    MECH_EXPECT_RUNTIME_SURFACE_DIGEST=a006c5b25aa925939f4973273e2aea9cac2897fbcca32dc25edd6be74631445d \
    MECH_EXPECT_SOURCE_SPECIALIZER_COUNT=0 \
    MECH_EXPECT_INTRINSIC_COUNT=0 \
    MECH_EXPECT_PRELUDE_COUNT=0 \
    MECH_EXPECT_MODULE_EXPORT_COUNT=0 \
    MECH_EXPECT_TOTAL_EXPORT_COUNT=0 \
    CARGO_PROFILE_TEST_DEBUG=0 \
    cargo_nightly test \
      --manifest-path "$repository_root/Cargo.toml" \
      -p mech-stdlib \
      --no-default-features \
      --features "runtime,f64,math_add" \
      --test profile_contracts \
      --target-dir "$scratch/selected-runtime-target" \
      distribution_size_report_catalog_counts \
      -- \
      --exact \
      --nocapture

  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly run \
    --manifest-path "$manifest" \
    --target-dir "$scratch/selected-runtime-target" \
    -- "$repository_root/tests/architecture/bytecode-v1/scalar-add-f64.mecb"
}

check_full_runtime() {
  manifest="$repository_root/tests/fixtures/full-bytecode-runtime/Cargo.toml"
  capture_fixture_profile "$manifest" "$scratch/full-runtime.tree"
  for package in mech-core mech-engine mech-stdlib mech-math mech-compare mech-logic mech-range mech-matrix mech-set mech-string mech-stats mech-combinatorics
  do
    check_graph "$scratch/full-runtime.tree" "full runtime" "require=$package v"
  done
  check_graph "$scratch/full-runtime.tree" "full runtime" \
    "forbid=mech-syntax v" \
    "forbid=mech-bytecode v" \
    'forbid=mech-engine feature "source"' \
    'forbid=mech-engine feature "compiler"' \
    'forbid=mech-stdlib feature "source"' \
    'forbid=mech-stdlib feature "compiler"'
  check_full_machine_layers "$scratch/full-runtime.tree" "full runtime" runtime

  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly test \
    --manifest-path "$repository_root/Cargo.toml" \
    -p mech-stdlib \
    --no-default-features \
    --features full_runtime \
    --test profile_contracts \
    --target-dir "$scratch/full-runtime-target"
  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly run \
    --manifest-path "$manifest" \
    --target-dir "$scratch/full-runtime-target" \
    -- "$repository_root/tests/architecture/bytecode-v1/scalar-add-f64.mecb"
}

check_full_source() {
  manifest="$repository_root/tests/fixtures/full-source-runtime/Cargo.toml"
  capture_fixture_profile "$manifest" "$scratch/full-source.tree"
  check_graph "$scratch/full-source.tree" "full source" \
    "require=mech-syntax v" \
    'require=mech-engine feature "source"' \
    'require=mech-stdlib feature "source"' \
    "forbid=mech-bytecode v" \
    'forbid=mech-engine feature "compiler"' \
    'forbid=mech-stdlib feature "compiler"'
  check_full_machine_layers "$scratch/full-source.tree" "full source" source

  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly test \
    --manifest-path "$repository_root/Cargo.toml" \
    -p mech-stdlib \
    --no-default-features \
    --features full_source \
    --test profile_contracts \
    --target-dir "$scratch/full-source-target"
  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly run \
    --manifest-path "$manifest" \
    --target-dir "$scratch/full-source-target"
}

check_full_compiler() {
  producer_manifest="$repository_root/tests/fixtures/bytecode-compiler-producer/Cargo.toml"
  consumer_manifest="$repository_root/tests/fixtures/bytecode-runtime-consumer/Cargo.toml"
  capture_fixture_profile "$producer_manifest" "$scratch/full-compiler.tree"
  check_graph "$scratch/full-compiler.tree" "full compiler" \
    "require=mech-syntax v" \
    "require=mech-bytecode v" \
    'require=mech-engine feature "compiler"' \
    'require=mech-stdlib feature "compiler"'
  check_full_machine_layers "$scratch/full-compiler.tree" "full compiler" compiler

  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly test \
    --manifest-path "$repository_root/Cargo.toml" \
    -p mech-stdlib \
    --no-default-features \
    --features full_compiler \
    --test profile_contracts \
    --target-dir "$scratch/full-compiler-target"

  output="$scratch/compiler-produced-scalar-add.mecb"
  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly run \
    --manifest-path "$producer_manifest" \
    --target-dir "$scratch/full-compiler-target" \
    -- "$output"
  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly run \
    --manifest-path "$consumer_manifest" \
    --target-dir "$scratch/compiler-runtime-target" \
    -- "$output"
}

check_wasm_source() {
  cargo_nightly tree \
    --manifest-path "$repository_root/Cargo.toml" \
    -p mech-wasm \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features browser_project \
    -e features,no-dev > "$scratch/wasm-source.tree"
  for package in mech-engine mech-stdlib mech-syntax mech-combinatorics mech-compare mech-logic mech-math mech-matrix mech-range mech-stats mech-string
  do
    cargo_nightly tree \
      --manifest-path "$repository_root/Cargo.toml" \
      -p mech-wasm \
      --target wasm32-unknown-unknown \
      --no-default-features \
      --features browser_project \
      -e features,no-dev \
      -i "$package" >> "$scratch/wasm-source.tree"
  done
  check_graph "$scratch/wasm-source.tree" "WASM source" \
    "require=mech-syntax v" \
    "require=mech-compare v" \
    "require=mech-logic v" \
    "require=mech-math v" \
    "require=mech-matrix v" \
    "require=mech-range v" \
    "require=mech-stats v" \
    "require=mech-string v" \
    "require=mech-combinatorics v" \
    "forbid=mech-set v" \
    'require=mech-engine feature "source"' \
    'require=mech-engine feature "state_machines"' \
    'require=mech-stdlib feature "source"' \
    "forbid=mech-bytecode v" \
    'forbid=mech-wasm feature "compiler"' \
    'forbid=mech-engine feature "compiler"' \
    'forbid=mech-stdlib feature "compiler"' \
    'forbid=mech-stdlib feature "full_compiler"' \
    'forbid=mech-stdlib feature "full_operations"' \
    'forbid=mech-stdlib feature "full_runtime"' \
    'forbid=mech-stdlib feature "full_source"' \
    'forbid=mech-stdlib feature "full_values"' \
    'forbid=mech-core feature "compiler"'
  for package in mech-combinatorics mech-compare mech-logic mech-math mech-matrix mech-range mech-stats mech-string
  do
    check_graph "$scratch/wasm-source.tree" "WASM source" \
      "require=$package feature \"runtime\"" \
      "require=$package feature \"source\"" \
      "forbid=$package feature \"compiler\""
  done

  CARGO_PROFILE_DEV_DEBUG=0 cargo_nightly check \
    --manifest-path "$repository_root/Cargo.toml" \
    -p mech-wasm \
    --target wasm32-unknown-unknown \
    --no-default-features \
    --features browser_project \
    --target-dir "$scratch/wasm-source-target"
}

case "$mode" in
  all)
    check_static_boundary
    check_engine
    check_selected_runtime
    check_full_runtime
    check_full_source
    check_full_compiler
    check_wasm_source
    ;;
  engine) check_engine ;;
  selected-runtime) check_selected_runtime ;;
  full-runtime) check_full_runtime ;;
  full-source) check_full_source ;;
  full-compiler) check_full_compiler ;;
  wasm-source) check_wasm_source ;;
  static) check_static_boundary ;;
esac

echo "static distribution profile contract passed ($mode)"
