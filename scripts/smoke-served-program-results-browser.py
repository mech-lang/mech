#!/usr/bin/env python3
"""Check native results and real served browser results with the shipping controller.

Run after building Mech and `scripts/build-wasm.py --profile browser`:
  MECH_BIN=target/debug/mech python3 scripts/smoke-served-program-results-browser.py
Pass --browser to select Chrome or Edge. Failure artifacts are preserved.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tests.browser.harness import BrowserFailure, ChromeSession, free_port


SOURCE_DIAGNOSTIC = re.compile(
    r"SourceDocumentIndexError|CanonicalProgramCompilationError|source-semantics/|"
    r"syntax/|missing closing delimiter|missing-value",
    re.IGNORECASE,
)
INVARIANTS = ("fizz!", "buzz!", "fizzbuzz!", "first-fifteen!")


def fizzbuzz_values() -> list[str]:
    return [
        "✨🐝" if number % 15 == 0 else "✨" if number % 3 == 0
        else "🐝" if number % 5 == 0 else str(number)
        for number in range(1, 101)
    ]


def quoted_values(text: str) -> list[str]:
    return [json.loads(match.group()) for match in re.finditer(r'"(?:\\.|[^"\\])*"', text)]


def native_result(binary: Path, source: Path, artifacts: Path, *, invalid: bool = False) -> str:
    completed = subprocess.run(
        [str(binary), "--no-config", "run", str(source)], cwd=source.parent,
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        text=True, encoding="utf-8", timeout=45,
    )
    (artifacts / "native.log").write_text(completed.stdout, encoding="utf-8")
    if invalid:
        if completed.returncode == 0 or not SOURCE_DIAGNOSTIC.search(completed.stdout):
            raise BrowserFailure("invalid source did not fail native execution with a diagnostic")
    elif completed.returncode != 0:
        raise BrowserFailure(f"native execution failed: {completed.stdout[-2000:]}")
    return completed.stdout.strip().splitlines()[-1] if completed.stdout.strip() else ""


@contextmanager
def serve(binary: Path, package: Path, source: Path, artifacts: Path,
          *, input_paths: tuple[Path, ...] | None = None):
    port = free_port()
    config = source.parent / "mech.mcfg"
    config_args = ["--config", str(config)] if config.is_file() else ["--no-config"]
    with (artifacts / "server.log").open("wb") as log:
        process = subprocess.Popen(
            [str(binary), *config_args, "serve", *(str(path) for path in input_paths or (source,)),
             "--address", "127.0.0.1",
             "--port", str(port), "--wasm", str(package),
             "--cap-root", str(source.parent),
             "--allow-read", str(package), "--allow-serve", str(package)],
            cwd=source.parent, stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
            start_new_session=os.name != "nt",
        )
        try:
            yield process, f"http://127.0.0.1:{port}/{source.name}"
        finally:
            if process.poll() is None:
                try:
                    if os.name == "nt":
                        process.terminate()
                    else:
                        os.killpg(process.pid, signal.SIGINT)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    try:
                        if os.name == "nt":
                            process.kill()
                        else:
                            os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    process.wait(timeout=5)


def wait_for_page(process: subprocess.Popen, url: str, timeout: float) -> tuple[int, str] | None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                return response.status, response.read().decode("utf-8", errors="replace")
        except urllib.error.HTTPError as error:
            return error.code, error.read().decode("utf-8", errors="replace")
        except OSError:
            if process.poll() is not None:
                return None
            time.sleep(0.1)
    raise BrowserFailure(f"server did not respond within {timeout:g}s")


def wait_for_content(process: subprocess.Popen, url: str, predicate, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        response = wait_for_page(process, url, max(0.1, deadline - time.monotonic()))
        if response is None:
            raise BrowserFailure("server stopped while refreshing an edited source")
        if predicate(*response):
            return
        time.sleep(0.1)
    raise BrowserFailure(f"served source did not refresh at {url}")


SNAPSHOT = r"""(() => {
  const decode = html => {
    const node = document.createElement('div');
    node.innerHTML = html || '';
    return node.textContent.trim();
  };
  const visible = node => {
    const style = getComputedStyle(node);
    const rect = node.getBoundingClientRect();
    return !node.hidden && style.display !== 'none' && style.visibility !== 'hidden' &&
      rect.width > 0 && rect.height > 0;
  };
  const outputs = [...document.querySelectorAll('.mech-block-output[id]')]
    .map(node => ({text: node.textContent.trim(), visible: visible(node)}));
  const yFigure = [...document.querySelectorAll('figure.mech-code-block')]
    .find(node => node.querySelector('code')?.textContent.trim() === 'y');
  const yOutput = yFigure?.querySelector('.mech-block-output');
  const cells = yOutput ? [...yOutput.querySelectorAll('td, th')]
    .map(node => node.textContent.trim().replace(/^"|"$/g, '')).filter(Boolean) : [];
  const controller = globalThis.MechDocumentController;
  const reflected = {};
  if (document.documentElement.dataset.mechDocumentStatus === 'ready' && controller) {
    for (const name of VALUE_NAMES) {
      const value = controller.renderedValue(name);
      reflected[name] = value ? decode(value.inlineHtml) : null;
    }
  }
  return {
    status: document.documentElement.dataset.mechDocumentStatus,
    errors: Object.fromEntries(['mechDocumentError', 'mechWindowError',
      'mechUnhandledRejection'].filter(name => document.documentElement.dataset[name])
      .map(name => [name, document.documentElement.dataset[name]])),
    outputs, reflected, yText: yOutput?.textContent.trim() || '',
    yVisible: Boolean(yOutput && visible(yOutput)), cells,
    diagnostic: document.querySelector('[data-mech-errors-panel]')?.innerText.trim() ||
      document.querySelector('body > pre')?.innerText.trim() || '',
  };
})()"""


def browser_result(browser: ChromeSession, url: str, artifacts: Path, names: list[str],
                   timeout: float, *, invalid: bool = False) -> dict:
    browser.navigate(url)
    browser.wait_for(
        f"location.href === {json.dumps(url)} && Boolean(document.documentElement) && "
        "(['ready', 'error'].includes(document.documentElement.dataset.mechDocumentStatus) || "
        "Boolean(document.querySelector('body > pre')?.innerText.trim()))",
        "a completed document or a visible source diagnostic", timeout=timeout,
    )
    result = browser.evaluate(SNAPSHOT.replace("VALUE_NAMES", json.dumps(names)))
    (artifacts / "result.json").write_text(
        json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8",
    )
    if invalid:
        if result["status"] == "ready" or not SOURCE_DIAGNOSTIC.search(result["diagnostic"]):
            raise BrowserFailure(f"invalid source produced no visible diagnostic: {result}")
    elif result["status"] != "ready" or result["errors"]:
        raise BrowserFailure(f"document failed to execute: {result}")
    elif not any(output["visible"] and output["text"] for output in result["outputs"]):
        raise BrowserFailure(f"document became ready with no visible result: {result}")
    return result


def require_answer(result: dict, expected: str, description: str) -> None:
    if result["reflected"].get("answer") != expected or not any(
        output["visible"] and output["text"] == expected for output in result["outputs"]
    ):
        raise BrowserFailure(f"{description} did not show {expected}: {result}")


def watch_recovery(binary: Path, browser: ChromeSession, process: subprocess.Popen,
                   url: str, source: Path, artifacts: Path, timeout: float) -> None:
    broken = "answer := [\n"
    source.write_text(broken, encoding="utf-8")
    invalid_artifacts = artifacts / "invalid-edit"
    invalid_artifacts.mkdir()
    native_result(binary, source, invalid_artifacts, invalid=True)
    wait_for_content(process, url, lambda _status, body: SOURCE_DIAGNOSTIC.search(body), timeout)
    browser_result(browser, url + "?edit=invalid", invalid_artifacts, [], timeout, invalid=True)
    fixed = "answer := 7 * 7\nanswer\n"
    source.write_text(fixed, encoding="utf-8")
    fixed_artifacts = artifacts / "fixed-edit"
    fixed_artifacts.mkdir()
    if native_result(binary, source, fixed_artifacts) != "49":
        raise BrowserFailure("corrected source did not produce native 49")
    raw_url = url.rsplit("/", 1)[0] + "/source/" + source.name
    wait_for_content(process, raw_url, lambda status, body: status == 200 and body == fixed, timeout)
    result = browser_result(browser, url + "?edit=fixed", fixed_artifacts, ["answer"], timeout)
    require_answer(result, "49", "corrected source")


def import_recovery(binary: Path, browser: ChromeSession, process: subprocess.Popen,
                    url: str, source: Path, artifacts: Path, timeout: float) -> None:
    original = source.read_bytes()
    fixed = "value := 42\n<+ value\n"
    (source.parent / "support.mec").write_text(fixed, encoding="utf-8")
    repaired_artifacts = artifacts / "repaired-import"
    repaired_artifacts.mkdir()
    if native_result(binary, source, repaired_artifacts) != "42":
        raise BrowserFailure("repairing the imported source did not produce native 42")
    raw_url = url.rsplit("/", 1)[0] + "/source/support.mec"
    wait_for_content(process, raw_url, lambda status, body: status == 200 and body == fixed, timeout)
    result = browser_result(browser, url + "?import=repaired", repaired_artifacts, ["answer"], timeout)
    require_answer(result, "42", "root after repairing only its imported source")
    if source.read_bytes() != original:
        raise BrowserFailure("import recovery changed the root source")


def require_private_sibling(process: subprocess.Popen, url: str, timeout: float) -> None:
    base = url.rsplit("/", 1)[0]
    for route in ("/unrelated.mec", "/unrelated.html", "/source/unrelated.mec", "/code/unrelated.mec"):
        response = wait_for_page(process, base + route, timeout)
        if response is None or response[0] != 404:
            raise BrowserFailure(f"file-only serve exposed an unrelated sibling at {route}")


def file_import_updates(binary: Path, browser: ChromeSession, process: subprocess.Popen,
                        url: str, source: Path, artifacts: Path, timeout: float) -> None:
    changed = "value := 49\n<+ value\n"
    support = source.parent / "support.mec"
    support.write_text(changed, encoding="utf-8")
    changed_artifacts = artifacts / "changed-import"
    changed_artifacts.mkdir()
    if native_result(binary, source, changed_artifacts) != "49":
        raise BrowserFailure("changed imported source did not produce native 49")
    raw_url = url.rsplit("/", 1)[0] + "/source/support.mec"
    wait_for_content(process, raw_url, lambda status, body: status == 200 and body == changed, timeout)
    result = browser_result(browser, url + "?import=changed", changed_artifacts, ["answer"], timeout)
    require_answer(result, "49", "root after changing only its imported source")
    support.write_text("value := [\n", encoding="utf-8")
    invalid_artifacts = artifacts / "invalid-import"
    invalid_artifacts.mkdir()
    native_result(binary, source, invalid_artifacts, invalid=True)
    wait_for_content(process, url, lambda _status, body: SOURCE_DIAGNOSTIC.search(body), timeout)
    browser_result(browser, url + "?import=invalid", invalid_artifacts, [], timeout, invalid=True)
    import_recovery(binary, browser, process, url, source, artifacts, timeout)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser", help="path to Chrome or Edge")
    parser.add_argument("--timeout", type=float, default=60)
    parser.add_argument("--wasm", type=Path, default=ROOT / "src/wasm/pkg",
                        help="real browser package to serve (never modified)")
    parser.add_argument("--keep-artifacts", action="store_true")
    parser.add_argument("--case", action="append", choices=(
        "plain", "fenced", "fizzbuzz", "watch-recovery", "syntax-error",
        "unbound-symbol", "type-error", "mixed-workspace", "import-recovery",
        "file-import-recovery", "file-import-invalid-recovery",
    ), help="run only the selected case (repeatable)")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    binary = Path(os.environ.get("MECH_BIN", ROOT / "target/debug/mech")).resolve()
    package = args.wasm.resolve()
    if not binary.is_file():
        parser.error(f"Mech binary does not exist: {binary}; set MECH_BIN")
    if not all((package / filename).is_file() for filename in ("mech_wasm.js", "mech_wasm_bg.wasm")):
        parser.error(f"real browser package is missing: {package}; run scripts/build-wasm.py --profile browser")
    work = Path(tempfile.mkdtemp(prefix="mech-served-program-results-"))
    browser = None
    passed = False
    current = work
    results = []
    try:
        cases = {
            "plain": "answer := 6 * 7\nanswer\n",
            "fenced": "```mech\nanswer := 6 * 7\nanswer\n```\n",
            "fizzbuzz": (ROOT / "examples/working/fizzbuzz.mec").read_bytes(),
            "watch-recovery": "answer := 6 * 7\nanswer\n",
            "syntax-error": "answer := [\n",
            "unbound-symbol": "answer := missing-value + 1\nanswer\n",
            "type-error": "answer := 1 + \"two\"\nanswer\n",
            "mixed-workspace": "answer := [\n",
            "import-recovery": "+> ./support\nanswer := support/value\nanswer\n",
            "file-import-recovery": "+> ./support\nanswer := support/value\nanswer\n",
            "file-import-invalid-recovery": "+> ./support\nanswer := support/value\nanswer\n",
        }
        browser = ChromeSession(args.browser, work / "profile", work / "browser.log",
                                flags=["--disable-gpu", "--no-first-run"]).start()
        for name, source_text in cases.items():
            if args.case and name not in args.case:
                continue
            current = work / name
            current.mkdir()
            mixed = name == "mixed-workspace"
            directory_import = name == "import-recovery"
            rejected_import = name in ("import-recovery", "file-import-invalid-recovery")
            file_import = name.startswith("file-import-")
            source = current / ("broken.mec" if mixed else "main.mec")
            source.write_bytes(source_text if isinstance(source_text, bytes) else source_text.encode())
            invalid = name in (
                "syntax-error", "unbound-symbol", "type-error", "mixed-workspace", "import-recovery",
                "file-import-invalid-recovery",
            )
            if rejected_import or file_import:
                (current / "support.mec").write_text(
                    "value := [\n" if rejected_import else "value := 42\n<+ value\n", encoding="utf-8",
                )
            if file_import:
                (current / "unrelated.mec").write_text("secret := 999\n", encoding="utf-8")
            if mixed:
                valid = current / "main.mec"
                valid.write_text("answer := 6 * 7\nanswer\n", encoding="utf-8")
                (current / "mech.mcfg").write_text(
                    'config := {hosts: [] run: {paths: ["main.mec"] grants: []} '
                    'serve: {paths: ["."]}}\n', encoding="utf-8",
                )
                if native_result(binary, valid, current) != "42":
                    raise BrowserFailure("valid mixed-workspace source did not return native 42")
            native = native_result(binary, source, current, invalid=invalid)
            if name in ("plain", "fenced", "watch-recovery", "file-import-recovery") and native != "42":
                raise BrowserFailure(f"{name} native result was {native!r}, expected '42'")
            if name == "fizzbuzz" and quoted_values(native) != fizzbuzz_values():
                raise BrowserFailure("native FizzBuzz did not produce the expected 100 values")
            with serve(binary, package, source, current,
                       input_paths=(valid, current) if mixed else
                       (source, current) if directory_import else None) as (process, url):
                response = wait_for_page(process, url, args.timeout)
                if file_import:
                    require_private_sibling(process, url, args.timeout)
                if mixed:
                    if response is None:
                        raise BrowserFailure("mixed workspace failed before its valid root could execute")
                    main_result = browser_result(
                        browser, url.rsplit("/", 1)[0] + "/main.mec", current,
                        ["answer"], args.timeout,
                    )
                    if main_result["reflected"].get("answer") != "42":
                        raise BrowserFailure("valid mixed-workspace source did not render 42")
                if rejected_import:
                    if response is None:
                        raise BrowserFailure("server stopped before its rejected import could be repaired")
                    browser_result(browser, url, current, [], args.timeout, invalid=True)
                    import_recovery(binary, browser, process, url, source, current, args.timeout)
                elif response is None:
                    log = (current / "server.log").read_text(errors="replace")
                    if not invalid or process.returncode == 0 or not SOURCE_DIAGNOSTIC.search(log):
                        raise BrowserFailure(f"server exited without an expected diagnostic: {log[-2000:]}")
                    result = {"rejected": "server", "exitCode": process.returncode}
                elif response[0] >= 400:
                    (current / "request-error.html").write_text(response[1], encoding="utf-8")
                    if not invalid or not SOURCE_DIAGNOSTIC.search(response[1]):
                        raise BrowserFailure(f"unexpected HTTP {response[0]}: {response[1][-2000:]}")
                    result = {"rejected": "request", "status": response[0]}
                else:
                    names = ["y", *INVARIANTS] if name == "fizzbuzz" else [] if invalid else ["answer"]
                    result = browser_result(browser, url, current, names, args.timeout, invalid=invalid)
                    if name in ("plain", "fenced", "watch-recovery", "file-import-recovery"):
                        require_answer(result, "42", name)
                        if name == "watch-recovery":
                            watch_recovery(binary, browser, process, url, source, current, args.timeout)
                        elif name == "file-import-recovery":
                            file_import_updates(binary, browser, process, url, source, current, args.timeout)
                    elif name == "fizzbuzz":
                        values = result["cells"] or quoted_values(result["yText"])
                        if not result["yVisible"] or values != fizzbuzz_values():
                            raise BrowserFailure(f"visible FizzBuzz output was incorrect: {values}")
                        if quoted_values(result["reflected"].get("y") or "") != fizzbuzz_values():
                            raise BrowserFailure("resident FizzBuzz value differed from its visible result")
                        if any(result["reflected"].get(invariant) != "true" for invariant in INVARIANTS):
                            raise BrowserFailure(f"FizzBuzz invariants failed: {result['reflected']}")
                if file_import:
                    require_private_sibling(process, url, args.timeout)
                outcome = "recovered" if rejected_import or file_import else "rejected" if invalid else "correct"
                results.append({"case": name, "native": outcome, "browser": outcome})
                print(f"PASS {name}", flush=True)
        passed = True
        print("SERVED_PROGRAM_RESULTS " + json.dumps(results))
    finally:
        if browser:
            if not passed:
                try:
                    browser.write_dom(current / "failure.html")
                    browser.capture_screenshot(current / "failure.png")
                except Exception:
                    pass
            browser.close()
        if passed and not args.keep_artifacts:
            shutil.rmtree(work)
        else:
            print(f"Served program result artifacts: {work}", file=sys.stderr)


if __name__ == "__main__":
    main()
