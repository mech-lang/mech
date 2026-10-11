const {pathToFileURL}=require('node:url');
const fs=require('fs');const assert=require('assert/strict');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const completeCount=data.languages.filter(l=>l.timed_benchmarks.length).length;
const mechPaths=data.languages.filter(l=>l.family==='mech'||l.key==='mech').map(l=>l.key);
const group=data.benchmarks.find(b=>b.key==='mandelbrot').groups.find(g=>g.language==='cpp');
const program=data.programs[group.programs[0]];
const selector=(table,key)=>`[data-sort-table="${table}"][data-sort-key="${key}"]`;
const ascending=(values,direction)=>values.every((value,i)=>!i||direction*(typeof value==='string'?values[i-1].localeCompare(value,'en',{numeric:true,sensitivity:'base'}):values[i-1]-value)<=0);

(async()=>{
  const playwright=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=playwright.default??playwright;
  const chrome='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||(fs.existsSync(chrome)?chrome:undefined),headless:true});
  const errors=[],checks=[];
  try{
    for(const width of [1440,320]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',error=>errors.push(error.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(expected=>window.benchmarkExplorer?.getSummary().length===expected,completeCount);
      const baseline=await page.evaluate(()=>({summary:window.benchmarkExplorer.getSummary(),selection:window.benchmarkExplorer.getSelection()}));
      const summaries=new Map(baseline.summary.map(row=>[row.language,row]));
      async function sort(table,key,direction){
        const button=page.locator(selector(table,key));
        const expected=direction===1?'ascending':'descending';
        if(await button.locator('..').getAttribute('aria-sort')!==expected)await button.click();
        if(await button.locator('..').getAttribute('aria-sort')!==expected)await button.click();
        assert.equal(await button.locator('..').getAttribute('aria-sort'),expected);
        assert.equal(await page.locator(`[data-sort-table="${table}"]`).evaluateAll(buttons=>buttons.filter(b=>b.parentElement.getAttribute('aria-sort')!=='none').length),1);
      }
      for(const key of ['language','loc','fastest','slope','length','areaFast','areaSmall']){
        for(const direction of [1,-1]){
          await sort('comparison',key,direction);
          const rows=await page.locator('#mean-table tbody [data-table-language]').evaluateAll(buttons=>buttons.map(b=>({key:b.dataset.tableLanguage,name:b.textContent})));
          const values=rows.map(row=>key==='language'?row.name:summaries.get(row.key).metrics[key]);
          assert(ascending(values.filter(value=>value!=null),direction),`Comparison ${key} at ${width}`);
          if(values.includes(null))assert(values.slice(values.indexOf(null)).every(value=>value===null),'Undefined metrics sort last');
        }
      }
      const header=page.locator(selector('comparison','fastest'));
      await sort('comparison','fastest',1);await header.focus();await page.keyboard.press('Enter');
      assert.equal(await header.locator('..').getAttribute('aria-sort'),'descending');
      assert.equal(await page.evaluate(()=>document.activeElement.dataset.sortKey),'fastest');
      assert.deepEqual(await page.evaluate(()=>({summary:window.benchmarkExplorer.getSummary(),selection:window.benchmarkExplorer.getSelection()})),baseline);
      if(width<=720)await page.locator('#toggle-benchmark-filters').click();
      await page.locator('[data-benchmark="nbody"]').uncheck();
      assert.equal(await header.locator('..').getAttribute('aria-sort'),'descending');
      const subset=new Map((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).map(row=>[row.language,row.fast.y]));
      const keys=await page.locator('#mean-table tbody [data-table-language]').evaluateAll(buttons=>buttons.map(b=>b.dataset.tableLanguage));
      assert(ascending(keys.map(key=>subset.get(key)),-1));
      await page.locator('#reset').click();
      assert.equal(await page.locator('[data-sort-table="arraySources"], #array-source-table').count(),0);
      await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),program.key);
      const source=data.files[program.files[0]].code;
      for(const key of ['run','elapsed'])for(const direction of [1,-1]){
        await sort('timings',key,direction);
        const rows=await page.locator('.timings-table tbody tr').evaluateAll(rows=>rows.map(row=>({run:Number(row.cells[0].textContent),elapsed:Number(row.cells[1].textContent)})));
        assert(ascending(rows.map(row=>row[key]),direction));
        assert.deepEqual(rows.map(row=>row.run).sort((a,b)=>a-b),program.samples.map((_,i)=>i+1));
        for(const row of rows)assert.equal(row.elapsed,Number(program.samples[row.run-1].toFixed(6)));
        assert.equal(await page.locator('#source-code').textContent(),source);
      }
      await page.locator(selector('timings','elapsed')).focus();await page.keyboard.press('Enter');
      assert.equal(await page.evaluate(()=>document.activeElement.dataset.sortKey),'elapsed');
      for(const key of ['program','loc','median','show'])for(const direction of [1,-1]){
        await sort('implementations',key,direction);
        const rows=await page.locator('.variant-list tbody tr').evaluateAll(rows=>rows.map(row=>({key:row.querySelector('[data-toggle-program]').dataset.toggleProgram,show:row.querySelector('input').checked?0:1})));
        const values=rows.map(row=>key==='show'?row.show:data.programs[row.key][{program:'id',loc:'loc',median:'seconds'}[key]]);
        assert(ascending(values,direction),`Implementation ${key} at ${width}`);
      }
      await sort('implementations','show',1);
      await page.locator(`[data-toggle-program="${program.key}"]`).uncheck();
      const last=page.locator('.variant-list tbody tr').last();
      assert.equal(await last.locator('input').getAttribute('data-toggle-program'),program.key);
      assert.equal(await last.locator('input').isChecked(),false);
      assert.equal(await page.locator('#detail-program').inputValue(),program.key);
      assert.equal(await page.locator('#source-code').textContent(),source);
      await page.locator(selector('implementations','median')).scrollIntoViewIfNeeded();
      const scroll=await page.locator('#inspector').evaluate(dialog=>dialog.scrollTop);
      await sort('implementations','median',1);
      assert(Math.abs(await page.locator('#inspector').evaluate(dialog=>dialog.scrollTop)-scroll)<=1,'Inspector scroll preserved');
      assert.equal(await page.evaluate(()=>document.activeElement.dataset.sortKey),'median');
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false,`Page overflow at ${width}`);
      assert(await page.locator('#sort-status').textContent());
      await page.locator('#close-inspector').click();
      await page.screenshot({path:__dirname+`/sorting-${width}-preview.png`,fullPage:true});
      checks.push({width,comparison:true,timings:true,original_run_numbers:true,implementations:true,keyboard:true,subset_persistence:true,source_preserved:true,scroll_preserved:true,overflow:false});
      await page.close();
    }
    assert.deepEqual(errors,[]);console.log(JSON.stringify({checks,errors}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
