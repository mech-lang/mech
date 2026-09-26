import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

// Run the production turn coordinator with deterministic host failures. These
// tests exercise publication bookkeeping, not a replacement numerical kernel.
const source=readFileSync(new URL('app.mjs',import.meta.url),'utf8');
const begin=source.indexOf('async function turn(');
const end=source.indexOf('async function loop(',begin);
const controlsBegin=source.indexOf('function controls()');
const controlsEnd=source.indexOf('function error(',controlsBegin);
const sensorsBegin=source.indexOf('function sensorControls()');
assert(begin>=0&&end>begin);
assert(controlsBegin>=0&&controlsEnd>controlsBegin&&sensorsBegin>controlsEnd&&sensorsBegin<begin);
const sensorIds=['velocity','omega','noise','landmark','camera-range'];
const guardedIds=['run','step','inject','reset','instances','backend','verify'];
function harness({failure=null,gpu=false,finishGate=null}={}) {
  const messages=new Map();
  const elements=new Map([...sensorIds,...guardedIds,'pause'].map(id=>[id,{disabled:false,value:'0'}]));
  for(const [id,value] of Object.entries({velocity:20,omega:.2,noise:.01,landmark:1,'camera-range':100}))elements.get(id).value=String(value);
  const context=vm.createContext({
    MEAN:'μ',COVARIANCE:'Σ',Float32Array,performance:{now:()=>10},
    state:new Float32Array([55,25,.4]),covariance:new Float32Array(9),
    busy:false,turnInFlight:false,ready:true,mode:'patrol',running:true,generation:0,accepted:0,rejected:0,
    active:0,samples:[],frames:[],faults:0,kernelTurns:0,sceneCommits:0,observations:[],
    $:id=>elements.get(id),telemetry(){if(failure==='telemetry')throw new Error('telemetry failed');},text:(id,value)=>messages.set(id,value),
    error:value=>messages.set('error',value),
    transition(event){assert.equal(event,'rejected');context.mode='fault';},
    rowMajor:values=>values,
    scene:{
      observation(controls){if(failure==='prepare')throw new Error('sensor failed');context.observations.push({...controls});return {inputs:{}};},
      accepted(){context.sceneCommits++;if(failure==='scene')throw new Error('scene failed');},
    },
    kernel:{
      faultCount:()=>context.faults,
      gpuInputs:inputs=>inputs,
      turn(){
        if(failure==='integrity'){context.faults++;throw new Error('integrity failed');}
        if(failure==='execute')throw new Error('execution failed');
        context.kernelTurns++;
      },
      stateSample(name){if(failure==='readback')throw new Error('readback failed');return name==='μ'?new Float32Array([1,2,3]):new Float32Array(9);},
    },
    manifest:{exports:[{name:'μ',outputName:'mean'},{name:'Σ',outputName:'covariance'}]},
    device:null,
  });
  if(gpu)context.device={
    submit:()=>({outputIndex:1}),
    async finish(){
      if(finishGate)await finishGate;
      if(failure==='execute')throw new Error('device failed');
      if(failure==='integrity')return {integrity:{constraint:'finite-candidate!',instance:7},outputs:[]};
      context.kernelTurns++;
      return {outputs:failure==='readback'?[]:[{name:'mean',values:[1,2,3]},{name:'covariance',values:Array(9).fill(0)}]};
    },
  };
  vm.runInContext(source.slice(controlsBegin,controlsEnd)+source.slice(sensorsBegin,end),context);
  return {context,messages,elements,run:()=>vm.runInContext('turn()',context),refresh:()=>vm.runInContext('controls()',context)};
}
for(const gpu of [false,true]) {
  for(const failure of [null,'prepare','integrity','execute','readback','scene','telemetry']) {
    const {context:c,messages,run}=harness({failure,gpu});
    await run();
    assert.equal(c.busy,false);
    assert.equal(c.turnInFlight,false);
    assert.equal(c.rejected,failure==='integrity'?1:0,`${gpu}/${failure}: only integrity failures count as rejected numerical turns`);
    const committed=failure===null||failure==='readback'||failure==='scene'||failure==='telemetry';
    assert.equal(c.accepted,committed?1:0,`${gpu}/${failure}: accepted numerical turns retain their count`);
    assert.equal(c.kernelTurns,committed?1:0);
    if(gpu)assert.equal(c.active,committed?1:0);
    if(failure){
      assert.equal(c.mode,'fault');
      assert.equal(c.running,false);
      if(failure!=='integrity'){
        assert.match(messages.get('error'),/Reset is required/);
        assert.match(messages.get('behavior-message'),/resynchronize/);
      }
      if(committed)assert.match(messages.get('error'),/numerical state was not rolled back/);
      if(failure==='prepare')assert.match(messages.get('error'),/before the EKF was submitted/);
      if(failure==='execute')assert.match(messages.get('error'),/publication status is unavailable/);
      const counts=[c.accepted,c.rejected,c.kernelTurns];
      await run();
      assert.deepEqual([c.accepted,c.rejected,c.kernelTurns],counts,'a faulted episode requires Reset');
    }
  }
}

// Native range dragging must survive every turn's control refresh, including
// the asynchronous GPU wait. Changes during that wait feed one later snapshot.
{
  let finish;
  const finishGate=new Promise(resolve=>{finish=resolve;});
  const {context:c,elements,run,refresh}=harness({gpu:true,finishGate});
  const pending=run();
  assert.equal(c.busy,true);
  assert.equal(c.turnInFlight,true);
  for(const id of sensorIds)assert.equal(elements.get(id).disabled,false,`${id} stays adjustable during a turn`);
  for(const id of guardedIds)assert.equal(elements.get(id).disabled,true,`${id} cannot overlap a turn`);
  const original={velocity:20,omega:.2,noise:.01,landmark:1,range:100};
  assert.deepEqual(c.observations,[original]);
  for(const [id,value] of Object.entries({velocity:35,omega:-.3,noise:.04,landmark:2,'camera-range':250}))elements.get(id).value=String(value);
  assert.deepEqual(c.observations,[original],'in-flight observation retains its complete input snapshot');
  assert.equal(c.sceneCommits,0);
  // Pausing while a GPU turn finishes also must not disable an active drag.
  c.running=false;c.mode='paused';refresh();
  for(const id of sensorIds)assert.equal(elements.get(id).disabled,false);
  finish();await pending;
  assert.equal(c.accepted,1);
  assert.equal(c.sceneCommits,1);
  await run();
  assert.deepEqual(c.observations,[original,{velocity:35,omega:-.3,noise:.04,landmark:2,range:250}]);
  assert.equal(c.accepted,2);
  assert.equal(c.sceneCommits,2);
  // Compile/reset/backend changes and verification still lock sensor controls.
  c.busy=true;refresh();
  for(const id of [...sensorIds,...guardedIds])assert.equal(elements.get(id).disabled,true);
  c.busy=false;c.ready=false;refresh();
  for(const id of [...sensorIds,...guardedIds])assert.equal(elements.get(id).disabled,true);
}
console.log('PASS: CPU/GPU publication counts, integrity rejection, preparation failures, post-acceptance scene/readback errors, Reset requirement, and live sensor-control snapshots.');
