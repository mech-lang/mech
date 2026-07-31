#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  if [[ "$CARGO_TARGET_DIR" = /* ]]; then
    target_dir="$CARGO_TARGET_DIR"
  else
    target_dir="$repo_root/$CARGO_TARGET_DIR"
  fi
else
  target_dir="$repo_root/target"
fi

MECH_BIN="${MECH_BIN:-$target_dir/debug/mech}"
if [[ ! -x "$MECH_BIN" ]]; then
  echo "Mech binary is not executable: $MECH_BIN" >&2
  exit 1
fi

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

# The server binary must have embedded the package at compile time.
rm -rf "$repo_root/src/wasm/pkg"

work_dir="$(mktemp -d "$target_dir/served-fizzbuzz.XXXXXX")"
server_pid=""

cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill -INT "$server_pid" 2>/dev/null || true
    for _ in $(seq 1 50); do
      if ! kill -0 "$server_pid" 2>/dev/null; then
        break
      fi
      sleep 0.1
    done
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$work_dir"
}
trap cleanup EXIT

cp examples/working/fizzbuzz.mec "$work_dir/fizzbuzz.mec"
cp tests/fixtures/serve/inline-shim.html "$work_dir/inline-shim.html"

port_for_test() {
  python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

wait_for_server() {
  local page_url="$1"
  local server_log="$2"
  for _ in $(seq 1 150); do
    if curl --fail --silent --output /dev/null "$page_url" 2>/dev/null; then
      return
    fi
    sleep 0.1
  done
  echo "Server did not respond at $page_url" >&2
  sed -n '1,260p' "$server_log" >&2 || true
  exit 1
}

assert_fizzbuzz_dom() {
  local dom_file="$1"
  python3 - "$dom_file" <<'PY'
from html.parser import HTMLParser
from pathlib import Path
import sys

class Node:
    def __init__(self, tag="", attrs=(), parent=None):
        self.tag = tag
        self.attrs = dict(attrs)
        self.parent = parent
        self.children = []
        self.text = []

class Tree(HTMLParser):
    def __init__(self):
        super().__init__()
        self.root = Node()
        self.stack = [self.root]
    def handle_starttag(self, tag, attrs):
        node = Node(tag, attrs, self.stack[-1])
        self.stack[-1].children.append(node)
        if tag not in {"area", "base", "br", "col", "embed", "hr", "img", "input",
                       "link", "meta", "param", "source", "track", "wbr"}:
            self.stack.append(node)
    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)
        if len(self.stack) > 1 and self.stack[-1].tag == tag:
            self.stack.pop()
    def handle_endtag(self, tag):
        for index in range(len(self.stack) - 1, 0, -1):
            if self.stack[index].tag == tag:
                del self.stack[index:]
                break
    def handle_data(self, data):
        self.stack[-1].text.append(data)

def walk(node):
    yield node
    for child in node.children:
        yield from walk(child)

def classes(node):
    return set(node.attrs.get("class", "").split())

def text(node):
    return "".join(node.text) + "".join(text(child) for child in node.children)

tree = Tree()
tree.feed(Path(sys.argv[1]).read_text())
html = next((node for node in walk(tree.root) if node.tag == "html"), None)
if html is None:
    raise SystemExit("dumped DOM has no html element")
if html.attrs.get("data-mech-document-status") != "ready":
    raise SystemExit(f"document did not become ready: {html.attrs!r}")
for forbidden in (
    "data-mech-document-error",
    "data-mech-window-error",
    "data-mech-unhandled-rejection",
):
    if forbidden in html.attrs:
        raise SystemExit(f"browser error marker {forbidden}: {html.attrs[forbidden]}")

outputs = [
    node for node in walk(tree.root)
    if "mech-block-output" in classes(node) and text(node).strip()
]
if not outputs:
    raise SystemExit("no nonempty Mech block output was rendered")

y_output = None
for block in walk(tree.root):
    if "mech-fenced-mech-block" not in classes(block):
        continue
    code = next(
        (node for node in walk(block) if "mech-code-block" in classes(node)),
        None,
    )
    output = next(
        (node for node in walk(block) if "mech-block-output" in classes(node)),
        None,
    )
    if code is not None and output is not None and text(code).strip() == "y":
        y_output = output
        break
if y_output is None:
    raise SystemExit("could not find the output box beneath the `y` block")

cells = [
    text(node).strip().strip('"')
    for node in walk(y_output)
    if node.tag in {"td", "th"} and text(node).strip()
]
expected = [
    "1", "2", "✨", "4", "🐝", "✨", "7", "8",
    "✨", "🐝", "11", "✨", "13", "14", "✨🐝",
]
if cells[:15] != expected:
    raise SystemExit(
        f"unexpected values in y output; first cells were {cells[:15]!r}"
    )
PY
}

run_chrome() {
  local page_url="$1"
  local chrome_profile="$2"
  local dom_file="$3"
  local chrome_log="$4"
  python3 - \
    "$chrome_bin" \
    "$page_url" \
    "$chrome_profile" \
    "$dom_file" \
    "$chrome_log" <<'PY'
import os
from pathlib import Path
import signal
import subprocess
import sys

chrome, page_url, profile, dom_file, chrome_log = sys.argv[1:]
args = [
    chrome,
    "--headless=new",
    "--no-sandbox",
    "--disable-gpu",
    "--disable-dev-shm-usage",
    "--run-all-compositor-stages-before-draw",
    "--virtual-time-budget=20000",
    "--dump-dom",
    f"--user-data-dir={profile}",
    page_url,
]
with Path(dom_file).open("wb") as stdout, Path(chrome_log).open("wb") as stderr:
    process = subprocess.Popen(
        args,
        stdout=stdout,
        stderr=stderr,
        start_new_session=True,
    )
    try:
        raise SystemExit(process.wait(timeout=45))
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        print("headless Chrome exceeded the 45-second smoke deadline", file=sys.stderr)
        raise SystemExit(124)
PY
}

run_case() {
  local label="$1"
  shift
  local port
  port="$(port_for_test)"
  local case_dir="$work_dir/$label"
  mkdir -p "$case_dir"
  local server_log="$case_dir/server.log"
  local chrome_log="$case_dir/chrome.stderr"
  local dom_file="$case_dir/chrome.dom"
  local chrome_profile="$case_dir/chrome-profile"

  "$MECH_BIN" --no-config serve \
    --address 127.0.0.1 \
    --port "$port" \
    "$work_dir/fizzbuzz.mec" \
    "$@" >"$server_log" 2>&1 &
  server_pid="$!"

  local page_url="http://127.0.0.1:${port}/"
  wait_for_server "$page_url" "$server_log"

  curl --fail --silent --show-error \
    "${page_url}_mech/pkg/mech_wasm_bg.wasm" \
    >"$case_dir/mech_wasm_bg.wasm"
  python3 - "$case_dir/mech_wasm_bg.wasm" <<'PY'
from pathlib import Path
import sys
if not Path(sys.argv[1]).read_bytes().startswith(b"\0asm"):
    raise SystemExit("served WASM does not start with raw WebAssembly magic")
PY

  set +e
  run_chrome "$page_url" "$chrome_profile" "$dom_file" "$chrome_log"
  local chrome_status="$?"
  set -e

  if [[ "$chrome_status" -ne 0 ]] \
    || grep -qE 'Uncaught|UnhandledPromiseRejection' "$chrome_log"; then
    echo "FizzBuzz browser case failed: $label" >&2
    sed -n '1,260p' "$server_log" >&2 || true
    sed -n '1,260p' "$chrome_log" >&2 || true
    sed -n '1,420p' "$dom_file" >&2 || true
    exit 1
  fi
  if ! assert_fizzbuzz_dom "$dom_file"; then
    echo "FizzBuzz DOM assertion failed: $label" >&2
    sed -n '1,260p' "$server_log" >&2 || true
    sed -n '1,260p' "$chrome_log" >&2 || true
    grep -nE \
      'data-mech-document|data-mech-block|mech-block-output|mech-document-error' \
      "$dom_file" \
      | tail -40 \
      | cut -c1-2000 >&2 || true
    sed -n '1,420p' "$dom_file" >&2 || true
    exit 1
  fi

  for route in \
    "/" \
    "/_mech/pkg/mech_wasm.js" \
    "/_mech/pkg/mech_wasm_bg.wasm"; do
    if ! grep -F "GET $route ->" "$server_log" >/dev/null; then
      echo "Browser did not request $route in $label case" >&2
      sed -n '1,260p' "$server_log" >&2 || true
      exit 1
    fi
  done
  if [[ "$label" = "default" ]] \
    && ! grep -F "GET /code/fizzbuzz.mec ->" "$server_log" >/dev/null; then
    echo "Default shim did not request /code/fizzbuzz.mec" >&2
    sed -n '1,260p' "$server_log" >&2 || true
    exit 1
  fi
  if [[ "$label" = "inline" ]] \
    && ! grep -F 'data-mech-inline-shim="true"' "$dom_file" >/dev/null; then
    echo "Custom inline shim did not own the rendered page" >&2
    exit 1
  fi
  if grep -F "GET /mech.mcfg ->" "$server_log" >/dev/null \
    || grep -F "GET /_mech/project.js ->" "$server_log" >/dev/null; then
    echo "Standalone browser case unexpectedly requested project assets: $label" >&2
    sed -n '1,260p' "$server_log" >&2 || true
    exit 1
  fi

  kill -INT "$server_pid"
  for _ in $(seq 1 50); do
    if ! kill -0 "$server_pid" 2>/dev/null; then
      break
    fi
    sleep 0.1
  done
  if kill -0 "$server_pid" 2>/dev/null; then
    echo "Server did not stop after one interrupt in $label case" >&2
    exit 1
  fi
  wait "$server_pid"
  server_pid=""
}

run_case default
run_case inline --shim "$work_dir/inline-shim.html"
