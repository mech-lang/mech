// Focused source assertions; run only in root's coordinated common session.
import assert from 'node:assert/strict';
import {languagesByYear,medianHistory,releaseYearsInWindow,clampReleaseWindow,decadeWindow,comparison,chartDomain} from './math.mjs';

const languages=[{key:'early',release_year:1957},{key:'middle',release_year:2000},{key:'recent',release_year:2026}];
const bounds={min:1957,max:2026};
const row=(key,loc,seconds)=>({key,language:key,loc,seconds,x:loc/10,y:seconds/2,samples:[seconds,seconds,seconds]});
const programs={early:row('early',10,2),middle:row('middle',20,4),recent:row('recent',30,6)};
const data={languages,programs,benchmarks:[{key:'one',baseline_loc:10,baseline_seconds:2,
  groups:languages.map(language=>({language:language.key,programs:[language.key],fast:language.key,small:language.key}))}]};
const selection={benchmarks:new Set(['one']),languages:new Set(languages.map(language=>language.key)),disabled:new Set()};
const original=structuredClone(data);

assert.deepEqual(languagesByYear(languages,{year:2000}),['early','middle']);
assert.deepEqual(languagesByYear(languages,{from:2000,to:2000}),['middle']);
assert.deepEqual(languagesByYear(languages,{from:2000,to:2026}),['middle','recent']);
assert.deepEqual(languagesByYear(languages,{from:2026,to:2025}),[]);
assert.deepEqual(releaseYearsInWindow(languages,{from:1957,to:2026}),[1957,2000,2026]);
assert.deepEqual(releaseYearsInWindow([...languages,languages[1]],{from:2000,to:2026}),[2000,2026]);
assert.deepEqual(releaseYearsInWindow(languages,{from:2001,to:2025}),[]);

assert.deepEqual(clampReleaseWindow({from:1957,to:2026},bounds),{from:1957,to:2026});
assert.deepEqual(clampReleaseWindow({from:1900,to:2030},bounds),{from:1957,to:2026});
assert.deepEqual(clampReleaseWindow({from:2020,to:2000},bounds,'start'),{from:2000,to:2000});
assert.deepEqual(clampReleaseWindow({from:2020,to:2000},bounds,'end'),{from:2020,to:2020});
assert.throws(()=>clampReleaseWindow({from:2020,to:2000},bounds));
assert.throws(()=>clampReleaseWindow({from:2000.5,to:2020},bounds));
assert.deepEqual(decadeWindow(1950,bounds),{from:1957,to:1959});
assert.deepEqual(decadeWindow(1960,bounds),{from:1960,to:1969});
assert.deepEqual(decadeWindow(1970,bounds),{from:1970,to:1979});
assert.deepEqual(decadeWindow(1980,bounds),{from:1980,to:1989});
assert.deepEqual(decadeWindow(1990,bounds),{from:1990,to:1999});
assert.deepEqual(decadeWindow(2000,bounds),{from:2000,to:2009});
assert.deepEqual(decadeWindow(2010,bounds),{from:2010,to:2019});
assert.deepEqual(decadeWindow(2020,bounds),{from:2020,to:2026});
assert.equal(decadeWindow(2030,bounds),null);

const whole=medianHistory(data,selection),window=medianHistory(data,selection,{from:2000,to:2026});
assert.deepEqual(whole.map(entry=>entry.year),[1957,2000,2026]);
assert.deepEqual(window.map(entry=>entry.year),[2000,2026]);
assert.equal(window[0].statistics.fast.n,1);
assert.deepEqual(window[0].statistics.fast.median,{x:2,y:2});
assert.equal(window[1].statistics.fast.n,2);
assert.deepEqual(window[1].statistics.fast.median,{x:2.5,y:2.5});
assert.deepEqual(medianHistory(data,selection,{from:2001,to:2025}),[]);
const visible=new Set(languagesByYear(languages,{from:2000,to:2026}));
assert.deepEqual(comparison(data,{...selection,languages:visible}).entries.map(entry=>entry.language.key),['middle','recent']);
const beforeDomain=chartDomain(data,{...selection,view:'benchmark',focus:'one',focusMeans:false});
const windowDomain=chartDomain(data,{...selection,languages:visible,releaseYearStart:2000,releaseYear:2026,view:'benchmark',focus:'one',focusMeans:false});
assert.deepEqual(windowDomain,beforeDomain);
assert.deepEqual(data,original);
console.log('Inclusive window, eight decade presets, uncrossed handles, bounded history and unchanged baselines passed.');
