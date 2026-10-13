import assert from 'node:assert/strict';
import {medianRankings,globalStatistics,comparison,languagesByYear,recordedSamplePoints,comparisonDensityPoints,densityGrid,chartDomain} from './math.mjs';

const close=(actual,expected)=>assert.ok(Math.abs(actual-expected)<1e-10,`${actual} != ${expected}`);
const entry=(key,speed,loc=speed)=>({language:{key},fast:{x:loc,y:speed},small:{x:loc,y:speed}});
const pair=[entry('a',1),entry('b',9)],rankings=medianRankings(pair);
assert.equal(rankings.speed.median,5); // Coordinate median, not geometric 3.
assert.equal(rankings.speed.median,globalStatistics(pair).fast.median.y);
assert.equal(rankings.loc.median,globalStatistics(pair).small.median.x);
close(rankings.speed.log_sample_sd,Math.log2(9)/Math.sqrt(2));
close(rankings.speed.entries[0].sigma_distance,-Math.log2(5)/rankings.speed.log_sample_sd);
assert.deepEqual(rankings.speed.bands[0].above.map(row=>row.entry.language.key),['b']);
assert.deepEqual(rankings.speed.bands[1].below.map(row=>row.entry.language.key),['a']);
const boundary=medianRankings([entry('lo',.5),entry('mid',1),entry('hi',2)]).speed;
assert.equal(boundary.log_sample_sd,1);
assert.deepEqual(boundary.ties.map(row=>row.entry.language.key),['mid']);
assert.equal(boundary.bands[0].below[0].sigma_distance,-1);
assert.equal(boundary.bands[0].above[0].sigma_distance,1);
assert.equal(boundary.bands.length,1); // The boundary belongs to within 1σ.
assert.equal(boundary.entries.length,boundary.ties.length+boundary.bands.reduce((n,b)=>n+b.below.length+b.above.length,0));
const equal=medianRankings([entry('b',4),entry('a',4)]).speed;
assert.equal(equal.log_sample_sd,0);assert.equal(equal.max_abs_sigma,0);
assert.deepEqual(equal.ties.map(row=>row.entry.language.key),['a','b']);
assert.ok(equal.entries.every(row=>row.sigma_distance===0&&row.ratio_to_median===1));
assert.equal(medianRankings([entry('only',2)]).loc.log_sample_sd,0);
assert.equal(medianRankings([]).speed.median,null);
assert.equal(medianRankings([entry('missing',2,null)]).speed.n,0);
const far=medianRankings([...Array.from({length:17},(_,i)=>entry(`same${i}`,1)),entry('far',2**-100)]).speed;
assert.equal(far.bands.length,5);assert.equal(far.bands[4].below[0].entry.language.key,'far');
const ordered=medianRankings([entry('a',.25),entry('b',.5),entry('mid',1),entry('c',2),entry('d',4)]).speed;
assert.deepEqual(ordered.bands[0].below.map(row=>row.entry.language.key),['b']);
assert.deepEqual(ordered.bands[1].below.map(row=>row.entry.language.key),['a']);

const p=(key,language,benchmark,seconds,loc,samples)=>({key,id:key,language,benchmark,seconds,loc,x:loc,y:seconds,samples});
const a=p('a','old','one',1,1,[.9,1,1.1]),b=p('b','new','one',4,2,[3,4,5]);
const c=p('c','new','two',8,3,[7,8,9]),pending=p('pending','new','one',null,1,[]);
const data={languages:[{key:'old',release_year:1960},{key:'new',release_year:2000}],programs:{a,b,c,pending},benchmarks:[
  {key:'one',baseline_seconds:1,groups:[{language:'old',programs:['a']},{language:'new',programs:['b','pending']}]},
  {key:'two',baseline_seconds:1,groups:[{language:'new',programs:['c']}]}]};
const selection={benchmarks:new Set(['one','two']),languages:new Set(languagesByYear(data.languages,{from:1990,to:2010})),disabled:new Set(['c'])};
const filtered=comparison(data,selection).entries;
assert.equal(filtered.length,1);assert.equal(filtered[0].coverage.measured,1);assert.equal(filtered[0].coverage.selected,2);
assert.equal(medianRankings(filtered).speed.median,4);assert.equal(medianRankings(filtered).loc.median,2);
assert.deepEqual(recordedSamplePoints(filtered).map(point=>point.elapsed),[3,4,5]);
assert.deepEqual(recordedSamplePoints(filtered,{fast:false,small:false,global:false}).map(point=>point.y),[3,4,5]);
assert.equal(recordedSamplePoints(filtered,{samples:false}).length,0);
assert.equal(recordedSamplePoints(filtered,{disabled:new Set(['b'])}).length,0);
assert.equal(recordedSamplePoints([...filtered,...filtered]).length,3);
assert.equal(recordedSamplePoints(filtered,{programsForInput:input=>[...input.programs,...input.programs,pending]}).length,3);
assert.equal(recordedSamplePoints(filtered)[0].program,b);
const outlier={...data,programs:{...data.programs,a:{...a,samples:[.9,1,1000]}}};
const fullSelection={benchmarks:new Set(['one','two']),languages:new Set(['old','new']),disabled:new Set()};
const meansDomain=chartDomain(outlier,{...fullSelection,focusMeans:true});
const fullDomain=chartDomain(outlier,{...fullSelection,focusMeans:false});
assert.ok(fullDomain.yMax>=Math.log2(1000)+.25);assert.ok(meansDomain.yMax<Math.log2(1000));
assert.ok(chartDomain(outlier,{...fullSelection,view:'benchmark',focus:'one'}).yMax>=Math.log2(1000)+.25);
assert.deepEqual(fullDomain,chartDomain(outlier,{...fullSelection,languages:new Set(['new']),disabled:new Set(['a']),fast:false,small:false,samples:false,focusMeans:false}));

const profiles=[{language:{key:'split'},fast:{x:1,y:1},small:{x:4,y:4}},entry('coincident',2)];
const points=comparisonDensityPoints(profiles);
assert.equal(points.length,3);close(points.reduce((n,point)=>n+point.weight,0),2);
assert.deepEqual(points.map(point=>point.weight),[.5,.5,1]);
assert.deepEqual(points[2].profiles,['fast','small']);
assert.equal(comparisonDensityPoints(profiles,{fast:false,small:false}).length,0);
assert.deepEqual(comparisonDensityPoints(profiles,{small:false}).map(point=>point.weight),[1,1]);
const domain={xMin:-2,xMax:2,yMin:-2,yMax:2};
const field=densityGrid([{x:1,y:1,weight:1,language:'one'}],domain,{columns:4,rows:4});
assert.equal(field.cells.length,16);assert.equal(field.variants,1);assert.equal(field.profile_positions,1);
assert.equal(field.total_variant_weight,1);close(field.bandwidth_x,1.25);close(field.bandwidth_y,1.25);
close(field.cells[0].value,field.cells[15].value);assert.ok(field.cells[5].value>field.cells[0].value);
assert.ok(field.cells.every(cell=>Number.isFinite(cell.value)&&cell.value>0));
const empty=densityGrid([],domain,{columns:4,rows:4});
assert.equal(empty.cells.length,16);assert.equal(empty.peak,0);assert.ok(empty.cells.every(cell=>cell.value===0));
assert.throws(()=>densityGrid(points,{...domain,xMax:domain.xMin}));
// Run counts never enter density: its input contains only selected profile positions.
assert.equal(densityGrid(comparisonDensityPoints(filtered),domain).profile_positions,1);
assert.equal(recordedSamplePoints(filtered).length,3);
console.log('Median rankings, recorded samples and density assertions passed.');
