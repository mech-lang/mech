import {prepareDocumentEditor} from './document-editor.js';
import {fetchAuditWasm} from './wasm-transport.js';
import init, * as api from './pkg/mech_wasm.js';
import {applyPublication, fixtures, json, runStreamingChecks} from './streaming-checks.js';
const $ = id => document.getElementById(id), encoder = new TextEncoder();
for (const button of document.querySelectorAll('.parser-lab button')) button.disabled = true;
const documentEditor = prepareDocumentEditor();
let stream, editor, editorSnapshot, generation = 0, documentNumber = 1n, timer, running = false;
let sourceBytes, cursor, decoder, pendingChunk, accepted, mirror, progress, records, lastWork, currentView = 'stream';
const number = (id, min = 0, max = Number.MAX_SAFE_INTEGER) => Math.max(min, Math.min(max, Math.trunc(Number($(id).value) || 0)));
const reportError = error => { pause(); $('stream-status').textContent = `Operation rejected: ${error}`; $('stream-status').classList.add('error'); };
const action = fn => () => { try { fn(); } catch (error) { reportError(error); } };
function pause() { running = false; clearTimeout(timer); }
function byteToUtf16(source, offset) { return new TextDecoder().decode(encoder.encode(source).slice(0, offset)).length; }
function selectRange(textarea, source, range) {
  if (!range) return;
  textarea.focus(); textarea.setSelectionRange(byteToUtf16(source, range[0]), byteToUtf16(source, range[1]));
}
function tableRow(values, click, styles = {}) {
  const tr = document.createElement('tr'); Object.assign(tr.style, styles);
  for (const value of values) { const td = document.createElement('td'); td.textContent = value ?? ''; tr.append(td); }
  if (click) { tr.style.cursor = 'pointer'; tr.tabIndex = 0; tr.onclick = click; tr.onkeydown = event => { if (event.key === 'Enter') click(); }; }
  return tr;
}
function chart(id, series, colors) {
  const canvas = $(id), context = canvas.getContext('2d'), width = canvas.width, height = canvas.height;
  context.clearRect(0, 0, width, height); context.fillStyle = '#53666d'; context.font = '11px system-ui';
  const peak = Math.max(1, ...series.flat());
  context.fillText(`${peak.toFixed(peak < 10 ? 2 : 0)}`, 6, 15); context.fillText('0', 6, height - 19);
  context.fillText(`Retained operations: ${records.length}`, 36, height - 3);
  for (let j = 0; j < series.length; j++) {
    context.beginPath(); context.strokeStyle = colors[j]; context.lineWidth = 1.7;
    series[j].forEach((value, i) => {
      const x = 40 + i * (width - 55) / Math.max(1, series[j].length - 1), y = height - 25 - value / peak * (height - 42);
      if (i) context.lineTo(x, y); else context.moveTo(x, y);
      if (series[j].length === 1) context.lineTo(x + 2, y);
    }); context.stroke();
  }
}
function record(operation, work, coreMs, viewMs, totalMs, suffix = '') {
  const parserDelta = Number(work.parser_work + work.preview_parser_work - (lastWork?.parser_work ?? 0n) - (lastWork?.preview_parser_work ?? 0n));
  lastWork = work;
  records.push({operation, state: progress, parserDelta, coreMs, viewMs, totalMs, suffix});
  records = records.slice(-number('history-limit', 1, 10000));
  $('history').replaceChildren(...records.map(row => tableRow([row.operation, row.state, row.parserDelta, row.coreMs.toFixed(3), row.viewMs.toFixed(3), row.totalMs.toFixed(3), row.suffix])));
  $('work').textContent = JSON.stringify(work, (_, x) => typeof x === 'bigint' ? x.toString() : x, 2);
  chart('work-chart', [records.map(x => x.parserDelta)], ['#216d73']);
  chart('time-chart', [records.map(x => x.coreMs), records.map(x => x.totalMs)], ['#216d73', '#809197']);
}
function renderDiagnostics(diagnostics, ranges, textarea, source) {
  $('diagnostics').replaceChildren();
  diagnostics.forEach((diagnostic, index) => {
    const button = document.createElement('button');
    button.textContent = `${diagnostic.severity}: ${diagnostic.code} — ${diagnostic.message}`;
    button.style.cssText = 'display:block;text-align:left;width:100%;margin:6px 0';
    button.onclick = () => { selectRange(textarea, source, ranges[index]); $('detail').textContent = JSON.stringify(diagnostic, (_, x) => typeof x === 'bigint' ? x.toString() : x, 2); };
    $('diagnostics').append(button);
  });
  if (!diagnostics.length) $('diagnostics').textContent = 'No published diagnostics in this interpretation.';
}
function clearSnapshot() {
  $('tree').replaceChildren(); $('detail').textContent = 'Select a tree row or diagnostic for its structured fields.';
  $('snapshot-status').textContent = 'No current explicit snapshot. Live events and diagnostics refer to the full stream identity below.';
  $('semantic-status').textContent = 'Stages: syntax inspection; semantic checking unverified; artifact construction unverified; activation unverified; execution unverified.';
}
function consume(update, operation, totalMs) {
  applyPublication(mirror, update); progress = update.progress; currentView = 'stream'; clearSnapshot();
  $('accepted').value = accepted; $('stream-status').classList.remove('error');
  $('stream-status').textContent = `${progress}; transport ${cursor}/${sourceBytes.length} bytes; accepted ${update.source_bytes} bytes; parsed through ${update.parsed_through}; pending [${update.pending_source.join(', ')}).`;
  $('identity').textContent = json({stream: update.identity, controller_generation: generation});
  $('events').replaceChildren(...mirror.events.map((event, index) => tableRow([index, event.event, event.kind, event.id?.toString(), event.range?.join('…')], () => { selectRange($('accepted'), accepted, event.range); $('detail').textContent = json(event); })));
  const ranges = mirror.diagnostics.map(d => d.primary?.kind === 'absolute' ? [d.primary.range.start, d.primary.range.end] : null);
  renderDiagnostics(mirror.diagnostics, ranges, $('accepted'), accepted);
  record(operation, update.work, update.operation_ms, update.view_prepare_ms, totalMs, `${update.syntax.from}: ${update.syntax.old_len} → ${update.syntax.new_len}`);
  if (['Finished', 'Cancelled', 'Limited'].includes(progress)) pause();
}
function call(operation, fn) {
  const start = performance.now(), update = fn();
  consume(update, operation, performance.now() - start); return update;
}
function restart() {
  pause(); generation++; stream?.free(); editor?.free(); editor = null; editorSnapshot = null;
  $('editor-source').value = ''; $('editor-status').textContent = 'No editor session.'; $('edit-result').textContent = '';
  stream = new api.WasmSyntaxStream(documentNumber++, number('max-source', 0, 4294967295), BigInt(number('max-work')));
  sourceBytes = encoder.encode($('source').value); cursor = 0; decoder = new TextDecoder('utf-8', {fatal: true}); pendingChunk = ''; accepted = '';
  mirror = {events: [], diagnostics: []}; progress = 'NeedInput'; records = []; lastWork = null; currentView = 'stream';
  $('accepted').value = ''; $('events').replaceChildren(); $('diagnostics').replaceChildren(); clearSnapshot();
  const view = stream.resync(); applyPublication(mirror, view);
  $('identity').textContent = json({stream: view.identity, controller_generation: generation});
  $('stream-status').textContent = 'Open. No input accepted. Step transports one chunk or advances pending parser work.';
  $('stream-status').classList.remove('error'); record('restart / initial view', view.work, 0, view.view_prepare_ms, 0);
}
function step() {
  if (['Finished', 'Cancelled', 'Limited'].includes(progress)) throw new Error(`stream is ${progress}; restart to accept new input`);
  const allowance = BigInt(number('allowance', 1, 1000000));
  if (progress === 'NeedsProcessing') return call('advance', () => stream.advance(allowance));
  if (!pendingChunk && cursor >= sourceBytes.length) { pause(); $('stream-status').textContent = 'All transport bytes were accepted. Finish explicitly to establish final EOF.'; return; }
  if (!pendingChunk) {
    const end = Math.min(sourceBytes.length, cursor + number('chunk', 1, 100000));
    pendingChunk = decoder.decode(sourceBytes.slice(cursor, end), {stream: end < sourceBytes.length}); cursor = end;
  }
  if (!pendingChunk) { $('stream-status').textContent = `Transport ${cursor}/${sourceBytes.length} bytes; incomplete UTF-8 scalar buffered. No source append.`; return; }
  const start = performance.now(), update = stream.append(pendingChunk, allowance); accepted += pendingChunk; pendingChunk = '';
  consume(update, 'append', performance.now() - start);
}
function resume() {
  if (running) return; running = true; const owner = generation;
  const tick = () => {
    if (!running || owner !== generation) return;
    try { step(); } catch (error) { reportError(error); }
    if (running && owner === generation) timer = setTimeout(tick, 15);
  }; tick();
}
function showSnapshot(snapshot, label, textarea, source, preserved = new Set()) {
  $('snapshot-status').textContent = `${label}; document ${snapshot.document}, revision ${snapshot.revision}; lossless ${snapshot.lossless}; strict syntax clean ${snapshot.strictly_clean}; typed document ${snapshot.typed_document}; executable source classification ${snapshot.contains_executable_source}.`;
  $('semantic-status').textContent = 'Stages: syntax inspection; semantic checking unverified; artifact construction unverified; activation unverified; execution unverified.';
  $('tree').replaceChildren(...snapshot.tree.map(row => tableRow([`${'\u00a0'.repeat(Math.min(Number(row.depth), 18) * 2)}${row.kind}${row.token ? ' (token)' : ''}`, `${row.token ? 't' : 'n'}${row.id}`, `[${row.range.join(', ')})`], () => {
    selectRange(textarea, source, row.range);
    $('detail').textContent = json({...row, text: new TextDecoder().decode(encoder.encode(source).slice(...row.range))});
  }, !row.token && preserved.has(row.id) ? {background: '#e8f4eb'} : {})));
  renderDiagnostics(snapshot.diagnostics, snapshot.diagnostic_ranges, textarea, source);
}
function exportSnapshot(provisional) {
  pause(); const owner = generation, start = performance.now();
  const [identity, snapshot, work, operationMs] = provisional ? stream.preview() : stream.materialize();
  const totalMs = performance.now() - start;
  if (owner !== generation) return;
  currentView = 'stream'; showSnapshot(snapshot, provisional ? 'Explicit finite-prefix preview; provisional' : `Explicit ${identity.kind} export`, $('accepted'), snapshot.source);
  $('identity').textContent = json({displayed_snapshot: identity, live_stream: mirror.identity, controller_generation: generation});
  record(provisional ? 'preview' : 'materialize', work, operationMs, 0, totalMs);
  if (identity.kind === 'Limited') {
    // Limited export changes the publication baseline. Do not treat this full
    // view as a delta relative to the preceding update.
    mirror = {events: [], diagnostics: []}; applyPublication(mirror, stream.resync());
  }
}
function loadEditor() {
  pause(); editor?.free(); editor = new api.WasmSyntaxEditor(documentNumber++, accepted);
  editorSnapshot = editor.snapshot(); $('editor-source').value = editorSnapshot.source; currentView = 'editor';
  $('identity').textContent = json({displayed_editor: {document: editorSnapshot.document, revision: editorSnapshot.revision, kind: 'EditorSnapshot'}, controller_generation: generation});
  showSnapshot(editorSnapshot, 'Editor snapshot', $('editor-source'), editorSnapshot.source);
  $('editor-status').textContent = `Editor document ${editorSnapshot.document}, revision ${editorSnapshot.revision}. Full initial parse.`;
  $('edit-result').textContent = json(editorSnapshot.parse_work);
}
function edit(remove) {
  if (!editor) throw new Error('Create an editor session first.');
  pause(); const area = $('editor-source'), old = editorSnapshot.source;
  const start = encoder.encode(old.slice(0, area.selectionStart)).length, end = encoder.encode(old.slice(0, area.selectionEnd)).length;
  const timer = performance.now(), result = editor.replace(start, end, remove ? '' : $('replacement').value), totalMs = performance.now() - timer;
  editorSnapshot = result.snapshot; area.value = editorSnapshot.source; currentView = 'editor';
  const identity = {document: editorSnapshot.document, revision: editorSnapshot.revision, kind: 'EditorSnapshot'};
  $('identity').textContent = json({displayed_editor: identity, controller_generation: generation});
  showSnapshot(editorSnapshot, 'Editor snapshot after full parse and reconciliation', area, editorSnapshot.source, new Set(result.preserved_ids));
  $('editor-status').textContent = `Edited bytes [${start}, ${end}); parser work ${result.work.total_parser_steps}; reconciliation work ${result.work.reconciliation_steps}; preserved ${result.preserved_ids.length}, new ${result.new_ids.length}, removed ${result.removed_ids.length} node identities. Mech edit ${result.operation_ms.toFixed(3)} ms; total call ${totalMs.toFixed(3)} ms.`;
  const {snapshot, ...details} = result; $('edit-result').textContent = JSON.stringify(details, (_, x) => typeof x === 'bigint' ? x.toString() : x, 2);
}
for (const [index, fixture] of fixtures.entries()) { const option = document.createElement('option'); option.value = index; option.textContent = fixture.name; $('scenario').append(option); }
$('source').value = fixtures[0].source;
$('source').addEventListener('input', () => { pause(); $('stream-status').textContent = 'Source-to-transport changed. Restart to begin a new stream; accepted source still belongs to the current stream.'; });
$('load').onclick = action(() => { $('source').value = fixtures[Number($('scenario').value)].source; restart(); });
$('restart').onclick = action(restart); $('step').onclick = action(step); $('resume').onclick = action(resume); $('pause').onclick = pause;
$('finish').onclick = action(() => { pause(); call('finish', () => stream.finish(BigInt(number('allowance', 1, 1000000)))); });
$('cancel').onclick = action(() => { pause(); call('cancel', () => stream.cancel()); });
$('preview').onclick = action(() => exportSnapshot(true)); $('materialize').onclick = action(() => exportSnapshot(false));
$('editor-load').onclick = action(loadEditor); $('edit-replace').onclick = action(() => edit(false)); $('edit-delete').onclick = action(() => edit(true));
$('check').onclick = async () => { $('check').disabled = true; $('checks').textContent = 'Running actual Mech WASM checks…'; try { window.streamingChecks = await runStreamingChecks(api); window.streamingChecks.artifact = window.streamingArtifact; $('checks').textContent = JSON.stringify(window.streamingChecks, null, 2); } catch(error) { window.streamingChecks = {status:'failed', error:String(error)}; $('checks').textContent = json(window.streamingChecks); } finally { $('check').disabled = false; } };
window.addEventListener('pagehide', () => { pause(); stream?.free(); editor?.free(); stream = null; editor = null; });
window.addEventListener('pageshow', event => { if (event.persisted && window.streamingReady) restart(); });
try {
  const wasmResponse = await fetchAuditWasm();
  if (!wasmResponse.ok) throw new Error(`WASM fetch returned ${wasmResponse.status}`);
  const wasmBytes = await wasmResponse.arrayBuffer();
  const loadedHash = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', wasmBytes))).map(x => x.toString(16).padStart(2, '0')).join('');
  await init({module_or_path: wasmBytes});
  let provenance; try { provenance = await (await fetch('./artifact.json')).json(); } catch (_) { provenance = null; }
  const expectedHash = provenance?.artifacts?.['pkg/mech_wasm_bg.wasm']?.sha256;
  if (expectedHash && loadedHash !== expectedHash) throw new Error('Loaded WASM SHA-256 differs from the recorded artifact.');
  window.streamingArtifact = {loadedHash, provenance, browser: navigator.userAgent};
  $('artifact').textContent = provenance ? `Loaded Mech WASM; baseline ${provenance.source_commit ?? provenance.baseline_commit}; features ${provenance.features}; artifact SHA-256 ${provenance.artifacts?.['pkg/mech_wasm_bg.wasm']?.sha256 ?? 'see artifact.json'}.` : `Mech WASM loaded, SHA-256 ${loadedHash}. No provenance manifest was loaded; source identity is unverified.`;
  for (const button of document.querySelectorAll('.parser-lab button')) button.disabled = false;
  documentEditor.connect(api);
  restart(); window.streamingReady = true;
} catch(error) { documentEditor.fail(error); $('artifact').textContent = `WASM unavailable: ${error}. Build the demonstrated browser package before using this page.`; reportError(error); }
