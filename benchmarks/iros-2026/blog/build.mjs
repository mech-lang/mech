import {readFileSync,writeFileSync,mkdirSync,copyFileSync,existsSync,readdirSync} from 'node:fs';
import {dirname,resolve,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {gzipSync} from 'node:zlib';
import {blogShell} from './blog-shell.mjs';
import {expandStandardExamples,decorateStandardExamples} from './standard-examples.mjs';
import {documentSourceBundle} from './document-source.mjs';

const root=dirname(fileURLToPath(import.meta.url));
const repo=resolve(root,'../../..');
const out=join(root,'dist'), temporary=join(root,'_build');
const read=path=>readFileSync(path,'utf8');
const hash=path=>createHash('sha256').update(readFileSync(path)).digest('hex');
const escape=text=>text.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
for(const dir of [out,temporary,join(out,'assets'),join(out,'source'),join(out,'evidence'),join(out,'_mech/pkg'),join(root,'vendor')]) mkdirSync(dir,{recursive:true});
const site=process.argv[2];
for(const [name,path] of [['logo.png','img/logo.png'],['FiraCode-Regular.ttf','fonts/FiraCode-Regular.ttf']]) {
  const dest=join(root,'vendor',name);
  if(!existsSync(dest)) {
    if(!site) throw new Error('First build: supply the public website asset directory as the first argument (see README).');
    copyFileSync(join(site,path),dest);
  }
  copyFileSync(dest,join(out,'assets',name));
}
const expected='a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2';
if(hash(join(root,'source/ekf.mec'))!==expected) throw new Error('The presentation EKF has changed; update its provenance and tests intentionally.');
const renderer=join(root,'render/target/debug/mech-iros-blog-render');
if(!existsSync(renderer)) throw new Error('Build render/Cargo.toml first.');
function render(file,shim,dest) {execFileSync(renderer,[file,shim,dest],{stdio:'inherit'});return read(dest);}
const {source:expanded,examples,downloads}=expandStandardExamples(read(join(root,'article.mec')));
let literate=expanded;
const slotLinks=new Map([
  ['BLOGLIVEDEMO',['Run the interactive EKF','https://mech-lang.org/iros-r4r-2026/index.html#live-demo']],
  ['BLOGPIPELINE',['Build, activation and checked reactive turns','assets/pipeline.svg']],
  ['BLOGCPUCHART',['CPU implementation comparison','assets/cpu.svg']],
  ['BLOGBACKENDCHART',['One source across Mech backends','assets/backends.svg']],
  ['BLOGMETALCHART',['Native Metal implementation comparison','assets/metal.svg']],
  ['BLOGPORTABLECHART',['Application source reuse across targets','assets/portable.svg']],
]);
for(const [token,[label,href]] of slotLinks) {
  if(literate.split(token).length!==2) throw new Error(`Missing or repeated slot ${token}`);
  literate=literate.replace(token,`[${label}](${href})`);
}
for(const item of downloads) writeFileSync(join(out,item.href),item.source);
const expandedPath=join(temporary,'article.mec');
writeFileSync(expandedPath,literate);
const shellPath=join(temporary,'blog-shell.html');
writeFileSync(shellPath,blogShell(repo).replace('{{DOCUMENT_SOURCES}}',documentSourceBundle(literate)));
let html=decorateStandardExamples(render(expandedPath,shellPath,join(temporary,'article.html')),examples);
function slot(name,content) {
  const href=slotLinks.get(name)[1];
  let count=0;
  html=html.replace(/<p class="mech-paragraph[^\"]*">[\s\S]*?<\/p>/g,paragraph=>{
    if(!paragraph.includes(`href="${href}"`)) return paragraph;
    count++;return content;
  });
  if(count!==1) throw new Error(`Expected one paragraph slot ${name}; found ${count}`);
}
slot('BLOGLIVEDEMO',read(join(root,'demo.html')));
// One tokenizing pass over the unescaped source avoids altering inserted markup.
function highlightRust(code) {
  const token=/\/\/[^\n]*|"(?:\\.|[^"\\])*"|\b(?:use|fn|let|mut|unsafe|for|in|pub|return)\b|\b\d+(?:\.\d+)?\b|[;,]/g;
  let last=0, result='';
  for(const match of code.matchAll(token)) {
    result+=escape(code.slice(last,match.index));
    const value=match[0];
    const kind=value.startsWith('//')?'comment':value.startsWith('"')?'string':/^\d/.test(value)?'number':'keyword';
    result+=`<span class="code-${kind}">${escape(value)}</span>`;last=match.index+value.length;
  }
  return result+escape(code.slice(last));
}
const rustDownloads=downloads.filter(item=>item.file.endsWith('.rs'));
let rustIndex=0;
html=html.replace(/<pre class="mech-code-block"[^>]*>[\s\S]*?<\/pre>/g,()=>{
  const item=rustDownloads[rustIndex++];
  if(!item) throw new Error('Unexpected non-Mech fence');
  return `<pre class="mech-code-block" data-language="rust"><code>${highlightRust(item.source.trimEnd())}</code></pre>`;
});
if(rustIndex!==rustDownloads.length) throw new Error('Rust code fence count changed');
slot('BLOGPIPELINE',`<figure class="mech-figure workshop-figure"><div class="figure-frame chart-desktop">${read(join(root,'pipeline.svg'))}</div><div class="figure-frame chart-mobile">${read(join(root,'pipeline-mobile.svg'))}</div><figcaption class="mech-figure-caption">Poster architecture diagram, adapted for the article. The build lane runs during build and activation; the execution lane repeats for each accepted or rejected turn. Telemetry values are illustrative, while the live figure above uses the actual computed values.</figcaption></figure>`);
copyFileSync(join(root,'pipeline.svg'),join(out,'assets/pipeline.svg'));
copyFileSync(join(root,'pipeline-mobile.svg'),join(out,'assets/pipeline-mobile.svg'));
copyFileSync(join(root,'hero.svg'),join(out,'assets/hero.svg'));
if(existsSync(join(root,'charts.mjs'))) {
  const {buildCharts}=await import('./charts.mjs');await buildCharts(join(out,'assets'));
}
for(const [file,token,caption] of [
  ['cpu','BLOGCPUCHART','Matched population and eight-worker CPU budget. The Mech and Rust rows use four-wide SIMD with fused turns. Every bar is median ± MAD from ten retained samples.'],
  ['backends','BLOGBACKENDCHART','Same archived Mech source, per-turn publication, five execution targets. JIT and AOT share numerical lowering; worker count and host capability also determine available parallelism.'],
  ['metal','BLOGMETALCHART','Cross-system native Metal campaign, collected September 24. GPU hatching distinguishes device measurements from CPU measurements.'],
  ['portable','BLOGPORTABLECHART','Application-source reuse in Mech, Taichi, and Halide. Each system selects CPU or Metal using its own backend options and schedules.']]) {
  const path=join(out,'assets',`${file}.svg`);
  slot(token,`<figure class="mech-figure workshop-figure"><div class="figure-frame chart-desktop">${read(path)}</div><div class="figure-frame chart-mobile">${read(join(out,'assets',`${file}-mobile.svg`))}</div><figcaption class="mech-figure-caption">${caption}</figcaption></figure>`);
}
if(/BLOG[A-Z]+|\{\{[A-Z_]+\}\}/.test(html)) throw new Error('Unfilled article slot.');
writeFileSync(join(out,'index.html'),html);
writeFileSync(join(out,'article.mec'),literate);
for(const file of ['app.mjs','drawing.mjs','verify.mjs','article.css','runtime.mjs']) copyFileSync(join(root,file),join(out,'assets',file));
for(const file of ['palette.css','mech-source.css','mechdown.css','mech-repl.css','style.css','blog.css','document.js','browser-compute.js']) {
  const input=join(repo,'include',file),dest=join(out,'assets',file);
  copyFileSync(input,dest);
}
for(const file of ['mech_wasm.js','mech_wasm_bg.wasm','mech_wasm.d.ts']) copyFileSync(join(repo,'src/wasm/pkg',file),join(out,'_mech/pkg',file));
writeFileSync(join(out,'_mech/pkg/mech_wasm_bg.wasm.gz'),gzipSync(readFileSync(join(out,'_mech/pkg/mech_wasm_bg.wasm')),{level:9}));
copyFileSync(join(root,'vendor/OFL.txt'),join(out,'assets/OFL.txt'));
for(const file of ['mika-pose-point.png','MIKA.md']) copyFileSync(join(root,'vendor',file),join(out,'assets',file));
for(const file of ['README.md','VALIDATION.md']) if(existsSync(join(root,file))) copyFileSync(join(root,file),join(out,file));
for(const file of ['apple-m1-cpu-equal-n10-2026-09-24.json','apple-m1-metal-equal-n10-2026-09-24.json','apple-m1-mech-backend-pairs-n10-2026-09-25.json']) copyFileSync(join(root,'../results',file),join(out,'evidence',file));
copyFileSync(join(root,'browser-verification.json'),join(out,'evidence/browser-verification.json'));
copyFileSync(join(repo,'LICENSE'),join(out,'LICENSE'));
const evidenceFiles=['article.mec',...readdirSync(join(out,'source')).map(x=>'source/'+x),...readdirSync(join(out,'assets')).map(x=>'assets/'+x),'_mech/pkg/mech_wasm.js','_mech/pkg/mech_wasm_bg.wasm','_mech/pkg/mech_wasm_bg.wasm.gz',...readdirSync(join(out,'evidence')).map(x=>'evidence/'+x)];
const manifest={builtAt:new Date().toISOString(),gitRevision:execFileSync('git',['rev-parse','HEAD'],{cwd:repo,encoding:'utf8'}).trim(),worktreeDirty:!!execFileSync('git',['status','--porcelain'],{cwd:repo,encoding:'utf8'}).trim(),files:Object.fromEntries(evidenceFiles.map(x=>[x,hash(join(out,x))]))};
writeFileSync(join(out,'build-manifest.json'),JSON.stringify(manifest,null,2)+'\n');
console.log(`Built ${out}/index.html`);
