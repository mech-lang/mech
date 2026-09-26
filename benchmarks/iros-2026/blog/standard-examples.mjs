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
    // POSTER annotations describe the print layout, not Rust program semantics.
    const downloadable = language === 'rust'
      ? original.replace(/^\s*\/\/ POSTER[^\n]*\n/gm, '') : original;
    let code = language === 'mech' ? sourceWithHeadingComments(original) : downloadable.trimEnd();
    if (file === 'behavior.mec') code += '\n\n-- Example invocation: Run from Paused.\n#Robot(0, 1)';
    if (/^\s*```/m.test(code)) throw new Error(`Nested fence in ${file}`);
    const href = `source/${file}`;
    examples.push({...definition, label: file, href});
    downloads.push({file, href, source: downloadable});
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
      return block.replace(/(<div class="mech-block-output" id="[^"]+")>/, '$1 data-workshop-kernel-output>');
    }
    if (body.includes('class="mech-code-block-namespace"')) throw new Error(`Unexpected executable namespace for ${example.file}`);
    return block.replace(opening, `${opening}\n          ${pill(example.label)}`);
  });
  if (index !== mechExamples.length) {
    throw new Error(`Expected ${mechExamples.length} native Mech fences; found ${index}`);
  }
  return decorated;
}
