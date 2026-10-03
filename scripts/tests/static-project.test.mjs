import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const moduleUrl = 'https://example.test/app/_mech/project.js';
const source = (await readFile(new URL('../../include/static-project.js', import.meta.url), 'utf8'))
  .replace(/^import .*\n/, '')
  .replaceAll('import.meta.url', JSON.stringify(moduleUrl));

async function bootstrap(manifest, project = {}) {
  const fetched = [];
  const errors = [];
  const admitted = [];
  const listeners = new Map();
  const frames = [];
  let stops = 0;
  const files = new Map([
    ['/app/mech.mcfg', 'configuration'],
    ['/app/source/main.mec', 'answer := 42'],
    ['/app/source/notes.mec', 'Presentation notes.'],
    ['/app/source/filters/index.mec', 'selected := 42'],
    ['/app/code/main.mec', 'root artifact'],
  ]);
  const script = { dataset: {}, getAttribute: () => './_mech/project.js' };
  vm.runInNewContext(source, {
    URL,
    init: async () => {},
    document: { baseURI: 'https://example.test/app/index.html', querySelectorAll: () => [script],
      documentElement: { getBoundingClientRect: () => ({ left: 10, top: 20, width: 200, height: 100 }) } },
    window: { location: { href: 'https://example.test/app/index.html' }, __MECH_HOST_CONFIG: {},
      addEventListener: (name, listener) => listeners.set(name, listener),
      removeEventListener: (name, listener) => { assert.equal(listeners.get(name), listener); listeners.delete(name); } },
    requestAnimationFrame: callback => frames.push(callback),
    console: { error: error => errors.push(String(error)) },
    WasmProject: {
      supportsServedAuthority: () => true,
      supportsServedDocumentResolutions: () => true,
      supportsServedDocumentProvenance: () => true,
      fromServedDocuments: (...args) => { admitted.push(args); return { start() {}, stop() { stops++; }, frame() {}, ...project }; },
    },
    fetch: async value => {
      const path = new URL(value).pathname;
      fetched.push(path);
      if (path === '/app/_mech/project-sources.json') return { ok: true, json: async () => manifest };
      return { ok: files.has(path), status: 404, statusText: 'Not Found', text: async () => files.get(path) };
    },
  });
  await new Promise(setImmediate);
  return { fetched, errors, admitted, listeners, frames, stops: () => stops };
}

test('static bootstrap fetches served prose and transports retained resolutions', async () => {
  const resolutions = [
    { referrer: 'main.mec', specifier: './filters', target: 'filters/index.mec' },
  ];
  const result = await bootstrap({ version: 4, roots: ['main.mec'], sources: [
    { specifier: 'main.mec', url: 'source/main.mec', documentUrl: 'code/main.mec' },
    { specifier: 'notes.mec', url: 'source/notes.mec' },
    { specifier: 'filters/index.mec', url: 'source/filters/index.mec' },
  ], resolutions });
  assert.deepEqual(result.errors, []);
  assert.equal(result.admitted.length, 1);
  assert.deepEqual(Object.keys(result.admitted[0][2]), ['main.mec']);
  assert.equal(result.admitted[0][1]['notes.mec'], 'Presentation notes.');
  assert.deepEqual(result.admitted[0][4], resolutions);
  assert.ok(result.fetched.includes('/app/source/notes.mec'));
  assert.ok(!result.fetched.includes('/app/code/notes.mec'));
});

const rootManifest = { version: 4, roots: ['main.mec'], sources: [
  { specifier: 'main.mec', url: 'source/main.mec', documentUrl: 'code/main.mec',
    nominalOrigin: { segments: ['project', 'main.mec'] }, nominalPackageId: 'package-hash' },
], resolutions: [] };

test('static bootstrap transports complete nominal provenance', async () => {
  const result = await bootstrap(rootManifest);
  assert.deepEqual(result.errors, []);
  assert.equal(JSON.stringify(result.admitted[0][5]), JSON.stringify({
    'main.mec': { nominalOrigin: { segments: ['project', 'main.mec'] }, nominalPackageId: 'package-hash' },
  }));
});

test('static pointer events deliver normalized samples and stop cleanly', async () => {
  const samples = [];
  const result = await bootstrap(rootManifest, { hasPointerInput: () => true, pointerInput: (...args) => samples.push(args) });
  const emit = (name, event) => result.listeners.get(name)?.(event);
  emit('pointermove', { clientX: 160, clientY: 45, timeStamp: 100 });
  emit('pointerdown', { button: 1, clientX: 110, clientY: 70, timeStamp: 110 });
  emit('pointerdown', { button: 0, clientX: 110, clientY: 70, timeStamp: 116 });
  emit('pointerup', { button: 1, clientX: 110, clientY: 70, timeStamp: 120 });
  emit('pointerup', { button: 0, clientX: 210, clientY: 120, timeStamp: 132 });
  emit('pointerdown', { button: 0, clientX: 10, clientY: 20, timeStamp: 148 });
  emit('pointercancel', { clientX: 10, clientY: 20, timeStamp: 164 });
  assert.deepEqual(samples, [[0.5, 0.5, false, 0], [0, 0, true, 0.016], [1, -1, false, 0.016], [-1, 1, true, 0.016], [-1, 1, false, 0.016]]);
  emit('beforeunload', {});
  assert.equal(result.stops(), 1);
  assert.deepEqual([...result.listeners.keys()], ['beforeunload']);
  result.frames.shift()();
  assert.equal(result.frames.length, 0);
});

test('static projects without pointer hosts install no pointer listeners', async () => {
  const result = await bootstrap(rootManifest, { hasPointerInput: () => false });
  assert.deepEqual([...result.listeners.keys()], ['beforeunload']);
});

test('pointer submission and frame failure detach listeners and stop the driver', async () => {
  for (const failure of ['pointerInput', 'frame']) {
    const result = await bootstrap(rootManifest, { hasPointerInput: () => true, pointerInput() {},
      [failure]: () => { throw new Error(`failed ${failure}`); } });
    if (failure === 'pointerInput') result.listeners.get('pointermove')({ clientX: 110, clientY: 70, timeStamp: 100 });
    else result.frames.shift()();
    assert.equal(result.stops(), 1);
    assert.match(result.errors[0], new RegExp(`failed ${failure}`));
    assert.deepEqual([...result.listeners.keys()], ['beforeunload']);
  }
});

test('static bootstrap rejects a configured root without its document', async () => {
  const result = await bootstrap({ version: 4, roots: ['main.mec'], sources: [
    { specifier: 'main.mec', url: 'source/main.mec' },
  ], resolutions: [] });
  assert.equal(result.admitted.length, 0);
  assert.match(result.errors[0], /root document is missing: main.mec/);
});
