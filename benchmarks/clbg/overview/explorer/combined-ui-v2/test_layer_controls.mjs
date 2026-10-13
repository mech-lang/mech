import assert from 'node:assert/strict';
import {layerPresentation,chartLayerState,summaryNumber,rankingDotLayout,recordedSamplePoints,sampleDensityPoints,densityGrid,chartDomain} from './math.mjs';

assert.deepEqual(layerPresentation('off'),{visible:false,opacity:0});
assert.deepEqual(layerPresentation('dim'),{visible:true,opacity:.15});
assert.deepEqual(layerPresentation('highlight'),{visible:true,opacity:1});
assert.throws(()=>layerPresentation('invalid'));
const defaults=chartLayerState({sampleCount:3,entryCount:1});
assert.equal(defaults.sampleLayer.opacity,.15);assert.equal(defaults.medianLayer.opacity,1);assert.equal(defaults.densityLayer.visible,false);
for(const samples of ['off','highlight'])for(const medians of ['off','highlight'])for(const density of ['off','highlight']){
  const layers=chartLayerState({samples,medians,density,global:true,sampleCount:3,entryCount:1});
  assert.equal(layers.showSamples,samples!=='off');
  assert.equal(layers.showMedians,medians!=='off');
  assert.equal(layers.showGlobal,medians!=='off');
  assert.equal(layers.showDensity,density!=='off');
  assert.equal(layers.empty,samples==='off'&&medians==='off'&&density==='off');
}
const onlyDensity=chartLayerState({samples:'off',medians:'off',density:'highlight',fast:false,small:false,global:true,sampleCount:3,entryCount:1});
assert.equal(onlyDensity.showDensity,true);assert.equal(onlyDensity.showGlobal,false);assert.equal(onlyDensity.empty,false);
const onlySamples=chartLayerState({samples:'dim',medians:'off',density:'off',fast:false,small:false,global:false,sampleCount:3,entryCount:1});
assert.equal(onlySamples.showSamples,true);assert.equal(onlySamples.empty,false);
assert.equal(chartLayerState({density:'highlight',sampleCount:0,entryCount:0}).empty,true);
assert.equal(chartLayerState({samples:'dim',medians:'dim',density:'dim',global:true,sampleCount:3,entryCount:1}).sampleLayer.opacity,.15);

assert.equal(summaryNumber(1.23456),'1.2');assert.equal(summaryNumber(1.26),'1.3');
assert.equal(summaryNumber(-.001,{signed:true}),'0.0');assert.equal(summaryNumber(-1.26,{signed:true}),'−1.3');
assert.equal(summaryNumber(1.26,{signed:true}),'+1.3');assert.throws(()=>summaryNumber(NaN));
const rows=Array.from({length:49},(_,i)=>({entry:{language:{key:`lang${i}`}},sigma_distance:(i%7-3)/4}));
const layout=rankingDotLayout(rows,{extent:3,width:240});
assert.equal(layout.points.length,49);assert.equal(new Set(layout.points.map(point=>point.row.entry.language.key)).size,49);
for(const point of layout.points)assert.equal(point.x,(point.row.sigma_distance+3)/6*240);
for(const a of layout.points)for(const b of layout.points)if(a!==b&&a.lane===b.lane)assert.ok(Math.abs(a.x-b.x)>=14);
const coincident=rankingDotLayout(rows.map(row=>({...row,sigma_distance:0})),{extent:3,width:240});
assert.equal(coincident.lanes,49);assert.ok(coincident.points.every(point=>point.x===120));
assert.throws(()=>rankingDotLayout(rows,{extent:0,width:240}));

const program={key:'p',id:'p',language:'one',benchmark:'b',loc:1,x:1,y:2,seconds:2,samples:[1,1,100]};
const benchmark={key:'b',baseline_seconds:1,groups:[{language:'one',programs:['p']}]};
const input={benchmark,programs:[program],fast:program,small:program};
const entries=[{language:{key:'one'},inputs:[input]}];
const population=recordedSamplePoints(entries),points=sampleDensityPoints(population);
assert.equal(points.length,3);assert.equal(points.filter(point=>point.y===1).length,2);
assert.equal(sampleDensityPoints([...population,...population]).length,3);
assert.equal(sampleDensityPoints(recordedSamplePoints(entries,{disabled:new Set(['p'])})).length,0);
assert.equal(sampleDensityPoints(recordedSamplePoints([{...entries[0],inputs:[{...input,programs:[{...program,seconds:null,samples:[]}]}]}])).length,0);
const domain={xMin:-.15,xMax:2,yMin:-.18,yMax:8},field=densityGrid(points,domain);
assert.equal(field.runs,3);assert.equal(field.programs,1);assert.equal(field.total_sample_weight,3);assert.equal(field.cells.length,1728);
// Visibility options are deliberately absent from the density population route.
for(const samples of ['off','dim','highlight'])for(const medians of ['off','dim','highlight'])for(const density of ['off','dim','highlight']){
  chartLayerState({samples,medians,density,fast:false,small:false,global:false,sampleCount:3,entryCount:1});
  assert.deepEqual(densityGrid(sampleDensityPoints(recordedSamplePoints(entries)),domain),field);
}
const data={languages:[{key:'one',release_year:2000}],programs:{p:program},benchmarks:[benchmark]};
const selection={benchmarks:new Set(['b']),languages:new Set(['one']),disabled:new Set()};
const medians=chartDomain(data,{...selection,focusMeans:true}),all=chartDomain(data,{...selection,focusMeans:false});
assert.ok(all.yMax>=Math.log2(100)+.25);assert.ok(medians.yMax<Math.log2(100));
assert.deepEqual(chartDomain(data,{...selection,focusMeans:true,sampleMode:'off',medianMode:'off',densityMode:'highlight'}),medians);
assert.deepEqual(chartDomain(data,{...selection,focusMeans:false,sampleMode:'off',medianMode:'dim',densityMode:'off'}),all);
console.log('Independent layer modes, actual-run density, framing and ranking presentation assertions passed.');
