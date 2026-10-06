#!/usr/bin/env python3
"""Execute the recurrence browser checks through the repository Chrome harness."""
import base64,json,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tests'))
from browser.harness.chrome import ChromeSession
output=ROOT/'audit/v04/evidence'
with tempfile.TemporaryDirectory(prefix='mech-trust-browser-') as profile:
 with ChromeSession(None,profile,output/'trust-chrome.log',window_size=(1440,1100)) as browser:
  browser.navigate('http://127.0.0.1:8764/audit/v04/site/trust.html')
  browser.wait_for('window.mechTrustReady || window.mechTrustError','trust browser initialization',timeout=60)
  result=browser.evaluate("window.mechTrustError ? {status:'blocked',error:window.mechTrustError} : window.mechTrustChecks()",timeout=60)
  result['native_record_status']=browser.evaluate("window.mechTrustNative?.status ?? 'pending'")
  result.update({"id":"E-TRUST-BROWSER","claim":'Canonical-source recurrence, source perturbation and rejected syntax/type/integrity candidates',"contract":'examples/r-stack-proof/README.md',"source_commit":"c4777b7015fe8ff47fdfa48d18606c49aace7d97","source_state":"baseline plus source modifications hashed in site/artifact.json","evidence_level":"browser-execution","outcome":result["status"],"artifact_identity":result.get("metadata",{}),"command":"python3 audit/v04/check_trust_browser.py"})
  (output/'trust-browser.json').write_text(json.dumps(result,indent=2)+'\n')
  shot=browser.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':True})
  (output/'trust-browser.png').write_bytes(base64.b64decode(shot['data']))
  print(json.dumps({'status':result['status'],'native':result['native_record_status'],'cases':[{'id':x['id'],'passed':x['passed']} for x in result.get('results',[])]}))
  if result['status']!='passed':raise SystemExit(1)
