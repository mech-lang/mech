const fs=require('node:fs');
const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const completeCount=data.languages.filter(l=>l.timed_benchmarks.length).length;
const modes=['mech','mech-jit','mech-resident'];
const click=(page,selector)=>page.evaluate(selector=>document.querySelector(selector).click(),selector);
const settle=page=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
(async()=>{
  const playwright=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=playwright.default??playwright;
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const errors=[],checks=[];
  try{
    for(const width of [1440,390,320]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',error=>errors.push(error.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(n=>window.benchmarkExplorer?.getSummary().length===n,completeCount);
      if(width<=720)await click(page,'#toggle-language-filters');
      for(const key of modes)assert.equal(await page.locator(`input[data-language="${key}"]`).isChecked(),true);
      const domains=await page.evaluate(()=>window.benchmarkExplorer.getDomains());
      await click(page,'#only-mech');await settle(page);
      assert.deepEqual(new Set((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages),new Set(modes));
      assert.equal(await page.locator('.line-label').count(),3);
      assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getDomains()),domains);
      assert.equal(await page.locator('.connector[stroke-dasharray="7 4"]').count(),1);
      assert.equal(await page.locator('.connector[stroke-dasharray="2 4"]').count(),1);
      for(const key of modes){
        const group=data.benchmarks.find(b=>b.key==='nbody').groups.find(g=>g.language===key);
        const program=data.programs[group.fast];
        await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),program.key);
        assert.equal(await page.locator('#source-code').textContent(),data.files[program.files[0]].code);
        assert.equal(await page.locator('.timings-table tbody tr').count(),program.samples.length);
        if(key!=='mech')assert.equal(await page.locator('.timing-boundary').textContent(),program.timing_summary);
        if(key==='mech-resident'){
          const header=page.locator('[data-sort-table="timings"][data-sort-key="wall_seconds"]');
          await header.click();
          const rows=await page.locator('.timings-table tbody tr').evaluateAll(rows=>rows.map(row=>({run:Number(row.cells[0].textContent),measured:Number(row.cells[1].textContent),wall:Number(row.cells[2].textContent)})));
          assert(rows.every((row,i)=>!i||row.wall>=rows[i-1].wall));
          for(const row of rows){assert.equal(row.measured,Number(program.samples[row.run-1].toFixed(6)));assert.equal(row.wall,Number(program.sample_details[row.run-1].wall_seconds.toFixed(6)));assert(row.wall>=row.measured);}
          await header.click();
          const sorted=await page.locator('.timings-table tbody tr').evaluateAll(rows=>rows.map(row=>Number(row.cells[2].textContent)));
          assert(sorted.every((value,i)=>!i||value<=sorted[i-1]));
        }
        await click(page,'#close-inspector');
      }
      for(const key of ['mech-jit','mech-resident'])await click(page,`[data-spread="${key}"]`);
      assert.equal(await page.locator('.spread-area').count(),12);
      await click(page,'[data-view="mech"]');
      assert.deepEqual(new Set((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).map(row=>row.language)),new Set(modes));
      assert.equal(await page.locator('.spread-area').count(),12);
      assert.equal(await page.locator('input[data-language="rust"]').isDisabled(),true);
      assert.equal(await page.locator('input[data-language="mech-jit"]').isDisabled(),false);
      await click(page,'#all-languages');await click(page,'[data-only-group="jit"]');
      assert((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages.includes('mech-jit'));
      assert(!(await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages.includes('mech-resident'));
      await click(page,'[data-only-group="interpreted"]');
      assert((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages.includes('mech-resident'));
      await click(page,'#reset');await settle(page);
      assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getDomains()),domains);
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false);
      if(width===1440)await page.screenshot({path:__dirname+'/execution-modes-1440-preview.png',fullPage:false});
      checks.push({width,full_coverage:true,source:true,timing_boundaries:true,whole_process_sorting:true,line_styles:true,family_selection:true,ellipses:true,fixed_axes:true});await page.close();
    }
    assert.deepEqual(errors,[]);console.log(JSON.stringify({checks,errors}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
