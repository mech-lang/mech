// Unexecuted candidate fixtures. Run only under the common timing mutex.
import assert from 'node:assert/strict';
import {measuredPlatform} from './platform.mjs';
import {languageTraits,emptyTraitFilters,matchesTraits,filterLanguageKeys} from './traits.mjs';
import {chartDomain,comparison,languagesByYear} from '../../overview/explorer/dist/math.mjs';

const digest='a'.repeat(64),source='b'.repeat(64);
const kinds={java:['openjdk-jit','jit',1995],clojure:['jvm-jit','jit',2007],
  gleam:['beam-jit','jit',2016],erlang:['beam-jit','jit',1986],
  csharp:['dotnet-jit','jit',2000],'csharp-aot':['dotnet-native-aot','aot',2000],
  fsharp:['dotnet-jit','jit',2005],'fsharp-aot':['dotnet-native-aot','aot',2005],
  graal:['graalvm-native-aot','aot',1995],c:['native-aot','aot',1972],
  javascript:['javascript-jit','jit',1995],mystery:[null,null,2020]};
const data={languages:Object.entries(kinds).map(([key,[,,year]])=>({key,label:key,release_year:year})),
  files:{[source]:{sha256:source}},provenance:{'report.json':digest,'runtime.json':digest},
  language_groups:[{key:'aot',languages:['java','csharp-aot','fsharp-aot','graal','c']},
    {key:'jit',languages:['clojure','gleam','erlang','csharp','fsharp','javascript']},
    {key:'dynamic',languages:['clojure','erlang','javascript']},
    {key:'functional',languages:['clojure','gleam','erlang','fsharp','fsharp-aot']}],
  benchmarks:[],programs:{}};
for(const [index,[language,[key,execution]] ]of Object.entries(kinds).entries()){
  const programKey=`toy:${language}`;
  data.programs[programKey]={key:programKey,benchmark:'toy',language,report:'report.json',
    seconds:index+1,loc:index+2,x:index+2,y:index+1,samples:[index+1,index+1,index+1],files:[source],
    ...(key?{measured_backend:{key,execution,label:`Measured ${key}`,verified:true,evidence:{verified:true,
      report:'report.json',report_sha256:digest,source_sha256:[source],command:['measured-runtime','main'],
      runtime_receipt:{verified:true,report:'runtime.json',report_sha256:digest,field:'version',value:'Saved runtime version'}}}}:{})};
}
data.benchmarks=[{key:'toy',title:'Toy',groups:Object.keys(kinds).map(language=>({language,programs:[`toy:${language}`]}))}];
let traits=languageTraits(data);
const filters=emptyTraitFilters();
const manual=new Set(Object.keys(kinds));
const results=()=>[...filterLanguageKeys(manual,traits,filters)].sort();
assert.deepEqual(results(),[...manual].sort(),'Empty platform category includes all entries');
assert.deepEqual([...traits.get('java').execution],['jit'],'Qualified Java receipts repair the stale AOT group');
assert.deepEqual([...traits.get('java').platform],['jvm']);
assert.deepEqual([...traits.get('graal').platform],['native'],'GraalVM NativeAOT is not JVM');
assert.deepEqual([...traits.get('mystery').platform],['unknown'],'Language identity supplies no platform');
filters.platform.add('beam');
assert.deepEqual(results(),['erlang','gleam']);
filters.platform.add('jvm');
assert.deepEqual(results(),['clojure','erlang','gleam','java'],'OR within platform');
filters.typing.add('static');
assert.deepEqual(results(),['gleam','java'],'AND across platform and typing');
assert.equal(matchesTraits(traits.get('csharp'),filters,'platform'),true,'Counts ignore only their own platform');
filters.typing.clear();filters.platform.clear();filters.platform.add('dotnet');
assert.deepEqual(results(),['csharp','csharp-aot','fsharp','fsharp-aot'],'.NET includes both measured backends');
filters.execution.add('aot');
assert.deepEqual(results(),['csharp-aot','fsharp-aot'],'Execution distinguishes NativeAOT');
filters.execution.clear();filters.platform.clear();filters.platform.add('beam');
const year=new Set(languagesByYear(data.languages,{year:1995}));
assert.deepEqual(results().filter(key=>year.has(key)),['erlang'],'Release year still intersects');
assert.deepEqual([...filterLanguageKeys(['java'],traits,filters)],[],'Manual selection stays a constraint');
assert.deepEqual([...manual].sort(),Object.keys(kinds).sort(),'Filters never mutate manual choices');
const selection={benchmarks:new Set(['toy']),languages:manual,disabled:new Set(),view:'summary'};
const fixed=chartDomain(data,selection);
const visible=filterLanguageKeys(manual,traits,filters);
assert.deepEqual(comparison(data,{...selection,languages:visible}).entries.map(row=>row.language.key),['gleam','erlang']);
assert.deepEqual(chartDomain(data,{...selection,languages:visible,releaseYear:1995,traits:filters}),fixed,'Axes retain the full cohort');

const java=data.programs['toy:java'];
const original=structuredClone(java);
for(const bad of [false,null,1,'true',undefined]){
  java.measured_backend.verified=bad;
  assert.deepEqual([...measuredPlatform(data,{key:'java'}).platform],['unknown'],'Qualification is literal True');
  java.execution_mode='aot';
  assert.deepEqual([...languageTraits(data).get('java').execution],['unknown'],
    'An explicit unresolved backend blocks the legacy Java AOT execution group');
}
data.programs['toy:java']=structuredClone(original);
for(const mutate of [
  program=>program.measured_backend.evidence.report_sha256='c'.repeat(64),
  program=>program.measured_backend.evidence.source_sha256=['d'.repeat(64)],
  program=>program.measured_backend.evidence.runtime_receipt.verified=false,
  program=>program.measured_backend.evidence.command=[],
  program=>program.execution_mode='aot',
]){
  const altered=structuredClone(original);mutate(altered);data.programs['toy:java']=altered;
  assert.deepEqual([...measuredPlatform(data,{key:'java'}).platform],['unknown'],'Contradictory or missing proof stays unclassified');
}
data.programs['toy:java']=structuredClone(original);
data.programs['toy:java-second']={...structuredClone(original),key:'toy:java-second'};
delete data.programs['toy:java-second'].measured_backend;
assert.deepEqual([...measuredPlatform(data,{key:'java'}).platform],['unknown'],'Every admitted measured profile needs proof');
assert.deepEqual([...languageTraits(data).get('java').execution],['unknown'],
  'A partially annotated language cannot fall back to the old execution group');
data.programs['toy:java-second']=structuredClone(data.programs['toy:graal']);
data.programs['toy:java-second'].language='java';
assert.deepEqual([...measuredPlatform(data,{key:'java'}).platform],['unknown'],'Mixed platform aggregates require explicit separation');
assert.deepEqual([...languageTraits(data).get('java').execution],['unknown']);

data.languages.push({key:'ruby',label:'ruby',release_year:1995});
data.programs['toy:ruby']={...structuredClone(original),key:'toy:ruby',language:'ruby',execution_mode:'jit',
  measured_backend:{verified:false,unresolved_reason:'Synthetic unavailable historical runtime'}};
assert.deepEqual([...languageTraits(data).get('ruby').execution],['unknown'],
  'An unresolved measured backend blocks the fixed Ruby JIT override');
delete data.programs['toy:ruby'].measured_backend;
assert.deepEqual([...languageTraits(data).get('ruby').execution],['jit'],
  'An unannotated historical language retains its separately established legacy execution policy');
const memoryLanguage=data.languages.find(item=>item.key==='gleam');
for(const [key,platform,execution] of [
  ['native-jit','native','jit'],
  ['native-interpreter','native','interpreted'],
  ['native-runtime','native','unknown'],
  ['javascript-runtime','javascript','unknown'],
]){
  const runtimeData=structuredClone(data);
  const program=structuredClone(original);
  program.language='runtime-probe';
  program.key='toy:runtime-probe';
  program.execution_mode=execution;
  program.measured_backend.key=key;
  program.measured_backend.execution=execution;
  program.measured_backend.label='Synthetic qualified runtime fixture';
  program.measured_backend.confidence={historical_platform_verified:true,
    historical_execution_verified:execution!=='unknown',historical_executable_bytes_verified:false};
  runtimeData.programs={['toy:runtime-probe']:program};
  runtimeData.languages=[{key:'runtime-probe',label:'Runtime fixture'}];
  assert.deepEqual([...measuredPlatform(runtimeData,{key:'runtime-probe'}).platform],[platform]);
  assert.deepEqual([...languageTraits(runtimeData).get('runtime-probe').execution],[execution],
    'An established runtime platform can retain an explicitly unknown execution mode');
  for(const mutate of [
    record=>record.confidence.historical_platform_verified=false,
    record=>record.confidence.historical_platform_verified='true',
    record=>record.confidence.historical_execution_verified=execution==='unknown',
    record=>record.confidence.historical_executable_bytes_verified=1,
    record=>delete record.confidence.historical_executable_bytes_verified,
  ]){
    const changed=structuredClone(program);mutate(changed.measured_backend);
    runtimeData.programs[program.key]=changed;
    assert.deepEqual([...measuredPlatform(runtimeData,{key:'runtime-probe'}).platform],['unknown'],
      'Explicit platform/mode confidence contradictions remain unclassified');
    assert.deepEqual([...languageTraits(runtimeData).get('runtime-probe').execution],['unknown']);
  }
  runtimeData.programs[program.key]=program;
  if(execution==='unknown'){
    const another=structuredClone(program);
    another.key='toy:runtime-probe-jit';
    another.execution_mode='jit';
    another.measured_backend.key=platform==='native'?'native-jit':'javascript-jit';
    another.measured_backend.execution='jit';
    another.measured_backend.confidence.historical_execution_verified=true;
    runtimeData.programs[another.key]=another;
    assert.deepEqual([...measuredPlatform(runtimeData,{key:'runtime-probe'}).platform],[platform],
      'A shared proved platform survives different execution modes');
    assert.deepEqual([...languageTraits(runtimeData).get('runtime-probe').execution],['unknown'],
      'Mixed execution modes cannot become a verified aggregate JIT claim');
  }
}
const memoryEvidence={path:'synthetic/gleam-metadata.json',sha256:'e'.repeat(64)};
data.provenance[memoryEvidence.path]=memoryEvidence.sha256;
memoryLanguage.measured_memory_policy={verified:true,strategies:['gc'],label:'Pinned measured BEAM runtime',evidence:memoryEvidence};
assert.deepEqual([...languageTraits(data).get('gleam').memory],['gc'],
  'New language memory comes from pinned actual cohort metadata');
for(const mutate of [
  record=>record.verified='true',
  record=>record.strategies=['gc','gc'],
  record=>record.strategies=['invented'],
  record=>record.evidence.sha256='f'.repeat(64),
  record=>record.label='',
]){
  memoryLanguage.measured_memory_policy=structuredClone({verified:true,strategies:['gc'],label:'Pinned runtime',evidence:memoryEvidence});
  mutate(memoryLanguage.measured_memory_policy);
  assert.deepEqual([...languageTraits(data).get('gleam').memory],['unknown'],
    'Unqualified memory evidence cannot become a runtime filter fact');
}
console.log('Platform evidence and selection fixtures passed.');
