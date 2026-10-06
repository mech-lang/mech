import init, {I64PublicationSession, inventorySource} from './pkg/mech_wasm.js';
const $=id=>document.getElementById(id);
const pretty=value=>JSON.stringify(value,null,2);
const hash=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))).map(x=>x.toString(16).padStart(2,'0')).join('');
const inputNames='["arrivals","demand"]';
const output=snapshot=>BigInt(snapshot.outputs[0].scalar_text);
let session,source,fixture,metadata={},history=[],accepted=100n,ledger=100n,generation=0;
function turn(target,arrivals,demand){return JSON.parse(target.update(pretty({arrivals:String(arrivals),demand:String(demand)})));}
function fresh(){const target=new I64PublicationSession(source,inputNames);const initial=turn(target,0,0);if(initial.outcome!=='accepted'||output(initial.after)!==100n){target.free();throw new Error(`Initialization failed: ${pretty(initial)}`);}return target;}
function checkReceipt(receipt,previous,arrivals,demand){
 const candidate=previous+arrivals-demand;
 const valid=arrivals>=0n&&demand>=0n&&candidate>=0n&&candidate<=1000000n;
 const expected=valid?candidate:previous;
 const actual=output(receipt.after);
 const unchanged=receipt.before.epoch===receipt.after.epoch&&receipt.before.state_hash===receipt.after.state_hash&&pretty(receipt.before.outputs)===pretty(receipt.after.outputs);
 return {candidate:String(candidate),expected_stock:String(expected),actual_stock:String(actual),expected_outcome:valid?'accepted':'rejected',passed:receipt.outcome===(valid?'accepted':'rejected')&&actual===expected&&(valid||unchanged&&receipt.accepted_state_unchanged),rejected_publication_unchanged:valid?null:unchanged};
}
function graph(){
 const ctx=$('plot').getContext('2d'),w=ctx.canvas.width,h=ctx.canvas.height,left=58,right=w-24,top=22,bottom=h-38;
 ctx.clearRect(0,0,w,h);ctx.fillStyle='#fff';ctx.fillRect(0,0,w,h);
 const records=[{stock:100,outcome:'initial'},...history.map(x=>({stock:Number(x.check.actual_stock),outcome:x.receipt.outcome}))];
 const ymax=Math.max(110,...records.map(x=>x.stock))*1.08;
 ctx.font='12px system-ui';ctx.textAlign='right';
 for(let i=0;i<=4;i++){const y=bottom-(bottom-top)*i/4;ctx.strokeStyle='#e3ebed';ctx.beginPath();ctx.moveTo(left,y);ctx.lineTo(right,y);ctx.stroke();ctx.fillStyle='#53666d';ctx.fillText(Math.round(ymax*i/4).toLocaleString(),left-8,y+4);}
 const x=i=>left+(right-left)*i/Math.max(records.length-1,1),y=value=>bottom-(bottom-top)*value/ymax;
 ctx.strokeStyle='#216d73';ctx.lineWidth=2;ctx.beginPath();records.forEach((r,i)=>i?ctx.lineTo(x(i),y(r.stock)):ctx.moveTo(x(i),y(r.stock)));ctx.stroke();
 records.forEach((r,i)=>{ctx.fillStyle=r.outcome==='rejected'?'#a24338':'#216d73';ctx.beginPath();ctx.arc(x(i),y(r.stock),r.outcome==='rejected'?5:3,0,2*Math.PI);ctx.fill();});
 ctx.fillStyle='#53666d';ctx.textAlign='center';ctx.fillText(`Attempt 0 → ${history.length}`,w/2,h-10);ctx.textAlign='left';ctx.fillText('Stock / units',left,14);
}
function render(snapshot){
 $('stock').textContent=output(snapshot).toLocaleString();$('epoch').textContent=snapshot.epoch.replace(/^InstanceEpoch\((.*)\)$/,'$1');$('rejections').textContent=history.filter(x=>x.receipt.outcome==='rejected').length;$('residual').textContent=String(output(snapshot)-ledger);
 $('snapshot').textContent=pretty(snapshot);$('session-scope').textContent=`Browser session ${generation}; ${history.length} attempted turns after initialization. Every displayed receipt belongs to this activated object.`;
 $('history').replaceChildren(...history.map((row,i)=>{const tr=document.createElement('tr');const values=[i+1,row.arrivals,row.demand,row.check.candidate,row.receipt.outcome,row.check.actual_stock,row.receipt.after.epoch,row.check.passed?'passed':'failed'];for(const value of values){const td=document.createElement('td');td.textContent=value;tr.append(td);}tr.onclick=()=>{$('receipt').textContent=pretty(row);$('receipt').parentElement.open=true;};return tr;}));graph();
}
function restart(){session?.free();session=fresh();generation++;history=[];accepted=ledger=100n;$('receipt').textContent='Select a turn to inspect its input, candidate, diagnostics and published state.';$('input-status').textContent='Fresh resident instance initialized with a zero-input turn.';render(JSON.parse(session.snapshot()));}
function submit(arrivalText=$('arrivals').value,demandText=$('demand').value){
 if(history.length>=200){$('input-status').textContent='This view retains 200 attempts. Export evidence, then restart to create another session.';return;}
 let arrivals,demand;try{if(!/^-?\d+$/.test(arrivalText)||!/^-?\d+$/.test(demandText))throw new Error('Enter two decimal integers.');arrivals=BigInt(arrivalText);demand=BigInt(demandText);if([arrivals,demand].some(x=>x< -1000000n||x>1000000n))throw new Error('Interactive inputs range from −1,000,000 to 1,000,000.');}catch(error){$('input-status').textContent=error.message;return;}
 const receipt=turn(session,arrivals,demand),check=checkReceipt(receipt,accepted,arrivals,demand);
 if(receipt.outcome==='accepted')ledger+=arrivals-demand;
 accepted=output(receipt.after);const row={arrivals:String(arrivals),demand:String(demand),check,receipt};history.push(row);render(receipt.after);$('receipt').textContent=pretty(row);$('input-status').textContent=`${receipt.outcome}; independent check ${check.passed?'passed':'failed'}. ${receipt.message??''}`;
 return row;
}
export function runInventoryChecks(){
 const target=fresh();let previous=100n;const records=[];
 for(const item of fixture.cases){const receipt=turn(target,item.arrivals,item.demand),check=checkReceipt(receipt,previous,BigInt(item.arrivals),BigInt(item.demand));check.literal_fixture_passed=check.actual_stock===String(item.expected_stock)&&receipt.outcome===item.expected_outcome;records.push({inputs:{arrivals:item.arrivals,demand:item.demand},check,receipt});previous=output(receipt.after);}
 const invalids=['{"arrivals":"0"}','{"arrivals":"0","demand":"9223372036854775808"}','{"arrivals":"0","demand":1}'].map(input=>JSON.parse(target.update(input)));
 target.free();
 const generic=new I64PublicationSession('~total⟨i64⟩ := 7\ntotal = total + change\ntotal\n','["change"]');
 const alternative=['3','-4'].map(change=>JSON.parse(generic.update(pretty({change}))));generic.free();
 const restarts=Array.from({length:3},()=>{const object=fresh();const value=output(JSON.parse(object.snapshot()));object.free();return String(value);});
 const passed=records.every(x=>x.check.passed&&x.check.literal_fixture_passed)&&invalids.every(x=>x.outcome==='rejected'&&x.accepted_state_unchanged)&&output(alternative[0].after)===10n&&output(alternative[1].after)===6n&&restarts.every(x=>x==='100');
 return {status:passed?'passed':'failed',metadata,source,fixture,records,input_admission:invalids,generic_source:alternative,restarts,scope:'WASM-hosted resident CPU; one activated object for the nine-turn inventory sequence'};
}
$('submit').onclick=()=>submit();$('restart').onclick=restart;$('sequence').onclick=()=>{restart();for(const row of fixture.cases)submit(String(row.arrivals),String(row.demand));};$('checks').onclick=()=>{$('check-result').hidden=false;$('check-result').textContent=pretty(runInventoryChecks());};
$('export').onclick=()=>{const bytes=new Blob([pretty({metadata,source,generation,history,snapshot:JSON.parse(session.snapshot())})],{type:'application/json'});const url=URL.createObjectURL(bytes);const anchor=document.createElement('a');anchor.href=url;anchor.download='mech-inventory-evidence.json';anchor.click();setTimeout(()=>URL.revokeObjectURL(url),0);};
window.addEventListener('pagehide',()=>{session?.free();session=null;});window.addEventListener('pageshow',event=>{if(event.persisted&&source)restart();});
try{
 const responses=await Promise.all([fetch('./pkg/mech_wasm_bg.wasm'),fetch('./artifact.json'),fetch('./fixtures/inventory.mec'),fetch('./fixtures/inventory.json')]);for(const response of responses)if(!response.ok)throw new Error(`Resource ${response.url}: ${response.status}`);
 const bytes=await responses[0].arrayBuffer();metadata=await responses[1].json();const fixtureSource=await responses[2].text(),fixtureText=await responses[3].text();fixture=JSON.parse(fixtureText);await init({module_or_path:bytes});
 const loadedHash=await hash(bytes);if(loadedHash!==metadata.artifacts['pkg/mech_wasm_bg.wasm'].sha256)throw new Error('Loaded WASM hash differs from the artifact manifest.');source=inventorySource();if(source!==fixtureSource)throw new Error('Compiled source fixture differs from the displayed fixture.');
 metadata={...metadata,loaded_wasm_sha256:loadedHash,loaded_wasm_bytes:bytes.byteLength,source_sha256:await hash(new TextEncoder().encode(source)),fixture_sha256:await hash(new TextEncoder().encode(fixtureText)),browser:navigator.userAgent};
 $('identity').textContent=pretty(metadata);$('source').textContent=source;restart();$('stages').textContent='Parsing: completed · semantic checking: completed · bytecode roundtrip: completed · activation: completed · initial publication: completed. Each later turn records its own publication result.';
 for(const id of ['submit','sequence','restart','checks','export'])$(id).disabled=false;$('boot').textContent='Resident Mech instance ready. All displayed inventory values come from copied published outputs.';window.mechInventoryChecks=runInventoryChecks;window.mechInventoryHistory=()=>history;window.mechInventoryReady=true;
}catch(error){$('boot').textContent=`Blocked: ${error.stack??error}`;window.mechInventoryError=String(error);}
