#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-standard-machine-baseline.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

core_patch="patch.crates-io.mech-core.path=\"$repository_root/src/core\""
abi_patch="patch.crates-io.mech-abi.path=\"$repository_root/src/abi\""

export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0

check_profile() {
  machine=$1
  profile=$2
  features=$3
  manifest="$repository_root/machines/$machine/Cargo.toml"
  target_dir="$scratch/$machine-target"

  echo "checking $machine $profile profile"
  cargo +nightly-2026-03-03 check \
    --manifest-path "$manifest" \
    --target-dir "$target_dir" \
    --config "$core_patch" \
    --config "$abi_patch" \
    --no-default-features \
    --features "$features"
}

check_machine() {
  machine=$1
  operation_features=$2

  check_profile "$machine" runtime "runtime,$operation_features"
  check_profile "$machine" source "source,$operation_features"
  check_profile "$machine" compiler "compiler,$operation_features"
  check_profile "$machine" source+compiler "source,compiler,$operation_features"
  check_profile "$machine" runtime_default runtime_default
  check_profile "$machine" source_default source_default
  check_profile "$machine" compiler_default compiler_default

  rm -rf "$scratch/$machine-target"
}

# These four profiles are also the reduced-closure contracts from PR3. They
# deliberately omit transpose, baselib, matrixd, and formulas respectively.
check_machine math "f64,add"
check_machine compare "bool,f64,lt"
check_machine logic "bool,and"
check_machine range "f64,row_vectord,inclusive"
check_machine matrix "f64,matrixd,vectord,solve"
check_machine set "set,f64,union"
check_machine string "string,concat"
check_machine stats "f64,matrixd,vectord,sum"
check_machine combinatorics "f64,n_choose_k"

# A runtime-only downstream crate must not be able to name a source
# specializer. The dependency itself has already passed its runtime profile,
# so a successful check here would mean the source boundary leaked.
source_probe="$scratch/source-specializer-probe"
mkdir -p "$source_probe/src"
cat > "$source_probe/Cargo.toml" <<EOF
[package]
name = "machine-source-specializer-probe"
version = "0.0.0"
edition = "2024"

[dependencies]
mech-math = { path = "$repository_root/machines/math", default-features = false, features = ["runtime", "f64", "add"] }
EOF
cat > "$source_probe/src/lib.rs" <<'EOF'
use mech_math::MathAdd;

pub fn source_specializer_must_not_exist(_: MathAdd) {}
EOF
if cargo +nightly-2026-03-03 check \
  --manifest-path "$source_probe/Cargo.toml" \
  --target-dir "$scratch/source-probe-target" \
  --config "$core_patch" \
  --quiet >"$scratch/source-probe.log" 2>&1
then
  echo "runtime-only math unexpectedly exposed MathAdd" >&2
  exit 1
fi
if ! grep -q 'MathAdd' "$scratch/source-probe.log"; then
  cat "$scratch/source-probe.log" >&2
  echo "source-specializer probe failed for an unrelated reason" >&2
  exit 1
fi

# Enabling the core compiler API alone must not add lowering implementations
# to a machine that did not select its compiler layer.
compiler_probe="$scratch/compiler-impl-probe"
mkdir -p "$compiler_probe/src"
cat > "$compiler_probe/Cargo.toml" <<EOF
[package]
name = "machine-compiler-impl-probe"
version = "0.0.0"
edition = "2024"

[dependencies]
mech-core = { path = "$repository_root/src/core", default-features = false, features = ["functions", "compiler", "f64"] }
mech-math = { path = "$repository_root/machines/math", default-features = false, features = ["runtime", "f64", "add"] }
EOF
cat > "$compiler_probe/src/lib.rs" <<'EOF'
use mech_core::MechFunctionCompiler;

fn assert_compiler<T: MechFunctionCompiler>() {}

pub fn compiler_impl_must_not_exist() {
    assert_compiler::<mech_math::AddSS<f64>>();
}
EOF
if cargo +nightly-2026-03-03 check \
  --manifest-path "$compiler_probe/Cargo.toml" \
  --target-dir "$scratch/compiler-probe-target" \
  --config "$core_patch" \
  --quiet >"$scratch/compiler-probe.log" 2>&1
then
  echo "runtime-only math unexpectedly exposed MechFunctionCompiler" >&2
  exit 1
fi
if ! grep -q 'MechFunctionCompiler' "$scratch/compiler-probe.log"; then
  cat "$scratch/compiler-probe.log" >&2
  echo "compiler-boundary probe failed for an unrelated reason" >&2
  exit 1
fi

echo "standard machine baseline passed"
