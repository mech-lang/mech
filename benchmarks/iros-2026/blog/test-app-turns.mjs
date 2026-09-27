import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

// Run the production turn coordinator with deterministic host failures. These
// tests exercise publication bookkeeping, not a replacement numerical kernel.
const source=readFileSync(new URL('app.mjs',import.meta.url),'utf8');
const begin=source.indexOf('async function turn(');
const end=source.indexOf('async function loop(',begin);
const loopEnd=source.indexOf('\nfor (const [id,label]',end);
const controlsBegin=source.indexOf('function controls()');
const controlsEnd=source.indexOf('function error(',controlsBegin);
const compileBegin=source.indexOf('async function compile()');
const sensorsBegin=source.indexOf('function sensorControls()');
const verifyBegin=source.indexOf("$('verify').onclick=async()=>{");
const verifyEnd=source.indexOf('\n\ntry {',verifyBegin);
assert(begin>=0&&end>begin&&loopEnd>end);
assert(controlsBegin>=0&&controlsEnd>controlsBegin&&sensorsBegin>controlsEnd&&sensorsBegin<begin);
assert(compileBegin>controlsEnd&&compileBegin<sensorsBegin&&verifyEnd>verifyBegin);
const sensorIds=['velocity','omega','noise','motion-noise','camera-range'];
const guardedIds=['run','step','reset','instances','backend','verify'];
// Production click/frame handlers intentionally use void turn(). Drain only
// their deterministic promise continuations, never another animation frame.
async function settleMicrotasks(){for(let tick=0;tick<8;tick++)await Promise.resolve();}
function harness({failure=null,gpu=false,finishGate=null,verificationGate=null}={}) {
  const messages=new Map();
  const animationFrames=[];
  const elements=new Map([...sensorIds,...guardedIds,'inject','pause','verification'].map(id=>[id,{disabled:false,value:'0',textContent:'',dataset:{}}]));
  for(const [id,value] of Object.entries({velocity:20,omega:.2,noise:1,'motion-noise':1,'camera-range':100}))elements.get(id).value=String(value);
  const context=vm.createContext({
    MEAN:'μ',COVARIANCE:'Σ',Float32Array,performance:{now:()=>10},
    state:new Float32Array([55,25,.4]),covariance:new Float32Array(9),
    busy:false,turnInFlight:false,injectionPending:false,ready:true,mode:'patrol',running:true,generation:0,accepted:0,rejected:0,
    active:0,samples:[],frames:[],faults:0,kernelTurns:0,sceneCommits:0,observations:[],invalidObservations:[],observationPackets:[],submittedPackets:[],sceneRefreshPending:false,sceneRefreshes:[],
    requestAnimationFrame:callback=>animationFrames.push(callback),
    $:id=>elements.get(id),telemetry(){if(failure==='telemetry')throw new Error('telemetry failed');},
    text(id,value){messages.set(id,value);if(elements.has(id))elements.get(id).textContent=String(value);},
    error:value=>messages.set('error',value),
    transition(event){assert(['rejected','pause','run'].includes(event));context.mode={rejected:'fault',pause:'paused',run:'patrol'}[event];},
    rowMajor:values=>values,
    source:'fixture',sceneSource:'fixture',committedBackend:'cpu',committedInstances:'4096',
    dispose:async()=>{},WasmKernel:{},
    MechScene:class {constructor(){throw new Error('fixture compilation stopped');}},
    async verifyKernel(){if(verificationGate)await verificationGate;return {status:'passed'};},
    scene:{
      observation(controls,invalid=false){
        if(failure==='prepare')throw new Error('sensor failed');
        context.observations.push({...controls});context.invalidObservations.push(invalid);
        const inputs={measurements:new Float32Array([10,invalid?NaN:.25,1])};
        context.observationPackets.push(inputs);return {inputs};
      },
      accepted(){context.sceneCommits++;if(failure==='scene')throw new Error('scene failed');},
      refresh(controls){context.sceneRefreshes.push({...controls});},
    },
    kernel:{
      faultCount:()=>context.faults,
      gpuInputs:inputs=>inputs,
      turn(inputs){
        context.submittedPackets.push(inputs);
        if(failure==='integrity'||Number.isNaN(inputs.measurements[1])){context.faults++;throw new Error('integrity failed');}
        if(failure==='execute')throw new Error('execution failed');
        context.kernelTurns++;
      },
      stateSample(name){if(failure==='readback')throw new Error('readback failed');return name==='μ'?new Float32Array([1,2,3]):new Float32Array(9);},
    },
    manifest:{exports:[{name:'μ',outputName:'mean'},{name:'Σ',outputName:'covariance'}]},
    device:null,
  });
  if(gpu)context.device={
    submit:({inputs},active)=>{context.submittedPackets.push(inputs);return {outputIndex:1-active,inputs};},
    async finish(submission){
      if(finishGate)await finishGate;
      if(failure==='execute')throw new Error('device failed');
      if(failure==='integrity'||Number.isNaN(submission.inputs.measurements[1]))return {integrity:{constraint:'finite-candidate!',instance:7},outputs:[]};
      context.kernelTurns++;
      return {outputs:failure==='readback'?[]:[{name:'mean',values:[1,2,3]},{name:'covariance',values:Array(9).fill(0)}]};
    },
  };
  const bindings=['inject','pause','run'].map(id=>{
    const start=source.indexOf(`$('${id}').onclick=`);
    const finish=source.indexOf("\n$('",start);
    assert(start>=0&&finish>start,`missing production ${id} handler`);
    return source.slice(start,finish);
  }).join('\n');
  vm.runInContext(source.slice(controlsBegin,controlsEnd)+source.slice(compileBegin,loopEnd)+bindings+'\n'+source.slice(verifyBegin,verifyEnd),context);
  return {context,messages,elements,animationFrames,
    run:()=>vm.runInContext('turn()',context),loop:()=>vm.runInContext('loop(generation)',context),
    compile:()=>vm.runInContext('compile()',context),refresh:()=>vm.runInContext('controls()',context),
    clickInject:()=>elements.get('inject').onclick(),pause:()=>elements.get('pause').onclick(),
    verify:()=>elements.get('verify').onclick(),
    async flushFrame(){assert(animationFrames.length,'expected a queued animation frame');await animationFrames.shift()(16);await settleMicrotasks();},
  };
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
  const {context:c,elements,refresh}=harness();
  refresh();
  for(const id of ['run','step','reset','instances','backend','verify'])
    assert.equal(elements.get(id).disabled,true,`${id} stays locked between active turns`);
  for(const id of [...sensorIds,'pause'])assert.equal(elements.get(id).disabled,false,`${id} stays live while running`);
  c.running=false;c.mode='paused';refresh();
  for(const id of [...guardedIds,'inject'])assert.equal(elements.get(id).disabled,false,`${id} is available when paused`);
  assert.equal(elements.get('pause').disabled,true);
}
{
  let finish;
  const finishGate=new Promise(resolve=>{finish=resolve;});
  const {context:c,elements,run,refresh}=harness({gpu:true,finishGate});
  const pending=run();
  assert.equal(c.busy,true);
  assert.equal(c.turnInFlight,true);
  for(const id of sensorIds)assert.equal(elements.get(id).disabled,false,`${id} stays adjustable during a turn`);
  for(const id of guardedIds)assert.equal(elements.get(id).disabled,true,`${id} cannot overlap a turn`);
  assert.equal(elements.get('inject').disabled,false,'injection remains available while a numerical GPU turn finishes');
  const original={velocity:20,omega:.2,noise:1,motionNoise:1,range:100};
  assert.deepEqual(c.observations,[original]);
  for(const [id,value] of Object.entries({velocity:35,omega:-.3,noise:2,'motion-noise':3,'camera-range':250}))elements.get(id).value=String(value);
  c.sceneRefreshPending=true;
  assert.deepEqual(c.observations,[original],'in-flight observation retains its complete input snapshot');
  assert.equal(c.sceneCommits,0);
  // Pausing while a GPU turn finishes also must not disable an active drag.
  c.running=false;c.mode='paused';refresh();
  for(const id of sensorIds)assert.equal(elements.get(id).disabled,false);
  finish();await pending;
  assert.equal(c.accepted,1);
  assert.equal(c.sceneCommits,1);
  assert.deepEqual(c.sceneRefreshes,[{velocity:35,omega:-.3,noise:2,motionNoise:3,range:250}],'queued scene controls refresh after the accepted commit');
  assert.equal(c.sceneRefreshPending,false);
  await run();
  assert.deepEqual(c.observations,[original,{velocity:35,omega:-.3,noise:2,motionNoise:3,range:250}]);
  assert.equal(c.accepted,2);
  assert.equal(c.sceneCommits,2);
  // Compile/reset/backend changes and verification still lock sensor controls.
  c.busy=true;refresh();
  for(const id of [...sensorIds,...guardedIds,'inject'])assert.equal(elements.get(id).disabled,true);
  c.busy=false;c.ready=false;refresh();
  for(const id of [...sensorIds,...guardedIds,'inject'])assert.equal(elements.get(id).disabled,true);
}

// Running clicks queue one invalid observation for the next loop turn. The
// click handler itself must not start an overlapping or extra numerical turn.
for(const gpu of [false,true]) {
  const {context:c,elements,animationFrames,clickInject,loop,refresh}=harness({gpu});
  refresh();assert.equal(elements.get('inject').disabled,false);
  clickInject();clickInject();clickInject();
  assert.equal(c.injectionPending,true);
  assert.equal(elements.get('inject').disabled,true,'a queued request coalesces repeated clicks');
  assert.match(elements.get('inject').textContent,/pending|queued/i,'pending request is visible in the button label');
  assert.equal(c.observations.length,0,'clicking while running does not bypass the loop');
  await loop();
  assert.deepEqual(c.invalidObservations,[true]);
  assert.equal(c.injectionPending,false,'the turn consumes the request exactly once');
  assert.equal(c.accepted,0);assert.equal(c.rejected,1);assert.equal(c.sceneCommits,0);
  assert.equal(c.mode,'fault');assert.equal(c.running,false);
  assert.equal(animationFrames.length,0,'a rejected turn cannot schedule another loop');
  clickInject();assert.equal(c.injectionPending,false,'faulted episodes cannot queue injections');
}

// A click during an awaited GPU turn must preserve that exact submitted
// observation, then reject the following turn. Pause must not strand the queue.
for(const pauseDuringTurn of [false,true]) {
  let finish;
  const finishGate=new Promise(resolve=>{finish=resolve;});
  const {context:c,elements,animationFrames,clickInject,loop,pause,flushFrame}=harness({gpu:true,finishGate});
  const pending=loop();
  const submitted=c.submittedPackets[0];
  const original=Array.from(submitted.measurements);
  assert.equal(c.turnInFlight,true);assert.equal(elements.get('inject').disabled,false);
  clickInject();clickInject();
  assert.equal(c.injectionPending,true);
  assert.deepEqual(Array.from(submitted.measurements),original,'the in-flight packet cannot become invalid retroactively');
  assert.deepEqual(c.invalidObservations,[false]);
  assert.equal(c.submittedPackets.length,1,'injection cannot submit concurrently');
  if(pauseDuringTurn)pause();
  finish();await pending;
  assert.equal(c.accepted,1);assert.equal(c.rejected,0);assert.equal(c.sceneCommits,1);
  assert.equal(c.injectionPending,true);
  assert.equal(animationFrames.length,1,'the regular loop or paused-drain path schedules exactly one next turn');
  await flushFrame();
  assert.deepEqual(c.invalidObservations,[false,true]);
  assert.equal(c.accepted,1);assert.equal(c.rejected,1);assert.equal(c.sceneCommits,1);
  assert.equal(c.kernelTurns,1);assert.equal(c.active,1,'rejection retains the accepted GPU buffer');
  assert.equal(c.injectionPending,false);assert.equal(animationFrames.length,0);
  assert.deepEqual(Array.from(submitted.measurements),original);
}

// An idle paused click runs immediately, without waiting for a Run action or
// scheduling a frame; GPU completion remains asynchronous in that case.
for(const gpu of [false,true]) {
  const {context:c,animationFrames,clickInject,refresh}=harness({gpu});
  c.running=false;c.mode='paused';refresh();
  clickInject();
  assert.deepEqual(c.invalidObservations,[true],'paused idle injection submits immediately');
  await settleMicrotasks();
  assert.equal(c.rejected,1);assert.equal(c.accepted,0);assert.equal(c.injectionPending,false);
  assert.equal(animationFrames.length,0);
}

// Pause between frames cancels the old run loop but must consume an already
// queued request immediately instead of leaving it attached to that old token.
for(const gpu of [false,true]) {
  const {context:c,animationFrames,clickInject,loop,pause,flushFrame}=harness({gpu});
  await loop();
  assert.equal(animationFrames.length,1);assert.equal(c.accepted,1);
  clickInject();assert.equal(c.injectionPending,true);
  pause();
  assert.deepEqual(c.invalidObservations,[false,true]);
  await settleMicrotasks();
  assert.equal(c.injectionPending,false);assert.equal(c.rejected,1);assert.equal(c.accepted,1);
  await flushFrame();
  assert.deepEqual(c.invalidObservations,[false,true],'the cancelled run frame cannot submit a duplicate');
  assert.equal(animationFrames.length,0);
}

// Reset/compilation can cancel the paused-drain frame after GPU completion.
// The stale callback must check the request again before invoking turn().
{
  let finish;
  const finishGate=new Promise(resolve=>{finish=resolve;});
  const {context:c,animationFrames,run,clickInject,pause,compile,flushFrame}=harness({gpu:true,finishGate});
  const pending=run();clickInject();pause();finish();await pending;
  assert.equal(c.injectionPending,true);assert.equal(animationFrames.length,1);
  const staleDrain=animationFrames.shift();
  const compilation=compile();
  assert.equal(c.injectionPending,false);assert.equal(animationFrames.length,1);
  await flushFrame();await compilation;
  assert.equal(c.busy,false);
  staleDrain();await settleMicrotasks();
  assert.deepEqual(c.invalidObservations,[false],'Reset cancels even a drain callback delivered after compilation finishes');
  assert.equal(c.accepted,1);assert.equal(c.rejected,0);assert.equal(animationFrames.length,0);
}

// Any turn failure discards a request queued while that turn was in flight.
{
  let finish;
  const finishGate=new Promise(resolve=>{finish=resolve;});
  const {context:c,animationFrames,run,clickInject}=harness({gpu:true,finishGate,failure:'execute'});
  const pending=run();clickInject();assert.equal(c.injectionPending,true);
  finish();await pending;
  assert.equal(c.injectionPending,false);assert.equal(animationFrames.length,0);
  assert.equal(c.mode,'fault');assert.equal(c.rejected,0);
}

// Compilation clears stale requests and, like verification, must not accept
// new ones. Invoke the actual production entry points and click binding.
{
  const {context:c,elements,compile,clickInject,flushFrame}=harness();
  c.injectionPending=true;
  const pending=compile();
  assert.equal(c.busy,true);assert.equal(c.turnInFlight,false);
  assert.equal(c.injectionPending,false,'compilation clears a stale queued request');
  assert.equal(elements.get('inject').disabled,true);
  clickInject();assert.equal(c.injectionPending,false);
  await flushFrame();await pending;
  assert.equal(c.observations.length,0);assert.equal(c.kernelTurns,0);
}
{
  let finish;
  const verificationGate=new Promise(resolve=>{finish=resolve;});
  const {context:c,elements,verify,clickInject}=harness({verificationGate});
  const pending=verify();
  assert.equal(c.busy,true);assert.equal(c.turnInFlight,false);
  assert.equal(elements.get('inject').disabled,true);
  clickInject();assert.equal(c.injectionPending,false);
  finish();await pending;
  assert.equal(c.observations.length,0);assert.equal(c.kernelTurns,0);
  c.ready=false;clickInject();assert.equal(c.injectionPending,false,'unready hosts cannot queue injections');
}
console.log('PASS: CPU/GPU publication counts, rollback, Reset requirement, live sensor snapshots, coalesced fault injection, immutable in-flight observations, paused GPU queue draining, and compile/verification guards.');
