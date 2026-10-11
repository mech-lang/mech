const assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
(async()=>{
  const playwright=await import(pathToFileURL(require.resolve(process.env.PLAYWRIGHT_MODULE)).href);
  const {chromium}=playwright.default??playwright;
  const browser=await chromium.launch({executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const checks=[];
  try{
    for(const width of [1440,390]){
      const page=await browser.newPage({viewport:{width,height:1000}}),errors=[];
      page.on('pageerror',e=>errors.push(e.message));
      await page.goto('http://127.0.0.1:8766/');
      await page.waitForFunction(()=>window.benchmarkExplorer?.getSummary().length>0);
      const click=s=>page.locator(s).evaluate(e=>e.click());
      await page.locator('#show-global').check();
      assert.equal(await page.locator('[data-global-statistic=average]').count(),0);
      assert.equal(await page.locator('[data-global-statistic=median]').count(),2);
      const opacity=await page.locator('.mean-mark').first().evaluate(e=>Number(getComputedStyle(e).opacity));
      assert(opacity<.2,'Language means fade in median mode');
      assert.equal(await page.locator('.median-trail').count(),2);
      const allYears=await page.evaluate(()=>window.benchmarkExplorer.getMedianHistory());
      const final=await page.evaluate(()=>window.benchmarkExplorer.getGlobalStatistics());
      assert.deepEqual(allYears.at(-1).statistics,final);
      const sampleOpacity=await page.locator('[data-sample=true]').first().evaluate(e=>Number(getComputedStyle(e).opacity));
      assert(sampleOpacity<.05,'Sample cloud fades below language means');
      await page.locator('#show-median-trail').uncheck();
      assert.equal(await page.locator('.median-trail,.median-trail-point').count(),0);
      await page.locator('#show-median-trail').check();
      for(const view of ['summary','benchmark','mech']){
        await click(`[data-view="${view}"]`);
        const before=await page.evaluate(()=>window.benchmarkExplorer.getDomains());
        await page.evaluate(()=>window.benchmarkExplorer.setReleaseYear(2026));
        await click('#play-years');
        assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).playing,true);
        const first=await page.evaluate(()=>window.benchmarkExplorer.getSelection().releaseYear);
        await page.waitForFunction(y=>window.benchmarkExplorer.getSelection().releaseYear>y,first);
        assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).view,view);
        assert.deepEqual(await page.evaluate(()=>window.benchmarkExplorer.getDomains()),before);
        await click('#play-years');
        assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).playing,false);
        const paused=await page.evaluate(()=>window.benchmarkExplorer.getSelection().releaseYear);
        await new Promise(r=>setTimeout(r,1000));
        assert.equal(await page.evaluate(()=>window.benchmarkExplorer.getSelection().releaseYear),paused);
      }
      await page.evaluate(()=>window.benchmarkExplorer.setReleaseYear(2026));
      await click('#play-years');
      await page.evaluate(()=>window.benchmarkExplorer.setReleaseYear(2014));
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).playing,false);
      assert.equal((await page.evaluate(()=>window.benchmarkExplorer.getSelection())).releaseYear,2014);
      await click('#reset');
      await page.locator('#show-global').check();
      await page.locator('#chart-wrap').screenshot({path:__dirname+`/median-playback-${width}-preview.png`});
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false);
      assert.deepEqual(errors,[]);checks.push({width,median_only:true,faded_cloud:true,year_playback_all_views:true,fixed_axes:true,pause:true});
      await page.close();
    }
    console.log(JSON.stringify({passed:true,checks}));
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});
