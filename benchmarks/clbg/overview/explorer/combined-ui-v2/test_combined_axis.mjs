// Source-authored integration fixtures. Execute only in root's common session.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {axisData,projectProgram,formattedText,formattedValue,hasFormattedLOC,inspectorProgram} from './formatted_axis.mjs';
import {comparison,chartDomain,languagesByYear,medianHistory,releaseYearsInWindow,clampReleaseWindow,decadeWindow,medianRankings,recordedSamplePoints,sampleDensityPoints,densityGrid,chartLayerState} from './math.mjs';
import {languageTraits,emptyTraitFilters,filterLanguageKeys} from './traits.mjs';

const sourceSHA='a'.repeat(64),reportSHA='b'.repeat(64),runtimeSHA='c'.repeat(64);
const backend=(key,execution)=>({key,execution,label:key,verified:true,evidence:{verified:true,
  report:'timing.json',report_sha256:reportSHA,command:['/actual/image'],source_sha256:[sourceSHA],
  runtime_receipt:{verified:true,report:'runtime.json',report_sha256:runtimeSHA,field:'/actual_version',value:'recorded version'}}});
const program=(key,language,loc,seconds,formatted_loc,formatted_x)=>({key,id:key,language,benchmark:'one',
  loc,x:loc/10,seconds,y:seconds/2,samples:[seconds-.1,seconds,seconds+.1],files:['source'],report:'timing.json',
  formatted_loc,formatted_x,formatted_loc_status:formatted_loc==null?'pending':'qualified',formatted_loc_complete:formatted_loc!=null,
  formatted_loc_evidence:{},measured_backend:backend(language==='vm'?'openjdk-jit':'native-aot',language==='vm'?'jit':'aot')});
const fast=program('fast','native',30,2,80,16),small=program('small','native',10,8,null,null);
const third=program('third','native',40,3,5,1),vm=program('vm','vm',20,4,null,null);
const data={languages:[{key:'native',label:'Native',release_year:2020},{key:'vm',label:'VM',release_year:2000}],
  language_groups:[{key:'aot',languages:['native']},{key:'jit',languages:['vm']},{key:'dynamic',languages:[]}],
  programs:{fast,small,third,vm},program_inventory:{fast,small,third,vm},files:{source:{sha256:sourceSHA,code:'original'}},
  provenance:{'timing.json':reportSHA,'runtime.json':runtimeSHA},benchmarks:[{key:'one',title:'One',baseline_loc:10,
    baseline_seconds:2,baseline_formatted_loc:5,groups:[{language:'native',programs:['fast','small','third'],fast:'fast',small:'small',
      formatted_fast:'fast',formatted_small:'third'},{language:'vm',programs:['vm'],fast:'vm',small:'vm'}]}]};
const original=structuredClone(data),selection={benchmarks:new Set(['one']),languages:new Set(['native','vm']),disabled:new Set()};
const canonical=comparison(data,selection),formatted=axisData(data,'formatted');
assert.equal(axisData(data,'canonical'),data);
assert.equal(formatted.program_inventory.third.loc,5);
assert.equal(formatted.programs.small.loc,null);
assert.equal(formatted.programs.small.x,null);
assert.equal(formatted.benchmarks[0].baseline_loc,5);
assert.equal(data.benchmarks[0].baseline_loc,10);
assert.equal(formatted.benchmarks[0].baseline_seconds,2);
assert.equal(formatted.programs.third.y,third.y);
assert.deepEqual(formatted.programs.third.samples,third.samples);
assert.equal(formattedValue(small),'—');
assert.equal(formattedText(small),'— formatted LOC (pending)');
assert.equal(hasFormattedLOC({...third,formatted_loc_complete:false}),false);
const missingCoordinate=projectProgram({...third,formatted_x:null},'formatted');
assert.equal(missingCoordinate.loc,5);
assert.equal(missingCoordinate.x,null);
const result=comparison(formatted,selection);
assert.deepEqual(result.entries.map(e=>e.language.key),['native']);
assert.equal(result.entries[0].inputs[0].fast.key,'fast');
assert.equal(result.entries[0].inputs[0].small.key,'third');
assert.equal(canonical.entries[0].inputs[0].small.key,'small');
// Ranking, recorded runs and density use only eligible coordinates from the
// selected metric. Missing formatted LOC retains no artificial X position.
const rankings=medianRankings(result.entries);
assert.equal(rankings.speed.n,1);assert.equal(rankings.speed.median,fast.y);
assert.equal(rankings.loc.median,third.formatted_x);
const runs=recordedSamplePoints(result.entries,{fast:false,small:false,global:false});
assert.equal(runs.length,6);
assert.deepEqual([...new Set(runs.map(point=>point.program.key))],['fast','third']);
assert.deepEqual(runs.filter(point=>point.program.key==='third').map(point=>point.elapsed),third.samples);
assert.ok(runs.filter(point=>point.program.key==='third').every(point=>point.x===third.formatted_x));
const density=sampleDensityPoints(runs);
assert.equal(density.length,6);assert.equal(density.reduce((n,p)=>n+p.weight,0),6);
assert.equal(new Set(density.map(point=>point.program)).size,2);
const onlyThird=comparison(formatted,{...selection,disabled:new Set(['fast'])}).entries;
assert.equal(recordedSamplePoints(onlyThird).length,3);assert.equal(sampleDensityPoints(recordedSamplePoints(onlyThird)).length,3);
assert.equal(medianRankings(comparison(formatted,{...selection,languages:new Set(['vm'])}).entries).speed.n,0);
const domain=chartDomain(formatted,{...selection,view:'benchmark',focus:'one',focusMeans:false});
assert.ok(Object.values(domain).every(Number.isFinite));
const field=densityGrid(density,domain);
assert.equal(field.runs,6);assert.equal(field.programs,2);
for(const samples of ['off','dim','highlight'])for(const medians of ['off','dim','highlight'])for(const density of ['off','dim','highlight']){
  const layers=chartLayerState({samples,medians,density,fast:false,small:false,sampleCount:runs.length,entryCount:result.entries.length});
  assert.equal(layers.showSamples,samples!=='off');assert.equal(layers.showMedians,false);
  assert.equal(layers.showDensity,density!=='off');
  assert.deepEqual(densityGrid(sampleDensityPoints(recordedSamplePoints(result.entries)),domain),field);
}
assert.equal(inspectorProgram(data,formatted,{language:'vm',benchmark:'one',kind:'fast',program:null},new Set()),vm);
assert.equal(inspectorProgram(data,formatted,{program:'small'},new Set()),small);
assert.deepEqual(data,original);

// Inclusive windows remove older releases from both the visible cohort and
// its history. Presets and either moving handle cannot cross the other end.
assert.deepEqual(languagesByYear(data.languages,{from:2000,to:2000}),['vm']);
assert.deepEqual(languagesByYear(data.languages,{from:2001,to:2020}),['native']);
assert.deepEqual(languagesByYear(data.languages,{from:2021,to:2020}),[]);
assert.deepEqual(releaseYearsInWindow(data.languages,{from:2000,to:2020}),[2000,2020]);
assert.deepEqual(releaseYearsInWindow(data.languages,{from:2001,to:2019}),[]);
const recent=medianHistory(data,selection,{from:2010,to:2020});
assert.deepEqual(recent.map(item=>item.year),[2020]);
assert.equal(recent[0].statistics.fast.n,1);
assert.deepEqual(recent[0].statistics.fast.median,{x:fast.x,y:fast.y});
assert.deepEqual(medianHistory(data,selection,{from:2001,to:2019}),[]);
assert.deepEqual(clampReleaseWindow({from:2021,to:2000},{min:1957,max:2026},'start'),{from:2000,to:2000});
assert.deepEqual(clampReleaseWindow({from:2020,to:1990},{min:1957,max:2026},'end'),{from:2020,to:2020});
assert.throws(()=>clampReleaseWindow({from:2020,to:1990},{min:1957,max:2026}));
assert.deepEqual(decadeWindow(1950,{min:1957,max:2026}),{from:1957,to:1959});
assert.deepEqual(decadeWindow(1990,{min:1957,max:2026}),{from:1990,to:1999});
assert.deepEqual(decadeWindow(2020,{min:1957,max:2026}),{from:2020,to:2026});
assert.equal(decadeWindow(2030,{min:1957,max:2026}),null);
assert.deepEqual(data,original);

// Backend filters use qualified original measurements, independently of LOC
// qualification. A hidden conflicting inventory form cannot be discarded.
const traits=languageTraits(data),filters=emptyTraitFilters();
filters.platform.add('jvm');
assert.deepEqual([...filterLanguageKeys(new Set(['native','vm']),traits,filters)],['vm']);
const hidden={...vm,key:'hidden',id:'hidden',measured_backend:backend('native-aot','aot')};
const complete={...data,programs:{...data.programs,hidden}};
assert.deepEqual([...languageTraits(complete).get('vm').platform],['unknown']);
const unproved={...data,provenance:{'timing.json':reportSHA}};
assert.deepEqual([...languageTraits(unproved).get('vm').platform],['unknown']);
const uncertain=structuredClone(data);
uncertain.programs.vm.measured_backend=backend('native-runtime','unknown');
uncertain.programs.vm.measured_backend.confidence={historical_platform_verified:true,
  historical_execution_verified:false,historical_executable_bytes_verified:false};
assert.deepEqual([...languageTraits(uncertain).get('vm').platform],['native']);
assert.deepEqual([...languageTraits(uncertain).get('vm').execution],['unknown']);
uncertain.programs.vm.measured_backend.confidence.historical_execution_verified=true;
assert.deepEqual([...languageTraits(uncertain).get('vm').platform],['unknown']);
assert.deepEqual(data,original);

// Optional actual immutable candidate: root supplies this path after closing
// its metadata-only integration. This does not replace any producer fixtures.
if(process.argv[2]){
  const actual=JSON.parse(readFileSync(process.argv[2],'utf8'));
  assert.equal(Object.keys(actual.programs).length,798);
  assert.equal(Object.keys(actual.program_inventory).length,80);
  const all={...actual.program_inventory,...actual.programs},saved=structuredClone(actual);
  const crystal=Object.values(all).filter(p=>p.language==='crystal');
  assert.equal(crystal.length,20);
  assert.ok(crystal.every(hasFormattedLOC));
  assert.ok(Object.values(all).filter(p=>p.language!=='crystal').every(p=>!hasFormattedLOC(p)));
  for(const [key,p] of Object.entries(actual.program_inventory))if(Object.hasOwn(actual.programs,key)){
    for(const field of ['loc','physical_loc','x','y','seconds','samples','files','measured_backend','execution_mode',
      'formatted_loc','formatted_loc_status','formatted_loc_complete','formatted_loc_evidence','formatted_x'])
      assert.deepEqual(p[field],actual.programs[key][field],`Shared identity ${key} ${field}`);
  }
  const projected=axisData(actual,'formatted');
  assert.equal(axisData(actual,'canonical'),actual);
  for(const [key,p] of Object.entries(actual.programs)){
    assert.equal(projected.programs[key].y,p.y);
    assert.equal(projected.programs[key].seconds,p.seconds);
    assert.deepEqual(projected.programs[key].samples,p.samples);
    assert.deepEqual(projected.programs[key].files,p.files);
    if(p.language!=='crystal'){
      assert.equal(projected.programs[key].loc,null);
      assert.equal(projected.programs[key].x,null);
    }
  }
  for(const [i,b] of actual.benchmarks.entries()){
    assert.equal(projected.benchmarks[i].baseline_seconds,b.baseline_seconds);
    assert.ok(Number.isSafeInteger(b.baseline_formatted_loc)&&b.baseline_formatted_loc>0);
  }
  const selection={benchmarks:new Set(actual.benchmarks.map(b=>b.key)),
    languages:new Set(actual.languages.map(l=>l.key)),disabled:new Set()};
  const entries=comparison(projected,selection).entries;
  assert.deepEqual(entries.map(entry=>entry.language.key),['crystal']);
  assert.equal(entries[0].inputs.length,10);
  const endpointRuns=recordedSamplePoints(entries);
  assert.ok(endpointRuns.length>0&&endpointRuns.length<=60);
  // Match the app's highlighted language-overview population: all qualified
  // inventory forms for each admitted benchmark, not only endpoint forms.
  const runs=recordedSamplePoints(entries,{programsForInput:(input,entry)=>
    Object.values(projected.program_inventory).filter(p=>p.language===entry.language.key&&p.benchmark===input.benchmark.key)});
  assert.equal(runs.length,60);
  assert.equal(new Set(runs.map(run=>run.program.key)).size,20);
  const field=densityGrid(sampleDensityPoints(runs),chartDomain(projected,{...selection,focusMeans:false}));
  assert.equal(field.runs,60);assert.equal(field.programs,20);
  assert.equal(medianRankings(entries).speed.n,1);
  const traits=languageTraits({...actual,programs:all}),jvm=emptyTraitFilters();
  jvm.platform.add('jvm');
  const keys=filterLanguageKeys(new Set(actual.languages.map(l=>l.key)),traits,jvm);
  assert.ok(keys.has('clojure'));
  assert.ok(!keys.has('crystal'));
  assert.deepEqual(actual,saved);
}
