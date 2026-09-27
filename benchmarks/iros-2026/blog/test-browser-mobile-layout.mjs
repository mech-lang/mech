// Real browser coverage for compact Contents, tables, and console layout.
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
const port=process.env.IROS_CDP_PORT||9227;
const url=process.env.IROS_URL||'http://127.0.0.1:8768/index.html';
const output=process.env.IROS_REPORT||'/private/tmp/iros-mobile-layout.json';
const expectedGap=Number(process.env.IROS_EXPECT_TOC_GAP??24);
const checks=new Set((process.env.IROS_CHECKS||'toc,tables,panes').split(','));
const target=await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'}).then(r=>r.json());
const socket=new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve,reject)=>{socket.onopen=resolve;socket.onerror=reject;});
let id=0;const pending=new Map(),exceptions=[];
socket.onmessage=({data})=>{const r=JSON.parse(data);if(r.id){const p=pending.get(r.id);pending.delete(r.id);r.error?p.reject(r.error):p.resolve(r.result);}if(r.method==='Runtime.exceptionThrown')exceptions.push(r.params.exceptionDetails);};
const send=(method,params={})=>new Promise((resolve,reject)=>{const n=++id;pending.set(n,{resolve,reject});socket.send(JSON.stringify({id:n,method,params}));});
const evaluate=async expression=>{const r=await send('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));return r.result.value;};
async function until(expression,message){const start=Date.now();while(!await evaluate(expression)){assert(Date.now()-start<60000,message);await new Promise(resolve=>setTimeout(resolve,100));}}
async function click(selector,{scroll=true}={}){
  if(scroll)await evaluate(`document.querySelector(${JSON.stringify(selector)}).scrollIntoView({block:'center',behavior:'instant'})`);
  await new Promise(resolve=>setTimeout(resolve,150));
  const point=await evaluate(`(()=>{const e=document.querySelector(${JSON.stringify(selector)}),r=e.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2,hit:e.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))};})()`);
  assert(point.hit,'pointer reaches '+selector);
  const {x,y}=point;
  await send('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'left',buttons:1,clickCount:1});
  await send('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'left',buttons:0,clickCount:1});
}
const report={url,expectedGap,startedAt:new Date().toISOString(),cases:[],exceptions};
try{
  await send('Page.enable');await send('Runtime.enable');
  report.browser=await send('Browser.getVersion');
  await send('Page.navigate',{url:url+'?mobile-layout-test='+Date.now()});
  await until('!!document.getElementById("run")&&!document.getElementById("run").disabled','runtime ready');
  await evaluate('document.fonts.ready.then(()=>true)');
  for(const [width,panelOpen] of checks.has('toc')?[[320,false],[390,false],[430,false],[900,false],[1400,true],[1920,false]]:[]){
    await send('Emulation.setDeviceMetricsOverride',{width,height:1050,deviceScaleFactor:1,mobile:false});
    await evaluate(`(()=>{const root=document.querySelector('[data-mech-repl-host]');if((root.dataset.mechConsoleOpen!=='false')!==${panelOpen})document.dispatchEvent(new KeyboardEvent('keydown',{key:String.fromCharCode(96),bubbles:true}));})()`);
    await new Promise(resolve=>setTimeout(resolve,250));
    const compact=width!==1920;
    await evaluate(`document.querySelector(${JSON.stringify(compact?'.mech-toc-toggle':'.main-content h2')}).scrollIntoView({block:'center',behavior:'instant'})`);
    const geometry=await evaluate(`(()=>{
      const toggle=document.querySelector('.mech-toc-toggle'),heading=document.querySelector('.main-content h2'),layout=document.querySelector('.article-layout');
      const t=toggle.getBoundingClientRect(),h=heading.getBoundingClientRect(),p=getComputedStyle(heading,'::before');
      return {toggleDisplay:getComputedStyle(toggle).display,layoutDisplay:getComputedStyle(layout).display,gap:h.top-t.bottom,headingTop:h.top,toggleBottom:t.bottom,
        eyebrow:{content:p.content,display:p.display,lineHeight:p.lineHeight},tocDisplay:getComputedStyle(document.querySelector('.toc')).display,overflow:document.documentElement.scrollWidth>innerWidth};
    })()`);
    assert.equal(geometry.overflow,false,'no page overflow at '+width);
    if(!compact){
      assert.equal(geometry.toggleDisplay,'none','wide desktop retains sidebar');
      assert.equal(geometry.layoutDisplay,'grid','wide desktop retains two-column grid');
      assert.notEqual(geometry.tocDisplay,'none');
    }else{
      assert(['flex','inline-flex'].includes(geometry.toggleDisplay),'compact Contents control visible');
      assert(Math.abs(geometry.gap-expectedGap)<1,`${width}px Contents-to-section gap is ${geometry.gap}, expected ${expectedGap}`);
      assert.equal(geometry.eyebrow.display,'block','section eyebrow remains in heading flow');
      const screenshot=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
      writeFileSync(output.replace(/\.json$/,`-${width}-closed.png`),Buffer.from(screenshot.data,'base64'));
      await click('.mech-toc-toggle');
      assert(await evaluate(`document.querySelector('.mech-toc-toggle').getAttribute('aria-expanded')==='true'&&getComputedStyle(document.querySelector('.main-content')).display==='none'&&getComputedStyle(document.querySelector('.toc')).display!=='none'`),'opening Contents shows links instead of article');
      assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'),'open Contents does not overflow');
      const expanded=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
      writeFileSync(output.replace(/\.json$/,`-${width}-open.png`),Buffer.from(expanded.data,'base64'));
      await click('.mech-toc-toggle');
      assert.equal(await evaluate(`document.querySelector('.mech-toc-toggle').getAttribute('aria-expanded')`),'false','toggle closes Contents');
      await click('.mech-toc-toggle');
      await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
      await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
      assert(await evaluate(`document.querySelector('.mech-toc-toggle').getAttribute('aria-expanded')==='false'&&document.activeElement===document.querySelector('.mech-toc-toggle')`),'Escape closes Contents and returns focus');
      await click('.mech-toc-toggle');
      const navigation=await evaluate(`(()=>{const link=[...document.querySelectorAll('.toc a')].find(e=>e.textContent.trim()==='Reactive');return {href:link.getAttribute('href'),target:link.hash.slice(1)};})()`);
      await click(`.toc a[href="${navigation.href}"]`);
      await until(`document.querySelector('.mech-toc-toggle').getAttribute('aria-expanded')==='false'`,'navigation closes Contents');
      await until(`(()=>{const r=document.getElementById(${JSON.stringify(navigation.target)}).getBoundingClientRect();return r.top>=-1&&r.top<200;})()`,'smooth Contents navigation reaches selected section');
      navigation.position=await evaluate(`(()=>{const r=document.getElementById(${JSON.stringify(navigation.target)}).getBoundingClientRect();return {top:r.top,bottom:r.bottom,visible:getComputedStyle(document.querySelector('.main-content')).display!=='none'};})()`);
      assert(navigation.position.visible&&navigation.position.top>=-1&&navigation.position.top<200,'Contents navigation reveals the selected section');
      geometry.navigation=navigation;
    }
    report.cases.push({width,panelOpen,...geometry});
  }
  report.tables=[];
  for(const width of checks.has('tables')?[320,390,430,600]:[]){
    await send('Emulation.setDeviceMetricsOverride',{width,height:1050,deviceScaleFactor:1,mobile:false});
    await evaluate(`(()=>{if(document.querySelector('[data-mech-repl-host]').dataset.mechConsoleOpen!=='false')document.dispatchEvent(new KeyboardEvent('keydown',{key:String.fromCharCode(96),bubbles:true}));})()`);
    await new Promise(resolve=>setTimeout(resolve,200));
    const tables=await evaluate(`(()=>{
      const lines=cell=>{
        const walker=document.createTreeWalker(cell,NodeFilter.SHOW_TEXT),rects=[],brokenWords=[];
        while(walker.nextNode())if(walker.currentNode.textContent.trim()){
          const range=document.createRange();range.selectNodeContents(walker.currentNode);
          rects.push(...[...range.getClientRects()].filter(r=>r.width>0&&r.height>0));
          for(const word of walker.currentNode.textContent.matchAll(/[\\p{L}\\p{N}]+/gu)){
            if(word[0].length<3)continue;
            range.setStart(walker.currentNode,word.index);range.setEnd(walker.currentNode,word.index+word[0].length);
            if(new Set([...range.getClientRects()].map(r=>Math.round(r.top))).size>1)brokenWords.push(word[0]);
          }
        }
        return {count:new Set(rects.map(r=>Math.round(r.top))).size,brokenWords,rects:rects.map(r=>({left:r.left,right:r.right,top:r.top,bottom:r.bottom}))};
      };
      return [...document.querySelectorAll('.main-content table')].map(table=>{
        const box=table.getBoundingClientRect();
        const savedLeft=table.scrollLeft;table.scrollTo({left:table.scrollWidth,behavior:'instant'});const maximumScrollLeft=table.scrollLeft;table.scrollTo({left:savedLeft,behavior:'instant'});
        return {sourceSize:table.classList.contains('workshop-source-size'),dylib:table.textContent.includes('Checked dynamic library'),width:box.width,scrollWidth:table.scrollWidth,clientWidth:table.clientWidth,maximumScrollLeft,
          cells:[...table.querySelectorAll('th,td')].map(cell=>{const r=cell.getBoundingClientRect(),l=lines(cell);return {text:cell.textContent.trim(),column:cell.cellIndex,header:cell.tagName==='TH',numeric:cell.classList.contains('right'),left:r.left,right:r.right,lines:l.count,brokenWords:l.brokenWords,clipped:l.rects.some(t=>t.left<r.left-.5||t.right>r.right+.5)};}),
          overflow:document.documentElement.scrollWidth>innerWidth};
      });
    })()`);
    const source=tables.find(table=>table.sourceSize);
    assert(source,'source-size table marked for responsive layout');
    assert(source.width<=width-35,'source-size table fits article width');
    for(const table of tables){
      assert.equal(table.overflow,false,'tables do not make the page horizontally scroll');
      for(const cell of table.cells.filter(cell=>cell.numeric&&!cell.header))assert.equal(cell.lines,1,'numeric table value stays on one line: '+cell.text);
      if(table.dylib&&table.scrollWidth>table.clientWidth+1)assert(table.maximumScrollLeft>0,'wide dylib columns remain reachable by internal scrolling');
    }
    assert.equal(source.cells.find(cell=>cell.header&&cell.text==='Mech')?.lines,1,'Mech column heading stays intact');
    assert(source.cells.every(cell=>!cell.clipped),'source-size cell text fits without overlaps');
    for(const table of tables.filter(table=>table.sourceSize||table.dylib))for(const cell of table.cells.filter(cell=>cell.header||cell.column===0))
      assert.equal(cell.brokenWords.length,0,'table labels wrap between words, not within '+cell.brokenWords.join(', '));
    report.tables.push({viewportWidth:width,tables});
    for(const name of ['source-size','dylib']){
      await evaluate(`(()=>{const table=${name==='source-size'?"document.querySelector('.workshop-source-size')":"[...document.querySelectorAll('.main-content table')].find(e=>e.textContent.includes('Checked dynamic library'))"};if(!table)throw new Error('Missing ${name} table');table.scrollIntoView({block:'center',behavior:'instant'});})()`);
      const shot=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
      writeFileSync(output.replace(/\.json$/,`-${width}-${name}.png`),Buffer.from(shot.data,'base64'));
    }
  }
  report.panes=[];
  for(const width of checks.has('panes')?[320,390,430]:[]){
    await send('Emulation.setDeviceMetricsOverride',{width,height:844,deviceScaleFactor:1,mobile:true});
    await evaluate(`document.querySelector('[data-workshop-open-output]').scrollIntoView({block:'center',behavior:'instant'})`);
    const before=await evaluate(`({windowY:scrollY,shellY:document.querySelector('.content-shell').scrollTop,articleY:document.querySelector('[data-workshop-open-output]').getBoundingClientRect().top})`);
    await click('[data-workshop-open-output]',{scroll:false});
    await new Promise(resolve=>setTimeout(resolve,250));
    const pane=await evaluate(`(()=>{
      const e=document.querySelector('[data-mech-console-pane]'),r=e.getBoundingClientRect(),close=e.querySelector('[data-mech-console-close]'),output=e.querySelector('[data-mech-output-fullscreen]'),workspace=e.querySelector('[data-mech-console-fullscreen]');
      return {left:r.left,top:r.top,width:r.width,height:r.height,viewportWidth:innerWidth,viewportHeight:innerHeight,close:close?.textContent.trim(),output:output?.textContent.trim(),outputName:output?.getAttribute('aria-label'),workspaceHidden:getComputedStyle(workspace).display==='none',closeVisible:close?.getBoundingClientRect().height>0,activePanel:e.dataset.mechConsoleActivePanel,runVisible:document.getElementById('run').getBoundingClientRect().height>0};
    })()`);
    assert(Math.abs(pane.top)<1&&Math.abs(pane.left)<1,'mobile pane opens at viewport origin');
    assert(Math.abs(pane.width-pane.viewportWidth)<1&&Math.abs(pane.height-pane.viewportHeight)<1,'mobile pane fills viewport');
    assert(pane.closeVisible&&pane.close==='Close','mobile pane has an explicit Close control');
    assert.match(pane.output,/Fullscreen output/,'output fullscreen has an unambiguous visible label');
    assert.match(pane.outputName,/fullscreen output/i,'output fullscreen accessible name identifies its target');
    assert(pane.workspaceHidden,'redundant workspace-fullscreen control hidden on mobile');
    assert.equal(pane.activePanel,'output','real demo launcher opens Output rather than the terminal');
    assert(pane.runVisible,'demo controls are visible after launch');
    const screenshot=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
    writeFileSync(output.replace(/\.json$/,`-${width}-pane.png`),Buffer.from(screenshot.data,'base64'));
    await click('[data-mech-console-close]',{scroll:false});
    await new Promise(resolve=>setTimeout(resolve,250));
    const after=await evaluate(`({closed:document.querySelector('[data-mech-repl-host]').dataset.mechConsoleOpen==='false',windowY:scrollY,shellY:document.querySelector('.content-shell').scrollTop,articleY:document.querySelector('[data-workshop-open-output]').getBoundingClientRect().top})`);
    assert(after.closed,'Close hides the mobile pane');
    for(const key of ['windowY','shellY','articleY'])assert(Math.abs(before[key]-after[key])<1,'Close preserves '+key);
    report.panes.push({width,before,pane,after});
  }
  if(checks.has('panes')){
  // Explicitly exercise browsers without the Fullscreen API. This is a
  // capability-negative Chrome check, not a claim of Safari/WebKit coverage.
  await send('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true});
  await evaluate(`document.querySelector('.workshop-source-size').scrollIntoView({block:'center',behavior:'instant'})`);
  const fallbackBefore=await evaluate('scrollY');
  await click('[data-mech-console-edge-handle]',{scroll:false});
  await evaluate(`(()=>{const pane=document.querySelector('[data-mech-console-pane]');window.workshopFullscreenOwnDescriptor=Object.getOwnPropertyDescriptor(pane,'requestFullscreen');Object.defineProperty(pane,'requestFullscreen',{value:undefined,configurable:true});})()`);
  await click('[data-mech-output-fullscreen]',{scroll:false});
  await until(`document.querySelector('[data-mech-repl-host]').dataset.mechOutputFullscreenActive==='true'`,'output fullscreen fallback opens');
  const fallback=await evaluate(`(()=>{const pane=document.querySelector('[data-mech-console-pane]'),r=pane.getBoundingClientRect(),button=pane.querySelector('[data-mech-output-fullscreen]');return {native:!!document.fullscreenElement,top:r.top,left:r.left,width:r.width,height:r.height,viewportHeight:innerHeight,label:button.textContent.trim(),outputVisible:!pane.querySelector('[data-mech-console-panel="output"]').hidden};})()`);
  report.fullscreenFallback={...fallback,beforeY:fallbackBefore,engine:'Chrome with requestFullscreen unavailable; not Safari/WebKit'};
  assert.equal(fallback.native,false,'test uses CSS fallback without native fullscreen');
  assert(fallback.outputVisible,'fallback selects actual output');
  assert.match(fallback.label,/Exit fullscreen/,'fullscreen exit has a clear visible label');
  assert(Math.abs(fallback.top)<1&&Math.abs(fallback.left)<1&&Math.abs(fallback.width-390)<1&&Math.abs(fallback.height-fallback.viewportHeight)<1,'fullscreen fallback fills the mobile viewport');
  const fallbackShot=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
  writeFileSync(output.replace(/\.json$/,'-390-output-fallback.png'),Buffer.from(fallbackShot.data,'base64'));
  await click('[data-mech-output-fullscreen]',{scroll:false});
  assert.equal(await evaluate(`document.querySelector('[data-mech-repl-host]').dataset.mechOutputFullscreenActive`),'false','Exit fullscreen restores the pane');
  report.fullscreenFallback.afterExitY=await evaluate('scrollY');
  await click('[data-mech-output-fullscreen]',{scroll:false});
  await click('[data-mech-console-close]',{scroll:false});
  await until(`document.querySelector('[data-mech-repl-host]').dataset.mechConsoleOpen==='false'`,'Close exits fullscreen and closes pane');
  assert.equal(await evaluate(`document.querySelector('[data-mech-repl-host]').dataset.mechOutputFullscreenActive`),'false','Close clears output fullscreen state');
  report.fullscreenFallback.afterCloseY=await evaluate('scrollY');
  assert(Math.abs(report.fullscreenFallback.afterCloseY-fallbackBefore)<1,'Close from fullscreen restores article position');
  await evaluate(`(()=>{const pane=document.querySelector('[data-mech-console-pane]');if(window.workshopFullscreenOwnDescriptor)Object.defineProperty(pane,'requestFullscreen',workshopFullscreenOwnDescriptor);else delete pane.requestFullscreen;delete window.workshopFullscreenOwnDescriptor;})()`);
  }
  assert.equal(exceptions.length,0);report.status='passed';
  console.log(JSON.stringify({status:report.status,tocLayouts:report.cases.length,tableWidths:report.tables.map(x=>x.viewportWidth),paneWidths:report.panes.map(x=>x.width),fullscreenFallback:report.fullscreenFallback,exceptions},null,2));
}catch(error){report.status='failed';report.error=String(error);process.exitCode=1;console.error(error);const shot=await send('Page.captureScreenshot',{format:'png'});writeFileSync(output.replace(/\.json$/,'-failure.png'),Buffer.from(shot.data,'base64'));}
finally{report.finishedAt=new Date().toISOString();writeFileSync(output,JSON.stringify(report,null,2)+'\n');await send('Page.close');socket.close();}
