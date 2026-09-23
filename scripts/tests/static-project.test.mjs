import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const moduleUrl = 'https://example.test/app/_mech/project.js';
const source = (await readFile(new URL('../../include/static-project.js', import.meta.url), 'utf8'))
  .replace(/^import .*\n/, '')
  .replaceAll('import.meta.url', JSON.stringify(moduleUrl));

async function bootstrap(manifest) {
  const fetched = [];
  const errors = [];
  const admitted = [];
  const files = new Map([
    ['/app/mech.mcfg', 'configuration'],
    ['/app/source/main.mec', 'answer := 42'],
    ['/app/source/notes.mec', 'Presentation notes.'],
    ['/app/code/main.mec', 'root artifact'],
  ]);
  const script = { dataset: {}, getAttribute: () => './_mech/project.js' };
  vm.runInNewContext(source, {
    URL,
    init: async () => {},
    document: { baseURI: 'https://example.test/app/index.html', querySelectorAll: () => [script] },
    window: { location: { href: 'https://example.test/app/index.html' }, __MECH_HOST_CONFIG: {}, addEventListener() {} },
    requestAnimationFrame() {},
    console: { error: error => errors.push(String(error)) },
    WasmProject: {
      supportsServedAuthority: () => true,
      fromServedBundle: (...args) => { admitted.push(args); return { start() {}, stop() {} }; },
    },
    fetch: async value => {
      const path = new URL(value).pathname;
      fetched.push(path);
      if (path === '/app/_mech/project-sources.json') return { ok: true, json: async () => manifest };
      return { ok: files.has(path), status: 404, statusText: 'Not Found', text: async () => files.get(path) };
    },
  });
  await new Promise(setImmediate);
  return { fetched, errors, admitted };
}

test('static bootstrap fetches served prose and only the configured root artifact', async () => {
  const result = await bootstrap({ version: 4, roots: ['main.mec'], sources: [
    { specifier: 'main.mec', url: 'source/main.mec', artifactUrl: 'code/main.mec',
      nominalOrigin: { segments: ['sample', 'main'] }, nominalPackageId: 'sample@1' },
    { specifier: 'notes.mec', url: 'source/notes.mec' },
  ] });
  assert.deepEqual(result.errors, []);
  assert.equal(result.admitted.length, 1);
  assert.deepEqual(Object.keys(result.admitted[0][2]), ['main.mec']);
  assert.equal(result.admitted[0][1]['notes.mec'], 'Presentation notes.');
  assert.equal(result.admitted[0][4]['main.mec'].nominalOrigin.segments[0], 'sample');
  assert.equal(result.admitted[0][4]['main.mec'].nominalPackageId, 'sample@1');
  assert.ok(result.fetched.includes('/app/source/notes.mec'));
  assert.ok(!result.fetched.includes('/app/code/notes.mec'));
});

test('static bootstrap rejects a configured root without its artifact', async () => {
  const result = await bootstrap({ version: 4, roots: ['main.mec'], sources: [
    { specifier: 'main.mec', url: 'source/main.mec' },
  ] });
  assert.equal(result.admitted.length, 0);
  assert.match(result.errors[0], /root artifact is missing: main.mec/);
});
