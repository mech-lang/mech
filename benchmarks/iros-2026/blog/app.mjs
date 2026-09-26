import initializeRuntime, { WasmKernel, WasmRepl } from './runtime.mjs';
import { MechScene } from './drawing.mjs';
import { verifyKernel } from './verify.mjs';

const $ = id => document.getElementById(id);
const text = (id, value) => { $(id).textContent = value; };
let scene;
const MEAN = 'μ', COVARIANCE = 'Σ';
function bindKernelListing() {
  // The live kernel is separate from the document REPL. Do not offer root
  // symbol inspection for values whose state is held by that kernel.
  for(const element of document.querySelectorAll('[data-workshop-kernel-listing] .mech-var-name, [data-workshop-scene-listing] .mech-var-name')) {
    element.dataset.mechValueInteractive='false';
    element.classList.remove('mech-clickable');
    element.removeAttribute('tabindex');
    element.removeAttribute('role');
  }
}
bindKernelListing();
window.addEventListener('mech:document-rendered',bindKernelListing);
document.addEventListener('click', event => {
  // The shared document controller handles TOC navigation and compact layouts.
  if (event.defaultPrevented) return;
  const launcher=event.target.closest?.('[data-workshop-open-output], [data-workshop-fullscreen], a[href="#live-demo"]');
  if(launcher) {
    event.preventDefault();
    window.MechDocumentController?.showOutput();
    if(launcher.hasAttribute('data-workshop-fullscreen')) document.querySelector('[data-mech-output-fullscreen]')?.click();
    return;
  }
  const link=event.target.closest?.('a[href^="#"]');
  if(!link) return;
  const target=document.getElementById(decodeURIComponent(link.getAttribute('href').slice(1)));
  if(target) {event.preventDefault();target.scrollIntoView({behavior:'instant',block:'start'});history.replaceState(null,'',link.getAttribute('href'));}
});
function openOutputFromHash() {
  if(location.hash==='#live-demo') window.MechDocumentController?.showOutput();
}
openOutputFromHash();
for(const event of ['mech:console-ready','mech:document-ready','hashchange']) window.addEventListener(event,openOutputFromHash);
let source, sceneSource, repl, kernel, device, manifest, adapter, active = 0;
let mode = 'paused', busy = false, running = false, generation = 0, samples = [], accepted = 0, rejected = 0;
let state, covariance, frames = [], ready = false;
let committedBackend = 'cpu', committedInstances = '4096';
const rowMajor = a => new Float32Array([a[0],a[3],a[6],a[1],a[4],a[7],a[2],a[5],a[8]]);
const median = values => {
  const a = [...values].sort((a, b) => a - b), m = a.length >> 1;
  return a.length % 2 ? a[m] : (a[m - 1] + a[m]) / 2;
};
function controls() {
  for (const id of ['run','step','inject']) $(id).disabled = !ready || busy || mode === 'fault';
  for (const id of ['reset','instances','backend','verify','landmark','camera-range','velocity','omega','noise']) $(id).disabled = !ready || busy;
  $('pause').disabled = !running;
  $('run').disabled ||= running;
}
function error(message) {
  $('runtime-error').hidden = !message;
  text('runtime-error', message || '');
}
function transition(event) {
  if(!['pause','run','rejected','reset'].includes(event)) throw new Error(`Unknown robot event: ${event}`);
  const response = repl.submit(`#Robot(:${mode}, :${event})`);
  const result = document.createElement('span');
  result.innerHTML = response.result?.inlineHtml || '';
  const atom = result.textContent.trim().match(/^:(paused|patrol|fault)$/);
  if (!atom) {
    throw new Error(`Mech behavior evaluation failed: ${JSON.stringify(response)}`);
  }
  mode = atom[1];
  document.querySelectorAll('[data-state]').forEach(el => {
    el.classList.toggle('active', el.dataset.state === mode);
  });
  text('behavior-message', {paused:'Paused. Run or advance one turn.',patrol:'Patrol. Successive measurements update the filter.',fault:'Fault. The last accepted state is retained; Reset starts a new episode.'}[mode]);
}
function telemetry() {
  const values=`μ = [${Array.from(state, x => x.toFixed(3)).join(' ')}]'\nΣ = [${[0,1,2].map(i => Array.from(covariance.slice(i*3,i*3+3),x=>x.toFixed(4).padStart(9)).join(' ')).join(';\n     ')}]`;
  text('state-values', values);
  const output=document.querySelector('[data-workshop-kernel-output]');
  if(output) {
    const caption=document.createElement('a');
    caption.href='#live-demo';
    caption.textContent=`EKF output · ${accepted} accepted / ${rejected} rejected turns`;
    const pre=document.createElement('pre');
    pre.textContent=values;
    output.replaceChildren(caption,pre);
  }
  text('turn-count', `${accepted} accepted / ${rejected} rejected`);
  text('sample-window', samples.length ? `${samples.length} accepted turns after 5 warm-ups · ${kernel.instances().toLocaleString()} independent filters` : 'Collecting 5 warm-up turns before timing summaries.');
  if (samples.length) {
    const med = median(samples), mad = median(samples.map(x => Math.abs(x-med)));
    text('turn-ms', `${med.toFixed(2)} ± ${mad.toFixed(2)} ms`);
    text('throughput', `${(kernel.instances() / med / 1000).toFixed(3)} M filter-turns/s`);
  }
}
async function dispose(oldDevice, oldKernel) {
  try { if (oldDevice) { oldDevice.dispose(); if (oldDevice.disposeCompletion) await oldDevice.disposeCompletion; } }
  finally { oldKernel?.free(); }
}
async function compile() {
  running = false; generation++; busy = true; controls(); error('');
  let nextKernel, nextDevice, nextScene;
  try {
    text('runtime-status', 'Compiling the displayed Mech source…');
    await new Promise(resolve => requestAnimationFrame(resolve));
    const n = Number($('instances').value);
    nextScene=new MechScene(sceneSource,n,[55,25,.4],[100,0,0,0,100,0,0,0,.15],sensorControls());
    const initial=nextScene.observation(sensorControls());
    nextKernel = WasmKernel.fromSource(source, {
      bearing: new Float32Array(n).fill(-0.55),
      u:initial.inputs.u, m:initial.inputs.m,
    }, [MEAN, COVARIANCE]);
    if (nextKernel.stateWidth(MEAN)!==3 || nextKernel.stateWidth(COVARIANCE)!==9) throw new Error('The robot view requires a three-value state and a 3×3 covariance.');
    const nextManifest = $('backend').value === 'gpu' ? nextKernel.computeManifest() : null;
    if (nextManifest) {
      const freshAdapter = await navigator.gpu.requestAdapter();
      if (!freshAdapter) throw new Error('WebGPU adapter is no longer available.');
      nextDevice = await MechBrowserCompute.Device.create(nextManifest, freshAdapter, nextManifest.exports.map(x=>x.outputName));
    }
    await dispose(device, kernel);
    scene?.dispose();
    scene=nextScene;
    kernel = nextKernel; device = nextDevice; manifest = nextManifest; active = 0;
    committedBackend = $('backend').value; committedInstances = $('instances').value;
    state = kernel.stateSample(MEAN,0); covariance = rowMajor(kernel.stateSample(COVARIANCE,0));
    accepted = rejected = 0; samples = []; frames = [];
    scene.draw(); transition('reset');
    text('fps','—'); text('turn-ms','—'); text('throughput','—'); telemetry();
    text('runtime-status', device ? 'GPU · Mech-generated WGSL / WebGPU · checked turns' : 'CPU · Mech interpreter (WebAssembly) · checked turns');
  } catch (e) {
    if (nextKernel !== kernel) await dispose(nextDevice,nextKernel);
    if (nextScene && nextScene!==scene) nextScene.dispose();
    $('backend').value = committedBackend; $('instances').value = committedInstances;
    error(String(e));
    text('runtime-status', kernel ? 'Compilation failed. The previous kernel is paused.' : 'The kernel could not be compiled.');
  } finally { busy = false; controls(); }
}
function sensorControls() {
  return {velocity:Number($('velocity').value),omega:Number($('omega').value),
    noise:Number($('noise').value),landmark:Number($('landmark').value),range:Number($('camera-range').value)};
}
async function turn(invalid = false) {
  if (busy || !kernel || mode === 'fault') return;
  busy = true; controls(); error('');
  let phase = 'preparation', published = false, integrityRejected = false;
  try {
    const observation = scene.observation(sensorControls(),invalid);
    const begin = performance.now();
    phase = 'execution';
    if (device) {
      const submission = device.submit({inputs: kernel.gpuInputs(observation.inputs)},active);
      const result = await device.finish(submission);
      if (result.integrity) {
        integrityRejected = true;
        throw new Error(`Integrity rejection: constraint ${result.integrity.constraint}, filter ${result.integrity.instance}`);
      }
      active = submission.outputIndex;
      published = true; accepted++; phase = 'readback';
      const output = name => {
        const binding = manifest.exports.find(x=>x.name===name);
        const found = result.outputs.find(x=>x.name===binding.outputName);
        if (!found) throw new Error(`Missing GPU output ${name}`);
        return new Float32Array(found.values);
      };
      state = output(MEAN); covariance = output(COVARIANCE);
    } else {
      const faultsBefore = kernel.faultCount();
      try { kernel.turn(observation.inputs); }
      catch(e) { integrityRejected = kernel.faultCount() > faultsBefore; throw e; }
      published = true; accepted++; phase = 'readback';
      state = kernel.stateSample(MEAN,0); covariance = rowMajor(kernel.stateSample(COVARIANCE,0));
    }
    const duration = performance.now()-begin;
    if (accepted > 5) { samples.push(duration); if(samples.length>60) samples.shift(); }
    phase = 'scene/display update';
    scene.accepted(state,covariance);
    const now=performance.now(); frames.push(now); frames=frames.filter(t=>now-t<=1000);
    if(frames.length>1) text('fps', `${((frames.length-1)*1000/(now-frames[0])).toFixed(1)} FPS`);
    phase = 'telemetry display';
    telemetry();
  } catch(e) {
    running = false; generation++; transition('rejected');
    let diagnostic;
    if (integrityRejected) {
      rejected++;
      diagnostic = String(e);
    } else {
      const detail = published
        ? `The EKF accepted turn ${accepted}, but its ${phase} failed. The numerical state was not rolled back.`
        : phase === 'preparation'
          ? 'Camera/scene preparation failed before the EKF was submitted.'
          : 'EKF execution failed without a reported integrity rejection; publication status is unavailable.';
      text('behavior-message','Execution stopped. Reset is required to resynchronize the filter and scene.');
      diagnostic = `${detail} Reset is required. ${String(e)}`;
    }
    error(diagnostic);
    if (phase !== 'telemetry display') {
      try { telemetry(); }
      catch(displayError) { error(`${diagnostic} Telemetry display also failed: ${String(displayError)}`); }
    }
  } finally {
    busy = false; controls();
  }
}
async function loop(token) {
  if(!running || token!==generation) return;
  await turn();
  if(running && token===generation) requestAnimationFrame(()=>loop(token));
}
for (const [id,label] of [['velocity','velocity-label'],['omega','omega-label'],['noise','noise-label'],['camera-range','camera-range-label']]) {
  $(id).addEventListener('input',()=>text(label,$(id).value));
}
for(const id of ['landmark','camera-range']) $(id).addEventListener('change',()=>{
  if(!scene || busy) return;
  try {scene.observation(sensorControls());scene.draw();} catch(e){error(String(e));}
});
$('run').onclick=()=>{transition('run');running=mode==='patrol';frames=[];const token=++generation;controls();requestAnimationFrame(()=>loop(token));};
$('pause').onclick=()=>{running=false;generation++;transition('pause');controls();};
$('step').onclick=()=>turn();
$('inject').onclick=()=>turn(true);
$('reset').onclick=()=>compile();
for(const id of ['backend','instances']) $(id).onchange=()=>compile();
$('verify').onclick=async()=>{
  running=false;generation++;transition('pause');busy=true;controls();text('verification','Checking CPU/WebGPU parity and rejected-turn rollback…');
  try {
    const result=await verifyKernel({WasmKernel,source});
    $('verification').textContent=JSON.stringify(result,null,2);
    $('verification').dataset.result=result.status;
  } catch(e) { text('verification',String(e)); $('verification').dataset.result='failed'; }
  finally {busy=false;controls();}
};

try {
  [source,sceneSource] = await Promise.all([
    fetch('source/ekf.mec').then(r=>{if(!r.ok)throw new Error('EKF source unavailable');return r.text();}),
    fetch('source/scene.mec').then(r=>{if(!r.ok)throw new Error('Mech scene source unavailable');return r.text();}),
    initializeRuntime()]);
  repl=new WasmRepl();
  const behavior=await fetch('source/behavior.mec').then(r=>r.text());
  const behaviorResult = repl.submit(behavior);
  if(behaviorResult.errors?.length) throw new Error(`Mech behavior could not load: ${JSON.stringify(behaviorResult.errors)}`);
  transition('reset');
  if(navigator.gpu) {
    try { adapter=await navigator.gpu.requestAdapter(); }
    catch { adapter=null; }
    if(adapter) { const option=$('backend').querySelector('[value=gpu]');option.disabled=false;option.textContent='GPU · Mech / WebGPU';text('gpu-support',`WebGPU is available${adapter.info?.description ? ': '+adapter.info.description : ''}.`); }
  }
  if(!adapter) text('gpu-support','WebGPU is unavailable in this browser. CPU execution is available; try a browser with WebGPU support to compare devices.');
  ready=true; await compile();
} catch(e) { error(String(e)); text('runtime-status','The browser runtime could not start. The source and archived results are still available.'); }

document.addEventListener('visibilitychange',()=>{if(document.hidden&&running){running=false;generation++;transition('pause');controls();}});
