#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-standard-machine-baseline.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

core_patch="patch.crates-io.mech-core.path=\"$repository_root/src/core\""
abi_patch="patch.crates-io.mech-abi.path=\"$repository_root/src/abi\""

# These disposable builds do not need debug information. Keeping all three
# configurations for a machine in one target preserves isolation while reusing
# its dependencies; the target is then discarded before the next machine.
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0

check_machine() {
  machine=$1
  runtime_features=$2
  compiler_features=$3
  manifest="$repository_root/machines/$machine/Cargo.toml"
  target_dir="$scratch/$machine-target"
  echo "checking $machine runtime-only operation configuration"
  cargo +nightly-2026-03-03 check \
    --manifest-path "$manifest" \
    --target-dir "$target_dir" \
    --config "$core_patch" \
    --no-default-features \
    --features "$runtime_features"

  echo "checking $machine compiler-enabled operation configuration"
  cargo +nightly-2026-03-03 check \
    --manifest-path "$manifest" \
    --target-dir "$target_dir" \
    --config "$core_patch" \
    --no-default-features \
    --features "$compiler_features"

  # Most standard machines currently have no default-profile unit assertions.
  # Type-check every default target so cfg(test), examples, and benches remain
  # healthy without code-generating and linking an empty test executable.
  echo "checking $machine default targets"
  case "$machine" in
    combinatorics)
      cargo +nightly-2026-03-03 check \
        --manifest-path "$manifest" \
        --target-dir "$target_dir" \
        --config "$core_patch" \
        --config "$abi_patch" \
        --all-targets
      ;;
    *)
      cargo +nightly-2026-03-03 check \
        --manifest-path "$manifest" \
        --target-dir "$target_dir" \
        --config "$core_patch" \
        --all-targets
      ;;
  esac

  # Combinatorics is the only PR0 machine with default-profile assertions.
  # Run those assertions against the already-validated reduced runtime profile.
  if [ "$machine" = combinatorics ]; then
    echo "testing combinatorics reduced-profile behavior"
    cargo +nightly-2026-03-03 test \
      --manifest-path "$manifest" \
      --target-dir "$target_dir" \
      --config "$core_patch" \
      --config "$abi_patch" \
      --no-default-features \
      --features "$runtime_features" \
      --lib
  fi

  rm -rf "$target_dir"
}

check_machine \
  math \
  "program,functions,f64,add" \
  "program,compiler,functions,f64,add"

check_machine \
  compare \
  "program,functions,bool,f64,lt" \
  "program,compiler,functions,bool,f64,lt"

check_machine \
  logic \
  "program,functions,bool,and" \
  "program,compiler,functions,bool,and"

check_machine \
  range \
  "program,functions,formulas,f64,row_vectord,inclusive" \
  "program,compiler,functions,formulas,f64,row_vectord,inclusive"

# The solve implementation imports Zero and One through a crate-root import
# currently gated on transpose or matmul. Transpose is the smallest operation
# feature that completes the existing solve-only feature closure.
check_machine \
  matrix \
  "program,functions,f64,matrixd,vectord,solve,transpose" \
  "program,compiler,functions,f64,matrixd,vectord,solve,transpose"

# The set operation root currently includes the core set representation, so the
# smallest valid reduced configuration uses baselib rather than a bare set root.
check_machine \
  set \
  "program,baselib,union" \
  "program,compiler,baselib,union"

check_machine \
  string \
  "program,functions,string,concat" \
  "program,compiler,functions,string,concat"

check_machine \
  stats \
  "program,functions,f64,matrixd,vectord,sum" \
  "program,compiler,functions,f64,matrixd,vectord,sum"

# NChooseKMatrix is registered unconditionally but its implementation is gated
# on matrix support. Matrixd is the smallest valid matrix representation root.
check_machine \
  combinatorics \
  "program,functions,f64,n_choose_k,matrixd" \
  "program,compiler,functions,f64,n_choose_k,matrixd"

echo "standard machine baseline passed"
