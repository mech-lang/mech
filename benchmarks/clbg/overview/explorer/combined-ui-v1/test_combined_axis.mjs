// Source-authored integration fixtures. Execute only in root's common session.
import assert from 'node:assert/strict';
import {axisData,projectProgram,formattedText,formattedValue,hasFormattedLOC,inspectorProgram} from './formatted_axis.mjs';
import {comparison,chartDomain,languagesByYear,medianHistory,releaseYearsInWindow,clampReleaseWindow,decadeWindow,medianRankings,recordedSamplePoints,comparisonDensityPoints} from './math.mjs';
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
const density=comparisonDensityPoints(result.entries);
assert.equal(density.length,2);assert.equal(density.reduce((n,p)=>n+p.weight,0),1);
const onlyThird=comparison(formatted,{...selection,disabled:new Set(['fast'])}).entries;
assert.equal(recordedSamplePoints(onlyThird).length,3);assert.equal(comparisonDensityPoints(onlyThird).length,1);
assert.equal(medianRankings(comparison(formatted,{...selection,languages:new Set(['vm'])}).entries).speed.n,0);
const domain=chartDomain(formatted,{...selection,view:'benchmark',focus:'one',focusMeans:false});
assert.ok(Object.values(domain).every(Number.isFinite));
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
assert.deepEqual(data,original);
