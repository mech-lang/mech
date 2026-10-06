import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

await import('../../include/browser-compute.js');
const { Session } = globalThis.MechBrowserCompute;

const moduleUrl = 'https://example.test/app/_mech/project.js';
const source = (await readFile(new URL('../../include/static-project.js', import.meta.url), 'utf8'))
  .replace(/^import .*\n/gm, '')
  .replaceAll('import.meta.url', JSON.stringify(moduleUrl));

async function bootstrap(manifest, project = {}, options = {}) {
  const fetched = [];
  const errors = [];
  const admitted = [];
  const listeners = new Map();
  const frames = [];
  const sessions = [];
  const events = [];
  const resources = [];
  let adapterRequests = 0;
  let starts = 0;
  let stops = 0;
  const files = new Map([
    ['/app/mech.mcfg', 'configuration'],
    ['/app/source/main.mec', 'answer := 42'],
    ['/app/source/notes.mec', 'Presentation notes.'],
    ['/app/source/filters/index.mec', 'selected := 42'],
    ['/app/code/main.mec', 'root artifact'],
  ]);
  const script = { dataset: {}, getAttribute: () => './_mech/project.js' };
  const ownerWindow = { location: { href: 'https://example.test/app/index.html' }, __MECH_HOST_CONFIG: {},
    addEventListener: (name, listener) => listeners.set(name, listener),
    removeEventListener: (name, listener) => { assert.equal(listeners.get(name), listener); listeners.delete(name); } };
  const compute = {
    Device: { create: async (...args) => {
      events.push('device-create');
      if (options.deviceFailure) throw options.deviceFailure;
      const resource = options.resource?.(...args);
      resources.push(resource);
      return resource;
    } },
    Session: class extends Session {
      constructor(options) {
        super(options);
        sessions.push(this);
      }
      retire() {
        events.push('retire');
        super.retire();
      }
    },
  };
  ownerWindow.MechBrowserCompute = compute;
  vm.runInNewContext(source, {
    URL,
    init: async () => {},
    document: { baseURI: 'https://example.test/app/index.html', querySelectorAll: () => [script],
      documentElement: { getBoundingClientRect: () => ({ left: 10, top: 20, width: 200, height: 100 }) } },
    window: ownerWindow,
    navigator: { gpu: { requestAdapter: async () => { adapterRequests++; return options.adapter === null ? null : {}; } } },
    MechBrowserCompute: compute,
    requestAnimationFrame: callback => frames.push(callback),
    console: { error: error => errors.push(String(error)) },
    WasmProject: {
      supportsServedAuthority: () => true,
      supportsServedDocumentResolutions: () => true,
      supportsServedDocumentProvenance: () => true,
      supportsCompute: () => options.supportsCompute === true,
      fromServedDocuments: (...args) => {
        admitted.push(args);
        const controller = options.projectFactory?.(ownerWindow.__MECH_GPU_AVAILABLE, admitted.length) ?? project;
        return { start() { starts++; events.push('start'); }, stop() { stops++; events.push('stop'); }, frame() {}, ...controller };
      },
    },
    fetch: async value => {
      const path = new URL(value).pathname;
      fetched.push(path);
      if (path === '/app/_mech/project-sources.json') return { ok: true, json: async () => manifest };
      return { ok: files.has(path), status: 404, statusText: 'Not Found', text: async () => files.get(path) };
    },
  });
  await new Promise(setImmediate);
  return { fetched, errors, admitted, listeners, frames, events, resources, sessions,
    adapterRequests: () => adapterRequests, starts: () => starts, stops: () => stops };
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

const computeManifest = (requestedBackend = 'auto') => ({
  physicalRevision: 'sha256:static-test-plan',
  requestedBackend,
});
const computeCommand = (sequence = 1) => ({
  dispatch: true,
  acknowledgementRequired: true,
  dispatchToken: `11:${sequence}`,
  requestedOutputs: ['estimate'],
});
const computeProject = (overrides = {}) => ({
  computeManifest: () => computeManifest(),
  computeBackend: () => 'wgpu',
  computeGeneration: () => '11',
  isComputeCommandTokenCurrent: () => true,
  completeComputeCommand() {},
  ...overrides,
});
function computeResource(completion, events) {
  return {
    physicalRevision: 'sha256:static-test-plan',
    device: { lost: new Promise(() => {}) },
    setRequestedOutputs(outputs) { events.push(['outputs', [...outputs]]); },
    submit(command, activeBuffer) {
      events.push(['submit', command.dispatchToken, activeBuffer]);
      return { outputIndex: 1, completion: Promise.resolve() };
    },
    finish() { return completion; },
    dispose() { events.push('dispose'); },
  };
}

test('static profiles without compute do not probe or construct GPU transport', async () => {
  const result = await bootstrap(rootManifest);
  assert.deepEqual(result.errors, []);
  assert.equal(result.adapterRequests(), 0);
  assert.equal(result.resources.length, 0);
  assert.equal(result.sessions.length, 0);
  assert.equal(result.starts(), 1);
});

test('static scalar compute starts without a browser compute session', async () => {
  const result = await bootstrap(rootManifest, computeProject({ computeBackend: () => 'cpu-scalar' }), {
    supportsCompute: true,
  });
  assert.deepEqual(result.errors, []);
  assert.equal(result.resources.length, 0);
  assert.equal(result.sessions.length, 0);
  assert.equal(result.starts(), 1);
});

test('static auto compute uses scalar execution when no adapter is available', async () => {
  const capabilities = [];
  const result = await bootstrap(rootManifest, {}, {
    supportsCompute: true,
    adapter: null,
    projectFactory: gpuAvailable => {
      capabilities.push(gpuAvailable);
      return computeProject({ computeBackend: () => gpuAvailable ? 'wgpu' : 'cpu-scalar' });
    },
  });
  assert.deepEqual(result.errors, []);
  assert.deepEqual(capabilities, [false]);
  assert.equal(result.adapterRequests(), 1);
  assert.equal(result.sessions.length, 0);
  assert.equal(result.starts(), 1);
});

test('static WebGPU commands use the real session and gate frames until completion', async () => {
  let release;
  const completion = new Promise(resolve => { release = resolve; });
  const resourceEvents = [];
  const acknowledgements = [];
  let frameCalls = 0;
  const result = await bootstrap(rootManifest, computeProject({
    frame() { frameCalls++; return { computeCommand: frameCalls === 1 ? computeCommand() : null }; },
    completeComputeCommand: value => acknowledgements.push(value),
  }), { supportsCompute: true, resource: () => computeResource(completion, resourceEvents) });
  assert.deepEqual(result.errors, []);
  assert.equal(result.adapterRequests(), 1);
  assert.equal(result.sessions.length, 1);
  assert.ok(result.sessions[0] instanceof Session);
  result.frames.shift()();
  assert.equal(result.sessions[0].pending, true);
  assert.equal(frameCalls, 1);
  result.frames.shift()();
  assert.equal(frameCalls, 1, 'a pending dispatch must not advance the resident runtime');
  assert.deepEqual(resourceEvents, [['outputs', ['estimate']], ['submit', '11:1', 0]]);
  release({ outputs: [{ name: 'estimate', values: [3, 7] }], integrity: null });
  await new Promise(setImmediate);
  assert.equal(result.sessions[0].pending, false);
  assert.deepEqual(acknowledgements, [{ version: 1, token: '11:1', status: 'completed',
    outputs: [{ name: 'estimate', values: [3, 7] }] }]);
  result.frames.shift()();
  assert.equal(frameCalls, 2);
});

test('static compute failure rejects its command and stops without replay', async () => {
  let reject;
  const completion = new Promise((resolve, rejectCompletion) => { reject = rejectCompletion; });
  const resourceEvents = [];
  const acknowledgements = [];
  let frameCalls = 0;
  const result = await bootstrap(rootManifest, computeProject({
    frame() { frameCalls++; return { computeCommand: computeCommand() }; },
    completeComputeCommand: value => acknowledgements.push(value),
  }), { supportsCompute: true, resource: () => computeResource(completion, resourceEvents) });
  result.frames.shift()();
  reject(new Error('static submitted work failed'));
  await new Promise(setImmediate);
  assert.equal(result.stops(), 1);
  assert.match(result.errors[0], /static submitted work failed/);
  assert.equal(acknowledgements[0].status, 'failed');
  assert.equal(acknowledgements[0].token, '11:1');
  result.frames.shift()();
  assert.equal(frameCalls, 1);
  assert.equal(result.frames.length, 0);
  assert.equal(result.admitted.length, 1, 'a submitted failure must never reconstruct scalar execution');
});

test('static immediate dispatch refusal stops its controller exactly once', async () => {
  const resourceEvents = [];
  const acknowledgements = [];
  const result = await bootstrap(rootManifest, computeProject({
    frame: () => ({ computeCommand: computeCommand() }),
    completeComputeCommand: value => acknowledgements.push(value),
  }), { supportsCompute: true, resource: () => ({
    ...computeResource(Promise.resolve({ outputs: [], integrity: null }), resourceEvents),
    submit() { throw new Error('static queue submission refused'); },
  }) });
  result.frames.shift()();
  assert.equal(result.stops(), 1, 'the failure hook and frame catch must share one teardown');
  assert.equal(acknowledgements.length, 1);
  assert.equal(acknowledgements[0].status, 'failed');
  assert.match(result.errors[0], /static queue submission refused/);
  assert.equal(resourceEvents.filter(event => event === 'dispose').length, 1);
  assert.equal(result.admitted.length, 1);
  assert.equal(result.frames.length, 0);
});

test('static unload retires its pending session before stopping the controller', async () => {
  let release;
  const completion = new Promise(resolve => { release = resolve; });
  const resourceEvents = [];
  const acknowledgements = [];
  const result = await bootstrap(rootManifest, computeProject({
    frame: () => ({ computeCommand: computeCommand() }),
    completeComputeCommand: value => acknowledgements.push(value),
  }), { supportsCompute: true, resource: () => computeResource(completion, resourceEvents) });
  result.frames.shift()();
  result.listeners.get('beforeunload')();
  assert.equal(result.sessions[0].retired, true);
  assert.ok(result.events.indexOf('retire') < result.events.indexOf('stop'));
  assert.ok(!resourceEvents.includes('dispose'), 'submitted resource remains alive until completion');
  release({ outputs: [], integrity: null });
  await new Promise(setImmediate);
  assert.deepEqual(acknowledgements, [], 'a retired generation cannot publish late completion');
  assert.equal(resourceEvents.filter(event => event === 'dispose').length, 1);
  result.frames.shift()();
  assert.equal(result.frames.length, 0);
});

test('static unload during device creation disposes the late resource without restarting', async () => {
  let release;
  const creation = new Promise(resolve => { release = resolve; });
  const resourceEvents = [];
  const result = await bootstrap(rootManifest, computeProject(), {
    supportsCompute: true,
    resource: () => creation,
  });
  assert.equal(result.starts(), 0);
  assert.equal(result.events.filter(event => event === 'device-create').length, 1);
  result.listeners.get('beforeunload')();
  assert.equal(result.stops(), 1);
  release(computeResource(Promise.resolve({ outputs: [], integrity: null }), resourceEvents));
  await new Promise(setImmediate);
  assert.deepEqual(result.errors, []);
  assert.equal(result.starts(), 0, 'asynchronous construction cannot revive an unloaded project');
  assert.equal(result.stops(), 1);
  assert.equal(result.sessions.length, 0);
  assert.equal(result.frames.length, 0);
  assert.equal(resourceEvents.filter(event => event === 'dispose').length, 1);
});

test('static auto compute may reconstruct scalar execution only before start', async () => {
  const capabilities = [];
  const result = await bootstrap(rootManifest, {}, {
    supportsCompute: true,
    deviceFailure: new Error('static device construction failed'),
    projectFactory: gpuAvailable => {
      capabilities.push(gpuAvailable);
      return computeProject({ computeBackend: () => gpuAvailable ? 'wgpu' : 'cpu-scalar' });
    },
  });
  assert.deepEqual(result.errors, []);
  assert.deepEqual(capabilities, [true, false]);
  assert.equal(result.admitted.length, 2);
  assert.equal(result.starts(), 1);
  assert.equal(result.stops(), 1);
  assert.equal(result.sessions.length, 0);
  assert.ok(result.events.indexOf('device-create') < result.events.indexOf('stop'));
  assert.ok(result.events.indexOf('stop') < result.events.indexOf('start'));
});

test('static explicit WebGPU refuses device failure without scalar reconstruction', async () => {
  const result = await bootstrap(rootManifest, computeProject({ computeManifest: () => computeManifest('wgpu') }), {
    supportsCompute: true,
    deviceFailure: new Error('static explicit device construction failed'),
  });
  assert.equal(result.admitted.length, 1);
  assert.equal(result.starts(), 0);
  assert.equal(result.stops(), 1);
  assert.equal(result.sessions.length, 0);
  assert.match(result.errors[0], /static explicit device construction failed/);
});
