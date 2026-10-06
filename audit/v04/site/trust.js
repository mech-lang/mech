import {fetchAuditWasm} from './wasm-transport.js';
import init, {inspectMechTypes} from './pkg/mech_wasm.js';
const $=id=>document.getElementById(id),pretty=value=>JSON.stringify(value,null,2);
const original='~x := 1.0\nnext := x * 2.0 + 3.0\nsafe! := next < 1000.0\nx = next\nx\n';
const cases=[
 {id:'original',name:'Original recurrence',source:original,expected:'5'},
 {id:'perturbed',name:'Changed source constants',source:original.replace('x * 2.0 + 3.0','x * 3.0 + 4.0'),expected:'7'},
 {id:'syntax',name:'Incomplete matrix',source:'~x := [1.0; 2.0',stage:'semantic_checking',status:'blocked by strict syntax validation'},
 {id:'type',name:'Incompatible multiplication',source:'~x := 1.0\nnext := x * "two"\nx = next\nx\n',stage:'semantic_checking',status:'rejected'},
 {id:'integrity',name:'Candidate 1021 at integrity boundary',source:original.replace('~x := 1.0','~x := 509.0'),stage:'execution',status:'rejected'}
];
let metadata,native;
for(const c of cases){const option=document.createElement('option');option.value=c.id;option.textContent=c.name;$('case').append(option);}
function choose(){$('result').textContent='Awaiting compilation of this source.';const c=cases.find(c=>c.id===$('case').value);$('source').value=c.source;$('expected').textContent=c.expected?`Independent expected first output: ${c.expected}.`:`Expected ${c.stage}: ${c.status}.`;}
function inspect(source){return JSON.parse(inspectMechTypes(source));}
function run(){if($('source').value!==cases.find(c=>c.id===$('case').value)?.source)$('expected').textContent='Custom source. Inspect the actual stage results.';const result=inspect($('source').value);$('result').textContent=pretty(result);return result;}
function checks(){const results=cases.map(c=>{const result=inspect(c.source);const passed=c.expected?result.values?.some(v=>v.scalar_text===c.expected):result.stages[c.stage]===c.status;return {id:c.id,passed:!!passed,expected:c.expected??`${c.stage}: ${c.status}`,result};});return {status:results.every(x=>x.passed)?'passed':'failed',environment:navigator.userAgent,metadata,scope:'canonical-source WASM single-candidate execution',results};}
$('source').oninput=()=>{$('result').textContent='Source changed. Compile to inspect the current candidate.';$('expected').textContent='Custom source. Inspect the actual stage results.';};$('case').onchange=choose;$('run').onclick=run;$('checks').onclick=()=>{$('check-results').hidden=false;$('check-results').textContent=pretty(checks());};choose();
try{
 const nativeResponse=await fetch('../evidence/trust-native.json');if(nativeResponse.ok){native=await nativeResponse.json();$('native-status').textContent=pretty({status:native.status,command:native.command,scope:native.scope});$('native-source').textContent=native.source;$('native-checks').textContent=pretty(native.checks);$('native-provenance').textContent=pretty(native);for(const turn of native.turns??[]){const row=document.createElement('tr');for(const value of [turn.turn,turn.expected,turn.source_artifact,turn.decoded_bytecode,turn.epoch]){const cell=document.createElement('td');cell.textContent=String(value);row.append(cell);}$('turns').append(row);}}else{$('native-status').textContent='Native execution evidence pending.';$('native-source').textContent=original;}
 const bytes=await (await fetchAuditWasm()).arrayBuffer();await init({module_or_path:bytes});
 metadata=await (await fetch('./artifact.json')).json();const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))).map(x=>x.toString(16).padStart(2,'0')).join('');if(hash!==metadata.artifacts['pkg/mech_wasm_bg.wasm'].sha256)throw new Error('Loaded WASM hash differs from recorded artifact.');metadata={...metadata,loaded_wasm_sha256:hash,browser:navigator.userAgent};$('artifact').textContent=pretty(metadata);
 for(const id of ['run','checks'])$(id).disabled=false;run();$('boot').textContent='Recorded Mech WASM loaded. Browser controls compile and execute the displayed source.';window.mechTrustChecks=checks;window.mechTrustNative=native;window.mechTrustReady=true;
}catch(error){$('boot').textContent=`Blocked: ${error.stack??error}`;window.mechTrustError=String(error);}
