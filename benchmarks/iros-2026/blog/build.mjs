import {readFileSync,writeFileSync,mkdirSync,copyFileSync,existsSync,readdirSync} from 'node:fs';
import {dirname,resolve,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {gzipSync} from 'node:zlib';

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
function namespaceFragment(fragment,prefix) {
  return fragment.replace(/\s+id="[^"]*:[^"]*"/g,'').replace(/\bid="([^"]+)"/g,(_,id)=>`id="${prefix}${id}"`).replace(/href="#([^"]+)"/g,(_,id)=>`href="#${prefix}${id}"`);
}
let html=render(join(root,'article.mec'),join(root,'shell.html'),join(temporary,'article.html'));
let literate=read(join(root,'article.mec'));
function slot(name,content) {
  const expression=new RegExp(`<span class="mech-code-block" data-mech-source><span class="mech-code"><span class="mech-expression"><span[^>]*>${name}<\\/span><\\/span><\\/span><\\/span>`,'g');
  const matches=html.match(expression);
  if(!matches || matches.length!==1) throw new Error(`Expected one paragraph slot ${name}; found ${matches?.length||0}`);
  html=html.replace(expression,()=>content);
}
for(const [file,token,title] of [['ekf.mec','BLOGEKFSOURCE','Complete executable paper source'],['behavior.mec','BLOGBEHAVIORSOURCE','Robot behavior executed by Mech']]) {
  const prefix=file.replace('.mec','')+'-';
  const formatted=namespaceFragment(render(join(root,'source',file),join(root,'fragment.html'),join(temporary,file+'.html')),prefix);
  slot(token,`<div class="source-card"><div class="source-label">${title} · <a href="source/${file}">download .mec</a></div>${formatted}</div>`);
  literate=literate.replace(token,()=>`\`\`\`mech:${file}\n${read(join(root,'source',file))}\n\`\`\``);
  copyFileSync(join(root,'source',file),join(out,'source',file));
}
slot('BLOGLIVEDEMO',read(join(root,'demo.html')));
literate=literate.replace('BLOGLIVEDEMO','The interactive figure is provided by the browser host: [run the EKF](https://mech-lang.org/iros-r4r-2026/index.html#live-demo). Its controls, source editor, and timing implementation are included in the article repository.');
for(const [name,token,label] of [['functions','BLOGFUNCTIONS','Functions and broadcasting'],['matching','BLOGMATCHING','Pattern matching']]) {
  const file=name+'.mec', raw=read(join(root,'source',file));
  const formatted=namespaceFragment(render(join(root,'source',file),join(root,'fragment.html'),join(temporary,file+'.html')),name+'-');
  slot(token,`<section class="feature-example" data-example="${name}"><div class="source-card"><div class="source-label">${label}</div>${formatted}</div><button data-run-example="${name}" disabled>Run ${label.toLowerCase()}</button><output id="${name}-result" class="feature-result" aria-live="polite"></output><details><summary>Edit this example</summary><label for="${name}-source">${label} source</label><textarea id="${name}-source" spellcheck="false" rows="${raw.split('\n').length}">${escape(raw)}</textarea></details></section>`);
  copyFileSync(join(root,'source',file),join(out,'source',file));
  literate=literate.replace(token,()=>`\`\`\`mech:${file}\n${raw}\n\`\`\``);
}
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
for(const [file,token,label] of [['main.rs','BLOGRUSTJIT','JIT compilation and successive updates'],['build.rs','BLOGRUSTBUILD','AOT bundle producer'],['load.rs','BLOGRUSTLOAD','AOT bundle consumer']]) {
  const raw=read(join(repo,'examples/embedded_ekf',file));
  const code=raw.replace(/^\s*\/\/ POSTER[^\n]*\n/gm,'');
  writeFileSync(join(out,'source',file),code);
  slot(token,`<div class="source-card"><div class="source-label">${label} · <a href="source/${file}">${file}</a></div><pre><code>${highlightRust(code)}</code></pre></div>`);
  literate=literate.replace(token,()=>`\`\`\`rust\n${code}\`\`\``);
}
slot('BLOGPIPELINE',`<figure class="figure"><div class="wide-scroll">${read(join(root,'pipeline.svg'))}</div><figcaption>Poster architecture diagram, adapted for the article. The upper lane runs during build and activation; the lower lane repeats for each accepted or rejected turn. Telemetry values are illustrative, while the live figure above uses the actual computed values.</figcaption></figure>`);
literate=literate.replace('BLOGPIPELINE','![Build, activation and checked reactive turns](assets/pipeline.svg)');
copyFileSync(join(root,'pipeline.svg'),join(out,'assets/pipeline.svg'));
if(existsSync(join(root,'charts.mjs'))) {
  const {buildCharts}=await import('./charts.mjs');await buildCharts(join(out,'assets'));
}
for(const [file,token,caption] of [
  ['cpu','BLOGCPUCHART','Matched population and eight-worker CPU budget. The Mech and Rust rows use four-wide SIMD with fused turns. Every bar is median ± MAD from ten retained samples.'],
  ['backends','BLOGBACKENDCHART','Same archived Mech source, per-turn publication, five execution targets. JIT and AOT share numerical lowering; worker count and host capability also determine available parallelism.'],
  ['metal','BLOGMETALCHART','Cross-system native Metal campaign, collected September 24. GPU hatching distinguishes device measurements from CPU measurements.'],
  ['portable','BLOGPORTABLECHART','Application-source reuse in Mech, Taichi, and Halide. Each system selects CPU or Metal using its own backend options and schedules.']]) {
  const path=join(out,'assets',`${file}.svg`);
  slot(token,`<figure class="figure"><div class="wide-scroll">${existsSync(path)?read(path):`<p>Chart build pending: ${file}</p>`}</div><figcaption>${caption}</figcaption></figure>`);
  literate=literate.replace(token,`![${caption}](assets/${file}.svg)`);
}
if(/BLOG[A-Z]+|\{\{[A-Z_]+\}\}/.test(html)) throw new Error('Unfilled article slot.');
writeFileSync(join(out,'index.html'),html);
writeFileSync(join(out,'article.mec'),literate);
for(const file of ['app.mjs','drawing.mjs','verify.mjs','article.css']) copyFileSync(join(root,file),join(out,'assets',file));
for(const file of ['palette.css','mech-source.css','browser-compute.js']) {
  const input=join(repo,'include',file),dest=join(out,'assets',file);
  if(file==='mech-source.css') writeFileSync(dest,read(input).replace(/^@import[^\n]*\n/,''));
  else copyFileSync(input,dest);
}
for(const file of ['mech_wasm.js','mech_wasm_bg.wasm','mech_wasm.d.ts']) copyFileSync(join(repo,'src/wasm/pkg',file),join(out,'_mech/pkg',file));
writeFileSync(join(out,'_mech/pkg/mech_wasm_bg.wasm.gz'),gzipSync(readFileSync(join(out,'_mech/pkg/mech_wasm_bg.wasm')),{level:9}));
copyFileSync(join(root,'vendor/OFL.txt'),join(out,'assets/OFL.txt'));
for(const file of ['README.md','VALIDATION.md']) if(existsSync(join(root,file))) copyFileSync(join(root,file),join(out,file));
for(const file of ['apple-m1-cpu-equal-n10-2026-09-24.json','apple-m1-metal-equal-n10-2026-09-24.json','apple-m1-mech-backend-pairs-n10-2026-09-25.json']) copyFileSync(join(root,'../results',file),join(out,'evidence',file));
copyFileSync(join(root,'browser-verification.json'),join(out,'evidence/browser-verification.json'));
copyFileSync(join(repo,'LICENSE'),join(out,'LICENSE'));
const evidenceFiles=['article.mec',...readdirSync(join(out,'source')).map(x=>'source/'+x),...readdirSync(join(out,'assets')).map(x=>'assets/'+x),'_mech/pkg/mech_wasm.js','_mech/pkg/mech_wasm_bg.wasm','_mech/pkg/mech_wasm_bg.wasm.gz',...readdirSync(join(out,'evidence')).map(x=>'evidence/'+x)];
const manifest={builtAt:new Date().toISOString(),gitRevision:execFileSync('git',['rev-parse','HEAD'],{cwd:repo,encoding:'utf8'}).trim(),worktreeDirty:!!execFileSync('git',['status','--porcelain'],{cwd:repo,encoding:'utf8'}).trim(),files:Object.fromEntries(evidenceFiles.map(x=>[x,hash(join(out,x))]))};
writeFileSync(join(out,'build-manifest.json'),JSON.stringify(manifest,null,2)+'\n');
console.log(`Built ${out}/index.html`);
