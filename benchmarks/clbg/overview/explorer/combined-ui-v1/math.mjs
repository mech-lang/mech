export const gm=values=>Math.exp(values.reduce((sum,v)=>sum+Math.log(v),0)/values.length);
export function median(values){
  if(!values.length)return null;
  const sorted=values.slice().sort((a,b)=>a-b),middle=Math.floor(sorted.length/2);
  return sorted.length%2?sorted[middle]:(sorted[middle-1]+sorted[middle])/2;
}
export function languageFamilies(languages){
  const families=new Map();
  for(const language of languages){
    const key=language.family??language.key;
    if(!families.has(key))families.set(key,{key,label:language.family_label??language.label,members:[]});
    families.get(key).members.push(language);
  }
  for(const family of families.values())family.members.sort((a,b)=>(a.variant_order??0)-(b.variant_order??0));
  return [...families.values()];
}
// Each visible execution entry has one vote, independent of its benchmark or
// timing-sample count. The plotted geometric average is the mean in log space.
export function globalStatistics(entries){
  return Object.fromEntries(['fast','small'].map(kind=>{
    const points=entries.map(entry=>entry[kind]);
    if(!points.length)return [kind,null];
    const midpoint={x:median(points.map(p=>p.x)),y:median(points.map(p=>p.y))};
    // Covariance measures dispersion about the log-space mean. Translate its
    // contours to the coordinate median without changing covariance or area.
    const atMedian=region=>region&&({...region,center:midpoint,
      boundary:region.boundary.map(p=>({x:p.x*midpoint.x/region.center.x,y:p.y*midpoint.y/region.center.y}))});
    return [kind,{n:points.length,median:midpoint,
      spread:atMedian(spread(points)),spread2:atMedian(spread(points,{sigma:2}))}];
  }));
}
// Reuse the chart's coordinate median, with sample SD in the same log2 units
// as its covariance. Each current eligible execution entry has one vote.
export function medianRankings(entries){
  const eligible=entries.filter(entry=>[entry.fast?.y,entry.small?.x].every(value=>Number.isFinite(value)&&value>0));
  const metric=field=>{
    const values=eligible.map(entry=>field==='speed'?entry.fast.y:entry.small.x),n=values.length;
    if(!n)return {n:0,median:null,log_mean:null,log_sample_sd:0,max_abs_sigma:0,entries:[],ties:[],bands:[]};
    const midpoint=median(values),logs=values.map(Math.log2),mean=logs.reduce((sum,value)=>sum+value,0)/n;
    const sd=n<2?0:Math.sqrt(logs.reduce((sum,value)=>sum+(value-mean)**2,0)/(n-1));
    const rows=eligible.map((entry,index)=>{
      const value=values[index],tie=value===midpoint,z=tie||sd===0?0:(logs[index]-Math.log2(midpoint))/sd;
      return {entry,value,ratio_to_median:value/midpoint,sigma_distance:z,
        direction:tie?'tie':value<midpoint?'below':'above',sigma_band:tie?0:Math.max(1,Math.ceil(Math.abs(z)))};
    });
    const strongest=(a,b)=>Math.abs(b.sigma_distance)-Math.abs(a.sigma_distance)||
      (a.direction==='below'?a.value-b.value:b.value-a.value)||a.entry.language.key.localeCompare(b.entry.language.key);
    const ties=rows.filter(row=>row.direction==='tie').sort((a,b)=>a.entry.language.key.localeCompare(b.entry.language.key));
    const max=rows.reduce((upper,row)=>Math.max(upper,Math.abs(row.sigma_distance)),0);
    const bands=Array.from({length:Math.max(1,Math.ceil(max))},(_,index)=>({lower_sigma:index,upper_sigma:index+1,
      below:rows.filter(row=>row.direction==='below'&&row.sigma_band===index+1).sort(strongest),
      above:rows.filter(row=>row.direction==='above'&&row.sigma_band===index+1).sort(strongest)}));
    return {n,median:midpoint,log_mean:mean,log_sample_sd:sd,max_abs_sigma:max,entries:rows,ties,bands};
  };
  return {speed:metric('speed'),loc:metric('loc')};
}
export function recordedSamplePoints(entries,{samples=true,disabled=new Set(),programsForInput=input=>input.programs??[input.fast,input.small]}={}){
  if(!samples)return [];
  const points=[],seen=new Set();
  for(const entry of entries)for(const input of entry.inputs)for(const program of programsForInput(input,entry)){
    if(!program||disabled.has(program.key)||seen.has(program.key)||!Array.isArray(program.samples)||
      ![program.loc,program.x,program.y,program.seconds,input.benchmark.baseline_seconds].every(value=>Number.isFinite(value)&&value>0))continue;
    seen.add(program.key);
    for(const [index,elapsed] of program.samples.entries())if(Number.isFinite(elapsed)&&elapsed>0)
      points.push({x:program.x,y:elapsed/input.benchmark.baseline_seconds,program,entry,benchmark:input.benchmark,run:index+1,elapsed});
  }
  return points;
}
export function comparisonDensityPoints(entries,{fast=true,small=true}={}){
  const points=[];
  for(const entry of entries){
    const positions=[];
    for(const kind of ['fast','small'])if((kind==='fast'?fast:small)&&entry[kind]&&
      [entry[kind].x,entry[kind].y].every(value=>Number.isFinite(value)&&value>0)){
      const position=positions.find(point=>same(point,entry[kind]));
      if(position)position.profiles.push(kind);
      else positions.push({x:entry[kind].x,y:entry[kind].y,profiles:[kind]});
    }
    for(const position of positions)points.push({...position,weight:1/positions.length,language:entry.language.key});
  }
  return points;
}
export function densityGrid(points,domain,{columns=48,rows=36,bandwidthCells=1.25}={}){
  const {xMin,xMax,yMin,yMax}=domain;
  if(![xMin,xMax,yMin,yMax].every(Number.isFinite)||xMax<=xMin||yMax<=yMin||
    ![columns,rows].every(value=>Number.isInteger(value)&&value>0&&value<=120)||!Number.isFinite(bandwidthCells)||bandwidthCells<=0)
    throw new Error('Invalid log2 density grid.');
  const inputs=points.filter(point=>[point.x,point.y,point.weight].every(value=>Number.isFinite(value)&&value>0))
    .map(point=>({...point,lx:Math.log2(point.x),ly:Math.log2(point.y)}));
  const bandwidth_x=(xMax-xMin)/columns*bandwidthCells,bandwidth_y=(yMax-yMin)/rows*bandwidthCells;
  const scale=1/(2*Math.PI*bandwidth_x*bandwidth_y),cells=[];
  let peak=0;
  for(let row=0;row<rows;row++)for(let column=0;column<columns;column++){
    const lx=xMin+(column+.5)*(xMax-xMin)/columns,ly=yMin+(row+.5)*(yMax-yMin)/rows;
    const value=inputs.reduce((sum,point)=>sum+point.weight*Math.exp(-.5*(((lx-point.lx)/bandwidth_x)**2+((ly-point.ly)/bandwidth_y)**2)),0)*scale;
    cells.push({row,column,value});peak=Math.max(peak,value);
  }
  return {columns,rows,cells,peak,bandwidth_x,bandwidth_y,profile_positions:inputs.length,
    variants:new Set(inputs.map(point=>point.language)).size,total_variant_weight:inputs.reduce((sum,point)=>sum+point.weight,0)};
}
export function medianHistory(data,selection,{from=-Infinity,to=Infinity}={}){
  const languages=data.languages.filter(l=>selection.languages.has(l.key));
  const years=releaseYearsInWindow(languages,{from,to});
  return years.map(year=>{
    const eligible=new Set(languagesByYear(languages,{from,to:year}));
    const entries=comparison(data,{...selection,languages:eligible}).entries;
    return {year,statistics:globalStatistics(entries)};
  });
}
export const same=(a,b)=>a&&b&&Math.abs(a.x-b.x)<1e-10&&Math.abs(a.y-b.y)<1e-10;
export function lineMetrics(fast,small){
  const dx=Math.log2(fast.x/small.x),dy=Math.log2(fast.y/small.y);
  const point=Math.abs(dx)<1e-10&&Math.abs(dy)<1e-10;
  return {loc:small.x,fastest:fast.y,slope:point?null:Math.abs(dx)<1e-10?(dy<0?-Infinity:Infinity):dy/dx,
    length:point?0:Math.hypot(dx,dy)};
}
export function comparisonMetrics(entry){
  return {...lineMetrics(entry.fast,entry.small),
    areaFast:entry.spread2.fast?.area??null,areaSmall:entry.spread2.small?.area??null};
}
export function languagesByYear(languages,{year=Infinity,from=-Infinity,to=year}={}){
  return languages.filter(l=>Number.isInteger(l.release_year)&&l.release_year>=from&&l.release_year<=to).map(l=>l.key);
}
export function releaseYearsInWindow(languages,{from=-Infinity,to=Infinity}={}){
  const eligible=new Set(languagesByYear(languages,{from,to}));
  return [...new Set(languages.filter(l=>eligible.has(l.key)).map(l=>l.release_year))].sort((a,b)=>a-b);
}
export function clampReleaseWindow({from,to},{min,max},moving='both'){
  if(![from,to,min,max].every(Number.isInteger)||min>max||!['start','end','both'].includes(moving))throw new Error('Invalid release-year window.');
  from=Math.max(min,Math.min(max,from));to=Math.max(min,Math.min(max,to));
  if(from>to){
    if(moving==='start')from=to;
    else if(moving==='end')to=from;
    else throw new Error('Release-year window must not cross.');
  }
  return {from,to};
}
export function decadeWindow(decade,{min,max}){
  if(!Number.isInteger(decade)||decade%10!==0||![min,max].every(Number.isInteger)||min>max)throw new Error('Invalid release decade.');
  if(decade>max||decade+9<min)return null;
  return {from:Math.max(min,decade),to:Math.min(max,decade+9)};
}
export function spread(points,{sigma=1}={}){
  if(!Number.isFinite(sigma)||sigma<=0)throw new Error('Covariance radius must be positive.');
  if(points.length<2)return null;
  const logs=points.map(p=>[Math.log2(p.x),Math.log2(p.y)]),n=logs.length;
  const center=[0,1].map(i=>logs.reduce((s,p)=>s+p[i],0)/n);
  const cov=(i,j)=>logs.reduce((s,p)=>s+(p[i]-center[i])*(p[j]-center[j]),0)/(n-1);
  const a=cov(0,0),b=cov(0,1),d=cov(1,1),delta=Math.hypot(a-d,2*b);
  const radii=[Math.sqrt(Math.max(0,(a+d+delta)/2)),Math.sqrt(Math.max(0,(a+d-delta)/2))];
  const angle=.5*Math.atan2(2*b,a-d),ca=Math.cos(angle),sa=Math.sin(angle);
  const boundary=Array.from({length:121},(_,i)=>{
    const t=2*Math.PI*i/120,u=sigma*radii[0]*Math.cos(t),v=sigma*radii[1]*Math.sin(t);
    return {x:2**(center[0]+ca*u-sa*v),y:2**(center[1]+sa*u+ca*v)};
  });
  // Two observations have rank-one covariance and exactly zero ellipse area.
  const area=n===2?0:Math.PI*sigma**2*radii[0]*radii[1];
  return {n,sigma,log_base:2,area,area_units:'squared_doublings',
    center:{x:2**center[0],y:2**center[1]},covariance:[[a,b],[b,d]],boundary};
}
export function sharedBenchmarks(data,selection){
  const languages=data.languages.filter(l=>selection.languages.has(l.key));
  if(!languages.length)return [];
  return data.benchmarks.filter(b=>selection.benchmarks.has(b.key)&&languages.every(l=>{
    const group=b.groups.find(g=>g.language===l.key);
    return group?.programs.some(k=>!selection.disabled.has(k)&&Number.isFinite(data.programs[k].seconds));
  }));
}
export function comparison(data,selection){
  const benchmarks=data.benchmarks.filter(b=>selection.benchmarks.has(b.key));
  const entries=[];
  for(const language of data.languages){
    if(!selection.languages.has(language.key))continue;
    const inputs=[];
    for(const benchmark of benchmarks){
      const group=benchmark.groups.find(g=>g.language===language.key);
      const programs=(group?.programs??[]).filter(k=>!selection.disabled.has(k)).map(k=>data.programs[k]).filter(p=>Number.isFinite(p?.seconds)&&p.seconds>0&&Number.isFinite(p.loc)&&p.loc>0&&Number.isFinite(p.x)&&p.x>0&&Number.isFinite(p.y)&&p.y>0);
      if(!programs.length)continue;
      const fast=programs.reduce((a,b)=>b.seconds<a.seconds||(b.seconds===a.seconds&&b.loc<a.loc)?b:a);
      const small=programs.reduce((a,b)=>b.loc<a.loc||(b.loc===a.loc&&b.seconds<a.seconds)?b:a);
      inputs.push({benchmark,fast,small,programs});
    }
    if(!inputs.length)continue;
    const included=new Set(inputs.map(input=>input.benchmark.key));
    const coverage={measured:inputs.length,selected:benchmarks.length,missing:benchmarks.filter(b=>!included.has(b.key))};
    const mean=kind=>({x:gm(inputs.map(i=>i[kind].x)),y:gm(inputs.map(i=>i[kind].y))});
    entries.push({language,inputs,coverage,fast:mean('fast'),small:mean('small'),
      spread:{fast:spread(inputs.map(i=>i.fast)),small:spread(inputs.map(i=>i.small))},
      spread2:{fast:spread(inputs.map(i=>i.fast),{sigma:2}),small:spread(inputs.map(i=>i.small),{sigma:2})}});
  }
  return {benchmarks,entries};
}

// Axis limits use the admitted cohort, independently of display filters. Include
// incomplete-coverage cohorts and shared subsets in the same reference frame.
export function chartDomain(data,{benchmarks,view='summary',focus='nbody',focusMeans=true}){
  const requested=data.benchmarks.filter(b=>view==='benchmark'?b.key===focus:benchmarks.has(b.key));
  const languages=view==='mech'?data.languages.filter(l=>l.family==='mech'||l.key==='mech'):data.languages;
  const cohorts=new Map();
  const addCohort=items=>{if(items.length)cohorts.set(items.map(b=>b.key).join(','),items);};
  addCohort(requested);
  for(const language of languages){
    const available=requested.filter(b=>b.groups.some(g=>g.language===language.key&&g.programs.some(key=>data.programs[key].seconds>0)));
    // Intersections also cover groups of partially measured languages.
    for(const cohort of [...cohorts.values()])addCohort(cohort.filter(b=>available.includes(b)));
    addCohort(available);
  }
  let xUpper=1,yUpper=1;
  const include=point=>{xUpper=Math.max(xUpper,point.x);yUpper=Math.max(yUpper,point.y);};
  const includeAll=points=>{for(const point of points)include(point);};
  for(const cohort of cohorts.values()){
    const reference=comparison(data,{benchmarks:new Set(cohort.map(b=>b.key)),languages:new Set(languages.map(l=>l.key)),disabled:new Set()});
    for(const entry of reference.entries){
      if(view==='benchmark'||!focusMeans)for(const input of entry.inputs)for(const program of input.programs){
        if(!Number.isFinite(program.x)||program.x<=0)continue;
        include({x:program.x,y:Number.isFinite(program.y)&&program.y>0?program.y:1});
        for(const elapsed of program.samples??[])if(Number.isFinite(elapsed)&&elapsed>0&&Number.isFinite(input.benchmark.baseline_seconds)&&input.benchmark.baseline_seconds>0)
          include({x:program.x,y:elapsed/input.benchmark.baseline_seconds});
      }
      for(const kind of ['fast','small']){
        if(view!=='benchmark'&&focusMeans)include(entry[kind]);
        else{
          includeAll(entry.inputs.map(input=>input[kind]));
          if(view!=='benchmark'&&entry.spread2[kind])includeAll(entry.spread2[kind].boundary);
        }
      }
    }
  }
  if(view==='benchmark'||!focusMeans){
    // Fit the global overlay and every cumulative release-year contour even
    // when they are hidden. Using the admitted cohort keeps year, language,
    // profile and overlay toggles from changing this range. Means-focused
    // views retain their compact endpoint range and intentionally crop SD.
    const selection={benchmarks:new Set(requested.map(b=>b.key)),
      languages:new Set(languages.map(l=>l.key)),disabled:new Set()};
    const includeGlobal=statistics=>{
      for(const kind of ['fast','small'])if(statistics[kind]){
        include(statistics[kind].median);
        if(statistics[kind].spread2)includeAll(statistics[kind].spread2.boundary);
      }
    };
    includeGlobal(globalStatistics(comparison(data,selection).entries));
    for(const item of medianHistory(data,selection))includeGlobal(item.statistics);
  }
  // Measured ratios have a fixed 1x baseline. Keep a little marker padding,
  // with 1x as the first tick, independently of covariance tails below 1x.
  const xMin=-.15,yMin=-.18;
  return {xMin,xMax:Math.max(xMin+1.3,Math.log2(xUpper)+.25),
    yMin,yMax:Math.max(yMin+1.3,Math.log2(yUpper)+.25)};
}
