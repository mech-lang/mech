const assert=require('node:assert/strict');
const fs=require('node:fs');
const {pathToFileURL}=require('node:url');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const modes=['mech','mech-jit','mech-resident'];
const click=(page,selector)=>page.evaluate(selector=>document.querySelector(selector).click(),selector);
const settle=page=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
(async()=>{
  const module=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=module.default??module;
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const errors=[],checks=[];
  try{
    for(const width of [1440,390,320]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',error=>errors.push(error.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(()=>window.benchmarkExplorer);
      const program=Object.values(data.programs).find(p=>p.language==='mech'&&p.seconds!=null);
      await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),program.key);
      await page.locator('#include-program').uncheck();
      await click(page,'#close-inspector');
      await page.locator('#show-global').check();
      await page.locator('#show-samples').uncheck();
      await page.locator('#fast-profile').uncheck();
      await page.locator('#small-profile').uncheck();
      await click(page,'#no-languages');
      await click(page,'#no-benchmarks');
      await page.evaluate(()=>window.benchmarkExplorer.setReleaseYear(2023));
      await click(page,'[data-view="mech"]');await settle(page);
      const state=await page.evaluate(()=>window.benchmarkExplorer.getSelection());
      assert.equal(state.view,'mech');assert.equal(state.releaseYear,2026);
      assert.equal(state.samples,true);assert.equal(state.focusMeans,false);
      assert.equal(state.global,false);
      assert.deepEqual(new Set(state.languages),new Set(modes));
      assert.equal(state.benchmarks.length,10);
      assert.equal(state.disabled.includes(program.key),false);
      assert.equal(await page.locator('#fast-profile').isChecked(),true);
      assert.equal(await page.locator('#small-profile').isChecked(),true);
      assert.equal(await page.locator('#empty-state').isVisible(),false);
      const means=await page.evaluate(()=>window.benchmarkExplorer.getSummary());
      assert.deepEqual(means.map(entry=>entry.n),[10,10,10]);
      const colors=await page.locator('.mean-mark').evaluateAll(nodes=>Object.fromEntries(nodes.map(n=>[n.dataset.language,n.getAttribute('fill')])));
      assert.equal(new Set(Object.values(colors)).size,3);
      assert.equal(await page.locator('.mech-mode-key').count(),3);
      for(const key of modes){
        assert(await page.locator(`.endpoint[data-language="${key}"][data-program]`).count()>=10);
        const actual=await page.locator(`.endpoint[data-language="${key}"]`).evaluateAll(nodes=>[...new Set(nodes.map(n=>n.getAttribute('fill')))]);
        assert.deepEqual(actual,[colors[key]]);
        assert.equal(await page.locator(`.mech-mode-key[data-mode-key="${key}"]`).evaluate(n=>getComputedStyle(n).color),await page.locator(`.endpoint[data-language="${key}"]`).first().evaluate(n=>getComputedStyle(n).fill));
      }
      const range=await page.evaluate(()=>window.benchmarkExplorer.getDomains());
      await click(page,'[data-language="mech-jit"]');await settle(page);
      assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getDomains()),range);
      assert.equal(await page.locator('.endpoint[data-language="mech-jit"]').count(),0);
      await click(page,'[data-view="summary"]');
      const brand=await page.locator('.mean-mark[data-language="mech"]').first().getAttribute('fill');
      await click(page,'[data-view="mech"]');
      assert.equal((await page.locator('.mean-mark[data-language="mech"]').first().getAttribute('fill')).toLowerCase(),brand.toLowerCase());
      assert((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages.includes('mech-jit'));
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false);
      await page.locator('#chart-wrap').screenshot({path:__dirname+`/.sites-runtime/mech-defaults-${width}.png`});
      checks.push({width,activation:true,all_samples:true,distinct_modes:true,fixed_limits:true});
      await page.close();
    }
    assert.deepEqual(errors,[]);console.log(JSON.stringify({passed:true,checks}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
