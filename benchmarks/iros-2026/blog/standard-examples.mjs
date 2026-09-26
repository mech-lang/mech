import {readFileSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const definitions = [
  {token: 'BLOGEKFSOURCE', file: 'ekf.mec', language: 'mech', namespace: 'ekf'},
  {token: 'BLOGBEHAVIORSOURCE', file: 'behavior.mec', language: 'mech'},
  {token: 'BLOGFUNCTIONS', file: 'functions.mec', language: 'mech'},
  {token: 'BLOGMATCHING', file: 'matching.mec', language: 'mech'},
  {token: 'BLOGRUSTJIT', file: 'main.rs', language: 'rust'},
  {token: 'BLOGRUSTBUILD', file: 'build.rs', language: 'rust'},
  {token: 'BLOGRUSTLOAD', file: 'load.rs', language: 'rust'},
];
const escapeHtml = value => value.replaceAll('&', '&amp;').replaceAll('<', '&lt;')
  .replaceAll('>', '&gt;').replaceAll('"', '&quot;');

// The archived native examples use English binding names. The live article's
// mathematical source exports μ and Σ instead. Adapt only the known kernel API
// strings before either rendering or publishing the Rust source; do not alter
// the archived examples or substitute prettier, non-executable display text.
export function adaptBlogRustExample(source, file) {
  const expected = {
    'main.rs': {export: 1, state: 1},
    'build.rs': {export: 1, state: 0},
    'load.rs': {export: 0, state: 1},
  }[file];
  if (!expected) throw new Error(`Unknown blog Rust example ${file}`);
  let adapted = source.replace(/^\s*\/\/ POSTER[^\n]*\n/gm, '');
  for (const [method, count] of Object.entries(expected)) {
    const calls = new RegExp(`\\.${method}\\("state"\\)`, 'g');
    const found = [...adapted.matchAll(calls)].length;
    if (found !== count) {
      throw new Error(`Expected ${count} ${method}("state") calls in ${file}; found ${found}`);
    }
    adapted = adapted.replace(calls, `.${method}("μ")`);
  }
  return adapted;
}

// A native executable fence accepts Mech code, not Mechdown document headings.
// Keep the heading text as a comment and leave all computational lines intact.
// The unmodified literate source remains the downloadable/compute input.
export function sourceWithHeadingComments(source) {
  const lines = source.replaceAll('\r\n', '\n').split('\n');
  const converted = [];
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() && /^\s*(?:={3,}|-{3,})\s*$/.test(lines[index + 1] ?? '')) {
      converted.push(`-- ${line.trim()}`);
      index += 1;
    } else if (/^\(\d+(?:\.\d+)*\)\s+\S/.test(line)) {
      converted.push(`-- ${line}`);
    } else {
      converted.push(line);
    }
  }
  return converted.join('\n').trimEnd();
}

// Several fences in the same namespace are one program. Split at the source's
// own literate section boundaries, never into independent copied kernels.
export function ekfSections(source) {
  const normalized = sourceWithHeadingComments(source);
  const boundaries = [...normalized.matchAll(/^-- \((\d+)\) (.+)$/gm)];
  if (boundaries.length !== 4) throw new Error('Expected four EKF sections');
  const introductions = [
    '**Initialization.** The imports, motion inputs, measurement covariance, and initial state establish the filter. Matrix shapes are inferred from their values.',
    '**Time update.** The motion model predicts the next pose, and its Jacobians propagate the state and process-noise covariance.',
    '**Measurement update.** The observed bearing corrects the prediction. Wrapping the angular innovation avoids a discontinuity at a full revolution; the Joseph form updates the covariance.',
    '**Checked publication.** Integrity predicates validate the candidate before the new mean and covariance replace the accepted state.',
  ];
  return boundaries.map((boundary, index) => ({
    stage: ['initialization', 'prediction', 'correction', 'publication'][index],
    title: boundary[2],
    prose: introductions[index],
    code: (index === 0 ? normalized.slice(0, boundary.index) : '')
      + normalized.slice(boundary.index + boundary[0].length,
        boundaries[index + 1]?.index ?? normalized.length).trim(),
  }));
}

/**
 * Expand all seven source slots before the single native document render/encode.
 * `downloads` contains the exact file contents the caller should publish at `href`.
 * `examples` stays in document order for strict native block decoration below.
 */
export function expandStandardExamples(article, {
  blogRoot = here,
  repoRoot = resolve(blogRoot, '../../..'),
} = {}) {
  const byToken = new Map(definitions.map(definition => [definition.token, definition]));
  for (const {token} of definitions) {
    const matches = article.match(new RegExp(`^${token}\\s*$`, 'gm')) ?? [];
    if (matches.length !== 1) {
      throw new Error(`Expected exactly one standalone source slot ${token}; found ${matches.length}`);
    }
  }
  const examples = [];
  const downloads = [];
  const source = article.replace(/^BLOG(?:EKFSOURCE|BEHAVIORSOURCE|FUNCTIONS|MATCHING|RUSTJIT|RUSTBUILD|RUSTLOAD)[ \t]*$/gm, token => {
    const definition = byToken.get(token.trim());
    const {file, language} = definition;
    const path = language === 'mech' ? join(blogRoot, 'source', file)
      : join(repoRoot, 'examples/embedded_ekf', file);
    const original = readFileSync(path, 'utf8');
    const downloadable = language === 'rust'
      ? adaptBlogRustExample(original, file) : original;
    let code = language === 'mech' ? sourceWithHeadingComments(original) : downloadable.trimEnd();
    if (file === 'behavior.mec') code += '\n\n-- Example invocation: run from paused.\n#Robot(:paused, :run)';
    if (/^\s*```/m.test(code)) throw new Error(`Nested fence in ${file}`);
    const href = `source/${file}`;
    downloads.push({file, href, source: downloadable});
    if (file === 'ekf.mec') {
      const sections = ekfSections(original);
      for (const section of sections) examples.push({...definition, ...section, label: `ekf · ${section.title.toLowerCase()}`, href});
      return sections.map(section => `${section.prose}\n\n\`\`\`mech:ekf\n${section.code}\n\`\`\``).join('\n\n')
        + `\n\n(i)> Download the complete [${file}](${href}). These blocks share the same \`ekf\` namespace and compile together.`;
    }
    examples.push({...definition, label: file, href});
    // The numerical EKF is owned by the separately compiled checked kernel.
    // Keep it out of resident-root projection; its native output is filled by
    // that kernel's live telemetry. Other examples execute in the root scope.
    const fence = definition.namespace ? `${language}:${definition.namespace}` : language;
    return `\`\`\`${fence}\n${code}\n\`\`\`\n\nDownload the complete [${file}](${href}).`;
  });
  return {source, examples, downloads};
}

/**
 * Add native pill markup to root-scope Mech fences. Preserve the EKF's native
 * named pill and mark its native output for the separate live-kernel host.
 * Never rewrite source spans, block IDs, output IDs, or namespace addresses.
 * This intentionally fails if the native formatter's contract changes.
 */
export function decorateStandardExamples(html, examples) {
  const mechExamples = examples.filter(example => example.language === 'mech');
  let index = 0;
  const decorated = html.replace(/(<div id="([^"]+)" class="mech-fenced-mech-block" data-mech-source(?: style="[^"]*")?>)([\s\S]*?<div class="mech-block-output" id="[^"]+")><\/div>\s*<\/div>/g, (block, opening, id, body) => {
    const example = mechExamples[index++];
    if (!example) throw new Error('Unexpected extra native Mech fence in article');
    const pill = label => `<div class="mech-code-block-namespace"><a href="#${escapeHtml(id)}">${escapeHtml(label)}</a></div>`;
    if (example.namespace) {
      if (!body.includes(pill(example.namespace))) throw new Error(`Missing native namespace pill for ${example.file}`);
      return block.replace(opening, opening.replace(' data-mech-source', ' data-workshop-kernel-listing data-mech-source'))
        .replace(pill(example.namespace), pill(example.label))
        .replace(/(<div class="mech-block-output" id="[^"]+")>/,
          example.stage === 'publication' ? '$1 data-workshop-kernel-output>' : '$1 hidden>');
    }
    if (body.includes('class="mech-code-block-namespace"')) throw new Error(`Unexpected executable namespace for ${example.file}`);
    return block.replace(opening, `${opening}\n          ${pill(example.label)}`);
  });
  if (index !== mechExamples.length) {
    throw new Error(`Expected ${mechExamples.length} native Mech fences; found ${index}`);
  }
  return decorated;
}
