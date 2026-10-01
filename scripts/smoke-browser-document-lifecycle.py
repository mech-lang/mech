#!/usr/bin/env python3
"""Exercise document occurrence and console transitions through the shipped controller."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tests.browser.harness import ChromeSession, free_port, wait_for_http


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mech-bin", default="target/debug/mech")
    parser.add_argument("--artifacts", default="target/browser-document-lifecycle")
    args = parser.parse_args()
    binary = Path(args.mech_bin).resolve()
    artifacts = Path(args.artifacts).resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    results = []
    with tempfile.TemporaryDirectory(prefix="document-source-", dir=artifacts) as directory:
        source_dir = Path(directory)
        (source_dir / "fence.mec").write_text("~~~mech\n11\n~~~\n")
        original = "first := 1\nsecond := 2\nsecond\n\nVisible {second}.\n"
        (source_dir / "console.mec").write_text(original)
        port = free_port()
        with (artifacts / "server.log").open("wb") as server_log:
            server = subprocess.Popen([str(binary), "--no-config", "serve", "--address", "127.0.0.1", "--port", str(port), str(source_dir)], stdout=server_log, stderr=subprocess.STDOUT)
            browser = None
            try:
                url = f"http://127.0.0.1:{port}"
                wait_for_http(url + "/fence.mec", server)
                browser = ChromeSession(None, artifacts / "chrome-profile", artifacts / "chrome.log", flags=["--disable-gpu"]).start()
                browser.navigate(url + "/fence.mec")
                browser.wait_for("document.documentElement.dataset.mechDocumentStatus === 'ready'", "fence document readiness")
                result = browser.evaluate_json("""(async () => {
                  const controller = globalThis.MechDocumentController;
                  const original = document.querySelector('.mech-block-output[id]');
                  const address = original?.id;
                  const check = (value) => {
                    if (!original || document.getElementById(address) !== original || original.textContent.trim() !== value) throw new Error(`Original placeholder: ${original?.textContent}, expected ${value}`);
                  };
                  check('11');
                  controller.applyEdit(7, 7, '{output: false}');
                  const body = controller.source().indexOf('11');
                  controller.applyEdit(body, body + 2, '22');
                  const accepted = controller.source();
                  let rejected = false;
                  try { controller.applyEdit(body, body + 2, '['); } catch (_) { rejected = true; }
                  if (!rejected || controller.source() !== accepted) throw new Error('Malformed suppressed edit changed accepted source');
                  controller.applyEdit(0, 0, 'Prose before the fence.\\n\\n');
                  const end = controller.source().length;
                  controller.applyEdit(end, end, '\\nProse after the fence.\\n');
                  const suppressor = controller.source().indexOf('{output: false}');
                  controller.applyEdit(suppressor, suppressor + 15, '');
                  check('22');
                  const fenceStart = controller.source().indexOf('~~~mech');
                  controller.applyEdit(fenceStart + 7, fenceStart + 7, '{output: false}');
                  controller.applyEdit(0, controller.source().length, '');
                  controller.applyEdit(0, 0, '~~~mech\\n33\\n~~~\\n\\n~~~mech\\n33\\n~~~\\n');
                  if (original.textContent.trim() === '33') throw new Error('Deleted placeholder adopted an unrelated duplicate');
                  return {contract:'suppression-edit-restoration', address, originalNodeRetained:document.getElementById(address) === original, malformedEditRejected:rejected, restoredValue:'22', unrelatedDuplicateRejected:true};
                })()""")
                results.append(result)
                browser.write_dom(artifacts / "fence.dom.html")
                browser.navigate(url + "/console.mec")
                browser.wait_for("document.documentElement.dataset.mechDocumentStatus === 'ready'", "console document readiness")
                result = browser.evaluate_json("""(async () => {
                  const controller = globalThis.MechDocumentController;
                  const initial = controller.source();
                  const address = document.querySelector('.mech-inline-mech-code[id]')?.id;
                  const states = [];
                  const check = (phase, first, extra) => {
                    const source = controller.source();
                    const output = document.querySelector('[data-mech-output-panel] .mech-document-output-html')?.textContent.trim();
                    const inline = document.getElementById(address)?.textContent.trim();
                    if (output !== '2' || inline !== '2' || source.includes('first := 1') !== first || source.includes('extra := 99') !== extra) throw new Error(`${phase}: ${JSON.stringify({source, output, inline})}`);
                    states.push({phase, source, output, inline});
                  };
                  check('initial', true, false);
                  await controller.invoke('extra := 99'); check('submit', true, true);
                  await controller.invoke(':clear first'); check('clear document definition', false, true);
                  const accepted = controller.source();
                  await controller.invoke(':clear second');
                  if (controller.source() !== accepted) throw new Error('Rejected dependent clear changed source');
                  check('rejected clear', false, true);
                  await controller.invoke(':reset'); check('reset', true, false);
                  if (controller.source() !== initial) throw new Error('Reset did not restore exact original source');
                  await controller.invoke('extra := 99');
                  await controller.invoke(':clear extra'); check('clear console definition', true, false);
                  await controller.invoke('extra := 99'); check('continued usability', true, true);
                  return {contract:'console-document-reset', address, states};
                })()""")
                results.append(result)
                browser.write_dom(artifacts / "console.dom.html")
                (artifacts / "results.json").write_text(json.dumps(results, indent=2) + "\n")
                print("browser document lifecycle: 2 operation sequences passed")
            finally:
                if browser is not None:
                    browser.close()
                if server.poll() is None:
                    server.send_signal(signal.SIGINT)
                    try:
                        server.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        server.terminate()
                        server.wait(timeout=10)


if __name__ == "__main__":
    main()
