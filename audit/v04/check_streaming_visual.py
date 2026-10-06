#!/usr/bin/env python3
"""Verify diagnostic selection and capture the final syntax interface."""
import base64
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests"))
from browser.harness.chrome import ChromeSession
evidence = ROOT / "audit/v04/evidence"
with tempfile.TemporaryDirectory(prefix="mech-syntax-visual-") as profile:
    with ChromeSession(None, profile, evidence / "streaming-visual-chrome.log", window_size=(1400, 1000)) as browser:
        browser.navigate('http://127.0.0.1:8764/audit/v04/site/streaming.html')
        browser.wait_for('window.streamingReady', 'syntax adapter initialization', timeout=90)
        browser.evaluate("""(() => {
          const $=id=>document.getElementById(id);
          $('source').value='x := 1\\ny := 2\\n';$('chunk').value='100';$('allowance').value='1000000';
          $('restart').click();$('step').click();$('finish').click();$('materialize').click();
        })()""")
        def screenshot(name):
            record=browser.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False})
            (evidence/name).write_bytes(base64.b64decode(record['data']))
        screenshot('streaming-desktop.png')
        browser.evaluate("document.getElementById('snapshot-status').parentElement.scrollIntoView({block:'start'})")
        screenshot('streaming-structures.png')
        browser.evaluate("""(() => {
          const $=id=>document.getElementById(id);$('editor-load').click();
          const at=$('editor-source').value.indexOf('1');$('editor-source').setSelectionRange(at,at+1);
          $('replacement').value='3';$('edit-replace').click();$('editor-source').parentElement.scrollIntoView({block:'start'});
        })()""")
        screenshot('streaming-editor.png')
        diagnostic=browser.evaluate("""(() => {
          const $=id=>document.getElementById(id);$('source').value='x := [1, +, 2]\\ny := 3\\n';
          $('restart').click();$('step').click();$('finish').click();$('materialize').click();
          const button=$('diagnostics').querySelector('button');if(!button)throw Error('expected malformed-source diagnostic');button.click();
          const area=$('accepted'),selected=area.value.slice(area.selectionStart,area.selectionEnd);
          if(!selected.includes('+'))throw Error('diagnostic selection must include malformed operator');
          $('snapshot-status').parentElement.scrollIntoView({block:'start'});
          return {status:'passed',selection:[area.selectionStart,area.selectionEnd],selected,diagnostic:JSON.parse($('detail').textContent),artifact:window.streamingArtifact};
        })()""")
        screenshot('streaming-diagnostics.png')
        (evidence/'streaming-diagnostic-ui.json').write_text(json.dumps(diagnostic,indent=2)+'\n')
        print(json.dumps({'status':diagnostic['status'],'selection':diagnostic['selection'],'artifact':diagnostic['artifact']['loadedHash']}))
