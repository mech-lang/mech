const assert=require('node:assert/strict');
const fs=require('node:fs');
const {pathToFileURL}=require('node:url');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const modes=['mech','mech-jit','mech-resident'];
const yellowPalette={mech:'#f6c04e','mech-jit':'#ffe59a','mech-resident':'#c99b2e'};
const click=(page,selector)=>page.evaluate(selector=>document.querySelector(selector).click(),selector);
const settle=page=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
const readColors=(page,selector)=>page.locator(selector).evaluateAll((nodes,modes)=>Object.fromEntries(nodes.filter(n=>modes.includes(n.dataset.language)).map(n=>[n.dataset.language,n.getAttribute('fill').toLowerCase()])),modes);
function assertYellow(hex){
  assert.match(hex,/^#[0-9a-f]{6}$/);
  const [red,green,blue]=[1,3,5].map(index=>parseInt(hex.slice(index,index+2),16));
  assert(red>green&&green>blue,`${hex} has a yellow channel balance`);
  const hue=60*(green-blue)/(red-blue),saturation=(red-blue)/red;
  assert(hue>=35&&hue<=60&&saturation>=.25,`${hex} is a saturated yellow shade`);
}
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
      assert.equal(await page.locator('#compare-shared').count(),0);
      const brandColors=await readColors(page,'.mean-mark');
      assert.deepEqual(Object.keys(brandColors).sort(),[...modes].sort());
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
      const colors=await readColors(page,'.mean-mark');
      assert.deepEqual(colors,yellowPalette,'Mech mode uses the requested yellow palette');
      assert.equal(new Set(Object.values(colors)).size,3);
      for(const color of Object.values(colors))assertYellow(color);
      assert.equal(await page.locator('.mech-mode-key').count(),3);
      for(const key of modes){
        assert(await page.locator(`.endpoint[data-language="${key}"][data-program]`).count()>=10);
        const actual=await page.locator(`.endpoint[data-language="${key}"]`).evaluateAll(nodes=>[...new Set(nodes.map(n=>n.getAttribute('fill').toLowerCase()))]);
        assert.deepEqual(actual,[colors[key]]);
        assert.equal(await page.locator(`.mech-mode-key[data-mode-key="${key}"]`).evaluate(n=>getComputedStyle(n).color),await page.locator(`.endpoint[data-language="${key}"]`).first().evaluate(n=>getComputedStyle(n).fill));
        const lineStyle=data.languages.find(language=>language.key===key).line_style;
        const dash={dashed:'7 4',dotted:'2 4'}[lineStyle]??null;
        assert.equal(await page.locator(`.mech-mode-key[data-mode-key="${key}"] line`).getAttribute('stroke-dasharray'),dash,'Mode key preserves its line style');
        const connectors=await page.locator(`.connector[data-chart-language="${key}"]`).evaluateAll(nodes=>nodes.map(n=>({color:n.getAttribute('stroke').toLowerCase(),dash:n.getAttribute('stroke-dasharray')})));
        assert(connectors.length>0);
        assert(connectors.every(connector=>connector.color===colors[key]&&connector.dash===dash),'Mode connectors use their yellow shade and original line style');
      }
      const range=await page.evaluate(()=>window.benchmarkExplorer.getDomains());
      const benchmarks=(await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks;
      await click(page,'[data-language="mech-jit"]');await settle(page);
      assert.deepEqual((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks,benchmarks,'Language toggle preserves benchmark selection');
      assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getDomains()),range);
      assert.equal(await page.locator('.endpoint[data-language="mech-jit"]').count(),0);
      await click(page,'[data-view="summary"]');
      await click(page,'[data-language="mech-jit"]');
      assert.deepEqual((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks,benchmarks);
      assert.deepEqual(await readColors(page,'.mean-mark'),brandColors,'Summary preserves the established Mech branding');
      await click(page,'[data-view="benchmark"]');
      assert.deepEqual(await readColors(page,'.endpoint'),brandColors,'Benchmark view preserves the established Mech branding');
      assert.equal(await page.locator('#compare-shared').count(),0);
      await click(page,'[data-view="mech"]');
      assert.deepEqual(await readColors(page,'.mean-mark'),yellowPalette);
      assert((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages.includes('mech-jit'));
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false);
      await page.locator('.chart-plot').screenshot({path:__dirname+`/.sites-runtime/mech-defaults-${width}.png`});
      checks.push({width,activation:true,all_samples:true,yellow_modes:true,line_styles:true,other_view_branding:true,benchmark_selection_preserved:true,fixed_limits:true});
      await page.close();
    }
    assert.deepEqual(errors,[]);console.log(JSON.stringify({passed:true,checks}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
