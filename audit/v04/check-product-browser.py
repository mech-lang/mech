#!/usr/bin/env python3
"""Qualify real Mech CPU/GPU and application pages through the shared harness."""
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(ROOT), str(ROOT / "scripts")]
from tests.browser.harness import ChromeSession
from browser_webgpu_flags import chrome_webgpu_test_flags

evidence = ROOT / "audit/v04/evidence"
with tempfile.TemporaryDirectory(prefix="mech-product-browser-") as temp:
    with ChromeSession(None, Path(temp) / "profile", evidence / "product-browser-chrome.log", flags=chrome_webgpu_test_flags(software_adapter=False), window_size=(1440, 1100)) as browser:
        browser.navigate("http://127.0.0.1:8764/audit/v04/site/compute.html")
        browser.wait_for("window.computeReady === true", "WASM compute page", timeout=120)
        compute = browser.evaluate("""(async()=>{
          const module=await import('./compute.js');const records=[];
          for(const columns of [3,1024])for(const backend of ['cpu','gpu'])records.push(await module.runCompute({backend,columns}));
          const placement=module.hardPlacementProbe();
          document.getElementById('results').textContent=JSON.stringify({records,placement},null,2);
          if(placement.outcome!=='rejected'||/parse|syntax/i.test(placement.diagnostic))throw Error('Hard placement did not establish capability rejection: '+JSON.stringify(placement));
          return {outcome:records.every(r=>r.outcome==='passed')?'passed':'blocked',records,placement};
        })()""", timeout=240)
        (evidence / "compute-browser-result.json").write_text(json.dumps(compute, indent=2) + "\n")
        browser.evaluate("document.getElementById('compare').click()")
        browser.wait_for("Array.isArray(window.lastComputeResults) && window.lastComputeResults.length === 2", "compute controls complete both backends", timeout=120)
        controls = browser.evaluate("window.lastComputeResults.map(r=>({requested:r.requested,outcome:r.outcome,completed:r.completed}))")
        if any(r['outcome'] != 'passed' for r in controls): raise RuntimeError(controls)
        compute['controls'] = controls
        (evidence / "compute-browser-result.json").write_text(json.dumps(compute, indent=2) + "\n")
        browser.capture_screenshot(evidence / "compute-browser.png")
        print(json.dumps({"compute": compute["outcome"], "runs": [{k: r.get(k) for k in ("requested", "selected", "completed", "outcome", "columns", "adapter")} for r in compute["records"]]}), flush=True)
        browser.navigate("http://127.0.0.1:8764/audit/v04/site/application.html")
        browser.wait_for("window.applicationReady === true", "WASM application page", timeout=120)
        application = browser.evaluate("window.applicationChecks()", timeout=240)
        maintained = browser.evaluate("window.runApplication({which:'ten',turns:2})", timeout=240)
        application["maintained_application"] = maintained
        (evidence / "application-browser-result.json").write_text(json.dumps(application, indent=2) + "\n")
        browser.evaluate("document.getElementById('start').click()")
        browser.wait_for("document.getElementById('runtime').textContent.includes('resident_accepted_turns')", "application start control", timeout=30)
        browser.evaluate("document.getElementById('stop').click()")
        browser.wait_for("document.getElementById('status').textContent === 'Stopped and disposed.' && document.getElementById('nbody').childElementCount === 0", "application stop control")
        browser.evaluate("document.getElementById('two-turns').click()")
        browser.wait_for("document.getElementById('status').textContent === 'Completed and disposed.'", "application bounded run control", timeout=90)
        application['controls'] = {'start_stop_and_bounded_run': 'passed'}
        (evidence / "application-browser-result.json").write_text(json.dumps(application, indent=2) + "\n")
        browser.capture_screenshot(evidence / "application-browser.png")
        print(json.dumps({"application": application["outcome"], "cases": len(application["cases"]), "maintained_application": maintained["outcome"]}))
