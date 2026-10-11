const {chromium}=require('playwright');
const fs=require('fs');
const path=require('path');
const data=JSON.parse(fs.readFileSync(path.join(__dirname,'original-style-results.json'),'utf8'));
const compact=new Set(['python','ruby','perl','lua','luajit','javascript','php']);
const expectedBuilds=data.studies.reduce((n,s)=>n+s.groups.reduce((m,g)=>m+g.count,0),0);
if(expectedBuilds!==575||data.studies.length!==8)throw new Error('Incomplete fresh cohort');
const checks=[];
(async()=>{
 const browser=await chromium.launch({executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
 const errors=[];
 try {
  for(const theme of ['light','dark'])for(const width of [320,736,1024]){
   const page=await browser.newPage({viewport:{width,height:1200},colorScheme:theme});
   page.on('pageerror',e=>errors.push(e.message));
   await page.goto('file://'+path.join(__dirname,'original-style-preview.html'));
   const frame=page.frames().find(f=>f!==page.mainFrame());
   await frame.waitForSelector('#mech-original-style figure:last-of-type svg rect[data-chart-frame]');
   const state=await frame.evaluate(()=>{
    const root=document.querySelector('#mech-original-style');
    const result={figures:root.querySelectorAll('figure').length,builds:root.querySelectorAll('.build').length,
     fastest:root.querySelectorAll('.fastest').length,compactFast:0,compactLines:0,overflow:root.scrollWidth>root.clientWidth+1,
     invalid:false,escapedMarks:false,clippedLabels:false,overlappingLabels:false,hiddenAxes:false};
    const compact=['python','ruby','perl','lua','luajit','javascript','php'];
    for(const svg of root.querySelectorAll('svg')){
     const box=svg.getBoundingClientRect(),f=svg.querySelector('[data-chart-frame]').getBoundingClientRect();
     const text=[...svg.querySelectorAll('.tick text,.axis-title,.mech-label')].filter(e=>getComputedStyle(e).visibility!=='hidden').map(e=>e.getBoundingClientRect());
     result.clippedLabels ||= text.some(b=>b.left<box.left-1||b.right>box.right+1||b.top<box.top-1||b.bottom>box.bottom+1);
     result.overlappingLabels ||= text.some((a,i)=>text.slice(i+1).some(b=>a.left<b.right+3&&a.right>b.left-3&&a.top<b.bottom+3&&a.bottom>b.top-3));
     result.hiddenAxes ||= [...svg.querySelectorAll('.axis-title')].some(e=>getComputedStyle(e).visibility==='hidden');
     for(const el of svg.querySelectorAll('.build,.fastest,.smallest,.connector')){
      const b=el.getBoundingClientRect();result.escapedMarks ||= b.left<f.left-1||b.right>f.right+1||b.top<f.top-1||b.bottom>f.bottom+1;
      result.invalid ||= [...el.attributes].some(a=>/NaN|Infinity/.test(a.value));
     }
     result.compactFast += [...svg.querySelectorAll('.fastest')].filter(e=>compact.includes(e.dataset.language)).length;
     result.compactLines += [...svg.querySelectorAll('.connector')].filter(e=>compact.includes(e.dataset.language)).length;
    }
    return result;
   });
   if(state.figures!==10||state.builds!==575||state.fastest!==80||state.compactFast||state.compactLines||state.overflow||state.invalid||state.escapedMarks||state.clippedLabels||state.overlappingLabels||state.hiddenAxes)throw new Error(JSON.stringify({theme,width,state}));
   const ruby=frame.locator('figure').first().locator('.smallest[data-language="ruby"]');
   await ruby.dispatchEvent('pointermove');
   // Exercise the plot overlay at Ruby's actual endpoint.
   await frame.evaluate(()=>{const svg=document.querySelector('figure svg'),mark=svg.querySelector('.smallest[data-language="ruby"]'),b=mark.getBoundingClientRect(),hit=svg.querySelector('[data-chart-hit]');hit.dispatchEvent(new PointerEvent('pointermove',{clientX:b.x+b.width/2,clientY:b.y+b.height/2,bubbles:true}));});
   const tooltip=await frame.locator('[data-hover]').innerText();
   if(!tooltip.includes('Ruby')||!tooltip.includes('Fastest program not sampled'))throw new Error('Compact endpoint tooltip missing scope');
   const before=await frame.locator('.smallest[data-language="luajit"]').count();
   const toggle=frame.locator('figure').first().locator('.language-toggle').filter({hasText:'LuaJIT'});
   await toggle.press('Enter');
   if(await frame.locator('.smallest[data-language="luajit"]').count()!==0||before!==9)throw new Error('Legend toggle did not hide LuaJIT in all figures');
   await toggle.press('Enter');
   if(await frame.locator('.build').count()!==575)throw new Error('Legend toggle did not restore builds');
   if(width===736){
    const height=await frame.evaluate(()=>Math.ceil(document.querySelector('#mech-original-style').getBoundingClientRect().height));
    await page.setViewportSize({width,height:height+32});await page.waitForTimeout(320);
    await frame.locator('figure[data-key="summary"][data-zoom="false"]').screenshot({path:path.join(__dirname,`original-summary-${theme}-check.png`)});
    await frame.locator('figure[data-key="spectralnorm"]').screenshot({path:path.join(__dirname,`original-spectralnorm-${theme}-check.png`)});
   }
   checks.push({theme,width,...state});await page.close();
  }
  if(errors.length)throw new Error(errors.join('\n'));
  fs.writeFileSync(path.join(__dirname,'original-style-checks.json'),JSON.stringify({passed:true,checks,errors},null,2)+'\n');
  console.log('Original style passed: all 575 builds, 80 fastest endpoints, compact scope, legend interactions, three widths and both themes.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
