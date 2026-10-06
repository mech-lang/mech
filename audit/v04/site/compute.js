import init, {WasmMixedComputeProject} from './pkg/mech_wasm.js';
import {loadAuditWasm} from './load-wasm.js';
import {executeCompute} from './execute-compute.js';
const $=id=>document.getElementById(id), json=x=>JSON.stringify(x,(_,v)=>typeof v==='bigint'?v.toString():v,2);
const f32=Math.fround;
let ready=false, busy=false, artifact;
const originalConfig=await (await fetch('../../../examples/gpu-particles/mech.mcfg')).text();
const config=originalConfig.replace('"input/force-strength", "input/dt", ', '');
export function computeSource(columns=3) { return `+> math
@pointer := pointer://pointer/frame{:read(pulse), :read(position)}
@particles := compute://particles/kernel{:write(input/force-point), :write(turn)}
@particles/input/force-point <- @pointer/position
@particles/turn <- @pointer/pulse

particle-field @compute
-------------------------------------------------------------------------------
force-point := [0f32; 0f32]
row := math/mod(1f32..=${columns}f32, 4f32)
~matrix := [row; row + 3f32]
matrix = matrix + force-point
matrix'
`; }
export async function runCompute({backend='cpu',columns=3,turns=2,x=10,y=20}={}) {
  if(!ready)throw Error('Mech initialization pending');
  if(![3,1024].includes(columns)||!Number.isInteger(turns)||turns<1||turns>8||![x,y].every(Number.isFinite))throw Error('Invalid experiment input');
  const reference=Array.from({length:columns},(_,i)=>[(i+1)%4,((i+1)%4)+3]);
  const expectedOutputs=[];
  for(let turn=0;turn<turns;turn++) {
    for(const row of reference){row[0]=f32(row[0]+f32(x));row[1]=f32(row[1]+f32(y));}
    expectedOutputs.push(reference.flat());
  }
  const tolerance=Number.isInteger(x)&&Number.isInteger(y)&&Math.max(Math.abs(x),Math.abs(y))*turns<1e6?0:0.0001;
  const inputs=Array.from({length:turns},()=>({x,y,pressed:false,delta_seconds:1/60}));
  const result=await executeCompute({source:computeSource(columns),configuration:config,backend,inputs,expectedOutputs,tolerance});
  return {...result,columns,turns,artifact};
}
function draw(values){const started=performance.now(),ctx=$('plot').getContext('2d'),w=640,h=270;ctx.clearRect(0,0,w,h);ctx.fillStyle='#f2f6f6';ctx.fillRect(0,0,w,h);const pairs=[];for(let i=0;i<values.length;i+=2)pairs.push([values[i],values[i+1]]);const xs=pairs.map(p=>p[0]),ys=pairs.map(p=>p[1]),minx=Math.min(...xs),miny=Math.min(...ys),dx=Math.max(1,Math.max(...xs)-minx),dy=Math.max(1,Math.max(...ys)-miny);ctx.fillStyle='#216d73';for(const [x,y]of pairs){ctx.beginPath();ctx.arc(30+(x-minx)/dx*580,240-(y-miny)/dy*210,4,0,2*Math.PI);ctx.fill();}return performance.now()-started;}
async function run(backends){if(busy)return;busy=true;$('status').textContent='Executing identified Mech artifact…';try{const results=[];for(const backend of backends){const r=await runCompute({backend,columns:Number($('columns').value),turns:Number($('turns').value),x:Number($('x').value),y:Number($('y').value)});results.push(r);if(r.outcome==='passed'){const last=r.frames.at(-1);r.drawing_ms=draw(last.actual);$('values').textContent=json({elements:last.elements,first_12:last.actual.slice(0,12),max_absolute_error:last.max_absolute_error});}}$('results').textContent=json(results);$('status').textContent=results.map(r=>`${r.requested}: ${r.outcome}; completed ${r.completed||'none'}`).join(' · ');window.lastComputeResults=results;}catch(e){$('status').textContent=`Failed: ${e}`;$('results').textContent=e.stack||String(e);window.lastComputeResults={outcome:'failed',error:String(e)};}finally{busy=false;}}
$('columns').onchange=()=>{$('source').value=computeSource(Number($('columns').value));};$('source').value=computeSource();$('config').textContent=config;
$('cpu').onclick=()=>run(['cpu']);$('gpu').onclick=()=>run(['gpu']);$('compare').onclick=()=>run(['cpu','gpu']);
export function hardPlacementProbe(){try{const source=computeSource().replace('particle-field @compute','particle-field @gpu');const p=WasmMixedComputeProject.fromSource(config,source,'cpu',false,undefined);p.stop();p.free();return {outcome:'failed',reason:'Hard GPU placement accepted CPU selector'};}catch(e){return {requested:'cpu',gpu_available:false,source_annotation:'@gpu',outcome:'rejected',diagnostic:String(e)};}}
$('hard').onclick=()=>{$('results').textContent=json(hardPlacementProbe());};
try{artifact=await loadAuditWasm(init);$('artifact').textContent=`Mech WASM · ${artifact.source_commit||artifact.baseline_commit||'see artifact record'} · ${json(artifact.artifacts||artifact.files||artifact).slice(0,450)}`;ready=true;window.computeReady=true;window.runCompute=runCompute;$('status').textContent='Ready for execution.';}catch(e){$('status').textContent=`Initialization failed: ${e}`;window.computeReady=false;}
