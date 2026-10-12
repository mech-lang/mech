const assert=require('node:assert/strict');
const fs=require('node:fs');
const {pathToFileURL}=require('node:url');
const {selectTraitOnly,setTraitChecked}=require('./trait_test_helpers.cjs');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const subset=['binarytrees','fasta','spectralnorm'];
const mechModes=data.languages.filter(language=>language.family==='mech'||language.key==='mech').map(language=>language.key);
const selection=page=>page.evaluate(()=>window.benchmarkExplorer.getSelection());
async function keepSubset(page,action){
  const before=(await selection(page)).benchmarks;
  await action();
  assert.deepEqual((await selection(page)).benchmarks,before,'Language controls preserve the selected benchmark subset');
  assert.deepEqual(new Set(before),new Set(subset));
}
async function assertCoverage(page){
  const state=await selection(page);
  const benchmarks=data.benchmarks.filter(benchmark=>state.benchmarks.includes(benchmark.key));
  const expected=data.languages.filter(language=>state.languages.includes(language.key)).flatMap(language=>{
    const measured=benchmarks.filter(benchmark=>benchmark.groups.some(group=>group.language===language.key&&group.programs.some(key=>!state.disabled.includes(key)&&Number.isFinite(data.programs[key].seconds)&&data.programs[key].seconds>0)));
    return measured.length?[{language:language.key,n:measured.length,selected:benchmarks.length,missing:benchmarks.filter(benchmark=>!measured.includes(benchmark)).map(benchmark=>benchmark.key)}]:[];
  });
  const actual=await page.evaluate(()=>window.benchmarkExplorer.getSummary().map(({language,n,selected,missing})=>({language,n,selected,missing})));
  assert.deepEqual(actual,expected,'Only eligible languages with measured selected groups contribute to the means');
  return expected;
}
async function assertPartialCoverage(page){
  const expected=await assertCoverage(page);
  const futhark=expected.find(row=>row.language==='futhark');
  assert(futhark&&futhark.n<futhark.selected&&futhark.missing.includes('binarytrees'));
  const coverage=`${futhark.n}/${futhark.selected}`;
  assert.equal(await page.locator('[data-table-language="futhark"] .coverage-badge').textContent(),coverage);
  assert(await page.locator('[data-table-language="futhark"] .coverage-badge').isVisible());
  assert((await page.locator('[data-label-language="futhark"]').textContent()).endsWith(` · ${coverage}`));
  assert(await page.locator('[data-label-language="futhark"]').isVisible());
  const message=await page.locator('#coverage-message').textContent();
  assert(message.includes(`${coverage} selected benchmarks; missing Binary trees`),'Partial coverage remains explained after manual selection');
  assert.doesNotMatch(message,/compare shared benchmarks/i);
}
async function assertNoOverflow(page,width){
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false,`No page overflow at ${width}px`);
}
async function assertRemovedControl(page){
  assert.equal(await page.locator('#compare-shared').count(),0);
  assert.equal(await page.getByRole('button',{name:/compare shared benchmarks/i}).count(),0);
  assert.doesNotMatch(await page.locator('#coverage-message').textContent(),/compare shared benchmarks/i);
}
(async()=>{
  const module=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=module.default??module;
  const chrome='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||(fs.existsSync(chrome)?chrome:undefined),headless:true});
  const errors=[],checks=[];
  try{
    for(const width of [1440,390,320]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',error=>errors.push(error.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(()=>window.benchmarkExplorer);
      await assertRemovedControl(page);await assertNoOverflow(page,width);
      for(const kind of ['benchmark','language']){
        const toggle=page.locator(`#toggle-${kind}-filters`);
        if(await toggle.getAttribute('aria-expanded')==='false')await toggle.click();
      }
      await page.locator('#no-benchmarks').click();
      for(const key of subset)await page.locator(`[data-benchmark="${key}"]`).check();
      assert.deepEqual(new Set((await selection(page)).benchmarks),new Set(subset));
      await assertPartialCoverage(page);await assertNoOverflow(page,width);

      await keepSubset(page,()=>page.locator('input[data-language="futhark"]').uncheck());
      assert.equal((await selection(page)).languages.includes('futhark'),false);
      await assertCoverage(page);
      await keepSubset(page,()=>page.locator('input[data-language="futhark"]').check());
      await assertPartialCoverage(page);

      await keepSubset(page,()=>page.locator('[data-family="mech"]').uncheck());
      const withoutFamily=await selection(page);
      assert(mechModes.every(key=>!withoutFamily.languages.includes(key)));
      await assertCoverage(page);
      await keepSubset(page,()=>page.locator('[data-family="mech"]').check());
      const afterFamily=await selection(page);
      assert(mechModes.every(key=>afterFamily.languages.includes(key)));

      await keepSubset(page,()=>selectTraitOnly(page,'interpreted'));
      await assertCoverage(page);
      assert(!(await selection(page)).languages.includes('ruby')&&!(await selection(page)).languages.includes('php'),'YJIT and tracing PHP are excluded from the interpreted execution path');
      await keepSubset(page,()=>setTraitChecked(page,'interpreted',false));
      await assertPartialCoverage(page);
      await keepSubset(page,()=>selectTraitOnly(page,'functional'));
      const afterGroup=await selection(page);
      const functional=data.language_groups.find(group=>group.key==='functional').languages.filter(key=>data.languages.find(language=>language.key===key).release_year<=afterGroup.releaseYear);
      assert.deepEqual(new Set(afterGroup.languages),new Set(functional));
      await assertPartialCoverage(page);

      await keepSubset(page,()=>page.locator('#all-languages').click());
      await assertPartialCoverage(page);
      await keepSubset(page,()=>page.locator('#no-languages').click());
      assert.deepEqual((await selection(page)).languages,[]);
      await assertCoverage(page);assert(await page.locator('#empty-state').isVisible());
      await keepSubset(page,()=>page.locator('#only-mech').click());
      assert.deepEqual(new Set((await selection(page)).languages),new Set(mechModes));
      await assertCoverage(page);
      await keepSubset(page,()=>page.locator('#all-languages').click());
      await assertPartialCoverage(page);await assertRemovedControl(page);await assertNoOverflow(page,width);
      checks.push({width,removed_control:true,manual_subset:true,language_controls_preserve_subset:true,partial_coverage:true,overflow:false});
      await page.close();
    }
    assert.deepEqual(errors,[]);console.log(JSON.stringify({passed:true,checks,errors}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
