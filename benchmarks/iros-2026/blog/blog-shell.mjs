import {readFileSync} from 'node:fs';
import {join} from 'node:path';

// Compose website navigation and the workshop host with the canonical blog
// template. Editorial layout, heading markup and TOC placement stay upstream.
export function blogShell(repo) {
  let template=readFileSync(join(repo,'include/blog.html'),'utf8');
  const component=name=>readFileSync(join(repo,'benchmarks/iros-2026/blog',name),'utf8');
  const replaceOnce=(before,after)=>{
    if(template.split(before).length!==2) throw new Error(`Canonical blog template changed near ${before.slice(0,60)}`);
    template=template.replace(before,after);
  };
  for(const [layer,slot,files] of [
    ['palette','PALETTE_STYLESHEET',['palette.css']],
    ['source','MECH_SOURCE_STYLESHEET',['mech-source.css']],
    ['mechdown','MECHDOWN_STYLESHEET',['mechdown.css']],
    ['page','PAGE_STYLESHEET',['style.css','blog.css']],
    ['repl','MECH_REPL_STYLESHEET',['mech-repl.css']],
  ]) replaceOnce(`<style data-mech-style-layer="${layer}">{{${slot}}}</style>`,files.map(file=>`<link rel="stylesheet" data-mech-style-layer="${layer}" href="assets/${file}">`).join('\n  '));
  replaceOnce('</head>',`  <link rel="stylesheet" href="assets/article.css">
  <link rel="canonical" href="https://mech-lang.org/iros-r4r-2026/index.html">
  <meta name="description" content="Mech's numerical, embedded, reactive and heterogeneous runtime, with a bearing-only EKF running on WebAssembly and WebGPU.">
  <script async defer src="https://buttons.github.io/buttons.js"></script>
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
      ${component('header-actions.html')}
    </div>
  </header>`);
  replaceOnce('{{SOURCE_URL_KEY}}','article.mec');
  replaceOnce('<section class="content-shell" id="contentShell">', '<section class="content-shell workshop-footer-shell" id="contentShell">');
  replaceOnce('aria-selected="false" aria-controls="mech-console-panel-output"', 'aria-selected="true" aria-controls="mech-console-panel-output"');
  replaceOnce('aria-selected="true" aria-controls="mech-console-panel-console"', 'aria-selected="false" aria-controls="mech-console-panel-console"');
  replaceOnce('data-mech-console-panel="console">', 'data-mech-console-panel="console" hidden>');
  replaceOnce('data-mech-console-panel="output" hidden>', 'data-mech-console-panel="output">');
  replaceOnce('<div class="console-scroll" data-mech-output-panel></div>', `<div class="console-scroll" data-mech-output-panel data-mech-output-host="application">
            <div class="mech-output-region mech-output-region-application" data-mech-output-region="application">${component('demo.html')}</div>
          </div>`);
  replaceOnce('    <div class="content-column">',`    <div class="content-column">
      <nav class="breadcrumbs" aria-label="Breadcrumb"><a href="https://mech-lang.org">Home</a><span class="sep">⟩</span><a href="https://mech-lang.org/blog/">Blog</a><span class="sep">⟩</span><span aria-current="page">IROS Rust for Robotics 2026</span></nav>`);
  replaceOnce('        <section class="article-backmatter"',`        ${component('separator.html')}
        <section class="article-backmatter"`);
  // The footer owns the full scroll-pane width; its inner container keeps
  // the official blog's centered links and release-card layout.
  replaceOnce('      </div>\n    </section>',`      </div>
      ${component('footer.html')}
    </section>`);
  replaceOnce(`<script
    type="module"
    data-mech-document-controller
    data-mech-wasm-module="{{WASM_MODULE_URL}}">
{{DOCUMENT_SCRIPT}}
  </script>`,`<script type="module" data-mech-document-controller data-mech-wasm-module="./runtime.mjs" src="assets/document.js"></script>`);
  replaceOnce('</body>',`
  <script src="assets/browser-compute.js"></script>
  <script type="module" src="assets/app.mjs"></script>
</body>`);
  return template.replace(/[ \t]+$/gm, '');
}
