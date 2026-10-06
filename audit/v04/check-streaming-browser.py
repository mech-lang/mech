#!/usr/bin/env python3
"""Run syntax adapter checks through the maintained Chrome harness."""
import functools
import http.server
import json
from pathlib import Path
import sys
import tempfile
import threading
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from tests.browser.harness import ChromeSession, free_port

site = ROOT / 'audit/v04/site'
site.joinpath('streaming-check.html').write_text('''<!doctype html><meta charset="utf-8"><title>Mech syntax checks</title>
<pre id="result">Running</pre><script type="module">
import init, * as api from './pkg/mech_wasm.js';
import {runStreamingChecks, json} from './streaming-checks.js';
try {
 const bytes = await (await fetch('./pkg/mech_wasm_bg.wasm')).arrayBuffer();
 await init({module_or_path: bytes});
 window.result = await runStreamingChecks(api);
 window.result.loadedWasmSha256 = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))).map(x => x.toString(16).padStart(2,'0')).join('');
 window.result.artifact = await (await fetch('./artifact.json')).json();
 if(window.result.loadedWasmSha256 !== window.result.artifact.artifacts['pkg/mech_wasm_bg.wasm'].sha256) throw Error('artifact hash mismatch');
}
catch(error) { window.result = {status:'failed', error:String(error), stack:error.stack}; }
document.querySelector('#result').textContent = json(window.result);
window.streamingTestDone = true;
</script>''')
port = free_port()
server = http.server.ThreadingHTTPServer(('127.0.0.1', port), functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(site)))
threading.Thread(target=server.serve_forever, daemon=True).start()
evidence = ROOT / 'audit/v04/evidence'
try:
    with tempfile.TemporaryDirectory(prefix='mech-streaming-browser-') as tmp:
        with ChromeSession(None, Path(tmp) / 'profile', evidence / 'streaming-browser-chrome.log', flags=['--headless=new']) as browser:
            browser.navigate(f'http://127.0.0.1:{port}/streaming-check.html')
            browser.wait_for('window.streamingTestDone', 'syntax browser tests and artifact verification', timeout=240)
            result = browser.evaluate('JSON.parse(JSON.stringify(window.result, (_, x) => typeof x === "bigint" ? x.toString() : x))')
            evidence.joinpath('streaming-browser-result.json').write_text(json.dumps(result, indent=2) + '\n')
            print(json.dumps(result, indent=2))
            if result['status'] != 'passed': raise SystemExit(1)
            browser.navigate(f'http://127.0.0.1:{port}/streaming.html')
            browser.wait_for('window.streamingReady', 'interactive syntax page', timeout=120)
            browser.evaluate('''(() => {
                const $ = id => document.getElementById(id);
                $('source').value = '-- é👩‍💻\\nx := 1\\ny := 2\\n';
                $('chunk').value = '1'; $('allowance').value = '127';
                $('restart').click(); $('resume').click();
            })()''')
            browser.wait_for("document.getElementById('stream-status').textContent.includes('Finish explicitly')", 'transport completion', timeout=60)
            ui = browser.evaluate('''(() => {
                const $ = id => document.getElementById(id);
                const assert = (condition, message) => {if(!condition) throw Error(message);};
                assert($('accepted').value === $('source').value, 'UI byte transport preserves Unicode');
                $('preview').click(); assert($('snapshot-status').textContent.includes('provisional'), 'preview labelled');
                $('tree').querySelector('tr').click();
                assert($('accepted').selectionEnd > $('accepted').selectionStart, 'tree selection highlights source');
                $('allowance').value = '100000'; $('finish').click(); $('materialize').click();
                assert($('snapshot-status').textContent.includes('Finalized'), 'final tree identity');
                $('editor-load').click(); assert($('identity').textContent.includes('Displayed editor identity'), 'editor identity owner'); const position = $('editor-source').value.indexOf('1');
                $('editor-source').setSelectionRange(position, position + 1); $('replacement').value = '3'; $('edit-replace').click();
                assert($('editor-source').value.includes('x := 3'), 'UI range edit');
                assert($('editor-status').textContent.includes('reconciliation work'), 'separate edit work');
                const editEvidence = $('edit-result').textContent;
                $('editor-source').setSelectionRange(0, $('editor-source').value.indexOf('\\n') + 1); $('edit-delete').click();
                assert($('editor-source').value.startsWith('x := 3'), 'UI Unicode prefix deletion');
                return {status:'passed', sourceSelection:true, unicodeTransport:true, editing:true, editEvidence};
            })()''')
            browser.capture_screenshot(evidence / 'streaming-browser.png')
            browser.evaluate('''(() => {
                const $ = id => document.getElementById(id), assert = (c,m) => {if(!c) throw Error(m);};
                $('resume').click(); $('restart').click();
                assert($('editor-source').value === '' && $('tree').children.length === 0, 'restart clears old editor/tree');
                $('cancel').click(); assert($('identity').textContent.includes('Cancelled'), 'cancel identity');
                assert($('tree').children.length === 0, 'cancel has no stale snapshot');
            })()''')
            evidence.joinpath('streaming-ui-result.json').write_text(json.dumps(ui, indent=2) + '\n')
finally:
    server.shutdown()
