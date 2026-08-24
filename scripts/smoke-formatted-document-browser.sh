#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  target_dir="$CARGO_TARGET_DIR"
  [[ "$target_dir" = /* ]] || target_dir="$repo_root/$target_dir"
else
  target_dir="$repo_root/target"
fi

MECH_BIN="${MECH_BIN:-$target_dir/debug/mech}"
[[ -x "$MECH_BIN" ]] || { echo "Mech binary is not executable: $MECH_BIN" >&2; exit 1; }

if [[ -n "${CHROME_BIN:-}" ]]; then
  chrome_bin="$CHROME_BIN"
elif command -v google-chrome >/dev/null 2>&1; then
  chrome_bin="$(command -v google-chrome)"
elif [[ -x "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]]; then
  chrome_bin="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
else
  echo "Google Chrome was not found" >&2
  exit 1
fi

work_dir="$(mktemp -d "$target_dir/formatted-document.XXXXXX")"
server_pid=""

cleanup() {
  local status="$?"
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  if [[ "$status" -ne 0 ]]; then
    echo "Formatted document browser artifacts retained at: $work_dir" >&2
  else
    rm -rf "$work_dir"
  fi
}
trap cleanup EXIT

port_for_test() {
  python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

mkdir -p \
  "$work_dir/project/vendor" \
  "$work_dir/project/package" \
  "$work_dir/shared"
cat > "$work_dir/project/main.mec" <<'MEC'
+> ./café.mec
+> ./extdep
+> ./package
+> ./vendor/support.mec
+> ./vendor/percent.mec
+> ./rate%.mec
{included.mec}
~answer := 0
answer += café/value + extdep/value + package/value + support/value + percent/value + included-value + nested-included-value
MEC
cat > "$work_dir/project/café.mec" <<'MEC'
value := 2
<+ value
MEC
cat > "$work_dir/project/extdep.mec" <<'MEC'
value := 3
<+ value
MEC
cat > "$work_dir/project/package/index.mec" <<'MEC'
value := 5
<+ value
MEC
cat > "$work_dir/project/included.mec" <<'MEC'
{nested-included.mec}
included-value := 13
MEC
cat > "$work_dir/project/nested-included.mec" <<'MEC'
nested-included-value := 17
MEC
cat > "$work_dir/project/rate%.mec" <<'MEC'
value := 29
literal-percent-pass! := value == 29
<+ value
MEC
cat > "$work_dir/shared/support.mec" <<'MEC'
+> ./nested.mec
value := nested/value
<+ value
MEC
cat > "$work_dir/shared/nested.mec" <<'MEC'
value := 7
<+ value
MEC
cat > "$work_dir/shared/rate%.mec" <<'MEC'
value := 11
<+ value
MEC
ln -s ../../shared/support.mec "$work_dir/project/vendor/support.mec"
ln -s '../../shared/rate%.mec' "$work_dir/project/vendor/percent.mec"
output_dir="$work_dir/static"
format_log="$work_dir/format.log"
if ! "$MECH_BIN" --no-config format "$work_dir/project/main.mec" --html --out "$output_dir" >"$format_log" 2>&1; then
  sed -n '1,240p' "$format_log" >&2 || true
  exit 1
fi

page_file="$(find "$output_dir" -name main.html -type f -print -quit)"
[[ -n "$page_file" ]] || { echo "formatter did not emit main.html" >&2; exit 1; }
[[ -s "$output_dir/_mech/pkg/mech_wasm.js" ]] || { echo "formatter did not emit mech_wasm.js" >&2; exit 1; }
[[ -s "$output_dir/_mech/pkg/mech_wasm_bg.wasm" ]] || {
  echo "formatter did not emit mech_wasm_bg.wasm" >&2
  exit 1
}
canonical_support="$(cd "$work_dir/shared" && pwd -P)/support.mec"
canonical_percent="$(cd "$work_dir/shared" && pwd -P)/rate%.mec"
if grep -F "$work_dir" "$page_file" >/dev/null \
  || grep -F 'file://' "$page_file" >/dev/null \
  || grep -F "$canonical_support" "$page_file" >/dev/null \
  || grep -F "$canonical_percent" "$page_file" >/dev/null; then
  echo "standalone source bundle leaked a filesystem location" >&2
  exit 1
fi

port="$(port_for_test)"
server_log="$work_dir/static-server.log"
PYTHONUNBUFFERED=1 python3 -m http.server "$port" --bind 127.0.0.1 --directory "$output_dir" >"$server_log" 2>&1 &
server_pid="$!"
page_relative="${page_file#"$output_dir"/}"
page_url="http://127.0.0.1:${port}/${page_relative}"
for _ in $(seq 1 150); do
  if curl --fail --silent --output /dev/null "$page_url" 2>/dev/null; then
    break
  fi
  sleep 0.1
done
curl --fail --silent --output /dev/null "$page_url" || {
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
}

python3 - "$chrome_bin" "$page_url" "$work_dir/chrome-profile" "$work_dir/chrome.log" <<'PY'
import json
import sys

from tests.browser.harness import BrowserFailure, ChromeSession


chrome, page_url, profile, chrome_log = sys.argv[1:]


def fail(message):
    raise BrowserFailure(message)


browser = ChromeSession(chrome, profile, chrome_log, flags=["--disable-gpu"]).start()
try:
    browser.navigate(page_url)
    evaluate = browser.evaluate

    def wait_for(expression, description):
        return browser.wait_for(expression, description, timeout=40)
    wait_for(
        "(() => { const html = document.documentElement; const root = document.querySelector('.mech-root'); "
        "const input = document.querySelector('.repl-input'); return Boolean(html && root && input && "
        "html.dataset.mechDocumentStatus === 'ready' && root.dataset.mechDocumentStatus === 'ready' && "
        "root.dataset.mechConsoleStatus === 'ready'); })()",
        "the standalone document controller",
    )

    def submit(command):
        encoded = json.dumps(command)
        if not evaluate(
            "(() => { const input = document.querySelector('.repl-input'); if (!input) return false; "
            f"input.focus(); input.value = {encoded}; input.dispatchEvent(new KeyboardEvent('keydown', {{key: 'Enter', bubbles: true, cancelable: true}})); return true; }})()"
        ):
            fail(f"could not submit browser REPL command: {command}")

    def wait_for_new_symbol_value(previous_count, name, value, description):
        wait_for(
            "(() => { const tables = [...document.querySelectorAll('.mech-repl-symbols')]; "
            f"const table = tables.at(-1); return tables.length > {previous_count} && "
            f"[...(table?.tBodies[0]?.rows || [])].some(row => "
            f"row.cells[0]?.textContent.trim() === {json.dumps(name)} && "
            f"row.textContent.includes({json.dumps(value)})); }})()",
            description,
        )

    submit("answer = 58")
    submit("answer")
    wait_for("[...document.querySelectorAll('.mech-repl-result')].some(row => /58/.test(row.textContent))", "the resident source value")
    previous_symbol_tables = evaluate("document.querySelectorAll('.mech-repl-symbols').length")
    submit(":whos answer")
    wait_for_new_symbol_value(previous_symbol_tables, "answer", "58", "the resident symbol value")
    submit(":clear")
    wait_for("[...document.querySelectorAll('.mech-repl-info')].some(row => /Resident workspace cleared/.test(row.textContent))", "the resident workspace clear")
    submit("answer := 59")
    previous_symbol_tables = evaluate("document.querySelectorAll('.mech-repl-symbols').length")
    submit(":whos answer")
    wait_for_new_symbol_value(previous_symbol_tables, "answer", "59", "the resident symbol value after reset")
    popup_state = evaluate("""
(() => {
  const root = document.querySelector('.mech-root');
  const pane = document.querySelector('[data-mech-console-pane]');
  const transcript = document.querySelector('.mech-repl-transcript');
  const value = [...document.querySelectorAll('.mech-var-name')].find(element =>
    !element.closest('[data-mech-console-pane]') &&
    (element.dataset.mechVarName || element.textContent.trim()) === 'answer');
  if (!root || !pane || !transcript || !value) return null;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  const transcriptEntries = transcript.children.length;
  value.click();
  const popup = document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  if (!popup) return null;
  const style = getComputedStyle(popup);
  const rect = popup.getBoundingClientRect();
  const valueRect = value.getBoundingClientRect();
  const result = {
    consoleClosed: root.dataset.mechConsoleOpen === 'false' && pane.hidden,
    rendered: /59/.test(popup.textContent || ''),
    role: popup.getAttribute('role'),
    styled:
      style.position === 'fixed' && style.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
      rect.width >= 200 && rect.height > 40,
    contained:
      rect.left >= 0 && rect.top >= 0 && rect.right <= innerWidth && rect.bottom <= innerHeight,
    anchored: Math.abs(rect.top - valueRect.top) < 80,
    transcriptClean: transcript.children.length === transcriptEntries,
  };
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'Escape', bubbles: true, cancelable: true,
  }));
  result.dismissed = !document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  result.reopened = root.dataset.mechConsoleOpen === 'true' && !pane.hidden;
  return result;
})()
""")
    if (
        popup_state is None or
        not popup_state["consoleClosed"] or
        not popup_state["rendered"] or
        popup_state["role"] != "dialog" or
        not popup_state["styled"] or
        not popup_state["contained"] or
        not popup_state["anchored"] or
        not popup_state["transcriptClean"] or
        not popup_state["dismissed"] or
        not popup_state["reopened"]
    ):
        fail(f"closed standalone console did not show a styled value popup: {popup_state!r}")
    previous_symbol_tables = evaluate("document.querySelectorAll('.mech-repl-symbols').length")
    submit(":whos ans")
    wait_for_new_symbol_value(
        previous_symbol_tables,
        "ans",
        "59",
        "the popup selection becoming ans",
    )
finally:
    browser.close()
PY

for request in \
  "GET /${page_relative} " \
  'GET /_mech/pkg/mech_wasm.js ' \
  'GET /_mech/pkg/mech_wasm_bg.wasm '; do
  grep -F "$request" "$server_log" >/dev/null || {
    echo "standalone browser did not request $request" >&2
    sed -n '1,240p' "$server_log" >&2 || true
    exit 1
  }
done

if grep -E 'GET /(code|source)/|GET /_mech/project-sources\.json|GET /mech\.mcfg|GET /_mech/project\.js' "$server_log" >/dev/null; then
  echo "standalone browser requested a server-only Mech route" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
fi

echo "formatted static document browser smoke passed"
