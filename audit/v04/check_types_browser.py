#!/usr/bin/env python3
"""Run the types demo in the repository's existing real Chrome harness."""
import argparse
import base64
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests"))
from browser.harness.chrome import ChromeSession

parser = argparse.ArgumentParser()
parser.add_argument("--url", default="http://127.0.0.1:8764/audit/v04/site/types.html")
args = parser.parse_args()
output = ROOT / "audit/v04/evidence"
output.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix="mech-type-browser-") as profile:
    with ChromeSession(None, profile, output / "types-chrome.log", window_size=(1440, 1100)) as browser:
        browser.navigate(args.url)
        browser.wait_for("window.mechTypesReady || window.mechTypesError", "type adapter initialization", timeout=60)
        result = browser.evaluate("window.mechTypesError ? {status:'blocked',error:window.mechTypesError} : window.mechTypeChecks()", timeout=60)
        if result.get("status") != "blocked":
            selection = browser.evaluate("""(() => {
              const source='café := 1\\n10⟨u8:1..10⟩\\n';
              const textarea=document.getElementById('source');textarea.value=source;
              document.getElementById('run').click();
              document.querySelector('#diagnostics button').click();
              const selected=source.slice(textarea.selectionStart,textarea.selectionEnd);
              const selection=[textarea.selectionStart,textarea.selectionEnd];
              textarea.dispatchEvent(new Event('input',{bubbles:true}));
              const stale_cleared=document.querySelectorAll('#diagnostics button').length===0&&document.getElementById('result').textContent==='';
              document.getElementById('run').click();document.querySelector('#diagnostics button').click();
              return {source,selected,selection,stale_cleared,passed:selected==='10'&&stale_cleared};
            })()""")
            result["diagnostic_selection"] = selection
            if not selection["passed"]: result["status"] = "failed"
        result.update({"id":"E-TYPES-BROWSER","claim":'Type admission, exact values, diagnostic selection and same-instance publication',"contract":'docs/design/grammar-audit/fixed-integer-intervals.md',"source_commit":"c4777b7015fe8ff47fdfa48d18606c49aace7d97","source_state":"baseline plus source modifications hashed in site/artifact.json","evidence_level":"browser-execution","outcome":result["status"],"artifact_identity":result.get("metadata",{}),"command":"python3 audit/v04/check_types_browser.py"})
        (output / "types-browser.json").write_text(json.dumps(result, indent=2) + "\n")
        browser.evaluate("document.getElementById('sequence').click()")
        screenshot = browser.call("Page.captureScreenshot", {"format":"png","captureBeyondViewport":True})
        (output / "types-browser.png").write_bytes(base64.b64decode(screenshot["data"]))
        print(json.dumps({"status":result["status"],"cases":[{"id":x["id"],"passed":x["passed"]} for x in result.get("results",[])],"publication":result.get("publication",{}).get("passed")}))
        if result["status"] != "passed": raise SystemExit(1)
