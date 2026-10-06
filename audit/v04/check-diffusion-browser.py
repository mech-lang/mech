#!/usr/bin/env python3
"""Run additional heat-diffusion workloads on real browser CPU and GPU paths."""
import json
from pathlib import Path
import sys
import tempfile
ROOT=Path(__file__).resolve().parents[2]
sys.path[:0]=[str(ROOT),str(ROOT/'scripts')]
from tests.browser.harness import ChromeSession
from browser_webgpu_flags import chrome_webgpu_test_flags
output=ROOT/'audit/v04/evidence'
with tempfile.TemporaryDirectory(prefix='mech-diffusion-') as temporary:
    with ChromeSession(None,Path(temporary)/'profile',output/'diffusion-chrome.log',flags=chrome_webgpu_test_flags(software_adapter=False),window_size=(1440,1200)) as browser:
        browser.navigate('http://127.0.0.1:8764/audit/v04/site/diffusion.html')
        browser.wait_for('window.diffusionReady || window.diffusionError','diffusion initialization',timeout=120)
        result=browser.evaluate('''(async()=>{
          if(window.diffusionError)throw Error(window.diffusionError);
          const records=[];
          for(const width of [4,8])for(const backend of ['cpu','gpu'])records.push(await window.runDiffusion({backend,width}));
          for(const backend of ['cpu','gpu'])records.push(await window.runDiffusion({backend,width:4,alpha:0.0625,turns:6,schedule:[false,true,true,false,true,false]}));
          const small=await window.runDiffusion({backend:'cpu',width:4,turns:1});
          const literal=[60,10,0,0,10,0,0,0,0,0,0,0,0,0,0,0];
          if(JSON.stringify(small.frames[0].actual)!==JSON.stringify(literal))throw Error('Independent first-step literal differs');
          return {id:'E-DIFFUSION-BROWSER',claim:'New heat diffusion workload executes CPU and hardware GPU matrix kernels',evidence_level:'browser-execution-and-independent-stencil-comparison',outcome:records.every(r=>r.outcome==='passed')?'passed':'blocked',records,literal_first_step:literal};
        })()''',timeout=300)
        (output/'diffusion-browser.json').write_text(json.dumps(result,indent=2)+'\n')
        browser.evaluate("document.getElementById('compare').click()")
        browser.wait_for('Array.isArray(window.diffusionLast)','diffusion controls complete',timeout=180)
        browser.capture_screenshot(output/'diffusion-browser.png')
        print(json.dumps({'outcome':result['outcome'],'runs':len(result['records']),'max_error':max((frame['max_absolute_error'] for r in result['records'] for frame in r.get('frames', [])), default=None)}))
