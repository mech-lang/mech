import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

// Run the production turn coordinator with deterministic host failures. These
// tests exercise publication bookkeeping, not a replacement numerical kernel.
const source=readFileSync(new URL('app.mjs',import.meta.url),'utf8');
const begin=source.indexOf('async function turn(');
const end=source.indexOf('async function loop(',begin);
assert(begin>=0&&end>begin);
function harness({failure=null,gpu=false}={}) {
  const messages=new Map();
  const context=vm.createContext({
    MEAN:'μ',COVARIANCE:'Σ',Float32Array,performance:{now:()=>10},
    state:new Float32Array([55,25,.4]),covariance:new Float32Array(9),
    busy:false,mode:'patrol',running:true,generation:0,accepted:0,rejected:0,
    active:0,samples:[],frames:[],faults:0,kernelTurns:0,sceneCommits:0,
    controls(){},telemetry(){if(failure==='telemetry')throw new Error('telemetry failed');},text:(id,value)=>messages.set(id,value),
    error:value=>messages.set('error',value),sensorControls:()=>({}),
    transition(event){assert.equal(event,'rejected');context.mode='fault';},
    rowMajor:values=>values,
    scene:{
      observation(){if(failure==='prepare')throw new Error('sensor failed');return {inputs:{}};},
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
      if(failure==='execute')throw new Error('device failed');
      if(failure==='integrity')return {integrity:{constraint:'finite-candidate!',instance:7},outputs:[]};
      context.kernelTurns++;
      return {outputs:failure==='readback'?[]:[{name:'mean',values:[1,2,3]},{name:'covariance',values:Array(9).fill(0)}]};
    },
  };
  vm.runInContext(source.slice(begin,end),context);
  return {context,messages,run:()=>vm.runInContext('turn()',context)};
}
for(const gpu of [false,true]) {
  for(const failure of [null,'prepare','integrity','execute','readback','scene','telemetry']) {
    const {context:c,messages,run}=harness({failure,gpu});
    await run();
    assert.equal(c.busy,false);
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
console.log('PASS: CPU/GPU publication counts, integrity rejection, preparation failures, post-acceptance scene/readback errors, and Reset requirement.');
