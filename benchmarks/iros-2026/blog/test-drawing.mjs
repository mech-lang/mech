import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import initializeWasm, {WasmSceneProgram, WasmKernel} from '../../../src/wasm/pkg/mech_wasm.js';
const source=readFileSync(new URL('source/scene.mec',import.meta.url),'utf8');
const bridge=readFileSync(new URL('drawing.mjs',import.meta.url),'utf8');
assert(source.includes('@scene/replace <- field-presentation'),'Mech publishes its scene tables');
for(const name of ['scene-circles','scene-lines','scene-line-strips','scene-text'])
  assert(source.includes(name),'scene table '+name);
assert(source.includes('0xad9159'),'muted gold trail');
assert(source.includes('0x46968a')&&source.includes('0xac8637'),'heading lines use darker shades of their robot colors');
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
  commit:0,'lane-indices':new Float64Array([1,2]),velocity:1,omega:0.015,noise:1,
  'motion-noise':0,'camera-pulses':[0,0,0,0],'camera-range':100,estimate:Array.from(estimate),
  covariance:{rows:3,columns:3,values:Array.from(covariance)},
});
const strips=program=>Object.fromEntries(program.scene().line_strips.map(strip=>[strip.id,strip]));
const center=(drawing,id)=>drawing[id+'-heading'].positions[0];
const radius=(drawing,id)=>drawing[id].positions[0][0]-center(drawing,id)[0];
const close=(actual,expected,message)=>assert(Math.abs(actual-expected)<1e-9,message);
const numbers=(program,name)=>Array.from(program.readNumbers(name));
const kernelInputs=program=>Object.fromEntries(['control','cameras','measurements'].map(name=>[name,Float32Array.from(program.readNumbers(name))]));
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
  assert.equal(initial['truth-heading'].stroke,'#46968a');
  assert.equal(initial['estimate-heading'].stroke,'#ac8637');

  const cameras=[[20,110],[180,110],[180,20],[20,20]];
  for(let index=1;index<=4;index++) {
    const ring=program.scene().circles.find(circle=>circle.id===`camera-range-${index}`);
    assert.deepEqual([ring.x,ring.y],cameras[index-1],'each sensing circle is centered on its fixed camera');
    assert.equal(ring.radius,100);
    assert(ring.opacity>0);
  }
  assert.deepEqual(numbers(program,'measurement-visible'),[1,0,0,1],'all cameras within range are available together');
  assert.equal(numbers(program,'measurements').length,24,'two lanes contain four range/bearing/visibility triples each');
  program.turn({commit:0,noise:0});
  const clean=numbers(program,'measurements'),candidate=numbers(program,'candidate-truth');
  close(clean[0],Math.hypot(candidate[0]-20,candidate[1]-20),'range comes from fixed camera to true robot');
  close(clean[1],Math.atan2(candidate[1]-20,candidate[0]-20),'bearing is world-referenced, not robot-heading-relative');
  assert.deepEqual(clean.slice(0,12),clean.slice(12),'zero sensor noise gives identical observations across lanes');
  program.turn({commit:0,noise:1});
  assert.notDeepEqual(numbers(program,'measurements'),clean,'sensor noise changes the actual input observations');
  assert.deepEqual(numbers(program,'candidate-truth'),candidate,'sensor noise cannot move the true robot');
  program.turn({commit:0,'motion-noise':1});
  assert.notDeepEqual(numbers(program,'candidate-truth'),candidate,'simulation noise changes the actual plant motion');
  assert.deepEqual(numbers(program,'control'),[0.1,1,0.015],'filter still receives commanded motion');
  assert.deepEqual(numbers(program,'truth'),[55,25,0.4],'preparation never advances true pose');
  program.turn({commit:0,'motion-noise':0,'camera-pulses':[1,0,0,0]});
  assert.deepEqual(numbers(program,'camera-enabled'),[0,1,1,1]);
  assert.deepEqual(numbers(program,'measurement-visible'),[0,0,0,1],'camera press removes its actual EKF measurement');
  assert.equal(program.scene().circles.find(circle=>circle.id==='camera-range-1').opacity,0,'disabled camera hides range circle');
  assert.equal(program.scene().circles.find(circle=>circle.id==='camera-1').fill,'#687780','disabled camera is gray');
  program.turn({commit:0,'camera-pulses':[2,0,0,0]});
  assert.deepEqual(numbers(program,'camera-enabled'),[1,1,1,1],'a second press restores the same camera');
  const boundary=numbers(program,'camera-distance')[0];
  program.turn({commit:0,'camera-range':boundary});
  assert.equal(numbers(program,'measurement-visible')[0],1,'range boundary is included');
  program.turn({commit:0,'camera-range':boundary-0.001});
  assert.equal(numbers(program,'measurement-visible')[0],0,'outside range supplies no correction');
  program.turn({commit:0,'camera-range':100});

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
  close(center(accepted,'truth')[0],55+0.1*Math.cos(0.4+0.015*0.1/2),'true x follows midpoint plant motion');
  close(center(accepted,'truth')[1],105-0.1*Math.sin(0.4+0.015*0.1/2),'true y follows midpoint plant motion');
  program.turn({commit:0});
  for(const id of ['truth','estimate'])
    assert.deepEqual(strips(program)[id+'-path'],accepted[id+'-path'],'uncommitted turn retains '+id+' history');
  program.stop();program.free();program=null;

  for(const [edge,x,y,heading] of [['right',199.99,65,0],['left',0.01,65,Math.PI],
    ['top',100,129.99,Math.PI/2],['bottom',100,0.01,-Math.PI/2]]) {
    const wrappedSource=source.replace("~truth := [55.0 25.0 0.4]'",`~truth := [${x} ${y} ${heading}]'`)
      .replaceAll('[55.0 105.0]',`[${x} ${130-y}]`);
    const inputs={...sceneInputs([x,y,heading],[100,0,0,0,100,0,0,0,0.15]),omega:0,noise:0};
    program=WasmSceneProgram.fromSource(wrappedSource,inputs);
    program.turn({commit:0});
    const expected=[((x+0.1*Math.cos(heading))%200+200)%200,
      ((y+0.1*Math.sin(heading))%130+130)%130,heading];
    const candidate=numbers(program,'candidate-truth');
    candidate.forEach((value,index)=>close(value,expected[index],edge+' wrap has expected pose'));
    const measurements=numbers(program,'measurements');
    close(measurements[0],Math.hypot(candidate[0]-20,candidate[1]-20),edge+' camera observes wrapped location');
    program.turn({commit:1,estimate:candidate});
    const drawing=strips(program);
    assert.deepEqual(numbers(program,'truth'),candidate,edge+' true position commits the wrapped candidate');
    for(const id of ['truth','estimate']) {
      const tail=center(drawing,id);
      for(const point of drawing[id+'-path'].positions)
        point.forEach((value,index)=>close(value,tail[index],edge+' '+id+' trail restarts without a field diagonal'));
    }
    assert.equal(numbers(program,'truth')[2],heading,'edge crossing preserves continuous heading');
    program.stop();program.free();program=null;
  }

  program=WasmSceneProgram.fromSource(source,sceneInputs([55,25,0.4],[100,0,0,0,100,0,0,0,0.15]));
  program.turn({commit:0});
  kernel=WasmKernel.fromSource(readFileSync(new URL('source/camera-ekf.mec',import.meta.url),'utf8'),kernelInputs(program),['μ','Σ']);
  for(let turn=0;turn<12;turn++) {
    program.turn({commit:0});
    kernel.turn(kernelInputs(program));
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

// Exercise the actual host queue against the real scene runtime. Rendering is
// covered by the browser suite; no simulation or enable mask is mocked here.
const runtimeUrl=new URL('../../../src/wasm/pkg/mech_wasm.js',import.meta.url).href;
const bridgeSource=bridge.replace("'./runtime.mjs'",JSON.stringify(runtimeUrl));
const {MechScene}=await import('data:text/javascript;base64,'+Buffer.from(bridgeSource).toString('base64'));
const controls={velocity:1,omega:0.015,noise:1,motionNoise:1,range:100};
const host=new MechScene(source,2,[55,25,0.4],[100,0,0,0,100,0,0,0,0.15],controls);
host.draw=()=>{};
try {
  host.toggleCamera(1);
  assert.deepEqual(numbers(host.program,'camera-enabled'),[1,1,1,1],'pointer press queues without mutating prepared state');
  const prepared=host.observation(controls);
  assert.equal(prepared.inputs.measurements[2],0,'next preparation consumes the queued press');
  host.toggleCamera(2);host.toggleCamera(2);host.toggleCamera(4);
  host.accepted([55.1,25.1,0.4],[100,0,0,0,100,0,0,0,0.15]);
  assert.deepEqual(numbers(host.program,'camera-enabled'),[0,1,1,1],'acceptance uses its frozen preparation mask');
  const truth=numbers(host.program,'truth'),path=numbers(host.program,'truth-path');
  host.refresh(controls);
  assert.deepEqual(numbers(host.program,'camera-enabled'),[0,1,1,0],'refresh consumes pending presses with even parity canceled');
  assert.deepEqual(numbers(host.program,'truth'),truth,'queued camera refresh does not advance truth');
  assert.deepEqual(numbers(host.program,'truth-path'),path,'queued camera refresh does not advance path');
  const invalid=host.observation(controls,true).inputs.measurements;
  assert(Number.isNaN(invalid[13]),'fault injection targets last lane camera-one bearing');
} finally {host.dispose();}

for(const instances of [65_536,4097]) {
  const packed=new MechScene(source,instances,[55,25,0.4],[100,0,0,0,100,0,0,0,0.15],controls);
  packed.draw=()=>{};
  let reference;
  try {
    const measured=packed.observation(controls).inputs.measurements;
    assert.equal(measured.length,12*instances,'packetization preserves the full numerical batch');
    assert.equal(numbers(packed.program,'lanes').length,4096,'sensor workspace stays at the bounded packet size');
    assert.equal(numbers(packed.program,'accepted-turns')[0],0,'sensor packets do not advance the simulation');
    const indices=[1,4096,4097,instances];
    reference=WasmSceneProgram.fromSource(source,{
      ...sceneInputs([55,25,0.4],[100,0,0,0,100,0,0,0,0.15]),
      'motion-noise':controls.motionNoise,'lane-indices':Float64Array.from(indices),
    });
    reference.turn({commit:0});
    const expected=Float32Array.from(reference.readNumbers('measurements'));
    indices.forEach((index,row)=>assert.deepEqual(measured.slice((index-1)*12,index*12),
      expected.slice(row*12,(row+1)*12),'global lane index '+index+' retains its exact Mech noise and camera readings'));
    assert.notDeepEqual(measured.slice(0,12),measured.slice(-12),'last lane retains independent noise, not a repeated packet');
    const candidate=numbers(packed.program,'candidate-truth');
    packed.accepted(candidate,[100,0,0,0,100,0,0,0,0.15]);
    assert.equal(numbers(packed.program,'accepted-turns')[0],1,'one complete filter batch commits the simulation once');
    assert.deepEqual(numbers(packed.program,'truth'),candidate);
  } finally {reference?.stop();reference?.free();packed.dispose();}
}
console.log('PASS: fixed cameras, independent noise, queued presses, four edge wraps, EKF pose/path updates, and exact 65,536-lane sensor packetization.');
