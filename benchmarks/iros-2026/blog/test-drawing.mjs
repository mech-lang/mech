import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import initializeWasm, {WasmSceneProgram, WasmKernel} from '../../../src/wasm/pkg/mech_wasm.js';
const source=readFileSync(new URL('source/scene.mec',import.meta.url),'utf8');
const bridge=readFileSync(new URL('drawing.mjs',import.meta.url),'utf8');
assert(source.includes('@scene/replace <- field-presentation'),'Mech publishes its scene tables');
for(const name of ['scene-circles','scene-lines','scene-line-strips','scene-text'])
  assert(source.includes(name),'scene table '+name);
assert(source.includes('0xad9159'),'muted gold trail');
assert(source.includes('0x687780'),'visible dark-gray heading');
assert(source.includes('stroke-dasharray'),'dotted trail lives in Mech source');
assert(source.includes('measurement-visible')&&source.includes('@input/camera-range'),'Mech controls sensor availability');
assert(bridge.includes('WasmSceneProgram.fromSource'),'compile and activate Mech scene');
assert(bridge.includes('MechDocumentController?.renderSceneSvg'),'use shared scene renderer');
assert(!/Math\.(sin|cos|atan2|sqrt|hypot)|createElementNS|replaceChildren/.test(bridge),'no duplicate JavaScript sensor or drawing implementation');

// Evaluate the actual drawing source: checking marker names alone cannot catch
// an estimate drawn on the truth coordinates or one marker hiding the other.
await initializeWasm({module_or_path:readFileSync(new URL('../../../src/wasm/pkg/mech_wasm_bg.wasm',import.meta.url))});
const rowMajor=values=>[values[0],values[3],values[6],values[1],values[4],values[7],values[2],values[5],values[8]];
const sceneInputs=(estimate,covariance)=>({
  commit:0,'lane-indices':new Float64Array([1]),velocity:1,omega:0.015,noise:0.02,
  'camera-index':1,'camera-range':100,estimate:Array.from(estimate),
  covariance:{rows:3,columns:3,values:Array.from(covariance)},
});
const strips=program=>Object.fromEntries(program.scene().line_strips.map(strip=>[strip.id,strip]));
const center=(drawing,id)=>drawing[id+'-heading'].positions[0];
const radius=(drawing,id)=>drawing[id].positions[0][0]-center(drawing,id)[0];
const close=(actual,expected,message)=>assert(Math.abs(actual-expected)<1e-9,message);
let program,kernel;
try {
  program=WasmSceneProgram.fromSource(source,sceneInputs([55,25,0.4],[100,0,0,0,100,0,0,0,0.15]));
  program.turn({commit:0});
  const initial=strips(program);
  assert.deepEqual(center(initial,'truth'),[55,105]);
  assert.deepEqual(center(initial,'estimate'),center(initial,'truth'),'initial poses truly coincide');
  assert.equal(initial.truth.fill,'none','true pose ring stays transparent');
  assert.equal(initial.truth.stroke,'#63d6c5');
  assert.equal(initial.estimate.fill,'#f6c04e');
  assert(radius(initial,'truth')-initial.truth.stroke_width/2>
    radius(initial,'estimate')+initial.estimate.stroke_width/2,'both marker edges remain visible at coincidence');
  assert.equal(initial['truth-path'].stroke,'#63d6c5');
  assert.deepEqual(initial['truth-path'].stroke_dasharray,[0],'true trajectory is solid beneath estimate dots');
  assert.equal(initial['estimate-path'].stroke,'#ad9159');
  assert.deepEqual(initial['estimate-path'].stroke_dasharray,[0.1,1.5]);
  for(const id of ['truth-heading','estimate-heading']) assert.equal(initial[id].stroke,'#687780');

  // A deliberately distinct input verifies the drawing binding, not a change
  // to the simulation or filter initial state used by the application.
  program.turn({commit:0,estimate:[65,45,-0.2]});
  const separate=strips(program);
  assert.deepEqual(center(separate,'truth'),[55,105],'estimate input cannot move actual robot');
  assert.deepEqual(center(separate,'estimate'),[65,85],'estimate uses its own accepted state');
  assert.deepEqual(separate['truth-path'],initial['truth-path'],'prepare retains actual history');
  assert.deepEqual(separate['estimate-path'],initial['estimate-path'],'prepare retains estimated history');
  program.turn({commit:1});
  const accepted=strips(program);
  for(const id of ['truth','estimate'])
    assert.deepEqual(accepted[id+'-path'].positions.at(-1),center(accepted,id),id+' path ends at its own current pose');
  close(center(accepted,'truth')[0],55+0.1*Math.cos(0.4),'true x follows the existing plant');
  close(center(accepted,'truth')[1],105-0.1*Math.sin(0.4),'true y follows the existing plant');
  program.turn({commit:0});
  for(const id of ['truth','estimate'])
    assert.deepEqual(strips(program)[id+'-path'],accepted[id+'-path'],'uncommitted turn retains '+id+' history');
  program.stop();program.free();program=null;

  kernel=WasmKernel.fromSource(readFileSync(new URL('source/ekf.mec',import.meta.url),'utf8'),
    {bearing:new Float32Array([-0.55]),u:[1,0.015,1],m:[140,12]},['μ','Σ']);
  program=WasmSceneProgram.fromSource(source,sceneInputs(kernel.stateSample('μ',0),rowMajor(kernel.stateSample('Σ',0))));
  for(let turn=0;turn<12;turn++) {
    program.turn({commit:0});
    kernel.turn({bearing:Float32Array.from(program.readNumbers('readings')),
      u:[1,0.015,program.readNumbers('measurement-visible')[0]],
      m:Float32Array.from(program.readNumbers('camera-position'))});
    const estimate=Array.from(kernel.stateSample('μ',0));
    program.turn({commit:1,estimate,covariance:{rows:3,columns:3,values:rowMajor(kernel.stateSample('Σ',0))}});
    const drawing=strips(program),truth=Array.from(program.readNumbers('truth'));
    assert.deepEqual(center(drawing,'truth'),[truth[0],130-truth[1]],'draw the actual simulated position');
    assert.deepEqual(center(drawing,'estimate'),[estimate[0],130-estimate[1]],'draw the accepted first filter estimate');
    for(const id of ['truth','estimate'])
      assert.deepEqual(drawing[id+'-path'].positions.at(-1),center(drawing,id),'accepted '+id+' pose extends its own trail');
    assert.equal(program.readNumbers('accepted-turns')[0],turn+1);
  }
} finally {
  program?.stop();program?.free();kernel?.free();
}
console.log('PASS: real Mech truth/estimate geometry, visible coincident markers, independent paths, and accepted EKF scene updates.');
