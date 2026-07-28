#!/bin/sh
set -eu

script_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
repository_root=${UNSAFE_BOUNDARY_ROOT:-$script_root}
allowlist=${UNSAFE_BOUNDARY_ALLOWLIST:-$repository_root/scripts/unsafe-boundaries.allowlist}

case "$allowlist" in
  /*) ;;
  *) allowlist="$repository_root/$allowlist" ;;
esac

fail() {
  echo "unsafe boundary audit failed: $*" >&2
  exit 1
}

[ -f "$allowlist" ] || fail "allowlist does not exist: $allowlist"

temporary_directory=$(mktemp -d "${TMPDIR:-/tmp}/mech-unsafe-audit.XXXXXX")
trap 'rm -rf "$temporary_directory"' EXIT HUP INT TERM
allowlisted_paths="$temporary_directory/allowlisted-paths"
rust_files="$temporary_directory/rust-files"
: > "$allowlisted_paths"

unsafe_syntax='unsafe[[:space:]]*(\{|fn|impl|trait|extern)|#\[unsafe[[:space:]]*\('
prohibited_syntax='(^|[^[:alnum:]_])(transmute(_copy)?|from_raw|from_raw_parts(_mut)?|ptr::(read|write|copy|copy_nonoverlapping)|NonNull::new_unchecked|get_unchecked(_mut)?|unwrap_unchecked|unreachable_unchecked|assume_init)[[:space:]]*[:<(]|Value::as_unchecked|\.as_unchecked[[:space:]]*[(<]'
raw_pointer_syntax='(\*|as[[:space:]]+\*)[[:space:]]*(const|mut)[[:space:]]+[[:alnum:]_:(<\[]'

code_matches() {
  matches=$(grep -En -- "$1" "$2" 2>/dev/null || true)
  printf '%s\n' "$matches" |
    grep -Ev '^[0-9]+:[[:space:]]*(//|/\*|\*)' ||
    true
}

line_number=0
while IFS= read -r line || [ -n "$line" ]; do
  line_number=$((line_number + 1))
  case "$line" in
    ''|'#'*) continue ;;
  esac

  previous_ifs=$IFS
  IFS='|'
  set -- $line
  IFS=$previous_ifs
  [ "$#" -eq 5 ] || fail "malformed allowlist line $line_number: expected five fields"
  path=$1
  category=$2
  owner=$3
  invariant=$4
  justification=$5
  [ -n "$path" ] || fail "malformed allowlist line $line_number: path is empty"
  [ -n "$category" ] || fail "malformed allowlist line $line_number: category is empty"
  [ -n "$owner" ] || fail "malformed allowlist line $line_number: owner is empty"
  [ -n "$invariant" ] || fail "malformed allowlist line $line_number: invariant is empty"
  [ -n "$justification" ] || fail "malformed allowlist line $line_number: justification is empty"

  case "$path" in
    /*|..|../*|*/..|*/../*)
      fail "allowlist line $line_number is not an exact repository-relative path: $path"
      ;;
  esac
  [ ! -d "$repository_root/$path" ] ||
    fail "allowlist line $line_number names a directory: $path"
  [ -f "$repository_root/$path" ] ||
    fail "allowlisted file does not exist: $path"
  if grep -Fqx -- "$path" "$allowlisted_paths"; then
    fail "duplicate allowlist entry: $path"
  fi
  printf '%s\n' "$path" >> "$allowlisted_paths"

  allowlisted_matches=$(code_matches \
    "$unsafe_syntax|$prohibited_syntax|$raw_pointer_syntax" \
    "$repository_root/$path")
  if [ -z "$allowlisted_matches" ]; then
    fail "stale allowlist entry contains no audited unsafe syntax: $path"
  fi
done < "$allowlist"

cd "$repository_root"
find . \
  \( -path './.git' -o -path './target' -o -path './vendor' \) -prune \
  -o -type f -name '*.rs' -print |
  sed 's#^\./##' |
  sort > "$rust_files"

while IFS= read -r path; do
  if grep -Fqx -- "$path" "$allowlisted_paths"; then
    continue
  fi
  matches=$(code_matches \
    "$unsafe_syntax|$prohibited_syntax|$raw_pointer_syntax" \
    "$path")
  if [ -n "$matches" ]; then
    echo "unsafe boundary audit failed: prohibited code exists outside the exact allowlist: $path" >&2
    echo "$matches" >&2
    exit 1
  fi
done < "$rust_files"

global_prohibited='ACTIVE_RUNTIME_PROGRAM_HOST|RuntimeProgramHostTarget|ActiveRuntimeProgramHostGuard|CURRENT_RUNTIME_(PTR|TARGET)|RUNTIME_(PTR|TARGET)_TLS|unsafe[[:space:]]+impl[[:space:]]+(Send|Sync)|journal[^[:space:](]*_unchecked|_unchecked[^[:space:](]*journal'
while IFS= read -r path; do
  global_matches=$(code_matches "$global_prohibited" "$path")
  if [ -n "$global_matches" ]; then
    echo "unsafe boundary audit failed: globally prohibited runtime architecture or manual safety promise exists: $path" >&2
    echo "$global_matches" >&2
    exit 1
  fi
done < "$rust_files"

echo "unsafe boundary audit passed"
