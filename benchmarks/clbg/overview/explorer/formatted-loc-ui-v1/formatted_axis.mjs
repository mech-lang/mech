// Source candidate only. The guarded Python overlay owns evidence admission.
// No canonical LOC or matched-control count becomes a formatted fallback.
const cache=new WeakMap();
export const hasFormattedLOC=p=>p.formatted_loc_status==='qualified'&&p.formatted_loc_complete===true&&
  Number.isSafeInteger(p.formatted_loc)&&p.formatted_loc>0;
export function axisData(data,metric='canonical'){
  if(metric==='canonical')return data;
  if(metric!=='formatted')throw new Error('Unknown source-line metric.');
  if(cache.has(data))return cache.get(data);
  const projected={...data,programs:Object.fromEntries(Object.entries(data.programs).map(([key,p])=>[
    key,{...p,loc:hasFormattedLOC(p)?p.formatted_loc:null,x:hasFormattedLOC(p)&&
      Number.isFinite(p.formatted_x)&&p.formatted_x>0?p.formatted_x:null}
  ])),benchmarks:data.benchmarks.map(b=>({...b,baseline_loc:b.baseline_formatted_loc??null,
    groups:b.groups.map(g=>({...g,fast:g.formatted_fast??null,small:g.formatted_small??null}))}))};
  cache.set(data,projected);return projected;
}
export function formattedText(program){
  return hasFormattedLOC(program)?`${program.formatted_loc} formatted LOC`:
    `— formatted LOC (${program.formatted_loc_status??'pending'})`;
}
export function inspectorProgram(data,projected,detail,disabled){
  if(detail.program)return data.programs[detail.program]??null;
  const group=projected.benchmarks.find(b=>b.key===detail.benchmark)?.groups.find(g=>g.language===detail.language);
  if(!group)return null;
  let candidates=group.programs.filter(k=>!disabled.has(k)).map(k=>projected.programs[k])
    .filter(p=>Number.isFinite(p.loc)&&p.loc>0&&Number.isFinite(p.x)&&p.x>0);
  if(!candidates.length){
    // The original source remains inspectable when the formatted endpoint is
    // unavailable. This selection never supplies a formatted chart coordinate.
    const canonical=data.benchmarks.find(b=>b.key===detail.benchmark)?.groups.find(g=>g.language===detail.language);
    return data.programs[canonical?.[detail.kind]??canonical?.small]??null;
  }
  if(detail.kind==='fast'&&candidates.some(p=>p.seconds!=null))candidates=candidates.filter(p=>p.seconds!=null);
  const chosen=candidates.reduce((a,b)=>detail.kind==='fast'?
    (b.seconds<a.seconds||b.seconds===a.seconds&&b.loc<a.loc?b:a):
    (b.loc<a.loc||b.loc===a.loc&&b.seconds<a.seconds?b:a));
  return data.programs[chosen.key];
}
// Both controls must independently name qualified counts of the exact source
// comparison. Generic canonical control_loc is deliberately ignored.
export function formattedMechRatio(program){
  // This packet does not admit matched-control formatter evidence. A future
  // source/receipt-bound control overlay must add that gate before any ratio.
  // Canonical control counts and unverified boolean labels cannot supply it.
  return null;
}
