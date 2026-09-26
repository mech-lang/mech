#!/usr/bin/env node
// Independent double-precision reference checks the real compiled WASM kernel.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL, fileURLToPath} from 'node:url';
import {resolve} from 'node:path';

const root=fileURLToPath(new URL('../../../',import.meta.url));
const modulePath=resolve(process.argv[2]??`${root}/src/wasm/pkg/mech_wasm.js`);
const binaryPath=resolve(process.argv[3]??`${root}/src/wasm/pkg/mech_wasm_bg.wasm`);
const {default:initialize,WasmKernel}=await import(pathToFileURL(modulePath));
await initialize({module_or_path:readFileSync(binaryPath)});
const source=readFileSync(new URL('./source/camera-ekf.mec',import.meta.url),'utf8');
const names=['μ','Σ'];
const cameras=[20,20,180,20,180,110,20,110];
const initialMean=[55,25,.4];
const initialCovariance=[[100,0,0],[0,100,0],[0,0,.15]];
const bits=a=>new Uint32Array(a.buffer,a.byteOffset,a.length);
const same=(a,b)=>a.length===b.length&&bits(a).every((v,i)=>v===bits(b)[i]);
const clone=value=>value.map(row=>row.slice());
const transpose=a=>a[0].map((_,j)=>a.map(row=>row[j]));
const multiply=(a,b)=>a.map(row=>b[0].map((_,j)=>row.reduce((sum,x,k)=>sum+x*b[k][j],0)));
const add=(a,b)=>a.map((row,i)=>row.map((x,j)=>x+b[i][j]));
const subtract=(a,b)=>a.map((row,i)=>row.map((x,j)=>x-b[i][j]));
const diagonal=values=>values.map((x,i)=>values.map((_,j)=>i===j?x:0));
const scale=(a,s)=>a.map(row=>row.map(x=>x*s));
const symmetrize=a=>scale(add(a,transpose(a)),.5);
const column=a=>a.map(x=>[x]);
const flattened=a=>transpose(a).flat();
const wrap=(value,extent)=>((value%extent)+extent)%extent;
const wrapMean=mean=>[wrap(mean[0],200),wrap(mean[1],130),mean[2]];

function reference(mean,covariance,inputs,lane=0) {
  const [dt,v,omega]=inputs.control;
  const theta=mean[2]+omega*dt/2,c=Math.cos(theta),s=Math.sin(theta),d=v*dt;
  let mu=wrapMean([mean[0]+d*c,mean[1]+d*s,mean[2]+omega*dt]);
  const G=[[1,0,-d*s],[0,1,d*c],[0,0,1]];
  const V=[[c*dt,-d*s*dt/2],[s*dt,d*c*dt/2],[0,dt]];
  let P=add(multiply(multiply(G,covariance),transpose(G)),multiply(multiply(V,diagonal([.08,.018])),transpose(V)));
  for(let camera=0;camera<4;camera++) {
    const offset=lane*12+camera*3;
    if(!inputs.measurements[offset+2]) { P=symmetrize(P); continue; }
    const observed=[inputs.cameras[camera*2]+inputs.measurements[offset]*Math.cos(inputs.measurements[offset+1]),
      inputs.cameras[camera*2+1]+inputs.measurements[offset]*Math.sin(inputs.measurements[offset+1])];
    mu=[mu[0]-200*Math.ceil((mu[0]-observed[0])/200-.5),
      mu[1]-130*Math.ceil((mu[1]-observed[1])/130-.5),mu[2]];
    const dx=mu[0]-inputs.cameras[camera*2],dy=mu[1]-inputs.cameras[camera*2+1];
    const q=dx*dx+dy*dy,r=Math.sqrt(q);
    const H=[[dx/r,dy/r,0],[-dy/q,dx/q,0]],R=diagonal([.0225,.0004]);
    const S=add(multiply(multiply(H,P),transpose(H)),R);
    const determinant=S[0][0]*S[1][1]-S[0][1]*S[1][0];
    const inverse=scale([[S[1][1],-S[0][1]],[-S[1][0],S[0][0]]],1/determinant);
    const K=multiply(multiply(P,transpose(H)),inverse);
    const angle=inputs.measurements[offset+1]-Math.atan2(dy,dx);
    const innovation=column([inputs.measurements[offset]-r,Math.atan2(Math.sin(angle),Math.cos(angle))]);
    mu=add(column(mu),multiply(K,innovation)).flat();
    const A=subtract(diagonal([1,1,1]),multiply(K,H));
    P=symmetrize(add(multiply(multiply(A,P),transpose(A)),multiply(multiply(K,R),transpose(K))));
  }
  return {mean:wrapMean(mu),covariance:P};
}

function packet({turn=1,instances=4,mask=[1,1,1,1],noise=1,control=[.1,1,.015],positions=cameras}={}) {
  const truth=[55+.1*turn*Math.cos(.4),25+.1*turn*Math.sin(.4)];
  const measurements=new Float32Array(instances*12);
  for(let lane=0;lane<instances;lane++) for(let camera=0;camera<4;camera++) {
    const dx=truth[0]-positions[camera*2],dy=truth[1]-positions[camera*2+1],offset=lane*12+camera*3;
    measurements[offset]=Math.hypot(dx,dy)+noise*.11*Math.sin(turn*.157+camera*.7+lane*.037);
    measurements[offset+1]=Math.atan2(dy,dx)+noise*.011*Math.sin(turn*.191+camera+lane*.019);
    measurements[offset+2]=mask[camera];
  }
  return {control,cameras:positions,measurements};
}
const create=(inputs=packet(),text=source)=>WasmKernel.fromSource(text,inputs,names);
function close(reference,actual,message,absolute=3e-4,relative=2e-4) {
  assert.equal(actual.length,reference.length,message);
  reference.forEach((value,i)=>assert(Number.isFinite(actual[i])&&Math.abs(value-actual[i])<=absolute+relative*Math.abs(value),`${message}[${i}]: ${actual[i]} vs ${value}`));
}
function integrity(kernel) {
  for(const value of kernel.state('μ')) assert(Number.isFinite(value));
  const covariance=kernel.state('Σ'),words=bits(covariance);
  for(let base=0;base<covariance.length;base+=9) {
    for(let i=0;i<9;i++) assert(Number.isFinite(covariance[base+i]));
    for(const i of [0,4,8]) assert(covariance[base+i]>0);
    for(const [i,j] of [[1,3],[2,6],[5,7]]) assert.equal(words[base+i],words[base+j],'published covariance is bitwise symmetric');
  }
}
function retained(kernel,before) { names.forEach((name,i)=>assert(same(kernel.state(name),before[i]),`rejected ${name} must be retained bitwise`)); }

const report={sourceSha256:createHash('sha256').update(source).digest('hex'),wasm:true,shaderGenerated:false,individualCameraContributions:0,mixedTurns:0,predictionOnlyTurns:0,noiseLevels:[],rollbackInputs:[]};
const kernel=create();
try {
  assert.equal(kernel.instances(),4);
  assert.equal(kernel.stateWidth('μ'),3);
  assert.equal(kernel.stateWidth('Σ'),9);
  const manifest=kernel.computeManifest();
  assert.equal(manifest.bindings.length,8);
  const shaders=JSON.stringify(manifest);
  assert(shaders.includes('@compute')&&shaders.includes('atan2'));
  report.shaderGenerated=true;
  const references=Array.from({length:4},()=>({mean:initialMean.slice(),covariance:clone(initialCovariance)}));
  for(let turn=1;turn<=200;turn++) {
    const mask=turn%20<4?[0,0,0,0]:[1,turn%3===0?0:1,1,turn%7===0?0:1];
    const noise=turn<=50?0:turn<=100?1:turn<=150?3:.25;
    if(!report.noiseLevels.includes(noise))report.noiseLevels.push(noise);
    const inputs=packet({turn,mask,noise});
    for(let lane=0;lane<4;lane++) references[lane]=reference(references[lane].mean,references[lane].covariance,inputs,lane);
    kernel.turn(inputs);
    for(let lane=0;lane<4;lane++) {
      close(references[lane].mean,kernel.stateSample('μ',lane),`mean turn ${turn} lane ${lane}`);
      close(flattened(references[lane].covariance),kernel.stateSample('Σ',lane),`covariance turn ${turn} lane ${lane}`);
    }
    integrity(kernel);
    if(mask.every(value=>value===0))report.predictionOnlyTurns++;
    report.mixedTurns++;
  }
  const valid=packet({turn:201});
  for(const value of [NaN,Infinity,-Infinity]) {
    const before=names.map(name=>kernel.state(name));
    const invalid={...valid,measurements:valid.measurements.slice()};
    invalid.measurements[3*12+1]=value;
    assert.throws(()=>kernel.turn(invalid),error=>String(error).includes('finite-candidate!')&&/instance: 3/.test(String(error)));
    retained(kernel,before);
    report.rollbackInputs.push(String(value));
  }
  kernel.turn(valid); integrity(kernel);
  report.recovery=true;
} finally { kernel.free(); }

for(let camera=0;camera<4;camera++) {
  const mask=[0,0,0,0];mask[camera]=1;
  const inputs=packet({mask}),single=create(inputs),all=create(packet());
  try {
    single.turn(inputs);all.turn(packet());
    const expected=reference(initialMean,initialCovariance,inputs);
    close(expected.mean,single.stateSample('μ',0),`camera ${camera+1} correction`);
    close(flattened(expected.covariance),single.stateSample('Σ',0),`camera ${camera+1} covariance`);
    const one=single.stateSample('Σ',0),four=all.stateSample('Σ',0);
    assert(four[0]+four[4]<one[0]+one[4],'all four cameras add position information');
    const missing=packet({mask:[1,1,1,1].map((value,i)=>i===camera?0:value)}),without=create(missing);
    try { without.turn(missing);assert(!same(without.stateSample('μ',0),all.stateSample('μ',0)),`camera ${camera+1} must affect all-camera correction`); }
    finally {without.free();}
    report.individualCameraContributions++;
  } finally { single.free();all.free(); }
}

const singular=packet({control:[.1,0,0],mask:[0,0,0,0],positions:[55,25,55,25,55,25,55,25]});
const disabled=create(singular);
try {
  disabled.turn(singular);integrity(disabled);
  close(initialMean,disabled.stateSample('μ',0),'disabled coincident cameras are prediction-only');
  disabled.turn({...singular,cameras:[56,25,56,25,56,25,56,25]});integrity(disabled);
  close(initialMean,disabled.stateSample('μ',0),'disabled cameras at (-1,0) offset are safe');
  const before=names.map(name=>disabled.state(name));
  const enabled={...singular,measurements:singular.measurements.slice()};
  for(let lane=0;lane<4;lane++)enabled.measurements[lane*12+2]=1;
  assert.throws(()=>disabled.turn(enabled),error=>String(error).includes('finite-candidate!'));
  retained(disabled,before);
  report.disabledCoincidentGeometry=true;
  report.disabledNegativeUnitOffset=true;
  report.activeSingularityRejected=true;
} finally { disabled.free(); }

const stationaryPacket=packet({control:[.1,0,0],noise:3}),stationary=create(stationaryPacket);
try {
  stationary.turn(stationaryPacket);
  close([initialMean[2]],[stationary.stateSample('μ',0)[2]],'fixed-camera bearing must not directly observe robot heading',1e-6,0);
  report.noDirectHeadingObservation=true;
} finally { stationary.free(); }

report.edgeCrossings=[];
for(const [name,start] of [['right',[199.95,65,0]],['left',[.05,65,Math.PI]],
                         ['top',[100,129.95,Math.PI/2]],['bottom',[100,.05,-Math.PI/2]]]) {
  for(const noise of [0,1]) for(const visible of [0,1]) {
    const changed=source.replace("~μ<[f32]> := [55 25 0.4]'",`~μ<[f32]> := [${start.join(' ')}]'`);
    const edgeInputs=packet({instances:2,control:[.1,1,0]});
    const edge=create(edgeInputs,changed);
    let truth=start.slice();
    try {
      for(let turn=1;turn<=20;turn++) {
        truth=wrapMean([truth[0]+.1*Math.cos(truth[2]),truth[1]+.1*Math.sin(truth[2]),truth[2]]);
        for(let lane=0;lane<2;lane++) for(let camera=0;camera<4;camera++) {
          const dx=truth[0]-cameras[camera*2],dy=truth[1]-cameras[camera*2+1],offset=lane*12+camera*3;
          edgeInputs.measurements[offset]=Math.hypot(dx,dy)+noise*.11*Math.sin(turn*.157+camera*.7+lane*.037);
          edgeInputs.measurements[offset+1]=Math.atan2(dy,dx)+noise*.011*Math.sin(turn*.191+camera+lane*.019);
          edgeInputs.measurements[offset+2]=visible;
        }
        edge.turn(edgeInputs);integrity(edge);
        const mean=edge.stateSample('μ',0);
        assert(mean[0]>=0&&mean[0]<200&&mean[1]>=0&&mean[1]<130,`${name}: published position wraps`);
        const error=Math.hypot(Math.min(Math.abs(truth[0]-mean[0]),200-Math.abs(truth[0]-mean[0])),
          Math.min(Math.abs(truth[1]-mean[1]),130-Math.abs(truth[1]-mean[1])));
        assert(error<(noise&&visible?2:.01),`${name}: boundary tracking error ${error}`);
      }
      report.edgeCrossings.push({edge:name,noise,visible,turns:20});
    } finally {edge.free();}
  }
}

// Motion bias means truth and estimate can cross a field edge on different
// turns. That phase mismatch exercises the observation's boundary branch.
report.longHorizon=[];
for(const dropout of [false,true]) {
  const inputs=packet({instances:4,control:[.1,6,.03]}),long=create(inputs);
  let truth=initialMean.slice(),maximumError=0,crossings=0;
  try {
    for(let turn=1;turn<=1000;turn++) {
      const time=turn*.1,heading=truth[2]+.03*.99*.05;
      const raw=[truth[0]+6*(.965+.025*Math.sin(time*.73))*.1*Math.cos(heading),
        truth[1]+6*(.965+.025*Math.sin(time*.73))*.1*Math.sin(heading),truth[2]+.03*.99*.1];
      if(raw[0]<0||raw[0]>=200||raw[1]<0||raw[1]>=130)crossings++;
      truth=wrapMean(raw);
      for(let camera=0;camera<4;camera++) for(let lane=0;lane<4;lane++) {
        const dx=truth[0]-cameras[camera*2],dy=truth[1]-cameras[camera*2+1],range=Math.hypot(dx,dy),offset=lane*12+camera*3;
        inputs.measurements[offset]=range+.11*Math.sin(time*1.57+camera*.7+lane*.037);
        inputs.measurements[offset+1]=Math.atan2(dy,dx)+.011*Math.sin(time*1.91+camera+lane*.019);
        inputs.measurements[offset+2]=range<=100&&!(dropout&&turn>=350&&turn<450)?1:0;
      }
      long.turn(inputs);integrity(long);
      const mean=long.stateSample('μ',0),dx=Math.abs(mean[0]-truth[0]),dy=Math.abs(mean[1]-truth[1]);
      maximumError=Math.max(maximumError,Math.hypot(Math.min(dx,200-dx),Math.min(dy,130-dy)));
    }
    assert(maximumError<(dropout?5:2),`wrapping with biased motion must remain localized: ${maximumError}`);
    assert.equal(crossings,4);
    report.longHorizon.push({turns:1000,instances:4,dropoutTurns:dropout?100:0,crossings,maximumPositionError:maximumError});
  } finally { long.free(); }
}

const raw='Σraw := A ** Σ- ** A\' + K ** R ** K\'';
assert.equal(source.split(raw).length,2);
const corrupted=create(packet(),source.replace(raw,`${raw} + [0f32 1f32 0f32; 0f32 0f32 0f32; 0f32 0f32 0f32]`));
try {
  const before=names.map(name=>corrupted.state(name));
  assert.throws(()=>corrupted.turn(packet()),error=>String(error).includes('symmetric-covariance!'));
  retained(corrupted,before);report.rawSymmetryRejection=true;
} finally { corrupted.free(); }

console.log(JSON.stringify({...report,status:'passed'},null,2));
