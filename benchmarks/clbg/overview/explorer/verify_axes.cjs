const fs=require('node:fs');
const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const data=JSON.parse(fs.readFileSync(__dirname+'/dist/data.json','utf8'));
const completeCount=data.languages.filter(l=>l.timed_benchmarks.length).length;
const mechPaths=data.languages.filter(l=>l.family==='mech'||l.key==='mech').map(l=>l.key);

async function snapshot(page){
  return page.evaluate(programs=>{
    const chart=document.querySelector('#chart');
    const summary=new Map(window.benchmarkExplorer.getSummary().map(row=>[row.language,row]));
    const domains=Object.fromEntries(['xMin','xMax','yMin','yMax'].map(key=>[key,Number(chart.dataset[key])]));
    const ticks=[...chart.querySelectorAll('.tick')].map(tick=>({text:tick.textContent,x:tick.getAttribute('x'),y:tick.getAttribute('y')}));
    const points=Object.fromEntries([...chart.querySelectorAll('.endpoint')].map(point=>{
      const {language,profile,program,mean}=point.dataset;
      const value=mean?summary.get(language)?.[profile]:programs[program];
      const key=[language,profile,mean?'mean':program,value?.x?.toPrecision(12),value?.y?.toPrecision(12)].join(':');
      const circle=point.tagName.toLowerCase()==='circle';
      return [key,{x:circle?Number(point.getAttribute('cx')):Number(point.getAttribute('x'))+Number(point.getAttribute('width'))/2,
        y:circle?Number(point.getAttribute('cy')):Number(point.getAttribute('y'))+Number(point.getAttribute('height'))/2}];
    }));
    const rect=chart.getBoundingClientRect();
    return {domains,ticks,points,viewBox:chart.getAttribute('viewBox'),rect:{left:rect.left+scrollX,top:rect.top+scrollY,width:rect.width,height:rect.height}};
  },data.programs);
}

function stable(before,after,label){
  assert(Object.values(after.domains).every(Number.isFinite),`${label}: explicit finite chart domains`);
  assert.deepEqual(after.domains,before.domains,`${label}: fixed log2 axis limits`);
  assert.deepEqual(after.ticks,before.ticks,`${label}: unchanged visible ticks and tick positions`);
  assert.equal(after.viewBox,before.viewBox,`${label}: unchanged plot dimensions`);
  for(const key of ['left','top','width','height'])assert(Math.abs(after.rect[key]-before.rect[key])<.1,`${label}: chart ${key} stays fixed on the page`);
  for(const [key,point] of Object.entries(after.points)){
    if(!before.points[key])continue; // Program exclusions can produce different mean values.
    for(const axis of ['x','y'])assert(Math.abs(point[axis]-before.points[key][axis])<1e-7,`${label}: ${key} retains its ${axis} position`);
  }
}

const click=(page,selector)=>page.evaluate(selector=>document.querySelector(selector).click(),selector);
async function checkBox(page,selector,checked){
  await page.evaluate(({selector,checked})=>{
    const input=document.querySelector(selector);
    if(input.checked===checked)return;
    input.checked=checked;input.dispatchEvent(new Event('change',{bubbles:true}));
  },{selector,checked});
}
async function setLanguages(page,keys){
  // Each change passes through the actual language control event handler.
  for(const language of data.languages)await checkBox(page,`input[data-language="${language.key}"]`,keys.includes(language.key));
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
      await page.waitForFunction(expected=>window.benchmarkExplorer?.getSummary().length===expected,completeCount);
      await page.evaluate(()=>document.fonts.ready);
      if(width<=720){await click(page,'#toggle-benchmark-filters');await click(page,'#toggle-language-filters');}
      assert.equal(await page.locator('.array-source-card, #array-source-table').count(),0);
      for(const view of ['summary','benchmark','mech']){
        await click(page,'#reset');
        await click(page,`[data-view="${view}"]`);
        let baseline=await snapshot(page);
        assert(baseline.ticks.length>0,`${view} at ${width}: visible axis ticks`);
        assert(baseline.ticks.every(tick=>Number(tick.text.replace('×',''))>=1),`${view}: 1× is the first labelled tick`);
        assert(Object.keys(baseline.points).length>0,`${view} at ${width}: visible measured points`);
        if(view==='benchmark')assert.equal(await page.locator('#focus-means').isDisabled(),true,'Focus is unavailable for a single benchmark');
        else{
          if(width!==320)await page.locator('#chart-wrap').screenshot({path:__dirname+`/focus-means-${view}-${width}-preview.png`});
          const summary=await page.evaluate(()=>window.benchmarkExplorer.getSummary());
          await page.locator('#focus-means').uncheck();
          const fullRange=await snapshot(page);
          if(width!==320)await page.locator('#chart-wrap').screenshot({path:__dirname+`/full-range-${view}-${width}-preview.png`});
          assert(fullRange.domains.xMax>baseline.domains.xMax,`${view} at ${width}: Focus on means tightens LOC bounds`);
          assert(fullRange.domains.yMax>baseline.domains.yMax,`${view} at ${width}: Focus on means tightens time bounds`);
          assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getSummary()),summary,'Zoom preserves means, line metrics, and covariance areas');
          await click(page,'#only-mech');stable(fullRange,await snapshot(page),`${view} at ${width}: full range stays fixed for Mech selection`);
          await click(page,'#no-languages');stable(fullRange,await snapshot(page),`${view} at ${width}: full range stays fixed with no languages`);
          await click(page,view==='mech'?'#only-mech':'#all-languages');stable(fullRange,await snapshot(page),`${view} at ${width}: full range stays fixed restoring its languages`);
          for(const selector of ['[data-spread="mech"]','#show-global']){
            await click(page,selector);stable(fullRange,await snapshot(page),`${view} at ${width}: full range stays fixed toggling ${selector}`);
            await click(page,selector);
          }
          await click(page,'#show-samples');
          stable(baseline,await snapshot(page),`${view} at ${width}: hiding samples restores means range`);
          assert.equal(await page.locator('#focus-means').isDisabled(),true);
          assert.equal(await page.locator('#focus-means-option').isVisible(),false);
          await click(page,'#show-samples');
          stable(baseline,await snapshot(page),`${view} at ${width}: showing samples retains means range`);
          await page.locator('#focus-means').uncheck();
          stable(fullRange,await snapshot(page),`${view} at ${width}: returning to full range restores it`);
          await page.locator('#focus-means').check();
        }
        const compare=async label=>stable(baseline,await snapshot(page),`${view} at ${width}: ${label}`);
        if(view!=='mech'){
          for(const language of data.languages){
            await checkBox(page,`input[data-language="${language.key}"]`,false);
            await compare(`remove ${language.key}`);
            await checkBox(page,`input[data-language="${language.key}"]`,true);
            await compare(`restore ${language.key}`);
          }
          for(const group of data.language_groups){
            await click(page,`[data-only-group="${group.key}"]`);
            await compare(`only ${group.key} group`);
            await setLanguages(page,data.languages.map(language=>language.key));
            await compare('restore all languages');
          }
          await setLanguages(page,['futhark']);
          await compare('only Futhark: labelled nine-test mean');
          assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).benchmarks.length,10,'Language changes do not change requested benchmarks');
          await setLanguages(page,['mech']);
          await compare('only Mech within the current view');
          await setLanguages(page,data.languages.map(language=>language.key));
          await click(page,'#only-mech');
          await compare('Mech language preset preserves current view');
          assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).view,view);
          await click(page,'#all-languages');
          await compare('All languages preset preserves current view');
          assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).view,view);
          await setLanguages(page,[]);
          await compare('all languages hidden: axes remain visible');
          assert.equal(await page.locator('#empty-state').isVisible(),true);
          await click(page,'#all-languages');
          await compare('restore all languages from empty state');
        }
        const ellipseLanguages=view==='mech'?mechPaths:data.languages.map(language=>language.key);
        for(const language of ellipseLanguages){
          await click(page,`[data-spread="${language}"]`);await compare(`toggle ${language} ellipse`);
        }
        for(const language of ellipseLanguages)await click(page,`[data-spread="${language}"]`);
        await compare('restore ellipse visibility');
        await click(page,'#no-languages');await compare('None language action preserves chart geometry');
        assert.deepEqual((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).selectedLanguages,[]);
        await click(page,'#only-mech');await compare('restore Mech after None');
        if(view!=='mech')await click(page,'#all-languages');
        for(const selector of ['#fast-profile','#small-profile','#show-samples']){
          await checkBox(page,selector,false);await compare(`hide ${selector}`);
          await checkBox(page,selector,true);await compare(`restore ${selector}`);
        }
        await checkBox(page,'#fast-profile',false);await checkBox(page,'#small-profile',false);
        await compare('both profiles hidden: axes remain visible');
        assert.equal(await page.locator('#empty-state').isVisible(),true);
        await checkBox(page,'#fast-profile',true);await checkBox(page,'#small-profile',true);
        const program=data.benchmarks.find(benchmark=>benchmark.key==='nbody').groups.find(group=>group.language==='mech').fast;
        const undocked=baseline;
        await page.evaluate(key=>window.benchmarkExplorer.inspectProgram(key),program);
        await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
        baseline=await snapshot(page);
        assert.deepEqual(baseline.domains,undocked.domains,'Docking details preserves axis limits');
        await checkBox(page,'#include-program',false);await compare('exclude Mech N-body program');
        await checkBox(page,'#include-program',true);await compare('restore Mech N-body program');
        await click(page,'#close-inspector');
        await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
        stable(undocked,await snapshot(page),`${view} at ${width}: close dock restores chart`);
        assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false,`${view} at ${width}: no page overflow`);
        checks.push({width,view,focus_means:view!=='benchmark',stable_domains:true,stable_ticks:true,stable_points:true,stable_page_position:true,program_visibility:true,source_section_removed:true});
      }
      await page.close();
    }
    assert.deepEqual(errors,[],'No uncaught browser errors');
    console.log(JSON.stringify({checks,errors}));
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exit(1);});
