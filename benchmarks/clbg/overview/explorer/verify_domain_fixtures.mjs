// Pure model fixtures: no browser, benchmark process, or measured-file writes.
import assert from 'node:assert/strict';
import {chartDomain,comparison,globalStatistics,medianHistory} from './dist/math.mjs';

const data={languages:[
  {key:'mech',family:'mech',release_year:1990},
  {key:'mech-jit',family:'mech',release_year:2000},
  {key:'mech-resident',family:'mech',release_year:2020}
],benchmarks:[],programs:{}};
for(const benchmark of ['a','b']){
  const groups=[];
  for(const [index,language] of data.languages.entries()){
    const key=`${benchmark}:${language.key}`,x=[1,8,4][index],y=[1,32,16][index];
    data.programs[key]={key,loc:x,seconds:y,x,y};
    groups.push({language:language.key,programs:[key]});
  }
  data.benchmarks.push({key:benchmark,groups});
}
const selection={benchmarks:new Set(['a','b']),
  languages:new Set(data.languages.map(language=>language.key)),disabled:new Set()};
const immutable=JSON.stringify(data);
const envelopePoints=statistics=>['fast','small'].flatMap(kind=>statistics[kind]
  ?[statistics[kind].median,...(statistics[kind].spread2?.boundary??[])]:[]);
const containsUpper=(domain,point)=>{
  assert(Math.log2(point.x)<=domain.xMax+1e-12,'Global LOC contour exceeds fitted upper bound');
  assert(Math.log2(point.y)<=domain.yMax+1e-12,'Global time contour exceeds fitted upper bound');
};
const history=medianHistory(data,selection);
const early=envelopePoints(history[1].statistics),late=envelopePoints(history.at(-1).statistics);
assert(Math.max(...early.map(point=>point.y))>Math.max(...late.map(point=>point.y)),
  'The fixture must exercise an earlier contour larger than the final cohort');

for(const view of ['summary','mech','benchmark']){
  const options={...selection,view,focus:'a',focusMeans:false};
  const domain=chartDomain(data,options);
  const scoped={...selection,benchmarks:view==='benchmark'?new Set(['a']):selection.benchmarks};
  const current=globalStatistics(comparison(data,scoped).entries);
  for(const point of envelopePoints(current))containsUpper(domain,point);
  for(const item of medianHistory(data,scoped))for(const point of envelopePoints(item.statistics))
    containsUpper(domain,point);
  // Before this correction, flat per-language benchmark series produced no
  // useful language ellipse; bounds stopped at max endpoint + .25 doubling.
  assert(domain.xMax>Math.log2(8)+.25);
  assert(domain.yMax>Math.log2(32)+.25);
  assert.equal(domain.xMin,-.15);assert.equal(domain.yMin,-.18);
  for(const releaseYear of [1990,2000,2020])for(const languages of [new Set(),new Set(['mech'])]){
    assert.deepEqual(chartDomain(data,{...options,releaseYear,languages,
      disabled:new Set(['a:mech-jit']),fast:false,small:true,global:false,trail:false,
      ellipses:new Set()}),domain,'Visibility and year changes must keep bounds fixed');
  }
  if(view==='benchmark')assert.deepEqual(chartDomain(data,{...options,focusMeans:true}),domain);
  else{
    const compact=chartDomain(data,{...options,focusMeans:true});
    assert(Math.abs(compact.xMax-(Math.log2(8)+.25))<1e-12);
    assert(Math.abs(compact.yMax-(Math.log2(32)+.25))<1e-12);
    assert(compact.xMax<domain.xMax&&compact.yMax<domain.yMax,
      'Means focus intentionally retains a compact range');
  }
}

// Mech view must not acquire a huge range from an unrelated language.
const withOther=structuredClone(data);
withOther.languages.push({key:'other',release_year:2000});
for(const benchmark of withOther.benchmarks){
  const key=`${benchmark.key}:other`;
  withOther.programs[key]={key,loc:1e6,seconds:1e6,x:1e6,y:1e6};
  benchmark.groups.push({language:'other',programs:[key]});
}
assert.deepEqual(chartDomain(withOther,{...selection,view:'mech',focusMeans:false}),
  chartDomain(data,{...selection,view:'mech',focusMeans:false}));
assert.equal(JSON.stringify(data),immutable,'Fitting must not change data or normalization');
assert(early.some(point=>point.x<1||point.y<1),'Fixture retains intentionally clipped lower tails');
console.log('Verified full-cohort and cumulative-year median envelopes, fixed visibility bounds, compact means focus, Mech scope, and unchanged 1× policy.');
