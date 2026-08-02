#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"

fail() {
  echo "complete function-system migration boundary failed: $*" >&2
  exit 1
}

obsolete_interpreter="mech-inter""preter"
obsolete_interpreter_ident="mech_inter""preter"
obsolete_interpreter_path="src/inter""preter/"
obsolete_program="mech-pro""gram"

[ -f "$repository_root/src/engine/Cargo.toml" ] || fail "src/engine/Cargo.toml is missing"
[ ! -e "$repository_root/src/inter""preter" ] || fail "obsolete interpreter package directory still exists"
[ ! -e "$repository_root/src/pro""gram" ] || fail "obsolete program package directory still exists"

cargo +nightly-2026-03-03 metadata \
  --manifest-path "$repository_root/Cargo.toml" \
  --format-version 1 \
  --no-deps |
  python3 -c '
import json
import sys

metadata = json.load(sys.stdin)
members = set(metadata["workspace_members"])
names = {
    package["name"]
    for package in metadata["packages"]
    if package["id"] in members
}
required = {"mech-engine"}
for name in required:
    if name not in names:
        raise SystemExit(f"complete function-system migration boundary failed: {name} is not a workspace member")
for name in {"mech-inter" + "preter", "mech-pro" + "gram"}:
    if name in names:
        raise SystemExit(f"complete function-system migration boundary failed: {name} remains a workspace member")
'

if rg -n -H \
  "$obsolete_interpreter|$obsolete_interpreter_ident|$obsolete_interpreter_path" \
  "$repository_root" \
  --hidden \
  --glob '!target/**' \
  --glob '!tests/architecture/**' \
  --glob '!.git/**' \
  --glob '!.agents/**' \
  --glob '!.codex/**'
then
  fail "active repository input still names the obsolete interpreter package"
fi

if rg -n -H \
  "$obsolete_program" \
  "$repository_root" \
  --glob 'Cargo.toml' \
  --glob '*.rs' \
  --glob '!target/**' \
  --glob '!tests/architecture/**' |
  awk '
    index($0, "class=\\\"") && index($0, "mech-program") { next }
    { print }
  ' |
  grep .
then
  fail "active Rust or manifest input still names the obsolete program package"
fi

legacy_pattern='FunctionDescriptor|FunctionCompilerDescriptor|ModuleItemDescriptor|FunctionSystem|default_function_system|\bFunctionTable\b|FunctionCompilerTable|FunctionsSnapshot|FunctionsRef|(struct|type)[[:space:]]+Functions\b|StaticNativeFunctionCompiler|NativeFunctionCompiler|LegacyFunctionBoundary|LegacySourceSpecializer|legacy_source_specializer|RuntimeFunctionUnavailable|register_descriptor|register_fxn_descriptor|register_assign_|register_define|register_horizontal_concatenate_fxn|register_vertical_concatenate_fxn|legacy_.*fallback|is_prelude_name|load_prelude|load_stdlib|LinkedModuleLoader'
if rg -n -H \
  "$legacy_pattern" \
  "$repository_root/src" \
  "$repository_root/machines" \
  "$repository_root/tests" \
  --glob '*.rs'
then
  fail "legacy function subsystem symbol remains"
fi

if rg -n -H \
  'inventory::(submit|iter)' \
  "$repository_root/src" \
  "$repository_root/machines" \
  "$repository_root/tests" \
  --glob '*.rs'
then
  fail "function inventory registration or enumeration remains"
fi

if rg -n -H \
  '(^|[[:space:]])inventory[[:space:]]*=' \
  "$repository_root" \
  --glob 'Cargo.toml' \
  --glob '!target/**'
then
  fail "function inventory dependency remains"
fi

if rg -n -H \
  '(mech_(math|compare|logic|range|matrix|set|string|stats|combinatorics)::[A-Z][A-Za-z0-9_]*|\b(Math|Compare|Logic|Range|Set|Stats|Combinatorics)[A-Z][A-Za-z0-9_]*|\bMatrix(Dot|MatMul|Solve|Transpose)|\bStringConcat)\s*\{\}\.(compile|specialize)' \
  "$repository_root/src/engine/src"
then
  fail "engine source dispatch still constructs a machine specializer directly"
fi

if rg -n -H \
  'static mut.*FunctionCatalog|Mutex<FunctionCatalog>|RwLock<FunctionCatalog>|Ref<FunctionCatalog>' \
  "$repository_root/src" \
  "$repository_root/machines"
then
  fail "function catalog is stored behind mutable shared state"
fi

if rg -n -H \
  'function_compilers|migrated_runtime_function_ids|migrated_operation_ids' \
  "$repository_root/src/engine/src" \
  "$repository_root/src/runtime/src"
then
  fail "legacy migration ownership state remains"
fi

# The surface shard compiles one standard profile and validates both frozen
# source contracts and the full runtime-factory contract. Other CI jobs own
# machine-profile, bytecode-consumer, native, WASM, and full package suites, so
# this boundary does not replay them.
bash "$repository_root/scripts/check-function-system-contracts.sh" surface

echo "complete function-system migration boundary passed"
