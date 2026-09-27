// Compare real rendered source/output outlines with the blog's neutral frame.
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';

const port=process.env.IROS_CDP_PORT||9227;
const url=process.env.IROS_URL||'http://127.0.0.1:8768/index.html';
const output=process.env.IROS_REPORT||'/private/tmp/iros-code-borders.json';
const target=await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'}).then(r=>r.json());
const socket=new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve,reject)=>{socket.onopen=resolve;socket.onerror=reject;});
let sequence=0;
const pending=new Map(),exceptions=[];
socket.onmessage=({data})=>{
  const message=JSON.parse(data);
  if(message.id){const p=pending.get(message.id);pending.delete(message.id);message.error?p.reject(message.error):p.resolve(message.result);}
  if(message.method==='Runtime.exceptionThrown')exceptions.push(message.params.exceptionDetails);
};
const send=(method,params={})=>new Promise((resolve,reject)=>{const id=++sequence;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method,params}));});
const evaluate=async expression=>{
  const result=await send('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});
  if(result.exceptionDetails)throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
};
const report={url,viewports:[],exceptions};
const border='1px solid rgb(77, 82, 87)';
function outlined(styles,label){
  for(const side of ['borderTop','borderRight','borderBottom','borderLeft'])
    assert.equal(styles[side],border,`${label} ${side}`);
}
try{
  await send('Page.enable');await send('Runtime.enable');
  await send('Page.navigate',{url:url+'?border-test='+Date.now()});
  const start=Date.now();
  while(!await evaluate('!!document.getElementById("run")&&!document.getElementById("run").disabled')){
    assert(Date.now()-start<60000,'workshop runtime starts');
    await new Promise(resolve=>setTimeout(resolve,150));
  }
  await evaluate(`document.fonts.ready.then(()=>true)`);
  await evaluate(`(()=>{if(document.querySelector('[data-mech-repl-host]').dataset.mechConsoleOpen!=='false')document.dispatchEvent(new KeyboardEvent('keydown',{key:String.fromCharCode(96),bubbles:true}));})()`);
  for(const width of [1440,390]){
    await send('Emulation.setDeviceMetricsOverride',{width,height:1050,deviceScaleFactor:1,mobile:false});
    await new Promise(resolve=>setTimeout(resolve,200));
    const measured=await evaluate(`(()=>{
      const style=e=>{const s=getComputedStyle(e);return Object.fromEntries(['borderTop','borderRight','borderBottom','borderLeft','borderTopLeftRadius','borderBottomLeftRadius','display'].map(k=>[k,s[k]]));};
      const blocks=[...document.querySelectorAll('.mech-fenced-mech-block')].map(e=>{
        const code=e.querySelector(':scope > .mech-code-block'),output=e.querySelector(':scope > .mech-block-output');
        return {id:e.id,code:style(code),output:output?style(output):null,visible:!!output&&!output.hidden&&output.childNodes.length>0&&getComputedStyle(output).display!=='none'};
      });
      const rust=[...document.querySelectorAll('pre.mech-code-block[data-language="rust"]')].map(style);
      const paired=document.querySelector('.mech-fenced-mech-block:has(> .mech-block-output:not([hidden]):not(:empty))');
      const body=paired.querySelector(':scope > .mech-block-output'),code=paired.querySelector(':scope > .mech-code-block');
      const content=[...body.childNodes];body.replaceChildren();const empty={code:style(code),output:style(body)};
      body.remove();const missing={code:style(code)};paired.append(body);body.append(...content);
      return {blocks,rust,empty,missing,restored:style(code),application:style(document.querySelector('.mech-output-region-application')),overflow:document.documentElement.scrollWidth>innerWidth};
    })()`);
    assert.equal(measured.blocks.length,12,'all native Mech fences are checked');
    assert(measured.blocks.some(b=>b.visible),'ordinary document outputs are visible');
    assert(measured.blocks.some(b=>!b.visible),'source-only listings are present');
    for(const block of measured.blocks){
      if(block.visible){
        outlined(block.output,'output '+block.id);
        for(const side of ['borderTop','borderRight','borderLeft'])assert.equal(block.code[side],border,'joined code '+side);
        assert.equal(block.code.borderBottom,'0px solid rgb(77, 82, 87)','one separator, not a doubled border');
        assert.equal(block.code.borderBottomLeftRadius,'0px');
        assert.equal(block.output.borderBottomLeftRadius,'10px');
      }else{outlined(block.code,'standalone '+block.id);assert.equal(block.code.borderBottomLeftRadius,'10px');}
    }
    assert.equal(measured.rust.length,3);for(const code of measured.rust)outlined(code,'Rust code');
    outlined(measured.empty.code,'empty output closes code');
    assert.equal(measured.empty.output.display,'none','empty output does not leave an unframed blank panel');
    outlined(measured.missing.code,'absent output closes code');
    assert.equal(measured.restored.borderBottom,'0px solid rgb(77, 82, 87)','output restoration rejoins code');
    outlined(measured.application,'application output');
    assert.equal(measured.overflow,false,'no page overflow at '+width);
    report.viewports.push({width,...measured});
    for(const [label,selector] of [
      ['standalone','[data-workshop-reference-listing]:nth-of-type(1)'],
      ['joined','.mech-fenced-mech-block:has(> .mech-block-output:not([hidden]):not(:empty))'],
    ]){
      const chosen=label==='standalone'?'[data-workshop-reference-listing]':selector;
      await evaluate(`document.querySelector(${JSON.stringify(chosen)}).scrollIntoView({block:'center',behavior:'instant'})`);
      await new Promise(resolve=>setTimeout(resolve,200));
      const shot=await send('Page.captureScreenshot',{format:'png'});
      writeFileSync(output.replace(/\.json$/,`-${width}-${label}.png`),Buffer.from(shot.data,'base64'));
    }
  }
  assert.equal(exceptions.length,0);
  report.status='passed';console.log('PASS: all Mech/Rust source frames, visible/hidden/empty/absent outputs, neutral application border, desktop and mobile.');
}catch(error){report.status='failed';report.error=String(error);process.exitCode=1;console.error(error);}
finally{writeFileSync(output,JSON.stringify(report,null,2)+'\n');await send('Page.close');socket.close();}
