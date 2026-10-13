import {comparison,same,chartDomain,comparisonMetrics,languagesByYear,globalStatistics,languageFamilies,medianHistory,median,releaseYearsInWindow,clampReleaseWindow,decadeWindow,medianRankings,recordedSamplePoints,sampleDensityPoints,densityGrid,layerPresentation,chartLayerState,summaryNumber} from './math.mjs?v=20261013-layers';
import {traitDimensions,languageTraits,emptyTraitFilters,matchesTraits,filterLanguageKeys} from './traits.mjs';
import {renderRankingCharts} from './rankings.mjs?v=20261013-layers';

const $=selector=>document.querySelector(selector);
const svgNS='http://www.w3.org/2000/svg';
let data,model,traits,programCatalog;
const state={benchmarks:new Set(),languages:new Set(),disabled:new Set(),ellipses:new Set(['mech']),
  view:'summary',focus:'nbody',fast:true,small:true,sampleMode:'dim',medianMode:'highlight',densityMode:'off',focusMeans:true,global:false,trail:true,playing:false,collapsedFamilies:new Set(),releaseYearStart:null,releaseYear:null,playbackYear:null,traits:emptyTraitFilters(),languageFocus:null,
  detail:{language:'mech',benchmark:'nbody',kind:'fast',program:null,aggregate:false,file:0}};
const colors={};
const mechViewColors={mech:'#F6C04E','mech-jit':'#FFE59A','mech-resident':'#C99B2E'};
const domains=new Map();
let currentDomain=null;
let playbackTimer=null,historyCache={key:null,series:[]};
const tableSort={comparison:{key:null,direction:1},timings:{key:'run',direction:1},implementations:{key:null,direction:1}};
const esc=text=>String(text).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const ratio=n=>`${summaryNumber(n)}×`;
const seconds=n=>n==null?'Not measured':`${n.toFixed(n<1?6:n>=100?2:3)} s`;
const language=key=>data.languages.find(l=>l.key===key);
const benchmark=key=>data.benchmarks.find(b=>b.key===key);
const catalogProgram=key=>Object.hasOwn(programCatalog,key)?programCatalog[key]:undefined;
const benchmarkPrograms=(b,key)=>Object.values(programCatalog).filter(p=>p.benchmark===b&&p.language===key);
const measuredTiming=p=>Number.isFinite(p?.seconds)&&p.seconds>0;
const sampleProgram=program=>program;
const shortName=key=>({c:'C',cpp:'C++',mech:'Mech (AOT)',python:'Python',ruby:'Ruby',javascript:'JavaScript'}[key]??language(key).label);
const chartName=key=>({mech:'Mech AOT','mech-jit':'Mech JIT','mech-resident':'Mech interpreted',bqn:'BQN',k:'K'}[key]??shortName(key));
const partialEntry=entry=>entry.coverage.measured<entry.coverage.selected;
const coverageLabel=entry=>`${entry.coverage.measured}/${entry.coverage.selected}`;
const missingNames=entry=>entry.coverage.missing.map(b=>b.title).join(', ');
const meanChartName=entry=>chartName(entry.language.key)+(partialEntry(entry)?` · ${coverageLabel(entry)}`:'');
const isMechLanguage=key=>language(key)?.family==='mech'||key==='mech';
const mechKeys=()=>data.languages.filter(l=>isMechLanguage(l.key)).map(l=>l.key);
const displayColor=key=>state.view==='mech'?(mechViewColors[key]??colors[key]):colors[key];
const inLanguageView=()=>state.view==='language'&&!!state.languageFocus;
const samplesVisible=()=>layerPresentation(state.sampleMode).visible;
const densityVisible=()=>layerPresentation(state.densityMode).visible;
const focusKeys=()=>new Set(state.languageFocus?.keys??[]);
function savedChartSelection(){
  return {...state,benchmarks:new Set(state.benchmarks),languages:new Set(state.languages),disabled:new Set(state.disabled),ellipses:new Set(state.ellipses),collapsedFamilies:new Set(state.collapsedFamilies),traits:Object.fromEntries(Object.entries(state.traits).map(([key,values])=>[key,new Set(values)])),detail:{...state.detail},languageFocus:null,playing:false,inspectorOpen:$('#inspector').open};
}
function focusPrograms(){
  const keys=focusKeys();
  return data.benchmarks.flatMap(b=>[...keys].flatMap(key=>benchmarkPrograms(b.key,key)));
}
function comparisonData(){
  const focus=state.languageFocus,selected=focus?.comparison?catalogProgram(focus.comparisonProgram):null;
  if(!selected)return data;
  // An explicit source comparison gets a separate view. The canonical model,
  // source records, endpoint pools and fixed baselines remain unchanged.
  return {...data,programs:{...data.programs,[selected.key]:selected},benchmarks:data.benchmarks.map(b=>{
    if(b.key!==selected.benchmark)return b;
    const replacement={language:selected.language,label:shortName(selected.language),programs:[selected.key],
      fast:measuredTiming(selected)?selected.key:null,small:selected.key};
    const present=b.groups.some(g=>g.language===selected.language);
    return {...b,groups:present?b.groups.map(g=>g.language===selected.language?{...g,...replacement}:g):[...b.groups,replacement]};
  })};
}
function showLanguageOverview(key,{family=false,restoreProgram=false}={}){
  pauseYears();
  const group=family?families().find(f=>f.key===key):null;
  const keys=group?group.members.map(l=>l.key):[key];
  if(!keys.every(k=>language(k)))return;
  const sameFocus=state.languageFocus&&keys.join(',')===state.languageFocus.keys.join(',');
  const origin=state.languageFocus?.origin??savedChartSelection();
  const savedDetail=sameFocus?state.languageFocus.savedDetail:null;
  const prior=sameFocus?state.languageFocus:null;
  state.languageFocus={keys,selectionKey:key,family:!!group,label:group?.label??shortName(key),origin,
    program:restoreProgram&&savedDetail?.program?savedDetail.program:prior?.program??null,
    variant:restoreProgram&&savedDetail? savedDetail.language:prior?.variant??keys[0],
    benchmark:restoreProgram&&savedDetail?savedDetail.benchmark:prior?.benchmark??state.focus,
    comparison:false,comparisonProgram:null,savedDetail,savedInspectorOpen:prior?.savedInspectorOpen??false};
  if(!restoreProgram&&$('#inspector').open){$('#inspector').close();$('#chart-detail-layout').classList.remove('inspector-open');}
  state.view='language';state.traits=emptyTraitFilters();fullReleaseWindow();
  state.benchmarks=new Set(data.benchmarks.map(b=>b.key));state.languages=new Set(data.languages.map(l=>l.key));
  state.disabled.clear();state.ellipses=new Set(keys);
  Object.assign(state,{fast:true,small:true,focusMeans:false,global:true,trail:false});
  for(const k of keys)state.collapsedFamilies.delete(language(k).family??k);
  if(restoreProgram&&savedDetail){
    state.detail={...savedDetail};
    if(prior.savedInspectorOpen&&!$('#inspector').open){$('#inspector').show();$('#chart-detail-layout').classList.add('inspector-open');}
    if(!prior.savedInspectorOpen&&$('#inspector').open){$('#inspector').close();$('#chart-detail-layout').classList.remove('inspector-open');}
  }
  update();
  if(restoreProgram&&savedDetail&&$('#inspector').open)renderInspector();
  $('#language-overview-title').focus({preventScroll:true});
  $('#language-overview').scrollIntoView({block:'start',behavior:'auto'});
}
function leaveLanguageOverview(){
  const origin=state.languageFocus?.origin;if(!origin)return;
  const {inspectorOpen,...selection}=origin;Object.assign(state,selection);
  if(!inspectorOpen&&$('#inspector').open){$('#inspector').close();$('#chart-detail-layout').classList.remove('inspector-open');}
  if(inspectorOpen&&!$('#inspector').open){$('#inspector').show();$('#chart-detail-layout').classList.add('inspector-open');}
  update();
}
function leaveLanguageFocusForSelection(){
  if(!state.languageFocus)return;
  state.languageFocus=null;if(state.view==='language')state.view='summary';state.global=false;
}
function languageMedianStatistics(entry){
  return Object.fromEntries(['fast','small'].map(kind=>{
    const points=entry.inputs.map(input=>input[kind]);
    if(!points.length)return [kind,null];
    const midpoint={x:2**median(points.map(p=>Math.log2(p.x))),y:2**median(points.map(p=>Math.log2(p.y)))};
    const atMedian=region=>region&&({...region,center:midpoint,boundary:region.boundary.map(p=>({x:p.x*midpoint.x/region.center.x,y:p.y*midpoint.y/region.center.y}))});
    return [kind,{n:points.length,median:midpoint,spread:atMedian(entry.spread[kind]),spread2:atMedian(entry.spread2[kind])}];
  }));
}
function renderLanguageOverview(){
  const focus=state.languageFocus,panel=$('#language-overview');
  panel.hidden=!focus||!(inLanguageView()||focus.comparison&&state.view==='benchmark');
  if(panel.hidden)return;
  const programs=focusPrograms(),measured=programs.filter(measuredTiming);
  const covered=data.benchmarks.filter(b=>measured.some(p=>p.benchmark===b.key));
  $('#language-overview-title').textContent=`${focus.label} · ${focus.comparison?'comparison':'language overview'}`;
  $('#language-overview-summary').textContent=`${covered.length}/${data.benchmarks.length} benchmarks · ${measured.length} timed published programs · ${measured.reduce((n,p)=>n+p.samples.length,0)} recorded runs. ${data.program_inventory?'Every qualified program retained in this publication is listed below.':'This publication retains its fastest and fewest-LOC program pool.'} Unavailable timings stay blank.`;
  if(focus.comparison){
    const viewData=comparisonData(),active=viewData.benchmarks.find(b=>b.key===state.focus);
    const missing=[...state.languages].filter(key=>!active?.groups.some(g=>g.language===key&&g.programs.some(id=>measuredTiming(viewData.programs[id]))));
    if(missing.length)$('#language-overview-summary').textContent+=` ${missing.map(shortName).join(', ')}: no measured timing for ${benchmark(state.focus).title}.`;
  }
  $('#return-language-overview').hidden=!focus.comparison;
  $('#overview-variant').innerHTML=focus.keys.map(k=>`<option value="${k}">${esc(shortName(k))}</option>`).join('');
  $('#overview-variant').closest('label').hidden=focus.keys.length===1;
  if(!focus.keys.includes(focus.variant))focus.variant=focus.keys[0];
  if(!benchmark(focus.benchmark))focus.benchmark=data.benchmarks[0].key;
  $('#overview-variant').value=focus.variant;
  $('#overview-benchmark').innerHTML=data.benchmarks.map(b=>`<option value="${b.key}" ${b.key===focus.benchmark?'selected':''}>${esc(b.title)}</option>`).join('');
  const others=data.languages.filter(l=>!focusKeys().has(l.key));
  const previous=$('#overview-comparison-language').value;
  $('#overview-comparison-language').innerHTML=others.map(l=>`<option value="${l.key}">${esc(shortName(l.key))}</option>`).join('');
  $('#overview-comparison-language').value=others.some(l=>l.key===previous)?previous:others.some(l=>l.key==='mech')?'mech':others[0]?.key??'';
  $('#overview-compare').disabled=!others.length;
  $('#overview-inspect').disabled=!programs.some(p=>p.language===focus.variant&&p.benchmark===focus.benchmark);
  $('#language-timings tbody').innerHTML=data.benchmarks.map(b=>{
    const rows=programs.filter(p=>p.benchmark===b.key);
    if(!rows.length)return `<tr><th scope="row">${esc(b.title)}</th><td colspan="7" class="missing-timing">No published program or timing.</td></tr>`;
    return rows.map((p,i)=>`<tr class="${p.key===focus.program?'chosen':''}">${i===0?`<th scope="rowgroup" rowspan="${rows.length}">${esc(b.title)}</th>`:''}<td>${esc(shortName(p.language))}</td><td><button data-inspect-program="${p.key}">${esc(p.id)}</button></td><td class="ratio">${p.loc}</td><td class="ratio">${measuredTiming(p)?seconds(p.seconds):'—'}</td><td class="ratio">${measuredTiming(p)?`${(p.seconds/b.baseline_seconds).toFixed(3)}×`:'—'}</td><td class="ratio">${measuredTiming(p)&&Number.isFinite(p.timing_sd)&&p.timing_sd>=0?seconds(p.timing_sd):'—'}</td><td class="sample-values">${measuredTiming(p)&&p.samples.length?p.samples.map(value=>Number(value).toFixed(6)).join(' · '):'—'}<button class="row-comparison" data-compare-program="${p.key}">Compare</button></td></tr>`).join('');
  }).join('');
}
function compareOverviewProgram(key=null){
  const focus=state.languageFocus;if(!focus)return;
  const p=key?catalogProgram(key):null,from=p?.language??focus.variant,b=p?.benchmark??focus.benchmark,to=$('#overview-comparison-language').value;
  if(!language(from)||!language(to)||!benchmark(b))return;
  const current=selectedProgram();
  focus.savedInspectorOpen=$('#inspector').open;
  focus.savedDetail=p&&current?.key!==p.key?{language:p.language,benchmark:p.benchmark,kind:'fast',program:p.key,aggregate:false,file:0}:
    {...state.detail,program:current?.key??state.detail.program};
  if(p)focus.program=p.key;
  focus.variant=from;focus.benchmark=b;focus.comparisonProgram=p?.key??null;
  state.detail=p?{...focus.savedDetail}:{language:from,benchmark:b,kind:'fast',program:null,aggregate:false,file:0};
  focus.comparison=true;state.view='benchmark';state.focus=b;state.benchmarks=new Set([b]);state.languages=new Set([from,to]);
  state.traits=emptyTraitFilters();fullReleaseWindow();state.disabled.clear();
  Object.assign(state,{fast:true,small:true,focusMeans:false,global:false,trail:false});
  if(!selectedProgram()&&$('#inspector').open){$('#inspector').close();$('#chart-detail-layout').classList.remove('inspector-open');}
  update();
  const viewData=comparisonData();
  const missing=[from,to].filter(k=>!viewData.benchmarks.find(item=>item.key===b).groups.some(g=>g.language===k&&g.programs.some(id=>measuredTiming(viewData.programs[id]))));
  if(missing.length)showHoverDetails(`${missing.map(shortName).join(', ')} has no measured timing for ${benchmark(b).title}. No result is invented.`);
}
function languageViewDomain(){
  const domain={...chartDomain(data,state)},keys=focusKeys();
  const include=point=>{
    if(!(point.x>0&&point.y>0))return;
    domain.xMin=Math.min(domain.xMin,Math.log2(point.x)-.15);domain.xMax=Math.max(domain.xMax,Math.log2(point.x)+.25);
    domain.yMin=Math.min(domain.yMin,Math.log2(point.y)-.18);domain.yMax=Math.max(domain.yMax,Math.log2(point.y)+.25);
  };
  for(const p of focusPrograms())if(measuredTiming(p)){
    include(p);
    const b=benchmark(p.benchmark);
    if(!state.focusMeans)for(const elapsed of p.samples)if(Number.isFinite(elapsed)&&elapsed>0)include({x:p.x,y:elapsed/b.baseline_seconds});
  }
  const entries=comparison(data,{benchmarks:new Set(data.benchmarks.map(b=>b.key)),languages:keys,disabled:new Set()}).entries;
  for(const entry of entries){
    const statistics=languageMedianStatistics(entry);
    for(const kind of ['fast','small']){
      if(!state.focusMeans){
        for(const p of entry.spread2[kind]?.boundary??[])include(p);
        for(const p of statistics[kind].spread2?.boundary??[])include(p);
      }
      include(statistics[kind].median);
    }
  }
  return domain;
}
function sourceComparisonDomain(){
  const domain={...chartDomain(comparisonData(),state)},p=catalogProgram(state.languageFocus?.comparisonProgram);
  if(!p||!measuredTiming(p))return domain;
  const b=benchmark(p.benchmark);
  if(Number.isFinite(p.x)&&p.x>0){domain.xMin=Math.min(domain.xMin,Math.log2(p.x)-.15);domain.xMax=Math.max(domain.xMax,Math.log2(p.x)+.25);}
  if(Number.isFinite(p.y)&&p.y>0){domain.yMin=Math.min(domain.yMin,Math.log2(p.y)-.18);domain.yMax=Math.max(domain.yMax,Math.log2(p.y)+.25);}
  if(!state.focusMeans)for(const elapsed of p.samples)if(Number.isFinite(elapsed)&&elapsed>0){
    const y=Math.log2(elapsed/b.baseline_seconds);
    domain.yMin=Math.min(domain.yMin,y-.18);domain.yMax=Math.max(domain.yMax,y+.25);
  }
  return domain;
}
function enterMechView(){
  pauseYears();
  state.languageFocus=null;
  state.traits=emptyTraitFilters();
  state.view='mech';fullReleaseWindow();
  state.benchmarks=new Set(data.benchmarks.map(b=>b.key));
  for(const key of mechKeys())state.languages.add(key);
  for(const program of Object.values(data.programs))if(isMechLanguage(program.language))state.disabled.delete(program.key);
  state.collapsedFamilies.delete('mech');
  Object.assign(state,{fast:true,small:true,focusMeans:false,global:false});
}
const families=()=>languageFamilies(data.languages).map(f=>({...f,label:f.members.length>1?f.label:shortName(f.members[0].key)}))
  .sort((a,b)=>a.label.localeCompare(b.label,'en',{sensitivity:'base'}));
const familyCount=keys=>new Set([...keys].map(key=>language(key).family??key)).size;
const lineDash=key=>({dashed:'7 4',dotted:'2 4'}[language(key).line_style]??null);
const yearBounds=()=>({min:Math.min(...data.languages.map(l=>l.release_year)),max:Math.max(...data.languages.map(l=>l.release_year))});
const selectedYearWindow=()=>({from:state.releaseYearStart,to:state.releaseYear});
const shownReleaseYear=()=>Math.max(state.releaseYearStart,Math.min(state.releaseYear,state.playbackYear??state.releaseYear));
const yearKeys=()=>new Set(languagesByYear(data.languages,{from:state.releaseYearStart,to:shownReleaseYear()}));
const traitLanguageKeys=()=>filterLanguageKeys(state.languages,traits,state.traits);
const effectiveLanguages=()=>{const allowed=yearKeys();return new Set([...traitLanguageKeys()].filter(key=>allowed.has(key)));};
const playbackYears=()=>releaseYearsInWindow(data.languages.filter(l=>traitLanguageKeys().has(l.key)&&(state.view!=='mech'||isMechLanguage(l.key))),selectedYearWindow());
function pauseYears({clearCursor=false}={}){clearTimeout(playbackTimer);playbackTimer=null;state.playing=false;if(clearCursor)state.playbackYear=null;}
function fullReleaseWindow(){const {min,max}=yearBounds();state.releaseYearStart=min;state.releaseYear=max;state.playbackYear=null;}
function setReleaseWindow(from,to,{moving='both'}={}){
  const window=clampReleaseWindow({from,to},yearBounds(),moving);
  pauseYears({clearCursor:true});state.releaseYearStart=window.from;state.releaseYear=window.to;update();
}
function setReleaseYear(year){
  const {min,max}=yearBounds();
  if(!Number.isInteger(year)||year<min||year>max)throw new Error('Invalid language release year filter.');
  setReleaseWindow(state.releaseYearStart,year,{moving:'end'});
}
function playYears(){
  if(state.playing){pauseYears();renderYears();return;}
  const years=playbackYears();
  if(!years.length){pauseYears({clearCursor:true});update();return;}
  state.playing=true;state.global=true;
  if(!years.includes(state.playbackYear)||state.playbackYear===years.at(-1))state.playbackYear=years[0];
  update();
  const advance=()=>{
    const next=playbackYears().find(year=>year>state.playbackYear);
    if(next==null){pauseYears({clearCursor:true});update();return;}
    state.playbackYear=next;update();
    playbackTimer=setTimeout(advance,850);
  };
  playbackTimer=setTimeout(advance,850);
}
function currentHistory(){
  const selection={benchmarks:state.view==='benchmark'?new Set(state.benchmarks.has(state.focus)?[state.focus]:[]):state.benchmarks,
    languages:state.view==='mech'?new Set(mechKeys().filter(key=>state.languages.has(key))):traitLanguageKeys(),disabled:state.disabled};
  const key=JSON.stringify([state.languageFocus?.comparisonProgram??null,state.releaseYearStart,state.releaseYear,Object.fromEntries(Object.entries(selection).map(([k,v])=>[k,[...v].sort()]))]);
  if(historyCache.key!==key)historyCache={key,series:medianHistory(data,selection,selectedYearWindow())};
  return historyCache.series.filter(item=>item.year<=shownReleaseYear());
}
function renderYears(){
  const {min,max}=yearBounds(),eligible=effectiveLanguages(),from=state.releaseYearStart,to=state.releaseYear;
  for(const [id,year,label,limitMin,limitMax] of [['release-year-start',from,'First',min,to],['release-year-end',to,'Last',from,max]]){
    const input=$('#'+id);input.min=min;input.max=max;input.value=year;
    input.setAttribute('aria-valuemin',String(limitMin));input.setAttribute('aria-valuemax',String(limitMax));
    input.setAttribute('aria-valuetext',`${label} release year ${year}, inclusive. Selected window ${from} to ${to}.`);
  }
  $('#release-year-window').style.setProperty('--year-from',`${100*(from-min)/(max-min||1)}%`);
  $('#release-year-window').style.setProperty('--year-to',`${100*(to-min)/(max-min||1)}%`);
  $('#release-year-value').textContent=from===to?String(from):`${from}–${to}`;
  $('#release-year-min').textContent=`From ${from}`;$('#release-year-max').textContent=`To ${to}`;
  for(const button of document.querySelectorAll('[data-release-decade]')){
    const window=decadeWindow(Number(button.dataset.releaseDecade),{min,max});button.disabled=!window;
    button.setAttribute('aria-pressed',String(!!window&&window.from===from&&window.to===to));
    button.title=window?`${window.from}–${window.to}, inclusive`:'Outside the published release-year span';
  }
  const shown=shownReleaseYear(),progress=state.playbackYear==null?'':` ${state.playing?'Playback through':'Playback paused at'} ${shown}; selected window ends ${to}.`;
  $('#release-year-status').textContent=`${eligible.size} of ${traitLanguageKeys().size} matching entries in ${from}–${shown}.${progress}`;
  $('#playback-year').textContent=state.playbackYear==null?`${from}–${to}`:`${state.playing?'Through':'Paused at'} ${shown}`;
  $('#play-years').textContent=state.playing?'Pause window':'Play window';
  $('#play-years').disabled=!playbackYears().length;
  $('#play-years').setAttribute('aria-pressed',String(state.playing));
}
function colorKeyStyle(key){
  const color=displayColor(key),style=language(key).line_style;
  return `background:${style==='dashed'?`repeating-linear-gradient(to right,${color} 0 5px,transparent 5px 8px)`:style==='dotted'?`repeating-linear-gradient(to right,${color} 0 2px,transparent 2px 5px)`:color}`;
}

function sortHeader(table,key,label,subtitle=''){
  const sort=tableSort[table],active=sort.key===key;
  const chevrons=active?(sort.direction===1?'M4 10 8 6 12 10':'M4 6 8 10 12 6'):'M4 6 8 2 12 6 M4 10 8 14 12 10';
  return `<th scope="col" aria-sort="${active?(sort.direction===1?'ascending':'descending'):'none'}"><button class="sort-header" data-sort-table="${table}" data-sort-key="${key}" data-sort-label="${esc(label+(subtitle?' '+subtitle:''))}"><span class="sort-label">${esc(label)}${subtitle?`<small>${esc(subtitle)}</small>`:''}</span><svg class="sort-indicator" viewBox="0 0 16 16" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="${chevrons}"/></svg></button></th>`;
}
function sortedRows(table,rows,value){
  const sort=tableSort[table];
  if(!sort.key)return rows;
  return rows.slice().sort((a,b)=>{
    const x=value(a,sort.key),y=value(b,sort.key);
    if(x==null||y==null)return x==null?(y==null?0:1):-1;
    return sort.direction*(typeof x==='string'?x.localeCompare(y,'en',{numeric:true,sensitivity:'base'}):x-y);
  });
}
function sortTable(button){
  const {sortTable:table,sortKey:key,sortLabel:label}=button.dataset,sort=tableSort[table];
  sort.direction=sort.key===key?-sort.direction:1;sort.key=key;
  const inspector=$('#inspector'),scrollTop=inspector.scrollTop;
  const variantScroll=$('.variant-list')?.scrollTop??0;
  if(table==='comparison')renderTable();else{
    renderInspector();inspector.scrollTop=scrollTop;
    $('.variant-list').scrollTop=variantScroll;
  }
  $(`[data-sort-table="${table}"][data-sort-key="${key}"]`).focus({preventScroll:true});
  $('#sort-status').textContent=`Sorted ${label} ${sort.direction===1?'ascending':'descending'}.`;
}

function node(tag,attributes={},parent){
  const element=document.createElementNS(svgNS,tag);
  for(const [key,value] of Object.entries(attributes))if(value!==null&&value!==undefined)element.setAttribute(key,String(value));
  if(parent)parent.append(element);
  return element;
}
function text(parent,value,x,y,attributes={}){
  const element=node('text',{x,y,...attributes},parent);element.textContent=value;return element;
}
function luminance(rgb){return rgb.map(v=>v/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);}
function darkColor(hex){
  const original=[1,3,5].map(i=>parseInt(hex.slice(i,i+2),16));
  const background=luminance([21,27,33]);
  for(let tint=0;tint<=1;tint+=.025){
    const rgb=original.map(v=>Math.round(v+(255-v)*tint));
    if((luminance(rgb)+.05)/(background+.05)>=4.2)return '#'+rgb.map(v=>v.toString(16).padStart(2,'0')).join('');
  }
  return hex;
}
function allSelection(){const enabled=effectiveLanguages();return {benchmarks:state.benchmarks,languages:state.view==='mech'?new Set(mechKeys().filter(key=>enabled.has(key))):enabled,disabled:state.disabled};}
function renderMedianRankings(){
  const active=state.view==='rankings',panel=$('#median-rankings');panel.hidden=!active;$('.chart-card').hidden=active;
  if(!active)return;
  const rankings=medianRankings(currentEntries()),count=rankings.speed.n;
  $('#median-rankings-summary').textContent=`${count} variant${count===1?'':'s'} · ${state.benchmarks.size} benchmarks · ${state.releaseYearStart}–${shownReleaseYear()}`;
  $('#median-rankings-empty').hidden=count!==0;$('#median-ranking-cards').hidden=count===0;
  if(!count){$('#median-ranking-cards').replaceChildren();return;}
  renderRankingCharts({container:$('#median-ranking-cards'),rankings,labelFor:shortName,colorFor:displayColor,onOpenLanguage:showLanguageOverview});
}
function update(){
  model=comparison(comparisonData(),allSelection());
  renderYears();renderControls();renderLanguageOverview();renderMedianRankings();renderChart();renderTable();
  if($('#inspector').open)renderInspector();
  $('#selection-summary').textContent=`${state.benchmarks.size}/10 selected · ${familyCount(model.entries.map(e=>e.language.key))} languages · ${model.entries.length} variants · ${state.disabled.size} hidden`;
  const visible=currentEntries();
  const enabled=effectiveLanguages();
  const partial=visible.filter(partialEntry);
  $('#coverage-message').textContent=partial.length?`${partial.map(e=>`${shortName(e.language.key)} uses ${coverageLabel(e)} selected benchmarks; missing ${missingNames(e)}.`).join(' ')} Each mean uses that language's measured tests from your selection. Failed tests contribute no timing or LOC ratio.`:'Enabled languages cover the compared benchmarks. The means give each benchmark equal weight.';
  $('#benchmark-filter-count').textContent=state.benchmarks.size;
  $('#language-filter-count').textContent=familyCount(state.view==='mech'?mechKeys().filter(key=>enabled.has(key)):enabled);
  for(const button of document.querySelectorAll('[data-view]'))button.setAttribute('aria-pressed',button.dataset.view===state.view);
  $('#focused-benchmark').hidden=state.view!=='benchmark';
  $('#focused-benchmark').value=state.focus;
  renderPlotOptions();
  for(const [id,key] of [['fast-profile','fast'],['small-profile','small'],['show-global','global'],['show-median-trail','trail']])$('#'+id).checked=state[key];
}
function renderPlotOptions(){
  const samples=$('#show-samples'),medians=$('#show-medians'),density=$('#show-density'),axes=$('#chart-axes');
  const samplesNote='Actual recorded runs: Off removes dots, Dim shows them faintly, Highlight shows them at full contrast. Independent of medians and density.';
  const medianNote='Profile summaries, benchmark medians, links, labels and SD. Global median & SD adds summary statistics within this layer. Off removes the entire median family.';
  const densityNote='Density of actual recorded runs, each run weighted equally. Independent of dots, medians, profiles and global overlay.';
  const axesNote='Medians fits source/profile summaries; All samples fits every recorded run and covariance envelope. Changing any layer leaves this range unchanged.';
  samples.disabled=medians.disabled=density.disabled=axes.disabled=false;
  samples.value=state.sampleMode;medians.value=state.medianMode;density.value=state.densityMode;axes.value=state.focusMeans?'means':'samples';
  $('#median-option-label').textContent=inLanguageView()?'Language median & SD':'Global median & SD';
  $('#show-global').closest('label').title='Include coordinate medians and covariance in the Medians layer. Its Off, Dim or Highlight mode controls this overlay too.';
  $('#show-median-trail').disabled=inLanguageView();
  for(const [input,note] of [[samples,samplesNote],[medians,medianNote],[density,densityNote],[axes,axesNote]]){
    input.closest('label').title=note;input.title=note;input.setAttribute('aria-description',note);
  }
}
function renderControls(){
  const allowed=yearKeys(),enabled=effectiveLanguages();
  const active=document.activeElement;
  const focusAttribute=['data-language','data-family','data-open-language','data-open-family','data-trait-option','data-spread','data-benchmark'].find(attribute=>active?.hasAttribute(attribute));
  const focusValue=focusAttribute?active.getAttribute(focusAttribute):null;
  $('#benchmark-controls').innerHTML=data.benchmarks.map(b=>`<div class="control-row"><input type="checkbox" data-benchmark="${b.key}" aria-label="Include ${esc(b.title)}" ${state.benchmarks.has(b.key)?'checked':''}><button class="benchmark-name ${state.focus===b.key&&state.view==='benchmark'?'active':''}" data-inspect-benchmark="${b.key}">${esc(b.title)}</button></div>`).join('');
  const languageRow=(l,child=false)=>`<div class="control-row ${child?'variant-row':''} ${allowed.has(l.key)?'':'year-excluded'} ${matchesTraits(traits.get(l.key),state.traits)?'':'trait-excluded'}"><input type="checkbox" data-language="${l.key}" aria-label="Include ${esc(shortName(l.key))}" ${enabled.has(l.key)?'checked':''} ${!matchesTraits(traits.get(l.key),state.traits)||!allowed.has(l.key)||state.view==='mech'&&!isMechLanguage(l.key)?'disabled':''}><button class="language-name ${child?'':'family-name'} ${focusKeys().has(l.key)?'active':''}" data-open-language="${l.key}" title="Open ${esc(shortName(l.key))} timings and programs"><span class="color-key" style="${colorKeyStyle(l.key)}"></span><span>${esc(child?l.variant_label:shortName(l.key))}${child?'':`<small class="release-label">${l.release_year}</small>`}${l.timed_benchmarks.length<10?`<small class="coverage-label" title="${esc(l.coverage_note??'Incomplete measured coverage')}">${l.timed_benchmarks.length}/10 admitted</small>`:''}</span></button><button class="spread-toggle" data-spread="${l.key}" aria-label="Toggle ${esc(shortName(l.key))} standard deviation" aria-pressed="${state.ellipses.has(l.key)}" title="${esc(shortName(l.key))}: standard-deviation ellipses" ${!enabled.has(l.key)||state.view==='mech'&&!isMechLanguage(l.key)?'disabled':''}><svg viewBox="0 0 28 18" aria-hidden="true"><ellipse cx="14" cy="9" rx="11" ry="5.5" transform="rotate(-22 14 9)" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="14" cy="9" r="1.5" fill="currentColor"/></svg></button></div>`;
  $('#language-controls').innerHTML=families().map(family=>{
    if(family.members.length===1)return `<div class="language-family" data-language-family="${family.key}">${languageRow(family.members[0])}</div>`;
    const eligible=family.members.filter(l=>matchesTraits(traits.get(l.key),state.traits)&&allowed.has(l.key)&&(state.view!=='mech'||isMechLanguage(l.key))),count=eligible.filter(l=>enabled.has(l.key)).length;
    const expanded=!state.collapsedFamilies.has(family.key),first=family.members[0];
    return `<div class="language-family" data-language-family="${family.key}"><div class="control-row family-row ${eligible.length?'':'year-excluded'}"><input type="checkbox" data-family="${family.key}" aria-label="Include all ${esc(family.label)} variants" ${count&&count===eligible.length?'checked':''} ${eligible.length?'':'disabled'}><button class="language-name family-name ${family.members.some(l=>focusKeys().has(l.key))?'active':''}" data-open-family="${family.key}" title="Open all ${esc(family.label)} execution variants"><span class="color-key" style="${colorKeyStyle(first.key)}"></span><span>${esc(family.label)}<small class="release-label">${first.release_year}</small></span></button><button class="family-disclosure" data-toggle-family="${family.key}" aria-label="${expanded?'Collapse':'Expand'} ${esc(family.label)} variants" aria-expanded="${expanded}" aria-controls="variants-${family.key}"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3 10 8 5 13"/></svg></button></div><div class="family-variants" id="variants-${family.key}" ${expanded?'':'hidden'}>${family.members.map(l=>languageRow(l,true)).join('')}</div></div>`;
  }).join('');
  for(const input of document.querySelectorAll('[data-family]')){
    const family=families().find(f=>f.key===input.dataset.family);
    const eligible=family.members.filter(l=>matchesTraits(traits.get(l.key),state.traits)&&allowed.has(l.key)&&(state.view!=='mech'||isMechLanguage(l.key))),count=eligible.filter(l=>enabled.has(l.key)).length;
    input.indeterminate=count>0&&count<eligible.length;
  }
  $('#language-groups').innerHTML=traitDimensions.map(dimension=>`<fieldset class="trait-dimension"><legend>${esc(dimension.label)}</legend>${dimension.options.map(option=>{
    const count=data.languages.filter(l=>allowed.has(l.key)&&state.languages.has(l.key)&&matchesTraits(traits.get(l.key),state.traits,dimension.key)&&traits.get(l.key)[dimension.key].has(option.key)).length;
    return `<label class="trait-choice"><input type="checkbox" data-trait-dimension="${dimension.key}" data-trait-option="${dimension.key}:${option.key}" aria-label="Filter ${esc(dimension.label)}: ${esc(option.label)}" ${state.traits[dimension.key].has(option.key)?'checked':''} ${state.view==='mech'?'disabled':''}><span>${esc(option.label)}</span><small>${count}</small></label>`;
  }).join('')}${dimension.note?`<p>${esc(dimension.note)}</p>`:''}</fieldset>`).join('');
  $('#trait-filter-status').textContent=`${enabled.size} matching variants`;
  $('#clear-traits').disabled=state.view==='mech'||!traitDimensions.some(({key})=>state.traits[key].size);
  if(focusAttribute)$(`[${focusAttribute}="${focusValue}"]`)?.focus({preventScroll:true});
}
function currentEntries(){
  if(state.view!=='benchmark')return model.entries;
  if(!state.benchmarks.has(state.focus))return [];
  return comparison(comparisonData(),{benchmarks:new Set([state.focus]),languages:effectiveLanguages(),disabled:state.disabled}).entries;
}
function currentSampleInputs(entries=currentEntries()){
  const languageView=inLanguageView(),selectedKeys=focusKeys();
  return recordedSamplePoints(entries,{disabled:state.disabled,programsForInput:(input,entry)=>
    (languageView&&selectedKeys.has(entry.language.key)?benchmarkPrograms(input.benchmark.key,entry.language.key):input.programs)
      .filter(program=>program.language===entry.language.key&&program.benchmark===input.benchmark.key).map(sampleProgram)});
}
function selectedPlotDomain(){
  if(inLanguageView())return languageViewDomain();
  if(state.languageFocus?.comparisonProgram)return sourceComparisonDomain();
  return chartDomain(data,state);
}
function currentDensityField(){return densityGrid(sampleDensityPoints(currentSampleInputs()),selectedPlotDomain());}
function openPoint(info){
  const explicit=catalogProgram(info.program);
  state.detail={language:explicit?.language??info.language,benchmark:explicit?.benchmark??info.benchmark??state.focus,kind:info.kind??'fast',
    program:info.program??null,aggregate:!!info.aggregate,file:0};
  if(!benchmark(state.detail.benchmark))state.detail.benchmark=data.benchmarks[0].key;
  if(!explicit&&!benchmark(state.detail.benchmark).groups.some(g=>g.language===state.detail.language))state.detail.benchmark=data.benchmarks.find(b=>b.groups.some(g=>g.language===state.detail.language)).key;
  if(state.languageFocus&&focusKeys().has(state.detail.language)){
    state.languageFocus.program=selectedProgram()?.key??null;
    state.languageFocus.variant=state.detail.language;state.languageFocus.benchmark=state.detail.benchmark;
  }
  renderLanguageOverview();
  renderInspector();
  if(!$('#inspector').open){
    $('#chart-detail-layout').classList.add('inspector-open');
    $('#inspector').show();
    renderChart();
  }
  clearHoverDetails();
  $('#close-inspector').focus({preventScroll:true});
}
function closeInspector(){
  $('#inspector').close();
  $('#chart-detail-layout').classList.remove('inspector-open');
  renderChart();
}
function clearHoverDetails(){
  const details=$('#hover-details');
  details.textContent='Hover a point or line for details.';
  details.classList.remove('active');
}
function showHoverDetails(description){
  const details=$('#hover-details');
  if(details.textContent!==description)details.textContent=description;
  details.classList.add('active');
}
function renderChart(){
  const svg=$('#chart');svg.replaceChildren();
  const languageView=inLanguageView(),selectedKeys=focusKeys();
  svg.classList.remove('median-focus');
  delete svg.dataset.highlightLanguage;svg.onpointerleave=null;clearHoverDetails();
  const entries=currentEntries(),isMech=state.view==='mech',single=state.view==='benchmark',density=densityVisible();
  const selectedComparison=state.languageFocus?.comparison?catalogProgram(state.languageFocus.comparisonProgram):null;
  const sampleInputs=currentSampleInputs(entries);
  const layers=chartLayerState({samples:state.sampleMode,medians:state.medianMode,density:state.densityMode,fast:state.fast,small:state.small,global:state.global,sampleCount:sampleInputs.length,entryCount:entries.length});
  const rawSamples=layers.showSamples?sampleInputs:[];
  $('#chart-title').textContent=languageView?`${state.languageFocus.label} across all ten benchmarks`:single?benchmark(state.focus).title:isMech?'Mech across selected benchmarks':`${state.benchmarks.size} selected benchmark${state.benchmarks.size===1?'':'s'}`;
  if(isMech){
    $('#chart-subtitle').innerHTML=mechKeys().map(key=>`<span class="mech-mode-key" data-mode-key="${key}" style="color:${displayColor(key)}"><svg width="22" height="8" viewBox="0 0 22 8" aria-hidden="true"><line x1="0" y1="4" x2="22" y2="4" stroke="currentColor" stroke-width="2.5" ${lineDash(key)?`stroke-dasharray="${lineDash(key)}"`:''}/></svg>${esc(language(key).variant_label)}</span>`).join('')+'<span>Benchmark endpoints</span>';
  }else{
    const selectedSource=state.languageFocus?.comparison?catalogProgram(state.languageFocus.comparisonProgram):null;
    $('#chart-subtitle').textContent=languageView?'Highlighted program medians, individual runs and benchmark covariance · other languages provide context':single?`${selectedSource?`${selectedSource.id} · selected source · `:''}${benchmark(state.focus).workload}`:entries.some(partialEntry)?'Equal-weight geometric means · partial suites labelled n/selected':'Equal-weight geometric means · selected benchmark subset';
  }
  const empty=layers.empty;
  $('#empty-state').hidden=!empty;
  $('#empty-state').textContent=single&&!state.benchmarks.has(state.focus)?'Check this benchmark to include it on the chart.':!state.benchmarks.size?'Select at least one benchmark.':entries.length&&!layers.showSamples&&!layers.showMedians&&!layers.showDensity?'Choose Dim or Highlight for Samples, Medians or Density to show a layer.':state.languages.size&&!allSelection().languages.size?'No enabled languages match this release-year window. Adjust either handle or choose All years.':'No measured comparisons. Enable a language or restore its programs.';
  const width=Math.max(290,svg.clientWidth),height=innerWidth<=720?Math.max(330,width*.86):Math.min(690,Math.max(440,width*.59));
  svg.setAttribute('viewBox',`0 0 ${width} ${height}`);svg.style.aspectRatio=`${width}/${height}`;
  const margins={left:innerWidth<=720?75:80,right:25,top:30,bottom:72};
  const w=width-margins.left-margins.right,h=height-margins.top-margins.bottom;
  const visibleKinds=['fast','small'].filter(k=>state[k]);
  const global=globalStatistics(entries);
  const overlayEntries=languageView?entries.filter(e=>selectedKeys.has(e.language.key)).map(entry=>({entry,statistics:languageMedianStatistics(entry)})):[{entry:null,statistics:global}];
  const globalNote=$('#global-statistics-note');
  globalNote.classList.toggle('inactive',!layers.showGlobal||empty);
  globalNote.setAttribute('aria-hidden',!layers.showGlobal||empty);
  globalNote.innerHTML=languageView?'Language coordinate medians · 1σ / 2σ covariance across measured benchmarks · run-to-run SD is in the timings table':`Global median: <span style="color:#edf3f6;opacity:${state.fast?1:.4}">Fastest</span> / <span style="color:#f6c04e;opacity:${state.small?1:.4}">Fewest LOC</span> · 1σ / 2σ covariance · <span class="global-count">${entries.length}</span> visible variants${state.trail?' · trail by release year':''}`;
  const shapes=[];
  if(layers.showMedians&&!single)for(const sigma of [2,1])for(const entry of entries)if(state.ellipses.has(entry.language.key)&&!(languageView&&state.global&&selectedKeys.has(entry.language.key)))for(const kind of visibleKinds){
    const region=(sigma===2?entry.spread2:entry.spread)[kind];if(region)shapes.push({entry,kind,region,sigma});
  }
  const domainKey=JSON.stringify([state.view,state.focusMeans,single?state.focus:[...state.benchmarks].sort(),languageView?[...selectedKeys].sort():null,state.languageFocus?.comparisonProgram??null]);
  if(!domains.has(domainKey))domains.set(domainKey,selectedPlotDomain());
  currentDomain=domains.get(domainKey);
  const {xMin:xmin,xMax:xmax,yMin:ymin,yMax:ymax}=currentDomain;
  for(const [key,value] of Object.entries(currentDomain))svg.dataset[key]=value;
  const dx=xmax-xmin,dy=ymax-ymin;
  const X=x=>margins.left+(Math.log2(x)-xmin)/dx*w,Y=y=>margins.top+h-(Math.log2(y)-ymin)/dy*h;
  const defs=node('defs',{},svg),clip=node('clipPath',{id:'plot-clip'},defs);
  node('rect',{x:margins.left,y:margins.top,width:w,height:h},clip);
  const baselineClip=node('clipPath',{id:'baseline-clip'},defs);
  node('rect',{x:X(1),y:margins.top,width:margins.left+w-X(1),height:Y(1)-margins.top},baselineClip);
  $('#density-legend').hidden=!density;
  if(density){
    const field=densityGrid(sampleDensityPoints(sampleInputs),currentDomain);
    const heat=node('g',{class:'density-field','clip-path':'url(#plot-clip)','pointer-events':'none',opacity:layers.densityLayer.opacity,
      'data-layer':'density','data-layer-mode':state.densityMode,'data-density-variants':field.variants,'data-density-runs':field.runs,'data-density-programs':field.programs,'data-density-peak':field.peak},svg);
    const stops=[[21,27,33],[30,92,122],[48,176,160],[246,192,78]];
    const color=value=>{
      const scaled=field.peak?Math.sqrt(value/field.peak)*3:0,index=Math.min(2,Math.floor(scaled)),fraction=scaled-index;
      return `rgb(${stops[index].map((v,i)=>Math.round(v+(stops[index+1][i]-v)*fraction)).join(',')})`;
    };
    for(const cell of field.cells)node('rect',{x:margins.left+cell.column*w/field.columns,y:margins.top+h-(cell.row+1)*h/field.rows,
      width:w/field.columns+.25,height:h/field.rows+.25,fill:color(cell.value),'data-density-cell':`${cell.column}:${cell.row}`},heat);
    $('#density-population').textContent=field.runs?`${field.runs} recorded runs · ${field.programs} programs · ${field.variants} variants · equal run weights · smoothing ${summaryNumber(field.bandwidth_x)} × ${summaryNumber(field.bandwidth_y)} log₂ doublings. Relative intensity; dot visibility does not change density.`:'No recorded runs in the current data selection.';
  }
  for(let exponent=Math.ceil(xmin);exponent<=Math.floor(xmin+dx);exponent++){
    const value=2**exponent,x=X(value);node('line',{x1:x,x2:x,y1:margins.top,y2:margins.top+h,class:'grid-line'},svg);
    text(svg,`${summaryNumber(value)}×`,x,margins.top+h+23,{class:'tick','text-anchor':'middle'});
  }
  for(let exponent=Math.ceil(ymin);exponent<=Math.floor(ymin+dy);exponent++){
    const value=2**exponent,y=Y(value);node('line',{x1:margins.left,x2:margins.left+w,y1:y,y2:y,class:'grid-line'},svg);
    text(svg,`${summaryNumber(value)}×`,margins.left-12,y+5,{class:'tick','text-anchor':'end'});
  }
  text(svg,'Source lines / fewest lines · log₂',margins.left+w/2,height-17,{class:'axis-title','text-anchor':'middle'});
  text(svg,'Time / fastest median · log₂',0,0,{class:'axis-title','text-anchor':'middle',transform:`translate(${innerWidth<=720?14:19} ${margins.top+h/2}) rotate(-90)`});
  if(empty)return;
  const plot=node('g',{'clip-path':'url(#plot-clip)'},svg);
  const medianPlot=node('g',{'data-layer':'medians','data-layer-mode':state.medianMode,opacity:layers.medianLayer.opacity},plot);
  const samplePlot=node('g',{'data-layer':'samples','data-layer-mode':state.sampleMode,opacity:layers.sampleLayer.opacity},plot);
  const globalRegions=node('g',{class:'global-regions','pointer-events':'none'},medianPlot);
  if(layers.showGlobal)for(const {entry,statistics} of overlayEntries)for(const kind of visibleKinds)for(const sigma of [2,1]){
    if(entry&&!state.ellipses.has(entry.language.key))continue;
    const region=(sigma===2?statistics[kind]?.spread2:statistics[kind]?.spread);if(!region)continue;
    const color=entry?displayColor(entry.language.key):kind==='fast'?'#edf3f6':'#f6c04e';
    node('path',{d:region.boundary.map((p,i)=>`${i?'L':'M'}${X(p.x)},${Y(p.y)}`).join(' ')+'Z',
      class:'global-spread','data-global-profile':kind,'data-global-sigma':sigma,
      fill:color,'fill-opacity':sigma===2?.025:.065,
      stroke:color,'stroke-opacity':sigma===2?.4:.85,
      'stroke-width':sigma===2?1:1.5,'stroke-dasharray':sigma===2?'2 5':kind==='small'?'6 4':null,
      'clip-path':languageView?null:'url(#baseline-clip)',
      'aria-label':`${entry?shortName(entry.language.key):'Global'} ${kind==='fast'?'fastest':'fewest LOC'} ${sigma}σ covariance across ${region.n} ${languageView?'benchmarks':'visible execution variants'}`},globalRegions);
  }
  const hoverSpreads=node('g',{class:'hover-spreads'},medianPlot);
  const lineLabels=[],labelObstacles=[],hitPoints=[],hitLines=[];
  let hoveredLanguage=null,focusedLanguage=null,highlightedLanguage;
  function updateHighlight(){
    const key=hoveredLanguage??focusedLanguage;
    if(key===highlightedLanguage)return;
    highlightedLanguage=key;
    if(key)svg.dataset.highlightLanguage=key;else delete svg.dataset.highlightLanguage;
    hoverSpreads.replaceChildren();
    if(layers.showMedians&&key&&!single&&!state.ellipses.has(key)){
      const entry=entries.find(item=>item.language.key===key);
      if(entry)for(const sigma of [2,1])for(const kind of visibleKinds){
        const region=(sigma===2?entry.spread2:entry.spread)[kind];
        if(region)drawSpread({entry,kind,region,sigma},hoverSpreads,true);
      }
    }
    for(const piece of plot.querySelectorAll('[data-chart-language]')){
      const selected=key?piece.dataset.chartLanguage===key:languageView&&selectedKeys.has(piece.dataset.chartLanguage);
      piece.classList.toggle('chart-dimmed',(!!key||languageView)&&!selected);
      piece.classList.toggle('sample-highlighted',selected&&piece.classList.contains('timing-run'));
      if(piece.classList.contains('label-crowded'))piece.classList.toggle('label-revealed',selected);
    }
  }
  function highlightPointer(event,key,description){
    if(event.pointerType==='touch')return;
    hoveredLanguage=key;updateHighlight();showHoverDetails(description);
  }
  function clearHover(){hoveredLanguage=null;updateHighlight();clearHoverDetails();}
  svg.onpointerleave=clearHover;
  function pointerPosition(event){
    const transform=svg.getScreenCTM();if(!transform)return null;
    const pointer=svg.createSVGPoint();pointer.x=event.clientX;pointer.y=event.clientY;
    return pointer.matrixTransform(transform.inverse());
  }
  function closestMarker(event,fallback){
    const position=pointerPosition(event);if(!position)return fallback;
    let chosen=fallback,distance=Math.hypot(position.x-fallback.x,position.y-fallback.y);
    for(const point of hitPoints){
      if(point.faint!==fallback.faint)continue;
      const dx=position.x-point.x,dy=position.y-point.y,d=Math.hypot(dx,dy);
      const inside=point.kind==='small'?Math.abs(dx)<=point.size+.7&&Math.abs(dy)<=point.size+.7:d<=point.size+.7;
      if(inside&&d<distance-.25){chosen=point;distance=d;}
    }
    return chosen;
  }
  function closestLine(event,fallback){
    const position=pointerPosition(event);if(!position)return fallback;
    const distance=line=>{
      const dx=line.b.x-line.a.x,dy=line.b.y-line.a.y;
      const t=Math.max(0,Math.min(1,((position.x-line.a.x)*dx+(position.y-line.a.y)*dy)/(dx*dx+dy*dy)));
      return Math.hypot(position.x-line.a.x-t*dx,position.y-line.a.y-t*dy);
    };
    let chosen=fallback,nearest=distance(fallback);
    for(const line of hitLines){const d=distance(line);if(d<nearest-.25){chosen=line;nearest=d;}}
    return chosen;
  }
  function drawSpread({entry,kind,region,sigma},parent=medianPlot,hover=false){
    const d=region.boundary.map((p,i)=>`${i?'L':'M'}${X(p.x)},${Y(p.y)}`).join(' ')+'Z';
    node('path',{d,fill:displayColor(entry.language.key),'fill-opacity':sigma===2?(kind==='fast'?.035:.018):(kind==='fast'?.10:.05),
      stroke:displayColor(entry.language.key),'stroke-opacity':sigma===2?.3:.6,'stroke-width':sigma===2?.9:1.2,
      'stroke-dasharray':sigma===2?'2 5':kind==='small'?'5 4':lineDash(entry.language.key),class:'spread-area',
      'clip-path':languageView?null:'url(#baseline-clip)',
      'data-chart-language':entry.language.key,'data-spread-language':entry.language.key,'data-spread-profile':kind,'data-spread-sigma':sigma,
      'data-hover-spread':hover?'true':null,
      'aria-label':`${shortName(entry.language.key)} ${kind==='fast'?'fastest':'fewest LOC'} ${sigma}σ covariance area across ${region.n} benchmarks`},parent);
  }
  for(const shape of shapes)drawSpread(shape);
  // Wide line hover targets sit below every marker, preserving point clicks.
  const connectorHits=node('g',{class:'connector-hit-regions'},medianPlot);
  function marker(point,entry,kind,{faint=false,mean=false,program=null,benchmarkKey=null,label=null}={}){
    if(!layers.showMedians)return null;
    const color=displayColor(entry.language.key),size=faint?3.3:mean?state.ellipses.has(entry.language.key)?8.5:7.2:7;
    const attrs={fill:color,stroke:mean&&state.ellipses.has(entry.language.key)?'#f3f3e6':color,'stroke-width':mean?1:.7,
      opacity:1,class:`endpoint ${mean?'mean-mark':''}`,'data-language':entry.language.key,'data-profile':kind,
      'data-chart-language':entry.language.key,style:'--chart-opacity:1',
      'data-program':program?.key??null,'data-mean':mean?'true':null,'data-profile-median':'true',tabindex:0,role:'button'};
    const name=label??`${shortName(entry.language.key)} · ${kind==='fast'?'fastest':'fewest LOC'}`;
    const description=program?`${name}\n${benchmark(benchmarkKey).title} · ${program.loc} LOC · ${seconds(program.seconds)}\n${program.samples.length} timing samples`:`${name}\n${ratio(point.x)} LOC · ${ratio(point.y)} time\n${coverageLabel(entry)} selected benchmarks${partialEntry(entry)?` · missing ${missingNames(entry)}`:''}`;
    attrs['aria-label']=description.replaceAll('\n',', ');
    const mark=kind==='small'?node('rect',{...attrs,x:X(point.x)-size,y:Y(point.y)-size,width:size*2,height:size*2},medianPlot):node('circle',{...attrs,cx:X(point.x),cy:Y(point.y),r:size},medianPlot);
    if(!faint)labelObstacles.push({x:X(point.x)-size-2,y:Y(point.y)-size-2,width:size*2+4,height:size*2+4});
    const info={language:entry.language.key,kind,program:program?.key,benchmark:benchmarkKey,aggregate:mean};
    const hit={x:X(point.x),y:Y(point.y),size,faint,kind,info,description};hitPoints.push(hit);
    // Dense means can overlap. Pick the closest visible center within that
    // marker layer instead of whichever SVG element happens to be on top.
    mark.addEventListener('click',event=>openPoint(closestMarker(event,hit).info));
    mark.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();openPoint(info);}});
    const hover=event=>{const nearest=closestMarker(event,hit);highlightPointer(event,nearest.info.language,nearest.description);};
    mark.addEventListener('pointerenter',hover);mark.addEventListener('pointermove',hover);
    mark.addEventListener('pointerleave',clearHover);
    mark.addEventListener('focus',()=>{focusedLanguage=entry.language.key;updateHighlight();showHoverDetails(description);});
    mark.addEventListener('blur',()=>{focusedLanguage=null;updateHighlight();clearHoverDetails();});
    return mark;
  }
  function pair(entry,fast,small,{mean=false,faint=false,benchmarkKey=null,label=null}={}){
    if(!layers.showMedians)return;
    if(state.fast&&state.small&&!same(fast,small)){
      const geometry={x1:X(small.x),y1:Y(small.y),x2:X(fast.x),y2:Y(fast.y)};
      node('line',{...geometry,stroke:displayColor(entry.language.key),
        'stroke-width':mean&&isMechLanguage(entry.language.key)?2.8:1.9,'stroke-dasharray':lineDash(entry.language.key),opacity:faint?.15:1,
        class:'connector','data-chart-language':entry.language.key,style:`--chart-opacity:${faint?.15:1}`},medianPlot);
      const hit={a:{x:geometry.x1,y:geometry.y1},b:{x:geometry.x2,y:geometry.y2},language:entry.language.key,
        description:`${label??shortName(entry.language.key)}\nFastest: ${ratio(fast.x)} LOC · ${ratio(fast.y)} time\nFewest LOC: ${ratio(small.x)} LOC · ${ratio(small.y)} time`};
      hitLines.push(hit);
      const target=node('line',{...geometry,class:'connector-hit','data-chart-language':entry.language.key,'data-connector-mean':mean},connectorHits);
      const hover=event=>{const nearest=closestLine(event,hit);highlightPointer(event,nearest.language,nearest.description);};
      target.addEventListener('pointerenter',hover);target.addEventListener('pointermove',hover);target.addEventListener('pointerleave',clearHover);
    }
    if(state.small&&(!state.fast||!same(fast,small)))marker(small,entry,'small',{faint,mean,program:mean?null:small,benchmarkKey});
    if(state.fast)marker(fast,entry,'fast',{faint,mean,program:mean?null:fast,benchmarkKey,label});
    if(label&&!faint&&(state.fast||state.small)){
      const a=state.small?small:fast,b=state.fast?fast:small;
      lineLabels.push({label,language:entry.language.key,benchmark:benchmarkKey,a:{x:X(a.x),y:Y(a.y)},b:{x:X(b.x),y:Y(b.y)}});
    }
  }
  if(layers.showMedians&&!single&&!isMech)for(const entry of entries)for(const input of entry.inputs){
    if(languageView&&selectedKeys.has(entry.language.key))continue;
    if(state.small)marker(input.small,entry,'small',{faint:true,program:input.small,benchmarkKey:input.benchmark.key});
    if(state.fast)marker(input.fast,entry,'fast',{faint:true,program:input.fast,benchmarkKey:input.benchmark.key});
  }
  for(const point of rawSamples){
    const {program,entry,benchmark:inputBenchmark,run:runNumber,elapsed}=point;
    // Runs share the program's actual source coordinate; no jitter or invented values.
      const description=`${shortName(program.language)} · ${inputBenchmark.title}\n${program.id} · run ${runNumber}: ${seconds(elapsed)} · ${program.loc} LOC\nRun time / fixed benchmark median baseline: ${ratio(point.y)}`;
      const run=node('circle',{cx:X(point.x),cy:Y(point.y),r:2.4,fill:'none',stroke:displayColor(entry.language.key),'stroke-width':1.15,
        class:'timing-run','data-sample':'true','data-chart-language':program.language,'data-program':program.key,'data-timing-run':runNumber,'data-run-seconds':elapsed,'data-ratio-x':point.x,'data-ratio-y':point.y,style:'--chart-opacity:1',tabindex:0,role:'button','aria-label':description.replaceAll('\n',', ')},samplePlot);
      const inspect=()=>openPoint({language:program.language,benchmark:program.benchmark,program:program.key});
      run.addEventListener('click',inspect);run.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();inspect();}});
      run.addEventListener('pointerenter',event=>highlightPointer(event,program.language,description));run.addEventListener('pointermove',event=>highlightPointer(event,program.language,description));run.addEventListener('pointerleave',clearHover);
      run.addEventListener('focus',()=>{focusedLanguage=program.language;updateHighlight();showHoverDetails(description);});run.addEventListener('blur',()=>{focusedLanguage=null;updateHighlight();clearHoverDetails();});
  }
  if(layers.showMedians&&(languageView||selectedComparison)){
    const seenPrograms=new Set();
    for(const {program,entry} of sampleInputs){
      if(seenPrograms.has(program.key)||!(languageView?selectedKeys.has(entry.language.key):entry.language.key===selectedComparison.language))continue;
      seenPrograms.add(program.key);
      const mark=marker(program,entry,state.fast?'fast':'small',{faint:true,program,benchmarkKey:program.benchmark,label:`${shortName(program.language)} · ${program.id} median`});
      mark.classList.add('overview-program-median');mark.removeAttribute('data-sample');mark.setAttribute('tabindex','0');
    }
  }
  for(const entry of entries){
    if(single||isMech||languageView&&selectedKeys.has(entry.language.key))for(const input of entry.inputs)pair(entry,input.fast,input.small,{benchmarkKey:input.benchmark.key,label:isMech||languageView?`${input.benchmark.title} · ${entry.language.variant_label??chartName(entry.language.key)}`:chartName(entry.language.key)});
    if(!single)pair(entry,entry.fast,entry.small,{mean:true,label:isMech?`${entry.language.execution??'AOT'} geometric mean`:meanChartName(entry)});
  }
  if(layers.showGlobal&&state.trail)for(const kind of visibleKinds){
    const history=currentHistory().filter(item=>item.statistics[kind]);
    const color=kind==='fast'?'#edf3f6':'#f6c04e';
    node('path',{d:history.map((item,i)=>{const p=item.statistics[kind].median;return `${i?'L':'M'}${X(p.x)},${Y(p.y)}`;}).join(' '),
      class:'median-trail','data-global-profile':kind,fill:'none',stroke:color,'stroke-width':2,'stroke-opacity':.75,
      'stroke-dasharray':kind==='small'?'5 4':null},medianPlot);
    for(const item of history){
      const p=item.statistics[kind].median;
      const mark=node('circle',{cx:X(p.x),cy:Y(p.y),r:2.8,fill:color,class:'median-trail-point',
        'data-global-profile':kind,'data-year':item.year,'data-ratio-x':p.x,'data-ratio-y':p.y},medianPlot);
      const description=`${item.year} · global ${kind==='fast'?'fastest':'fewest LOC'} median\n${ratio(p.x)} LOC · ${ratio(p.y)} time\n${item.statistics[kind].n} visible execution variants`;
      mark.addEventListener('pointerenter',event=>{clearHover();showHoverDetails(description);});
      mark.addEventListener('pointermove',event=>showHoverDetails(description));mark.addEventListener('pointerleave',clearHover);
    }
  }
  if(layers.showGlobal)for(const {entry,statistics} of overlayEntries)for(const kind of visibleKinds){
    if(!statistics[kind])continue;
    const statistic='median',point=statistics[kind].median,x=X(point.x),y=Y(point.y),size=8;
    const name=`${entry?shortName(entry.language.key):'Global'} ${kind==='fast'?'fastest':'fewest LOC'} median`;
    const description=`${name}\n${ratio(point.x)} LOC · ${ratio(point.y)} time\n${statistics[kind].n} ${languageView?'benchmarks':'visible execution variants'}, equal weight`;
    const color=entry?displayColor(entry.language.key):kind==='fast'?'#edf3f6':'#f6c04e';
    const mark=node('path',{d:`M${x},${y-size}l${size},${size}l${-size},${size}l${-size},${-size}Z`,
      class:'global-center','data-global-profile':kind,'data-global-statistic':statistic,
      'data-ratio-x':point.x,'data-ratio-y':point.y,'data-median-language':entry?.language.key??null,fill:color,
      stroke:'#151b21','stroke-width':2,tabindex:0,role:'img','aria-label':description.replaceAll('\n',', ')},medianPlot);
    text(medianPlot,`${entry?`${entry.language.variant_label??chartName(entry.language.key)} · `:''}${kind==='fast'?'Fastest':'Fewest LOC'} median`,x+12,y-12,{class:'median-label',fill:color});
    labelObstacles.push({x:x-size-3,y:y-size-3,width:2*size+6,height:2*size+6});
    mark.addEventListener('pointerenter',event=>{clearHover();showHoverDetails(description);});
    mark.addEventListener('pointermove',event=>showHoverDetails(description));mark.addEventListener('pointerleave',clearHover);
    mark.addEventListener('focus',()=>showHoverDetails(description));
    mark.addEventListener('blur',clearHover);
  }
  // Label only the visible profiles, not their faint benchmark samples. Prefer
  // a nearby offset along the line and avoid covering another label or marker.
  const labels=node('g',{class:'line-labels'},medianPlot),placed=[];
  const overlap=(a,b)=>Math.max(0,Math.min(a.x+a.width,b.x+b.width)-Math.max(a.x,b.x))*Math.max(0,Math.min(a.y+a.height,b.y+b.height)-Math.max(a.y,b.y));
  for(const item of lineLabels){
    // A means-focused range can crop benchmark pairs. Keep their labels with
    // their lines rather than pinning labels for offscreen pairs to the border.
    if(Math.max(item.a.x,item.b.x)<margins.left||Math.min(item.a.x,item.b.x)>margins.left+w||
      Math.max(item.a.y,item.b.y)<margins.top||Math.min(item.a.y,item.b.y)>margins.top+h)continue;
    const label=text(labels,item.label,0,0,{class:'line-label clickable-chart-label',fill:displayColor(item.language),'data-label-language':item.language,'data-chart-language':item.language,tabindex:0,role:'button','aria-label':item.benchmark?`Inspect ${shortName(item.language)} ${benchmark(item.benchmark).title}`:`Open ${shortName(item.language)} language overview`});
    const activate=()=>item.benchmark?openPoint({language:item.language,benchmark:item.benchmark}):showLanguageOverview(item.language);
    label.addEventListener('click',activate);label.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();activate();}});
    label.addEventListener('pointerenter',event=>highlightPointer(event,item.language,item.benchmark?`${shortName(item.language)} · ${benchmark(item.benchmark).title}\nClick to inspect its programs.`:`${shortName(item.language)}\nClick for all timings, programs and comparisons.`));label.addEventListener('pointerleave',clearHover);
    label.addEventListener('focus',()=>{focusedLanguage=item.language;updateHighlight();showHoverDetails(item.benchmark?`${shortName(item.language)} · ${benchmark(item.benchmark).title}\nPress Enter to inspect its programs.`:`${shortName(item.language)}\nPress Enter for all timings, programs and comparisons.`);});label.addEventListener('blur',()=>{focusedLanguage=null;updateHighlight();clearHoverDetails();});
    const box=label.getBBox(),lw=box.width,lh=box.height;
    const vx=item.b.x-item.a.x,vy=item.b.y-item.a.y,length=Math.hypot(vx,vy);
    const nx=length>1?-vy/length:0,ny=length>1?vx/length:-1;
    let best=null;
    for(const t of [.5,.7,.3,.9,.1])for(const distance of [12,-12,23,-23,36,-36,51,-51]){
      const anchor={x:item.a.x+vx*t,y:item.a.y+vy*t};
      const center={x:anchor.x+nx*distance,y:anchor.y+ny*distance};
      for(const side of [0,1,-1]){
        const x=Math.max(margins.left+2,Math.min(margins.left+w-lw-2,center.x-lw/2+side*(lw/2+8)));
        const y=Math.max(margins.top+2,Math.min(margins.top+h-lh-2,center.y-lh/2));
        const candidate={x,y,width:lw,height:lh};
        const padded={x:x-3,y:y-2,width:lw+6,height:lh+4};
        const collisions=placed.reduce((sum,other)=>sum+overlap(padded,other),0);
        const marks=labelObstacles.reduce((sum,other)=>sum+overlap(padded,other),0);
        const offset=Math.hypot(x+lw/2-anchor.x,y+lh/2-anchor.y);
        const score=collisions*1000+marks*20+offset+Math.abs(t-.5)*5;
        if(!best||score<best.score)best={...candidate,score,padded,anchor,collisions};
      }
    }
    label.setAttribute('x',best.x-box.x);label.setAttribute('y',best.y-box.y);
    const crowded=w<500&&best.collisions>0;
    label.classList.toggle('label-crowded',crowded);
    if(!crowded)placed.push(best.padded);
    const nearest={x:Math.max(best.x,Math.min(best.x+lw,best.anchor.x)),y:Math.max(best.y,Math.min(best.y+lh,best.anchor.y))};
    if(Math.hypot(nearest.x-best.anchor.x,nearest.y-best.anchor.y)>19){
      const guide=node('line',{x1:best.anchor.x,y1:best.anchor.y,x2:nearest.x,y2:nearest.y,stroke:displayColor(item.language),class:'label-guide','data-chart-language':item.language},labels);
      guide.classList.toggle('label-crowded',crowded);
      labels.insertBefore(guide,label);
    }
  }
  if(single)text(svg,`${benchmark(state.focus).baseline_loc} LOC baseline · ${seconds(benchmark(state.focus).baseline_seconds)} time baseline`,margins.left,16,{class:'chart-symbol-legend'});
  else if(shapes.length){
    const counts=shapes.map(s=>s.region.n),min=Math.min(...counts),max=Math.max(...counts);
    text(svg,`${new Set(shapes.map(s=>s.entry.language.key)).size} language spread${new Set(shapes.map(s=>s.entry.language.key)).size===1?'':'s'} · n = ${min===max?min:`${min}–${max}`}`,margins.left,16,{class:'chart-symbol-legend'});
  }
  updateHighlight();
}
function renderTable(){
  const entries=sortedRows('comparison',currentEntries().map(entry=>({...entry,metrics:comparisonMetrics(entry)})),(entry,key)=>key==='language'?shortName(entry.language.key):entry.metrics[key]);
  $('#mean-table thead').innerHTML='<tr>'+[
    sortHeader('comparison','language','Language'),
    sortHeader('comparison','loc','LOC ×'),sortHeader('comparison','fastest','Fastest ×'),
    sortHeader('comparison','slope','Slope'),sortHeader('comparison','length','Length'),
    sortHeader('comparison','areaFast','2σ area','Fastest'),
    sortHeader('comparison','areaSmall','2σ area','Fewest LOC')].join('')+'</tr>';
  const slope=value=>value==null?'—':Number.isFinite(value)?summaryNumber(value):value<0?'−∞':'∞';
  const area=value=>value==null?'—':summaryNumber(value);
  $('#mean-table tbody').innerHTML=entries.map(entry=>`<tr><td><button data-table-language="${entry.language.key}" class="table-language"><span class="color-key" style="${colorKeyStyle(entry.language.key)}"></span>${esc(shortName(entry.language.key))}${partialEntry(entry)?` <small class="coverage-badge" title="${esc(`${coverageLabel(entry)} selected benchmarks; missing ${missingNames(entry)}`)}">${coverageLabel(entry)}</small>`:''}</button></td><td class="ratio">${ratio(entry.metrics.loc)}</td><td class="ratio">${ratio(entry.metrics.fastest)}</td><td class="ratio" title="${entry.metrics.slope==null?'Same endpoint: no line slope':'Change in log₂ time divided by change in log₂ LOC'}">${slope(entry.metrics.slope)}</td><td class="ratio" title="Euclidean distance between endpoints in log₂ coordinates">${summaryNumber(entry.metrics.length)}</td><td class="ratio" data-area-profile="fast" title="Fastest-profile 2σ covariance ellipse area in squared doublings">${area(entry.metrics.areaFast)}</td><td class="ratio" data-area-profile="small" title="Fewest-LOC-profile 2σ covariance ellipse area in squared doublings">${area(entry.metrics.areaSmall)}</td></tr>`).join('');
}
function selectedProgram(){
  if(state.detail.program)return catalogProgram(state.detail.program);
  const viewData=comparisonData();
  const group=viewData.benchmarks.find(b=>b.key===state.detail.benchmark)?.groups.find(g=>g.language===state.detail.language);
  if(!group)return null;
  let candidates=group.programs.filter(k=>!state.disabled.has(k)).map(k=>viewData.programs[k]);
  if(!candidates.length)return viewData.programs[group[state.detail.kind]??group.small];
  if(state.detail.kind==='fast'&&candidates.some(p=>p.seconds!=null))candidates=candidates.filter(p=>p.seconds!=null);
  return candidates.reduce((a,b)=>state.detail.kind==='fast'?(b.seconds<a.seconds||b.seconds===a.seconds&&b.loc<a.loc?b:a):(b.loc<a.loc||b.loc===a.loc&&b.seconds<a.seconds?b:a));
}
function renderInspector(){
  const program=selectedProgram();
  if(!program)return;
  const b=benchmark(program.benchmark),lang=language(program.language),group=b.groups.find(g=>g.language===program.language);
  const entry=model.entries.find(e=>e.language.key===program.language);
  const pending=program.seconds==null;
  if(inLanguageView()&&focusKeys().has(program.language)){
    state.languageFocus.program=program.key;state.languageFocus.variant=program.language;state.languageFocus.benchmark=program.benchmark;
  }
  if(pending)state.detail.kind='small';
  const available=data.benchmarks.filter(item=>item.groups.some(g=>g.language===program.language)||benchmarkPrograms(item.key,program.language).length);
  $('#inspector-context').textContent=state.detail.aggregate?'Geometric mean contributors':b.title;
  $('#inspector-title').textContent=lang.label;
  const aggregate=state.detail.aggregate&&entry?`<p class="profile-note">${state.detail.kind==='fast'?'Fastest':'Fewest-LOC'} profile: <strong>${ratio(entry[state.detail.kind].x)} LOC · ${ratio(entry[state.detail.kind].y)} time</strong>, geometric means across ${coverageLabel(entry)} selected benchmarks.${partialEntry(entry)?` Missing: ${esc(missingNames(entry))}.`:''} Choose a benchmark below to inspect a contributor.</p>`:'';
  const file=data.files[program.files[Math.min(state.detail.file,program.files.length-1)]];
  if(tableSort.timings.key==='wall_seconds'&&program.execution_mode!=='resident')tableSort.timings.key='elapsed';
  const samples=sortedRows('timings',program.samples.map((elapsed,i)=>({...program.sample_details?.[i],run:i+1,elapsed})),(sample,key)=>sample[key]);
  const availablePrograms=benchmarkPrograms(b.key,program.language);
  const implementations=sortedRows('implementations',availablePrograms,(item,key)=>({show:state.disabled.has(item.key)?1:0,program:item.id,loc:item.loc,median:item.seconds})[key]);
  const canonicalProgram=group?.programs.includes(program.key)??false;
  const browsing=state.languageFocus&&focusKeys().has(program.language)?focusPrograms():availablePrograms;
  const browsingIndex=browsing.findIndex(p=>p.key===program.key);
  const navigation=`<div class="program-navigation"><button class="quiet-button" data-step-program="-1" ${browsingIndex<=0?'disabled':''}>Previous program</button><span>${browsingIndex+1} / ${browsing.length} published programs</span><button class="quiet-button" data-step-program="1" ${browsingIndex>=browsing.length-1?'disabled':''}>Next program</button></div>${state.languageFocus&&focusKeys().has(program.language)?`<button class="inspector-comparison quiet-button" data-compare-program="${program.key}">Compare this benchmark</button>`:`<button class="inspector-comparison quiet-button" data-open-language="${program.language}">Open language overview</button>`}`;
  const bestComparison=pending?'':`<p class="profile-note best-comparison">This program: <strong>${program.seconds.toFixed(6)} s</strong> · <strong>${(program.seconds/b.baseline_seconds).toFixed(3)}× best time</strong>, against this benchmark’s fixed best-time baseline (<strong>${b.baseline_seconds.toFixed(6)} s</strong>).</p>`;
  $('#inspector-body').innerHTML=`${aggregate}${navigation}<div class="inspector-controls"><label>Benchmark<select id="detail-benchmark">${available.map(item=>`<option value="${item.key}" ${item.key===b.key?'selected':''}>${esc(item.title)}</option>`).join('')}</select></label><label>Profile<select id="detail-profile"><option value="fast" ${pending?'disabled':''} ${state.detail.kind==='fast'?'selected':''}>Fastest</option><option value="small" ${state.detail.kind==='small'?'selected':''}>Fewest LOC</option></select></label><label class="program-choice">Program<select id="detail-program">${availablePrograms.map(p=>`<option value="${p.key}" ${program.key===p.key?'selected':''}>${esc(p.id)} · ${p.loc} LOC · ${seconds(p.seconds)}</option>`).join('')}</select></label></div>
    ${canonicalProgram?`<label class="inclusion"><input id="include-program" type="checkbox" ${pending||inLanguageView()?'disabled':''} ${pending||state.disabled.has(program.key)?'':'checked'}>Include this program in the speed comparison${inLanguageView()?' · all published programs shown in this view':''}</label>`:'<p class="profile-note">This qualified program is retained in the language overview. The main comparison uses the published fastest and fewest-LOC endpoints.</p>'}<p class="profile-note">${esc(b.workload)}</p>
    <div class="detail-stats"><div class="detail-stat"><strong>${program.loc}</strong><span>source LOC</span></div><div class="detail-stat"><strong>${seconds(program.seconds)}</strong><span>median measured time</span></div><div class="detail-stat"><strong>${pending?'Not measured':seconds(program.timing_sd)}</strong><span>run-to-run SD</span></div></div>
    ${bestComparison}
    ${program.timing_summary?`<p class="profile-note timing-boundary">${esc(program.timing_summary)}</p>`:''}
    <section class="detail-section"><h3>Recorded timing samples</h3>${pending?'<p class="profile-note">Timing pending. This source is included for its audited LOC only and contributes no speed ratio, geometric mean, or standard-deviation ellipse.</p>':`<div class="timings-table-wrap"><table class="timings-table"><thead><tr>${sortHeader('timings','run','Run')}${sortHeader('timings','elapsed','Measured seconds')}${program.execution_mode==='resident'?sortHeader('timings','wall_seconds','Whole process seconds'):''}<th scope="col">${program.samples.length} samples</th></tr></thead><tbody>${samples.map(sample=>`<tr><td>${sample.run}</td><td class="sample-value">${sample.elapsed.toFixed(6)}</td>${program.execution_mode==='resident'?`<td class="sample-value">${sample.wall_seconds.toFixed(6)}</td>`:''}<td><div class="sample-bar" style="width:${sample.elapsed/Math.max(...program.samples)*100}%"></div></td></tr>`).join('')}</tbody></table></div>`}</section>
    ${program.physical_loc!=null&&program.physical_loc!==program.loc?`<details class="loc-audit"><summary>LOC audit: ${program.physical_loc} physical lines → ${program.loc} normalized LOC</summary><p class="profile-note">Packed sequential statements and nested assignments count separately. The original source is preserved below.</p>${program.loc_expansions.map(e=>`<p class="profile-note">Source line ${e.line}: ${e.extra_lines} extra LOC</p><pre>${esc(Array.isArray(e.units)?e.units.join('\n'):e.reason??e.original??'')}</pre>`).join('')}</details>`:''}
    <section class="detail-section"><div class="code-toolbar"><select id="detail-file" aria-label="Source file">${program.files.map((key,i)=>`<option value="${i}" ${i===Math.min(state.detail.file,program.files.length-1)?'selected':''}>${esc(data.files[key].name)}${data.files[key].role==='runtime-support'?' · runtime support':''}</option>`).join('')}</select><a id="download-source" href="#">Download source</a>${program.source_url?`<a href="${esc(program.source_url)}" target="_blank" rel="noopener">${esc(program.source_link_label??'Upstream listing')}</a>`:''}${program.license_url?`<a href="${esc(program.license_url)}" target="_blank" rel="noopener">License</a>`:''}</div><pre><code id="source-code"></code></pre><div class="source-meta">${file.loc_note?esc(file.loc_note)+'<br>':''}Verified source SHA-256: ${file.sha256}<br>Timing report: ${esc(program.report)}</div></section>
    ${program.local_adaptation?`<p class="profile-note source-provenance">${esc(program.source_provenance)}</p>`:''}${program.algorithm?.reason?`<p class="profile-note">${esc(program.algorithm.reason)}</p>`:''}${program.runtime_policy?`<p class="profile-note">Runtime policy: ${esc(program.runtime_policy)}</p>`:''}${program.runtime_support?`<p class="profile-note">${esc(program.runtime_support)}</p>`:''}
    <section class="detail-section"><h3>Benchmark programs</h3><div class="variant-list"><table><thead><tr>${sortHeader('implementations','show','Show')}${sortHeader('implementations','program','Program')}${sortHeader('implementations','loc','LOC')}${sortHeader('implementations','median','Median')}</tr></thead><tbody>${implementations.map(item=>`<tr class="${item.key===program.key?'chosen':''}"><td><label><input data-toggle-program="${item.key}" type="checkbox" aria-label="Include ${esc(item.id)}" ${item.seconds==null||inLanguageView()||!group?.programs.includes(item.key)?'disabled':''} ${item.seconds==null||state.disabled.has(item.key)?'':'checked'}></label></td><td><button data-inspect-program="${item.key}">${esc(item.id)}</button></td><td>${item.loc}</td><td>${seconds(item.seconds)}</td></tr>`).join('')}</tbody></table></div></section>`;
  $('#source-code').textContent=file.code;
  renderLanguageOverview();
  $('#download-source').onclick=event=>{event.preventDefault();const url=URL.createObjectURL(new Blob([file.code],{type:'text/plain'}));const link=document.createElement('a');link.href=url;link.download=file.name;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);};
}
function reset(){
  pauseYears();
  state.benchmarks=new Set(data.benchmarks.map(b=>b.key));state.languages=new Set(data.languages.map(l=>l.key));
  state.disabled.clear();state.ellipses=new Set(['mech']);state.view='summary';state.focus='nbody';state.languageFocus=null;
  state.collapsedFamilies.clear();
  state.traits=emptyTraitFilters();
  Object.assign(state,{fast:true,small:true,sampleMode:'dim',medianMode:'highlight',densityMode:'off',focusMeans:true,global:false,trail:true});fullReleaseWindow();update();
}
function registerTools(){
  if(!document.modelContext?.registerTool)return;
  const controller=new AbortController();
  const tool={name:'configure_benchmark_chart',title:'Configure benchmark chart',
    description:'Select the benchmarks, languages, and standard-deviation ellipses shown in the explorer.',
    inputSchema:{type:'object',properties:{benchmarks:{type:'array',items:{type:'string'}},languages:{type:'array',items:{type:'string'}},ellipses:{type:'array',items:{type:'string'}}},additionalProperties:false},
    annotations:{readOnlyHint:false,untrustedContentHint:false},
    execute(input){
      if(!input||typeof input!=='object'||Array.isArray(input))throw new Error('Expected a selection object.');
      const allowed={benchmarks:data.benchmarks.map(b=>b.key),languages:data.languages.map(l=>l.key),ellipses:data.languages.map(l=>l.key)};
      for(const key of Object.keys(input))if(!allowed[key])throw new Error('Unknown selection field.');
      for(const [key,values] of Object.entries(input))if(!Array.isArray(values)||values.some(v=>typeof v!=='string'||!allowed[key].includes(v)))throw new Error(`Invalid ${key} selection.`);
      for(const [key,values] of Object.entries(input))state[key]=new Set(values);
      if(input.languages)state.traits=emptyTraitFilters();
      state.languageFocus=null;state.view='summary';update();return {benchmarks:[...state.benchmarks],languages:model.entries.map(e=>e.language.key),ellipses:[...state.ellipses]};
    }};
  try{Promise.resolve(document.modelContext.registerTool(tool,{signal:controller.signal})).catch(()=>{});}catch{}
  addEventListener('pagehide',()=>controller.abort(),{once:true});
}
async function start(){
  data=await fetch(new URL('./data.json',import.meta.url),{cache:'no-cache'}).then(response=>{if(!response.ok)throw new Error('Could not load measured benchmark data.');return response.json();});
  // The qualified inventory keeps source navigation complete without changing
  // the canonical comparison pool or its immutable normalization baselines.
  const inventory=data.program_inventory??{};
  if(typeof inventory!=='object'||Array.isArray(inventory))throw new Error('Invalid qualified program inventory.');
  for(const [key,p] of Object.entries(inventory))if(Object.hasOwn(data.programs,key)){
    const canonical=data.programs[key];
    for(const field of ['id','benchmark','language','loc','seconds','samples','files','report','binary_sha256','runtime_arguments','command'])
      if(JSON.stringify(p[field])!==JSON.stringify(canonical[field]))throw new Error(`Conflicting qualified program identity: ${key}.`);
  }
  programCatalog=Object.fromEntries(Object.entries({...inventory,...data.programs}).map(([key,p])=>{
    const b=benchmark(p.benchmark);
    if(p.key!=null&&p.key!==key||!b||!language(p.language)||!Array.isArray(p.files)||!p.files.length||p.files.some(file=>!Object.hasOwn(data.files,file)))throw new Error(`Invalid published program/source identity: ${key}.`);
    if(!Array.isArray(p.samples)||p.seconds!=null&&(!measuredTiming(p)||p.samples.length<3||p.samples.some(sample=>!Number.isFinite(sample)||sample<=0)))throw new Error(`Invalid published timing samples: ${key}.`);
    const x=p.x??p.loc/b.baseline_loc,y=p.seconds==null?null:p.y??p.seconds/b.baseline_seconds;
    if(!Number.isSafeInteger(p.loc)||p.loc<=0||!Number.isFinite(x)||x<=0||p.seconds!=null&&(!Number.isFinite(y)||y<=0))throw new Error(`Invalid published source/timing coordinates: ${key}.`);
    return [key,{...p,key,x,y}];
  }));
  traits=languageTraits(data);
  for(const l of data.languages)colors[l.key]=darkColor(l.color);
  const yearSources=[...new Map(data.languages.map(l=>[l.release_name,l])).values()].sort((a,b)=>a.release_year-b.release_year||a.release_name.localeCompare(b.release_name));
  $('#release-year-sources').innerHTML=yearSources.map(l=>`<li><a href="${esc(l.release_source)}" target="_blank" rel="noopener">${esc(l.release_name)} · ${l.release_year}</a><span>${esc(l.release_note)}</span></li>`).join('');
  $('#release-decades').innerHTML=Array.from({length:8},(_,index)=>1950+index*10).map(decade=>`<button data-release-decade="${decade}" aria-pressed="false">${decade}s</button>`).join('');
  $('#focused-benchmark').innerHTML=data.benchmarks.map(b=>`<option value="${b.key}">${esc(b.title)}</option>`).join('');
  reset();registerTools();
  for(const kind of ['benchmark','language']){
    const toggle=$(`#toggle-${kind}-filters`),body=$(`#${kind}-filter-body`);
    const setExpanded=value=>{toggle.setAttribute('aria-expanded',value);body.hidden=!value;};
    setExpanded(innerWidth>720);
    toggle.onclick=()=>setExpanded(body.hidden);
  }
  $('#all-benchmarks').onclick=()=>{leaveLanguageFocusForSelection();state.benchmarks=new Set(data.benchmarks.map(b=>b.key));update();};
  $('#no-benchmarks').onclick=()=>{leaveLanguageFocusForSelection();state.benchmarks.clear();update();};
  $('#all-languages').onclick=()=>{leaveLanguageFocusForSelection();state.traits=emptyTraitFilters();state.languages=new Set(data.languages.map(l=>l.key));if(state.view==='mech')state.view='summary';update();};
  $('#no-languages').onclick=()=>{leaveLanguageFocusForSelection();state.traits=emptyTraitFilters();state.languages.clear();update();};
  $('#only-mech').onclick=()=>{leaveLanguageFocusForSelection();state.traits=emptyTraitFilters();state.languages=new Set(mechKeys());update();};
  $('#clear-traits').onclick=()=>{leaveLanguageFocusForSelection();state.traits=emptyTraitFilters();update();};
  for(const [id,moving] of [['release-year-start','start'],['release-year-end','end']])$('#'+id).oninput=()=>{
    const from=Number($('#release-year-start').value),to=Number($('#release-year-end').value);
    leaveLanguageFocusForSelection();setReleaseWindow(from,to,{moving});
  };
  $('#all-release-years').onclick=()=>{leaveLanguageFocusForSelection();const {min,max}=yearBounds();setReleaseWindow(min,max);};
  $('#play-years').onclick=()=>{leaveLanguageFocusForSelection();playYears();};
  addEventListener('pagehide',pauseYears,{once:true});
  $('#reset').onclick=reset;
  $('#reset-rankings').onclick=()=>{reset();state.view='rankings';update();};
  $('#close-inspector').onclick=closeInspector;
  $('#leave-language-overview').onclick=leaveLanguageOverview;
  $('#return-language-overview').onclick=()=>{const focus=state.languageFocus;showLanguageOverview(focus.selectionKey,{family:focus.family,restoreProgram:true});};
  $('#overview-compare').onclick=()=>compareOverviewProgram();
  $('#overview-inspect').onclick=()=>{
    const focus=state.languageFocus,rows=benchmarkPrograms(focus.benchmark,focus.variant);
    const p=rows.find(p=>p.key===focus.program)??rows[0];if(p)openPoint({language:p.language,benchmark:p.benchmark,program:p.key});
  };
  $('#focused-benchmark').onchange=event=>{state.focus=event.target.value;update();};
  for(const [id,key] of [['fast-profile','fast'],['small-profile','small'],['show-global','global'],['show-median-trail','trail']])$('#'+id).onchange=event=>{if(event.target.disabled)return;state[key]=event.target.checked;update();};
  $('#chart-axes').onchange=event=>{if(!['means','samples'].includes(event.target.value))return;state.focusMeans=event.target.value==='means';update();};
  for(const [id,key] of [['show-samples','sampleMode'],['show-medians','medianMode'],['show-density','densityMode']])$('#'+id).onchange=event=>{layerPresentation(event.target.value);state[key]=event.target.value;update();};
  document.addEventListener('click',event=>{
    const decade=event.target.closest('[data-release-decade]');if(decade&&!decade.disabled){
      const window=decadeWindow(Number(decade.dataset.releaseDecade),yearBounds());
      if(window){leaveLanguageFocusForSelection();setReleaseWindow(window.from,window.to);}return;
    }
    const openLanguage=event.target.closest('[data-open-language]');if(openLanguage){showLanguageOverview(openLanguage.dataset.openLanguage);return;}
    const openFamily=event.target.closest('[data-open-family]');if(openFamily){showLanguageOverview(openFamily.dataset.openFamily,{family:true});return;}
    const compareProgram=event.target.closest('[data-compare-program]');if(compareProgram){compareOverviewProgram(compareProgram.dataset.compareProgram);return;}
    const stepProgram=event.target.closest('[data-step-program]');if(stepProgram&&!stepProgram.disabled){
      const current=selectedProgram(),browsing=state.languageFocus&&focusKeys().has(current.language)?focusPrograms():benchmarkPrograms(current.benchmark,current.language);
      const p=browsing[browsing.findIndex(p=>p.key===current.key)+Number(stepProgram.dataset.stepProgram)];
      if(p)openPoint({language:p.language,benchmark:p.benchmark,program:p.key});return;
    }
    const disclosure=event.target.closest('[data-toggle-family]');if(disclosure){
      const key=disclosure.dataset.toggleFamily;state.collapsedFamilies.has(key)?state.collapsedFamilies.delete(key):state.collapsedFamilies.add(key);
      renderControls();$(`[data-toggle-family="${key}"]`).focus({preventScroll:true});return;
    }
    const sort=event.target.closest('[data-sort-table]');if(sort){sortTable(sort);return;}
    const view=event.target.closest('[data-view]');if(view){leaveLanguageFocusForSelection();if(view.dataset.view==='mech')enterMechView();else state.view=view.dataset.view;update();}
    const inspect=event.target.closest('[data-inspect-benchmark]');if(inspect){leaveLanguageFocusForSelection();state.focus=inspect.dataset.inspectBenchmark;state.view='benchmark';update();openPoint({language:'mech',benchmark:state.focus});}
    const spread=event.target.closest('[data-spread]');if(spread){const key=spread.dataset.spread;state.ellipses.has(key)?state.ellipses.delete(key):state.ellipses.add(key);update();}
    const table=event.target.closest('[data-table-language]');if(table){showLanguageOverview(table.dataset.tableLanguage);return;}
    const program=event.target.closest('[data-inspect-program]');if(program){const p=catalogProgram(program.dataset.inspectProgram);openPoint({language:p.language,benchmark:p.benchmark,program:p.key});}
  });
  document.addEventListener('change',event=>{
    const element=event.target;
    if(element.disabled)return;
    if(element.id==='overview-variant'||element.id==='overview-benchmark'){
      const focus=state.languageFocus;if(!focus)return;
      focus[element.id==='overview-variant'?'variant':'benchmark']=element.value;focus.program=null;renderLanguageOverview();return;
    }
    if(element.dataset.family){
      leaveLanguageFocusForSelection();
      const family=families().find(f=>f.key===element.dataset.family),allowed=yearKeys();
      for(const l of family.members)if(matchesTraits(traits.get(l.key),state.traits)&&allowed.has(l.key)&&(state.view!=='mech'||isMechLanguage(l.key)))element.checked?state.languages.add(l.key):state.languages.delete(l.key);
      update();return;
    }
    if(element.dataset.traitOption){
      leaveLanguageFocusForSelection();
      const [dimension,option]=element.dataset.traitOption.split(':');
      element.checked?state.traits[dimension].add(option):state.traits[dimension].delete(option);
      update();return;
    }
    if(element.dataset.benchmark){leaveLanguageFocusForSelection();element.checked?state.benchmarks.add(element.dataset.benchmark):state.benchmarks.delete(element.dataset.benchmark);update();}
    if(element.dataset.language){leaveLanguageFocusForSelection();element.checked?state.languages.add(element.dataset.language):state.languages.delete(element.dataset.language);update();}
    const key=element.dataset.toggleProgram??(element.id==='include-program'?selectedProgram().key:null);
    if(key){state.detail.program=selectedProgram().key;element.checked?state.disabled.delete(key):state.disabled.add(key);update();}
    if(element.id==='detail-benchmark'){state.detail.benchmark=element.value;state.detail.program=null;state.detail.file=0;renderInspector();}
    if(element.id==='detail-profile'){state.detail.kind=element.value;state.detail.program=null;state.detail.file=0;renderInspector();}
    if(element.id==='detail-program'){state.detail.program=element.value;state.detail.file=0;renderInspector();}
    if(element.id==='detail-file'){state.detail.file=Number(element.value);renderInspector();}
  });
  document.addEventListener('keydown',event=>{if(event.key==='Escape'&&$('#inspector').open)closeInspector();});
  new ResizeObserver(()=>requestAnimationFrame(renderChart)).observe($('#chart-wrap'));
  let rankingWidth=-1;
  new ResizeObserver(entries=>{const width=entries[0].contentRect.width;if(width!==rankingWidth){rankingWidth=width;requestAnimationFrame(renderMedianRankings);}}).observe($('#median-ranking-cards'));
  window.benchmarkExplorer={getSelection:()=>({benchmarks:[...state.benchmarks],languages:[...effectiveLanguages()],selectedLanguages:[...state.languages],traits:Object.fromEntries(traitDimensions.map(({key})=>[key,[...state.traits[key]]])),releaseYearStart:state.releaseYearStart,releaseYear:state.releaseYear,shownReleaseYear:shownReleaseYear(),playbackYear:state.playbackYear,ellipses:[...state.ellipses],disabled:[...state.disabled],view:state.view,languageFocus:state.languageFocus?{keys:[...state.languageFocus.keys],program:state.languageFocus.program,comparison:state.languageFocus.comparison}:null,samples:samplesVisible(),sampleMode:state.sampleMode,medianMode:state.medianMode,densityMode:state.densityMode,focusMeans:state.focusMeans,global:state.global,trail:state.trail,playing:state.playing}),
    getMedianHistory:currentHistory,
    getDensity:currentDensityField,
    setLayerModes:({samples=state.sampleMode,medians=state.medianMode,density=state.densityMode}={})=>{for(const mode of [samples,medians,density])layerPresentation(mode);Object.assign(state,{sampleMode:samples,medianMode:medians,densityMode:density});update();},
    getMedianRankings:()=>medianRankings(currentEntries()),
    getGlobalStatistics:()=>inLanguageView()?currentEntries().filter(e=>focusKeys().has(e.language.key)).map(entry=>({language:entry.language.key,statistics:languageMedianStatistics(entry)})):globalStatistics(currentEntries()),
    getDomains:()=>({...currentDomain}),
    getSummary:()=>model.entries.map(e=>({language:e.language.key,fast:e.fast,small:e.small,n:e.inputs.length,selected:e.coverage.selected,missing:e.coverage.missing.map(b=>b.key),metrics:comparisonMetrics(e)})),setReleaseYear,setReleaseWindow,inspectProgram:key=>{const program=catalogProgram(key);if(!program)throw new Error('Unknown program.');openPoint({language:program.language,benchmark:program.benchmark,program:key});}};
}
start().catch(error=>{$('#load-error').hidden=false;$('#load-error').textContent=error.message;$('#selection-summary').textContent='Data unavailable';console.error(error);});
