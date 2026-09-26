import init, { WasmKernel, WasmRepl } from '../_mech/pkg/mech_wasm.js';
import { RobotScene } from './drawing.mjs';
import { verifyKernel } from './verify.mjs';

const $ = id => document.getElementById(id);
const text = (id, value) => { $(id).textContent = value; };
const scene = new RobotScene($('robot-scene'));
document.addEventListener('click', event => {
  const link=event.target.closest?.('a[href^="#"]');
  if(!link) return;
  const target=document.getElementById(decodeURIComponent(link.getAttribute('href').slice(1)));
  if(target) {event.preventDefault();target.scrollIntoView({behavior:'instant',block:'start'});history.replaceState(null,'',link.getAttribute('href'));}
});
let original, source, repl, kernel, device, manifest, adapter, active = 0;
let mode = 0, busy = false, running = false, generation = 0, samples = [], accepted = 0, rejected = 0;
let state, covariance, frames = [], ready = false;
let committedBackend = 'cpu', committedInstances = '4096';
const rowMajor = a => new Float32Array([a[0],a[3],a[6],a[1],a[4],a[7],a[2],a[5],a[8]]);
async function initializeRuntime() {
  if (typeof DecompressionStream !== 'undefined') {
    const response=await fetch(new URL('../_mech/pkg/mech_wasm_bg.wasm.gz',import.meta.url));
    if (response.ok) {
      let bytes=await response.arrayBuffer();
      const magic=new Uint8Array(bytes,0,2);
      // Some hosts apply Content-Encoding and fetch decompresses for us.
      if(magic[0]===0x1f && magic[1]===0x8b) bytes=await new Response(new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip'))).arrayBuffer();
      return init({module_or_path:bytes});
    }
  }
  return init();
}
const median = values => {
  const a = [...values].sort((a, b) => a - b), m = a.length >> 1;
  return a.length % 2 ? a[m] : (a[m - 1] + a[m]) / 2;
};
function controls() {
  for (const id of ['run','step','inject']) $(id).disabled = !ready || busy || mode === 2;
  for (const id of ['reset','compile','restore','instances','backend','verify']) $(id).disabled = !ready || busy;
  $('pause').disabled = !running;
  $('run').disabled ||= running;
  $('verify').disabled ||= source !== original;
}
function error(message) {
  $('runtime-error').hidden = !message;
  text('runtime-error', message || '');
}
function transition(event) {
  const response = repl.submit(`#Robot(${mode}, ${event})`);
  if (!response.result || !/^[012]$/.test(response.result.inlineHtml.trim())) {
    throw new Error(`Mech behavior evaluation failed: ${JSON.stringify(response)}`);
  }
  mode = Number(response.result.inlineHtml);
  document.querySelectorAll('[data-state]').forEach(el => {
    el.classList.toggle('active', Number(el.dataset.state) === mode);
  });
  text('behavior-message', ['Paused. Run or advance one turn.','Patrol. Successive measurements update the filter.','Fault. The last accepted state is retained; Reset starts a new episode.'][mode]);
}
function telemetry() {
  text('state-values', `μ = [${Array.from(state, x => x.toFixed(3)).join(', ')}]\nΣ =\n${[0,1,2].map(i => '  '+Array.from(covariance.slice(i*3,i*3+3),x=>x.toFixed(4).padStart(9)).join(' ')).join('\n')}`);
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
async function compile(nextSource = source) {
  running = false; generation++; busy = true; controls(); error('');
  let nextKernel, nextDevice;
  try {
    text('runtime-status', 'Compiling the displayed Mech source…');
    await new Promise(resolve => requestAnimationFrame(resolve));
    const n = Number($('instances').value);
    nextKernel = WasmKernel.fromSource(nextSource, {
      bearing: new Float32Array(n).fill(-0.55),
      v: [Number($('velocity').value)], w: [Number($('omega').value)]
    }, ['state', 'covariance']);
    if (nextKernel.stateWidth('state')!==3 || nextKernel.stateWidth('covariance')!==9) throw new Error('The robot view requires a three-value state and a 3×3 covariance.');
    const nextManifest = $('backend').value === 'gpu' ? nextKernel.computeManifest() : null;
    if (nextManifest) {
      const freshAdapter = await navigator.gpu.requestAdapter();
      if (!freshAdapter) throw new Error('WebGPU adapter is no longer available.');
      nextDevice = await MechBrowserCompute.Device.create(nextManifest, freshAdapter, nextManifest.exports.map(x=>x.outputName));
    }
    await dispose(device, kernel);
    kernel = nextKernel; device = nextDevice; manifest = nextManifest; active = 0; source = nextSource;
    committedBackend = $('backend').value; committedInstances = $('instances').value;
    state = kernel.stateSample('state',0); covariance = rowMajor(kernel.stateSample('covariance',0));
    accepted = rejected = 0; samples = []; frames = [];
    scene.reset(); scene.draw(state,covariance); transition(3);
    text('fps','—'); text('turn-ms','—'); text('throughput','—'); telemetry();
    text('runtime-status', device ? 'WebGPU · generated WGSL · checked turns' : 'CPU · Rust/WASM scalar interpreter · checked turns');
    const modified = source !== original;
    text('source-origin',modified ? 'Executing the edited source below' : 'Executing the source above');
    text('source-status', modified ? 'Running your edited source. The archived charts are unchanged. Restore the paper source to run its retained parity test.' : 'Running the complete EKF source printed above.');
    $('modified-source').hidden = !modified;
    $('modified-source-code').textContent = source;
  } catch (e) {
    if (nextKernel !== kernel) await dispose(nextDevice,nextKernel);
    $('backend').value = committedBackend; $('instances').value = committedInstances;
    error(String(e));
    text('runtime-status', kernel ? 'Compilation failed. The previous kernel is paused.' : 'The kernel could not be compiled.');
  } finally { busy = false; controls(); }
}
async function turn(invalid = false) {
  if (busy || !kernel || mode === 2) return;
  busy = true; controls(); error('');
  const observation = scene.observation(Number($('velocity').value),Number($('omega').value),Number($('noise').value),kernel.instances(),invalid);
  const begin = performance.now();
  try {
    if (device) {
      const submission = device.submit({inputs: kernel.gpuInputs(observation.inputs)},active);
      const result = await device.finish(submission);
      if (result.integrity) throw new Error(`Integrity rejection: constraint ${result.integrity.constraint}, filter ${result.integrity.instance}`);
      active = submission.outputIndex;
      const output = name => {
        const binding = manifest.exports.find(x=>x.name===name);
        const found = result.outputs.find(x=>x.name===binding.outputName);
        if (!found) throw new Error(`Missing GPU output ${name}`);
        return new Float32Array(found.values);
      };
      state = output('state'); covariance = output('covariance');
    } else {
      kernel.turn(observation.inputs);
      state = kernel.stateSample('state',0); covariance = rowMajor(kernel.stateSample('covariance',0));
    }
    const duration = performance.now()-begin;
    accepted++;
    if (accepted > 5) { samples.push(duration); if(samples.length>60) samples.shift(); }
    scene.accepted(observation.next,state,covariance);
    const now=performance.now(); frames.push(now); frames=frames.filter(t=>now-t<=1000);
    if(frames.length>1) text('fps', `${((frames.length-1)*1000/(now-frames[0])).toFixed(1)} FPS`);
  } catch(e) {
    rejected++; running = false; generation++; transition(2); error(String(e));
  } finally { telemetry(); busy = false; controls(); }
}
async function loop(token) {
  if(!running || token!==generation) return;
  await turn();
  if(running && token===generation) requestAnimationFrame(()=>loop(token));
}
for (const [id,label] of [['velocity','velocity-label'],['omega','omega-label'],['noise','noise-label']]) {
  $(id).addEventListener('input',()=>text(label,$(id).value));
}
$('run').onclick=()=>{transition(1);running=mode===1;frames=[];const token=++generation;controls();requestAnimationFrame(()=>loop(token));};
$('pause').onclick=()=>{running=false;generation++;transition(0);controls();};
$('step').onclick=()=>turn();
$('inject').onclick=()=>turn(true);
$('reset').onclick=()=>compile();
for(const id of ['backend','instances']) $(id).onchange=()=>compile();
$('compile').onclick=()=>compile($('source-editor').value);
$('restore').onclick=()=>{$('source-editor').value=original;compile(original);};
$('verify').onclick=async()=>{
  running=false;generation++;transition(0);busy=true;controls();text('verification','Checking CPU/WebGPU parity and rejected-turn rollback…');
  try {
    const result=await verifyKernel({WasmKernel,source});
    $('verification').textContent=JSON.stringify(result,null,2);
    $('verification').dataset.result=result.status;
  } catch(e) { text('verification',String(e)); $('verification').dataset.result='failed'; }
  finally {busy=false;controls();}
};

try {
  [original] = await Promise.all([fetch('source/ekf.mec').then(r=>{if(!r.ok)throw new Error('EKF source unavailable');return r.text();}), initializeRuntime()]);
  source=original; $('source-editor').value=source;
  repl=new WasmRepl();
  const behavior=await fetch('source/behavior.mec').then(r=>r.text());
  repl.submit(behavior); transition(3);
  if(navigator.gpu) {
    try { adapter=await navigator.gpu.requestAdapter(); }
    catch { adapter=null; }
    if(adapter) { const option=$('backend').querySelector('[value=gpu]');option.disabled=false;option.textContent='GPU · WebGPU';text('gpu-support',`WebGPU is available${adapter.info?.description ? ': '+adapter.info.description : ''}.`); }
  }
  if(!adapter) text('gpu-support','WebGPU is unavailable in this browser. CPU execution is available; try a browser with WebGPU support to compare devices.');
  ready=true; await compile();
  for(const button of document.querySelectorAll('[data-run-example]')) {
    button.disabled=false;
    button.onclick=()=>{
      const name=button.dataset.runExample, example=new WasmRepl();
      try {
        const response=example.submit($(name+'-source').value);
        if(!response.result) throw new Error(JSON.stringify(response.events));
        const value=response.result.inlineHtml.replace(/<[^>]*>/g,'').replaceAll('&quot;','"').replaceAll('&#39;',"'").replaceAll('&lt;','<').replaceAll('&gt;','>').replaceAll('&amp;','&');
        text(name+'-result',value);
      } catch(e) {text(name+'-result',String(e));}
      finally {example.free();}
    };
  }
} catch(e) { error(String(e)); text('runtime-status','The browser runtime could not start. The source and archived results are still available.'); }

document.addEventListener('visibilitychange',()=>{if(document.hidden&&running){running=false;generation++;transition(0);controls();}});
