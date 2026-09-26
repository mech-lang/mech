import assert from 'node:assert/strict';
import {readFileSync,existsSync,mkdtempSync} from 'node:fs';
import {join,dirname} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {buildCharts} from './charts.mjs';
import {ekfSections} from './standard-examples.mjs';

const root=dirname(fileURLToPath(import.meta.url)), out=join(root,'dist');
const repo=join(root,'../../..');
const html=readFileSync(join(out,'index.html'),'utf8');
const publishedUrl='https://mech-lang.org/iros-r4r-2026/index.html';
assert(html.includes(`<link rel="canonical" href="${publishedUrl}">`),'wrong publication destination');
assert(!html.includes('href="https://about.mech-lang.org" aria-current="page"'),'article must not identify itself as About');
assert(html.includes('data-mech-shim="blog"'),'use canonical blog shell');
assert(!html.includes('data-mech-document-mode="presentation"'),'the article requires the full resident document REPL');
assert(html.includes('data-mech-wasm-module="./runtime.mjs"'),'document and kernel use a shared v0.4 runtime');
assert(html.includes('id="contentShell"')&&html.includes('id="articleLayout"'),'shared scroll and TOC boundaries');
assert(!html.includes('class="contents"'),'do not nest the canonical TOC inside a custom wrapper');
assert(html.includes('data-mech-console-pane')&&html.includes('data-mech-repl-mount'),'real document console mounts');
assert(html.includes('src="assets/pittsburgh-hero.jpg"'),'licensed Pittsburgh hero photograph');
assert(html.includes('Photo: Georg Pflueger / Unsplash'),'retain the replacement photograph credit');
assert(html.includes('class="github-button"')&&html.includes('aria-label="Star mech-lang/mech on GitHub"'),'canonical GitHub star button');
assert(html.includes('class="mika-separator"')&&html.includes('class="footer-main"'),'blog separator and complete footer');
assert(html.includes('IROS 2026: Rust for Robotics Workshop')&&html.includes('September 27, 2026'),'workshop metadata');
assert(html.includes('Why an EKF?')&&html.includes('HYTRADBOI'),'explain the representative workload and cite earlier implementations');
for(const structure of ['set','table','tuple','map']) assert(html.includes(`https://docs.mech-lang.org/reference/${structure}.html`),'link numerical-syntax callout to data-structure references');
assert(html.includes('evidence/ekf-long-horizon-diagnostic.md'),'retain the integrity-constraint case-study evidence');
const blogCss=readFileSync(join(repo,'include/blog.css'),'utf8');
assert.match(blogCss,/grid-template-columns: minmax\(0, 1fr\) minmax\(0, 1\.05fr\)/,'hero needs balanced title and artwork columns');
assert.match(blogCss,/font-size: clamp\(34px, 4\.2cqw, 52px\)/,'title size follows the article pane, not the viewport');
assert.match(blogCss,/\.hero \.mech-meta \.mech-date\s*\{[^}]*white-space: nowrap;/,'keep the date together while author names wrap');
const articleCss=readFileSync(join(out,'assets/article.css'),'utf8');
assert.match(articleCss,/aspect-ratio:4 \/ 3/,'skyline uses a photographic crop rather than a thin panorama');
assert(!articleCss.includes('saturate(.78)'),'do not desaturate the Pittsburgh photograph');
assert.match(blogCss,/\.hero:has\(> \.hero-visual:empty\)\s*\{\s*grid-template-columns: minmax\(0, 1fr\);/,'empty hero must use the full title width');
assert.match(blogCss,/\.hero > \.hero-visual:empty\s*\{\s*display: none;/,'empty artwork must not reserve vertical space');
assert.match(readFileSync(join(out,'assets/app.mjs'),'utf8'),/if \(event.defaultPrevented\) return;/,'demo anchor handling must defer to the shared TOC');
for(const file of ['palette.css','mech-source.css','mechdown.css','mech-repl.css','style.css','blog.css','document.js']) {
  assert.equal(readFileSync(join(out,'assets',file),'utf8'),readFileSync(join(repo,'include',file),'utf8'),`shared layer drift: ${file}`);
}
assert(!/BLOG[A-Z]+|\{\{[A-Z_]+\}\}|Chart build pending/.test(html),'unfilled article slot');
assert(!html.includes('Citations may contain at most one link.'),'citation definitions must render without formatter diagnostics');
const ids=[...html.matchAll(/\bid="([^"]+)"/g)].map(x=>x[1]);
// Native symbol references intentionally share hash:namespace addresses so
// the controller can select all occurrences. Structural/anchor IDs are unique.
const structuralIds=ids.filter(id=>!/^\d+:\d+$/.test(id));
assert.equal(new Set(structuralIds).size,structuralIds.length,'duplicate structural DOM IDs');
for(const id of ['backend','run','pause','reset','step','inject','verify','state-values']) assert(ids.includes(id),`missing control ${id}`);
for(const id of ['source-editor','compile','restore','functions-source','matching-source','modified-source']) assert(!ids.includes(id),`source editing must be removed: ${id}`);
assert(!/<textarea|contenteditable="true"|data-run-example/.test(html),'no static inline source editors');
assert.equal([...html.matchAll(/class="mech-fenced-mech-block"/g)].length,12,'four reference EKF stages, four live camera stages, one scene table and three language examples');
assert.equal([...html.matchAll(/class="mech-code-block-namespace"/g)].length,12,'twelve standard label pills');
assert.equal([...html.matchAll(/data-workshop-reference-listing/g)].length,4,'bearing-only reference remains separate from live camera state');
assert.equal([...html.matchAll(/data-workshop-kernel-listing/g)].length,4,'four camera EKF listings use their live kernel bindings');
assert.equal([...html.matchAll(/data-workshop-scene-listing/g)].length,1,'scene excerpt uses its separate resident program');
assert(html.includes('CPU · Mech interpreter (WebAssembly)'),'identify the language and CPU execution mode');
for(const id of ['camera-range','noise','motion-noise','measurement-status'])assert(ids.includes(id),`missing camera control ${id}`);
assert(!ids.includes('landmark'),'all fixed cameras are independently enabled rather than selecting a robot-mounted landmark');
assert.match(html,/data-mech-output-region="application">\s*<section class="live-demo"[^>]*id="ekf-app"/,'working app belongs in the Output pane');
assert(html.includes('data-mech-output-host="application"'),'app output survives REPL refresh');
assert(html.includes('data-workshop-fullscreen'),'article opens existing output fullscreen');
assert(html.includes('v0.4.0-beta'),'current workshop version');
assert.deepEqual(readFileSync(join(out,'poster.pdf')),readFileSync(join(root,'../poster/IROS-2026-Mech-Poster-prose-v3.pdf')),'publish the identified current PDF without re-exporting it');
for(const match of html.matchAll(/(?:src|href)="([^"#]+)"/g)) {
  const ref=match[1];
  if(/^(?:https?:|mailto:|data:)/.test(ref)) continue;
  assert(!ref.startsWith('/'),'article assets must be relative to its workshop directory');
  assert(new URL(ref,publishedUrl).pathname.startsWith('/iros-r4r-2026/'),`asset escapes workshop directory: ${ref}`);
  assert(existsSync(join(out,ref.split('#')[0])),`missing local asset ${ref}`);
}
for(const match of html.matchAll(/href="#([^"]+)"/g)) assert(ids.includes(match[1]),`broken anchor ${match[1]}`);
const citedEntries=[...html.matchAll(/<div id="(\d+)" class="mech-citation">\s*<div class="mech-citation-id">\[(\d+)\]:<\/div>/g)];
const citationNumbers=new Map(citedEntries.map(match=>[match[1],match[2]]));
assert(citationNumbers.size>0,'article requires Works Cited entries');
for(const match of html.matchAll(/<div id="\d+" class="mech-citation">\s*<div class="mech-citation-id">\[\d+\]:<\/div>(.*?)<\/div>/gs)) {
  assert(match[1].includes('<a href='),'every cited source must retain its verified link');
}
assert.equal(citationNumbers.size,citedEntries.length,'duplicate Works Cited targets');
assert.equal(new Set(citationNumbers.values()).size,citationNumbers.size,'duplicate Works Cited numbers');
const citationReferences=[...html.matchAll(/<a href="#(\d+)" class="mech-reference-link">(\d+)<\/a>/g)];
assert(citationReferences.length>0,'article requires inline citations');
for(const [,target,number] of citationReferences) {
  assert.equal(citationNumbers.get(target),number,`inline citation [${number}] disagrees with Works Cited target ${target}`);
}
const kernel=readFileSync(join(out,'source/ekf.mec'));
assert.equal(createHash('sha256').update(kernel).digest('hex'),'69480e5b46a4da7b9391755dc2a40a5e45a3e39e0899351910c83e5352893689');
assert.equal(readFileSync(join(out,'source/scene.mec'),'utf8'),readFileSync(join(root,'source/scene.mec'),'utf8'),'scene download is its actual executed source');
assert.equal(kernel.toString(),readFileSync(join(root,'source/ekf.mec'),'utf8'));
const cameraKernel=readFileSync(join(out,'source/camera-ekf.mec'),'utf8');
assert.equal(cameraKernel,readFileSync(join(root,'source/camera-ekf.mec'),'utf8'),'displayed camera source is published unchanged');
assert.match(readFileSync(join(out,'assets/app.mjs'),'utf8'),/fetch\('source\/camera-ekf\.mec'\)/,'live host executes the camera listing rather than the reference kernel');
const article=readFileSync(join(out,'article.mec'),'utf8');
assert(article.includes(`${publishedUrl}#live-demo`),'download must link to the workshop demo');
for (const section of ekfSections(kernel.toString())) assert(article.includes(section.code),'downloadable article must contain unchanged EKF computational lines');
for (const section of ekfSections(cameraKernel,{camera:true})) assert(article.includes(section.code),'downloadable article must contain unchanged live camera computational lines');
assert(article.includes('```mech:paper-ekf')&&article.includes('```mech:ekf'),'reference and live numerical programs need separate namespaces');
assert(article.includes('do not measure either this correction or the fixed-camera extension'),'do not attribute archived native timings to the new camera workload');
assert(!/BLOG[A-Z]+/.test(article),'download contains unexpanded slots');
const chartDir=mkdtempSync(join(tmpdir(),'mech-blog-chart-test-'));
buildCharts(chartDir); // Also validates n=10 and medians/MAD against the archive.
for(const name of ['cpu','backends','metal','portable']) {
  const svg=readFileSync(join(chartDir,name+'.svg'),'utf8');
  assert(svg.includes('<title')&&svg.includes('<desc'),'accessible chart');
  assert(svg.includes('unchecked (upper)')&&svg.includes('checked (lower)'),'paired ordering legend');
  assert(svg.includes("font-family: 'Fira Code'"),'chart title must use the loaded shared font family');
  assert.equal(svg,readFileSync(join(out,'assets',name+'.svg'),'utf8'),'chart build drift');
}
const wasm=readFileSync(join(out,'_mech/pkg/mech_wasm_bg.wasm'));
assert(WebAssembly.validate(wasm),'valid executable WASM artifact');
assert.deepEqual(gunzipSync(readFileSync(join(out,'_mech/pkg/mech_wasm_bg.wasm.gz'))),wasm,'compressed runtime identity');
console.log('PASS: article assets, anchors, source identity, expanded literate download, charts and WASM binary.');
