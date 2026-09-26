import assert from 'node:assert/strict';
import {readFileSync,existsSync,mkdtempSync} from 'node:fs';
import {join,dirname} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {buildCharts} from './charts.mjs';

const root=dirname(fileURLToPath(import.meta.url)), out=join(root,'dist');
const repo=join(root,'../../..');
const html=readFileSync(join(out,'index.html'),'utf8');
const publishedUrl='https://mech-lang.org/iros-r4r-2026/index.html';
assert(html.includes(`<link rel="canonical" href="${publishedUrl}">`),'wrong publication destination');
assert(!html.includes('href="https://about.mech-lang.org" aria-current="page"'),'article must not identify itself as About');
assert(html.includes('data-mech-shim="blog"'),'use canonical blog shell');
assert(html.includes('data-mech-document-mode="presentation"'),'reuse shared document navigation without a second runtime');
assert(html.includes('id="contentShell"')&&html.includes('id="articleLayout"'),'shared scroll and TOC boundaries');
assert(!html.includes('class="contents"'),'do not nest the canonical TOC inside a custom wrapper');
assert(!html.includes('data-mech-console-pane'),'no dormant console');
assert(html.includes('<div class="hero-visual"></div>'),'no-artwork hero must match the shared :empty fallback');
const blogCss=readFileSync(join(repo,'include/blog.css'),'utf8');
assert.match(blogCss,/\.hero:has\(> \.hero-visual:empty\)\s*\{\s*grid-template-columns: minmax\(0, 1fr\);/,'empty hero must use the full title width');
assert.match(blogCss,/\.hero > \.hero-visual:empty\s*\{\s*display: none;/,'empty artwork must not reserve vertical space');
assert.match(readFileSync(join(out,'assets/app.mjs'),'utf8'),/if \(event.defaultPrevented\) return;/,'demo anchor handling must defer to the shared TOC');
for(const file of ['palette.css','mech-source.css','mechdown.css','style.css','blog.css','document.js']) {
  assert.equal(readFileSync(join(out,'assets',file),'utf8'),readFileSync(join(repo,'include',file),'utf8'),`shared layer drift: ${file}`);
}
assert(!/BLOG[A-Z]+|\{\{[A-Z_]+\}\}|Chart build pending/.test(html),'unfilled article slot');
const ids=[...html.matchAll(/\bid="([^"]+)"/g)].map(x=>x[1]);
assert.equal(new Set(ids).size,ids.length,'duplicate DOM IDs');
for(const id of ['backend','run','pause','reset','step','inject','verify','source-editor','state-values','functions-source','matching-source']) assert(ids.includes(id),`missing control ${id}`);
for(const match of html.matchAll(/(?:src|href)="([^"#]+)"/g)) {
  const ref=match[1];
  if(/^(?:https?:|mailto:|data:)/.test(ref)) continue;
  assert(!ref.startsWith('/'),'article assets must be relative to its workshop directory');
  assert(new URL(ref,publishedUrl).pathname.startsWith('/iros-r4r-2026/'),`asset escapes workshop directory: ${ref}`);
  assert(existsSync(join(out,ref.split('#')[0])),`missing local asset ${ref}`);
}
for(const match of html.matchAll(/href="#([^"]+)"/g)) assert(ids.includes(match[1]),`broken anchor ${match[1]}`);
const kernel=readFileSync(join(out,'source/ekf.mec'));
assert.equal(createHash('sha256').update(kernel).digest('hex'),'a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2');
assert.equal(kernel.toString(),readFileSync(join(root,'source/ekf.mec'),'utf8'));
const article=readFileSync(join(out,'article.mec'),'utf8');
assert(article.includes(`${publishedUrl}#live-demo`),'download must link to the workshop demo');
assert(article.includes(kernel.toString()),'downloadable article must contain the actual kernel');
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
