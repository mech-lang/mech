#!/usr/bin/env python3
"""Check census-backed atlas drill-down in the maintained Chrome harness."""
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from tests.browser.harness import ChromeSession

evidence = ROOT / "audit/v04/evidence"
with tempfile.TemporaryDirectory(prefix="mech-atlas-") as temp:
    with ChromeSession(None, Path(temp) / "profile", evidence / "atlas-chrome.log", window_size=(1440, 1050)) as browser:
        browser.navigate("http://127.0.0.1:8764/audit/v04/site/")
        browser.wait_for("document.querySelectorAll('#files tr').length === 1976 && Boolean(document.querySelector('#extraction-bars .bar-row')) && document.querySelectorAll('#evidence-view tbody tr').length > 0", "complete census and extraction atlas")
        result = browser.evaluate("""(() => {
          const q=s=>document.querySelector(s);
          const before=document.querySelectorAll('#files tr').length;
          const syntax=[...document.querySelectorAll('#subsystems button')].find(x=>x.textContent.includes('src/syntax'));
          syntax.click();
          const scoped=[...document.querySelectorAll('#files tr')];
          if (!scoped.length || !scoped.every(x=>x.firstElementChild.textContent.startsWith('src/syntax/'))) throw Error('subsystem drill-down mixed unrelated files');
          const filtered=scoped.length;
          syntax.click();
          q('#measure').value='bytes';q('#measure').dispatchEvent(new Event('change'));
          q('#dependency-graph .dep-edge').dispatchEvent(new Event('click'));
          if(!q('#dependency-detail').textContent.includes('records'))throw Error('dependency edge failed to expose records');
          return {status:'passed',before,syntax_files:filtered,capabilities:document.querySelectorAll('#capability-view article').length,findings:document.querySelectorAll('#finding-view article').length,extraction_bars:document.querySelectorAll('#extraction-bars button').length,evidence_records:document.querySelectorAll('#evidence-view tbody tr').length,units:q('#measure').value};
        })()""")
        browser.capture_screenshot(evidence / "atlas.png")
        (evidence / "atlas-browser-result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
