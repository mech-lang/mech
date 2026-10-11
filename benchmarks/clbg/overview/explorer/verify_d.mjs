import fs from 'node:fs';
import assert from 'node:assert/strict';
import {comparison,gm,languagesByYear} from './dist/math.mjs';
const data=JSON.parse(fs.readFileSync(new URL('./dist/data.json',import.meta.url)));
const d=data.languages.find(l=>l.key==='d');
assert(d);assert.equal(d.label,'D');assert.equal(d.color,'#ba595e');assert.equal(d.execution,'AOT');assert.equal(d.release_year,2001);
assert(!languagesByYear(data.languages,{year:2000}).includes('d'));assert(languagesByYear(data.languages,{year:2001}).includes('d'));
assert(data.language_groups.find(g=>g.key==='aot').languages.includes('d'));
assert.equal(d.timed_benchmarks.length,10);
for(const b of data.benchmarks){
 const group=b.groups.find(g=>g.language==='d');assert(group?.fast&&group?.small);
 for(const key of new Set([group.fast,group.small])){
  const p=data.programs[key];assert.equal(p.language,'d');assert.equal(p.samples.length,3);
  assert.equal(p.full_output_validation.length,3);assert(p.full_output_validation.every(v=>v.reference_match));
  assert(p.required_work_validated&&p.algorithm.strict&&p.algorithm.native_benchmark_kernel===false);
  assert.equal(p.loc,p.physical_loc+p.loc_expansions.reduce((s,e)=>s+e.extra_lines,0));
  assert(p.files.every(f=>data.files[f].name.endsWith('.d')));
 }
}
const selection={benchmarks:new Set(data.benchmarks.map(b=>b.key)),languages:new Set(['d']),disabled:new Set()};
const result=comparison(data,selection);assert.equal(result.entries.length,1);assert.equal(result.entries[0].inputs.length,10);
for(const profile of ['fast','small'])for(const axis of ['x','y'])assert.equal(result.entries[0][profile][axis],gm(result.entries[0].inputs.map(i=>i[profile][axis])));
assert.equal(data.d_audit.samples_completed,data.d_audit.admitted_sources*3+30);
if(process.argv[2]){
 const before=JSON.parse(fs.readFileSync(process.argv[2]));
 for(const [key,p] of Object.entries(before.programs)){
  const current=data.programs[key];assert(current,`Old endpoint removed: ${key}`);
  for(const field of ['loc','seconds','samples','files','binary_sha256'])assert.deepEqual(current[field],p[field],`${key}: ${field}`);
 }
 assert.equal(Object.keys(data.programs).filter(k=>data.programs[k].language!=='d').length,Object.keys(before.programs).length);
}
console.log('Verified D all-ten coverage, source/LOC/output checks, AOT/year selection, geometric means and preservation of existing measurements.');
