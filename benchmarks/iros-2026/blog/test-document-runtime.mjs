import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const pagePath = resolve(process.argv[2] || join(root, 'dist/index.html'));
const out = dirname(pagePath);
const html = readFileSync(pagePath, 'utf8');
for (const id of ['source-editor', 'functions-source', 'matching-source', 'compile', 'restore']) {
  assert(!new RegExp(`id="${id}"`).test(html), `read-only article regained editor control ${id}`);
}
function embedded(type) {
  const escaped = type.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = html.match(new RegExp(`<script\\b[^>]*type="${escaped}"[^>]*>([\\s\\S]*?)<\\/script>`));
  assert(match?.[1].trim(), `missing embedded ${type}`);
  return match[1].trim();
}
const encoded = embedded('application/x-mech-code');
const bundle = JSON.parse(Buffer.from(embedded('application/x-mech-source-bundle'), 'base64').toString('utf8'));
assert.equal(bundle.version, 2);
const sourceMap = Object.fromEntries(bundle.sources.map(({ specifier, source }) => [specifier, source]));
assert.equal(typeof sourceMap[bundle.rootSpecifier], 'string');

// Exercise the shipped browser initializer with local responses, including
// concurrent starts by the document controller and the kernel host.
const requests = [];
const originalFetch = globalThis.fetch;
globalThis.fetch = async input => {
  const url = new URL(input);
  assert.equal(url.protocol, 'file:', 'the runtime smoke must stay local');
  requests.push(url.pathname);
  return new Response(readFileSync(url), { headers: { 'Content-Type': url.pathname.endsWith('.wasm') ? 'application/wasm' : 'application/gzip' } });
};
let document;
try {
  const runtime = await import(pathToFileURL(join(out, 'assets/runtime.mjs')));
  const first = runtime.default();
  assert.strictEqual(runtime.default(), first, 'concurrent hosts must share the initialization promise');
  await first;
  assert.equal(requests.length, 1, 'one runtime download for both hosts');
  for (const name of ['WasmDocument', 'WasmKernel', 'replInputAction', 'replStepLimit']) {
    assert.equal(typeof runtime[name], 'function', `missing actual runtime export ${name}`);
  }
  document = runtime.WasmDocument.fromEncodedWithBundle(encoded, bundle.rootSpecifier, sourceMap, bundle.resolutions);
  assert.equal(document.replSource(), sourceMap[bundle.rootSpecifier], 'REPL and rendered document must retain the same source');
  const outputIds = [...html.matchAll(/class="mech-block-output" id="(\d+):(\d+)"/g)];
  const residentOutputs = outputIds.filter(([, , namespace]) => namespace === '0');
  const kernelOutputs = outputIds.filter(([, , namespace]) => namespace !== '0');
  assert.equal(residentOutputs.length, 3, 'behavior, functions, and matching belong to the resident document');
  assert.equal(kernelOutputs.length, 1, 'the named EKF block belongs to the live kernel host');
  assert.equal((html.match(/data-workshop-kernel-output/g) || []).length, 1,
    'exactly one native output must be connected to the live kernel');
  assert(html.includes(`id="${kernelOutputs[0][1]}:${kernelOutputs[0][2]}" data-workshop-kernel-output`),
    'the live kernel marker must preserve the native EKF output identity');
  for (const [, id] of residentOutputs) {
    const output = document.renderedOutput(BigInt(id));
    assert(output?.blockHtml, `native document output ${id} did not render`);
  }
  assert.equal(document.renderedOutput(BigInt(kernelOutputs[0][1])), null,
    'the document must not claim the independently hosted EKF output');
  const kernel = runtime.WasmKernel.fromSource(readFileSync(join(out, 'source/ekf.mec'), 'utf8'), {
    bearing: new Float32Array([-0.55]), v: [1], w: [0.015],
  }, ['state', 'covariance']);
  try {
    kernel.turn({ bearing: new Float32Array([-0.55]), v: [1], w: [0.015] });
    assert(Array.from(kernel.stateSample('state', 0)).every(Number.isFinite));
    assert.equal(kernel.stateSample('covariance', 0).length, 9);
  } finally {
    kernel.free();
  }
  function invoke(source, symbol) {
    const response = document.replInvoke(source);
    assert(response && !JSON.stringify(response).includes('"severity":"error"'),
      `resident REPL rejected ${source}: ${JSON.stringify(response)}`);
    return document.renderedDocumentValue(symbol)?.inlineHtml;
  }
  assert.equal(invoke('iros-runtime-probe := 40 + 2\niros-runtime-probe', 'iros-runtime-probe'), '42',
    'the REPL must update the actual document resident state');
  assert(Math.abs(Number(invoke('iros-heading-probe := wrap-angle(7.25)\niros-heading-probe', 'iros-heading-probe')) - 0.9668146928204135) < 1e-12,
    'the REPL must retain the article function and imported math library');
  assert.equal(invoke('iros-mode-probe := #Robot(0, 1)\niros-mode-probe', 'iros-mode-probe'), '1',
    'the REPL must retain the article state machine');
  assert.equal(invoke('iros-decision-probe := decision\niros-decision-probe', 'iros-decision-probe'), '&quot;correct&quot;',
    'the REPL must retain the article pattern matching result');
  console.log(`PASS: shared v0.4 WASM initialization, resident document REPL, ${residentOutputs.length} resident output blocks, and the separate EKF kernel.`);
} finally {
  document?.stop();
  document?.free();
  globalThis.fetch = originalFetch;
}
