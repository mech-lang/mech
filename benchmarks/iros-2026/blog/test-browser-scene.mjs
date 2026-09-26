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
const snapshot=()=>evaluate(`({count:document.getElementById('turn-count').textContent,telemetry:document.getElementById('state-values').textContent,trail:document.querySelector('[data-mech-scene-id="estimate-path"]')?.getAttribute('points'),truth:document.querySelector('[data-mech-scene-id="truth"]')?.getAttribute('points'),truthTrail:document.querySelector('[data-mech-scene-id="truth-path"]')?.getAttribute('points'),status:document.getElementById('measurement-status').textContent})`);
async function dragSlider(name){
  await evaluate(`document.getElementById(${JSON.stringify(name)}).scrollIntoView({block:'center'})`);
  const box=await evaluate(`(()=>{const e=document.getElementById(${JSON.stringify(name)}),r=e.getBoundingClientRect();return {x:r.x,y:r.y,w:r.width,h:r.height,min:Number(e.min),max:Number(e.max)};})()`);
  const y=box.y+box.h/2,start=box.x+box.w*.2;
  await send('Input.dispatchMouseEvent',{type:'mousePressed',x:start,y,button:'left',buttons:1,clickCount:1});
  const samples=[];
  for(let i=0;i<=8;i++){
    await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:box.x+box.w*(.2+i*.04),y,button:'left',buttons:1});
    await new Promise(r=>setTimeout(r,70));
    samples.push(await evaluate(`({disabled:document.getElementById(${JSON.stringify(name)}).disabled,value:Number(document.getElementById(${JSON.stringify(name)}).value)})`));
  }
  await send('Input.dispatchMouseEvent',{type:'mouseReleased',x:box.x+box.w*.52,y,button:'left',buttons:0,clickCount:1});
  assert(samples.every(s=>!s.disabled),name+' stays enabled throughout a real pointer drag');
  assert(new Set(samples.map(s=>s.value)).size>=5,name+' follows the held pointer across running turns');
  const final=samples.at(-1).value;
  assert(Math.abs((final-box.min)/(box.max-box.min)-.52)<.05,name+' reaches the dragged position');
  return {name,samples};
}
async function clickCamera(index){
  const selector=`#robot-scene [data-camera-index="${index}"][role="button"]`;
  await evaluate(`document.querySelector(${JSON.stringify(selector)}).scrollIntoView({block:'center',behavior:'instant'})`);
  await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
  const point=await evaluate(`(()=>{const r=document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  const hit=await evaluate(`document.elementFromPoint(${point.x},${point.y})?.closest('[data-camera-index]')?.getAttribute('data-camera-index')`);
  assert.equal(hit,String(index),'pointer reaches camera '+index+' at '+JSON.stringify(point));
  await send('Input.dispatchMouseEvent',{type:'mousePressed',...point,button:'left',buttons:1,clickCount:1});
  await send('Input.dispatchMouseEvent',{type:'mouseReleased',...point,button:'left',buttons:0,clickCount:1});
}
const cameraEnabled=index=>evaluate(`document.querySelector('#robot-scene [data-camera-index="${index}"][role="button"]').getAttribute('aria-pressed')==='true'`);
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
    for(const file of ['source/ekf.mec','source/camera-ekf.mec','source/scene.mec','_mech/pkg/mech_wasm_bg.wasm']){
      const bytes=await fetch(file).then(r=>r.arrayBuffer());
      hashes[file]=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),v=>v.toString(16).padStart(2,'0')).join('');
    }
    return hashes;
  })()`);
  const gpu=await evaluate('!document.querySelector("#backend option[value=gpu]").disabled');
  assert.equal(await evaluate('!!document.getElementById("landmark")'),false,'no landmark dropdown');
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
      await change('camera-range',250);
      const initial=await snapshot();
      assert.match(initial.status,/^4 of 4 enabled cameras in range/,'all cameras observe the robot at maximum range');
      for(let i=1;i<=4;i++)assert(await cameraEnabled(i),'all cameras initially enabled');
      await step();const accepted=await snapshot();
      assert.notEqual(accepted.trail,initial.trail,'accepted state advances trail');
      assert.notEqual(accepted.truthTrail,initial.truthTrail,'accepted turn advances actual robot path');
      const poses=await evaluate(`Object.fromEntries(['truth','estimate'].map(id=>{const e=document.querySelector('[data-mech-scene-id="'+id+'"]');return [id,{points:e.getAttribute('points'),stroke:e.getAttribute('stroke'),fill:e.getAttribute('fill')}]}))`);
      assert.notEqual(poses.truth.points,poses.estimate.points,'truth and estimate use distinguishable geometry');
      assert.equal(poses.truth.fill,'none','truth ring does not hide the estimate when they overlap');
      for(let i=1;i<=4;i++){
        await clickCamera(i);
        await until(`document.querySelector('#robot-scene [data-camera-index="${i}"][role="button"]').getAttribute('aria-pressed')==='false'`,'disable camera '+i);
      }
      const disabled=await snapshot();
      assert.equal(disabled.count,accepted.count,'camera clicks do not advance the numerical state');
      assert.equal(disabled.truthTrail,accepted.truthTrail,'camera clicks do not move the simulated robot');
      assert(disabled.status.includes('prediction'),'all disabled gives prediction only');
      await step();const unobserved=await snapshot();
      assert.notEqual(unobserved.telemetry,disabled.telemetry,'all-off still predicts');
      for(let i=1;i<=4;i++){
        await clickCamera(i);
        await until(`document.querySelector('#robot-scene [data-camera-index="${i}"][role="button"]').getAttribute('aria-pressed')==='true'`,'enable camera '+i);
      }
      await change('camera-range',10);
      const outside=await snapshot();
      assert.equal(outside.count,unobserved.count,'control edit must not advance numerical turn');
      assert.equal(outside.trail,unobserved.trail,'control edit must not advance trail');
      assert(outside.status.includes('prediction'),'all out-of-range cameras give prediction only');
      await step();const predicted=await snapshot();
      assert.notEqual(predicted.telemetry,outside.telemetry,'missing measurement still predicts');
      await change('camera-range',250);await step();
      const recovered=await snapshot();assert.match(recovered.status,/^4 of 4 enabled cameras in range/,'all four cameras contribute within range');
      const drawing=await evaluate(`({dash:document.querySelector('[data-mech-scene-id="estimate-path"]')?.getAttribute('stroke-dasharray'),trail:document.querySelector('[data-mech-scene-id="estimate-path"]')?.getAttribute('stroke'),heading:document.querySelector('[data-mech-scene-id="estimate-heading"]')?.getAttribute('stroke'),sceneCount:document.querySelectorAll('#robot-scene').length})`);
      assert.equal(drawing.sceneCount,1);assert(drawing.dash);assert.equal(drawing.trail,'#ad9159');assert.equal(drawing.heading,'#687780');
      await evaluate('document.getElementById("inject").click()');
      await until('document.getElementById("runtime-error").textContent.includes("finite-candidate")','injected rejection',60000,'finite-candidate');
      const rejected=await snapshot();
      assert.equal(rejected.telemetry,recovered.telemetry,'rejection retains numerical state');
      assert.equal(rejected.trail,recovered.trail,'rejection retains Mech trail');
      assert.equal(rejected.truthTrail,recovered.truthTrail,'rejection retains actual robot path');
      assert(rejected.count.includes('1 rejected'));
      report.cases.push({backend,instances,initial,accepted,disabled,unobserved,predicted,recovered,rejected,drawing});
      console.log('PASS',backend,instances,'four camera clicks, all-off prediction, range, reentry, scene, rejection');
      await evaluate('document.getElementById("reset").click()');
      await until('!document.getElementById("run").disabled','reset recovery');
    }
  }
  if(!process.env.IROS_QUICK){
    report.liveDrags=[];
    for(const backend of modes){
      await change('backend',backend);await until('!document.getElementById("run").disabled','drag backend compilation');
      await change('instances',4096);await until('!document.getElementById("run").disabled','drag batch compilation');
      await change('velocity',1);await change('omega',.015);await change('noise',1);await change('motion-noise',1);await change('camera-range',250);
      const before=await snapshot();
      await evaluate('document.getElementById("run").click()');
      await clickCamera(1);
      await until(`document.querySelector('#robot-scene [data-camera-index="1"][role="button"]').getAttribute('aria-pressed')==='false'`,'camera click during running turns');
      await clickCamera(1);
      await until(`document.querySelector('#robot-scene [data-camera-index="1"][role="button"]').getAttribute('aria-pressed')==='true'`,'camera restoration during running turns');
      const drags=[];
      for(const name of ['velocity','omega','noise','motion-noise','camera-range'])drags.push(await dragSlider(name));
      await evaluate('document.getElementById("pause").click()');
      await until('!document.getElementById("step").disabled','pause after live slider drag');
      const after=await snapshot();assert.notEqual(after.count,before.count,'turns continue while sliders are dragged');
      assert.notEqual(after.truthTrail,before.truthTrail,'robot moves while steering');
      report.liveDrags.push({backend,drags,before:before.count,after:after.count});
      await evaluate('document.getElementById("reset").click()');await until('!document.getElementById("run").disabled','drag reset');
      await change('instances',256);await until('!document.getElementById("run").disabled','wrap batch compilation');
      await change('velocity',12);await change('omega',0);await change('noise',0);await change('motion-noise',0);await change('camera-range',250);
      await evaluate(`(()=>{
        const pose=()=>document.querySelector('[data-mech-scene-id="truth-heading"]').getAttribute('points').trim().split(/[ ,]+/).map(Number).slice(0,2);
        let previous=pose();window.workshopWrapEvents=[];
        window.workshopWrapObserver=new MutationObserver(()=>{const next=pose();if(Math.abs(next[0]-previous[0])>100||Math.abs(next[1]-previous[1])>65){workshopWrapEvents.push({from:previous,to:next});document.getElementById('pause').click();}previous=next;});
        workshopWrapObserver.observe(document.querySelector('.scene-wrap'),{childList:true});
      })()`);
      await evaluate('document.getElementById("run").click()');
      await until('workshopWrapEvents.length>0&&!document.getElementById("step").disabled','robot wraps the field while running',120000);
      const wrap=await evaluate(`(()=>{workshopWrapObserver.disconnect();const p=document.querySelector('[data-mech-scene-id="truth-path"]').getAttribute('points').trim().split(/\\s+/);return {events:workshopWrapEvents,trailPoints:new Set(p).size,count:document.getElementById('turn-count').textContent};})()`);
      assert.equal(wrap.trailPoints,1,'truth trail restarts at wrap instead of crossing the field');
      (report.wraps??=[]).push({backend,...wrap});
      await evaluate('document.getElementById("reset").click()');await until('!document.getElementById("run").disabled','wrap reset');
    }
    report.parity=await evaluate(`(async()=>{const runtime=await import('./assets/runtime.mjs');const {verifyKernel}=await import('./assets/verify.mjs');return verifyKernel({WasmKernel:runtime.WasmKernel,source:await fetch('source/camera-ekf.mec').then(r=>r.text())});})()`);
    assert.equal(report.parity.status,'passed',JSON.stringify(report.parity));
    await evaluate('document.fonts.ready.then(()=>true)');
    for(const [width,height] of [[320,740],[360,800],[390,844],[900,1000],[1920,1100]]){
      await send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
      await evaluate('MechDocumentController.showOutput()');
      await evaluate(`(()=>{for(let e=document.getElementById('ekf-app');e;e=e.parentElement)e.scrollTop=0;})()`);
      for(const panel of ['open','closed']) {
      if(panel==='closed') await evaluate('document.dispatchEvent(new KeyboardEvent("keydown",{key:String.fromCharCode(96),bubbles:true}))');
      await new Promise(r=>setTimeout(r,200)); // Let the drawer's 120ms transition settle before layout/screenshots.
      assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'),'no page horizontal overflow at '+width);
      const metadata=await evaluate(`(()=>{
        const date=document.querySelector('.hero .mech-date'),author=document.querySelector('.hero .mech-author');
        const range=document.createRange();range.selectNodeContents(date.querySelector('.mech-text')||date);
        return {dateLines:range.getClientRects().length,authorRight:author.getBoundingClientRect().right,dateLeft:date.getBoundingClientRect().left,dateRight:date.getBoundingClientRect().right,metaRight:date.parentElement.getBoundingClientRect().right};
      })()`);
      assert.equal(metadata.dateLines,1,'date stays on one line at '+width);
      assert(metadata.authorRight<metadata.dateLeft&&metadata.dateRight<=metadata.metaRight+1,'metadata fits without overlap at '+width);
      assert(metadata.dateLeft-metadata.authorRight<40,'date sits near wrapped authors at '+width);
      if(panel==='open'){
        const layout=await evaluate(`(()=>{const svg=document.getElementById('robot-scene'),r=svg.getBoundingClientRect(),v=svg.viewBox.baseVal;return {width:r.width,height:r.height,aspect:v.width/v.height,parentWidth:svg.parentElement.clientWidth};})()`);
        assert(Math.abs(layout.width/layout.height-layout.aspect)<.01,'scene scales proportionally at '+width);
        assert(Math.abs(layout.width-layout.parentWidth)<2,'scene fills available width at '+width);
        (report.sceneLayouts??=[]).push({viewportWidth:width,...layout});
        if(width>900){
          const grip=await evaluate(`(()=>{const track=document.querySelector('[data-mech-repl-host]>[data-mech-console-resizer]:not([data-mech-console-edge-handle])'),r=track.getBoundingClientRect(),p=document.querySelector('[data-mech-console-pane]').getBoundingClientRect(),s=getComputedStyle(track,'::after');return {centerX:r.x+parseFloat(s.left),dividerX:p.x+.5,centerY:r.y+parseFloat(s.top),paneCenterY:p.y+p.height/2};})()`);
          assert(Math.abs(grip.centerX-grip.dividerX)<1,'grip centered on divider');
          assert(Math.abs(grip.centerY-grip.paneCenterY)<1,'grip vertically centered');
          (report.gripLayouts??=[]).push({viewportWidth:width,...grip});
        }
      }
      (report.metadataLayouts??=[]).push({width,panel,...metadata});
      const image=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
      writeFileSync(output.replace(/\.json$/,`-${width}-${panel}.png`),Buffer.from(image.data,'base64'));
      }
    }
  }
  assert.equal(exceptions.length,0,'no uncaught browser exceptions');
  report.status='passed';
}catch(error){report.status='failed';report.error=String(error);process.exitCode=1;console.error(error);const shot=await send('Page.captureScreenshot',{format:'png'});writeFileSync(output.replace(/\.json$/,'-failure.png'),Buffer.from(shot.data,'base64'));}
finally{report.finishedAt=new Date().toISOString();writeFileSync(output,JSON.stringify(report,null,2)+'\n');await send('Page.close');socket.close();}
