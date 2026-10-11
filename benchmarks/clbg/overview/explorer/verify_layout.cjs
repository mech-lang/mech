const fs=require('node:fs');
const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const completeCount=data.languages.filter(l=>l.timed_benchmarks.length).length;
const mechPaths=data.languages.filter(l=>l.family==='mech'||l.key==='mech').map(l=>l.key);
const click=(page,selector)=>page.evaluate(selector=>document.querySelector(selector).click(),selector);
const settle=page=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
async function layout(page){return page.evaluate(()=>{
  const rect=selector=>{const b=document.querySelector(selector).getBoundingClientRect();return {left:b.left+scrollX,top:b.top+scrollY,right:b.right+scrollX,bottom:b.bottom+scrollY,width:b.width,height:b.height};};
  return {chart:rect('#chart'),panel:rect('#inspector'),table:rect('#mean-table-wrap'),open:document.querySelector('#inspector').open,overflow:document.documentElement.scrollWidth>innerWidth+1,domains:window.benchmarkExplorer.getDomains()};
});}
function sameChart(a,b,label){for(const key of ['left','top','width','height'])assert(Math.abs(a.chart[key]-b.chart[key])<.1,`${label}: stable chart ${key}`);assert.deepEqual(a.domains,b.domains,`${label}: stable axis limits`);}
(async()=>{
  const playwright=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=playwright.default??playwright;
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const checks=[],errors=[];
  try{
    for(const width of [1440,1280,1150,900,390,320]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',error=>errors.push(error.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(expected=>window.benchmarkExplorer?.getSummary().length===expected,completeCount);
      if(width<=720){await click(page,'#toggle-benchmark-filters');await click(page,'#toggle-language-filters');}
      await settle(page);
      const original=await layout(page);
      for(const group of ['dynamic','array','interpreted']){
        await click(page,`[data-only-group="${group}"]`);await settle(page);
        sameChart(original,await layout(page),`${width}: ${group} only`);
        assert.equal(await page.locator('#coverage-note').evaluate(note=>note.open),false,'Coverage never opens itself');
      }
      await click(page,'#all-languages');await click(page,'#coverage-note summary');await settle(page);
      const expanded=await layout(page);
      for(const group of ['dynamic','array']){
        await click(page,`[data-only-group="${group}"]`);await settle(page);
        sameChart(expanded,await layout(page),`${width}: ${group} with coverage note expanded`);
      }
      await click(page,'#all-languages');await click(page,'[data-view="benchmark"]');await settle(page);
      const before=await layout(page);
      await click(page,'[data-inspect-benchmark="nbody"]');await settle(page);
      const opened=await layout(page);
      assert(opened.open);assert(!opened.overflow);assert.deepEqual(opened.domains,before.domains);
      if(width>1150){
        assert(opened.panel.left>=opened.chart.right,`${width}: panel sits beside the plot`);
        assert(opened.chart.width<before.chart.width,`${width}: plot gives the panel space`);
      }else{
        assert(opened.panel.top>=opened.chart.bottom,`${width}: panel sits below the plot`);
        assert(opened.table.top>=opened.panel.bottom,`${width}: panel precedes the comparison table`);
        sameChart(before,opened,`${width}: details leave the chart in place`);
      }
      assert.equal(await page.locator('#source-code').textContent(),data.files[data.programs[data.benchmarks.find(b=>b.key==='nbody').groups.find(g=>g.language==='mech').fast].files[0]].code);
      if(width===1440)await page.screenshot({path:__dirname+'/docked-details-1440-preview.png',fullPage:false});
      if(width===390){await page.evaluate(()=>window.scrollTo(0,document.querySelector('#chart').getBoundingClientRect().top+scrollY-20));await page.screenshot({path:__dirname+'/docked-details-390-preview.png',fullPage:false});}
      await click(page,'#close-inspector');await settle(page);sameChart(before,await layout(page),`${width}: close restores plot`);
      checks.push({width,groups_stable:true,coverage_note_stable:true,details_never_overlap:true,close_restores_chart:true});
      await page.close();
    }
    assert.deepEqual(errors,[]);console.log(JSON.stringify({checks,errors}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
