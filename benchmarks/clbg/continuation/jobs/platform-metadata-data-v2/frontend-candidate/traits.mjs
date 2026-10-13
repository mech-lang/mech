// Traits describe the language and the execution path measured in this cohort.
// Choices within one dimension are alternatives; dimensions are intersected.
import {platformDimension,measuredPlatform} from './platform.mjs';
export const traitDimensions = [
  platformDimension,
  {key:'execution',label:'Execution',note:'The runtime used for these measurements.',options:[
    {key:'aot',label:'AOT'}, {key:'jit',label:'JIT'}, {key:'interpreted',label:'Interpreted'},
    {key:'unknown',label:'Unclassified'}]},
  {key:'typing',label:'Typing',options:[
    {key:'static',label:'Static'}, {key:'dynamic',label:'Dynamic'}]},
  {key:'style',label:'Language style',options:[
    {key:'array',label:'Array'}, {key:'functional',label:'Functional'}]},
  {key:'memory',label:'Memory management',note:'Runtime strategies can overlap. Foreign libraries are excluded; unverified cases stay unclassified.',options:[
    {key:'gc',label:'Tracing / cycle GC'}, {key:'rc',label:'Reference counting'},
    {key:'owned',label:'Ownership / manual'}, {key:'unknown',label:'Unclassified'}]},
];

const memoryStrategies={
  gc:new Set(['go','java','julia','ruby','javascript','lua','luajit','haskell','ocaml',
    'erlang','lisp','racket','elm','csharp','csharp-aot','fsharp','fsharp-aot',
    'dart','dart-jit','smalltalk','d','r','python','php','bqn','nim','crystal','pony']),
  rc:new Set(['perl','swift','roc','k','uiua','futhark','python','php','bqn',
    'mech','mech-jit','mech-resident','chapel','pascal','nim']),
  owned:new Set(['c','cpp','rust','mojo','zig','odin','ada','fortran',
    'mech','mech-jit','mech-resident','chapel','pascal']),
};

export function languageTraits(data) {
  const groups = new Map(data.language_groups.map(group=>[group.key,new Set(group.languages)]));
  const executionOverrides = {ruby:'jit',php:'jit'};
  return new Map(data.languages.map(language=>{
    const key=language.key;
    const measured=measuredPlatform(data,language);
    const execution=measured.execution_authoritative ? measured.execution??'unknown' :
      executionOverrides[key]??['aot','jit','interpreted'].find(option=>groups.get(option)?.has(key));
    const memory=Object.entries(memoryStrategies).filter(([,members])=>members.has(key)).map(([strategy])=>strategy);
    return [key,{
      platform:measured.platform,
      execution:new Set(execution?[execution]:[]),
      typing:new Set([groups.get('dynamic')?.has(key)?'dynamic':'static']),
      style:new Set(['array','functional'].filter(option=>groups.get(option)?.has(key))),
      memory:new Set(memory.length?memory:['unknown']),
    }];
  }));
}

export const emptyTraitFilters = ()=>Object.fromEntries(traitDimensions.map(dimension=>[dimension.key,new Set()]));

export function matchesTraits(traits,filters,skipDimension=null) {
  return traitDimensions.every(({key})=>key===skipDimension||!filters[key]?.size||
    [...filters[key]].some(option=>traits?.[key]?.has(option)));
}

export function filterLanguageKeys(keys,traits,filters) {
  return new Set([...keys].filter(key=>matchesTraits(traits.get(key),filters)));
}
