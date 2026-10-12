const fs=require('node:fs');
const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const {selectTraitOnly,expectedTraitKeys}=require('./trait_test_helpers.cjs');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const keys=['csharp','csharp-aot','fsharp','fsharp-aot','ada','pascal','fortran',
  'dart','dart-jit','smalltalk'];
const families={csharp:['csharp-aot','csharp'],fsharp:['fsharp-aot','fsharp'],
  dart:['dart','dart-jit']};
const click=(page,selector)=>page.evaluate(s=>document.querySelector(s).click(),selector);
const selection=page=>page.evaluate(()=>window.benchmarkExplorer.getSelection());
const domain=page=>page.evaluate(()=>window.benchmarkExplorer.getDomains());
(async()=>{
  const playwright=await import(process.env.PLAYWRIGHT_MODULE?
    pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=playwright.default??playwright;
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const errors=[],checks=[];
  try{
    for(const width of [1440,390]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',e=>errors.push(e.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(n=>window.benchmarkExplorer?.getSummary().length===n,data.languages.length);
      const bounds=await domain(page);
      if(width<=720)await click(page,'#toggle-language-filters');
      assert.equal(await page.locator('[data-language="toit"],[data-language-family="toit"],[data-language="micropython"]').count(),0);
      for(const [family,members] of Object.entries(families)){
        assert.equal(await page.locator(`[data-language-family="${family}"]`).count(),1);
        await click(page,'#no-languages');
        await page.locator(`input[data-family="${family}"]`).check();
        assert.deepEqual(new Set((await selection(page)).languages),new Set(members));
        await page.locator(`input[data-language="${members[0]}"]`).uncheck();
        assert.equal(await page.locator(`input[data-family="${family}"]`).evaluate(i=>i.indeterminate),true);
        assert.deepEqual(new Set((await selection(page)).languages),new Set(members.slice(1)));
        assert.deepEqual(await domain(page),bounds);
      }
      for(const language of keys){
        await click(page,'#no-languages');
        await page.locator(`input[data-language="${language}"]`).check();
        const summary=await page.evaluate(()=>window.benchmarkExplorer.getSummary());
        assert.equal(summary.length,1);assert.equal(summary[0].language,language);assert.equal(summary[0].n,10);
        assert.equal(await page.locator('.line-label').count(),1);
        await click(page,`[data-spread="${language}"]`);
        assert.equal(await page.locator(`[data-spread-language="${language}"]`).count(),4);
        assert.deepEqual(await domain(page),bounds);
        for(const benchmark of data.benchmarks){
          const group=benchmark.groups.find(g=>g.language===language);
          for(const key of group.programs){
            const p=data.programs[key];
            await page.evaluate(k=>window.benchmarkExplorer.inspectProgram(k),key);
            assert.equal(await page.locator('#source-code').textContent(),data.files[p.files[0]].code);
            assert.equal(await page.locator('.timings-table tbody tr').count(),3);
            const text=await page.locator('#inspector-body').textContent();assert(text.includes(p.runtime_policy));
            const comparison=await page.locator('.mech-comparison').textContent();
            assert(comparison.includes(`${p.seconds.toFixed(6)} s`));
            assert(comparison.includes(`${p.mech_comparison.time_x_mech.toFixed(3)}× Mech AOT time`));
            assert(comparison.includes(`${p.mech_comparison.seconds.toFixed(6)} s`));
            assert(comparison.includes(`${p.mech_comparison.fastest_loc_x_mech.toFixed(3)}×`));
            assert(comparison.includes(`${p.mech_comparison.smallest_loc_x_mech.toFixed(3)}×`));
            assert((await page.locator('.timings-table thead').textContent()).includes('Measured seconds'));
            const times=await page.locator('.timings-table td.sample-value').allTextContents();
            assert.deepEqual(times,p.samples.map(t=>t.toFixed(6)),'Timing cells contain raw seconds');
            await click(page,'#close-inspector');
          }
        }
      }
      for(const group of data.language_groups){
        await selectTraitOnly(page,group.key);
        assert.deepEqual(new Set((await selection(page)).languages),new Set(expectedTraitKeys(data,group.key)));
        assert.deepEqual(await domain(page),bounds);
      }
      await click(page,'#reset');
      const yearSteps=[...new Set(data.languages.map(l=>l.release_year))].sort((a,b)=>a-b);
      for(const key of keys){
        const language=data.languages.find(l=>l.key===key);
        const previousYear=yearSteps.filter(y=>y<language.release_year).at(-1);
        if(previousYear!=null){
          await page.evaluate(y=>window.benchmarkExplorer.setReleaseYear(y),previousYear);
          assert(!(await selection(page)).languages.includes(key));
        }
        await page.evaluate(y=>window.benchmarkExplorer.setReleaseYear(y),language.release_year);
        assert((await selection(page)).languages.includes(key));
      }
      await click(page,'#reset');
      assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
      await page.locator('#chart-wrap').screenshot({path:__dirname+`/remaining-languages-${width}-preview.png`});
      checks.push({width,execution_variants:true,workloads:10,seconds_and_mech_ratios:true,
        source_and_samples:true,groups_and_years:true,fixed_axes:true});
      await page.close();
    }
    assert.deepEqual(errors,[]);
    const result={status:'passed',checks,errors};
    fs.writeFileSync(__dirname+'/remaining-languages-ui-checks.json',JSON.stringify(result,null,2)+'\n');
    console.log(JSON.stringify(result));
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});
