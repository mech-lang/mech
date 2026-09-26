import assert from 'node:assert/strict';
import {readFileSync,existsSync,mkdtempSync} from 'node:fs';
import {join,dirname} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {buildCharts} from './charts.mjs';

const root=dirname(fileURLToPath(import.meta.url)), out=join(root,'dist');
const html=readFileSync(join(out,'index.html'),'utf8');
assert(!/BLOG[A-Z]+|\{\{[A-Z_]+\}\}|Chart build pending/.test(html),'unfilled article slot');
const ids=[...html.matchAll(/\bid="([^"]+)"/g)].map(x=>x[1]);
assert.equal(new Set(ids).size,ids.length,'duplicate DOM IDs');
for(const id of ['backend','run','pause','reset','step','inject','verify','source-editor','state-values','functions-source','matching-source']) assert(ids.includes(id),`missing control ${id}`);
for(const match of html.matchAll(/(?:src|href)="([^"#]+)"/g)) {
  const ref=match[1];
  if(/^(?:https?:|mailto:|data:)/.test(ref)) continue;
  assert(existsSync(join(out,ref.split('#')[0])),`missing local asset ${ref}`);
}
for(const match of html.matchAll(/href="#([^"]+)"/g)) assert(ids.includes(match[1]),`broken anchor ${match[1]}`);
const kernel=readFileSync(join(out,'source/ekf.mec'));
assert.equal(createHash('sha256').update(kernel).digest('hex'),'a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2');
assert.equal(kernel.toString(),readFileSync(join(root,'source/ekf.mec'),'utf8'));
const article=readFileSync(join(out,'article.mec'),'utf8');
assert(article.includes(kernel.toString()),'downloadable article must contain the actual kernel');
assert(!/BLOG[A-Z]+/.test(article),'download contains unexpanded slots');
const chartDir=mkdtempSync(join(tmpdir(),'mech-blog-chart-test-'));
buildCharts(chartDir); // Also validates n=10 and medians/MAD against the archive.
for(const name of ['cpu','backends','metal','portable']) {
  const svg=readFileSync(join(chartDir,name+'.svg'),'utf8');
  assert(svg.includes('<title')&&svg.includes('<desc'),'accessible chart');
  assert(svg.includes('unchecked (upper)')&&svg.includes('checked (lower)'),'paired ordering legend');
  assert.equal(svg,readFileSync(join(out,'assets',name+'.svg'),'utf8'),'chart build drift');
}
const wasm=readFileSync(join(out,'_mech/pkg/mech_wasm_bg.wasm'));
assert(WebAssembly.validate(wasm),'valid executable WASM artifact');
assert.deepEqual(gunzipSync(readFileSync(join(out,'_mech/pkg/mech_wasm_bg.wasm.gz'))),wasm,'compressed runtime identity');
console.log('PASS: article assets, anchors, source identity, expanded literate download, charts and WASM binary.');
