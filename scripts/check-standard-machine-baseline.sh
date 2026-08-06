#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/mech-standard-machine-baseline.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

core_patch="patch.crates-io.mech-core.path=\"$repository_root/src/core\""
abi_patch="patch.crates-io.mech-abi.path=\"$repository_root/src/abi\""

export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_INCREMENTAL=0

mode=${1:-full}
case "$mode" in
  full | representative) ;;
  *)
    echo "usage: $0 [full|representative]" >&2
    exit 2
    ;;
esac

for utility in cargo cat grep mktemp rm
do
  command -v "$utility" >/dev/null 2>&1 || {
    echo "standard machine baseline requires '$utility'" >&2
    exit 1
  }
done

removed_distribution_features='^(base|baselib|stdlib|program|prelude|pretty_print|serde|statements_default|subscript_default|statements|variables|variable_define|variable_assign|kind_define|kind_annotation|formulas|functions|symbol_table)[[:space:]]*='
for manifest in "$repository_root"/machines/*/Cargo.toml
do
  if grep -E "$removed_distribution_features" "$manifest" >/dev/null
  then
    grep -En "$removed_distribution_features" "$manifest" >&2
    echo "removed broad distribution feature remains in $manifest" >&2
    exit 1
  fi
done

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

  package="mech-$machine"
  graph_path="$scratch/$machine-$profile.tree"
  # This contract describes the graph shipped to downstream users. Benchmark
  # and test tooling may depend on broader compiler layers through dev edges.
  cargo +nightly-2026-03-03 tree \
    --manifest-path "$manifest" \
    --config "$core_patch" \
    --config "$abi_patch" \
    --no-default-features \
    --features "$features" \
    -e normal,build,features > "$graph_path"
  cargo +nightly-2026-03-03 tree \
    --manifest-path "$manifest" \
    --config "$core_patch" \
    --config "$abi_patch" \
    --no-default-features \
    --features "$features" \
    -e normal,build,features \
    -i "$package" >> "$graph_path"
  cargo +nightly-2026-03-03 tree \
    --manifest-path "$manifest" \
    --config "$core_patch" \
    --config "$abi_patch" \
    --no-default-features \
    --features "$features" \
    -e normal,build,features \
    -i mech-core >> "$graph_path"
  graph=$(cat "$graph_path")
  runtime_feature="$package feature \"runtime\""
  source_feature="$package feature \"source\""
  compiler_feature="$package feature \"compiler\""
  core_compiler_feature='mech-core feature "compiler"'

  case "$graph" in
    *"$runtime_feature"*) ;;
    *) echo "$machine $profile omitted its runtime layer" >&2; exit 1 ;;
  esac
  case "$profile" in
    runtime | runtime_default)
      required=""
      forbidden="$source_feature|$compiler_feature|$core_compiler_feature"
      ;;
    source | source_default)
      required="$source_feature"
      forbidden="$compiler_feature|$core_compiler_feature"
      ;;
    compiler)
      required="$compiler_feature|$core_compiler_feature"
      forbidden="$source_feature"
      ;;
    source+compiler | compiler_default)
      required="$source_feature|$compiler_feature|$core_compiler_feature"
      forbidden=""
      ;;
    *) echo "unknown machine profile '$profile'" >&2; exit 1 ;;
  esac

  old_ifs=$IFS
  IFS='|'
  for feature in $required
  do
    case "$graph" in
      *"$feature"*) ;;
      *) echo "$machine $profile omitted required layer: $feature" >&2; exit 1 ;;
    esac
  done
  for feature in $forbidden
  do
    case "$graph" in
      *"$feature"*) echo "$machine $profile leaked forbidden layer: $feature" >&2; exit 1 ;;
      *) ;;
    esac
  done
  IFS=$old_ifs

  if test "$profile" = runtime
  then
    accidental_feature=""
    case "$machine" in
      matrix) accidental_feature='mech-matrix feature "transpose"' ;;
      set) accidental_feature='mech-set feature "baselib"' ;;
      combinatorics) accidental_feature='mech-combinatorics feature "matrixd"' ;;
      range) accidental_feature='mech-range feature "formulas"' ;;
    esac
    if test -n "$accidental_feature"
    then
      case "$graph" in
        *"$accidental_feature"*)
          echo "$machine runtime restored accidental closure: $accidental_feature" >&2
          exit 1
          ;;
      esac
    fi
  fi
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
if test "$mode" = representative
then
  # Exercise every layer on one machine plus each reduced-closure edge from
  # PR3. The scheduled exhaustive workflow retains the 9 x 7 profile matrix.
  check_machine math "f64,add"
  check_profile range runtime "f64,row_vectord,inclusive"
  check_profile matrix runtime "f64,matrixd,vectord,solve"
  check_profile set runtime "set,f64,union"
  check_profile combinatorics runtime "f64,n_choose_k"
else
  check_machine math "f64,add"
  check_machine compare "bool,f64,lt"
  check_machine logic "bool,and"
  check_machine range "f64,row_vectord,inclusive"
  check_machine matrix "f64,matrixd,vectord,solve"
  check_machine set "set,f64,union"
  check_machine string "string,concat"
  check_machine stats "f64,matrixd,vectord,sum"
  check_machine combinatorics "f64,n_choose_k"
fi

# A downstream runtime-only crate can install the selected concrete factories
# without enabling source specialization or compiler support.
runtime_probe="$scratch/runtime-factory-probe"
mkdir -p "$runtime_probe/src"
cat > "$runtime_probe/Cargo.toml" <<EOF
[package]
name = "machine-runtime-factory-probe"
version = "0.0.0"
edition = "2024"

[dependencies]
mech-core = { path = "$repository_root/src/core", default-features = false, features = ["functions", "f64"] }
mech-math = { path = "$repository_root/machines/math", default-features = false, features = ["runtime", "f64", "add"] }
EOF
cat > "$runtime_probe/src/main.rs" <<'EOF'
use mech_core::FunctionCatalogBuilder;

fn main() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_math::install_runtime(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    assert!(catalog.runtime_factory_count() > 0);
    assert_eq!(catalog.specializer_count(), 0);
}
EOF
cargo +nightly-2026-03-03 run \
  --manifest-path "$runtime_probe/Cargo.toml" \
  --target-dir "$scratch/runtime-probe-target" \
  --config "$core_patch" \
  --quiet

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

# Compiler-only machine layers add lowering to concrete runtime nodes without
# reintroducing source specializers.
compiler_source_probe="$scratch/compiler-source-specializer-probe"
mkdir -p "$compiler_source_probe/src"
cat > "$compiler_source_probe/Cargo.toml" <<EOF
[package]
name = "machine-compiler-source-specializer-probe"
version = "0.0.0"
edition = "2024"

[dependencies]
mech-math = { path = "$repository_root/machines/math", default-features = false, features = ["compiler", "f64", "add"] }
EOF
cat > "$compiler_source_probe/src/lib.rs" <<'EOF'
use mech_math::MathAdd;

pub fn source_specializer_must_not_exist(_: MathAdd) {}
EOF
if cargo +nightly-2026-03-03 check \
  --manifest-path "$compiler_source_probe/Cargo.toml" \
  --target-dir "$scratch/compiler-source-probe-target" \
  --config "$core_patch" \
  --quiet >"$scratch/compiler-source-probe.log" 2>&1
then
  echo "compiler-only math unexpectedly exposed MathAdd" >&2
  exit 1
fi
if ! grep -q 'MathAdd' "$scratch/compiler-source-probe.log"; then
  cat "$scratch/compiler-source-probe.log" >&2
  echo "compiler-only source-specializer probe failed for an unrelated reason" >&2
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
