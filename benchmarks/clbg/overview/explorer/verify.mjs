import fs from 'node:fs';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import {comparison,spread,gm,chartDomain,lineMetrics,comparisonMetrics,languagesByYear,sharedBenchmarks,globalStatistics,median,languageFamilies} from './dist/math.mjs';
const data=JSON.parse(fs.readFileSync(new URL('./dist/data.json',import.meta.url)));

// Older cohorts retain their original measured Mech control after the current
// n-body endpoints are explicitly remeasured. Its source and archived binary
// hash still must match; the new timing is checked independently below.
function historicalMeasurement(mech){
  if(mech.benchmark==='nbody'&&data.nbody_compiler_audit&&mech.previous_measurement){
    assert.equal(data.nbody_compiler_audit.source_changes,false);
    return {...mech,...mech.previous_measurement};
  }
  return mech;
}

const selection={benchmarks:new Set(data.benchmarks.map(b=>b.key)),languages:new Set(data.languages.map(l=>l.key)),disabled:new Set()};
const years=JSON.parse(fs.readFileSync(new URL('./language-years.json',import.meta.url)));
assert.equal(data.provenance['language-years.json'],crypto.createHash('sha256').update(fs.readFileSync(new URL('./language-years.json',import.meta.url))).digest('hex'));
assert.equal(new Set(data.languages.map(l=>l.release_name)).size,37);
for(const l of data.languages){
  const record=years.languages[l.family??l.key];
  assert.equal(l.release_year,record.year);assert.equal(l.release_source,record.source);
  assert.equal(l.release_name,record.name);assert(l.release_note);
}
assert.deepEqual(new Set(languagesByYear(data.languages,{year:1972})),new Set(['fortran','pascal','c','smalltalk']));
assert(languagesByYear(data.languages,{year:2014}).includes('swift'));
assert(!languagesByYear(data.languages,{year:2014}).includes('futhark'));
assert(languagesByYear(data.languages,{year:2017}).includes('futhark'));
assert.deepEqual(new Set(languagesByYear(data.languages,{year:2023})),new Set(data.languages.filter(l=>l.family!=='mech').map(l=>l.key)));
assert.deepEqual(new Set(languagesByYear(data.languages,{year:2026})),new Set(data.languages.map(l=>l.key)));
for(const language of data.languages.filter(l=>l.family==='mech'))assert.equal(language.release_year,2026);
assert.equal(data.languages.find(l=>l.key==='mech-resident').label,'Mech (interpreted)');
const diagonal=lineMetrics({x:4,y:1},{x:1,y:8});
assert.equal(diagonal.loc,1);assert.equal(diagonal.fastest,1);assert.equal(diagonal.slope,-1.5);assert(Math.abs(diagonal.length-Math.sqrt(13))<1e-12);
assert.equal(lineMetrics({x:40,y:2},{x:10,y:16}).length,diagonal.length);
assert.deepEqual(lineMetrics({x:2,y:3},{x:2,y:3}),{loc:2,fastest:3,slope:null,length:0});
assert.equal(lineMetrics({x:2,y:1},{x:2,y:4}).slope,-Infinity);
assert.equal(lineMetrics({x:4,y:1},{x:1,y:1}).slope,0);
const full=comparison(data,selection);
assert.equal(full.entries.length,43);
assert.equal(data.languages.length,43);
const futharkMean=full.entries.find(e=>e.language.key==='futhark');
assert.equal(futharkMean.inputs.length,9);
assert.equal(futharkMean.coverage.selected,10);
assert.deepEqual(futharkMean.coverage.missing.map(b=>b.key),['binarytrees']);
assert(full.entries.filter(e=>!['futhark','matlab','lua'].includes(e.language.key)).every(e=>e.inputs.length===10));
const sharedKeys=sharedBenchmarks(data,selection).map(b=>b.key);
const shared=comparison(data,{...selection,benchmarks:new Set(sharedKeys)});
assert.equal(shared.entries.length,43);
assert(shared.entries.every(e=>e.inputs.length===sharedKeys.length));
for(const entry of shared.entries)for(const profile of ['fast','small'])for(const axis of ['x','y'])assert.equal(entry[profile][axis],gm(entry.inputs.map(i=>i[profile][axis])));
assert.deepEqual(data.languages.find(l=>l.key==='futhark').unsupported_benchmarks,['binarytrees']);
assert.equal(data.benchmarks.find(b=>b.key==='binarytrees').groups.some(g=>g.language==='futhark'),false);
assert(data.exclusions.some(e=>e.language==='futhark'&&e.benchmark==='binarytrees'&&e.reason.includes('per-node')));
assert.equal(data.futhark_allocation_audit.status,'unsupported-allocation-model');
assert.deepEqual(new Set(data.futhark_allocation_audit.constructor_probes.map(p=>p.backend)),new Set(['c','multicore']));
assert(data.futhark_allocation_audit.conclusion.includes('releases the temporary immediately'));
const endpointKeys=new Set();
for(const benchmark of data.benchmarks)for(const group of benchmark.groups){
  const endpoints=new Set([group.fast,group.small].filter(Boolean));
  assert.deepEqual(new Set(group.programs),endpoints);
  for(const key of endpoints){
    assert(data.programs[key],`Missing endpoint: ${key}`);
    endpointKeys.add(key);
  }
}
assert.deepEqual(new Set(Object.keys(data.programs)),endpointKeys);
const sourceKeys=new Set(Object.values(data.programs).flatMap(program=>program.files));
assert.deepEqual(new Set(Object.keys(data.files)),sourceKeys);
assert.equal(data.protocol.programs,'Fastest and fewest-LOC endpoints only');
assert.equal(data.language_groups.length,6);
assert(data.language_groups.every(group=>group.languages.every(key=>data.languages.some(language=>language.key===key))));
const fasta=data.benchmarks.find(b=>b.key==='fasta');
const rustFasta=data.programs[fasta.groups.find(g=>g.language==='rust').fast];
assert.equal(rustFasta.key,'fasta:rust7-floating-16-local');
assert.equal(rustFasta.local_adaptation,true);
assert.equal(rustFasta.samples.length,5);
assert.equal(rustFasta.seconds,0.563287167);
assert(data.files[rustFasta.files[0]].code.includes('prob as f64 / IM as f64'));
assert(!data.programs['fasta:fasta-rust-6']);
const mechFasta=data.programs[fasta.groups.find(g=>g.language==='mech').fast];
assert.equal(mechFasta.report,rustFasta.report);
assert.equal(mechFasta.samples.length,5);
for(const row of data.reference_summary){
  const actual=full.entries.find(e=>e.language.key===row.lang);
  for(const profile of ['fast','small'])for(const axis of ['x','y'])assert(Math.abs(actual[profile][axis]-row[profile][axis])<1e-11);
}
for(const file of Object.values(data.files))assert.equal(crypto.createHash('sha256').update(file.code).digest('hex'),file.sha256);
for(const program of Object.values(data.programs)){
  assert.notEqual(program.algorithm?.strict,false,`Excluded work mismatch entered data: ${program.key}`);
  if(program.measurement_status==='source-only'){
    assert.equal(program.seconds,null);assert.equal(program.y,null);
    assert.equal(program.timing_sd,null);assert.deepEqual(program.samples,[]);
    assert.equal(program.loc,program.physical_loc+program.loc_expansions.reduce((s,e)=>s+e.extra_lines,0));
    continue;
  }
  const sorted=[...program.samples].sort((a,b)=>a-b),n=sorted.length;
  const median=n%2?sorted[Math.floor(n/2)]:(sorted[n/2-1]+sorted[n/2])/2;
  assert(Math.abs(median-program.seconds)<1e-10);
  if(program.language==='mech')for(const id of program.files)assert(data.files[id].name.endsWith('.mec'));
}
const subsetKeys=['nbody','mandelbrot','pidigits'];
const subset=comparison(data,{...selection,benchmarks:new Set(subsetKeys)});
for(const entry of subset.entries)for(const kind of ['fast','small'])for(const axis of ['x','y']){
  assert.equal(entry.inputs.length,subsetKeys.filter(key=>entry.language.timed_benchmarks.includes(key)).length);
  assert(Math.abs(entry[kind][axis]-gm(entry.inputs.map(i=>i[kind][axis])))<1e-11);
}
const nbody=data.benchmarks.find(b=>b.key==='nbody');
const mech=nbody.groups.find(g=>g.language==='mech');
const one=comparison(data,{...selection,benchmarks:new Set(['nbody'])});
assert.equal(one.entries.length,data.languages.filter(l=>l.timed_benchmarks.includes('nbody')).length);
assert.equal(one.entries.find(e=>e.language.key==='bqn').inputs[0].small.loc,26);
assert.equal(one.entries.find(e=>e.language.key==='bqn').inputs[0].fast.loc,39);
assert.equal(one.entries.find(e=>e.language.key==='k').inputs[0].small.loc,36);
assert.equal(data.programs[mech.small].loc,20);assert.equal(data.programs[mech.fast].loc,25);
assert.equal(comparison(data,{...selection,benchmarks:new Set(['spectralnorm'])}).entries.length,43);
const spectral=data.benchmarks.find(b=>b.key==='spectralnorm').groups.find(g=>g.language==='mech');
assert.equal(data.programs[spectral.fast].loc,12);
assert(data.files[data.programs[spectral.fast].files[0]].code.includes('coefficient(i<u64>, j<u64>)'));
assert.deepEqual(new Set(data.array_source_benchmarks),new Set(data.benchmarks.map(b=>b.key)));
for(const language of ['bqn','k','mojo','futhark']){
  assert.equal(full.entries.find(e=>e.language.key===language).inputs.length,language==='futhark'?9:10);
  for(const b of data.benchmarks){
    if(language==='futhark'&&b.key==='binarytrees')continue;
    const group=b.groups.find(g=>g.language===language);assert(group?.fast);
    const p=data.programs[group.fast];assert(p.samples.length>=3);
    if(b.key!=='nbody')assert.equal(p.algorithm.strict,true);
  }
}
const arraysOnly=comparison(data,{...selection,languages:new Set(['bqn','k'])},{sharedFallback:true});
assert.equal(arraysOnly.entries.length,2);assert.equal(Boolean(arraysOnly.usedSharedCoverage),false);
assert.equal(arraysOnly.benchmarks.length,10);
assert(arraysOnly.entries.every(e=>e.inputs.length===10));
for(const b of data.benchmarks)for(const p of b.groups.flatMap(g=>g.programs.map(k=>data.programs[k]))){
  assert.equal(p.x,p.loc/b.baseline_loc);
  assert.equal(p.y,p.seconds==null?null:p.seconds/b.baseline_seconds);
}
assert.equal(one.entries.find(e=>e.language.key==='mech').spread.fast,null);
const omitted=comparison(data,{...selection,benchmarks:new Set(['nbody']),disabled:new Set([mech.fast])});
assert.notEqual(omitted.entries.find(e=>e.language.key==='mech').inputs[0].fast.key,mech.fast);
const withoutMech=comparison(data,{...selection,languages:new Set(['rust','go'])});
assert.equal(withoutMech.entries.length,2);
const known=spread([{x:1,y:1},{x:4,y:8},{x:2,y:2}]);
assert(Math.abs(known.center.x-2)<1e-12);
assert.equal(known.n,3);
assert(known.covariance[0][1]>0);
assert(known.boundary.every(p=>Number.isFinite(p.x)&&Number.isFinite(p.y)&&p.x>0&&p.y>0));
assert.equal(known.log_base,2);assert.equal(known.sigma,1);
const doubleSpread=spread([{x:1,y:1},{x:4,y:8},{x:2,y:2}],{sigma:2});
assert.deepEqual(doubleSpread.center,known.center);assert.deepEqual(doubleSpread.covariance,known.covariance);
assert.equal(doubleSpread.sigma,2);
// This fixture has log2 sample covariance [[1, 1.5], [1.5, 7/3]].
assert(Math.abs(known.covariance[0][0]-1)<1e-12);
assert(Math.abs(known.covariance[0][1]-1.5)<1e-12);
assert(Math.abs(known.covariance[1][1]-7/3)<1e-12);
const determinant=7/3-1.5**2;
assert.equal(known.area_units,'squared_doublings');
assert(Math.abs(known.area-Math.PI/Math.sqrt(12))<1e-12);
assert(Math.abs(doubleSpread.area-4*Math.PI/Math.sqrt(12))<1e-12);
assert(Math.abs(doubleSpread.area/known.area-4)<1e-12);
const translated=spread([{x:8,y:32},{x:32,y:256},{x:16,y:64}],{sigma:2});
assert(Math.abs(translated.area-doubleSpread.area)<1e-12,'Normalization shifts must not change covariance area');
assert.equal(spread([{x:1,y:1},{x:2,y:4}],{sigma:2}).area,0);
assert.equal(spread([{x:1,y:1},{x:2,y:4},{x:4,y:16}],{sigma:2}).area,0);
assert.equal(comparisonMetrics(one.entries[0]).areaFast,null);
assert.equal(comparisonMetrics(one.entries[0]).areaSmall,null);
for(const entry of full.entries){
  const metrics=comparisonMetrics(entry);
  assert.equal(metrics.areaFast,entry.spread2.fast.area);
  assert.equal(metrics.areaSmall,entry.spread2.small.area);
  for(const key of ['areaFast','areaSmall'])assert(Number.isFinite(metrics[key])&&metrics[key]>=0);
}
for(const point of doubleSpread.boundary){
  const x=Math.log2(point.x)-1,y=Math.log2(point.y)-4/3;
  assert(Math.abs(((7/3)*x*x-3*x*y+y*y)/determinant-4)<1e-10,'2σ boundary must have squared Mahalanobis distance four');
}
for(const [i,point] of doubleSpread.boundary.entries())for(const axis of ['x','y']){
  assert(Math.abs(Math.log2(point[axis]/known.center[axis])-2*Math.log2(known.boundary[i][axis]/known.center[axis]))<1e-12);
}
assert.equal(spread([{x:1,y:1}],{sigma:2}),null);
assert.throws(()=>spread([],{sigma:0}));
assert.equal(comparison(data,{...selection,benchmarks:new Set()}).entries.length,0);
for(const language of ['mojo','futhark']){
  const l=data.languages.find(l=>l.key===language);
  assert.equal(l.color,language==='mojo'?'#ff4c1f':'#5f021f');
  for(const b of data.benchmarks)for(const key of b.groups.find(g=>g.language===language)?.programs??[]){
    const p=data.programs[key];assert(p.required_work_validated);assert.equal(p.algorithm.native_benchmark_kernel,false);
    assert(p.compiler_sha256?.length===64);assert(p.binary_sha256?.length===64);
    assert(p.files.every(id=>data.files[id].role==='runtime-support'||data.files[id].name.endsWith(language==='mojo'?'.mojo':'.fut')));
  }
}
const kMean=full.entries.find(e=>e.language.key==='k'),mechMean=full.entries.find(e=>e.language.key==='mech');
assert(Math.abs(kMean.fast.x/mechMean.fast.x-gm(data.benchmarks.map(b=>data.programs[b.groups.find(g=>g.language==='k').fast].loc/data.programs[b.groups.find(g=>g.language==='mech').fast].loc)))<1e-12);
const futharkOnly=comparison(data,{...selection,languages:new Set(['futhark'])});
assert.equal(futharkOnly.benchmarks.length,10);assert.equal(futharkOnly.entries[0].inputs.length,9);
assert.equal(comparison(data,{...selection,benchmarks:new Set(['binarytrees']),languages:new Set(['futhark'])}).entries.length,0);
for(const kind of ['fast','small'])for(const axis of ['x','y']){
  assert.equal(futharkMean[kind][axis],futharkOnly.entries[0][kind][axis]);
  assert.equal(futharkMean[kind][axis],gm(futharkMean.inputs.map(i=>i[kind][axis])));
}
assert.equal(futharkMean.spread2.fast.n,9);
const hiddenNbody=comparison(data,{...selection,languages:new Set(['mech']),disabled:new Set(mech.programs)}).entries[0];
assert.equal(hiddenNbody.inputs.length,9);assert.deepEqual(hiddenNbody.coverage.missing.map(b=>b.key),['nbody']);
console.log(`Verified ${endpointKeys.size} endpoint programs, ${sourceKeys.size} referenced source hashes, ten-test Mojo and nine-test Futhark coverage, common-subset geometric means, allocation exclusion, sample medians, and covariance ellipses.`);

// Focus fits full-cohort means; the full range includes samples and hidden
// ellipse envelopes. Visibility filters must not determine either range.
const domain=chartDomain(data,selection);
const sampleDomain=chartDomain(data,{...selection,focusMeans:false});
assert.equal(domain.xMin,-.15);assert.equal(domain.yMin,-.18);
const contains=(domain,point)=>{
  assert(Math.log2(point.x)>=domain.xMin&&Math.log2(point.x)<=domain.xMax);
  assert(Math.log2(point.y)>=domain.yMin&&Math.log2(point.y)<=domain.yMax);
};
for(const reference of [full,shared])for(const entry of reference.entries)for(const kind of ['fast','small']){
  contains(domain,entry[kind]);
  for(const point of entry.spread[kind]?.boundary??[])if(point.x>=1&&point.y>=1)contains(sampleDomain,point);
  for(const point of entry.spread2[kind]?.boundary??[])if(point.x>=1&&point.y>=1)contains(sampleDomain,point);
}
for(const focusMeans of [true,false])for(const languages of [new Set(['mech']),new Set(['futhark']),new Set()]){
  assert.deepEqual(chartDomain(data,{...selection,focusMeans,languages,disabled:new Set([mech.fast]),fast:false,ellipses:new Set()}),focusMeans?domain:sampleDomain);
}
assert(sampleDomain.xMax>domain.xMax,'Focus must tighten the full-suite LOC range');
assert(sampleDomain.yMax>domain.yMax,'Focus must tighten the full-suite time range');
const cohortReferences=[full,shared,...['pidigits','binarytrees'].map(missing=>comparison(data,{...selection,benchmarks:new Set(data.benchmarks.filter(b=>b.key!==missing).map(b=>b.key))}))];
const cohortMeans=cohortReferences.flatMap(reference=>reference.entries.flatMap(entry=>[entry.fast,entry.small]));
assert.equal(domain.xMax,Math.log2(Math.max(...cohortMeans.map(p=>p.x)))+.25,'Covariance tails must not override means-focused LOC bounds');
assert.equal(domain.yMax,Math.log2(Math.max(...cohortMeans.map(p=>p.y)))+.25,'Covariance tails must not override means-focused time bounds');
const mechDomain=chartDomain(data,{...selection,view:'mech'});
const mechSampleDomain=chartDomain(data,{...selection,view:'mech',focusMeans:false});
for(const entry of full.entries.filter(entry=>entry.language.family==='mech'||entry.language.key==='mech'))for(const kind of ['fast','small']){
  contains(mechDomain,entry[kind]);
  for(const point of entry.spread2[kind]?.boundary??[])if(point.x>=1&&point.y>=1)contains(mechSampleDomain,point);
}
assert(mechSampleDomain.xMax>mechDomain.xMax,'Focus also tightens Mech-view LOC bounds');
assert(mechSampleDomain.yMax>mechDomain.yMax,'Focus also tightens Mech-view time bounds');
const singleDomain=chartDomain(data,{...selection,view:'benchmark',focus:'nbody'});
assert.deepEqual(chartDomain(data,{...selection,view:'benchmark',focus:'nbody',focusMeans:false}),singleDomain,'Focus does not affect single-benchmark bounds');
const nbodyPrograms=nbody.groups.flatMap(group=>group.programs.map(key=>data.programs[key]));
assert.equal(singleDomain.xMin,-.15);
assert.equal(singleDomain.yMin,-.18);
assert.equal(singleDomain.xMax,Math.max(1.15,Math.log2(Math.max(...nbodyPrograms.map(p=>p.x)))+.25));
assert.equal(singleDomain.yMax,Math.max(1.12,Math.log2(Math.max(...nbodyPrograms.map(p=>p.y)))+.25));
for(const point of nbodyPrograms)contains(singleDomain,point);
for(const reference of [full,shared])for(const entry of reference.entries)for(const input of entry.inputs){
  contains(sampleDomain,input.fast);contains(sampleDomain,input.small);
}
assert(Object.values(chartDomain(data,{...selection,benchmarks:new Set()})).every(Number.isFinite));
console.log('Verified distinct means-focused and full-range bounds, labelled partial-suite means, visibility independence, single-benchmark limits, sample coverage, and empty selections.');

for(const key of ['mech-jit','mech-resident']){
  const language=data.languages.find(l=>l.key===key);
  assert.equal(language.family,'mech');assert.equal(language.timed_benchmarks.length,10);
  assert.equal(language.color,data.languages.find(l=>l.key==='mech').color);
  const entry=full.entries.find(e=>e.language.key===key);assert.equal(entry.inputs.length,10);
  for(const benchmark of data.benchmarks){
    const group=benchmark.groups.find(g=>g.language===key);assert(group?.fast&&group?.small);
    for(const program of group.programs.map(key=>data.programs[key])){
      assert.equal(program.execution_mode,key==='mech-jit'?'jit':'resident');
      assert.equal(program.samples.length,3);assert.equal(program.sample_details.length,3);
      assert(program.required_work_validated);
      assert(program.files.every(file=>data.files[file].name.endsWith('.mec')));
      for(const [i,sample] of program.sample_details.entries()){
        assert(sample.wall_seconds>=sample.kernel_seconds);
        assert.equal(program.samples[i],key==='mech-jit'?sample.wall_seconds:sample.kernel_seconds);
      }
      assert(program.timing_summary.includes(key==='mech-jit'?'compilation':'activation excluded'));
    }
  }
}
assert.equal(data.execution_mode_audit.semantic_checks,54);
assert.equal(data.execution_mode_audit.native_objects_identical,15);
assert(Object.values(data.execution_mode_audit.aot_regressions).every(row=>row.native_object_identical));
assert(data.execution_mode_audit.timing_report.includes('20261010-final'));
assert(data.execution_mode_audit.aot_runtime_control.identical_native_object);
assert(data.execution_mode_audit.optimization_changes);
for(const benchmark of data.benchmarks)for(const key of ['mech-jit','mech-resident']){
  for(const program of benchmark.groups.find(g=>g.language===key).programs.map(k=>data.programs[k])){
    assert.equal(program.report,data.execution_mode_audit.timing_report);
    assert.equal(program.seconds,data.execution_mode_audit.optimization_changes[program.key].after_seconds);
  }
}
const jitNbody=nbody.groups.find(g=>g.language==='mech-jit');
assert(data.programs[jitNbody.fast].seconds<=data.programs[jitNbody.small].seconds);
console.log('Verified full-suite JIT and resident coverage, identical counted Mech source, timing boundaries, raw process samples, and AOT regression provenance.');

const swift=data.languages.find(l=>l.key==='swift');
assert.equal(swift.color,'#F05138');assert.equal(swift.release_year,2014);
assert.equal(swift.execution,'AOT');assert.equal(swift.timed_benchmarks.length,10);
assert(data.language_groups.find(g=>g.key==='aot').languages.includes('swift'));
for(const g of data.language_groups.filter(g=>g.key!=='aot'))assert(!g.languages.includes('swift'));
assert.equal(data.swift_audit.published_sources,37);assert.equal(data.swift_audit.corrected_sources,10);
assert.equal(Object.keys(data.swift_audit.controls).length,10);
for(const benchmark of data.benchmarks){
  const group=benchmark.groups.find(g=>g.language==='swift');assert(group?.fast&&group?.small);
  for(const key of group.programs){
    const program=data.programs[key];assert.equal(program.samples.length,3);
    assert.equal(program.algorithm.strict,true);assert.equal(program.required_work_validated,true);
    assert.equal(program.algorithm.native_benchmark_kernel,false);
    assert(program.source_url.startsWith('https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/'));
    assert(data.files[program.files[0]].name.endsWith('.swift'));
    assert(program.runtime_policy.includes('Apple Swift 5.10'));
    if(['binarytrees','fasta'].includes(benchmark.key)){
      assert.equal(program.local_adaptation,true);assert(program.key.endsWith('-faithful'));
      assert(program.adaptations.length>0);
    }
    if(benchmark.key==='pidigits'){
      assert(program.files.some(id=>data.files[id].role==='runtime-support'));
      assert(data.files[program.files[0]].code.includes('extractDigit(3)'));
      assert(data.files[program.files[0]].code.includes('extractDigit(4)'));
    }
  }
}
console.log('Verified ten Swift workloads, three samples, source correction labels, LOC endpoints, compiler policy, AOT grouping, and 2014 release filtering.');

assert.deepEqual(data.language_groups.find(g=>g.key==='functional').languages,['haskell','ocaml','fsharp','fsharp-aot','futhark','mech','mech-jit','mech-resident','erlang','lisp','racket','roc','elm']);
assert.equal(data.functional_audit.published_sources,60);
assert.equal(data.functional_audit.allocation_evidence.length,9);
assert.equal(Object.keys(data.functional_audit.controls).length,10);
for(const [key,color,year,suffix] of [['haskell','#5E5086',1990,'.hs'],['ocaml','#EF7A08',1996,'.ml']]){
  const lang=data.languages.find(l=>l.key===key);
  assert.equal(lang.color,color);assert.equal(lang.release_year,year);assert.equal(lang.execution,'AOT');
  assert.equal(lang.timed_benchmarks.length,10);
  assert(data.language_groups.find(g=>g.key==='aot').languages.includes(key));
  assert(!languagesByYear(data.languages,{year:year-1}).includes(key));
  assert(languagesByYear(data.languages,{year}).includes(key));
  for(const benchmark of data.benchmarks){
    const group=benchmark.groups.find(g=>g.language===key);assert(group?.fast&&group?.small);
    for(const id of group.programs){
      const program=historicalMeasurement(data.programs[id]);assert.equal(program.samples.length,3);
      assert(program.algorithm.strict&&program.required_work_validated);assert.equal(program.algorithm.native_benchmark_kernel,false);
      assert.equal(program.loc,program.physical_loc+program.loc_expansions.reduce((n,e)=>n+e.extra_lines,0));
      assert(data.files[program.files[0]].name.endsWith(suffix));
      assert(program.runtime_policy.includes(key==='haskell'?'GHC 9.10.3':'Flambda'));
      if(benchmark.key==='binarytrees')assert(program.local_adaptation&&program.key.endsWith('-faithful'));
    }
  }
}
console.log('Verified complete Haskell/OCaml workloads, three samples, audited statement LOC, allocation evidence, Functional/AOT groups, GitHub colors, and release-year filters.');

const matlab=data.languages.find(l=>l.key==='matlab');
assert.equal(matlab.color,'#e16737');assert.equal(matlab.release_year,1984);
assert.equal(matlab.execution,'JIT');assert(matlab.timed_benchmarks.length>0&&matlab.timed_benchmarks.length<=10);
assert(!languagesByYear(data.languages,{year:1983}).includes('matlab'));
assert(languagesByYear(data.languages,{year:1984}).includes('matlab'));
for(const group of data.language_groups)assert.equal(group.languages.includes('matlab'),['array','dynamic','jit'].includes(group.key));
const matlabAudit=data.matlab_audit;
assert.equal(matlabAudit.attempted_sources,20);assert.equal(matlabAudit.admitted_sources,2*matlab.timed_benchmarks.length);assert.equal(matlabAudit.sample_count,3);
assert.deepEqual(new Set(matlabAudit.completed_benchmarks),new Set(matlab.timed_benchmarks));
assert.equal(matlabAudit.coverage_status,matlab.timed_benchmarks.length===10?'complete':'partial');
assert.equal(Object.keys(matlabAudit.controls).length,matlab.timed_benchmarks.length);
assert.equal(matlabAudit.allocation_evidence.status,'passed');
for(const profile of ['compact','optimized'])for(const [index,depth] of matlabAudit.allocation_evidence.depths.entries()){
  const inventory=matlabAudit.allocation_evidence.results[profile][index],nodes=2**(depth+1)-1;
  assert.deepEqual(inventory,[nodes,nodes,nodes,2**depth,1]);
}
const allocationBranches=matlabAudit.supplemental_allocation_evidence;
assert.equal(allocationBranches.status,'passed');assert.equal(allocationBranches.timed,false);
assert.deepEqual(allocationBranches.depths,[1,3,7,11]);
assert.match(allocationBranches.report_sha256,/^[a-f0-9]{64}$/);
for(const profile of ['compact','optimized'])for(const [index,depth] of allocationBranches.depths.entries()){
  const nodes=2**(depth+1)-1;
  assert.deepEqual(allocationBranches.evidence[profile][index],[nodes,nodes,nodes,2**depth,1]);
}
const expectedMatlabWork={spectralnorm:{coefficient_evaluations:1210000000},mandelbrot:{pixels:256000000},
  regexredux:{count_patterns:9,ordered_substitutions:5},revcomp:{nucleotides:250000000},
  fasta:{lcg_updates:200000000},nbody:{steps:50000000,pair_interactions:500000000},
  pidigits:{digits:10000},fannkuchredux:{permutations:479001600},
  knucleotide:{hash_table_window_updates:874999961},binarytrees:{total_nodes:613766494,node_slots:2}};
for(const benchmark of data.benchmarks){
  if(!matlab.timed_benchmarks.includes(benchmark.key)){assert(!benchmark.groups.some(g=>g.language==='matlab'));continue;}
  const group=benchmark.groups.find(g=>g.language==='matlab');assert(group?.fast&&group?.small);
  assert(data.programs[group.fast].seconds<=data.programs[group.small].seconds);
  assert(data.programs[group.small].loc<=data.programs[group.fast].loc);
  const control=matlabAudit.controls[benchmark.key];assert.equal(control.samples_seconds.length,3);
  assert.equal(control.chart_key,benchmark.groups.find(g=>g.language==='mech').fast);
  assert.equal(control.source_sha256,data.files[data.programs[control.chart_key].files[0]].sha256);
  for(const key of group.programs){
    const program=data.programs[key];assert.equal(program.samples.length,3);
    assert(program.required_work_validated&&program.algorithm.strict);
    assert.equal(program.algorithm.native_benchmark_kernel,false);
    assert.equal(program.loc,program.physical_loc+program.loc_expansions.reduce((n,e)=>n+e.extra_lines,0));
    assert.equal(program.execution_mode,'jit');assert.equal(program.loc_metric,'statement-normalized');
    assert.equal(program.source_link_label,'Task specification');assert(program.local_adaptation);
    assert.equal(program.source_url,`https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/${benchmark.key}.html`);
    assert.equal(program.runtime_sha256,matlabAudit.runtimes.engine.sha256);
    assert(program.runtime_policy.includes('MATLAB R2026b'));
    assert(program.timing_boundary.includes('Fresh-process'));
    assert.equal(program.full_output_validation.length,3);
    assert(program.full_output_validation.every(sample=>sample.reference_match));
    assert(data.files[program.files[0]].name.endsWith('.m'));
    assert(program.files.every(id=>data.files[id].role==='runtime-support'||data.files[id].name.endsWith('.m')));
    for(const [field,value] of Object.entries(expectedMatlabWork[benchmark.key]))assert.equal(program.algorithm.full_work[field],value);
    const support=program.files.filter(id=>data.files[id].role==='runtime-support');
    assert.equal(support.length,['regexredux','revcomp','knucleotide'].includes(benchmark.key)?1:0);
    for(const id of support){assert(data.files[id].code.includes('exec 3<&0'));assert(data.files[id].loc_note.includes('stream routing only'));}
  }
}
assert.equal(full.entries.find(e=>e.language.key==='matlab').spread2.fast.n,matlab.timed_benchmarks.length);
console.log(`Verified ${matlab.timed_benchmarks.length}/10 MATLAB workloads, ${matlabAudit.admitted_sources} measured candidates, labelled coverage, three samples, statement LOC, even/odd-depth independent cell allocations, fastest Mech AOT controls, stream-only support, JIT/Array/Dynamic groups, and 1984 release filtering.`);

const erlang=data.languages.find(l=>l.key==='erlang'),erlangAudit=data.erlang_audit;
assert.equal(erlang.color,'#B83998');assert.equal(erlang.release_year,1990);assert.equal(erlang.execution,'JIT');
assert.equal(erlang.timed_benchmarks.length,10);
assert(!languagesByYear(data.languages,{year:1989}).includes('erlang'));
assert(languagesByYear(data.languages,{year:1990}).includes('erlang'));
for(const group of data.language_groups)assert.equal(group.languages.includes('erlang'),['dynamic','functional','jit'].includes(group.key));
assert.equal(erlangAudit.published_sources,21);assert.equal(erlangAudit.corrected_sources,11);
assert.equal(erlangAudit.admitted_sources,21);assert.equal(erlangAudit.sample_count,3);
assert.equal(erlangAudit.runtime_details.jit,'jit');assert.equal(erlangAudit.runtime_details.schedulers,8);
assert.equal(erlangAudit.allocation_evidence.length,2);assert.equal(Object.keys(erlangAudit.controls).length,10);
assert.equal(data.provenance['erlang_data.py'],crypto.createHash('sha256').update(fs.readFileSync(new URL('./erlang_data.py',import.meta.url))).digest('hex'));
assert.equal(data.provenance['benchmarks-game-license.txt'],crypto.createHash('sha256').update(fs.readFileSync(new URL('./dist/benchmarks-game-license.txt',import.meta.url))).digest('hex'));
for(const benchmark of data.benchmarks){
  const group=benchmark.groups.find(g=>g.language==='erlang');assert(group?.fast&&group?.small);
  assert(data.programs[group.fast].seconds<=data.programs[group.small].seconds);
  assert(data.programs[group.small].loc<=data.programs[group.fast].loc);
  const control=erlangAudit.controls[benchmark.key],mech=historicalMeasurement(data.programs[control.chart_key]);
  assert.equal(control.samples_seconds.length,3);assert.equal(control.chart_key,benchmark.groups.find(g=>g.language==='mech').fast);
  assert.equal(control.source_sha256,data.files[mech.files[0]].sha256);assert.equal(control.binary_sha256,mech.binary_sha256);
  assert.equal(control.previous_seconds,mech.seconds);
  for(const id of group.programs){
    const program=historicalMeasurement(data.programs[id]);assert.equal(program.samples.length,3);assert(program.required_work_validated&&program.algorithm.strict);
    assert.equal(program.algorithm.native_benchmark_kernel,false);assert.equal(program.execution_mode,'jit');
    assert.equal(program.loc_metric,'statement-normalized');assert.equal(program.loc,program.physical_loc+program.loc_expansions.reduce((n,e)=>n+e.extra_lines,0));
    assert.equal(program.runtime_sha256,erlangAudit.runtimes.engine.sha256);
    assert.equal(program.compiler_sha256,erlangAudit.runtimes.compiler.sha256);
    assert.equal(program.full_output_validation.length,3);assert(program.full_output_validation.every(v=>v.reference_match));
    assert(program.files.every(id=>data.files[id].name.endsWith('.erl')));
    assert(program.source_url.startsWith('https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/'));
    assert.equal(program.license_url,'./benchmarks-game-license.txt');
    assert(program.runtime_policy.includes('load-time JIT'));assert(program.timing_boundary.includes('initialization'));
    if(['binarytrees','fasta','knucleotide','mandelbrot','regexredux'].includes(benchmark.key))assert(program.local_adaptation&&program.key.endsWith('-faithful'));
  }
}
assert.equal(full.entries.find(e=>e.language.key==='erlang').spread2.fast.n,10);
console.log('Verified all ten published Erlang workloads, 21 admitted candidates, three full samples, statement LOC, BEAM JIT and allocation evidence, unchanged Mech controls, Functional/Dynamic/JIT groups, GitHub color, and public-introduction filtering.');

const lispRacketAudit=data.lisp_racket_audit;
assert.equal(lispRacketAudit.published_sources,59);assert.equal(lispRacketAudit.corrected_sources,19);
assert.equal(lispRacketAudit.admitted_sources,59);assert.equal(lispRacketAudit.sample_count,3);
assert.deepEqual(lispRacketAudit.published_counts,{lisp:33,racket:26});
assert.equal(lispRacketAudit.allocation_evidence.length,6);assert.equal(Object.keys(lispRacketAudit.controls).length,10);
assert.equal(lispRacketAudit.loc_fixtures,18);
assert.equal(data.provenance['lisp_racket_data.py'],crypto.createHash('sha256').update(fs.readFileSync(new URL('./lisp_racket_data.py',import.meta.url))).digest('hex'));
for(const [key,color,year,runtime,extension] of [['lisp','#3fb68b',1984,'sbcl','.lisp'],['racket','#3c5caa',1995,'racket','.rkt']]){
  const language=data.languages.find(l=>l.key===key);
  assert.equal(language.color,color);assert.equal(language.release_year,year);assert.equal(language.execution,'AOT');
  assert.equal(language.timed_benchmarks.length,10);
  assert(!languagesByYear(data.languages,{year:year-1}).includes(key));assert(languagesByYear(data.languages,{year}).includes(key));
  for(const group of data.language_groups)assert.equal(group.languages.includes(key),['dynamic','functional','aot'].includes(group.key));
  for(const benchmark of data.benchmarks){
    const group=benchmark.groups.find(g=>g.language===key);assert(group?.fast&&group?.small);
    assert(data.programs[group.fast].seconds<=data.programs[group.small].seconds);
    assert(data.programs[group.small].loc<=data.programs[group.fast].loc);
    const control=lispRacketAudit.controls[benchmark.key],mech=historicalMeasurement(data.programs[control.chart_key]);
    assert.equal(control.samples_seconds.length,3);assert.equal(control.chart_key,benchmark.groups.find(g=>g.language==='mech').fast);
    assert.equal(control.source_sha256,data.files[mech.files[0]].sha256);assert.equal(control.binary_sha256,mech.binary_sha256);
    assert.equal(control.previous_seconds,mech.seconds);
    for(const id of group.programs){
      const program=historicalMeasurement(data.programs[id]);assert.equal(program.samples.length,3);assert(program.required_work_validated&&program.algorithm.strict);
      assert.equal(program.algorithm.native_benchmark_kernel,false);assert.equal(program.execution_mode,'aot');
      assert.equal(program.loc_metric,'statement-normalized');assert.equal(program.loc,program.physical_loc+program.loc_expansions.reduce((n,e)=>n+e.extra_lines,0));
      assert.equal(program.runtime_sha256,lispRacketAudit.runtimes[runtime].sha256);
      assert.equal(program.compiler_sha256,lispRacketAudit.runtimes[runtime].sha256);
      assert.equal(program.full_output_validation.length,3);assert(program.full_output_validation.every(v=>v.reference_match));
      assert(program.files.every(id=>data.files[id].name.endsWith(extension)));
      assert(program.source_url.startsWith('https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/'));
      assert.equal(program.license_url,'./benchmarks-game-license.txt');
      assert(program.timing_boundary.includes('Fresh-process wall time'));assert(program.timing_boundary.includes('shutdown'));
      if(program.local_adaptation){assert(program.key.endsWith('-faithful'));assert(program.adaptations.length>0);}
    }
  }
  assert.equal(full.entries.find(e=>e.language.key===key).spread2.fast.n,10);
}
console.log('Verified all ten Common Lisp/Racket workloads, 59 admitted candidates, three full samples, reader/statement LOC, native precompilation, allocation evidence, unchanged Mech controls, GitHub colors, and release/group filters.');

const zigCohort=data.zig_odin_roc_elm_audit;
assert.equal(zigCohort.admitted_sources,61);assert.equal(zigCohort.published_source_candidates,20);
assert.equal(zigCohort.locally_authored_source_candidates,41);assert.equal(zigCohort.sample_count,3);
assert.equal(zigCohort.completed_samples,213);assert.equal(zigCohort.comparisons_to_mech.length,40);
assert.equal(Object.keys(zigCohort.controls).length,10);assert.equal(zigCohort.general_helper_tests.total_expectations,228);
assert.equal(zigCohort.general_helper_tests.optimized_regex_cases,14);
assert.equal(zigCohort.general_helper_tests.regex_implementations,2);
assert.equal(zigCohort.general_helper_tests.optimized_regex_evidence.status,'passed');
assert.equal(zigCohort.elm_packing_evidence.status,'passed');assert.equal(zigCohort.elm_packing_evidence.cases.length,9);
assert(zigCohort.elm_packing_evidence.cases.every(c=>c.full_output_match));
assert.equal(zigCohort.elm_packing_evidence.representation.maximum_value,2**52-1);
assert.equal(zigCohort.elm_packing_evidence.representation.largest_exact_integer,2**53-1);
assert.equal(zigCohort.nucleotide_key_evidence.status,'passed');assert.equal(zigCohort.nucleotide_key_evidence.cases.length,4);
assert(zigCohort.nucleotide_key_evidence.cases.every(c=>c.full_output_match&&c.widths.join(',')==='1,2,3,4,6,12,18'));
const [fullKeyA,fullKeyB]=zigCohort.nucleotide_key_evidence.distinct_full_keys;
assert.notEqual(fullKeyA,fullKeyB);assert.equal(fullKeyA%2**32,fullKeyB%2**32);
assert.equal(zigCohort.allocation_evidence.status,'passed');
assert(zigCohort.allocation_evidence.elm.cases.every(c=>c.actualRunCheckOrder&&c.distinctNodes===c.nodesExpected&&c.traversedNodes===c.nodesExpected));
assert(zigCohort.allocation_evidence.roc.long_lived_retained_through_temporary_loop);
assert.equal(zigCohort.allocation_evidence.native_arenas.programs.length,4);
for(const row of zigCohort.allocation_evidence.native_arenas.programs){
  assert.equal(row.status,'passed');assert.equal(row.cases.length,2);
  for(const c of row.cases){assert.equal(c.node_bytes,16);assert.equal(c.allocation_requests,c.expected_allocation_requests);assert(c.distinct_children&&c.complete_output_unchanged);}
}
assert.equal(zigCohort.allocation_evidence.timed_native_assembly.status,'passed');
assert.equal(zigCohort.allocation_evidence.timed_native_assembly.programs.length,4);
for(const row of zigCohort.allocation_evidence.timed_native_assembly.programs){
  assert.equal(row.status,'passed');assert.equal(row.node_allocation_bytes,16);assert.equal(row.recursive_child_constructor_calls,2);
  assert(row.both_pointer_children_stored_and_loaded&&row.construction_and_traversal_inside_iteration_loop&&row.reset_after_traversal&&row.allocator_call_retained);
  const counted=zigCohort.allocation_evidence.native_arenas.programs.find(p=>p.key===row.key);
  assert(counted);assert.equal(counted.source_sha256,row.source_sha256);assert.equal(counted.binary_sha256,row.binary_sha256);
}
assert.equal(zigCohort.retained_report.samples,75);assert.equal(zigCohort.retained_source_report.samples,198);
assert.equal(zigCohort.retained_initial_source_report.samples,75);
for(const extension of ['csv','md']){
  const name=`zig-odin-roc-elm-mech.${extension}`;
  assert.equal(zigCohort.comparison_downloads[extension],`./${name}`);
  assert.equal(data.provenance[name],crypto.createHash('sha256').update(fs.readFileSync(new URL(`./dist/${name}`,import.meta.url))).digest('hex'));
}
assert.equal(zigCohort.work_exclusions.length,2);
for(const row of zigCohort.work_exclusions){assert.equal(row.samples_excluded,0);assert(!data.programs[row.key]);assert(data.exclusions.some(e=>e.key===row.key));}
assert.equal(data.provenance['zig_odin_roc_elm_data.py'],crypto.createHash('sha256').update(fs.readFileSync(new URL('./zig_odin_roc_elm_data.py',import.meta.url))).digest('hex'));
for(const [key,color,year,extension,execution,categories] of [
 ['zig','#ec915c',2016,'.zig','AOT',['aot']],['odin','#60AFFE',2016,'.odin','AOT',['aot']],
 ['roc','#7c38f5',2020,'.roc','AOT',['functional','aot']],['elm','#60B5CC',2012,'.elm','JIT',['functional','jit']]]){
  const language=data.languages.find(l=>l.key===key);
  assert.equal(language.color,color);assert.equal(language.release_year,year);assert.equal(language.execution,execution);
  assert.equal(language.timed_benchmarks.length,10);assert.equal((language.unsupported_benchmarks??[]).length,0);
  assert(!languagesByYear(data.languages,{year:year-1}).includes(key));assert(languagesByYear(data.languages,{year}).includes(key));
  for(const group of data.language_groups)assert.equal(group.languages.includes(key),categories.includes(group.key));
  for(const benchmark of data.benchmarks){
    const group=benchmark.groups.find(g=>g.language===key),comparison=zigCohort.comparisons_to_mech.find(c=>c.language===key&&c.benchmark===benchmark.key);
    assert(group?.fast&&group?.small);assert.equal(group.fast,comparison.fastest.key);assert.equal(group.small,comparison.smallest.key);
    const control=zigCohort.controls[benchmark.key],mech=historicalMeasurement(data.programs[control.chart_key]);
    assert.equal(control.samples.length,3);assert.equal(control.source_sha256,data.files[mech.files[0]].sha256);
    assert.equal(control.binary_sha256,mech.binary_sha256);assert.equal(control.preserved_chart_seconds,mech.seconds);
    assert.equal(control.fastest_loc,mech.loc);
    const smallMech=data.programs[benchmark.groups.find(g=>g.language==='mech').small];
    assert.equal(control.smallest_loc,smallMech.loc);
    for(const kind of ['fastest','smallest']){
      const endpoint=comparison[kind],program=historicalMeasurement(data.programs[endpoint.key]);
      assert.equal(endpoint.seconds,program.seconds);assert.equal(endpoint.loc,program.loc);
      assert.equal(endpoint.time_x_mech,program.seconds/control.seconds);
      assert.equal(endpoint.loc_x_mech,program.loc/(kind==='fastest'?control.fastest_loc:control.smallest_loc));
      assert.equal(program.mech_comparison.seconds,control.seconds);assert.equal(program.mech_comparison.time_x_mech,endpoint.time_x_mech);
      assert.equal(program.mech_comparison.fastest_loc_x_mech,program.loc/control.fastest_loc);
      assert.equal(program.mech_comparison.smallest_loc_x_mech,program.loc/control.smallest_loc);
    }
    for(const id of group.programs){
      const program=historicalMeasurement(data.programs[id]);assert.equal(program.samples.length,3);
      assert(program.required_work_validated&&program.algorithm.strict);assert.equal(program.algorithm.native_benchmark_kernel,false);
      assert.equal(program.execution_mode,execution.toLowerCase());assert.equal(program.loc_metric,'statement-normalized');
      assert.equal(program.loc,program.physical_loc+program.loc_expansions.reduce((n,e)=>n+e.extra_lines,0));
      assert.equal(program.compiler_sha256,zigCohort.runtimes[key].sha256);
      assert.equal(program.binary_sha256,comparison[id===group.fast?'fastest':'smallest'].binary_sha256);
      assert.equal(program.full_output_validation.length,3);assert(program.full_output_validation.every(v=>v.reference_match));
      assert(data.files[program.files[0]].name.endsWith(extension));assert(program.timing_boundary.includes('Fresh process startup'));
      assert(program.source_url.startsWith('https://'));assert(program.runtime_support.includes('No native benchmark executor')||['roc','elm'].includes(key));
      if(key==='elm'){
        assert.equal(program.files.length,3);assert(program.runtime_policy.includes('--optimize'));
        assert.equal(data.files[program.files[1]].name,'elm-runner.cjs');assert.equal(data.files[program.files[2]].name,'elm.json');
        for(const id of program.files.slice(1)){assert.equal(data.files[id].role,'runtime-support');assert(data.files[id].loc_note.includes('excluded from application LOC'));}
      }else assert.equal(program.files.length,1);
      if(program.upstream_source_sha256)assert.equal(program.license_url,'./programming-language-benchmarks-license.txt');
      if(key==='roc')assert(program.runtime_policy.includes('2025-09-09'));
    }
  }
  assert.equal(full.entries.find(e=>e.language.key===key).spread2.fast.n,10);
}
console.log('Verified all ten Zig/Odin/Roc/Elm workloads, 61 admitted sources and 213 valid samples, statement LOC, exact Mech ratios, pure-language algorithms, exact numeric packing, lifetime/allocation and timed assembly evidence, preserved samples, GitHub colors, and release/execution filters.');

// Whole-set statistics weight plotted variants, not repeated timing samples.
assert.equal(median([]),null);assert.equal(median([8,1,2]),2);assert.equal(median([8,1,2,4]),3);
const fixture=[{fast:{x:1,y:1},small:{x:1,y:4}},{fast:{x:4,y:16},small:{x:4,y:16}}];
const global=globalStatistics(fixture);
assert.deepEqual(global.fast.average,{x:2,y:4});assert.deepEqual(global.fast.median,{x:2.5,y:8.5});
assert.deepEqual(global.small.average,{x:2,y:Math.exp((Math.log(4)+Math.log(16))/2)});
assert.equal(global.fast.spread.covariance[0][0],2);assert.equal(global.fast.spread.covariance[1][1],8);assert.equal(global.fast.spread.covariance[0][1],4);
assert.equal(global.fast.spread2.area,0);assert.equal(global.fast.n,2);
assert.equal(globalStatistics([]).fast,null);assert.equal(globalStatistics(fixture.slice(0,1)).fast.spread,null);
assert.deepEqual(globalStatistics(fixture.slice(0,1)).fast.average,fixture[0].fast);
const familyGroups=languageFamilies(data.languages);
assert.equal(familyGroups.length,37);
assert.deepEqual(familyGroups.find(f=>f.key==='mech').members.map(l=>l.variant_label),['AOT','JIT','Interpreted']);
assert.deepEqual(familyGroups.find(f=>f.key==='lua').members.map(l=>l.key),['lua','luajit']);
const lua=full.entries.find(e=>e.language.key==='lua');
assert.equal(lua.inputs.length,9);assert.deepEqual(lua.coverage.missing.map(b=>b.key),['pidigits']);
assert.equal(data.benchmarks.find(b=>b.key==='pidigits').groups.some(g=>g.language==='lua'),false);
assert(data.language_groups.find(g=>g.key==='interpreted').languages.includes('lua'));
assert(data.language_groups.find(g=>g.key==='jit').languages.includes('luajit'));
const population=globalStatistics(full.entries);
assert.equal(population.fast.n,43);
for(const kind of ['fast','small']){
 assert.equal(population[kind].average.x,gm(full.entries.map(e=>e[kind].x)));
 assert.equal(population[kind].median.y,median(full.entries.map(e=>e[kind].y)));
 assert(Math.abs(population[kind].spread2.area-4*population[kind].spread.area)<1e-10);
}
console.log('Verified grouped execution variants, restored nine-test Lua coverage, and global geometric averages, coordinate medians and covariance.');

const remaining=data.remaining_languages_audit;
assert.equal(remaining.completed_samples,remaining.expected_samples);
assert.equal(remaining.expected_samples,645);
assert.equal(remaining.planned_samples,648);
assert.equal(remaining.sample_count,3);
assert.equal(remaining.full_failures.length,1);
assert.equal(remaining.full_failures[0].key,'regexredux:regexredux-ada-pcre2');
assert.match(remaining.full_failures[0].error,/stack overflow/);
assert.equal(remaining.unresolved_full_failures.length,0);
assert.equal(remaining.repairs.length,1);
assert.equal(remaining.repairs[0].replacement_key,'regexredux:regexredux-ada-pcre2-heap');
assert.equal(Object.keys(remaining.controls).length,11);
assert.equal(remaining.comparisons_to_mech.length,100);
assert(!data.languages.some(l=>['toit','micropython'].includes(l.key)));
assert(!Object.values(data.programs).some(p=>['toit','micropython'].includes(p.language)));
assert(!data.language_groups.some(g=>g.languages.some(k=>['toit','micropython'].includes(k))));
for(const [family,members] of [['csharp',['csharp-aot','csharp']],['fsharp',['fsharp-aot','fsharp']],
  ['dart',['dart','dart-jit']]]){
  assert.deepEqual(familyGroups.find(f=>f.key===family).members.map(l=>l.key),members);
}
for(const language of ['csharp','csharp-aot','fsharp','fsharp-aot','ada','pascal','fortran',
  'dart','dart-jit','smalltalk']){
  const meta=data.languages.find(l=>l.key===language);
  assert.equal(meta.timed_benchmarks.length,10);
  for(const bench of data.benchmarks){
    const group=bench.groups.find(g=>g.language===language);assert(group?.fast&&group?.small);
    const endpoints=remaining.comparisons_to_mech.find(c=>c.language===language&&c.benchmark===bench.key);
    assert.equal(group.fast,endpoints.fastest.key);assert.equal(group.small,endpoints.smallest.key);
    for(const key of group.programs){
      const p=data.programs[key];assert.equal(p.samples.length,3);
      assert(p.required_work_validated&&p.algorithm.strict);assert.equal(p.algorithm.native_benchmark_kernel,false);
      assert.equal(p.full_output_validation.length,3);assert(p.full_output_validation.every(v=>v.reference_match));
      assert.equal(p.loc_metric,'statement-normalized');
      assert.equal(p.loc,p.physical_loc+p.loc_expansions.reduce((n,e)=>n+e.extra_lines,0));
      assert.equal(p.mech_comparison.time_x_mech,p.seconds/p.mech_comparison.seconds);
      assert.equal(p.mech_comparison.fastest_loc_x_mech,p.loc/p.mech_comparison.fastest_loc);
      assert.equal(p.mech_comparison.smallest_loc_x_mech,p.loc/p.mech_comparison.smallest_loc);
      assert(p.timing_boundary.includes('process startup'));assert(p.source_url.startsWith('https://'));
      assert.equal(p.files.length,1);
    }
  }
}
assert(remaining.supplemental_evidence.compiled_units.every(u=>u.included_in_counted_source));
assert.equal(remaining.loc_corrections.length,4);
console.log('Verified remaining language endpoints, complete workloads, seconds and exact Mech time/LOC ratios, counted helpers, family variants, scope exclusions, and separately measured Ada heap repair.');

const nbodyCompiler=data.nbody_compiler_audit;
assert.equal(nbodyCompiler.status,'passed');assert.equal(nbodyCompiler.completed_samples,49);
assert.deepEqual(nbodyCompiler.selected,{fast:'pairwise-seed8',compact:'compact-regions8'});
assert.equal(nbodyCompiler.source_changes,false);assert.equal(nbodyCompiler.specialized_host_kernel,false);
assert.equal(nbodyCompiler.tests.fused,32);assert.equal(nbodyCompiler.tests.native_map,14);
assert(nbodyCompiler.independent_max_scaled_error<=1e-10);
for(const key of nbodyCompiler.updated_endpoints){
  const p=data.programs[key];assert.equal(p.samples.length,7);
  assert.equal(p.report,nbodyCompiler.confirmation_report);assert(p.previous_measurement);
}
assert.equal(data.programs['nbody:compact-baseline'].seconds,1.090478292);
assert.equal(data.programs['nbody:pairwise-mass-tails'].seconds,1.039486459);
assert(data.programs['nbody:compact-baseline'].native_flags.includes('--schedule-regions'));
assert(data.programs['nbody:pairwise-mass-tails'].native_flags.includes('--schedule-regions'));
assert(data.programs['nbody:pairwise-mass-tails'].native_flags.includes('--simplify-square-seeds'));
assert(!data.programs['nbody:pairwise-mass-tails'].native_flags.includes('--share-scaled-products'));
assert.equal(data.programs['nbody:nbody-zig-2-faithful'].seconds,1.006515291);
console.log('Verified seven matched full-size n-body rounds, unchanged counted sources, selected general compiler flags, complete-state audit, and archived earlier cohort controls.');
