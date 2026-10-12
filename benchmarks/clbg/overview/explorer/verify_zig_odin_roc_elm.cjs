const fs=require('node:fs');
const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const {selectTraitOnly}=require('./trait_test_helpers.cjs');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const click=(page,selector)=>page.evaluate(selector=>document.querySelector(selector).click(),selector);
(async()=>{
  const playwright=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=playwright.default??playwright;
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const errors=[],checks=[];
  try{
    for(const width of [1440,390]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',error=>errors.push(error.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(n=>window.benchmarkExplorer?.getSummary().length===n,data.languages.length);
      const domain=await page.evaluate(()=>window.benchmarkExplorer.getDomains());
      if(width<=720)await click(page,'#toggle-language-filters');
      for(const key of ['zig','odin','roc','elm']){
        assert.equal(await page.locator(`[data-language-family="${key}"]`).count(),1);
        await click(page,'#no-languages');
        await page.locator(`input[data-language="${key}"]`).check();
        const rows=await page.evaluate(()=>window.benchmarkExplorer.getSummary());
        assert.equal(rows.length,1);assert.equal(rows[0].language,key);assert.equal(rows[0].n,10);
        assert.equal(await page.locator('.line-label').count(),1);
        assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getDomains()),domain);
        await click(page,`[data-spread="${key}"]`);
        assert.equal(await page.locator(`[data-spread-language="${key}"]`).count(),4);
        for(const benchmark of data.benchmarks){
          const group=benchmark.groups.find(g=>g.language===key);
          for(const id of group.programs){
            const program=data.programs[id];
            await page.evaluate(id=>window.benchmarkExplorer.inspectProgram(id),id);
            assert.equal(await page.locator('#source-code').textContent(),data.files[program.files[0]].code);
            assert.equal(await page.locator('.timings-table tbody tr').count(),3);
            const text=await page.locator('#inspector-body').textContent();
            assert(text.includes('LOC'));assert(text.includes(program.runtime_policy));
            const comparison=await page.locator('.mech-comparison').textContent();
            assert(comparison.includes(`${program.seconds.toFixed(6)} s`));
            assert(comparison.includes(`${program.mech_comparison.time_x_mech.toFixed(3)}× Mech AOT time`));
            assert(comparison.includes(`${program.mech_comparison.seconds.toFixed(6)} s`));
            assert(comparison.includes(`${program.mech_comparison.fastest_loc_x_mech.toFixed(3)}×`));
            assert(comparison.includes(`${program.mech_comparison.smallest_loc_x_mech.toFixed(3)}×`));
            assert.equal(await page.locator(`#inspector .code-toolbar a[href="${program.source_url}"]`).count(),1);
            if(program.local_adaptation)assert.equal(await page.locator('.source-provenance').textContent(),program.source_provenance);
            if(key==='elm'){
              assert.equal(await page.locator('#detail-file option').count(),3);
              await page.selectOption('#detail-file','1');
              assert.equal(await page.locator('#source-code').textContent(),data.files[program.files[1]].code);
              assert((await page.locator('.source-meta').textContent()).includes('excluded from application LOC'));
            }
            await click(page,'#close-inspector');
          }
        }
      }
      for(const group of ['functional','aot','jit']){
        await selectTraitOnly(page,group);
        const selected=await page.evaluate(()=>window.benchmarkExplorer.getSelection().languages);
        if(group==='functional')assert(selected.includes('roc')&&selected.includes('elm'));
        if(group==='aot')assert(['zig','odin','roc'].every(key=>selected.includes(key))&&!selected.includes('elm'));
        if(group==='jit')assert(selected.includes('elm')&&!selected.includes('roc'));
        assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getDomains()),domain);
      }
      await click(page,'#reset');
      for(const key of ['zig','odin','roc','elm']){
        const language=data.languages.find(language=>language.key===key);
        await page.evaluate(year=>window.benchmarkExplorer.setReleaseYear(year),language.release_year-1);
        assert(!(await page.evaluate(()=>window.benchmarkExplorer.getSelection().languages)).includes(key));
        await page.evaluate(year=>window.benchmarkExplorer.setReleaseYear(year),language.release_year);
        assert((await page.evaluate(()=>window.benchmarkExplorer.getSelection().languages)).includes(key));
      }
      await click(page,'#reset');
      assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
      await page.locator('#chart-wrap').screenshot({path:__dirname+`/zig-odin-roc-elm-${width}-preview.png`});
      checks.push({width,languages:['zig','odin','roc','elm'],complete_workloads:10,source_and_samples:'passed',support_labels:'passed',groups_and_years:'passed',fixed_domains:true});
      await page.close();
    }
    assert.deepEqual(errors,[]);
    fs.writeFileSync(__dirname+'/zig-odin-roc-elm-ui-checks.json',JSON.stringify({status:'passed',checks,errors},null,2)+'\n');
    process.stdout.write(JSON.stringify({status:'passed',checks,errors})+'\n');
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
