import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const pagePath = resolve(process.argv[2] || join(root, 'dist/index.html'));
const out = dirname(pagePath);
const html = readFileSync(pagePath, 'utf8');
for (const id of ['source-editor', 'functions-source', 'matching-source', 'apply-source', 'restore-source', 'compile', 'restore']) {
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
assert.equal(sourceMap[bundle.rootSpecifier], readFileSync(join(out, 'article.mec'), 'utf8'),
  'the encoded runtime bundle must retain the downloadable expanded article source');

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
  for (const name of ['WasmDocument', 'WasmKernel', 'WasmSceneProgram', 'replInputAction', 'replStepLimit']) {
    assert.equal(typeof runtime[name], 'function', `missing actual runtime export ${name}`);
  }
  document = runtime.WasmDocument.fromEncodedWithBundle(encoded, bundle.rootSpecifier, sourceMap, bundle.resolutions);
  assert.equal(document.replSource(), sourceMap[bundle.rootSpecifier], 'REPL and rendered document must retain the same source');
  const versionResponse = document.replInvoke(':version');
  const versionRows = versionResponse.events
    .filter(envelope => envelope.event.channel === 'repl')
    .map(envelope => envelope.event.event)
    .filter(event => event.kind === 'response' && event.payload?.content?.kind === 'table')
    .flatMap(event => event.payload.content.data.rows);
  assert.equal(versionRows.length, 13, 'the maintained browser profile must report all installed product, library, and host versions');
  for (const [component, kind, version] of versionRows) {
    assert.equal(version, '0.4.0-beta', `${kind} ${component} reports stale compiled Rust metadata`);
  }
  const outputIds = [...html.matchAll(/class="mech-block-output" id="(\d+):(\d+)"/g)];
  const residentOutputs = outputIds.filter(([, , namespace]) => namespace === '0');
  const kernelOutput = html.match(/class="mech-block-output" id="(\d+):(\d+)" data-workshop-kernel-output/);
  assert(kernelOutput, 'the live EKF output must retain its native namespace');
  const kernelOutputs = outputIds.filter(([, , namespace]) => namespace === kernelOutput[2]);
  const hostedOutputs = outputIds.filter(([, , namespace]) => namespace !== '0');
  const sceneOutputs = hostedOutputs.filter(([, , namespace]) => namespace !== kernelOutput[2]);
  assert.equal(residentOutputs.length, 3, 'behavior, functions, and matching belong to the resident document');
  assert.equal(kernelOutputs.length, 4, 'the four named EKF sections belong to the live kernel host');
  assert.equal(sceneOutputs.length, 1, 'the extracted scene table belongs to its separate Mech scene host');
  assert.equal((html.match(/data-workshop-kernel-output/g) || []).length, 1,
    'exactly one native output must be connected to the live kernel');
  assert.equal((html.match(/data-workshop-kernel-listing/g) || []).length, kernelOutputs.length,
    'every EKF listing must retain its separate kernel marker');
  assert.equal((html.match(/data-workshop-scene-listing/g) || []).length, sceneOutputs.length,
    'the scene table must retain its separate scene-host marker');
  assert(kernelOutputs.some(([, id, namespace]) => html.includes(`id="${id}:${namespace}" data-workshop-kernel-output`)),
    'the live kernel marker must preserve the native EKF output identity');
  for (const [, id] of residentOutputs) {
    const output = document.renderedOutput(BigInt(id));
    assert(output?.blockHtml, `native document output ${id} did not render`);
  }
  for (const [, id] of hostedOutputs) {
    assert.equal(document.renderedOutput(BigInt(id)), null,
      'the document must not claim independently hosted EKF or scene output');
  }
  const instances = 256;
  const inputs = { bearing: new Float32Array(instances).fill(-0.55), v: [1], w: [0.015] };
  const liveInputs = { bearing: inputs.bearing, u: [1, 0.015, 1], m: [140, 12] };
  const historicalSource = readFileSync(join(root, 'evidence/ekf-before-symmetry-stabilization.mec'), 'utf8');
  assert.equal(createHash('sha256').update(historicalSource).digest('hex'),
    'cefe87b0ee184f1f30c34c66e626948f6d43236c8449dca0054a68e9e5cd932f',
    'historical Unicode equivalence fixture must remain unchanged');
  const kernel = runtime.WasmKernel.fromSource(historicalSource, inputs, ['μ', 'Σ']);
  const liveKernel = runtime.WasmKernel.fromSource(readFileSync(join(out, 'source/ekf.mec'), 'utf8'), liveInputs, ['μ', 'Σ']);
  let referenceKernel, candidateKernel, simulation;
  try {
    const previousSource = readFileSync(join(root, '../../../src/wasm/tests/fixtures/paper-ekf.mec'), 'utf8');
    assert.equal(createHash('sha256').update(previousSource).digest('hex'),
      'a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2',
      'the source-equivalence reference must not be rewritten with the presentation source');
    referenceKernel = runtime.WasmKernel.fromSource(previousSource, inputs, ['state', 'covariance']);
    candidateKernel = runtime.WasmKernel.fromSource(readFileSync(join(root, 'evidence/ekf-symmetry-candidate.mec'), 'utf8'), inputs, ['state', 'covariance']);
    const snapshot = (current, names) => names.map(name => {
      const values = current.state(name);
      return Array.from(new Uint32Array(values.buffer, values.byteOffset, values.length));
    });
    const currentSnapshot = () => snapshot(kernel, ['μ', 'Σ']);
    const referenceSnapshot = () => snapshot(referenceKernel, ['state', 'covariance']);
    const compare = label => assert.deepEqual(currentSnapshot(), referenceSnapshot(), label);
    const liveSnapshot = () => snapshot(liveKernel, ['μ', 'Σ']);
    const candidateSnapshot = () => snapshot(candidateKernel, ['state', 'covariance']);
    const compareStabilized = label => {
      assert.deepEqual(liveSnapshot(), candidateSnapshot(), label);
      const covariance = liveKernel.state('Σ');
      const words = new Uint32Array(covariance.buffer, covariance.byteOffset, covariance.length);
      for (let offset = 0; offset < covariance.length; offset += 9) {
        for (const [left, right] of [[1, 3], [2, 6], [5, 7]]) {
          assert.equal(words[offset + left], words[offset + right], 'accepted covariance must be bitwise symmetric');
        }
      }
    };
    compare('source revision must preserve initial exported-state bits');
    compareStabilized('live Unicode stabilization must match the tested candidate initially');
    const rowMajorCovariance = values => [
      values[0], values[3], values[6],
      values[1], values[4], values[7],
      values[2], values[5], values[8],
    ];
    simulation = runtime.WasmSceneProgram.fromSource(readFileSync(join(out, 'source/scene.mec'), 'utf8'), {
      commit: 0, 'lane-indices': Float64Array.from({ length: instances }, (_, index) => index + 1),
      velocity: 1, omega: 0.015, noise: 0.02, 'camera-index': 1, 'camera-range': 100,
      estimate: Array.from(liveKernel.stateSample('μ', 0)),
      covariance: { rows: 3, columns: 3, values: rowMajorCovariance(liveKernel.stateSample('Σ', 0)) },
    });
    simulation.turn({ commit: 0 });
    for (let turn = 0; turn < 40; turn++) {
      simulation.turn({ commit: 0 });
      const bearing = Float32Array.from(simulation.readNumbers('readings'));
      assert.equal(bearing.length, instances, 'Mech must generate one camera reading per filter');
      assert.equal(simulation.readNumbers('measurement-visible')[0], 1,
        'the historical comparison must keep its original visible landmark');
      // Archived kernels retain their original public inputs. Only the live
      // kernel accepts the control/visibility vector and selected landmark.
      const observation = { bearing, v: [1], w: [0.015] };
      referenceKernel.turn(observation);
      kernel.turn(observation);
      liveKernel.turn({ bearing, u: [1, 0.015, 1], m: Float32Array.from(simulation.readNumbers('camera-position')) });
      candidateKernel.turn(observation);
      compare(`source revision changed exported-state bits at accepted turn ${turn + 1}`);
      compareStabilized(`live stabilization differs from tested candidate at turn ${turn + 1}`);
      simulation.turn({
        commit: 1, estimate: Array.from(liveKernel.stateSample('μ', 0)),
        covariance: { rows: 3, columns: 3, values: rowMajorCovariance(liveKernel.stateSample('Σ', 0)) },
      });
      assert.equal(simulation.readNumbers('accepted-turns')[0], turn + 1,
        'the Mech scene must advance once for each accepted filter turn');
    }
    const scene = simulation.scene();
    const estimateTrail = scene.line_strips.find(strip => strip.id === 'estimate-path');
    assert(estimateTrail, 'the published scene must include the Mech trail table');
    assert.deepEqual(estimateTrail.stroke_dasharray, [0.1, 1.5]);
    for (const invalid of [NaN, Infinity, -Infinity]) {
      const accepted = currentSnapshot();
      const stabilizedAccepted = liveSnapshot();
      const bearing = new Float32Array(instances).fill(-0.55);
      bearing[instances - 1] = invalid;
      assert.throws(() => referenceKernel.turn({ bearing }), /finite-candidate!/);
      assert.throws(() => kernel.turn({ bearing }), /finite-candidate!/);
      assert.throws(() => liveKernel.turn({ bearing }), /finite-candidate!/);
      assert.throws(() => candidateKernel.turn({ bearing }), /finite-candidate!/);
      assert.deepEqual(referenceSnapshot(), accepted, 'reference rejection must retain every accepted state bit');
      assert.deepEqual(currentSnapshot(), accepted, 'presentation rejection must retain every accepted state bit');
      assert.deepEqual(liveSnapshot(), stabilizedAccepted, 'live stabilized rejection must retain every accepted state bit');
      assert.deepEqual(candidateSnapshot(), stabilizedAccepted, 'diagnostic candidate rejection must retain every accepted state bit');
      bearing[instances - 1] = -0.55;
      referenceKernel.turn({ bearing });
      kernel.turn({ bearing });
      liveKernel.turn({ bearing });
      candidateKernel.turn({ bearing });
      compare(`source revision changed recovery after ${invalid}`);
      compareStabilized(`live stabilization differs on recovery after ${invalid}`);
    }
    assert(Array.from(kernel.state('μ')).every(Number.isFinite));
    assert.equal(kernel.stateSample('Σ', 0).length, 9);
  } finally {
    simulation?.stop();
    simulation?.free();
    referenceKernel?.free();
    candidateKernel?.free();
    liveKernel.free();
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
  assert.equal(invoke('iros-column-probe := [0; 1; 1]\niros-column-probe', 'iros-column-probe'), '[0 1 1]&#39;',
    'resident column vectors must use the compact transposed-row output form');
  assert.equal(invoke('iros-scalar-probe<f32> := 0.5\niros-scalar-probe', 'iros-scalar-probe'), '0.5',
    'resident scalar values must omit inferable type suffixes');
  assert(Math.abs(Number(invoke('iros-heading-probe := wrap-angle(7.25)\niros-heading-probe', 'iros-heading-probe')) - 0.9668146928204135) < 1e-12,
    'the REPL must retain the article function and imported math library');
  for (const [label, mode, event, expected] of [
    ['pause-initial', ':paused', ':pause', ':paused'],
    ['run', ':paused', ':run', ':patrol'],
    ['pause-patrol', ':patrol', ':pause', ':paused'],
    ['reject', ':patrol', ':rejected', ':fault'],
    ['fault-latch', ':fault', ':run', ':fault'],
    ['reset', ':fault', ':reset', ':paused'],
  ]) {
    const symbol = `iros-mode-${label}-probe`;
    assert.equal(invoke(`${symbol} := #Robot(${mode}, ${event})\n${symbol}`, symbol), expected,
      `the resident typed-atom state machine must preserve ${label}`);
  }
  assert.equal(invoke('iros-decision-probe := decision\niros-decision-probe', 'iros-decision-probe'), '&quot;correct&quot;',
    'the REPL must retain the article pattern matching result');
  console.log(`PASS: 13 compiled v0.4.0-beta component versions, shared WASM initialization, resident document REPL, ${residentOutputs.length} resident output blocks, Mech camera/scene execution, historical EKF equivalence and 256-filter stabilized-source equivalence/rollback/recovery.`);
} finally {
  document?.stop();
  document?.free();
  globalThis.fetch = originalFetch;
}
