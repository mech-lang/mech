const {pathToFileURL}=require('node:url');
const fs=require('fs');const assert=require('assert/strict');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const completeCount=data.languages.filter(l=>l.timed_benchmarks.length).length;
const matlabCount=data.languages.find(l=>l.key==='matlab').timed_benchmarks.length;
const sharedKeys=data.benchmarks.filter(b=>data.languages.every(l=>l.timed_benchmarks.includes(b.key))).map(b=>b.key);
const mechPaths=data.languages.filter(l=>l.family==='mech'||l.key==='mech').map(l=>l.key);
function expectedCoverage(selection){
  const benchmarks=data.benchmarks.filter(benchmark=>selection.benchmarks.includes(benchmark.key));
  return data.languages.filter(language=>selection.languages.includes(language.key)&&(selection.view!=='mech'||mechPaths.includes(language.key))).flatMap(language=>{
    const measured=benchmarks.filter(benchmark=>benchmark.groups.some(group=>group.language===language.key&&group.programs.some(key=>!selection.disabled.includes(key)&&Number.isFinite(data.programs[key].seconds)&&data.programs[key].seconds>0)));
    return measured.length?[{language:language.key,n:measured.length,selected:benchmarks.length,missing:benchmarks.filter(benchmark=>!measured.includes(benchmark)).map(benchmark=>benchmark.key)}]:[];
  });
}
async function assertCoverage(page){
  const {selection,summary}=await page.evaluate(()=>({selection:window.benchmarkExplorer.getSelection(),summary:window.benchmarkExplorer.getSummary()}));
  const expected=expectedCoverage(selection);
  assert.deepEqual(summary.map(({language,n,selected,missing})=>({language,n,selected,missing})),expected,'Coverage follows the selected measured benchmark groups and eligible languages');
  return expected;
}
async function selectBenchmarks(page,keys){
  for(const benchmark of data.benchmarks)await page.locator(`[data-benchmark="${benchmark.key}"]`).setChecked(keys.includes(benchmark.key));
  assert.deepEqual(new Set((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks),new Set(keys));
}
async function keepBenchmarks(page,changeLanguages){
  const before=(await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks;
  await changeLanguages();
  assert.deepEqual((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks,before,'Language controls preserve selected benchmarks');
}
async function clickChartPoint(page,selector){
  const marker=page.locator(selector);await marker.scrollIntoViewIfNeeded();
  const center=await marker.evaluate(mark=>{
    const point=mark.ownerSVGElement.createSVGPoint();const circle=mark.tagName.toLowerCase()==='circle';
    point.x=Number(mark.getAttribute(circle?'cx':'x'))+(circle?0:Number(mark.getAttribute('width'))/2);
    point.y=Number(mark.getAttribute(circle?'cy':'y'))+(circle?0:Number(mark.getAttribute('height'))/2);
    const screen=point.matrixTransform(mark.getScreenCTM());return {x:screen.x,y:screen.y};
  });
  await page.mouse.click(center.x,center.y);
}
(async()=>{
  const playwright=await import(process.env.PLAYWRIGHT_MODULE?pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href:'playwright');
  const {chromium}=playwright.default??playwright;
  const chrome='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
  const browser=await chromium.launch({executablePath:process.env.BROWSER_EXECUTABLE||(fs.existsSync(chrome)?chrome:undefined),headless:true});
  const errors=[];const checks=[];
  try{
    for(const width of [1440,900,390,320]){
      const page=await browser.newPage({viewport:{width,height:1000},colorScheme:'dark'});
      page.on('pageerror',e=>errors.push(e.message));
      await page.goto(process.env.BENCHMARK_EXPLORER_URL||'http://127.0.0.1:8766/');
      await page.waitForFunction(expected=>window.benchmarkExplorer?.getSummary().length===expected,completeCount);
      const first=await page.evaluate(()=>({overflow:document.documentElement.scrollWidth>innerWidth+1,
        marks:document.querySelectorAll('[data-mean=true]').length,
        ellipses:document.querySelectorAll('[data-spread-language=mech]').length,
        labels:[...document.querySelectorAll('#chart text')].map(t=>t.textContent),
        summary:window.benchmarkExplorer.getSummary()}));
      assert(!first.overflow,`Page overflow at ${width}`);assert(first.marks>=18);assert.equal(first.ellipses,4);
      assert.equal(await page.locator('[data-spread-language="mech"][data-spread-sigma="1"]').count(),2);
      assert.equal(await page.locator('[data-spread-language="mech"][data-spread-sigma="2"]').count(),2);
      assert.equal(await page.locator('#show-two-sigma').count(),0);
      assert.equal(await page.locator('#compare-shared').count(),0,'Shared-benchmark button is removed');
      assert.equal(await page.getByRole('button',{name:/compare shared benchmarks/i}).count(),0);
      assert.doesNotMatch(await page.locator('#coverage-message').textContent(),/compare shared benchmarks/i,'Coverage text no longer refers to the removed button');
      for(const key of ['areaFast','areaSmall'])assert.equal(await page.locator(`[data-sort-table="comparison"][data-sort-key="${key}"]`).count(),1);
      for(const profile of ['fast','small'])assert.equal(await page.locator(`#mean-table [data-area-profile="${profile}"]`).count(),completeCount);
      if(width<=720)await page.locator('#toggle-language-filters').click();
      const beforeContours=await page.evaluate(()=>({domains:window.benchmarkExplorer.getDomains(),summary:window.benchmarkExplorer.getSummary(),areas:[...document.querySelectorAll('#mean-table [data-area-profile]')].map(cell=>cell.textContent)}));
      await page.locator('[data-spread="mech"]').click();
      assert.equal(await page.locator('[data-spread-language="mech"]').count(),0);
      assert.deepEqual(await page.evaluate(()=>({domains:window.benchmarkExplorer.getDomains(),summary:window.benchmarkExplorer.getSummary(),areas:[...document.querySelectorAll('#mean-table [data-area-profile]')].map(cell=>cell.textContent)})),beforeContours);
      await page.locator('[data-spread="mech"]').click();
      assert.equal(await page.locator('[data-spread-sigma="2"]').count(),2);
      if(width<=720)await page.locator('#toggle-language-filters').click();
      await page.locator('#small-profile').uncheck();
      assert.equal(await page.locator('[data-spread-profile="small"]').count(),0);
      assert.equal(await page.locator('[data-spread-profile="fast"]').count(),2);
      await page.locator('#small-profile').check();
      assert.equal(await page.locator('.array-source-card, #array-source-table').count(),0,'Dedicated array-language source comparison is removed');
      assert(first.labels.some(label=>label.includes('log₂')));
      const ticks=await page.locator('.tick').allTextContents();
      assert(ticks.every(tick=>Number(tick.replace('×',''))>=1),'1× is the lowest labelled ratio');
      assert.equal(await page.locator('.spread-area[clip-path="url(#baseline-clip)"]').count(),4);
      assert.equal(await page.locator('#baseline-clip rect').count(),1);
      assert.equal(await page.locator('.line-label').count(),completeCount);
      const partial=first.summary.find(row=>row.language==='futhark');
      assert.equal(partial.n,9);assert.equal(partial.selected,10);assert.deepEqual(partial.missing,['binarytrees']);
      await assertCoverage(page);
      assert.equal(first.summary.find(row=>row.language==='matlab').n,matlabCount);
      assert.equal(await page.locator('[data-label-language="matlab"]').textContent(),matlabCount===10?'MATLAB':`MATLAB · ${matlabCount}/10`);
      assert.equal(await page.locator('[data-label-language="futhark"]').textContent(),'Futhark · 9/10');
      assert.equal(await page.locator('[data-table-language="futhark"] .coverage-badge').textContent(),'9/10');
      assert.equal(await page.locator('.endpoint[data-language="futhark"]:not([data-mean])').count(),18);

      const labelGeometry=await page.locator('.line-label').evaluateAll(labels=>labels.map(label=>{
        const box=label.getBBox();return {language:label.dataset.labelLanguage,x:box.x,y:box.y,width:box.width,height:box.height,color:label.getAttribute('fill')};
      }));
      assert.equal(new Set(labelGeometry.map(label=>label.language)).size,completeCount);
      assert(labelGeometry.every(box=>Number.isFinite(box.x)&&Number.isFinite(box.y)&&box.width>0&&box.height>0));
      if(width<=720){await page.locator('#toggle-benchmark-filters').click();await page.locator('#toggle-language-filters').click();}
      const languageLabels=await page.locator('.family-name').evaluateAll(labels=>labels.map(label=>label.querySelector('span:last-child').firstChild.textContent));
      assert(languageLabels.every((name,index)=>index===0||languageLabels[index-1].localeCompare(name,'en',{sensitivity:'base'})<=0),'Language checkboxes stay alphabetical');
      const beforeNone=await page.evaluate(()=>({selection:window.benchmarkExplorer.getSelection(),domains:window.benchmarkExplorer.getDomains()}));
      await page.locator('#no-languages').click();
      const afterNone=await page.evaluate(()=>({selection:window.benchmarkExplorer.getSelection(),domains:window.benchmarkExplorer.getDomains()}));
      assert.deepEqual(afterNone.selection.languages,[]);assert.deepEqual(afterNone.selection.selectedLanguages,[]);
      assert.deepEqual(afterNone.selection.benchmarks,beforeNone.selection.benchmarks);
      assert.equal(afterNone.selection.releaseYear,beforeNone.selection.releaseYear);
      assert.deepEqual(afterNone.selection.ellipses,beforeNone.selection.ellipses);
      assert.equal('twoSigma' in afterNone.selection,false);
      assert.deepEqual(afterNone.domains,beforeNone.domains);
      assert.equal(await page.locator('[data-language]:checked').count(),0);
      assert.equal(await page.locator('[data-language-group]:checked').count(),0);
      assert.equal(await page.locator('.endpoint, .spread-area, .line-label').count(),0);
      assert.equal(await page.locator('#mean-table tbody tr').count(),0);
      assert(await page.locator('#empty-state').isVisible());
      await keepBenchmarks(page,()=>page.locator('input[data-language="haskell"]').check());
      assert.deepEqual((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages,['haskell']);
      assert.equal(await page.locator('.line-label').count(),1);
      await keepBenchmarks(page,()=>page.locator('#all-languages').click());
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).length,completeCount);
      await keepBenchmarks(page,()=>page.locator('[data-family="mech"]').uncheck());
      await keepBenchmarks(page,()=>page.locator('[data-family="mech"]').check());
      assert.equal(await page.locator('[data-language-group]').count(),data.language_groups.length);
      const functional=data.language_groups.find(group=>group.key==='functional').languages;
      await keepBenchmarks(page,()=>page.locator('[data-only-group="functional"]').click());
      assert.deepEqual(new Set((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages),new Set(functional));
      assert.equal(await page.locator('.line-label').count(),(await assertCoverage(page)).length);
      await page.locator('[data-benchmark="binarytrees"]').uncheck();
      assert.equal(await page.locator('.line-label').count(),(await assertCoverage(page)).length);
      await page.locator('#reset').click();
      await keepBenchmarks(page,()=>page.locator('[data-only-group="interpreted"]').click());
      const interpreted=data.language_groups.find(group=>group.key==='interpreted').languages;
      assert.deepEqual(new Set((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages),new Set(interpreted));
      assert.equal(await page.locator('.line-label').count(),(await assertCoverage(page)).length);
      await keepBenchmarks(page,()=>page.locator('input[data-language="python"]').uncheck());
      assert.equal(await page.locator('[data-language-group="interpreted"]').evaluate(input=>input.indeterminate),true);
      await keepBenchmarks(page,()=>page.locator('[data-language-group="interpreted"]').check());
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages.length,interpreted.length);
      await keepBenchmarks(page,()=>page.locator('[data-only-group="array"]').click());
      assert.deepEqual(new Set((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages),new Set(data.language_groups.find(group=>group.key==='array').languages));
      await selectBenchmarks(page,data.benchmarks.filter(b=>data.language_groups.find(g=>g.key==='array').languages.every(key=>data.languages.find(l=>l.key===key).timed_benchmarks.includes(b.key))).map(b=>b.key));
      assert.equal(await page.locator('.line-label').count(),(await assertCoverage(page)).length);
      await page.locator('#reset').click();
      for(const group of ['array','dynamic','jit']){
        await keepBenchmarks(page,()=>page.locator(`[data-only-group="${group}"]`).click());
        assert((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages.includes('matlab'));
        assert.equal(await page.locator('[data-mean=true][data-language="matlab"]').count(),2);
      }
      await page.locator('#no-languages').click();
      await keepBenchmarks(page,()=>page.locator('input[data-language="matlab"]').check());
      assert.deepEqual((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).languages,['matlab']);
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSummary()))[0].n,matlabCount);
      await clickChartPoint(page,'[data-mean=true][data-language="matlab"][data-profile="fast"]');
      assert.equal(await page.locator('#inspector-title').textContent(),'MATLAB');
      assert((await page.locator('#inspector-body .profile-note').first().textContent()).includes(`${matlabCount}/10 selected benchmarks`));
      assert.equal(await page.locator('.timings-table tbody tr').count(),3);
      assert.equal(await page.locator('#inspector .code-toolbar a').filter({hasText:'Task specification'}).count(),1);
      await page.locator('#close-inspector').click();
      await page.locator('[data-spread="matlab"]').click();
      assert.equal(await page.locator('[data-spread-language="matlab"]').count(),4);
      assert((await page.locator('[data-spread-language="matlab"]').evaluateAll(paths=>paths.map(p=>p.getAttribute('aria-label')))).every(label=>label.includes(`across ${matlabCount} benchmarks`)));
      const matlabRegex=data.benchmarks.find(b=>b.key==='regexredux').groups.find(g=>g.language==='matlab');
      await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),matlabRegex.fast);
      assert.equal(await page.locator('#source-code').textContent(),data.files[data.programs[matlabRegex.fast].files[0]].code);
      await page.locator('#detail-file').selectOption('1');
      assert((await page.locator('#source-code').textContent()).includes('exec 3<&0'));
      assert((await page.locator('#inspector .source-meta').textContent()).includes('stream routing only'));
      await page.locator('#close-inspector').click();
      await page.locator('#reset').click();
      const fasta=data.benchmarks.find(b=>b.key==='fasta').groups.find(g=>g.language==='rust');
      await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),fasta.fast);
      assert.equal(await page.locator('.timings-table tbody tr').count(),5);
      assert((await page.locator('.source-provenance').textContent()).includes('Local adaptation'));
      assert.equal(await page.locator('#source-code').textContent(),data.files[data.programs[fasta.fast].files[0]].code);
      await page.locator('#close-inspector').click();
      assert((await page.locator('#coverage-message').textContent()).includes('missing Binary trees'));
      assert.equal(await page.locator('.lang-check:has(input[data-language="futhark"]) .coverage-label').textContent(),'9/10 admitted');
      await selectBenchmarks(page,sharedKeys);
      await assertCoverage(page);
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks.includes('binarytrees'),false);
      await page.locator('#reset').click();
      const reference=data.reference_summary.find(r=>r.lang==='mech');
      assert(Math.abs(first.summary.find(r=>r.language==='mech').fast.x-reference.fast.x)<1e-10);
      await page.locator('[data-spread="mech"]').click();
      assert.equal(await page.locator('[data-spread-language="mech"]').count(),0);
      await page.locator('[data-spread="rust"]').click();
      assert.equal(await page.locator('[data-spread-language="rust"]').count(),4);
      await page.locator('[data-benchmark="mandelbrot"]').uncheck();
      await assertCoverage(page);
      await keepBenchmarks(page,()=>page.locator('input[data-language="ruby"]').uncheck());
      assert.equal(await page.locator('[data-mean=true][data-language="ruby"]').count(),0);
      await clickChartPoint(page,'[data-mean=true][data-language="mech"][data-profile="fast"]');
      await page.waitForSelector('#inspector[open] #source-code');
      assert((await page.locator('#inspector-title').innerText()).includes('Mech (AOT)'));
      const id=await page.locator('#detail-program').inputValue();const program=data.programs[id];
      assert.equal(await page.locator('#source-code').textContent(),data.files[program.files[0]].code);
      assert.equal(await page.locator('.timings-table tbody tr').count(),program.samples.length);
      await page.locator('#detail-profile').selectOption('small');
      await page.locator('#include-program').uncheck();
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).disabled.length,1);
      await page.keyboard.press('Escape');
      assert.equal(await page.locator('#inspector[open]').count(),0);
      await page.locator('#reset').click();
      await page.locator('[data-inspect-benchmark="nbody"]').click();
      assert.equal(await page.locator('#chart-title').innerText(),'N-body');
      assert((await page.locator('#mean-table [data-area-profile]').allTextContents()).every(value=>value==='—'),'A single benchmark has no covariance area');
      await page.locator('#close-inspector').click();
      assert.equal(await page.locator('[data-mean=true]').count(),0);
      await page.locator('[data-view="mech"]').click();
      assert((await page.locator('#chart-subtitle').innerText()).startsWith('AOT'));
      assert.equal(await page.locator('[data-spread-language="mech"]').count(),4);
      await keepBenchmarks(page,()=>page.locator('#no-languages').click());
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).length,0);
      assert.equal(await page.locator('.endpoint').count(),0);
      await keepBenchmarks(page,()=>page.locator('#only-mech').click());
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).length,mechPaths.length);
      await page.locator('#no-benchmarks').click();
      assert(await page.locator('#empty-state').isVisible());
      await page.locator('#reset').click();
      assert.equal(await page.locator('#show-variants').count(),0);
      assert.equal(await page.getByText('Other programs',{exact:true}).count(),0);
      const chartPrograms=await page.locator('.endpoint[data-program]').evaluateAll(points=>points.map(point=>point.dataset.program));
      const endpoints=new Set(data.benchmarks.flatMap(b=>b.groups.flatMap(g=>[g.fast,g.small])));
      assert(chartPrograms.every(key=>endpoints.has(key)));
      await page.locator('[data-inspect-benchmark="mandelbrot"]').click();
      assert(await page.locator('#detail-program option').count()<=2);
      await page.locator('#close-inspector').click();
      await page.locator('#reset').click();
      assert((await page.locator('.method-copy').textContent()).includes('BQN and K cover all ten full workloads'));
      const spectral=data.benchmarks.find(b=>b.key==='spectralnorm').groups.find(g=>g.language==='mech');
      await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),spectral.fast);
      assert.equal(await page.locator('#inspector-title').textContent(),'Mech (AOT)');
      assert.equal(await page.locator('.timings-table tbody tr').count(),data.programs[spectral.fast].samples.length);
      assert.equal(await page.locator('#source-code').textContent(),data.files[data.programs[spectral.fast].files[0]].code);
      assert.equal(await page.locator('.detail-stat strong').first().textContent(),'12');
      await page.locator('#close-inspector').click();
      await page.locator('#no-benchmarks').click();
      await page.locator('[data-benchmark="nbody"]').check();
      await assertCoverage(page);
      assert.equal(await page.locator('[data-mean=true][data-language="bqn"]').count(),2);
      await clickChartPoint(page,'[data-mean=true][data-language="bqn"][data-profile="fast"]');
      assert.equal(await page.locator('.timings-table tbody tr').count(),5);
      assert.equal(await page.locator('.detail-stat strong').first().textContent(),'39');
      await page.locator('#close-inspector').click();
      await page.locator('#reset').click();
      await keepBenchmarks(page,async()=>{for(const lang of data.languages)if(!['bqn','k'].includes(lang.key))await page.locator(`input[data-language="${lang.key}"]`).uncheck();});
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).length,2);
      assert.equal(await page.locator('#empty-state').isVisible(),false);
      assert.equal(await page.locator('#chart-title').textContent(),'10 selected benchmarks');
      assert((await page.locator('#chart-subtitle').textContent()).includes('geometric means'));
      assert((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).every(row=>row.n===10));
      await page.screenshot({path:__dirname+`/arrays-only-${width}-preview.png`,fullPage:true});
      assert.equal(await page.locator('#compare-shared').count(),0);
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks.length,10);
      assert.equal(await page.locator('.lang-check:has(input[data-language="futhark"]) .coverage-label').textContent(),'9/10 admitted');
      assert.equal(await page.locator('#coverage-note').isVisible(),true);
      assert.equal(await page.locator('#coverage-note').evaluate(note=>note.open),false);
      const nucleotide=data.benchmarks.find(b=>b.key==='knucleotide').groups.find(g=>g.language==='bqn');
      await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),nucleotide.fast);
      assert.equal(await page.locator('.timings-table tbody tr').count(),3);
      assert.equal(await page.locator('#source-code').textContent(),data.files[data.programs[nucleotide.fast].files[0]].code);
      await page.locator('#detail-file').selectOption('1');
      assert((await page.locator('#inspector .source-meta').textContent()).includes('excluded from application LOC'));
      await page.locator('#close-inspector').click();
      if(width===1440){
        for(const program of Object.values(data.programs).filter(p=>['bqn','k','mojo','futhark','haskell','ocaml','matlab','erlang'].includes(p.language))){
          await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),program.key);
          assert.equal(await page.locator('#source-code').textContent(),data.files[program.files[0]].code);
          assert.equal(await page.locator('.timings-table tbody tr').count(),program.samples.length);
          if(program.language==='erlang'){
            assert.equal(await page.locator('#inspector a[href="./benchmarks-game-license.txt"]').textContent(),'License');
            assert((await page.locator('.timing-boundary').textContent()).includes('load-time JIT'));
            if(program.local_adaptation)assert.equal(await page.locator('.source-provenance').textContent(),program.source_provenance);
          }
          if(program.physical_loc!==program.loc){
            const auditText=await page.locator('.loc-audit').textContent();
            for(const expansion of program.loc_expansions)if(expansion.reason)assert(auditText.includes(expansion.reason));
          }
          await page.locator('#close-inspector').click();
        }
      }
      await page.locator('#reset').click();
      await keepBenchmarks(page,async()=>{for(const lang of data.languages)if(!['mojo','futhark'].includes(lang.key))await page.locator(`input[data-language="${lang.key}"]`).uncheck();});
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).length,2);
      assert((await page.locator('#coverage-message').textContent()).includes('missing Binary trees'));
      await page.locator('[data-benchmark="binarytrees"]').uncheck();
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSummary())).length,2);
      await assertCoverage(page);
      await page.locator('[data-spread="futhark"]').click();
      assert.equal(await page.locator('[data-spread-language="futhark"]').count(),4);
      if(width===390||width===1440)await page.screenshot({path:__dirname+`/mojo-futhark-${width}-preview.png`,fullPage:true});
      await page.locator('#reset').click();
      if(width===1440){await page.screenshot({path:__dirname+'/desktop-preview.png',fullPage:true});
        await page.locator('[data-inspect-benchmark="nbody"]').click();
        await page.screenshot({path:__dirname+'/source-preview.png',fullPage:false});}
      if(width===390){await page.locator('#toggle-benchmark-filters').click();await page.locator('#toggle-language-filters').click();await page.screenshot({path:__dirname+'/mobile-preview.png',fullPage:true});}
      checks.push({width,overflow:false,subset:true,source:true,timings:true,program_exclusion:true,ellipses:true,views:true,empty_state:true});
      await page.close();
    }
    assert.equal(errors.length,0,errors.join('\n'));
    fs.writeFileSync(__dirname+'/ui-checks.json',JSON.stringify({checks,errors},null,2)+'\n');
    console.log(JSON.stringify({checks,errors}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
