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
data.programs['toy:java-second']=structuredClone(data.programs['toy:graal']);
data.programs['toy:java-second'].language='java';
assert.deepEqual([...measuredPlatform(data,{key:'java'}).platform],['unknown'],'Mixed platform aggregates require explicit separation');
console.log('Platform evidence and selection fixtures passed.');
