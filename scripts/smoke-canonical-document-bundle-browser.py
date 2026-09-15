#!/usr/bin/env python3
"""Verify shipping document admission, rich prose, live values, selection, and edits."""

from __future__ import annotations

import argparse
import functools
import http.server
import json
from pathlib import Path
import shutil
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tests.browser.harness import ChromeSession, NavigationContextPending


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser", help="path to Chrome or Edge")
    parser.add_argument("--fixtures", required=True, help="full-source-runtime emitted bundle directory")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()
    work = Path(tempfile.mkdtemp(prefix="canonical-document-bundle-"))
    shutil.copytree(ROOT / "src/wasm/pkg", work / "pkg")
    fixtures = {name: json.loads((Path(args.fixtures) / f"{name}.json").read_text())
                for name in ("plain", "imported", "replacement", "capture", "capture-fenced", "rich", "rich-fenced")}
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
    assert(revision(doc) === fixture.revision, 'initial artifact identity: ' + name);
    const initial = value(doc);
    assert(initial?.inlineHtml === '2', 'initial answer output: ' + name);
    doc.step(1n);
    assert(JSON.stringify(value(doc)) !== JSON.stringify(initial), 'step advances state');
    doc.reset(fixture.encoded);
    assert(revision(doc) === fixture.revision, 'reset artifact identity');
    assert(JSON.stringify(value(doc)) === JSON.stringify(initial), 'reset state');
    const changed = '~answer := 0\nanswer += 3\nanswer\n';
    const response = doc.replReplaceSource(changed);
    assert(doc.replSource() === changed, 'source replacement accepted: ' + JSON.stringify(response));
    assert(revision(doc) !== fixture.revision, 'edited source must compile fresh artifact');
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
    assert(revision(doc) === replacement.revision, 'different reset bundle artifact identity');
    assert(value(doc)?.inlineHtml === '3', 'different reset bundle initial state');
    rows.push({name, initialRevision: fixture.revision, editedRevision: accepted.revision, initial, edited: accepted.output});
    doc.free();
  }
  for (const name of ['capture', 'capture-fenced']) {
    const doc = WasmDocument.fromEncoded(fixtures[name].encoded);
    const original = doc.renderedProgramOutput();
    assert(original?.inlineHtml === '41', 'initial program result: ' + name);
    for (const source of ['answer + 1', 'answer + 2', '99']) {
      const response = doc.replInvoke(source);
      assert(doc.replSource().includes(source), 'console source accepted: ' + JSON.stringify(response));
      const current = doc.renderedProgramOutput();
      assert(current?.inlineHtml === '41', 'original program result survives console append: ' + name);
      assert(current.selectionToken === original.selectionToken, 'program selection identity is stable');
      const selection = doc.replSelectRetained(current.selectionToken, true);
      assert(selection.rendered?.inlineHtml === '41', 'original program result remains selectable');
    }
    rows.push({name, programResult: doc.renderedProgramOutput()});
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
      assert(doc.renderedProgramOutput()?.inlineHtml === String(count), 'comments do not replace program result');
      if (count !== 6) doc.step(1n);
    }
    rows.push({name, inlineValues: mounts.map(mount => mount.textContent), links: container.querySelectorAll('a').length});
    container.remove();
    doc.free();
  }
  const imported = fixtures.imported;
  let rejected = false;
  try {
    const stale = WasmDocument.fromEncodedWithSources(imported.encoded, 'document.mec',
      {...imported.sources, 'dep.mec': 'value := 999\n<+ value\n'});
    stale.free();
  } catch (_) {rejected = true;}
  assert(rejected, 'stale transitive dependency must be rejected');
  window.outcome = {ok: true, results: rows};
} catch (error) {
  window.outcome = {ok: false, error: String(error), stack: error?.stack};
}
'''.replace('FIXTURES', json.dumps(fixtures))
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
