#!/usr/bin/env python3
"""Exercise actual resident inventory updates through the browser interface."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests"))
from browser.harness.chrome import ChromeSession

parser = argparse.ArgumentParser()
parser.add_argument("--url", default="http://127.0.0.1:8764/audit/v04/site/inventory.html")
args = parser.parse_args()
evidence = ROOT / "audit/v04/evidence"
with tempfile.TemporaryDirectory(prefix="mech-inventory-browser-") as profile:
    with ChromeSession(None, profile, evidence / "inventory-chrome.log", window_size=(1440, 1100)) as browser:
        browser.navigate(args.url)
        browser.wait_for("window.mechInventoryReady || window.mechInventoryError", "inventory adapter initialization", timeout=120)
        result = browser.evaluate("window.mechInventoryError ? {status:'blocked',error:window.mechInventoryError} : window.mechInventoryChecks()", timeout=60)
        if result["status"] == "passed":
            native = json.loads((evidence / "inventory/native-result.json").read_text())
            native_source = (evidence / "inventory/inventory.mec").read_bytes()
            result["native_source_sha256"] = hashlib.sha256(native_source).hexdigest()
            result["native_source_equal"] = result["native_source_sha256"] == result["metadata"]["source_sha256"]
            result["native_output_equal"] = [x["check"]["actual_stock"] for x in result["records"]] == [str(x["accepted_stock"]) for x in native["records"]]
            ui = browser.evaluate("""(() => {
              const $=id=>document.getElementById(id), assert=(c,m)=>{if(!c)throw Error(m);};
              const submit=(a,d)=>{$('arrivals').value=String(a);$('demand').value=String(d);$('submit').click();};
              $('sequence').click();
              assert($('stock').textContent==='12','recorded sequence ends at 12');
              assert(window.mechInventoryHistory().length===9,'nine recorded attempts');
              assert($('rejections').textContent==='4','four guarded rejections');
              submit(8,3);assert($('stock').textContent==='17','changed input accepted');
              submit(0,18);assert($('stock').textContent==='17','overdraw leaves stock');
              const rejected=window.mechInventoryHistory().at(-1);assert(rejected.receipt.accepted_state_unchanged,'reject preserves epoch/hash/output');
              submit(2,4);assert($('stock').textContent==='15','same instance recovers');
              assert($('residual').textContent==='0','conservation');
              const length=window.mechInventoryHistory().length;submit('2.5',0);assert(window.mechInventoryHistory().length===length,'decimal input blocked in form');
              document.querySelector('#history tr').click();assert($('receipt').parentElement.open,'receipt selection');
              const history=window.mechInventoryHistory();
              return {status:'passed',history,stock:$('stock').textContent,residual:$('residual').textContent,selected_receipt:JSON.parse($('receipt').textContent),scope:$('session-scope').textContent};
            })()""")
            result["ui"] = ui
            result["status"] = "passed" if result["native_source_equal"] and result["native_output_equal"] else "failed"
            browser.evaluate("window.scrollTo(0,300)")
            screenshot = browser.call("Page.captureScreenshot", {"format":"png","captureBeyondViewport":False})
            (evidence / "inventory-browser.png").write_bytes(base64.b64decode(screenshot["data"]))
            browser.evaluate("document.getElementById('history').scrollIntoView({block:'center'})")
            screenshot = browser.call("Page.captureScreenshot", {"format":"png","captureBeyondViewport":False})
            (evidence / "inventory-receipts.png").write_bytes(base64.b64decode(screenshot["data"]))
            browser.evaluate("document.getElementById('restart').click(); if(document.getElementById('stock').textContent!=='100'||window.mechInventoryHistory().length) throw Error('restart failed');")
            result["ui"]["restart"] = "passed"
        result.update({"id":"E-INVENTORY-BROWSER","claim":"Retained inventory with changing external inputs, source invariants, transactional rejection and recovery", "contract":"Canonical source + bytecode-v1 + ReactiveInstance prepare/publish + integrity constraints", "source_commit":"c4777b7015fe8ff47fdfa48d18606c49aace7d97", "source_state":"baseline plus source modifications hashed in the attached artifact identity", "evidence_level":"browser-execution","outcome":result["status"],"artifact_identity":result.get("metadata",{}),"command":"python3 audit/v04/check_inventory_browser.py"})
        (evidence / "inventory-browser.json").write_text(json.dumps(result,indent=2)+"\n")
        print(json.dumps({"status":result["status"],"source_equal":result.get("native_source_equal"),"native_output_equal":result.get("native_output_equal"),"sequence_cases":len(result.get("records",[])),"error":result.get("error")}))
        if result["status"] != "passed": raise SystemExit(1)
