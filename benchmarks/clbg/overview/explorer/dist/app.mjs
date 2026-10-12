import {comparison,same,chartDomain,comparisonMetrics,languagesByYear,globalStatistics,languageFamilies,medianHistory} from './math.mjs';
import {traitDimensions,languageTraits,emptyTraitFilters,matchesTraits,filterLanguageKeys} from './traits.mjs';

const $=selector=>document.querySelector(selector);
const svgNS='http://www.w3.org/2000/svg';
let data,model,traits;
const state={benchmarks:new Set(),languages:new Set(),disabled:new Set(),ellipses:new Set(['mech']),
  view:'summary',focus:'nbody',fast:true,small:true,samples:true,focusMeans:true,global:false,trail:true,playing:false,collapsedFamilies:new Set(),releaseYear:null,traits:emptyTraitFilters(),
  detail:{language:'mech',benchmark:'nbody',kind:'fast',program:null,aggregate:false,file:0}};
const colors={};
const mechViewColors={mech:'#F6C04E','mech-jit':'#FFE59A','mech-resident':'#C99B2E'};
const domains=new Map();
let currentDomain=null;
let playbackTimer=null,historyCache={key:null,series:[]};
const tableSort={comparison:{key:null,direction:1},timings:{key:'run',direction:1},implementations:{key:null,direction:1}};
const esc=text=>String(text).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const ratio=n=>`${n.toFixed(2)}×`;
const seconds=n=>n==null?'Not measured':`${n.toFixed(n<1?6:n>=100?2:3)} s`;
const language=key=>data.languages.find(l=>l.key===key);
const benchmark=key=>data.benchmarks.find(b=>b.key===key);
const shortName=key=>({c:'C',cpp:'C++',mech:'Mech (AOT)',python:'Python',ruby:'Ruby',javascript:'JavaScript'}[key]??language(key).label);
const chartName=key=>({mech:'Mech AOT','mech-jit':'Mech JIT','mech-resident':'Mech interpreted',bqn:'BQN',k:'K'}[key]??shortName(key));
const partialEntry=entry=>entry.coverage.measured<entry.coverage.selected;
const coverageLabel=entry=>`${entry.coverage.measured}/${entry.coverage.selected}`;
const missingNames=entry=>entry.coverage.missing.map(b=>b.title).join(', ');
const meanChartName=entry=>chartName(entry.language.key)+(partialEntry(entry)?` · ${coverageLabel(entry)}`:'');
const isMechLanguage=key=>language(key)?.family==='mech'||key==='mech';
const mechKeys=()=>data.languages.filter(l=>isMechLanguage(l.key)).map(l=>l.key);
const displayColor=key=>state.view==='mech'?(mechViewColors[key]??colors[key]):colors[key];
function enterMechView(){
  pauseYears();
  state.traits=emptyTraitFilters();
  state.view='mech';state.releaseYear=yearBounds().max;
  state.benchmarks=new Set(data.benchmarks.map(b=>b.key));
  for(const key of mechKeys())state.languages.add(key);
  for(const program of Object.values(data.programs))if(isMechLanguage(program.language))state.disabled.delete(program.key);
  state.collapsedFamilies.delete('mech');
  Object.assign(state,{samples:true,fast:true,small:true,focusMeans:false,global:false});
}
const families=()=>languageFamilies(data.languages).map(f=>({...f,label:f.members.length>1?f.label:shortName(f.members[0].key)}))
  .sort((a,b)=>a.label.localeCompare(b.label,'en',{sensitivity:'base'}));
const familyCount=keys=>new Set([...keys].map(key=>language(key).family??key)).size;
const lineDash=key=>({dashed:'7 4',dotted:'2 4'}[language(key).line_style]??null);
const yearBounds=()=>({min:Math.min(...data.languages.map(l=>l.release_year)),max:Math.max(...data.languages.map(l=>l.release_year))});
const releaseYears=()=>[...new Set(data.languages.map(l=>l.release_year))].sort((a,b)=>a-b);
const yearKeys=()=>new Set(languagesByYear(data.languages,{year:state.releaseYear}));
const traitLanguageKeys=()=>filterLanguageKeys(state.languages,traits,state.traits);
const effectiveLanguages=()=>{const allowed=yearKeys();return new Set([...traitLanguageKeys()].filter(key=>allowed.has(key)));};
function pauseYears(){clearTimeout(playbackTimer);playbackTimer=null;state.playing=false;}
function setReleaseYear(year,{playback=false}={}){
  const {min,max}=yearBounds();
  if(!Number.isInteger(year)||year<min||year>max)throw new Error('Invalid language release year filter.');
  if(!playback)pauseYears();
  state.releaseYear=releaseYears().filter(y=>y<=year).at(-1);update();
}
function playYears(){
  if(state.playing){pauseYears();renderYears();return;}
  state.playing=true;state.global=true;
  const years=releaseYears();
  if(state.releaseYear===years.at(-1))state.releaseYear=years[0];
  update();
  const advance=()=>{
    const next=years[years.indexOf(state.releaseYear)+1];
    if(next==null){pauseYears();renderYears();return;}
    setReleaseYear(next,{playback:true});
    playbackTimer=setTimeout(advance,850);
  };
  playbackTimer=setTimeout(advance,850);
}
function currentHistory(){
  const selection={benchmarks:state.view==='benchmark'?new Set(state.benchmarks.has(state.focus)?[state.focus]:[]):state.benchmarks,
    languages:state.view==='mech'?new Set(mechKeys().filter(key=>state.languages.has(key))):traitLanguageKeys(),disabled:state.disabled};
  const key=JSON.stringify(Object.fromEntries(Object.entries(selection).map(([k,v])=>[k,[...v].sort()])));
  if(historyCache.key!==key)historyCache={key,series:medianHistory(data,selection)};
  return historyCache.series.filter(item=>item.year<=state.releaseYear);
}
function renderYears(){
  const years=releaseYears(),eligible=effectiveLanguages();
  $('#release-year').min=0;$('#release-year').max=years.length-1;$('#release-year').value=years.indexOf(state.releaseYear);
  $('#release-year').setAttribute('aria-valuetext',String(state.releaseYear));
  $('#release-year-value').textContent=state.releaseYear;
  $('#release-year-min').textContent=years[0];$('#release-year-max').textContent=years.at(-1);
  $('#release-year-status').textContent=`${eligible.size} of ${traitLanguageKeys().size} matching entries released by ${state.releaseYear}.`;
  $('#playback-year').textContent=state.releaseYear;
  $('#play-years').textContent=state.playing?'Pause years':'Play years';
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
function update(){
  if(state.view!=='benchmark'&&!state.samples)state.focusMeans=true;
  model=comparison(data,allSelection());
  renderYears();renderControls();renderChart();renderTable();
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
  for(const [id,key] of [['fast-profile','fast'],['small-profile','small'],['show-samples','samples'],['focus-means','focusMeans'],['show-global','global'],['show-median-trail','trail']])$('#'+id).checked=state[key];
}
function renderPlotOptions(){
  const single=state.view==='benchmark',samples=$('#show-samples'),focus=$('#focus-means'),option=$('#focus-means-option');
  const samplesNote=single?'Benchmark view always shows the per-language endpoints. Samples applies in Geometric means and Mech views.':'Show the individual benchmark inputs behind the geometric means.';
  const focusNote=single?'Benchmark view fits axes to its endpoints and global covariance contours. Focus on means applies in Geometric means and Mech views.':'Fit axes to geometric-mean endpoints. Uncheck to fit benchmark samples and covariance contours while Samples is enabled.';
  samples.disabled=single;
  focus.disabled=single||!state.samples;
  samples.closest('label').title=samplesNote;
  option.title=focusNote;
  for(const [input,note] of [[samples,samplesNote],[focus,focusNote]]){
    input.title=note;
    input.setAttribute('aria-description',note);
  }
  option.classList.toggle('unavailable',!single&&!state.samples);
  option.setAttribute('aria-hidden',!single&&!state.samples);
}
function renderControls(){
  const allowed=yearKeys(),enabled=effectiveLanguages();
  const active=document.activeElement;
  const focusAttribute=['data-language','data-family','data-trait-option','data-spread','data-benchmark'].find(attribute=>active?.hasAttribute(attribute));
  const focusValue=focusAttribute?active.getAttribute(focusAttribute):null;
  $('#benchmark-controls').innerHTML=data.benchmarks.map(b=>`<div class="control-row"><input type="checkbox" data-benchmark="${b.key}" aria-label="Include ${esc(b.title)}" ${state.benchmarks.has(b.key)?'checked':''}><button class="benchmark-name ${state.focus===b.key&&state.view==='benchmark'?'active':''}" data-inspect-benchmark="${b.key}">${esc(b.title)}</button></div>`).join('');
  const languageRow=(l,child=false)=>`<div class="control-row ${child?'variant-row':''} ${allowed.has(l.key)?'':'year-excluded'} ${matchesTraits(traits.get(l.key),state.traits)?'':'trait-excluded'}"><label class="lang-check ${child?'':'family-name'}" title="${esc(l.release_note)}"><input type="checkbox" data-language="${l.key}" aria-label="Include ${esc(shortName(l.key))}" ${enabled.has(l.key)?'checked':''} ${!matchesTraits(traits.get(l.key),state.traits)||!allowed.has(l.key)||state.view==='mech'&&!isMechLanguage(l.key)?'disabled':''}><span class="color-key" style="${colorKeyStyle(l.key)}"></span><span>${esc(child?l.variant_label:shortName(l.key))}${child?'':`<small class="release-label">${l.release_year}</small>`}${l.timed_benchmarks.length<10?`<small class="coverage-label" title="${esc(l.coverage_note??'Incomplete measured coverage')}">${l.timed_benchmarks.length}/10 admitted</small>`:''}</span></label><button class="spread-toggle" data-spread="${l.key}" aria-label="Toggle ${esc(shortName(l.key))} standard deviation" aria-pressed="${state.ellipses.has(l.key)}" title="${esc(shortName(l.key))}: standard-deviation ellipses" ${!enabled.has(l.key)||state.view==='mech'&&!isMechLanguage(l.key)?'disabled':''}><svg viewBox="0 0 28 18" aria-hidden="true"><ellipse cx="14" cy="9" rx="11" ry="5.5" transform="rotate(-22 14 9)" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="14" cy="9" r="1.5" fill="currentColor"/></svg></button></div>`;
  $('#language-controls').innerHTML=families().map(family=>{
    if(family.members.length===1)return `<div class="language-family" data-language-family="${family.key}">${languageRow(family.members[0])}</div>`;
    const eligible=family.members.filter(l=>matchesTraits(traits.get(l.key),state.traits)&&allowed.has(l.key)&&(state.view!=='mech'||isMechLanguage(l.key))),count=eligible.filter(l=>enabled.has(l.key)).length;
    const expanded=!state.collapsedFamilies.has(family.key),first=family.members[0];
    return `<div class="language-family" data-language-family="${family.key}"><div class="control-row family-row ${eligible.length?'':'year-excluded'}"><label class="lang-check family-name" title="Toggle all ${esc(family.label)} variants"><input type="checkbox" data-family="${family.key}" aria-label="Include all ${esc(family.label)} variants" ${count&&count===eligible.length?'checked':''} ${eligible.length?'':'disabled'}><span class="color-key" style="${colorKeyStyle(first.key)}"></span><span>${esc(family.label)}<small class="release-label">${first.release_year}</small></span></label><button class="family-disclosure" data-toggle-family="${family.key}" aria-label="${expanded?'Collapse':'Expand'} ${esc(family.label)} variants" aria-expanded="${expanded}" aria-controls="variants-${family.key}"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3 10 8 5 13"/></svg></button></div><div class="family-variants" id="variants-${family.key}" ${expanded?'':'hidden'}>${family.members.map(l=>languageRow(l,true)).join('')}</div></div>`;
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
  return comparison(data,{benchmarks:new Set([state.focus]),languages:effectiveLanguages(),disabled:state.disabled}).entries;
}
function openPoint(info){
  state.detail={language:info.language,benchmark:info.benchmark??state.focus,kind:info.kind??'fast',
    program:info.program??null,aggregate:!!info.aggregate,file:0};
  if(!benchmark(state.detail.benchmark))state.detail.benchmark=data.benchmarks[0].key;
  if(!benchmark(state.detail.benchmark).groups.some(g=>g.language===info.language))state.detail.benchmark=data.benchmarks.find(b=>b.groups.some(g=>g.language===info.language)).key;
  renderInspector();
  if(!$('#inspector').open){
    $('#chart-detail-layout').classList.add('inspector-open');
    $('#inspector').show();
    renderChart();
  }
  $('#tooltip').hidden=true;
  $('#close-inspector').focus({preventScroll:true});
}
function closeInspector(){
  $('#inspector').close();
  $('#chart-detail-layout').classList.remove('inspector-open');
  renderChart();
}
function tooltip(event,description){
  const tip=$('#tooltip');tip.textContent=description;tip.hidden=false;
  const box=tip.getBoundingClientRect();
  tip.style.left=`${Math.max(8,Math.min(innerWidth-box.width-8,event.clientX+14))}px`;
  tip.style.top=`${Math.max(8,Math.min(innerHeight-box.height-8,event.clientY+14))}px`;
}
function renderChart(){
  const svg=$('#chart');svg.replaceChildren();
  svg.classList.toggle('median-focus',state.global);
  delete svg.dataset.highlightLanguage;svg.onpointerleave=null;$('#tooltip').hidden=true;
  const entries=currentEntries(),isMech=state.view==='mech',single=state.view==='benchmark';
  $('#chart-title').textContent=single?benchmark(state.focus).title:isMech?'Mech across selected benchmarks':`${state.benchmarks.size} selected benchmark${state.benchmarks.size===1?'':'s'}`;
  if(isMech){
    $('#chart-subtitle').innerHTML=mechKeys().map(key=>`<span class="mech-mode-key" data-mode-key="${key}" style="color:${displayColor(key)}"><svg width="22" height="8" viewBox="0 0 22 8" aria-hidden="true"><line x1="0" y1="4" x2="22" y2="4" stroke="currentColor" stroke-width="2.5" ${lineDash(key)?`stroke-dasharray="${lineDash(key)}"`:''}/></svg>${esc(language(key).variant_label)}</span>`).join('')+'<span>Benchmark endpoints</span>';
  }else $('#chart-subtitle').textContent=single?benchmark(state.focus).workload:entries.some(partialEntry)?'Equal-weight geometric means · partial suites labelled n/selected':'Equal-weight geometric means · selected benchmark subset';
  const empty=!entries.length||!state.fast&&!state.small;
  $('#empty-state').hidden=!empty;
  $('#empty-state').textContent=single&&!state.benchmarks.has(state.focus)?'Check this benchmark to include it on the chart.':!state.benchmarks.size?'Select at least one benchmark.':!state.fast&&!state.small?'Enable Fastest or Fewest LOC to show a profile.':state.languages.size&&!allSelection().languages.size?'No enabled languages match this release year. Adjust the year or choose All years.':'No measured comparisons. Enable a language or restore its programs.';
  const width=Math.max(290,svg.clientWidth),height=innerWidth<=720?Math.max(330,width*.86):Math.min(690,Math.max(440,width*.59));
  svg.setAttribute('viewBox',`0 0 ${width} ${height}`);svg.style.aspectRatio=`${width}/${height}`;
  const margins={left:innerWidth<=720?75:80,right:25,top:30,bottom:72};
  const w=width-margins.left-margins.right,h=height-margins.top-margins.bottom;
  const visibleKinds=['fast','small'].filter(k=>state[k]);
  const global=globalStatistics(entries);
  const globalNote=$('#global-statistics-note');
  globalNote.classList.toggle('inactive',!state.global||empty);
  globalNote.setAttribute('aria-hidden',!state.global||empty);
  globalNote.innerHTML=`Global median: <span style="color:#edf3f6;opacity:${state.fast?1:.4}">Fastest</span> / <span style="color:#f6c04e;opacity:${state.small?1:.4}">Fewest LOC</span> · 1σ / 2σ covariance · <span class="global-count">${entries.length}</span> visible variants${state.trail?' · trail by release year':''}`;
  const shapes=[];
  if(!single)for(const sigma of [2,1])for(const entry of entries)if(state.ellipses.has(entry.language.key))for(const kind of visibleKinds){
    const region=(sigma===2?entry.spread2:entry.spread)[kind];if(region)shapes.push({entry,kind,region,sigma});
  }
  const domainKey=JSON.stringify([state.view,state.focusMeans,single?state.focus:[...state.benchmarks].sort()]);
  if(!domains.has(domainKey))domains.set(domainKey,chartDomain(data,state));
  currentDomain=domains.get(domainKey);
  const {xMin:xmin,xMax:xmax,yMin:ymin,yMax:ymax}=currentDomain;
  for(const [key,value] of Object.entries(currentDomain))svg.dataset[key]=value;
  const dx=xmax-xmin,dy=ymax-ymin;
  const X=x=>margins.left+(Math.log2(x)-xmin)/dx*w,Y=y=>margins.top+h-(Math.log2(y)-ymin)/dy*h;
  const defs=node('defs',{},svg),clip=node('clipPath',{id:'plot-clip'},defs);
  node('rect',{x:margins.left,y:margins.top,width:w,height:h},clip);
  const baselineClip=node('clipPath',{id:'baseline-clip'},defs);
  node('rect',{x:X(1),y:margins.top,width:margins.left+w-X(1),height:Y(1)-margins.top},baselineClip);
  for(let exponent=Math.ceil(xmin);exponent<=Math.floor(xmin+dx);exponent++){
    const value=2**exponent,x=X(value);node('line',{x1:x,x2:x,y1:margins.top,y2:margins.top+h,class:'grid-line'},svg);
    text(svg,`${value>=1?value:value.toFixed(2)}×`,x,margins.top+h+23,{class:'tick','text-anchor':'middle'});
  }
  for(let exponent=Math.ceil(ymin);exponent<=Math.floor(ymin+dy);exponent++){
    const value=2**exponent,y=Y(value);node('line',{x1:margins.left,x2:margins.left+w,y1:y,y2:y,class:'grid-line'},svg);
    text(svg,`${value>=1?value:value.toFixed(2)}×`,margins.left-12,y+5,{class:'tick','text-anchor':'end'});
  }
  text(svg,'Source lines / fewest lines · log₂',margins.left+w/2,height-17,{class:'axis-title','text-anchor':'middle'});
  text(svg,'Time / fastest median · log₂',0,0,{class:'axis-title','text-anchor':'middle',transform:`translate(${innerWidth<=720?14:19} ${margins.top+h/2}) rotate(-90)`});
  if(empty)return;
  const plot=node('g',{'clip-path':'url(#plot-clip)'},svg);
  const globalRegions=node('g',{class:'global-regions','pointer-events':'none'},plot);
  if(state.global)for(const kind of visibleKinds)for(const sigma of [2,1]){
    const region=(sigma===2?global[kind].spread2:global[kind].spread);if(!region)continue;
    node('path',{d:region.boundary.map((p,i)=>`${i?'L':'M'}${X(p.x)},${Y(p.y)}`).join(' ')+'Z',
      class:'global-spread','data-global-profile':kind,'data-global-sigma':sigma,
      fill:kind==='fast'?'#edf3f6':'#f6c04e','fill-opacity':sigma===2?.025:.065,
      stroke:kind==='fast'?'#edf3f6':'#f6c04e','stroke-opacity':sigma===2?.4:.85,
      'stroke-width':sigma===2?1:1.5,'stroke-dasharray':sigma===2?'2 5':kind==='small'?'6 4':null,
      'clip-path':'url(#baseline-clip)',
      'aria-label':`Global ${kind==='fast'?'fastest':'fewest LOC'} ${sigma}σ covariance across ${region.n} visible execution variants`},globalRegions);
  }
  const hoverSpreads=node('g',{class:'hover-spreads'},plot);
  const lineLabels=[],labelObstacles=[],hitPoints=[],hitLines=[];
  let hoveredLanguage=null,focusedLanguage=null,highlightedLanguage=null;
  function updateHighlight(){
    const key=hoveredLanguage??focusedLanguage;
    if(key===highlightedLanguage)return;
    highlightedLanguage=key;
    if(key)svg.dataset.highlightLanguage=key;else delete svg.dataset.highlightLanguage;
    hoverSpreads.replaceChildren();
    if(key&&!single&&!state.ellipses.has(key)){
      const entry=entries.find(item=>item.language.key===key);
      if(entry)for(const sigma of [2,1])for(const kind of visibleKinds){
        const region=(sigma===2?entry.spread2:entry.spread)[kind];
        if(region)drawSpread({entry,kind,region,sigma},hoverSpreads,true);
      }
    }
    for(const piece of plot.querySelectorAll('[data-chart-language]')){
      piece.classList.toggle('chart-dimmed',!!key&&piece.dataset.chartLanguage!==key);
      piece.classList.toggle('sample-highlighted',!!key&&piece.dataset.chartLanguage===key&&piece.dataset.sample==='true');
      if(piece.classList.contains('label-crowded'))piece.classList.toggle('label-revealed',!!key&&piece.dataset.chartLanguage===key);
    }
  }
  function highlightPointer(event,key,description){
    if(event.pointerType==='touch')return;
    hoveredLanguage=key;updateHighlight();tooltip(event,description);
  }
  function clearHover(){hoveredLanguage=null;updateHighlight();$('#tooltip').hidden=true;}
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
  function drawSpread({entry,kind,region,sigma},parent=plot,hover=false){
    const d=region.boundary.map((p,i)=>`${i?'L':'M'}${X(p.x)},${Y(p.y)}`).join(' ')+'Z';
    node('path',{d,fill:displayColor(entry.language.key),'fill-opacity':sigma===2?(kind==='fast'?.035:.018):(kind==='fast'?.10:.05),
      stroke:displayColor(entry.language.key),'stroke-opacity':sigma===2?.3:.6,'stroke-width':sigma===2?.9:1.2,
      'stroke-dasharray':sigma===2?'2 5':kind==='small'?'5 4':lineDash(entry.language.key),class:'spread-area',
      'clip-path':'url(#baseline-clip)',
      'data-chart-language':entry.language.key,'data-spread-language':entry.language.key,'data-spread-profile':kind,'data-spread-sigma':sigma,
      'data-hover-spread':hover?'true':null,
      'aria-label':`${shortName(entry.language.key)} ${kind==='fast'?'fastest':'fewest LOC'} ${sigma}σ covariance area across ${region.n} benchmarks`},parent);
  }
  for(const shape of shapes)drawSpread(shape);
  // Wide line hover targets sit below every marker, preserving point clicks.
  const connectorHits=node('g',{class:'connector-hit-regions'},plot);
  function marker(point,entry,kind,{faint=false,mean=false,program=null,benchmarkKey=null,label=null}={}){
    const color=displayColor(entry.language.key),size=faint?3.3:mean?state.ellipses.has(entry.language.key)?8.5:7.2:7;
    const attrs={fill:color,stroke:mean&&state.ellipses.has(entry.language.key)?'#f3f3e6':color,'stroke-width':mean?1:.7,
      opacity:faint?.23:1,class:`endpoint ${mean?'mean-mark':''}`,'data-language':entry.language.key,'data-profile':kind,
      'data-chart-language':entry.language.key,style:`--chart-opacity:${faint?.23:1}`,
      'data-program':program?.key??null,'data-mean':mean?'true':null,'data-sample':faint?'true':null,tabindex:faint?-1:0,role:'button'};
    const name=label??`${shortName(entry.language.key)} · ${kind==='fast'?'fastest':'fewest LOC'}`;
    const description=program?`${name}\n${benchmark(benchmarkKey).title} · ${program.loc} LOC · ${seconds(program.seconds)}\n${program.samples.length} timing samples`:`${name}\n${ratio(point.x)} LOC · ${ratio(point.y)} time\n${coverageLabel(entry)} selected benchmarks${partialEntry(entry)?` · missing ${missingNames(entry)}`:''}`;
    attrs['aria-label']=description.replaceAll('\n',', ');
    const mark=kind==='small'?node('rect',{...attrs,x:X(point.x)-size,y:Y(point.y)-size,width:size*2,height:size*2},plot):node('circle',{...attrs,cx:X(point.x),cy:Y(point.y),r:size},plot);
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
    mark.addEventListener('focus',()=>{focusedLanguage=entry.language.key;updateHighlight();});
    mark.addEventListener('blur',()=>{focusedLanguage=null;updateHighlight();});
    return mark;
  }
  function pair(entry,fast,small,{mean=false,faint=false,benchmarkKey=null,label=null}={}){
    if(state.fast&&state.small&&!same(fast,small)){
      const geometry={x1:X(small.x),y1:Y(small.y),x2:X(fast.x),y2:Y(fast.y)};
      node('line',{...geometry,stroke:displayColor(entry.language.key),
        'stroke-width':mean&&isMechLanguage(entry.language.key)?2.8:1.9,'stroke-dasharray':lineDash(entry.language.key),opacity:faint?.15:1,
        class:'connector','data-chart-language':entry.language.key,style:`--chart-opacity:${faint?.15:1}`},plot);
      const hit={a:{x:geometry.x1,y:geometry.y1},b:{x:geometry.x2,y:geometry.y2},language:entry.language.key,
        description:`${label??shortName(entry.language.key)}\nFastest: ${ratio(fast.x)} LOC · ${ratio(fast.y)} time\nFewest LOC: ${ratio(small.x)} LOC · ${ratio(small.y)} time`};
      hitLines.push(hit);
      const target=node('line',{...geometry,class:'connector-hit','data-chart-language':entry.language.key,'data-connector-mean':mean},connectorHits);
      const hover=event=>{const nearest=closestLine(event,hit);highlightPointer(event,nearest.language,nearest.description);};
      target.addEventListener('pointerenter',hover);target.addEventListener('pointermove',hover);target.addEventListener('pointerleave',clearHover);
    }
    if(state.small&&(!state.fast||!same(fast,small)))marker(small,entry,'small',{faint,mean,program:mean?null:small,benchmarkKey});
    if(state.fast)marker(fast,entry,'fast',{faint,mean,program:mean?null:fast,benchmarkKey,label});
    if(label&&!faint){
      const a=state.small?small:fast,b=state.fast?fast:small;
      lineLabels.push({label,language:entry.language.key,a:{x:X(a.x),y:Y(a.y)},b:{x:X(b.x),y:Y(b.y)}});
    }
  }
  if(!single&&!isMech&&state.samples)for(const entry of entries)for(const input of entry.inputs){
    if(state.small)marker(input.small,entry,'small',{faint:true,program:input.small,benchmarkKey:input.benchmark.key});
    if(state.fast)marker(input.fast,entry,'fast',{faint:true,program:input.fast,benchmarkKey:input.benchmark.key});
  }
  for(const entry of entries){
    if(single||isMech&&state.samples)for(const input of entry.inputs)pair(entry,input.fast,input.small,{benchmarkKey:input.benchmark.key,label:isMech?`${input.benchmark.title} · ${entry.language.execution??'AOT'}`:chartName(entry.language.key)});
    if(!single)pair(entry,entry.fast,entry.small,{mean:true,label:isMech?`${entry.language.execution??'AOT'} geometric mean`:meanChartName(entry)});
  }
  if(state.global&&state.trail)for(const kind of visibleKinds){
    const history=currentHistory().filter(item=>item.statistics[kind]);
    const color=kind==='fast'?'#edf3f6':'#f6c04e';
    node('path',{d:history.map((item,i)=>{const p=item.statistics[kind].median;return `${i?'L':'M'}${X(p.x)},${Y(p.y)}`;}).join(' '),
      class:'median-trail','data-global-profile':kind,fill:'none',stroke:color,'stroke-width':2,'stroke-opacity':.75,
      'stroke-dasharray':kind==='small'?'5 4':null},plot);
    for(const item of history){
      const p=item.statistics[kind].median;
      const mark=node('circle',{cx:X(p.x),cy:Y(p.y),r:2.8,fill:color,class:'median-trail-point',
        'data-global-profile':kind,'data-year':item.year,'data-ratio-x':p.x,'data-ratio-y':p.y},plot);
      const description=`${item.year} · global ${kind==='fast'?'fastest':'fewest LOC'} median\n${ratio(p.x)} LOC · ${ratio(p.y)} time\n${item.statistics[kind].n} visible execution variants`;
      mark.addEventListener('pointerenter',event=>{clearHover();tooltip(event,description);});
      mark.addEventListener('pointermove',event=>tooltip(event,description));mark.addEventListener('pointerleave',clearHover);
    }
  }
  if(state.global)for(const kind of visibleKinds){
    const statistic='median',point=global[kind].median,x=X(point.x),y=Y(point.y),size=8;
    const name=`Global ${kind==='fast'?'fastest':'fewest LOC'} median`;
    const description=`${name}\n${ratio(point.x)} LOC · ${ratio(point.y)} time\n${global[kind].n} visible execution variants, equal weight`;
    const mark=node('path',{d:`M${x},${y-size}l${size},${size}l${-size},${size}l${-size},${-size}Z`,
      class:'global-center','data-global-profile':kind,'data-global-statistic':statistic,
      'data-ratio-x':point.x,'data-ratio-y':point.y,fill:kind==='fast'?'#edf3f6':'#f6c04e',
      stroke:'#151b21','stroke-width':2,tabindex:0,role:'img','aria-label':description.replaceAll('\n',', ')},plot);
    text(plot,`${kind==='fast'?'Fastest':'Fewest LOC'} median`,x+12,y-12,{class:'median-label',fill:kind==='fast'?'#edf3f6':'#f6c04e'});
    labelObstacles.push({x:x-size-3,y:y-size-3,width:2*size+6,height:2*size+6});
    mark.addEventListener('pointerenter',event=>{clearHover();tooltip(event,description);});
    mark.addEventListener('pointermove',event=>tooltip(event,description));mark.addEventListener('pointerleave',clearHover);
    mark.addEventListener('focus',()=>{const box=mark.getBoundingClientRect();tooltip({clientX:box.right,clientY:box.top},description);});
    mark.addEventListener('blur',clearHover);
  }
  // Label only the visible profiles, not their faint benchmark samples. Prefer
  // a nearby offset along the line and avoid covering another label or marker.
  const labels=node('g',{class:'line-labels'},plot),placed=[];
  const overlap=(a,b)=>Math.max(0,Math.min(a.x+a.width,b.x+b.width)-Math.max(a.x,b.x))*Math.max(0,Math.min(a.y+a.height,b.y+b.height)-Math.max(a.y,b.y));
  for(const item of lineLabels){
    // A means-focused range can crop benchmark pairs. Keep their labels with
    // their lines rather than pinning labels for offscreen pairs to the border.
    if(Math.max(item.a.x,item.b.x)<margins.left||Math.min(item.a.x,item.b.x)>margins.left+w||
      Math.max(item.a.y,item.b.y)<margins.top||Math.min(item.a.y,item.b.y)>margins.top+h)continue;
    const label=text(labels,item.label,0,0,{class:'line-label',fill:displayColor(item.language),'data-label-language':item.language,'data-chart-language':item.language});
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
}
function renderTable(){
  const entries=sortedRows('comparison',currentEntries().map(entry=>({...entry,metrics:comparisonMetrics(entry)})),(entry,key)=>key==='language'?shortName(entry.language.key):entry.metrics[key]);
  $('#mean-table thead').innerHTML='<tr>'+[
    sortHeader('comparison','language','Language'),
    sortHeader('comparison','loc','LOC ×'),sortHeader('comparison','fastest','Fastest ×'),
    sortHeader('comparison','slope','Slope'),sortHeader('comparison','length','Length'),
    sortHeader('comparison','areaFast','2σ area','Fastest'),
    sortHeader('comparison','areaSmall','2σ area','Fewest LOC')].join('')+'</tr>';
  const slope=value=>value==null?'—':Number.isFinite(value)?value.toFixed(2):value<0?'−∞':'∞';
  const area=value=>value==null?'—':value===0?'0.00':value>=.01?value.toFixed(2):value.toPrecision(2);
  $('#mean-table tbody').innerHTML=entries.map(entry=>`<tr><td><button data-table-language="${entry.language.key}" class="table-language"><span class="color-key" style="${colorKeyStyle(entry.language.key)}"></span>${esc(shortName(entry.language.key))}${partialEntry(entry)?` <small class="coverage-badge" title="${esc(`${coverageLabel(entry)} selected benchmarks; missing ${missingNames(entry)}`)}">${coverageLabel(entry)}</small>`:''}</button></td><td class="ratio">${ratio(entry.metrics.loc)}</td><td class="ratio">${ratio(entry.metrics.fastest)}</td><td class="ratio" title="${entry.metrics.slope==null?'Same endpoint: no line slope':'Change in log₂ time divided by change in log₂ LOC'}">${slope(entry.metrics.slope)}</td><td class="ratio" title="Euclidean distance between endpoints in log₂ coordinates">${entry.metrics.length.toFixed(2)}</td><td class="ratio" data-area-profile="fast" title="Fastest-profile 2σ covariance ellipse area in squared doublings">${area(entry.metrics.areaFast)}</td><td class="ratio" data-area-profile="small" title="Fewest-LOC-profile 2σ covariance ellipse area in squared doublings">${area(entry.metrics.areaSmall)}</td></tr>`).join('');
}
function selectedProgram(){
  if(state.detail.program)return data.programs[state.detail.program];
  const group=benchmark(state.detail.benchmark)?.groups.find(g=>g.language===state.detail.language);
  if(!group)return null;
  let candidates=group.programs.filter(k=>!state.disabled.has(k)).map(k=>data.programs[k]);
  if(!candidates.length)return data.programs[group[state.detail.kind]??group.small];
  if(state.detail.kind==='fast'&&candidates.some(p=>p.seconds!=null))candidates=candidates.filter(p=>p.seconds!=null);
  return candidates.reduce((a,b)=>state.detail.kind==='fast'?(b.seconds<a.seconds||b.seconds===a.seconds&&b.loc<a.loc?b:a):(b.loc<a.loc||b.loc===a.loc&&b.seconds<a.seconds?b:a));
}
function renderInspector(){
  const program=selectedProgram();
  if(!program)return;
  const b=benchmark(program.benchmark),lang=language(program.language),group=b.groups.find(g=>g.language===program.language);
  const entry=model.entries.find(e=>e.language.key===program.language);
  const pending=program.seconds==null;
  if(pending)state.detail.kind='small';
  const available=data.benchmarks.filter(item=>item.groups.some(g=>g.language===program.language));
  $('#inspector-context').textContent=state.detail.aggregate?'Geometric mean contributors':b.title;
  $('#inspector-title').textContent=lang.label;
  const aggregate=state.detail.aggregate&&entry?`<p class="profile-note">${state.detail.kind==='fast'?'Fastest':'Fewest-LOC'} profile: <strong>${ratio(entry[state.detail.kind].x)} LOC · ${ratio(entry[state.detail.kind].y)} time</strong>, geometric means across ${coverageLabel(entry)} selected benchmarks.${partialEntry(entry)?` Missing: ${esc(missingNames(entry))}.`:''} Choose a benchmark below to inspect a contributor.</p>`:'';
  const file=data.files[program.files[Math.min(state.detail.file,program.files.length-1)]];
  if(tableSort.timings.key==='wall_seconds'&&program.execution_mode!=='resident')tableSort.timings.key='elapsed';
  const samples=sortedRows('timings',program.samples.map((elapsed,i)=>({...program.sample_details?.[i],run:i+1,elapsed})),(sample,key)=>sample[key]);
  const implementations=sortedRows('implementations',group.programs.map(key=>data.programs[key]),(item,key)=>({show:state.disabled.has(item.key)?1:0,program:item.id,loc:item.loc,median:item.seconds})[key]);
  const mech=program.mech_comparison;
  const mechComparison=mech?.scope==='paired-control'?`<p class="profile-note mech-comparison">This program: <strong>${program.seconds.toFixed(6)} s</strong> · <strong>${mech.time_x_mech.toFixed(3)}×</strong> the fresh paired Mech AOT control’s time (<strong>${mech.seconds.toFixed(6)} s</strong>, ${mech.control_loc} LOC).<br>LOC: <strong>${mech.control_loc_x_mech.toFixed(3)}×</strong> the paired control’s ${mech.control_loc}-line source.</p>`:mech?`<p class="profile-note mech-comparison">This program: <strong>${program.seconds.toFixed(6)} s</strong> · <strong>${mech.time_x_mech.toFixed(3)}× Mech AOT time</strong>, against the fresh <strong>${mech.seconds.toFixed(6)} s</strong> control.<br>LOC against the Mech sources recorded for this comparison: <strong>${mech.fastest_loc_x_mech.toFixed(3)}×</strong> fastest source (${mech.fastest_loc} LOC)${mech.fastest_loc===mech.smallest_loc?' and its fewest-LOC source':`; <strong>${mech.smallest_loc_x_mech.toFixed(3)}×</strong> its fewest-LOC source (${mech.smallest_loc} LOC)`}.</p>`:'';
  $('#inspector-body').innerHTML=`${aggregate}<div class="inspector-controls"><label>Benchmark<select id="detail-benchmark">${available.map(item=>`<option value="${item.key}" ${item.key===b.key?'selected':''}>${esc(item.title)}</option>`).join('')}</select></label><label>Profile<select id="detail-profile"><option value="fast" ${pending?'disabled':''} ${state.detail.kind==='fast'?'selected':''}>Fastest</option><option value="small" ${state.detail.kind==='small'?'selected':''}>Fewest LOC</option></select></label><label class="program-choice">Program<select id="detail-program">${group.programs.map(key=>`<option value="${key}" ${program.key===key?'selected':''}>${esc(data.programs[key].id)} · ${data.programs[key].loc} LOC · ${seconds(data.programs[key].seconds)}</option>`).join('')}</select></label></div>
    <label class="inclusion"><input id="include-program" type="checkbox" ${pending?'disabled':state.disabled.has(program.key)?'':'checked'}>Include this program in the speed comparison</label><p class="profile-note">${esc(b.workload)}</p>
    <div class="detail-stats"><div class="detail-stat"><strong>${program.loc}</strong><span>source LOC</span></div><div class="detail-stat"><strong>${seconds(program.seconds)}</strong><span>median measured time</span></div><div class="detail-stat"><strong>${seconds(program.timing_sd)}</strong><span>run-to-run SD</span></div></div>
    ${mechComparison}
    ${program.timing_summary?`<p class="profile-note timing-boundary">${esc(program.timing_summary)}</p>`:''}
    <section class="detail-section"><h3>Recorded timing samples</h3>${pending?'<p class="profile-note">Timing pending. This source is included for its audited LOC only and contributes no speed ratio, geometric mean, or standard-deviation ellipse.</p>':`<div class="timings-table-wrap"><table class="timings-table"><thead><tr>${sortHeader('timings','run','Run')}${sortHeader('timings','elapsed','Measured seconds')}${program.execution_mode==='resident'?sortHeader('timings','wall_seconds','Whole process seconds'):''}<th scope="col">${program.samples.length} samples</th></tr></thead><tbody>${samples.map(sample=>`<tr><td>${sample.run}</td><td class="sample-value">${sample.elapsed.toFixed(6)}</td>${program.execution_mode==='resident'?`<td class="sample-value">${sample.wall_seconds.toFixed(6)}</td>`:''}<td><div class="sample-bar" style="width:${sample.elapsed/Math.max(...program.samples)*100}%"></div></td></tr>`).join('')}</tbody></table></div>`}</section>
    ${program.physical_loc!=null&&program.physical_loc!==program.loc?`<details class="loc-audit"><summary>LOC audit: ${program.physical_loc} physical lines → ${program.loc} normalized LOC</summary><p class="profile-note">Packed sequential statements and nested assignments count separately. The original source is preserved below.</p>${program.loc_expansions.map(e=>`<p class="profile-note">Source line ${e.line}: ${e.extra_lines} extra LOC</p><pre>${esc(e.units?.join('\n')??e.reason??'')}</pre>`).join('')}</details>`:''}
    <section class="detail-section"><div class="code-toolbar"><select id="detail-file" aria-label="Source file">${program.files.map((key,i)=>`<option value="${i}" ${i===Math.min(state.detail.file,program.files.length-1)?'selected':''}>${esc(data.files[key].name)}${data.files[key].role==='runtime-support'?' · runtime support':''}</option>`).join('')}</select><a id="download-source" href="#">Download source</a>${program.source_url?`<a href="${esc(program.source_url)}" target="_blank" rel="noopener">${esc(program.source_link_label??'Upstream listing')}</a>`:''}${program.license_url?`<a href="${esc(program.license_url)}" target="_blank" rel="noopener">License</a>`:''}</div><pre><code id="source-code"></code></pre><div class="source-meta">${file.loc_note?esc(file.loc_note)+'<br>':''}Verified source SHA-256: ${file.sha256}<br>Timing report: ${esc(program.report)}</div></section>
    ${program.local_adaptation?`<p class="profile-note source-provenance">${esc(program.source_provenance)}</p>`:''}${program.algorithm?.reason?`<p class="profile-note">${esc(program.algorithm.reason)}</p>`:''}${program.runtime_policy?`<p class="profile-note">Runtime policy: ${esc(program.runtime_policy)}</p>`:''}${program.runtime_support?`<p class="profile-note">${esc(program.runtime_support)}</p>`:''}
    <section class="detail-section"><h3>Benchmark programs</h3><div class="variant-list"><table><thead><tr>${sortHeader('implementations','show','Show')}${sortHeader('implementations','program','Program')}${sortHeader('implementations','loc','LOC')}${sortHeader('implementations','median','Median')}</tr></thead><tbody>${implementations.map(item=>`<tr class="${item.key===program.key?'chosen':''}"><td><label><input data-toggle-program="${item.key}" type="checkbox" aria-label="Include ${esc(item.id)}" ${item.seconds==null?'disabled':state.disabled.has(item.key)?'':'checked'}></label></td><td><button data-inspect-program="${item.key}">${esc(item.id)}</button></td><td>${item.loc}</td><td>${seconds(item.seconds)}</td></tr>`).join('')}</tbody></table></div></section>`;
  $('#source-code').textContent=file.code;
  $('#download-source').onclick=event=>{event.preventDefault();const url=URL.createObjectURL(new Blob([file.code],{type:'text/plain'}));const link=document.createElement('a');link.href=url;link.download=file.name;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);};
}
function reset(){
  pauseYears();
  state.benchmarks=new Set(data.benchmarks.map(b=>b.key));state.languages=new Set(data.languages.map(l=>l.key));
  state.disabled.clear();state.ellipses=new Set(['mech']);state.view='summary';state.focus='nbody';
  state.collapsedFamilies.clear();
  state.traits=emptyTraitFilters();
  Object.assign(state,{fast:true,small:true,samples:true,focusMeans:true,global:false,trail:true,releaseYear:yearBounds().max});update();
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
      state.view='summary';update();return {benchmarks:[...state.benchmarks],languages:model.entries.map(e=>e.language.key),ellipses:[...state.ellipses]};
    }};
  try{Promise.resolve(document.modelContext.registerTool(tool,{signal:controller.signal})).catch(()=>{});}catch{}
  addEventListener('pagehide',()=>controller.abort(),{once:true});
}
async function start(){
  data=await fetch(new URL('./data.json',import.meta.url)).then(response=>{if(!response.ok)throw new Error('Could not load measured benchmark data.');return response.json();});
  traits=languageTraits(data);
  for(const l of data.languages)colors[l.key]=darkColor(l.color);
  const yearSources=[...new Map(data.languages.map(l=>[l.release_name,l])).values()].sort((a,b)=>a.release_year-b.release_year||a.release_name.localeCompare(b.release_name));
  $('#release-year-sources').innerHTML=yearSources.map(l=>`<li><a href="${esc(l.release_source)}" target="_blank" rel="noopener">${esc(l.release_name)} · ${l.release_year}</a><span>${esc(l.release_note)}</span></li>`).join('');
  $('#focused-benchmark').innerHTML=data.benchmarks.map(b=>`<option value="${b.key}">${esc(b.title)}</option>`).join('');
  reset();registerTools();
  for(const kind of ['benchmark','language']){
    const toggle=$(`#toggle-${kind}-filters`),body=$(`#${kind}-filter-body`);
    const setExpanded=value=>{toggle.setAttribute('aria-expanded',value);body.hidden=!value;};
    setExpanded(innerWidth>720);
    toggle.onclick=()=>setExpanded(body.hidden);
  }
  $('#all-benchmarks').onclick=()=>{state.benchmarks=new Set(data.benchmarks.map(b=>b.key));update();};
  $('#no-benchmarks').onclick=()=>{state.benchmarks.clear();update();};
  $('#all-languages').onclick=()=>{state.traits=emptyTraitFilters();state.languages=new Set(data.languages.map(l=>l.key));if(state.view==='mech')state.view='summary';update();};
  $('#no-languages').onclick=()=>{state.traits=emptyTraitFilters();state.languages.clear();update();};
  $('#only-mech').onclick=()=>{state.traits=emptyTraitFilters();state.languages=new Set(mechKeys());update();};
  $('#clear-traits').onclick=()=>{state.traits=emptyTraitFilters();update();};
  $('#release-year').oninput=event=>setReleaseYear(releaseYears()[Number(event.target.value)]);
  $('#play-years').onclick=playYears;
  addEventListener('pagehide',pauseYears,{once:true});
  $('#reset').onclick=reset;
  $('#close-inspector').onclick=closeInspector;
  $('#focused-benchmark').onchange=event=>{state.focus=event.target.value;update();};
  for(const [id,key] of [['fast-profile','fast'],['small-profile','small'],['show-samples','samples'],['focus-means','focusMeans'],['show-global','global'],['show-median-trail','trail']])$('#'+id).onchange=event=>{if(event.target.disabled)return;state[key]=event.target.checked;update();};
  document.addEventListener('click',event=>{
    const disclosure=event.target.closest('[data-toggle-family]');if(disclosure){
      const key=disclosure.dataset.toggleFamily;state.collapsedFamilies.has(key)?state.collapsedFamilies.delete(key):state.collapsedFamilies.add(key);
      renderControls();$(`[data-toggle-family="${key}"]`).focus({preventScroll:true});return;
    }
    const sort=event.target.closest('[data-sort-table]');if(sort){sortTable(sort);return;}
    const view=event.target.closest('[data-view]');if(view){if(view.dataset.view==='mech')enterMechView();else state.view=view.dataset.view;update();}
    const inspect=event.target.closest('[data-inspect-benchmark]');if(inspect){state.focus=inspect.dataset.inspectBenchmark;state.view='benchmark';update();openPoint({language:'mech',benchmark:state.focus});}
    const spread=event.target.closest('[data-spread]');if(spread){const key=spread.dataset.spread;state.ellipses.has(key)?state.ellipses.delete(key):state.ellipses.add(key);update();}
    const table=event.target.closest('[data-table-language]');if(table)openPoint({language:table.dataset.tableLanguage,benchmark:state.focus,aggregate:state.view!=='benchmark'});
    const program=event.target.closest('[data-inspect-program]');if(program){const p=data.programs[program.dataset.inspectProgram];openPoint({language:p.language,benchmark:p.benchmark,program:p.key});}
  });
  document.addEventListener('change',event=>{
    const element=event.target;
    if(element.disabled)return;
    if(element.dataset.family){
      const family=families().find(f=>f.key===element.dataset.family),allowed=yearKeys();
      for(const l of family.members)if(matchesTraits(traits.get(l.key),state.traits)&&allowed.has(l.key)&&(state.view!=='mech'||isMechLanguage(l.key)))element.checked?state.languages.add(l.key):state.languages.delete(l.key);
      update();return;
    }
    if(element.dataset.traitOption){
      const [dimension,option]=element.dataset.traitOption.split(':');
      element.checked?state.traits[dimension].add(option):state.traits[dimension].delete(option);
      update();return;
    }
    if(element.dataset.benchmark){element.checked?state.benchmarks.add(element.dataset.benchmark):state.benchmarks.delete(element.dataset.benchmark);update();}
    if(element.dataset.language){element.checked?state.languages.add(element.dataset.language):state.languages.delete(element.dataset.language);update();}
    const key=element.dataset.toggleProgram??(element.id==='include-program'?selectedProgram().key:null);
    if(key){state.detail.program=selectedProgram().key;element.checked?state.disabled.delete(key):state.disabled.add(key);update();}
    if(element.id==='detail-benchmark'){state.detail.benchmark=element.value;state.detail.program=null;state.detail.file=0;renderInspector();}
    if(element.id==='detail-profile'){state.detail.kind=element.value;state.detail.program=null;state.detail.file=0;renderInspector();}
    if(element.id==='detail-program'){state.detail.program=element.value;state.detail.file=0;renderInspector();}
    if(element.id==='detail-file'){state.detail.file=Number(element.value);renderInspector();}
  });
  document.addEventListener('keydown',event=>{if(event.key==='Escape'&&$('#inspector').open)closeInspector();});
  new ResizeObserver(()=>requestAnimationFrame(renderChart)).observe($('#chart-wrap'));
  window.benchmarkExplorer={getSelection:()=>({benchmarks:[...state.benchmarks],languages:[...effectiveLanguages()],selectedLanguages:[...state.languages],traits:Object.fromEntries(traitDimensions.map(({key})=>[key,[...state.traits[key]]])),releaseYear:state.releaseYear,ellipses:[...state.ellipses],disabled:[...state.disabled],view:state.view,samples:state.samples,focusMeans:state.focusMeans,global:state.global,trail:state.trail,playing:state.playing}),
    getMedianHistory:currentHistory,
    getGlobalStatistics:()=>globalStatistics(currentEntries()),
    getDomains:()=>({...currentDomain}),
    getSummary:()=>model.entries.map(e=>({language:e.language.key,fast:e.fast,small:e.small,n:e.inputs.length,selected:e.coverage.selected,missing:e.coverage.missing.map(b=>b.key),metrics:comparisonMetrics(e)})),setReleaseYear,inspectProgram:key=>{const program=data.programs[key];if(!program)throw new Error('Unknown program.');openPoint({language:program.language,benchmark:program.benchmark,program:key});}};
}
start().catch(error=>{$('#load-error').hidden=false;$('#load-error').textContent=error.message;$('#selection-summary').textContent='Data unavailable';console.error(error);});
