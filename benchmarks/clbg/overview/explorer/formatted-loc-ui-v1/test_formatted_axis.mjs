// Unexecuted source fixtures. Run only with the shared timing mutex held.
import assert from 'node:assert/strict';
import {axisData,hasFormattedLOC,formattedMechRatio,inspectorProgram} from './formatted_axis.mjs';
import {comparison,chartDomain,medianHistory} from './math.mjs';
const p=(key,loc,seconds,formatted,status='qualified')=>({key,id:key,language:key,benchmark:'nbody',loc,
  seconds,samples:[seconds,seconds,seconds],x:loc/10,y:seconds,
  formatted_loc:formatted,formatted_loc_status:status,formatted_loc_complete:status==='qualified',
  formatted_x:formatted==null?null:formatted/20});
const data={languages:[{key:'mech',label:'Mech',release_year:2026},{key:'crystal',label:'Crystal',release_year:2014}],
  programs:{mech:p('mech',10,1,null,'unavailable'),crystal:p('crystal',30,2,20)},
  benchmarks:[{key:'nbody',baseline_loc:10,baseline_seconds:1,baseline_formatted_loc:20,groups:[
    {language:'mech',programs:['mech'],fast:'mech',small:'mech',formatted_fast:null,formatted_small:null},
    {language:'crystal',programs:['crystal'],fast:'crystal',small:'crystal',formatted_fast:'crystal',formatted_small:'crystal'}]}]};
const selected={languages:new Set(['mech','crystal']),benchmarks:new Set(['nbody']),disabled:new Set()};
const saved=structuredClone(data),projected=axisData(data,'formatted');
assert.deepEqual(data,saved);
assert.equal(axisData(data,'canonical'),data);
assert.equal(projected.programs.mech.loc,null);
assert.equal(projected.programs.crystal.loc,20);
assert.equal(projected.programs.crystal.x,1);
assert.equal(projected.programs.crystal.y,2);
assert.deepEqual(comparison(projected,selected).entries.map(e=>e.language.key),['crystal']);
// A missing formatted Mech endpoint still refreshes the original source in
// the inspector; it cannot turn into a numeric formatted chart candidate.
assert.equal(inspectorProgram(data,projected,{language:'mech',benchmark:'nbody',kind:'fast'},new Set()),data.programs.mech);
assert.equal(inspectorProgram(data,projected,{language:'crystal',benchmark:'nbody',kind:'small'},new Set()),data.programs.crystal);
assert.equal(projected.programs.mech.x,null);
assert.equal(hasFormattedLOC(data.programs.mech),false);
// Manual/year visibility never recomputes the admitted formatter baseline.
assert.equal(comparison(projected,{...selected,languages:new Set(['crystal'])}).entries[0].small.x,1);
const domain=chartDomain(projected,{benchmarks:new Set(['nbody'])});
assert.ok(Object.values(domain).every(Number.isFinite));
assert.ok(medianHistory(projected,selected).every(item=>item.statistics));
assert.equal(formattedMechRatio({...data.programs.crystal,mech_comparison:{control_loc:10}}),null);
assert.equal(formattedMechRatio({...data.programs.crystal,mech_comparison:{formatted_loc_evidence:{
  status:'qualified',complete:true,formatted_loc:10,source_identity_matches:true}}}),null);
assert.equal(hasFormattedLOC({...data.programs.crystal,formatted_loc:20.5}),false);
assert.equal(hasFormattedLOC({...data.programs.crystal,formatted_loc_complete:1}),false);
assert.equal(hasFormattedLOC({...data.programs.crystal,formatted_loc_status:'failed'}),false);
assert.throws(()=>axisData(data,'physical'));
