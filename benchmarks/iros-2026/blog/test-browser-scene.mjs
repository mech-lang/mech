// Run against an isolated Chrome debugging session and a locally served build.
// This drives the real page; no scene or numerical runtime is mocked.
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
const port=process.env.IROS_CDP_PORT||9227;
const url=process.env.IROS_URL||'http://127.0.0.1:8768/index.html';
const output=process.env.IROS_REPORT||'/private/tmp/iros-scene-browser-report.json';
const target=await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'}).then(r=>r.json());
const socket=new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve,reject)=>{socket.onopen=resolve;socket.onerror=reject;});
let id=0;const pending=new Map(),exceptions=[];
socket.onmessage=({data})=>{
  const r=JSON.parse(data);
  if(r.id){const p=pending.get(r.id);pending.delete(r.id);r.error?p.reject(r.error):p.resolve(r.result);}
  if(r.method==='Runtime.exceptionThrown')exceptions.push(r.params.exceptionDetails);
};
const send=(method,params={})=>new Promise((resolve,reject)=>{const n=++id;pending.set(n,{resolve,reject});socket.send(JSON.stringify({id:n,method,params}));});
const evaluate=async expression=>{
  const r=await send('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});
  if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));
  return r.result.value;
};
async function until(expression,description,limit=60000,expectedError=''){
  const start=Date.now();
  while(Date.now()-start<limit){
    if(await evaluate(expression))return;
    const error=await evaluate('document.getElementById("runtime-error")?.textContent||""');
    if(error&&!error.includes(expectedError||'\u0000'))throw new Error(description+': '+error);
    await new Promise(r=>setTimeout(r,150));
  }
  throw new Error('Timed out: '+description);
}
const change=async(id,value)=>evaluate(`(()=>{const e=document.getElementById(${JSON.stringify(id)});e.value=${JSON.stringify(String(value))};e.dispatchEvent(new Event('input',{bubbles:true}));e.dispatchEvent(new Event('change',{bubbles:true}));return true;})()`);
const snapshot=()=>evaluate(`({count:document.getElementById('turn-count').textContent,telemetry:document.getElementById('state-values').textContent,trail:document.querySelector('[data-mech-scene-id="estimate-path"]')?.getAttribute('points'),truth:document.querySelector('[data-mech-scene-id="truth-body"]')?.getAttribute('points'),status:document.getElementById('measurement-status').textContent})`);
async function step(){
  const before=await evaluate('document.getElementById("turn-count").textContent');
  await evaluate('document.getElementById("step").click()');
  await until(`document.getElementById('turn-count').textContent!==${JSON.stringify(before)}&&!document.getElementById('step').disabled`,'accepted turn');
}
const report={url,startedAt:new Date().toISOString(),cases:[],exceptions};
try{
  await send('Page.enable');await send('Runtime.enable');await send('Network.enable');
  await send('Network.setCacheDisabled',{cacheDisabled:true});
  await send('Emulation.setDeviceMetricsOverride',{width:1440,height:1100,deviceScaleFactor:1,mobile:false});
  await send('Page.navigate',{url:url+'?scene-test='+Date.now()});
  await until('!!document.getElementById("run")&&!document.getElementById("run").disabled','initial scene');
  report.browser=await send('Browser.getVersion');
  report.artifacts=await evaluate(`(async()=>{
    const hashes={};
    for(const file of ['source/ekf.mec','source/scene.mec','_mech/pkg/mech_wasm_bg.wasm']){
      const bytes=await fetch(file).then(r=>r.arrayBuffer());
      hashes[file]=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),v=>v.toString(16).padStart(2,'0')).join('');
    }
    return hashes;
  })()`);
  const gpu=await evaluate('!document.querySelector("#backend option[value=gpu]").disabled');
  report.gpuAvailable=gpu;
  const modes=process.env.IROS_CPU_ONLY?['cpu']:['cpu','gpu'];
  const batches=process.env.IROS_QUICK?[256]:[1,256,4096,65536];
  for(const backend of modes){
    if(backend==='gpu'&&!gpu)throw new Error('WebGPU unavailable: cannot claim CPU/GPU verification');
    await change('backend',backend);
    await until('!document.getElementById("run").disabled','backend compilation');
    for(const instances of batches){
      await change('instances',instances);
      await until('!document.getElementById("run").disabled','batch compilation');
      await change('landmark',1);await change('camera-range',100);
      const initial=await snapshot();
      assert(initial.status.includes('in camera range'));
      await step();const accepted=await snapshot();
      assert.notEqual(accepted.trail,initial.trail,'accepted state advances trail');
      await change('camera-range',10);
      const outside=await snapshot();
      assert.equal(outside.count,accepted.count,'control edit must not advance numerical turn');
      assert.equal(outside.trail,accepted.trail,'control edit must not advance trail');
      assert(outside.status.includes('motion prediction only'),'out-of-range camera');
      await step();const predicted=await snapshot();
      assert.notEqual(predicted.telemetry,outside.telemetry,'missing measurement still predicts');
      await change('landmark',2);await change('camera-range',200);await step();
      const recovered=await snapshot();assert(recovered.status.includes('bearing correction'));
      const drawing=await evaluate(`({dash:document.querySelector('[data-mech-scene-id="estimate-path"]')?.getAttribute('stroke-dasharray'),trail:document.querySelector('[data-mech-scene-id="estimate-path"]')?.getAttribute('stroke'),heading:document.querySelector('[data-mech-scene-id="estimate-heading"]')?.getAttribute('stroke'),sceneCount:document.querySelectorAll('#robot-scene').length})`);
      assert.equal(drawing.sceneCount,1);assert(drawing.dash);assert.equal(drawing.trail,'#ad9159');assert.equal(drawing.heading,'#687780');
      await evaluate('document.getElementById("inject").click()');
      await until('document.getElementById("runtime-error").textContent.includes("finite-candidate")','injected rejection',60000,'finite-candidate');
      const rejected=await snapshot();
      assert.equal(rejected.telemetry,recovered.telemetry,'rejection retains numerical state');
      assert.equal(rejected.trail,recovered.trail,'rejection retains Mech trail');
      assert(rejected.count.includes('1 rejected'));
      report.cases.push({backend,instances,initial,accepted,predicted,recovered,rejected,drawing});
      console.log('PASS',backend,instances,'range, reentry, scene, rejection');
      await evaluate('document.getElementById("reset").click()');
      await until('!document.getElementById("run").disabled','reset recovery');
    }
  }
  if(!process.env.IROS_QUICK){
    report.parity=await evaluate(`(async()=>{const runtime=await import('./assets/runtime.mjs');const {verifyKernel}=await import('./assets/verify.mjs');return verifyKernel({WasmKernel:runtime.WasmKernel,source:await fetch('source/ekf.mec').then(r=>r.text())});})()`);
    assert.equal(report.parity.status,'passed',JSON.stringify(report.parity));
    for(const [width,height] of [[390,844],[900,1000],[1920,1100]]){
      await send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
      assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'),'no page horizontal overflow at '+width);
      const image=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
      writeFileSync(output.replace(/\.json$/,`-${width}.png`),Buffer.from(image.data,'base64'));
    }
  }
  assert.equal(exceptions.length,0,'no uncaught browser exceptions');
  report.status='passed';
}catch(error){report.status='failed';report.error=String(error);process.exitCode=1;console.error(error);}
finally{report.finishedAt=new Date().toISOString();writeFileSync(output,JSON.stringify(report,null,2)+'\n');await send('Page.close');socket.close();}
