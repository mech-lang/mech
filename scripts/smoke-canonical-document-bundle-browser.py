#!/usr/bin/env python3
"""Verify shipping document admission, rich prose, live values, selection, and edits."""

from __future__ import annotations

import argparse
import functools
import http.server
import json
import os
import re
import subprocess
import urllib.request
from pathlib import Path
import shutil
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tests.browser.harness import ChromeSession, NavigationContextPending, free_port, wait_for_http


def served_compute_fixture(work: Path) -> dict:
    """Produce the admitted bundle and authority through the shipping server."""
    project = work / "compute-project"
    project.mkdir()
    source = """@compute := compute://worker/kernel{:write(turn), :read(sample/result)}
@clock := timer://clock/tick{:read(tick)}
@compute/turn <- @clock/tick
answer := @compute/sample/result -- **Result** {ans}
answer

calculation @compute
-------------------
~counter := 0f32
counter += 1f32
counter
"""
    config = 'config := {runtime: {resident-durability: "volatile"} hosts: [{name: "clock" provider: "timer" settings: {frequency-hz: 1000 queue-policy: "latest"}} {name: "worker" provider: "compute" settings: {region: "calculation" backend: "cpu"}}] run: {paths: ["document.mec"] grants: [{target: "clock/tick" operations: ["read"] paths: ["tick"]} {target: "worker/kernel" operations: ["read", "write"] paths: ["turn", "sample/result"]}]} serve: {paths: ["document.mec"]}}'
    (project / "document.mec").write_text(source)
    (project / "mech.mcfg").write_text(config)
    binary = Path(os.environ.get("MECH_BIN", ROOT / "target/debug/mech"))
    if not binary.is_absolute():
        binary = ROOT / binary
    port = free_port()
    with (work / "compute-server.log").open("wb") as log:
        server = subprocess.Popen([str(binary), "serve", str(project), "--port", str(port),
                                   "--wasm", str(ROOT / "src/wasm/pkg")],
                                  cwd=ROOT, stdin=subprocess.DEVNULL, stdout=log, stderr=log)
        try:
            base = f"http://127.0.0.1:{port}"
            wait_for_http(base + "/code/document.mec", server, timeout=120)
            def read(route):
                with urllib.request.urlopen(base + route, timeout=15) as response:
                    return response.read().decode()
            encoded = read("/code/document.mec")
            html = read("/document.mec")
            match = re.search(r"window\.__MECH_HOST_CONFIG = (.*?);</script>", html)
            if not match:
                raise RuntimeError("served document has no projected host authority")
            return {"encoded": encoded, "config": config, "source": source,
                    "sources": {"document.mec": source}, "html": html,
                    "authority": json.loads(match.group(1))}
        finally:
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser", help="path to Chrome or Edge")
    parser.add_argument("--fixtures", required=True, help="full-source-runtime emitted bundle directory")
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--served-compute", action="store_true", help="exercise configured compute edits through the shipping server and WasmDocument")
    args = parser.parse_args()
    work_parent = ROOT / "target" if args.served_compute else None
    if work_parent is not None:
        work_parent.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="canonical-document-bundle-", dir=work_parent))
    shutil.copytree(ROOT / "src/wasm/pkg", work / "pkg")
    fixtures = {name: json.loads((Path(args.fixtures) / f"{name}.json").read_text())
                for name in ("plain", "imported", "replacement", "capture", "capture-fenced", "rich", "rich-fenced")}
    if args.served_compute:
        try:
            fixtures["served-compute"] = served_compute_fixture(work)
        except Exception:
            print(f"Configured compute fixture artifacts: {work}", file=sys.stderr)
            raise
    script = r'''import init, {WasmDocument} from './pkg/mech_wasm.js';
try {
  await init();
  const fixtures = FIXTURES;
  const assert = (condition, message) => {if (!condition) throw new Error(message);};
  const revision = doc => doc.runtimeInfo().program_revision;
  const value = doc => doc.renderedSymbol('answer');
  const rows = [];
  for (const [name, fixture] of Object.entries(fixtures).filter(([name]) => ['plain', 'imported'].includes(name))) {
    const doc = name === 'plain'
      ? WasmDocument.fromEncoded(fixture.encoded)
      : WasmDocument.fromEncodedWithSources(fixture.encoded, 'document.mec', fixture.sources);
    const initialRevision = revision(doc);
    assert(typeof initialRevision === 'string' && initialRevision.length > 0, 'initial artifact identity: ' + name);
    const initial = value(doc);
    assert(initial?.inlineHtml === '2', 'initial answer output: ' + name);
    doc.step(1n);
    assert(JSON.stringify(value(doc)) !== JSON.stringify(initial), 'step advances state');
    doc.reset(fixture.encoded);
    assert(revision(doc) === initialRevision, 'reset recompiles the same artifact identity');
    assert(JSON.stringify(value(doc)) === JSON.stringify(initial), 'reset state');
    const changed = '~answer := 0\nanswer += 3\nanswer\n';
    const response = doc.replReplaceSource(changed);
    assert(doc.replSource() === changed, 'source replacement accepted: ' + JSON.stringify(response));
    assert(revision(doc) !== initialRevision, 'edited source must compile fresh artifact');
    doc.step(1n);
    assert(value(doc)?.inlineHtml === '5', 'edited program advances preserved state by three');
    const accepted = {revision: revision(doc), output: value(doc), source: doc.replSource()};
    try {doc.replReplaceSource('~answer := [\n');} catch (_) {}
    assert(revision(doc) === accepted.revision, 'failed edit revision rollback');
    assert(JSON.stringify(value(doc)) === JSON.stringify(accepted.output), 'failed edit output rollback');
    assert(doc.replSource() === accepted.source, 'failed edit source rollback');
    try {doc.reset('invalid-bundle');} catch (_) {}
    assert(revision(doc) === accepted.revision, 'failed reset revision rollback');
    assert(JSON.stringify(value(doc)) === JSON.stringify(accepted.output), 'failed reset output rollback');
    const replacement = fixtures.replacement;
    doc.reset(replacement.encoded);
    assert(revision(doc) === accepted.revision, 'the same replacement source recompiles the same artifact identity');
    assert(value(doc)?.inlineHtml === '3', 'different reset bundle initial state');
    rows.push({name, initialRevision, editedRevision: accepted.revision, initial, edited: accepted.output});
    doc.free();
  }
  for (const name of ['capture', 'capture-fenced']) {
    const doc = WasmDocument.fromEncoded(fixtures[name].encoded);
    const fenced = name === 'capture-fenced';
    const markup = new DOMParser().parseFromString(fixtures[name].html, 'text/html');
    const fenceOutputId = fenced
      ? BigInt(markup.querySelector('.mech-block-output').getAttribute('data-mech-output-address').split(':')[0])
      : null;
    const rendered = () => fenced ? doc.renderedOutput(fenceOutputId) : doc.renderedProgramOutput();
    const original = rendered();
    assert(original?.inlineHtml === '41', 'initial program result: ' + name + ' ' + JSON.stringify(original));
    for (const source of ['answer + 1', 'answer + 2', '99']) {
      const response = doc.replInvoke(source);
      assert(doc.replSource().includes(source), 'console source accepted: ' + JSON.stringify(response));
      const current = rendered();
      assert(current?.inlineHtml === '41', 'original document output survives console append: ' + name);
      const selection = fenced
        ? doc.replSelectOutput(fenceOutputId, true)
        : doc.replSelectRetained(current.selectionToken, true);
      if (!fenced) assert(current.selectionToken === original.selectionToken, 'program selection identity is stable');
      assert(selection.rendered?.inlineHtml === '41', 'original document output remains selectable');
    }
    rows.push({name, documentResult: rendered()});
    doc.free();
  }
  for (const name of ['rich', 'rich-fenced']) {
    const doc = WasmDocument.fromEncoded(fixtures[name].encoded);
    const container = document.createElement('article');
    container.innerHTML = fixtures[name].html;
    document.body.append(container);
    assert(container.querySelector('.mech-comment strong')?.textContent === 'Count', 'rich comment markup');
    assert(container.querySelector('.mech-comment a')?.getAttribute('href') === 'https://mech-lang.org', 'comment link');
    assert(container.textContent.includes('A paragraph with'), 'first Markdown paragraph');
    assert(container.textContent.includes('Another paragraph displays'), 'second Markdown paragraph');
    assert(container.querySelector('em')?.textContent === 'emphasis', 'paragraph emphasis');
    assert(container.querySelectorAll('a').length === 2, 'comment and paragraph links');
    const mounts = [...container.querySelectorAll('.mech-inline-mech-code')];
    assert(mounts.length === 3, 'only evaluated expressions mount; double braces and code stay inert');
    const ids = mounts.map(mount => BigInt(mount.getAttribute('data-mech-output-address').split(':')[0]));
    for (const count of [2, 4, 6]) {
      const expected = [String(count), String(count + 1), String(count)];
      ids.forEach((id, index) => {
        const output = doc.renderedOutput(id);
        assert(output?.inlineHtml === expected[index], 'live inline value: ' + name + ' ' + index);
        mounts[index].innerHTML = output.inlineHtml;
        const selected = doc.replSelectOutput(id, true);
        assert(selected.rendered?.inlineHtml === expected[index], 'inline selection: ' + name + ' ' + index);
        assert(selected.identity, 'inline selection has a retained identity');
      });
      const programResult = doc.renderedProgramOutput();
      assert(programResult?.inlineHtml === String(count), 'comments do not replace program result: ' + name + ' ' + count + ' ' + JSON.stringify(programResult));
      if (count !== 6) doc.step(1n);
    }
    rows.push({name, inlineValues: mounts.map(mount => mount.textContent), links: container.querySelectorAll('a').length});
    container.remove();
    doc.free();
  }
  if (fixtures['served-compute']) {
    const fixture = fixtures['served-compute'];
    window.__MECH_HOST_CONFIG = fixture.authority;
    const doc = WasmDocument.fromServedEncoded(fixture.encoded, 'document.mec', fixture.config, fixture.sources);
    assert(doc.computeBackend() === 'cpu-scalar', 'configured document selects CPU compute');
    const markup = new DOMParser().parseFromString(fixture.html, 'text/html');
    const mounts = [...markup.querySelectorAll('.mech-inline-mech-code')];
    assert(mounts.length === 1, 'configured compute document has one inline result');
    const outputId = BigInt(mounts[0].getAttribute('data-mech-output-address').split(':')[0]);
    async function dispatchTimerTurn() {
      const deadline = performance.now() + 5000;
      let frame;
      do {
        await new Promise(resolve => setTimeout(resolve, 10));
        frame = doc.frame(1);
      } while (frame.processed === 0 && performance.now() < deadline);
      assert(frame.processed === 1 && frame.pending === 1, 'timer dispatch queues one sample: ' + JSON.stringify(frame));
    }
    doc.start();
    await dispatchTimerTurn();
    assert(doc.renderedSymbol('answer')?.inlineHtml === '0', 'dispatch turn sees the initial sample');
    const sampleFrame = doc.frame(1);
    assert(sampleFrame.processed === 1, 'sample is published on the next host-input turn: ' + JSON.stringify(sampleFrame));
    const initial = doc.renderedSymbol('answer');
    assert(initial?.inlineHtml === '1', 'initial compute result: ' + JSON.stringify(initial));
    assert(doc.renderedOutput(outputId)?.inlineHtml === '1', 'initial inline compute result');
    const originalGeneration = doc.computeGeneration();
    const originalManifest = doc.computeManifest();
    const changed = fixture.source.replace('counter += 1f32', 'counter += 3f32');
    const response = doc.replReplaceSource(changed);
    assert(doc.replSource() === changed, 'compute source accepted: ' + JSON.stringify(response));
    assert(doc.computeGeneration() !== originalGeneration, 'compute generation advances after edit');
    assert(doc.computeManifest().physicalRevision !== originalManifest.physicalRevision, 'edited compute body changes kernel');
    await dispatchTimerTurn();
    assert(doc.frame(1).processed === 1, "edited sample publishes on its host-input turn");
    const edited = doc.renderedSymbol('answer');
    assert(edited?.inlineHtml === '3', 'edited compute result: ' + JSON.stringify(edited));
    assert(doc.renderedOutput(outputId)?.inlineHtml === '3', 'inline identity survives compute edit');
    assert(doc.replSelectOutput(outputId, true).rendered?.inlineHtml === '3', 'edited inline result remains selectable');
    const accepted = {generation: doc.computeGeneration(), manifest: doc.computeManifest().physicalRevision, value: doc.renderedSymbol('answer')};
    try {doc.replReplaceSource(changed.replace('counter += 3f32', 'counter += ['));} catch (_) {}
    assert(doc.replSource() === changed, 'failed compute edit rolls back source');
    assert(doc.computeGeneration() === accepted.generation, 'failed compute edit rolls back generation');
    assert(doc.computeManifest().physicalRevision === accepted.manifest, 'failed compute edit keeps kernel');
    assert(JSON.stringify(doc.renderedSymbol('answer')) === JSON.stringify(accepted.value), 'failed compute edit preserves output');
    rows.push({name: 'served-compute', initial, edited, generation: accepted.generation});
    doc.stop();
    doc.free();
    delete window.__MECH_HOST_CONFIG;
  }
  const imported = fixtures.imported;
  const updated = WasmDocument.fromEncodedWithSources(imported.encoded, 'document.mec',
    {...imported.sources, 'dep.mec': 'value := 999\n<+ value\n'});
  assert(value(updated)?.inlineHtml === '999', 'updated dependency is recompiled from the source map');
  updated.free();
  let rejected = false;
  try {
    const stale = WasmDocument.fromEncodedWithSources(imported.encoded, 'document.mec',
      {...imported.sources, 'document.mec': 'answer := 999\nanswer\n'});
    stale.free();
  } catch (_) {rejected = true;}
  assert(rejected, 'payload and root source map must have one exact authority');
  window.outcome = {ok: true, results: rows};
} catch (error) {
  window.outcome = {ok: false, error: String(error), stack: error?.stack};
}
'''.replace('FIXTURES', json.dumps(fixtures).replace('<', chr(92) + 'u003c'))
    (work / "index.html").write_text('<script type="module">' + script + '</script>')

    class Handler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, *args):
            pass

    server = http.server.ThreadingHTTPServer(
        ("127.0.0.1", 0), functools.partial(Handler, directory=str(work)),
    )
    threading.Thread(target=server.serve_forever, daemon=True).start()
    session = None
    passed = False
    try:
        session = ChromeSession(
            args.browser, work / "profile", work / "chrome.log", flags=[],
        ).start()
        session.navigate(f"http://127.0.0.1:{server.server_port}/")
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            try:
                outcome = session.evaluate("window.outcome || null")
            except NavigationContextPending:
                outcome = None
            if outcome:
                if not outcome["ok"]:
                    raise RuntimeError(json.dumps(outcome))
                print("CANONICAL_DOCUMENT_BUNDLE", json.dumps(outcome["results"]))
                passed = True
                break
            time.sleep(0.1)
        else:
            raise RuntimeError("canonical document bundle browser timeout")
    finally:
        if session:
            session.close()
        server.shutdown()
        server.server_close()
        if passed:
            shutil.rmtree(work, ignore_errors=True)
        else:
            print(f"Canonical document bundle browser artifacts: {work}", file=sys.stderr)


if __name__ == "__main__":
    main()
