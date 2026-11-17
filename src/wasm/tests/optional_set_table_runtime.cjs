// Run against an actual wasm-pack Node package, not the native test harness:
// wasm-pack build src/wasm --dev --target nodejs --out-dir /tmp/mech-language-wasm --no-default-features --features base,u64
// node src/wasm/tests/optional_set_table_runtime.cjs /tmp/mech-language-wasm/mech_wasm.js
const assert = require('node:assert/strict');
const {resolve} = require('node:path');
const {WasmRepl} = require(resolve(process.argv[2]));

function errors(response) {
  return response.events.map(entry => entry.event)
    .filter(event => event.channel === 'diagnostic' && event.event.severity === 'error');
}

function valueText(response) {
  const value = response.events.map(entry => entry.event)
    .findLast(event => event.channel === 'repl' && event.event.kind === 'response'
      && event.event.payload.content?.kind === 'value');
  assert.ok(value, JSON.stringify(response));
  return value.event.payload.content.data.text;
}

function evaluate(source) {
  const repl = new WasmRepl();
  try {
    const response = repl.submit(source);
    assert.deepEqual(errors(response), [], source);
    assert.ok(response.result, source);
    return valueText(response);
  } finally {
    repl.shutdown();
    repl.free();
  }
}

for (const [reading, expected] of [['0.3', 'recharge'], ['0.8', 'patrol'], ['_', 'no reading']]) {
  const result = evaluate(`battery<f64?> := ${reading}
status := battery?
  ├ charge, charge < 0.4 => "recharge"
  ├ charge => "patrol"
  └ * => "no reading".
status
`);
  assert.equal(result.replace(/^"|"$/g, ''), expected);
}

assert.equal(evaluate('known := {1, 2, 2, 3}\nknown == {1, 2, 3}\n'), 'true');
assert.equal(Number(evaluate(`robots := |id<f64> ready<bool>| 1 true | 2 false | 3 true |
selected := robots[robots.ready]
selected.id[2]
`)), 3);

const repl = new WasmRepl();
try {
  assert.equal(valueText(repl.submit('accepted := 42\naccepted')), '42');
  const response = repl.submit(`battery<f64?> := 40
status := battery?
  ├ charge, charge < 0.4 => "recharge"
  ├ charge => 1234
  └ * => "no reading".
`);
  assert.equal(response.result, null);
  assert.ok(errors(response).some(event => event.event.message.includes('incompatible-match-result-kind')));
  assert.equal(valueText(repl.submit('accepted')), '42');
} finally {
  repl.shutdown();
  repl.free();
}
console.log('PASS Wasm: optional guards and absence, duplicate set literals, table masks, mixed-arm diagnostics and preserved accepted state');
