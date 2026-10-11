const fs=require('node:fs');
const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const keys=data.languages.map(l=>l.key);
const years=[...new Set(data.languages.map(l=>l.release_year))].sort((a,b)=>a-b);
const expected=(year,selected=keys)=>data.languages.filter(l=>selected.includes(l.key)&&l.release_year<=year).map(l=>l.key);
const click=(page,selector)=>page.evaluate(selector=>document.querySelector(selector).click(),selector);
const selection=page=>page.evaluate(()=>window.benchmarkExplorer.getSelection());
const setYear=(page,year)=>page.evaluate(year=>window.benchmarkExplorer.setReleaseYear(year),year);
const setSlider=(page,index)=>page.evaluate(index=>{const range=document.querySelector('#release-year');range.value=index;range.dispatchEvent(new Event('input',{bubbles:true}));},index);
async function geometry(page){
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  return page.evaluate(()=>{
    const svg=document.querySelector('#chart'),box=svg.getBoundingClientRect();
    return {domain:window.benchmarkExplorer.getDomains(),viewBox:svg.getAttribute('viewBox'),
      ticks:[...svg.querySelectorAll('.tick')].map(t=>[t.textContent,t.getAttribute('x'),t.getAttribute('y')]),
      rect:[box.left+scrollX,box.top+scrollY,box.width,box.height],
      c:[...svg.querySelectorAll('[data-mean=true][data-language=c]')].map(p=>[p.getAttribute('cx'),p.getAttribute('cy'),p.getAttribute('x'),p.getAttribute('y')])};
  });
}
function stable(a,b){
  assert.deepEqual(a.domain,b.domain);assert.equal(a.viewBox,b.viewBox);assert.deepEqual(a.ticks,b.ticks);
  a.rect.forEach((v,i)=>assert(Math.abs(v-b.rect[i])<.1,'Chart position and dimensions stay fixed'));
  if(b.c.length)assert.deepEqual(a.c,b.c,'C endpoints retain their coordinates');
}
async function metrics(page){
  const rows=await page.locator('#mean-table tbody tr').evaluateAll(rows=>rows.map(row=>({key:row.querySelector('[data-table-language]').dataset.tableLanguage,cells:[...row.cells].map(c=>c.textContent.trim())})));
  const entries=await page.evaluate(()=>window.benchmarkExplorer.getSummary());
  assert.equal(rows.length,entries.length);
  for(const {key,cells} of rows){
    assert.equal(cells.length,7);
    const {fast,small}=entries.find(row=>row.language===key),dx=Math.log2(fast.x/small.x),dy=Math.log2(fast.y/small.y);
    const point=Math.abs(dx)<1e-10&&Math.abs(dy)<1e-10;
    assert.equal(cells[1],small.x.toFixed(2)+'×');assert.equal(cells[2],fast.y.toFixed(2)+'×');
    assert.equal(cells[3],point?'—':Math.abs(dx)<1e-10?(dy<0?'−∞':'∞'):(dy/dx).toFixed(2));
    assert.equal(cells[4],(point?0:Math.hypot(dx,dy)).toFixed(2));
  }
}
(async()=>{
  const playwright=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=playwright.default??playwright;
  const chrome='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||(fs.existsSync(chrome)?chrome:undefined),headless:true});
  const errors=[],checks=[];
  try{
    for(const width of [1440,390,320]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',error=>errors.push(error.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(count=>window.benchmarkExplorer?.getSummary().length===count,data.languages.filter(l=>l.timed_benchmarks.length).length);
      await page.evaluate(()=>document.fonts.ready);
      if(width<=720)await click(page,'#toggle-language-filters');
      assert.equal(await page.locator('#release-year').getAttribute('min'),'0');
      assert.equal(await page.locator('#release-year').getAttribute('max'),String(years.length-1));
      assert.equal(await page.locator('#release-year').getAttribute('step'),'1');
      assert.equal(await page.locator('.year-heading label').textContent(),'Release year');
      assert.equal(await page.locator('#release-mode,#release-oldest,#release-next,#release-all').count(),0);
      assert.equal(await page.locator('#release-year-sources a').count(),new Set(data.languages.map(l=>l.release_name)).size);
      assert.equal(await page.locator('#mean-table thead th').count(),7);
      const baseline=await geometry(page);
      await metrics(page);
      await setSlider(page,0);
      assert.deepEqual((await selection(page)).languages,['fortran']);
      for(const year of years){
        if(year!==years[0]){await page.locator('#release-year').focus();await page.keyboard.press('ArrowRight');}
        const state=await selection(page);
        assert.equal(state.releaseYear,year);assert.deepEqual(new Set(state.languages),new Set(expected(year)));
        assert.equal(await page.locator('#release-year').getAttribute('aria-valuetext'),String(year));
        assert.equal(await page.locator('#release-year-value').textContent(),String(year));
        assert.deepEqual(new Set(state.selectedLanguages),new Set(keys));
        stable(baseline,await geometry(page));
      }
      await page.locator('#release-year').focus();await page.keyboard.press('ArrowLeft');
      assert.equal((await selection(page)).releaseYear,2023);
      assert(!(await selection(page)).languages.some(key=>key.startsWith('mech')));
      stable(baseline,await geometry(page));
      await page.keyboard.press('ArrowRight');
      assert.equal((await selection(page)).releaseYear,2026);
      await setYear(page,2019);
      assert.equal((await selection(page)).releaseYear,2017);
      assert.deepEqual(new Set((await selection(page)).languages),new Set(expected(2017)));
      await setYear(page,2026);
      await page.locator('input[data-language="python"]').uncheck();
      await click(page,'[data-spread="rust"]');
      const custom=await selection(page);
      await setYear(page,1995);await setYear(page,2020);await setYear(page,2026);
      assert.deepEqual((await selection(page)).selectedLanguages,custom.selectedLanguages);
      assert.deepEqual((await selection(page)).ellipses,custom.ellipses);
      assert.equal((await selection(page)).languages.includes('python'),false);
      await setYear(page,1996);
      await click(page,'#no-languages');
      assert.deepEqual((await selection(page)).selectedLanguages,[]);
      assert.equal((await selection(page)).releaseYear,1996);
      await setYear(page,2026);
      assert.deepEqual((await selection(page)).languages,[],'None also clears languages hidden by the year filter');
      await click(page,'#all-languages');
      assert.deepEqual(new Set((await selection(page)).languages),new Set(keys));
      await click(page,'[data-only-group="array"]');
      const array=data.language_groups.find(g=>g.key==='array').languages;
      await setYear(page,2017);
      assert.deepEqual(new Set((await selection(page)).languages),new Set(expected(2017,array)));
      await setYear(page,2026);
      assert.deepEqual(new Set((await selection(page)).languages),new Set(array));
      await click(page,'#reset');
      await click(page,'#only-mech');await setYear(page,2010);
      assert.equal((await selection(page)).languages.length,0);
      assert.equal(await page.locator('#empty-state').isVisible(),true);
      assert((await page.locator('#empty-state').textContent()).includes('release year'));
      await setSlider(page,years.length-1);
      assert.equal((await selection(page)).releaseYear,2026);
      assert.deepEqual(new Set((await selection(page)).languages),new Set(['mech','mech-jit','mech-resident']));
      await click(page,'#reset');
      if(width<=720)await click(page,'#toggle-benchmark-filters');
      await page.locator('[data-benchmark="nbody"]').uncheck();
      await metrics(page);
      await click(page,'#reset');
      const mech=data.benchmarks.find(b=>b.key==='nbody').groups.find(g=>g.language==='mech');
      await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),mech.fast);
      await page.locator('#include-program').uncheck();
      await click(page,'#close-inspector');
      const excluded=(await selection(page)).disabled;
      await setYear(page,1995);await setYear(page,2026);
      assert.deepEqual((await selection(page)).disabled,excluded);
      await metrics(page);
      await click(page,'#reset');
      await click(page,'#no-benchmarks');await page.locator('[data-benchmark="nbody"]').check();
      await metrics(page);
      const point=await page.locator('#mean-table tbody tr').evaluateAll(rows=>rows.find(row=>row.cells[3].textContent==='—')?.cells[4].textContent);
      assert.equal(point,'0.00');
      await click(page,'#reset');await setYear(page,2014);
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false,`No page overflow at ${width}`);
      await page.screenshot({path:__dirname+`/release-years-${width}-preview.png`,fullPage:true});
      await page.locator('#mean-table').scrollIntoViewIfNeeded();
      await page.screenshot({path:__dirname+`/release-metrics-${width}-preview.png`});
      checks.push({width,year_accumulation:true,distinct_year_steps:true,mech_2026:true,manual_selection_preserved:true,none_clears_hidden_languages:true,fixed_geometry:true,compact_metrics:true,keyboard:true,overflow:false});
      await page.close();
    }
    assert.deepEqual(errors,[]);console.log(JSON.stringify({checks,errors}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
