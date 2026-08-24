#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

require_command() {
  local command_name="$1"
  local install_hint="${2:-}"

  if command -v "$command_name" >/dev/null 2>&1; then
    return
  fi

  printf '%s is required.' "$command_name" >&2
  if [[ -n "$install_hint" ]]; then
    printf ' Install it with:\n%s' "$install_hint" >&2
  fi
  printf '\n' >&2
  exit 1
}

require_command cargo
require_command rustup
require_command wasm-pack 'cargo install wasm-pack --locked'

case "$(uname -s)" in
  CYGWIN*|MINGW*|MSYS*) native_artifact="target/release/mech.exe" ;;
  *) native_artifact="target/release/mech" ;;
esac

# Do not let a stale native executable make a failed build look complete.
rm -f "$native_artifact"

python3 scripts/build-wasm.py --profile browser-compute
cargo build --locked --release --features compute_backends_native

artifacts=(
  "$native_artifact"
  "src/wasm/pkg/mech_wasm.js"
  "src/wasm/pkg/mech_wasm_bg.wasm"
)

for artifact in "${artifacts[@]}"; do
  if [[ ! -f "$artifact" ]]; then
    printf 'Build completed without expected artifact: %s\n' "$artifact" >&2
    exit 1
  fi
done

if [[ ! -x "$native_artifact" ]]; then
  printf 'Native Mech artifact is not executable: %s\n' "$native_artifact" >&2
  exit 1
fi

printf 'Built the complete Mech resident-executor product:\n'
printf '  %s\n' "${artifacts[@]}"
