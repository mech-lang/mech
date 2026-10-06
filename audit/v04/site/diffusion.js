import init from './pkg/mech_wasm.js';
import {loadAuditWasm} from './load-wasm.js';
import {executeCompute} from './execute-compute.js';
const $ = id => document.getElementById(id);
const json = value => JSON.stringify(value, null, 2);
const f32 = Math.fround;
let artifact, busy = false;
const originalConfig = await (await fetch('../../../examples/gpu-particles/mech.mcfg')).text();
const configuration = originalConfig.replace('"input/force-point", ', '').replace('particle-field', 'heat-diffusion');

export function diffusionSource(width) {
  const count = width * width;
  const laplacian = Array.from({length:count}, (_, i) => {
    const row = Array(count).fill(0), x = i % width, y = Math.floor(i / width);
    for (const j of [x > 0 ? i-1 : -1, x+1 < width ? i+1 : -1,
      y > 0 ? i-width : -1, y+1 < width ? i+width : -1]) {
      if (j >= 0) { row[j] = 1; row[i]--; }
    }
    return row.map(v => `${v}f32`).join(' ');
  }).join('\n');
  const vector = fn => Array.from({length:count}, (_,i) => `${fn(i)}f32`).join('; ');
  return `@pointer := pointer://pointer/frame{:read(pulse), :read(pressed), :read(delta-seconds)}
@particles := compute://particles/kernel{:write(input/force-strength), :write(input/dt), :write(turn)}
@particles/input/force-strength <- @pointer/pressed
@particles/input/dt <- @pointer/delta-seconds
@particles/turn <- @pointer/pulse

heat-diffusion @compute
-------------------------------------------------------------------------------
force-strength := 0f32
dt := 0.125<f32>
laplacian := [${laplacian}]
heater-mask := [${vector(i => i === Math.floor(width/2)*width+Math.floor(width/2) ? 1 : 0)}]
~temperature := [${vector(i => i === 0 ? 80 : 0)}]
temperature = temperature + (laplacian ** temperature) * dt + heater-mask * force-strength
temperature
`;
}

// Independent stencil reference: the Mech path uses a dense matrix product.
export function referenceTrace(width, inputs) {
  let state = Array(width*width).fill(0); state[0] = 80;
  return inputs.map(input => {
    const next = state.map((value,i) => {
      const x = i%width, y = Math.floor(i/width);
      let exchange = 0;
      if(x > 0) exchange += state[i-1]-value;
      if(x+1 < width) exchange += state[i+1]-value;
      if(y > 0) exchange += state[i-width]-value;
      if(y+1 < width) exchange += state[i+width]-value;
      const heater = input.pressed && i === Math.floor(width/2)*width+Math.floor(width/2) ? 1 : 0;
      return f32(f32(value + f32(exchange*input.delta_seconds)) + heater);
    });
    state = next;
    return next;
  });
}

export async function runDiffusion({backend='cpu', width=4, alpha=0.125, turns=8, heater=false, schedule}={}) {
  if(!artifact) throw Error('WASM initialization pending');
  if(![4,8].includes(width) || ![0.125,0.0625].includes(alpha) || !Number.isInteger(turns) || turns<1 || turns>16) throw Error('Unsupported grid or timestep');
  const inputs = Array.from({length:turns}, (_,i) => ({x:0,y:0,pressed:schedule ? schedule[i] : heater,delta_seconds:alpha}));
  const expectedOutputs = referenceTrace(width,inputs);
  const result = await executeCompute({source:diffusionSource(width),configuration,backend,inputs,expectedOutputs,tolerance:1e-5});
  if(result.outcome !== 'passed') return result;
  const conservation = result.frames.map((frame,i) => {
    const expected_sum = 80 + inputs.slice(0,i+1).filter(v=>v.pressed).length;
    const actual_sum = frame.actual.reduce((a,b)=>a+b,0);
    if(Math.abs(actual_sum-expected_sum)>1e-4) throw Error('Heat balance exceeds tolerance');
    if(Math.min(...frame.actual)<-1e-5) throw Error('Temperature positivity violated');
    return {turn:i+1,expected_sum,actual_sum};
  });
  return {...result,width,alpha,turns,artifact,reference_method:'Independent nearest-neighbour flux sum',conservation};
}

function draw(values,width) {
  const ctx=$('grid').getContext('2d'), size=512/width;
  for(let i=0;i<values.length;i++) {
    const scale=Math.min(1,values[i]/80);
    ctx.fillStyle=`rgb(${Math.round(25+230*scale)},${Math.round(70+80*scale)},${Math.round(110-75*scale)})`;
    ctx.fillRect((i%width)*size,Math.floor(i/width)*size,size-1,size-1);
    ctx.fillStyle='white';ctx.font=width===4?'16px monospace':'10px monospace';
    ctx.fillText(values[i].toFixed(3),(i%width)*size+5,Math.floor(i/width)*size+size/2);
  }
}
async function run(backends) {
  if(busy) return; busy=true;$('status').textContent='Compiling and executing heat-diffusion source…';
  try {
    const records=[];
    for(const backend of backends) {
      const result=await runDiffusion({backend,width:Number($('width').value),alpha:Number($('alpha').value),turns:Number($('turns').value),heater:$('heater').checked});
      records.push(result);
      if(result.outcome==='passed') {
        draw(result.frames.at(-1).actual,result.width);
        $('summary').textContent=json({requested:result.requested,selected:result.selected,completed:result.completed,max_absolute_error:Math.max(...result.frames.map(f=>f.max_absolute_error)),heat_balance:result.conservation.at(-1)});
      }
    }
    $('result').textContent=json(records);$('status').textContent=records.map(r=>`${r.requested}: ${r.outcome}`).join(' · ');window.diffusionLast=records;
  } catch(error) {$('status').textContent=String(error);window.diffusionLast={outcome:'failed',error:String(error)};}
  finally {busy=false;}
}
$('cpu').onclick=()=>run(['cpu']);$('gpu').onclick=()=>run(['gpu']);$('compare').onclick=()=>run(['cpu','gpu']);
$('width').onchange=()=>{$('source').value=diffusionSource(Number($('width').value));};
$('source').value=diffusionSource(4);$('config').textContent=configuration;
try {artifact=await loadAuditWasm(init);$('artifact').textContent=`Mech WASM SHA-256 ${artifact.loaded_wasm_sha256 || artifact.artifacts['pkg/mech_wasm_bg.wasm'].sha256}`;$('status').textContent='Ready.';window.diffusionReady=true;window.runDiffusion=runDiffusion;}
catch(error){$('status').textContent=String(error);window.diffusionError=String(error);}
