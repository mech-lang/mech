import {readFileSync} from 'node:fs';
import {join} from 'node:path';

// Compose website navigation and the workshop host with the canonical blog
// template. Editorial layout, heading markup and TOC placement stay upstream.
export function blogShell(repo) {
  let template=readFileSync(join(repo,'include/blog.html'),'utf8');
  const replaceOnce=(before,after)=>{
    if(template.split(before).length!==2) throw new Error(`Canonical blog template changed near ${before.slice(0,60)}`);
    template=template.replace(before,after);
  };
  for(const [layer,slot,files] of [
    ['palette','PALETTE_STYLESHEET',['palette.css']],
    ['source','MECH_SOURCE_STYLESHEET',['mech-source.css']],
    ['mechdown','MECHDOWN_STYLESHEET',['mechdown.css']],
    ['page','PAGE_STYLESHEET',['style.css','blog.css']],
    ['repl','MECH_REPL_STYLESHEET',[]],
  ]) replaceOnce(`<style data-mech-style-layer="${layer}">{{${slot}}}</style>`,files.map(file=>`<link rel="stylesheet" data-mech-style-layer="${layer}" href="assets/${file}">`).join('\n  '));
  replaceOnce('</head>',`  <link rel="stylesheet" href="assets/article.css">
  <link rel="canonical" href="https://mech-lang.org/iros-r4r-2026/index.html">
  <meta name="description" content="Mech's numerical, embedded, reactive and heterogeneous runtime, with a bearing-only EKF running on WebAssembly and WebGPU.">
</head>`);
  replaceOnce('<body>',`<body>
  <header class="site-header">
    <div class="header-inner">
      <div class="brand"><a href="https://mech-lang.org" aria-label="Mech home"><img src="assets/logo.png" alt="Mech Programming Language" height="35"></a></div>
      <nav class="top-nav" aria-label="Primary">
        <a href="https://about.mech-lang.org">About</a>
        <a href="https://mech-lang.org/blog/" class="active" aria-current="page">Blog</a>
        <a href="https://mech-lang.org/community">Community</a>
        <a href="https://docs.mech-lang.org">Docs</a>
        <a href="https://mech-lang.org/explore/">Explore</a>
      </nav>
      <div class="header-actions"><a href="https://github.com/mech-lang/mech">GitHub</a></div>
    </div>
  </header>`);
  replaceOnce('    data-mech-console-mode="docked"\n','');
  replaceOnce('    data-mech-repl-host>','    data-mech-presentation-host>');
  replaceOnce('    mech-engine-id="0"\n','');
  replaceOnce('    data-mech-source-url-key="{{SOURCE_URL_KEY}}"\n','');
  replaceOnce('    data-mech-presentation="{{PRESENTATION}}"\n','');
  replaceOnce('    <div class="content-column">',`    <div class="content-column">
      <nav class="breadcrumbs" aria-label="Breadcrumb"><a href="https://mech-lang.org">Home</a><span class="sep">⟩</span><a href="https://mech-lang.org/blog/">Blog</a><span class="sep">⟩</span><span aria-current="page">IROS Rust for Robotics 2026</span></nav>`);
  replaceOnce('        <div class="article-layout"',`        <p class="workshop-links"><a href="#live-demo">Run the browser EKF</a><a href="source/ekf.mec">Executable EKF source</a><a href="article.mec">Article source</a><a href="build-manifest.json">Build identity</a></p>
        <div class="article-layout"`);
  replaceOnce('      </div>\n    </section>',`        <footer class="footer"><div class="footer-inner"><div class="footer-meta"><p>Mech is open source under Apache 2.0. <a href="https://github.com/mech-lang/mech">Contributions and bug reports</a> help guide development.</p></div></div></footer>
      </div>
    </section>`);
  const consoleStart=template.indexOf('\n    <div class="resize-handle"');
  const mainEnd=template.indexOf('\n  </main>',consoleStart);
  if(consoleStart<0 || mainEnd<0) throw new Error('Canonical console boundary changed');
  template=template.slice(0,consoleStart)+template.slice(mainEnd);
  const runtimeStart=template.indexOf('\n  <script type="application/x-mech-code"');
  const bodyEnd=template.indexOf('\n</body>',runtimeStart);
  if(runtimeStart<0 || bodyEnd<0) throw new Error('Canonical runtime boundary changed');
  template=template.slice(0,runtimeStart)+`
  <script type="module" data-mech-document-controller data-mech-document-mode="presentation" src="assets/document.js"></script>
  <script src="assets/browser-compute.js"></script>
  <script type="module" src="assets/app.mjs"></script>`+template.slice(bodyEnd);
  return template.replace(/[ \t]+$/gm, '');
}
