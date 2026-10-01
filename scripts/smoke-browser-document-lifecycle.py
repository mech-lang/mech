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


OUTPUT_ASSERTIONS = """
  const checkOutput = (node, address, value) => {
    if (!node || document.getElementById(address) !== node || node.textContent.trim() !== value) {
      throw new Error(`Original placeholder: ${node?.textContent}, expected ${value}`);
    }
    if (!node.classList.contains('mech-clickable') || node.getAttribute('role') !== 'button' || node.tabIndex !== 0 || node.hasAttribute('aria-disabled')) {
      throw new Error('Published output is not selectable');
    }
  };
  const checkUnavailable = (node, address) => {
    if (document.getElementById(address) !== node || node.childNodes.length !== 0 || node.hasAttribute('data-mech-source')) {
      throw new Error(`Unavailable placeholder retains content: ${node.textContent}`);
    }
    if (node.dataset.mechValueAvailable !== 'false' || node.classList.contains('mech-clickable') || node.hasAttribute('role') || node.hasAttribute('tabindex') || node.getAttribute('aria-disabled') !== 'true') {
      throw new Error('Unavailable placeholder remains selectable');
    }
    const selectionState = () => JSON.stringify([
      ...document.querySelectorAll('[data-mech-repl-popup], .mech-repl-transcript, [data-mech-errors-panel]'),
    ].map(element => element.outerHTML));
    const before = selectionState();
    node.click();
    for (const key of ['Enter', ' ']) {
      node.dispatchEvent(new KeyboardEvent('keydown', {key, bubbles:true, cancelable:true}));
    }
    if (selectionState() !== before) throw new Error('Unavailable output handled a selection');
  };
  const checkSelection = (node, address, value) => {
    checkOutput(node, address, value);
    // Use the closed-console inspector to observe selection of the restored node.
    const root = document.querySelector('.mech-root');
    const consoleOpen = root.dataset.mechConsoleOpen;
    root.dataset.mechConsoleOpen = 'false';
    node.click();
    const popup = document.querySelector('[data-mech-repl-popup]');
    if (!popup || popup.querySelector('.mech-output-value')?.textContent.trim() !== value || popup.classList.contains('mech-inline-popup--error')) {
      throw new Error('Restored output selection did not inspect the current value');
    }
    popup.querySelector('.mech-inline-popup__close').click();
    root.dataset.mechConsoleOpen = consoleOpen;
  };
"""


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
        (source_dir / "inline.mec").write_text("anchor := 0\n\nVisible {11}.\n")
        (source_dir / "title.mec").write_text("Document\n========\nsection: {1}\n========\n\nVisible {1}.\n")
        (source_dir / "documentation.mec").write_text("answer := 1\nanswer")
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
                browser.wait_for("document.documentElement?.dataset.mechDocumentStatus === 'ready'", "fence document readiness")
                result = browser.evaluate_json("(async () => {" + OUTPUT_ASSERTIONS + """
                  const controller = globalThis.MechDocumentController;
                  const original = document.querySelector('.mech-block-output[id]');
                  const address = original?.id;
                  checkOutput(original, address, '11');
                  controller.applyEdit(7, 7, '{output: false}');
                  checkUnavailable(original, address);
                  const body = controller.source().indexOf('11');
                  controller.applyEdit(body, body + 2, '22');
                  checkUnavailable(original, address);
                  const accepted = controller.source();
                  let rejected = false;
                  try { controller.applyEdit(body, body + 2, '['); } catch (_) { rejected = true; }
                  if (!rejected || controller.source() !== accepted) throw new Error('Malformed suppressed edit changed accepted source');
                  checkUnavailable(original, address);
                  controller.applyEdit(0, 0, 'Prose before the fence.\\n\\n');
                  const end = controller.source().length;
                  controller.applyEdit(end, end, '\\nProse after the fence.\\n');
                  checkUnavailable(original, address);
                  const suppressor = controller.source().indexOf('{output: false}');
                  controller.applyEdit(suppressor, suppressor + 15, '');
                  checkSelection(original, address, '22');
                  const fenceStart = controller.source().indexOf('~~~mech');
                  controller.applyEdit(fenceStart + 7, fenceStart + 7, '{output: false}');
                  checkUnavailable(original, address);
                  controller.applyEdit(0, controller.source().length, '');
                  checkUnavailable(original, address);
                  controller.applyEdit(0, 0, '~~~mech\\n33\\n~~~\\n\\n~~~mech\\n33\\n~~~\\n');
                  checkUnavailable(original, address);
                  return {contract:'suppression-edit-restoration', address, originalNodeRetained:document.getElementById(address) === original, malformedEditRejected:rejected, restoredValue:'22', unavailableOutputCleared:true, unavailableSelectionDisabled:true, restoredSelectionAvailable:true, unrelatedDuplicateRejected:true};
                })()""")
                results.append(result)
                browser.write_dom(artifacts / "fence.dom.html")
                browser.navigate(url + "/inline.mec")
                browser.wait_for("document.documentElement?.dataset.mechDocumentStatus === 'ready'", "inline document readiness")
                result = browser.evaluate_json("(async () => {" + OUTPUT_ASSERTIONS + """
                  const controller = globalThis.MechDocumentController;
                  const original = document.querySelector('.mech-inline-mech-code[id]');
                  const address = original?.id;
                  checkOutput(original, address, '11');
                  const start = controller.source().indexOf('{11}');
                  controller.applyEdit(start, start + 4, '');
                  checkUnavailable(original, address);
                  const accepted = controller.source();
                  let rejected = false;
                  try { controller.applyEdit(start, start, '{[}'); } catch (_) { rejected = true; }
                  if (!rejected || controller.source() !== accepted) throw new Error('Malformed inline edit changed accepted source');
                  checkUnavailable(original, address);
                  controller.applyEdit(start, start, '{11}');
                  checkSelection(original, address, '11');
                  controller.applyEdit(start, start + 4, '');
                  checkUnavailable(original, address);
                  controller.applyEdit(start, start, '{33} and {33}');
                  checkUnavailable(original, address);
                  return {contract:'inline-deletion-restoration', address, originalNodeRetained:document.getElementById(address) === original, malformedEditRejected:rejected, unavailableOutputCleared:true, unavailableSelectionDisabled:true, restoredSelectionAvailable:true, unrelatedDuplicateRejected:true};
                })()""")
                results.append(result)
                browser.write_dom(artifacts / "inline.dom.html")
                browser.navigate(url + "/title.mec")
                browser.wait_for("document.documentElement?.dataset.mechDocumentStatus === 'ready'", "title document readiness")
                result = browser.evaluate_json("(async () => {" + OUTPUT_ASSERTIONS + """
                  const controller = globalThis.MechDocumentController;
                  const original = document.querySelector('.hero-kicker .mech-inline-mech-code[id]');
                  const address = original?.id;
                  const body = [...document.querySelectorAll('.mech-inline-mech-code[id]')].find(node => node !== original);
                  const bodyAddress = body?.id;
                  const initial = controller.source();
                  for (const replace of [false, true]) {
                    controller.replaceSource(initial);
                    checkOutput(original, address, '1');
                    for (const value of ['{2}', '{3}', 'literal', '{4}']) {
                      const source = controller.source();
                      const start = source.lastIndexOf('========');
                      const addition = `section: ${value}\\n`;
                      if (replace) controller.replaceSource(source.slice(0, start) + addition + source.slice(start));
                      else controller.applyEdit(start, start, addition);
                      if (value === 'literal') checkUnavailable(original, address);
                      else checkSelection(original, address, value.slice(1, -1));
                      checkOutput(body, bodyAddress, '1');
                      const accepted = controller.source();
                      let rejected = false;
                      try { controller.applyEdit(0, 0, '[\\n'); } catch (_) { rejected = true; }
                      if (!rejected || controller.source() !== accepted) throw new Error('Malformed title edit changed accepted source');
                    }
                    for (const [value, expected] of [['{4}', null], ['literal', '3'], ['{3}', '2'], ['{2}', '1']]) {
                      const source = controller.source();
                      const removal = `section: ${value}\\n`;
                      const start = source.indexOf(removal);
                      if (replace) controller.replaceSource(source.replace(removal, ''));
                      else controller.applyEdit(start, start + removal.length, '');
                      if (expected === null) checkUnavailable(original, address);
                      else checkSelection(original, address, expected);
                      checkOutput(body, bodyAddress, '1');
                    }
                  }
                  return {contract:'title-slot-winner-transfer', address, originalNodeRetained:document.getElementById(address) === original, applyEdit:true, replaceSource:true, restoration:true, bodyOutputPreserved:true};
                })()""")
                results.append(result)
                browser.write_dom(artifacts / "title.dom.html")
                browser.navigate(url + "/documentation.mec")
                browser.wait_for("document.documentElement?.dataset.mechDocumentStatus === 'ready'", "documentation document readiness")
                result = browser.evaluate_json("(async () => {" + OUTPUT_ASSERTIONS + """
                  const nativeFetch = globalThis.fetch;
                  globalThis.fetch = (input, init) => String(input).includes('/browser-lifecycle/main/docs/fence.mec')
                    ? Promise.resolve(new Response('Result {answer + 1}.\\n\\n~~~mech\\nanswer + 2\\n~~~', {status:200}))
                    : nativeFetch(input, init);
                  try {
                    await globalThis.MechDocumentController.invoke(':docs browser-lifecycle/fence');
                    const row = document.querySelector('[data-mech-documentation-topic="browser-lifecycle/fence"]');
                    const inline = row?.querySelector('.mech-inline-mech-code[id]');
                    const fence = row?.querySelector('.mech-block-output[id]');
                    checkSelection(inline, inline?.id, '2');
                    checkSelection(fence, fence?.id, '3');
                    return {contract:'documentation-fence-without-final-newline', inlineValue:'2', fenceValue:'3', liveSelection:true};
                  } finally {
                    globalThis.fetch = nativeFetch;
                  }
                })()""")
                results.append(result)
                browser.write_dom(artifacts / "documentation.dom.html")
                browser.navigate(url + "/console.mec")
                browser.wait_for("document.documentElement?.dataset.mechDocumentStatus === 'ready'", "console document readiness")
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
                print("browser document lifecycle: 5 operation sequences passed")
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
