const aliases = {
  artifact_bytes:'Compiled program size', artifact_transport:'Program transport', correct_publication:'Publication',
  semantic_checking:'Type checking', artifact_construction:'Program compilation', epoch:'Publication epoch',
  scalar_text:'Value', max_absolute_error:'Maximum absolute error', loaded_wasm_sha256:'Loaded executable SHA-256',
  source_commit:'Source revision', baseline_commit:'Baseline revision', source_sha256:'Source SHA-256',
  source_modifications:'Source changes', source_modifications_sha256:'Source changes SHA-256',
  loaded_wasm_bytes:'Executable size', source_bytes:'Source size', bytecode_bytes:'Bytecode size',
  actual_stock:'Published stock', expected_stock:'Expected stock', accepted_state_unchanged:'Accepted state preserved',
  resident_accepted_turns:'Accepted turns', requested:'Requested backend', selected:'Selected backend', completed:'Completed backend',
};
const label = key => aliases[key] || String(key).replaceAll('_',' ').replace(/\b(?:cpu|gpu|wasm|html|json|sha256|id)\b/gi,s=>s.toUpperCase()).replace(/^./,s=>s.toUpperCase());
const el = (tag, className, text) => {const node=document.createElement(tag);if(className)node.className=className;if(text!==undefined)node.textContent=text;return node;};
const sourceKeys = /^(source|configuration|config|command|native_source|fixture_source|code|command_line)$/;
const allowedTags = new Set(['SPAN','DIV','PRE','CODE','TABLE','THEAD','TBODY','TFOOT','TR','TH','TD','CAPTION','DL','DT','DD','UL','OL','LI','BR','STRONG','EM']);
export function renderMechValue(value) {
  const container=el('div','mech-formatted-value');
  if (typeof value?.html === 'string') {
    const template=document.createElement('template');template.innerHTML=value.html;
    for (const node of [...template.content.querySelectorAll('*')]) {
      if (!allowedTags.has(node.tagName)) {node.replaceWith(document.createTextNode(node.textContent));continue;}
      for (const attribute of [...node.attributes]) {
        if (attribute.name==='class') node.className=node.className.split(/\s+/).filter(name=>/^mech-[a-z0-9_-]+$/i.test(name)).join(' ');
        else if (!(attribute.name === 'scope' && ['row','col'].includes(attribute.value)) && !(['colspan','rowspan'].includes(attribute.name) && /^\d+$/.test(attribute.value))) node.removeAttribute(attribute.name);
      }
    }
    container.append(template.content);
  } else if (value?.scalar_text != null || value?.text != null) container.textContent=value.scalar_text ?? value.text;
  else container.textContent=readableValue(value?.value ?? value?.error ?? 'Value formatting unavailable.');
  return container;
}
function readableValue(text) {
  return String(text).replace(/F64Bits\((\d+)\)/g,(_,bits)=>{try{const view=new DataView(new ArrayBuffer(8));view.setBigUint64(0,BigInt(bits));return String(view.getFloat64(0));}catch{return 'Floating-point value';}})
    .replace(/F32Bits\((\d+)\)/g,(_,bits)=>{const view=new DataView(new ArrayBuffer(4));view.setUint32(0,Number(bits));return String(view.getFloat32(0));})
    .replace(/\b(?:InstanceEpoch|Revision|DocumentId)\((\d+)\)/g,'$1');
}
function schemaLabel(text) {
  const s=String(text);
  if (s.includes('Record(')) {
    const fields=[...s.matchAll(/SchemaField \{ name: "([^"]+)", schema: ([^}]+)\}/g)];
    return fields.length ? `Record · ${fields.map(m=>`${m[1]}: ${schemaLabel(m[2])}`).join(' · ')}` : 'Record';
  }
  if(s.includes('Table'))return 'Table';
  if(s.includes('Matrix')) {const sizes=[...s.matchAll(/Constant\((\d+)\)/g)].map(m=>m[1]);const element=s.match(/element: ([^,]+)/)?.[1];return `${sizes.length?sizes.join(' × ')+' ':''}matrix${element?' · '+schemaLabel(element):''}`;}
  if(s.includes('Option'))return 'Optional value';
  if(s.includes('Tuple'))return 'Tuple';
  if(s.includes('IntegerInterval'))return 'Integer interval';
  const primitive=s.match(/(FloatingPoint|UnsignedInteger|SignedInteger)\(W(\d+)\)/);
  if(primitive)return ({FloatingPoint:'f',UnsignedInteger:'u',SignedInteger:'i'}[primitive[1]])+primitive[2];
  if(s.includes('Boolean'))return 'Boolean';if(s.includes('String'))return 'Text';
  return s.replace(/^Some\((.*)\)$/,'$1');
}
function primitive(value,key='') {
  if(value===null)return el('span','data-empty','—');
  if(typeof value==='boolean')return el('span',value?'data-positive':'data-neutral',value?'Yes':'No');
  if(typeof value==='number')return el('span','data-number',/bytes$/.test(key)?`${value.toLocaleString('en-US')} bytes`:String(value));
  const text=String(value);
  if(sourceKeys.test(key))return el('pre','data-code',text);
  try {const nested=JSON.parse(text.replace(/^Error:\s*/,''));if(nested && typeof nested === 'object')return renderHumanData(nested,{key,depth:1});} catch {}
  if(key==='schema'||key==='outputs'&&text.includes('Schema'))return el('span','data-type',schemaLabel(text));
  if(/sha256|hash|commit|revision|fingerprint/.test(key)&&/^[a-f0-9]{20,}$/i.test(text))return el('code','data-identifier',text);
  if(['status','outcome','execution','parsing','semantic_checking','activation'].includes(key))return el('span',/^(passed|completed|accepted|ready)/.test(text)?'data-positive':/^(failed|rejected|blocked)/.test(text)?'data-negative':'data-neutral',text.replace(/^./,s=>s.toUpperCase()));
  return el('span','data-text',readableValue(text));
}
function lazyDetails(title,build,open=false) {
  const details=el('details','data-disclosure');details.append(el('summary','',title));let loaded=false;
  const populate=()=>{if(!loaded&&details.open){loaded=true;details.append(build());}};
  details.addEventListener('toggle',populate);if(open){details.open=true;populate();}return details;
}
function itemName(item,index,key) {
  if(item && typeof item==='object') {
    const title=item.title||item.name||item.case||item.claim;
    if(typeof title==='string')return title;
    if(item.turn!==undefined)return `Turn ${item.turn}`;
    if(item.operation!==undefined)return String(item.operation);
    if(item.id!==undefined)return String(item.id);
  }
  return `${key==='values'?'Output':'Item'} ${index+1}`;
}
function arrayView(values,key,depth) {
  if(!values.length)return el('span','data-empty','0 items');
  const container=el('div','data-array');
  const append=(slice,start=0)=>{
    if(slice.every(v=>v===null||typeof v!=='object')) {
      const list=el('ol','data-list');list.start=start+1;
      for(const value of slice){const item=el('li');item.append(primitive(value,key));list.append(item);}return list;
    }
    const group=el('div');slice.forEach((item,index)=>{const title=itemName(item,start+index,key);group.append(lazyDetails(title,()=>renderHumanData(item,{key,depth:depth+1}),slice.length<=3&&depth<2));});return group;
  };
  container.append(append(values.slice(0,12)));
  if(values.length>12)container.append(lazyDetails(`${values.length-12} additional items`,()=>append(values.slice(12),12)));
  return container;
}
export function renderHumanData(value,{key='',depth=0}={}) {
  if(value===null||typeof value!=='object')return primitive(value,key);
  if(Array.isArray(value))return arrayView(value,key,depth);
  if(('scalar_text' in value || 'html' in value) && ('schema' in value || 'kind' in value)) {
    const box=el('div','data-value');box.append(el('div','data-type',value.kind||schemaLabel(value.schema||'')),renderMechValue(value));return box;
  }
  const definition=el('dl','data-fields');
  const documentOnly=value.diagnostics?.some?.(d=>d.code==='source-semantics/empty-document');
  let entries=Object.entries(value).filter(([name])=>!(name==='outputs'&&Array.isArray(value.values)));
  for(const [name,original] of entries) {
    let item=original;
    if(name==='stages'&&documentOnly)item={...original,semantic_checking:'Document inspected; executable source required for program compilation'};
    if(name==='diagnostics'&&Array.isArray(item))item=item.map(d=>d.code==='source-semantics/empty-document'?{...d,severity:'Info'}:d);
    const dt=el('dt','',label(name)),dd=el('dd');
    if(name==='source' && value.stages) {dd.append(lazyDetails('Compiled source',()=>primitive(item,name)));}
    else if(item&&typeof item==='object'&&!Array.isArray(item)&&depth>=1&&!('scalar_text' in item))dd.append(lazyDetails(`${Object.keys(item).length} fields`,()=>renderHumanData(item,{key:name,depth:depth+1})));
    else dd.append(renderHumanData(item,{key:name,depth:depth+1}));
    definition.append(dt,dd);
  }
  return definition;
}
// Legacy display sinks retain their data for existing export and verification hooks.
// Their visible sibling presents the same record as labeled fields and tables.
const skipIds=new Set(['source','native-source','state-source','config','positions','document-highlight','document-gutter']);
const records=new WeakMap();
function updatePanel(pre) {
  if(pre.closest('.human-data-view,.mech-formatted-value,.document-preview') || skipIds.has(pre.id) || pre.classList.contains('data-code'))return;
  const text=pre.textContent.trim(), previous=records.get(pre);
  if(previous?.text===text){previous.view.hidden=pre.hidden;return;}
  let value;
  try {if(!/^[\[{]/.test(text))throw Error();value=JSON.parse(text);}catch {
    if(previous){previous.view.remove();records.delete(pre);pre.classList.remove('structured-source');pre.removeAttribute('aria-hidden');}return;
  }
  const view=previous?.view||el('div','human-data-view');view.dataset.for=pre.id||'record';view.hidden=pre.hidden;
  view.replaceChildren(renderHumanData(value));
  if(!view.isConnected)pre.after(view);
  pre.classList.add('structured-source');pre.setAttribute('aria-hidden','true');records.set(pre,{text,view});
}
function startPanels() {
  document.querySelectorAll('pre').forEach(updatePanel);
  const observer=new MutationObserver(changes=>{
    const pending=new Set();
    for(const change of changes) {
      const element=change.target.nodeType===1?change.target:change.target.parentElement;
      const pre=element?.closest('pre');if(pre)pending.add(pre);
      for(const node of change.addedNodes??[])if(node.nodeType===1&&!node.closest('.human-data-view,.mech-formatted-value,.document-preview')) {if(node.matches('pre'))pending.add(node);node.querySelectorAll('pre').forEach(pre=>pending.add(pre));}
    }
    pending.forEach(updatePanel);
  });
  observer.observe(document.body,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:['hidden']});
}
if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',startPanels,{once:true});else startPanels();
