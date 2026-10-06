// Behavioral browser checks against the same Mech WASM package as the UI.
// JavaScript controls transport and verifies records; all grammar operations run in Rust.
export const json = value => JSON.stringify(value, (_, x) => typeof x === 'bigint' ? x.toString() : x);
const assert = (condition, message) => { if (!condition) throw new Error(message); };
export function applyPublication(mirror, update) {
  if (mirror.identity) {
    assert(mirror.identity.document === update.identity.document, 'publication document ownership');
    assert(update.identity.revision >= mirror.identity.revision, 'publication revision order');
    assert(update.identity.interpretation >= mirror.identity.interpretation, 'publication interpretation order');
  }
  for (const [key, delta, records] of [['events', update.syntax, update.events], ['diagnostics', update.diagnostics, update.diagnostic_records]]) {
    const indexes = ['from', 'old_len', 'new_len'].map(name => {
      const value = Number(delta[name]);
      assert(Number.isSafeInteger(value) && value >= 0 && BigInt(value) === BigInt(delta[name]), `${key}: exact browser array index`);
      return value;
    });
    const [from, oldLength, newLength] = indexes;
    assert(mirror[key].length === oldLength, `${key}: predecessor length mismatch ${mirror[key].length}/${oldLength}`);
    assert(from <= oldLength && from <= newLength, `${key}: invalid suffix`);
    mirror[key].splice(from, mirror[key].length - from, ...records);
    assert(mirror[key].length === newLength, `${key}: published length mismatch`);
  }
  mirror.identity = update.identity;
  return update;
}
const treeShape = snapshot => snapshot.tree.map(({kind, range, flags, depth, token}) => ({kind, range, flags, depth, token}));
const diagnostics = snapshot => snapshot.diagnostics.map((d, i) => ({code: d.code, severity: d.severity, phase: d.phase, expected: d.expected, recovery: d.recovery, range: snapshot.diagnostic_ranges[i]}));
export const fixtures = [
  {name: 'Definition', source: 'x := 1\n', kind: 'VariableDefine', clean: true},
  {name: 'Interval syntax', source: '1..=10\n', kind: 'RangeExpression', clean: true},
  {name: 'Incomplete matrix', source: 'x := [1,', clean: false},
  {name: 'Recovery before valid material', source: 'x := [1, +, 2]\ny := 3\n', clean: false},
  {name: 'Unicode', source: 'x := "é👩‍💻"\n', clean: true},
  {name: 'Mixed document', source: 'Title\n=====\n\nSome text.\n\n```mech\nx := 1\n```\n', kind: 'CodeBlock', clean: true},
];
export async function runStreamingChecks(api) {
  const {WasmSyntaxStream, WasmSyntaxEditor} = api;
  const results = []; let schedules = 0, publications = 0;
  function streamAt(source, split, allowance = 13n) {
    const stream = new WasmSyntaxStream(1001n, 100000, 10000000n);
    const mirror = {events: [], diagnostics: []};
    const consume = update => { publications++; return applyPublication(mirror, update); };
    function drain(update) {
      consume(update); let turns = 0;
      while (update.progress === 'NeedsProcessing') {
        assert(++turns < 100000, 'termination bound'); update = consume(stream.advance(allowance));
      }
      return update;
    }
    drain(stream.append(source.slice(0, split), allowance));
    drain(stream.append(source.slice(split), allowance));
    const final = drain(stream.finish(allowance));
    assert(final.progress === 'Finished', 'explicit EOF finalizes');
    const [identity, snapshot] = stream.materialize();
    assert(identity.kind === 'Finalized', 'final identity kind');
    assert(snapshot.source === source && snapshot.lossless, 'exact source and losslessness');
    stream.free(); schedules++; return snapshot;
  }
  for (const fixture of fixtures) {
    const reference = new WasmSyntaxEditor(1001n, fixture.source);
    const oneShot = reference.snapshot(); reference.free();
    assert(oneShot.strictly_clean === fixture.clean, `${fixture.name}: independent clean/error expectation`);
    if (fixture.kind) assert(oneShot.tree.some(x => x.kind === fixture.kind), `${fixture.name}: expected grammar node ${fixture.kind}`);
    if (fixture.name === 'Recovery before valid material') {
      const bytes = new TextEncoder().encode(fixture.source);
      assert(oneShot.tree.some(x => x.kind === 'VariableDefine' && new TextDecoder().decode(bytes.slice(...x.range)).includes('y := 3')), 'recovery retains the following definition');
    }
    let split = 0;
    for (const char of [''].concat(Array.from(fixture.source))) {
      split += char.length;
      const actual = streamAt(fixture.source, split);
      assert(json(treeShape(actual)) === json(treeShape(oneShot)), `${fixture.name}: tree at scalar cut ${split}`);
      assert(json(diagnostics(actual)) === json(diagnostics(oneShot)), `${fixture.name}: diagnostics at scalar cut ${split}`);
    }
    results.push({case: fixture.name, status: 'passed', scalarCuts: Array.from(fixture.source).length + 1});
    await new Promise(resolve => setTimeout(resolve, 0));
  }
  const large = Array.from({length: 24}, (_, i) => `x${i} := ${i}\n`).join('');
  const largeEditor = new WasmSyntaxEditor(1500n, large), largeReference = largeEditor.snapshot(); largeEditor.free();
  assert(largeReference.tree.filter(x => x.kind === 'VariableDefine').length === 24, 'independent definition count for larger fixture');
  for (const seed of [1, 7, 42, 20261006]) {
    let random = seed >>> 0, offset = 0;
    const scheduled = new WasmSyntaxStream(1500n, 100000, 10000000n), mirror = {events: [], diagnostics: []};
    const drain = update => {
      applyPublication(mirror, update); publications++; let turns = 0;
      while (update.progress === 'NeedsProcessing') { assert(++turns < 100000, 'seeded drain termination'); update = scheduled.advance(31n); applyPublication(mirror, update); publications++; }
    };
    while (offset < large.length) {
      random = (Math.imul(random, 1664525) + 1013904223) >>> 0;
      const end = Math.min(large.length, offset + 1 + random % 23);
      drain(scheduled.append(large.slice(offset, end), 31n)); offset = end;
    }
    drain(scheduled.finish(31n));
    const [, final] = scheduled.materialize();
    assert(json(treeShape(final)) === json(treeShape(largeReference)), `seed ${seed}: larger tree`);
    assert(final.source === large && final.strictly_clean && final.lossless, `seed ${seed}: losslessness`);
    schedules++; scheduled.free();
  }
  results.push({case: 'Larger input with recorded seeded chunk schedules', status: 'passed', seeds: [1, 7, 42, 20261006]});
  // Transport splits scalar encodings. A streaming decoder buffers incomplete UTF-8.
  const unicode = fixtures.find(x => x.name === 'Unicode').source;
  const decoder = new TextDecoder('utf-8', {fatal: true}); let decoded = '';
  const transported = new WasmSyntaxStream(1750n, 10000, 100000n), transportedMirror = {events: [], diagnostics: []};
  const consumeTransport = update => {
    applyPublication(transportedMirror, update); publications++; let turns = 0;
    while (update.progress === 'NeedsProcessing') { assert(++turns < 100000, 'transport drain termination'); update = transported.advance(31n); applyPublication(transportedMirror, update); publications++; }
  };
  for (const byte of new TextEncoder().encode(unicode)) {
    const text = decoder.decode(Uint8Array.of(byte), {stream: true}); decoded += text;
    if (text) consumeTransport(transported.append(text, 31n));
  }
  const tail = decoder.decode(); decoded += tail;
  if (tail) consumeTransport(transported.append(tail, 31n));
  consumeTransport(transported.finish(31n));
  const [, transportedSnapshot] = transported.materialize(); transported.free();
  assert(decoded === unicode && transportedSnapshot.source === unicode && transportedSnapshot.lossless, 'single-byte UTF-8 transport preserves source');
  results.push({case: 'UTF-8 byte transport into Mech', status: 'passed'});
  const stream = new WasmSyntaxStream(2002n, 1000, 100000n);
  const mirror = {events: [], diagnostics: []};
  let update = applyPublication(mirror, stream.append('x := [', 1n)); publications++;
  assert(update.progress === 'NeedsProcessing', 'small allowance suspends');
  let turns = 0;
  while (update.progress === 'NeedsProcessing') { assert(++turns < 100000, 'preview drain termination'); update = applyPublication(mirror, stream.advance(31n)); publications++; }
  const [previewId, preview] = stream.preview();
  assert(previewId.kind === 'FinitePreview' && !preview.strictly_clean, 'provisional incomplete input is not clean');
  update = applyPublication(mirror, stream.cancel()); publications++;
  assert(update.identity.kind === 'Cancelled' && json(update.identity) !== json(previewId), 'cancel invalidates preview identity');
  let rejected = false; try { stream.append('1]', 99n); } catch (_) { rejected = true; }
  assert(rejected, 'cancelled stream rejects appends'); stream.free();
  const resumed = new WasmSyntaxStream(2003n, 1000, 10000000n), resumedMirror = {events: [], diagnostics: []};
  const drainResumed = value => {
    applyPublication(resumedMirror, value); publications++; let turns = 0;
    while (value.progress === 'NeedsProcessing') { assert(++turns < 100000, 'continuation drain termination'); value = resumed.advance(31n); applyPublication(resumedMirror, value); publications++; }
    return value;
  };
  drainResumed(resumed.append('x := [', 31n));
  const [finiteIdentity, finite] = resumed.preview();
  drainResumed(resumed.append('1]\n', 31n)); drainResumed(resumed.finish(31n));
  const [finalIdentity, finalized] = resumed.materialize();
  const finiteIds = new Set(finite.tree.filter(x => !x.token).map(x => x.id));
  assert(finalized.strictly_clean && finalized.source === 'x := [1]\n', 'resume completes open matrix: ' + json({finalIdentity, source:finalized.source, clean:finalized.strictly_clean, diagnostics:finalized.diagnostics, work:finalized.parse_work}));
  assert(!finalized.tree.filter(x => !x.token).some(x => finiteIds.has(x.id)), 'preview and live node IDs do not alias');
  assert(json(finiteIdentity) !== json(finalIdentity), 'finalization changes full interpretation identity'); resumed.free();
  results.push({case: 'Suspension, continuation, preview isolation, cancellation', status: 'passed'});
  const limited = new WasmSyntaxStream(3003n, 100, 1n);
  const limitedMirror = {events: [], diagnostics: []};
  let limit = applyPublication(limitedMirror, limited.append('x := 1\n', 100n)); publications++;
  turns = 0;
  while (limit.progress === 'NeedsProcessing') { assert(++turns < 100000, 'limit drain termination'); limit = applyPublication(limitedMirror, limited.advance(100n)); publications++; }
  assert(limit.progress === 'Limited', 'cumulative parser-work limit');
  const [limitedIdentity, limitedSnapshot] = limited.materialize();
  assert(limitedIdentity.kind === 'Limited' && limitedSnapshot.lossless && !limitedSnapshot.strictly_clean, 'limited export is lossless and rejected for execution');
  const resynchronized = {events: [], diagnostics: []}; applyPublication(resynchronized, limited.resync());
  assert(resynchronized.identity.kind === 'Limited', 'limited materialization explicitly resynchronizes'); limited.free();
  const small = new WasmSyntaxStream(3004n, 1, 1000n);
  rejected = false; try { small.append('xx', 1n); } catch (_) { rejected = true; }
  assert(rejected, 'source-size rejection');
  assert(small.resync().source_bytes === 0, 'rejected source not accepted'); small.free();
  results.push({case: 'Parser and source resource limits', status: 'passed'});
  const editor = new WasmSyntaxEditor(4004n, 'x := 1\ny := 2\n');
  const changed = editor.replace(5, 6, '3');
  assert(changed.snapshot.source === 'x := 3\ny := 2\n', 'replacement exact');
  assert(changed.preserved_ids.length > 0 && changed.removed_ids.length > 0 && changed.new_ids.length > 0, 'identity reuse and replacement');
  assert(changed.work.document_fallbacks === 1n, 'editor performs full-document parse');
  assert(changed.work.total_parser_steps > 0n && changed.work.reconciliation_steps > 0n, 'separate parse and reconciliation counters');
  const broken = editor.replace(5, 6, '['); assert(!broken.snapshot.strictly_clean && broken.added_diagnostics.length > 0, 'introduced error');
  const repaired = editor.replace(5, 6, '4'); assert(repaired.snapshot.strictly_clean && repaired.removed_diagnostics.length > 0, 'repair removes diagnostics');
  const inserted = editor.replace(0, 0, '-- note\n'); assert(inserted.snapshot.source.startsWith('-- note\n'), 'insertion');
  const removed = editor.replace(0, 8, ''); assert(removed.snapshot.source === 'x := 4\ny := 2\n', 'deletion'); editor.free();
  results.push({case: 'Replacement, identity reuse, repair, insertion, deletion', status: 'passed'});
  return {status: 'passed', schedules, publications, results, environment: navigator.userAgent};
}
