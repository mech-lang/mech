import assert from 'node:assert/strict';
import {languageTraits,emptyTraitFilters,matchesTraits,filterLanguageKeys} from './dist/traits.mjs';

const keys=['javascript','julia','lua','luajit','ruby','php','mech','mech-jit','mech-resident','futhark'];
const data={languages:keys.map(key=>({key})),language_groups:[
  {key:'dynamic',languages:['javascript','julia','lua','luajit','ruby','php']},
  {key:'array',languages:['julia','mech','mech-jit','mech-resident','futhark']},
  {key:'functional',languages:['mech','mech-jit','mech-resident','futhark']},
  {key:'aot',languages:['mech','futhark']},
  {key:'jit',languages:['javascript','julia','luajit','mech-jit']},
  {key:'interpreted',languages:['lua','ruby','php','mech-resident']},
]};
const traits=languageTraits(data),filters=emptyTraitFilters();
const result=()=>[...filterLanguageKeys(keys,traits,filters)].sort();
assert.deepEqual(result(),[...keys].sort());
filters.style.add('array');
assert.deepEqual(result(),['futhark','julia','mech','mech-jit','mech-resident']);
filters.typing.add('dynamic');
assert.deepEqual(result(),['julia'],'Array and Dynamic intersect; JavaScript stays excluded');
filters.typing.add('static');
assert.deepEqual(result(),['futhark','julia','mech','mech-jit','mech-resident'],'Typing choices are alternatives');
filters.execution.add('aot');
assert.deepEqual(result(),['futhark','mech']);
filters.execution.add('jit');
assert.deepEqual(result(),['futhark','julia','mech','mech-jit']);
assert.equal(matchesTraits(traits.get('mech-resident'),filters,'execution'),true,'Option counts ignore only their own dimension');
filters.typing.delete('static');
filters.execution.delete('jit');
assert.deepEqual(result(),[],'An empty intersection is preserved, not broadened');
filters.execution.clear();
assert.deepEqual(result(),['julia']);
assert.deepEqual([...filterLanguageKeys(['javascript'],traits,filters)],[],'Manual language selection remains a constraint');
assert.deepEqual([...traits.get('ruby').execution],['jit'],'Measured Ruby uses YJIT');
assert.deepEqual([...traits.get('php').execution],['jit'],'Measured PHP uses tracing JIT');
assert.deepEqual([...traits.get('lua').execution],['interpreted']);
assert.deepEqual([...traits.get('luajit').execution],['jit']);
assert.deepEqual([...traits.get('mech-resident').typing],['static']);
assert(traits.get('php').memory.has('gc')&&traits.get('php').memory.has('rc'));
assert(traits.get('mech').memory.has('owned')&&traits.get('mech').memory.has('rc'));
filters.memory.add('gc');
assert.deepEqual(result(),['julia'],'Memory management also intersects the existing constraints');
filters.memory.clear();
console.log('Language trait intersection checks passed.');
