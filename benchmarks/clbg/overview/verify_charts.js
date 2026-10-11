const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const fs = require('fs');
const path = require('path');
const studies = JSON.parse(fs.readFileSync(path.join(__dirname, 'normalized-results.json'), 'utf8'));
const compactLanguages = new Set(['python', 'ruby', 'perl', 'lua', 'luajit', 'javascript', 'php']);
const byKey = new Map(studies.map(s => [s.key, s]));
const fasta = byKey.get('fasta').rows;
const mechFasta = fasta.filter(r => r.lang === 'mech').sort((a,b) => a.score-b.score)[0];
const historicalRust = fasta.filter(r => r.lang === 'rust' && !r.algorithm.strict).sort((a,b) => a.score-b.score)[0];
const treeMech = byKey.get('binarytrees').rows.filter(r => r.lang === 'mech').sort((a,b) => a.score-b.score)[0];
const expectedFast = studies.flatMap(s => [...new Set(s.rows.filter(r => !compactLanguages.has(r.lang)).map(r => r.lang))]);
const expectedProfiles = [...new Set(expectedFast)].filter(lang => studies.every(s => s.rows.some(r => r.lang === lang)));
const compactProfiles = [...compactLanguages].filter(lang => studies.every(s => s.rows.some(r => r.lang === lang)));

async function geometry(frame, root) {
  return frame.evaluate(selector => {
    const root = document.querySelector(selector);
    let clippedLabels = false, overlappingLabels = false, escapedMarks = false, invalid = false;
    let fastCompact = 0, compactConnectors = 0, compactSquares = 0;
    for (const svg of root.querySelectorAll('svg')) {
      const box = svg.getBoundingClientRect();
      const labels = [...svg.querySelectorAll('.tick text,.axis-title')]
        .filter(e => getComputedStyle(e).visibility !== 'hidden').map(e => e.getBoundingClientRect());
      clippedLabels ||= labels.some(b => b.left < box.left-1 || b.right > box.right+1 || b.top < box.top-1 || b.bottom > box.bottom+1);
      overlappingLabels ||= labels.some((b,i) => labels.slice(i+1).some(a => b.left < a.right+3 && b.right > a.left-3 && b.top < a.bottom+3 && b.bottom > a.top-3));
      const frame = svg.querySelector('rect[data-chart-frame]').getBoundingClientRect();
      for (const mark of svg.querySelectorAll('.fastest circle,.smallest rect,.observations circle,.builds circle,.connectors line')) {
        const b = mark.getBoundingClientRect();
        escapedMarks ||= b.left < frame.left-1 || b.right > frame.right+1 || b.top < frame.top-1 || b.bottom > frame.bottom+1;
        invalid ||= [...mark.attributes].some(a => /(?:NaN|Infinity)/.test(a.value));
      }
      const isCompact = d => ['python','ruby','perl','lua','luajit','javascript','php'].includes(d.lang);
      fastCompact += [...svg.querySelectorAll('.fastest circle')].filter(e => isCompact(e.__data__)).length;
      compactConnectors += [...svg.querySelectorAll('.connectors line')].filter(e => isCompact(e.__data__)).length;
      compactSquares += [...svg.querySelectorAll('.smallest rect')].filter(e => isCompact(e.__data__)).length;
    }
    return { overflow: root.scrollWidth > root.clientWidth+1, clippedLabels, overlappingLabels,
      escapedMarks, invalid, fastCompact, compactConnectors, compactSquares,
      fastest: root.querySelectorAll('.fastest circle').length };
  }, root);
}

function assertGeometry(state, context) {
  if (state.overflow || state.clippedLabels || state.overlappingLabels || state.escapedMarks || state.invalid || state.fastCompact || state.compactConnectors)
    throw new Error(JSON.stringify({context,state}));
}

async function capture(page, frame, root, filename) {
  // Chromium can omit offscreen iframe content from a full-page capture.
  // Paint the complete fragment inside the viewport before taking the image.
  const viewport = page.viewportSize();
  const height = await frame.evaluate(selector => Math.ceil(document.querySelector(selector).getBoundingClientRect().height), root);
  await page.setViewportSize({...viewport, height: height + 32});
  await page.waitForTimeout(320);
  await page.screenshot({path: path.join(__dirname, filename), fullPage: true});
  await page.setViewportSize(viewport);
  await page.waitForTimeout(320);
}

(async () => {
  const browser = await chromium.launch({executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
  const checks = [], errors = [];
  try {
    for (const theme of ['light','dark']) for (const width of [320,736,1024]) {
      const page = await browser.newPage({viewport:{width,height:1200},colorScheme:theme});
      page.on('pageerror', error => errors.push(error.message));
      for (const [file,root] of [['mega-preview.html','#mech-mega-chart'],['preview.html','#mech-benchmark-charts']]) {
        await page.goto('file://'+path.join(__dirname,file));
        const frame = page.frames().find(f => f !== page.mainFrame());
        await frame.waitForSelector(root+' rect[data-chart-frame]');
        const legends = await frame.locator(root+' .language-toggle').count();
        if (legends !== 15) throw new Error('Expected 15 runtime toggles');
        if (file === 'mega-preview.html') {
          for (const metric of ['loc','gzip']) {
            await frame.locator(`[data-metric="${metric}"]`).press("Enter");
            for (const mode of ['summary','endpoints','builds']) {
              await frame.locator(`[data-mode="${mode}"]`).press("Enter");await page.waitForTimeout(320);
              const state = await geometry(frame,root);assertGeometry(state,{file,width,theme,metric,mode});
              if (state.fastest !== (mode === 'summary' ? expectedProfiles.length : expectedFast.length)) throw new Error('Fastest-point coverage mismatch');
              if (mode === 'summary' && state.compactSquares !== compactProfiles.length) throw new Error('Compact aggregate coverage mismatch');
              checks.push({file,width,theme,metric,mode,...state});
            }
          }
          await frame.locator('[data-mode="summary"]').press('Enter');
          if (width === 736) await capture(page,frame,root,`mega-${theme}-check.png`);
        } else {
          if (await frame.locator('section').count() !== studies.length) throw new Error('Missing benchmark panel');
          for (const metric of ['loc','gzip']) {
            await frame.locator(`[data-size="${metric}"]`).press("Enter");await page.waitForTimeout(320);
            const state = await geometry(frame,root);assertGeometry(state,{file,width,theme,metric,view:'overview'});
            checks.push({file,width,theme,metric,view:'overview',...state});
          }
          await frame.locator('section[data-key="fasta"] [data-expand]').press('Enter');
          for (const metric of ['loc','gzip']) {
            await frame.locator(`[data-size="${metric}"]`).press("Enter");await page.waitForTimeout(320);
            const text = await frame.locator('[data-endpoint-table]').innerText();
            if (!text.includes(mechFasta.seconds.toFixed(3)+' s') || !text.includes(mechFasta.loc+' LOC') || !text.includes(historicalRust.seconds.toFixed(3)+' s')) throw new Error('FASTA table does not match full listed cohort');
            const state = await geometry(frame,root);assertGeometry(state,{file,width,theme,metric,view:'fasta'});
            checks.push({file,width,theme,metric,view:'fasta',...state});
          }
          await frame.locator('[data-fasta-switch]').press('Enter');await frame.waitForSelector('section[data-key="fasta-strict"]');
          if ((await frame.locator('[data-endpoint-table]').innerText()).includes(historicalRust.id)) throw new Error('Historical source in strict cohort');
          await frame.locator('[data-fasta-switch]').press('Enter');await frame.waitForSelector('section[data-key="fasta"]');
          if (width === 736) await capture(page,frame,root,`fasta-${theme}-check.png`);
          await frame.locator('[data-overview]').press('Enter');await frame.locator('section[data-key="binarytrees"] [data-expand]').press('Enter');
          const text = await frame.locator('[data-endpoint-table]').innerText();
          if (!text.includes(treeMech.seconds.toFixed(3)+' s') || !text.includes(treeMech.loc+' LOC')) throw new Error(JSON.stringify({message:'Binary trees table does not match measurements',width,theme,expected:treeMech,text}));
          if (width === 736) await capture(page,frame,root,`binarytrees-${theme}-check.png`);
          await frame.locator('[data-overview]').press('Enter');await frame.locator('section[data-key="revcomp"] [data-expand]').press('Enter');
          const compactRows = await frame.locator('[data-endpoint-table] tbody tr').allTextContents();
          if (compactRows.filter(t => t.includes('Not sampled')).length !== 7) throw new Error('Compact-only entries incorrectly infer fastest programs');
          const before = await frame.locator('svg .smallest rect').count();
          await frame.locator('.language-toggle').filter({hasText:'LuaJIT'}).press('Enter');
          const after = await frame.locator('svg .smallest rect').count();
          if (after !== before-1 || (await frame.locator('[data-endpoint-table]').innerText()).includes('LuaJIT')) throw new Error('Runtime toggle failed to hide chart and table');
          await frame.locator('.language-toggle').filter({hasText:'LuaJIT'}).press('Enter');
          await frame.locator('[data-overview]').press('Enter');
        }
      }
      await page.close();
    }
    if (errors.length) throw new Error(errors.join('\n'));
    fs.writeFileSync(path.join(__dirname,'visual-checks.json'),JSON.stringify({passed:true,compactProfiles,checks,errors},null,2)+'\n');
    console.log('Passed '+checks.length+' chart checks at three widths in both themes.');
  } finally { await browser.close(); }
})().catch(error => {console.error(error);process.exitCode=1;});
