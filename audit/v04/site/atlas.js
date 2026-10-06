const $ = id => document.getElementById(id);
const number = n => Number(n || 0).toLocaleString('en-US');
const get = async path => { const r = await fetch(`../data/${path}`); if (!r.ok) throw new Error(`${path}: HTTP ${r.status}`); return r.json(); };
const escape = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const records = x => Array.isArray(x) ? x : x.capabilities || x.findings || x.records || x.boundaries || x.components || [];
let census, history, dependencies, selected = null, findingData=[];
const source = path => `https://github.com/mech-lang/mech/blob/${census.metadata.baseline_commit}/${path}`;
function bars(element, rows, key, onClick) {
  element.replaceChildren(); const max = Math.max(1, ...rows.map(r => Math.abs(r[key] || 0)));
  for (const row of [...rows].sort((a,b) => Math.abs(b[key] || 0) - Math.abs(a[key] || 0))) {
    const button = document.createElement('button'); button.className = `bar-row ${(row[key] || 0) < 0 ? 'negative' : ''}`;
    button.innerHTML = `<span>${escape(row.name)}</span><span class="bar-track"><span class="bar-fill" style="display:block;width:${Math.abs(row[key] || 0)/max*100}%"></span></span><span class="bar-value">${number(row[key])}</span>`;
    button.title = `${row.name}: ${number(row[key])} ${key}; ${number(row.files)} files`;
    button.onclick = () => onClick?.(row); element.append(button);
  }
}
function composition() {
  const key = $('measure').value;
  bars($('subsystems'), census.metadata.by_subsystem, key, row => {selected = selected === row.name ? null : row.name; files();});
  bars($('roles'), census.metadata.by_role, key, row => {$('search').value = row.name; selected=null; files();}); files();
}
function files() {
  const query = $('search').value.toLowerCase();
  const rows = census.files.filter(f => (!selected || f.subsystem === selected) && (!query || `${f.path} ${f.role} ${f.responsibility}`.toLowerCase().includes(query)));
  $('file-title').textContent = selected || 'All files';
  $('files').innerHTML = rows.map(f => `<tr><td><a href="${source(f.path)}" target="_blank" rel="noopener">${escape(f.path)}</a><br><small>${escape(f.blob.slice(0,12))}</small></td><td>${escape(f.responsibility)}</td><td title="${escape(f.notes.join(' '))}">${escape(f.role)}</td><td class="numeric">${number(f.physical_lines)}</td><td class="numeric">${number(f.bytes)}</td></tr>`).join('');
  $('file-count').textContent = `${number(rows.length)} files; ${number(rows.reduce((a,f)=>a+f.physical_lines,0))} physical text lines. Every file appears once in this selection.`;
  const paths=new Set(rows.map(r=>r.path)),related=findingData.filter(f=>(f.locations||[]).some(l=>paths.has(l.path)));
  $('subsystem-findings').innerHTML=related.length?`<p>Findings in this selection: ${related.map(f=>`<a href="#finding-${escape(f.id)}">${escape(f.id)} · ${escape(f.title)}</a>`).join(' · ')}</p>`:'<p>Recorded findings in this selection: 0. Refer to the manual review coverage for inspected spans.</p>';
}
function graph() {
  const kind = $('edge-kind').value, nodes = dependencies.nodes.filter(n => !n.path.startsWith('tests/'));
  const group = n => n.path.startsWith('machines/') || n.id==='mech-stdlib' ? 'Standard library' : n.path.startsWith('hosts/') ? 'Hosts' : n.id==='mech-wasm' ? 'WASM' : n.id==='mech' ? 'Product CLI' : 'Language / runtime';
  const byName = new Map(nodes.map(n => [n.id,group(n)])); const groups=['Product CLI','Language / runtime','Standard library','Hosts','WASM','External crates'];
  const edges = new Map();
  for(const e of dependencies.edges.filter(e=>e.kind===kind && byName.has(e.source))){const a=byName.get(e.source), b=byName.get(e.target)||'External crates';const key=`${a} → ${b}`; if(!edges.has(key)) edges.set(key,{a,b,records:[]});edges.get(key).records.push(e);}
  const ns='http://www.w3.org/2000/svg', svg=document.createElementNS(ns,'svg'); svg.setAttribute('viewBox','0 0 1000 360'); svg.setAttribute('role','img');svg.setAttribute('aria-label','Cargo dependency declarations collapsed into six ownership groups');
  const positions = new Map(groups.map((g,i)=>[g,{x:150+(i%3)*350,y:85+Math.floor(i/3)*195}]));
  for(const [key,e] of edges){const a=positions.get(e.a),b=positions.get(e.b);const path=document.createElementNS(ns,'path');path.classList.add('dep-edge');path.setAttribute('fill','none');path.setAttribute('stroke-width',Math.min(10,1+Math.sqrt(e.records.length)));path.setAttribute('d',e.a===e.b?`M${a.x-30},${a.y-10} C${a.x-90},${a.y-90} ${a.x+90},${a.y-90} ${a.x+30},${a.y-10}`:`M${a.x},${a.y} Q500,180 ${b.x},${b.y}`);const title=document.createElementNS(ns,'title');title.textContent=`${key}: ${e.records.length} declarations`;path.append(title);path.onclick=()=>{$('dependency-detail').textContent=JSON.stringify({relationship:key,kind,records:e.records},null,2);};svg.append(path);}
  for(const [name,p] of positions){const rect=document.createElementNS(ns,'rect');for(const [k,v]of Object.entries({x:p.x-87,y:p.y-21,width:174,height:42,rx:5}))rect.setAttribute(k,v);rect.classList.add('dep-node');svg.append(rect);const text=document.createElementNS(ns,'text');text.setAttribute('x',p.x);text.setAttribute('y',p.y+5);text.setAttribute('text-anchor','middle');text.textContent=name;svg.append(text);}
  $('dependency-graph').replaceChildren(svg);
}
function jsonDetails(value, label='Underlying record') { return `<details><summary>${escape(label)}</summary><pre>${escape(JSON.stringify(value,null,2))}</pre></details>`; }
async function supplementary(name,id,render) { try { render(await get(name)); } catch(e) {$(id).innerHTML=`<p class="warning">Record pending: ${escape(e.message)}. Status: unverified.</p>`;} }
try {
  [census,history,dependencies] = await Promise.all([get('census.json'),get('history.json'),get('dependencies.json')]);
  const m=census.metadata;
  $('identity').textContent=`Baseline ${m.baseline_commit} · release comparison v0.3.5-beta · recorded audit`;
  $('scope').textContent=`Units: ${m.units}. Scope: ${m.scope}`;
  const prod=m.by_role.find(r=>r.name==='production'), mixed=m.by_role.find(r=>r.name==='production-mixed-tests');
  $('summary').innerHTML=[[m.totals.files,'tracked files'],[m.totals.physical_lines,'physical text lines, all roles'],[prod.physical_lines,'lines in pure-production files'],[mixed.physical_lines,'lines in files mixing production and tests']].map(([n,l])=>`<div class="metric"><strong>${number(n)}</strong><span>${l}</span></div>`).join('');
  $('method').textContent=JSON.stringify({counter:m.counter_version,classification:m.classification_policy,mixed:m.mixed_policy,generated:m.generated_policy,unresolved:m.unresolved},null,2);
  composition(); $('measure').onchange=composition; $('search').oninput=files;
  $('history-scope').textContent=`${history.comparison_label}. ${history.comparison_commit} → ${history.baseline_commit}. ${history.rename_detection}. Signed difference in physical text lines across all roles; moved files are attributed to their destination subsystem.`;
  const by=new Map();for(const row of history.changes){if(!by.has(row.subsystem))by.set(row.subsystem,{name:row.subsystem,physical_lines:0,files:0});const g=by.get(row.subsystem);g.physical_lines+=row.delta_lines;g.files++;}bars($('history-bars'),[...by.values()],'physical_lines');
  $('changes').innerHTML=history.changes.map(r=>`<tr><td>${escape(r.status)}</td><td>${escape(r.before||'∅')} → ${escape(r.after||'∅')}</td><td class="numeric">${number(r.delta_lines)}</td></tr>`).join('');
  $('dependency-scope').textContent=dependencies.meaning; graph();$('edge-kind').onchange=graph;
  await supplementary('extraction-boundaries.json','extraction-view',data=>{
    const a=data.arithmetic, unit='physical_lines';
    const rows=[['Baseline',a.baseline],['Proposed external move',a.estimated_moved],['Estimated remaining, before boundary additions',a.estimated_remaining_before_boundary_additions],['Measured remaining, retained integration tests',a.actual_remaining],['Measured external',a.actual_external],['Measured combined',a.actual_combined]];
    const baseline=a.baseline[unit],moved=a.estimated_moved[unit],remaining=a.estimated_remaining_before_boundary_additions[unit],boundary=a.actual_remaining[unit]-remaining,y=v=>230-v/baseline*180;
    const waterfall=`<svg viewBox="0 0 1000 285" role="img" aria-label="Extraction estimate waterfall in physical text lines" style="width:100%;max-height:300px"><line x1="20" x2="980" y1="230" y2="230" stroke="#bacacc"/><rect x="45" y="${y(baseline)}" width="125" height="180" fill="#37858a"/><rect x="235" y="${y(baseline)}" width="125" height="${moved/baseline*180}" fill="#987452"/><line x1="170" x2="235" y1="${y(baseline)}" y2="${y(baseline)}" stroke="#7c969a" stroke-dasharray="4"/><line x1="360" x2="590" y1="${y(remaining)}" y2="${y(remaining)}" stroke="#7c969a" stroke-dasharray="4"/><rect x="610" y="45" width="145" height="185" fill="none" stroke="#b39457" stroke-dasharray="5"/><rect x="820" y="${y(a.actual_remaining[unit])}" width="125" height="${a.actual_remaining[unit]/baseline*180}" fill="#37858a"/><g fill="#28414b" font-size="13" text-anchor="middle"><text x="108" y="35">${number(baseline)}</text><text x="298" y="35">−${number(moved)}</text><text x="478" y="${y(remaining)-12}">0 deleted</text><text x="683" y="120">Boundary / repair delta</text><text x="683" y="145">${number(boundary)} net lines</text><text x="882" y="35">${number(a.actual_remaining[unit])}</text><text x="108" y="258">Baseline</text><text x="298" y="258">External move</text><text x="478" y="258">Deletion</text><text x="683" y="258">New boundary material</text><text x="882" y="258">Measured remaining</text></g></svg>`;
    $('extraction-view').innerHTML=`<p>All-role physical text lines; baseline ${escape(data.metadata.baseline)}. The measured layout retains ${number(a.estimated_retained_integration_tests[unit])} integration-test lines and their build script. Core boundary and repair changes add ${number(boundary)} net lines; external changes add ${number(a.actual_external[unit]-moved)} net lines. Additional release and CI migration work is listed in the boundary analysis.</p>${waterfall}<details><summary>Compare estimated, measured and combined totals</summary><div id="extraction-bars"></div></details><p><strong>${number(baseline)} − ${number(moved)} − ${number(a.deleted_implementation[unit])} + ${number(boundary)} net boundary/repair lines = ${number(a.actual_remaining[unit])} remaining core lines.</strong></p>${jsonDetails(data.by_role,'Production, mixed files and supporting material, counted separately')}${jsonDetails(data.limits,'Limitations')}${jsonDetails(data,'All boundary and file records')}`;
    bars($('extraction-bars'),rows.map(([name,r])=>({name,...r})),unit);
  });
  await supplementary('capabilities.json','capability-view',data=>{const stageNames=['parsing','semantic_checking','artifact_construction','activation','execution','correct_publication'];const matrix=`<div class="table-wrap"><table><thead><tr><th>Capability</th>${stageNames.map(s=>`<th>${escape(s.replaceAll('_',' '))}</th>`).join('')}</tr></thead><tbody>${records(data).map(r=>`<tr><td><a href="#cap-${escape(r.id)}">${escape(r.id)}</a></td>${stageNames.map(s=>`<td>${escape(r.stages?.[s]||'unverified')}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`;$('capability-view').innerHTML=matrix+records(data).map(r=>`<article class="finding" id="cap-${escape(r.id)}"><h3>${escape(r.id||'')} ${escape(r.claim||r.name||r.capability)}</h3><span class="badge ${escape(r.status||r.outcome)}">${escape(r.status||r.outcome||'unverified')}</span><p>${escape(typeof r.limits==='string'?r.limits:JSON.stringify(r.limits||[]))}</p>${jsonDetails(r,'Interfaces, configurations, stages and evidence')}</article>`).join('');});
  await supplementary('findings.json','finding-view',data=>{findingData=records(data);$('finding-view').innerHTML=findingData.map(r=>`<article class="finding" id="finding-${escape(r.id)}"><h3>${escape(r.id||'')} ${escape(r.title||r.finding)}</h3><p>${escape(r.consequence||r.observation||'')}</p>${jsonDetails(r,'Location, evidence, proposed action and validation')}</article>`).join('')||jsonDetails(data);files();});
  await supplementary('evidence.json','evidence-view',data=>{$('evidence-view').innerHTML=`<div class="table-wrap"><table><thead><tr><th>Claim</th><th>Evidence level</th><th>Outcome</th><th>Reproduction</th></tr></thead><tbody>${records(data).map(r=>`<tr><td>${escape(r.claim||r.id)}</td><td>${escape(r.evidence_level||'See detailed record')}</td><td>${escape(r.outcome||r.status||'unverified')}</td><td>${jsonDetails(r,'Record and command')}</td></tr>`).join('')}</tbody></table></div>`;});
} catch(e) {$('scope').textContent=`Audit data failed to load: ${e.message}. Serve this repository over HTTP using the documented command.`;}
