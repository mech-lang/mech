const fs=require('node:fs');
const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
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
      page.on('pageerror',e=>errors.push(e.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(n=>window.benchmarkExplorer?.getSummary().length===n,data.languages.length);
      const original=await page.evaluate(()=>window.benchmarkExplorer.getDomains());
      if(width<=720)await click(page,'#toggle-language-filters');
      for(const key of ['lisp','racket']){
        assert.equal(await page.locator(`[data-language-family="${key}"]`).count(),1);
        await click(page,'#no-languages');
        await page.locator(`input[data-language="${key}"]`).check();
        const rows=await page.evaluate(()=>window.benchmarkExplorer.getSummary());
        assert.equal(rows.length,1);assert.equal(rows[0].language,key);assert.equal(rows[0].n,10);
        assert.equal(await page.locator('.line-label').count(),1);
        assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getDomains()),original);
        await click(page,`[data-spread="${key}"]`);
        assert.equal(await page.locator(`[data-spread-language="${key}"]`).count(),4);
        for(const benchmark of data.benchmarks){
          const group=benchmark.groups.find(g=>g.language===key);
          for(const id of group.programs){
            const program=data.programs[id];
            await page.evaluate(id=>window.benchmarkExplorer.inspectProgram(id),id);
            assert.equal(await page.locator('#source-code').textContent(),data.files[program.files[0]].code);
            assert.equal(await page.locator('.timings-table tbody tr').count(),3);
            const output=await page.locator('#inspector-body').textContent();
            assert(output.includes('LOC'));assert(output.includes('AOT'));
            assert.equal(await page.locator(`#inspector .code-toolbar a[href="${program.source_url}"]`).count(),1);
            if(program.local_adaptation){
              assert.equal(await page.locator('.source-provenance').textContent(),program.source_provenance);
              assert(program.source_provenance.startsWith('Locally corrected published'));
            }else assert.equal(await page.locator('.source-provenance').count(),0);
            await click(page,'#close-inspector');
          }
        }
      }
      await click(page,'#all-languages');
      for(const group of ['dynamic','functional','aot']){
        await click(page,`[data-only-group="${group}"]`);
        const selected=await page.evaluate(()=>window.benchmarkExplorer.getSelection().languages);
        assert(selected.includes('lisp')&&selected.includes('racket'));
      }
      await click(page,'#reset');
      await page.evaluate(()=>window.benchmarkExplorer.setReleaseYear(1984));
      assert((await page.evaluate(()=>window.benchmarkExplorer.getSelection().languages)).includes('lisp'));
      assert(!(await page.evaluate(()=>window.benchmarkExplorer.getSelection().languages)).includes('racket'));
      await page.evaluate(()=>window.benchmarkExplorer.setReleaseYear(1995));
      assert((await page.evaluate(()=>window.benchmarkExplorer.getSelection().languages)).includes('racket'));
      await click(page,'#reset');
      assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
      await page.locator('#chart-wrap').screenshot({path:__dirname+`/lisp-racket-${width}-preview.png`});
      checks.push({width,languages:['lisp','racket'],complete_workloads:10,source_and_sample_tables:'passed',groups_and_years:'passed'});
      await page.close();
    }
    assert.deepEqual(errors,[]);
    fs.writeFileSync(__dirname+'/lisp-racket-ui-checks.json',JSON.stringify({status:'passed',checks,errors},null,2)+'\n');
    process.stdout.write(JSON.stringify({status:'passed',checks,errors})+'\n');
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
