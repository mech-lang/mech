import {renderMechValue} from './human-data.js';
import {renderDocumentPreview} from './document-preview.js';
const examples = {
  document: 'Calculation\n===========\n\nThe result is published by the final expression.\n\n```mech\nanswer := 6 * 7\nanswer\n```\n',
  matrix: 'matrix := [1 2;\n3 4]\nmatrix\n',
  syntax: 'x := [1, +, 2]\ny := [3, *, 4]\n',
  recovery: 'Calculation\n===========\n\nThe result is published by the final expression.\n\n```mech\nanswer := [1 2 3 4 5\nanswer\n```\n\n1. Section One\n------------------------------\n\nThis is the first section.\n',
  semantic: 'Café 👩‍💻\n=========\n\nAn interval admits values from 1 through 9.\n\n```mech\nanswer := 10⟨u8:1..10⟩\nanswer\n```\n',
};
const encoder = new TextEncoder();
const graphemes = new Intl.Segmenter(undefined, {granularity:'grapheme'});
const json = value => JSON.stringify(value, (_, item) => typeof item === 'bigint' ? item.toString() : item, 2);
function byteOffsets(source) {
  const map = new Uint32Array(encoder.encode(source).length + 1);
  let byte = 0, unit = 0;
  for (const scalar of source) {
    const size = encoder.encode(scalar).length;
    for (let i = 0; i < size; i++) map[byte++] = unit;
    unit += scalar.length;
    map[byte] = unit;
  }
  return map;
}
function tokenClass(row, parents) {
  const kinds = parents.map(parent => parent.kind);
  if (kinds.includes('Comment')) return 'comment';
  if (kinds.some(kind => /StringLiteral|Utf8String/.test(kind))) return 'string';
  if (kinds.some(kind => /^(Title|Subtitle|Heading)$/.test(kind))) return 'heading';
  if (kinds.includes('KindAnnotation')) return 'type';
  if (kinds.some(kind => /^(Number|RealNumber|IntegerLiteral|UntypedInteger|FloatLiteral)$/.test(kind))) return 'number';
  if (kinds.includes('Identifier')) {
    const index = kinds.lastIndexOf('Identifier');
    return kinds[index - 1] === 'FunctionCall' ? 'function' : 'identifier';
  }
  if (/True|False/.test(row.kind)) return 'boolean';
  if (kinds.some(kind => /Operator$/.test(kind))) return 'operator';
  if (/Grave|CodeBlockSigil/.test(row.kind)) return 'fence';
  if (/Operator|Plus|Dash|Star|Slash|Equal|Colon|Left|Right|Comma|Exclamation|Caret|Ampersand|Pipe|Percent/.test(row.kind)) return 'operator';
  return '';
}
function syntaxSpans(snapshot, offsets) {
  const parents = [], spans = [];
  for (const row of snapshot.tree) {
    const depth = Number(row.depth);
    parents.length = depth;
    if (row.token) {
      const style = tokenClass(row, parents.filter(Boolean));
      if (style) spans.push({start: offsets[row.range[0]], end: offsets[row.range[1]], style});
    }
    parents[depth] = row;
  }
  return spans.filter(span => Number.isFinite(span.start) && span.end > span.start).sort((a, b) => a.start - b.start);
}
function position(source, offset) {
  const preceding = source.slice(0, offset);
  const lines = preceding.split('\n');
  return {line: lines.length, column: Array.from(graphemes.segment(lines.at(-1))).length + 1};
}
export function prepareDocumentEditor() {
  const $ = id => document.getElementById(id);
  const area = $('document-source'), colors = $('document-colors'), layer = $('document-highlight');
  let api, parser, snapshot, parsedSource, colorSpans = [], diagnostics = [], diagnosticSource = null;
  let parseTimer, composing = false, busy = false, runNumber = 0, revision = 0, documentNumber = 500000n;
  area.value = examples.document;
  const lifecycle = new AbortController(), eventOptions = {signal: lifecycle.signal};
  const status = (message, state = '') => {$('document-status').textContent = message; $('document-status').dataset.state = state;};
  function syncScroll() {
    layer.style.height = `${area.clientHeight}px`; layer.style.width = `${area.clientWidth}px`;
    $('document-gutter').style.height = `${area.clientHeight}px`;
    layer.scrollTop = area.scrollTop; layer.scrollLeft = area.scrollLeft;
    $('document-gutter').scrollTop = area.scrollTop;
  }
  function caret() {
    const at = position(area.value, area.selectionStart);
    $('document-position').textContent = `Line ${at.line}, column ${at.column}`;
  }
  function paint() {
    const source = area.value, stops = new Set([0, source.length]), marks = diagnostics.filter(d => d.range && d.severity === 'error');
    for (const span of colorSpans) {stops.add(span.start); stops.add(span.end);}
    for (const diagnostic of marks) {stops.add(diagnostic.start); stops.add(diagnostic.end);}
    const boundaries = [...stops].filter(n => Number.isInteger(n) && n >= 0 && n <= source.length).sort((a, b) => a - b);
    const fragment = document.createDocumentFragment();
    let token = 0;
    for (let i = 0; i < boundaries.length; i++) {
      const start = boundaries[i], end = boundaries[i + 1];
      for (const diagnostic of marks.filter(d => d.start === start && d.end === start)) {
        const marker = document.createElement('span'); marker.className = 'error-point'; marker.title = diagnostic.message;
        marker.dataset.start = start; marker.dataset.end = start; fragment.append(marker);
      }
      if (end === undefined) break;
      while (token < colorSpans.length && colorSpans[token].end <= start) token++;
      const span = document.createElement('span');
      if (colorSpans[token]?.start <= start && colorSpans[token].end >= end) span.className = `syntax-${colorSpans[token].style}`;
      span.textContent = source.slice(start, end);
      const errors = marks.filter(d => d.start < end && d.end > start);
      if (errors.length) {
        const mark = document.createElement('mark'); mark.className = 'error-mark'; mark.title = errors.map(d => d.message).join('\n');
        mark.dataset.start = start; mark.dataset.end = end; mark.append(span); fragment.append(mark);
      } else fragment.append(span);
    }
    if (!source || source.endsWith('\n')) fragment.append(document.createTextNode(' '));
    colors.replaceChildren(fragment);
    area.parentElement.classList.add('highlighted');
    const errorLines = new Set();
    for (const diagnostic of marks) {
      const first = position(source, diagnostic.start).line, last = position(source, Math.max(diagnostic.start, diagnostic.end - 1)).line;
      for (let line = first; line <= last; line++) errorLines.add(line);
    }
    $('document-gutter').replaceChildren(...source.split('\n').map((_, i) => {
      const line = document.createElement('span'); line.textContent = i + 1;
      if (errorLines.has(i + 1)) line.className = 'line-error';
      return line;
    }));
    syncScroll(); caret();
  }
  function parse() {
    clearTimeout(parseTimer);
    if (!api || composing) return;
    const source = area.value;
    if (source === parsedSource && snapshot) return;
    if (!parser) {parser = new api.WasmSyntaxEditor(documentNumber++, source); snapshot = parser.snapshot();}
    else snapshot = parser.replace(0, encoder.encode(parsedSource).length, source).snapshot;
    parsedSource = source;
    colorSpans = syntaxSpans(snapshot, byteOffsets(source));
    try {
      $('document-preview').replaceChildren(renderDocumentPreview(parser.renderHtml()));
      delete $('document-preview').dataset.renderFailed;
    } catch (error) {
      $('document-preview').textContent = 'Complete the document’s syntax to update its preview.';
      $('document-preview').dataset.renderFailed = 'true';
    }
    if (snapshot.strictly_clean) {paint();renderPreviewDiagnostics();}
    else showDiagnostics(syntaxDiagnostics(), true);
  }
  function scheduleParse() {
    clearTimeout(parseTimer);
    parseTimer = setTimeout(() => {try {parse();} catch (error) {status(`Syntax inspection failed: ${error}`, 'error');}}, 100);
  }
  function changed() {
    revision++; diagnosticSource = null; diagnostics = []; colorSpans = [];
    $('document-errors').textContent = 'Compile to check the current document.'; $('document-error-count').textContent = '0';
    $('document-output').textContent = 'Document changed. Compile to see its output.';
    $('document-run-label').textContent = 'Awaiting run'; $('document-stages').replaceChildren(); $('document-result').textContent = 'Awaiting compilation.';
    $('document-preview-status').textContent = 'Updating preview';
    status(api ? 'Document changed. Ctrl+Enter compiles and runs the current source.' : 'Loading Mech. You can continue editing.');
    paint(); scheduleParse();
  }
  function locate(diagnostic) {
    if (!diagnostic.range || diagnosticSource !== area.value) return;
    area.focus(); area.setSelectionRange(diagnostic.start, diagnostic.end);
    const lineHeight = parseFloat(getComputedStyle(area).lineHeight);
    area.scrollTop = Math.max(0, (diagnostic.line - 1) * lineHeight - area.clientHeight / 3);
    syncScroll();
    const marker = colors.querySelector(`[data-start="${diagnostic.start}"]`);
    if (marker) area.scrollLeft = Math.max(0, area.scrollLeft + marker.getBoundingClientRect().left - area.getBoundingClientRect().left - area.clientWidth / 2);
    syncScroll(); caret();
  }
  function syntaxDiagnostics() {
    return snapshot.diagnostics.map((diagnostic, index) => ({
      ...diagnostic, range: snapshot.diagnostic_ranges[index],
      presentation: snapshot.diagnostic_presentations[index],
    }));
  }
  function diagnosticHeading(diagnostic, preview = false) {
    const button = document.createElement(diagnostic.range ? 'button' : 'div');
    button.className = preview ? 'preview-diagnostic-heading' : `diagnostic-row diagnostic-${diagnostic.severity}`;
    if (diagnostic.range) {button.type = 'button';button.onclick = () => locate(diagnostic);}
    const severity = document.createElement('span');severity.className = 'diagnostic-severity';
    severity.textContent = diagnostic.severity === 'info' ? 'Info' : diagnostic.severity === 'warning' ? 'Warning' : 'Error';
    const location = document.createElement('strong');
    location.textContent = diagnostic.range ? `Line ${diagnostic.line}, column ${diagnostic.column}` : diagnostic.phase || 'Program diagnostic';
    const message = document.createElement('span');message.className = 'diagnostic-message';message.textContent = diagnostic.message || 'Compilation diagnostic';
    button.append(severity, location, message);
    if (diagnostic.range) button.setAttribute('aria-label', `${severity.textContent}: ${message.textContent}. ${location.textContent}. Locate in source.`);
    return button;
  }
  function sourceReport(text, className) {
    const pre = document.createElement('pre');pre.className = className;
    const code = document.createElement('code');
    // Compiler text is passive content, including arbitrary source strings.
    for (const line of text.trimEnd().split('\n')) {
      const row = document.createElement('span');row.className = 'diagnostic-source-line';row.textContent = line + '\n';
      if (/^\s*\|.*\^/.test(line)) row.classList.add('diagnostic-underline');
      if (/^\s*= help:/.test(line)) row.classList.add('diagnostic-help');
      code.append(row);
    }
    pre.append(code);return pre;
  }
  function diagnosticCard(diagnostic, preview = false) {
    const card = document.createElement('article');
    card.className = preview ? `preview-diagnostic preview-diagnostic-${diagnostic.severity}` : `diagnostic-card diagnostic-${diagnostic.severity}`;
    card.append(diagnosticHeading(diagnostic, preview));
    if (!preview && diagnostic.code) {
      const code = document.createElement('div');code.className = 'diagnostic-code';code.textContent = diagnostic.code;card.append(code);
    }
    const presentation = diagnostic.presentation;
    const text = preview ? presentation?.excerpt : presentation?.report?.split('\n').slice(1).join('\n');
    if (text) card.append(sourceReport(text, preview ? 'preview-error-source' : 'diagnostic-source'));
    return card;
  }
  function renderPreviewDiagnostics() {
    const preview = $('document-preview');
    preview.querySelectorAll('.preview-diagnostic,.preview-diagnostic-summary').forEach(node => node.remove());
    preview.querySelectorAll('.preview-region-error,.preview-region-warning').forEach(node => node.classList.remove('preview-region-error','preview-region-warning'));
    const issues = diagnosticSource === area.value ? diagnostics.filter(d => d.severity === 'error' || d.severity === 'warning') : [];
    const errors = issues.filter(d => d.severity === 'error').length;
    const warnings = issues.length - errors;
    preview.dataset.state = errors ? 'error' : warnings ? 'warning' : '';
    const previewStatus = $('document-preview-status');
    previewStatus.dataset.state = preview.dataset.state;
    const count = [errors ? `${errors} error${errors === 1 ? '' : 's'}` : '', warnings ? `${warnings} warning${warnings === 1 ? '' : 's'}` : ''].filter(Boolean).join(' · ');
    previewStatus.textContent = preview.dataset.renderFailed ? `Preview pending${count ? ' · ' + count : ''}` : `Current source${!snapshot.strictly_clean ? ' · recovered syntax' : ''}${count ? ' · ' + count : ''}`;
    if (!issues.length) return;
    const summary = document.createElement('div');summary.className = 'preview-diagnostic-summary';summary.setAttribute('role','status');
    const heading = document.createElement('strong');heading.textContent = count;
    const detail = document.createElement('span');detail.textContent = 'Source diagnostics appear at the affected document regions.';
    summary.append(heading,detail);preview.prepend(summary);
    const regions = [...preview.querySelectorAll('[data-mech-start][data-mech-end]')].map(element => ({element,start:Number(element.dataset.mechStart),end:Number(element.dataset.mechEnd)})).filter(region => Number.isInteger(region.start) && Number.isInteger(region.end) && region.end >= region.start);
    for (const diagnostic of issues) {
      const region = diagnostic.range ? regions.filter(region => diagnostic.range[0] >= region.start && diagnostic.range[1] <= region.end).sort((a,b) => (a.end-a.start)-(b.end-b.start))[0] : null;
      const card = diagnosticCard(diagnostic, true);
      if (region) {region.element.classList.add(`preview-region-${diagnostic.severity}`);region.element.insertBefore(card, [...region.element.children].find(child => !child.classList.contains('preview-diagnostic')) || null);}
      else {card.classList.add('preview-diagnostic-unmapped');summary.after(card);}
    }
  }
  function showDiagnostics(records, live = false) {
    const source = area.value, offsets = byteOffsets(source);
    diagnosticSource = source;
    diagnostics = records.map(record => {
      const range = Array.isArray(record.range) && record.range.length === 2 && record.range.every(n => Number.isInteger(n) && n >= 0 && n < offsets.length) && record.range[1] >= record.range[0] ? record.range : null;
      const start = range ? offsets[range[0]] : null, end = range ? offsets[range[1]] : null;
      const severity = record.code === 'source-semantics/empty-document' ? 'info' : ({information:'info',hint:'info'}[record.severity] || record.severity || 'error');
      return {...record, severity, range, start, end, ...(range ? position(source, start) : {})};
    });
    $('document-error-count').textContent = diagnostics.length ? ['error','warning','info'].map(severity => {const count = diagnostics.filter(d => d.severity === severity).length;return count ? `${count} ${severity === 'info' ? 'info' : severity + (count === 1 ? '' : 's')}` : null;}).filter(Boolean).join(' · ') : '0';
    $('document-errors').replaceChildren(...diagnostics.map(diagnostic => diagnosticCard(diagnostic)));
    if (!diagnostics.length) $('document-errors').textContent = live ? 'Syntax inspection completed with 0 diagnostics.' : 'Compilation completed with 0 diagnostics.';
    paint();renderPreviewDiagnostics();
  }

  function showOutputs(result) {
    const container = $('document-output'); container.replaceChildren();
    if (diagnostics.length && diagnostics.every(d => d.severity === 'info')) {container.textContent = 'Document inspection completed. Add executable Mech code to produce program output.';return;}
    if (result.stages?.execution === 'awaiting external inputs') {
      const p = document.createElement('p'); p.textContent = `Compilation completed. Execution requires external inputs: ${(result.inputs || []).map(input => input.name).join(', ')}.`;container.append(p);return;
    }
    if (result.stages?.execution !== 'completed') {container.textContent = 'Resolve the reported diagnostics, then compile again.';return;}
    if (!result.values?.length) {container.textContent = 'Execution completed with 0 published outputs.';return;}
    for (const [index, value] of result.values.entries()) {
      const card = document.createElement('div'); card.className = 'output-value';
      const label = document.createElement('h3'); label.textContent = `Output ${index + 1}`; card.append(label);
      const kind = document.createElement('div'); kind.className = 'output-kind mech-output-kind';
      kind.textContent = value.kind || 'Kind unavailable'; kind.setAttribute('aria-label', `Kind: ${kind.textContent}`);
      card.append(kind);
      card.append(renderMechValue(value));
      container.append(card);
    }
  }
  async function compile() {
    if (!api || busy || composing) return null;
    busy = true; $('document-compile').disabled = true; status('Compiling and executing the document…');
    const owner = revision;
    try {
      await new Promise(resolve => requestAnimationFrame(() => setTimeout(resolve, 0)));
      if (owner !== revision) {status('Document changed. Ctrl+Enter compiles the current source.');return null;}
      parse(); const source = area.value, start = performance.now();
      const result = JSON.parse(api.inspectMechTypes(source));
      runNumber++;
      const records = snapshot.strictly_clean ? result.diagnostics ?? [] : syntaxDiagnostics();
      showDiagnostics(records); showOutputs(result);
      $('document-result').textContent = json(result);
      $('document-stages').replaceChildren(...Object.entries(result.stages ?? {}).map(([stage, state]) => {const p = document.createElement('p');p.textContent = `${stage.replaceAll('_', ' ')}: ${stage === 'semantic_checking' && diagnostics.length && diagnostics.every(d => d.severity === 'info') ? 'executable source required for program compilation' : state}`;return p;}));
      $('document-run-label').textContent = `Run ${runNumber}`;
      const elapsed = (performance.now() - start).toFixed(1);
      const success = result.stages?.execution === 'completed';
      const errors = diagnostics.filter(d => d.severity === 'error').length;
      const informationOnly = diagnostics.length > 0 && diagnostics.every(d => d.severity === 'info');
      status(informationOnly ? 'Document inspected. 1 information message.' : success ? `Run ${runNumber} completed in ${elapsed} ms. ${result.values?.length ?? 0} published output${result.values?.length === 1 ? '' : 's'}.` : diagnostics.length ? `${diagnostics.length} diagnostic${diagnostics.length === 1 ? '' : 's'}. Select a diagnostic to inspect its source.` : `Compilation completed; ${result.stages?.execution || 'inspect the compilation stages'}.`, informationOnly ? 'info' : success ? 'success' : errors ? 'error' : '');
      window.mechDocumentLastRun = JSON.parse(json({source, revision, runNumber, result, diagnostics}));
      return window.mechDocumentLastRun;
    } catch (error) {
      showDiagnostics([{phase:'compilation',message:String(error)}]);
      $('document-output').textContent = 'Compilation could not complete.'; status(`Compilation failed: ${error}`, 'error');
      return {error:String(error)};
    } finally {busy = false; $('document-compile').disabled = !api;}
  }
  area.addEventListener('input', changed, eventOptions);
  area.addEventListener('scroll', syncScroll, eventOptions);
  new ResizeObserver(syncScroll).observe(area);
  for (const name of ['click','keyup','select']) area.addEventListener(name, caret, eventOptions);
  area.addEventListener('compositionstart', () => {composing = true;clearTimeout(parseTimer);}, eventOptions);
  area.addEventListener('compositionend', () => {composing = false;scheduleParse();}, eventOptions);
  $('document-compile').addEventListener('click', compile, eventOptions);
  $('document-load').addEventListener('click', () => {area.value = examples[$('document-example').value];changed();area.focus();}, eventOptions);
  document.addEventListener('keydown', event => {
    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey) && !event.altKey && !event.isComposing && !composing) {event.preventDefault();compile();}
  }, eventOptions);
  window.addEventListener('pagehide', () => {clearTimeout(parseTimer);parser?.free();parser = null;snapshot = null;parsedSource = undefined;}, eventOptions);
  window.addEventListener('pageshow', event => {if (event.persisted) {scheduleParse();}}, eventOptions);
  paint();
  return {
    connect(loadedApi) {api = loadedApi;parse();$('document-compile').disabled = false;status('Ready. Ctrl+Enter compiles and runs the current document.');window.mechDocumentReady = true;window.compileMechDocument = compile;},
    fail(error) {status(`Mech could not load: ${error}`, 'error');window.mechDocumentError = String(error);},
  };
}
