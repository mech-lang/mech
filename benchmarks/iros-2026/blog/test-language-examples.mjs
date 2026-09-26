// Real public WASM REPL tests, not a JavaScript model of the language.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const pkg = resolve(process.argv[2] || resolve(here, '../../../src/wasm/pkg'));
const { initSync, WasmRepl } = await import(pathToFileURL(resolve(pkg, 'mech_wasm.js')));
initSync({ module: readFileSync(resolve(pkg, 'mech_wasm_bg.wasm')) });
const behavior = readFileSync(resolve(here, 'source/behavior.mec'), 'utf8');
const rejected = response => JSON.stringify(response).includes('"severity":"error"');
function submit(repl, source) {
  const response = repl.submit(source);
  assert(!rejected(response), JSON.stringify(response));
  assert.equal(response.pending, false);
  return response.result?.inlineHtml;
}

const repl = new WasmRepl();
try {
  submit(repl, behavior);
  let cases = 0;
  for (const mode of ['paused', 'patrol', 'fault']) {
    for (const event of ['pause', 'run', 'rejected', 'reset']) {
      const expected = event === 'reset' ? 'paused'
        : event === 'rejected' || mode === 'fault' ? 'fault'
          : event === 'pause' ? 'paused' : 'patrol';
      assert.equal(submit(repl, `#Robot(:${mode}, :${event})`), `:${expected}`);
      cases++;
    }
  }
  for (const source of ['#Robot(:unknown, :run)', '#Robot(:paused, :unknown)', '#Robot(0, 1)']) {
    assert(rejected(repl.submit(source)), `undeclared value admitted: ${source}`);
    assert.equal(submit(repl, '#Robot(:paused, :run)'), ':patrol', 'failed admission must not poison later calls');
  }
  const typed = submit(repl, 'current<mode> := :patrol\ncurrent');
  assert.equal(typed, ':patrol');
  assert(rejected(repl.submit('invalid<mode> := :unknown')));
  console.log(`PASS: ${cases} named-mode/event cases, wildcard precedence, unknown/numeric rejection, typed enum source and recovery.`);
} finally {
  repl.free();
}

for (const [name, expected] of [['functions', 0.9668146928204135], ['matching', '&quot;correct&quot;']]) {
  const repl = new WasmRepl();
  try {
    const result = submit(repl, readFileSync(resolve(here, `source/${name}.mec`), 'utf8'));
    if (typeof expected === 'number') assert(Math.abs(Number(result) - expected) < 1e-12);
    else assert.equal(result, expected);
  } finally {
    repl.free();
  }
}
console.log('PASS: standalone function and pattern-matching source examples.');

// State payload construction must use the same declared enum schemas as arm
// matching, including literal next states and partially typed payloads.
const fsmCases = [
  ['initial', ':State(mode<mode>)', ':State(:ready)', ':State(:ready) => :ready'],
  ['next', ':Start | :State(mode<mode>)', ':Start', ':Start -> :State(:ready)\n  :State(:ready) => :ready'],
  ['async', ':Start | :State(mode<mode>)', ':Start', ':Start ~> :State(:ready)\n  :State(:ready) => :ready'],
  ['mixed', ':State(mode<mode>, payload)', ':State(:ready, (:busy, :ready))', ':State(:ready, (left, right)) => right'],
];
for (const [name, states, initial, arms] of fsmCases) {
  const repl = new WasmRepl();
  try {
    const source = `<mode> := :ready | :busy\n#Check() => <mode> := | ${states}.\n#Check() -> ${initial}\n  ${arms}\n.\n#Check()`;
    assert.equal(submit(repl, source), ':ready', `${name} state enum construction`);
  } finally {
    repl.free();
  }
}
console.log('PASS: enum payloads in initial, next, asynchronous and mixed typed/untyped states.');
