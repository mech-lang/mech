import {fetchAuditWasm} from './wasm-transport.js';
import init, {WasmTypeInspector, TypePublicationSession} from './pkg/mech_wasm.js';
const $=id=>document.getElementById(id);
const pretty=value=>JSON.stringify(value,null,2);
const expr=value=>`answer := ${value}\nanswer\n`;
export const cases=[
 {id:'lower',name:'Interval lower bound',source:expr('1⟨u8:1..10⟩'),expected:'1'},
 {id:'last',name:'Interval last admitted value',source:expr('9⟨u8:1..10⟩'),expected:'9'},
 {id:'exclusive',name:'Excluded upper bound',source:expr('10⟨u8:1..10⟩'),error:'source-semantics/integer-interval-violation'},
 {id:'below',name:'Below lower bound',source:expr('0⟨u8:1..10⟩'),error:'source-semantics/integer-interval-violation'},
 {id:'inclusive',name:'Included upper bound',source:expr('10⟨u8:1..=10⟩'),expected:'10'},
 {id:'interval-arithmetic',name:'Interval arithmetic admission',source:'answer := 1⟨u8:1..10⟩ + 2⟨u8:1..10⟩\nanswer\n',error:'source-semantics/non-numeric-arithmetic-kind'},
 {id:'option',name:'Present option',source:'answer<u8?> := 1\nanswer\n',schema:'Option',expectedValue:'Option(Some(U8(1)))'},
 {id:'absent',name:'Absent option',source:'answer<u8?> := _\nanswer\n',schema:'Option',expectedValue:'Option(None)'},
 {id:'nested',name:'Optional record with an optional field',source:'answer<{missing<u8?>}?> := {missing:_}\nanswer\n',schema:'Option',expectedValue:'Option(Some(Record(RecordValue { fields: [Option(None)] })))'},
 {id:'enum',name:'Standalone enum',source:'<color> := :red | :green | :blue\nmy-color<color> := :red\n',schema:'Enum',expectedValue:'Enum(EnumValue { ordinal: 0, payload: None })'},
 {id:'enum-variant',name:'Unknown enum variant',source:'<color> := :red | :green | :blue\nmy-color<color> := :yellow\n',error:'source-semantics/unknown-enum-variant'},
 {id:'inference',name:'Peer-constrained external input',source:expr('signal + 1'),input:true},
 {id:'shape',name:'Matrix shape',source:expr('[1 2; 3 4]'),schema:'Matrix',elements:[1,2,3,4]},
 {id:'named',name:'Named argument order',source:expr('math/sub(right: 3, left: 10)'),expected:'7'},
 {id:'wide',name:'Exact u128 maximum',source:expr('340282366920938463463374607431768211455u128'),expected:'340282366920938463463374607431768211455'},
];
let inspector,instance,history=[],metadata={};
for(const item of cases){const option=document.createElement('option');option.value=item.id;option.textContent=item.name;$('example').append(option);}
function invalidate(){ $('diagnostics').replaceChildren();$('stages').textContent='Source changed. Compile to inspect the current candidate.';$('values').textContent='Awaiting compilation.';$('result').textContent='';}
function loadCase(){invalidate();const item=cases.find(x=>x.id===$('example').value);$('source').value=item.source;$('expectation').textContent=item.error?`Expected rejection: ${item.error}`:item.expected?`Independent expected scalar: ${item.expected}`:item.input?'Expected semantic inference; execution awaits external inputs.':`Expected checked schema containing ${item.schema}.`;}
function render(result){
 $('result').textContent=pretty(result);$('values').textContent=result.values||result.outputs||result.inputs?pretty(result.values??{outputs:result.outputs,inputs:result.inputs}):'Candidate has no published values.';
 $('stages').replaceChildren(...Object.entries(result.stages).map(([phase,status])=>{const p=document.createElement('p');p.textContent=`${phase}: ${status}`;return p;}));
 $('diagnostics').replaceChildren(...(result.diagnostics??[]).map(d=>{const box=document.createElement('div');box.className='warning';const button=document.createElement('button');button.textContent=`${d.code??d.phase??'diagnostic'}: ${d.message??pretty(d)}`;button.onclick=()=>{if(!d.range)return;const bytes=new TextEncoder().encode($('source').value);const decode=n=>new TextDecoder().decode(bytes.slice(0,n)).length;$('source').focus();$('source').setSelectionRange(decode(d.range[0]),decode(d.range[1]));};box.append(button);return box;}));
 return result;
}
function run(){if($('source').value!==cases.find(x=>x.id===$('example').value)?.source)$('expectation').textContent='Custom source. Inspect attained stages and diagnostic results.';return render(JSON.parse(inspector.inspect($('source').value)));}
function restart(){instance?.free();instance=new TypePublicationSession();history=[];$('state-source').textContent=instance.source();$('snapshot').textContent=pretty(JSON.parse(instance.snapshot()));$('history').textContent='Fresh instance accepted its initial value 2.';}
function update(value=$('input').value){const event=JSON.parse(instance.update(value));history.push(event);$('snapshot').textContent=pretty(event.after);$('history').textContent=pretty(history);return event;}
export function runTypeChecks(){
 const results=cases.map(item=>{const result=JSON.parse(inspector.inspect(item.source));let passed;if(item.error)passed=result.diagnostics?.some(d=>d.code===item.error&&Array.isArray(d.range))&&result.product_diagnostic?.semantic?.code===item.error&&typeof result.product_diagnostic.presentation_range==='string';else if(item.expected)passed=result.values?.some(v=>v.scalar_text===item.expected);else if(item.input)passed=result.inputs?.some(x=>x.name==='signal'&&x.schema.includes('FloatingPoint(W64)'))&&result.stages.semantic_checking==='completed';else passed=(JSON.stringify(result.outputs)??'').includes(item.schema)&&result.stages.execution==='completed';if(item.expectedValue)passed=passed&&result.values?.some(v=>v.value===item.expectedValue);if(item.elements){const view=new DataView(new ArrayBuffer(8));const bits=item.elements.map(n=>{view.setFloat64(0,n);return `F64Bits(${view.getBigUint64(0)})`;});const expected=`Matrix(MatrixValue { elements: F64([${bits.join(', ')}]) })`;passed=passed&&result.values?.some(v=>v.value===expected&&v.schema.includes('dimensions: [Constant(2), Constant(2)]'));}return {id:item.id,passed:!!passed,expectation:item.expected??item.error??item.schema??'peer-constrained signal',result};});
 const session=new TypePublicationSession();const sequence=['9','10','3'].map(value=>JSON.parse(session.update(value)));session.free();
 const restarts=Array.from({length:3},()=>{const fresh=new TypePublicationSession();const start=JSON.parse(fresh.snapshot());const accepted=JSON.parse(fresh.update('9'));fresh.free();return {initial:start.output.scalar_text,accepted:accepted.after.output.scalar_text,passed:start.output.scalar_text==='2'&&accepted.after.output.scalar_text==='9'};});
 const publication=sequence[0].outcome==='accepted'&&sequence[1].outcome==='rejected'&&sequence[1].accepted_state_unchanged===true&&sequence[2].outcome==='accepted'&&sequence[2].after.output.scalar_text==='3';
 return {status:results.every(x=>x.passed)&&publication&&restarts.every(x=>x.passed)?'passed':'failed',environment:navigator.userAgent,metadata,results,publication:{passed:publication,sequence,restarts}};
}
$('source').oninput=()=>{invalidate();$('expectation').textContent='Custom source. Compile to inspect its stages and diagnostics.';};$('example').onchange=loadCase;$('run').onclick=run;$('restart').onclick=restart;$('update').onclick=()=>update();$('sequence').onclick=()=>{for(const value of ['9','10','3'])update(value);};$('checks').onclick=()=>{$('check-result').hidden=false;$('check-result').textContent=pretty(runTypeChecks());};loadCase();
try{
 const wasmResponse=await fetchAuditWasm();if(!wasmResponse.ok)throw new Error(`WASM download ${wasmResponse.status}`);const wasmBytes=await wasmResponse.arrayBuffer();
 await init({module_or_path:wasmBytes});
 inspector=new WasmTypeInspector();
 const loadedHash=Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256",wasmBytes))).map(x=>x.toString(16).padStart(2,"0")).join("");
 const response=await fetch('./artifact.json');if(response.ok)metadata=await response.json();
 else metadata={source_commit:'c4777b7015fe8ff47fdfa48d18606c49aace7d97',build:'audit adapters and recorded patch; consult build evidence',artifact:'./pkg/mech_wasm_bg.wasm'};
 const expectedHash=metadata.artifacts?.['pkg/mech_wasm_bg.wasm']?.sha256;if(expectedHash&&expectedHash!==loadedHash)throw new Error(`WASM identity mismatch: expected ${expectedHash}, loaded ${loadedHash}`);
 metadata={...metadata,loaded_wasm_sha256:loadedHash,loaded_wasm_bytes:wasmBytes.byteLength,browser:navigator.userAgent};
 $('identity').textContent=pretty(metadata);restart();run();for(const id of ['run','checks','update','sequence','restart'])$(id).disabled=false;$('boot').textContent='Mech WebAssembly loaded; canonical source and resident publication APIs are available.';
 window.mechTypeChecks=runTypeChecks;window.mechTypeInspect=source=>JSON.parse(inspector.inspect(source));window.mechTypesReady=true;
}catch(error){$('boot').textContent=`Blocked: ${error.stack??error}`;window.mechTypesError=String(error);}
