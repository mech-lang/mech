// Static source/export guards:
//   node benchmarks/iros-2026/blog/test-rust-examples.mjs
// Compile and run the exact downloadable examples against the current blog EKF:
//   node benchmarks/iros-2026/blog/test-rust-examples.mjs --native --target-dir /path/to/cargo/target
// Set RUSTUP_TOOLCHAIN=nightly-2026-03-03 to reproduce the workshop toolchain.
// Only a new temporary consumer crate is written; the original examples and
// repository manifests/lockfiles are never modified.
import assert from 'node:assert/strict';
import {copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join, resolve} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {adaptBlogRustExample, expandStandardExamples} from './standard-examples.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../../..');
const article = readFileSync(join(here, 'article.mec'), 'utf8');
const kernel = readFileSync(join(here, 'source/ekf.mec'), 'utf8');
const {source, downloads} = expandStandardExamples(article);
const rust = downloads.filter(item => item.file.endsWith('.rs'));
assert.deepEqual(rust.map(item => item.file), ['main.rs', 'build.rs', 'load.rs']);
const outputs = new Set([...kernel.matchAll(/^~([^\s<:]+)(?:<[^\n]+>)?\s*:=/gm)].map(match => match[1]));
assert(outputs.has('μ') && outputs.has('Σ'), 'blog kernel must publish its mathematical state bindings');
const readNames = item => [...item.source.matchAll(/\.(export|state)\("([^"\n]+)"\)/g)];
for (const item of rust) {
  assert(source.includes(`\`\`\`rust\n${item.source.trimEnd()}\n\`\`\``), `${item.file}: displayed and downloadable Rust must be identical`);
  assert(!/\.\s*(?:export|state)\("(?:state|covariance)"\)/.test(item.source), `${item.file}: stale English export`);
  for (const [, method, name] of readNames(item)) {
    assert(outputs.has(name), `${item.file}: ${method}("${name}") must name an actual blog kernel binding`);
  }
  const original = readFileSync(join(repo, 'examples/embedded_ekf', item.file), 'utf8');
  assert(original.includes('"state"'), `${item.file}: preserve the original native example`);
  assert(!original.includes('"μ"'), `${item.file}: blog adaptation must not modify the original`);
  assert.equal(item.source, original.replace(/^\s*\/\/ POSTER[^\n]*\n/gm, '')
    .replace(/\.(export|state)\("state"\)/g, '.$1("μ")'), 'adapt only known API names and print-layout annotations');
}
const savedExports = new Set(readNames(rust.find(item => item.file === 'build.rs'))
  .filter(([, method]) => method === 'export').map(([, , name]) => name));
for (const [, , name] of readNames(rust.find(item => item.file === 'load.rs'))) {
  assert(savedExports.has(name), 'AOT loader must request an export saved by the builder');
}
assert.throws(() => adaptBlogRustExample('fn main() {}', 'main.rs'), /Expected 1 export/);
assert.throws(() => adaptBlogRustExample('', 'other.rs'), /Unknown blog Rust example/);
console.log('PASS: exact displayed/downloaded Rust parity, μ export identity, saved-bundle interface and archived-source preservation.');

const args = process.argv.slice(2);
if (args.length && args[0] !== '--native') throw new Error('Expected --native [--target-dir PATH]');
if (args.includes('--native')) {
  const targetArg = args.indexOf('--target-dir');
  if (targetArg !== -1 && !args[targetArg + 1]) throw new Error('--target-dir requires a path');
  const target = resolve(targetArg === -1 ? join(repo, 'target') : args[targetArg + 1]);
  const temporary = mkdtempSync(join(tmpdir(), 'mech-blog-rust-examples-'));
  try {
    mkdirSync(join(temporary, 'src'));
    for (const item of rust) writeFileSync(join(temporary, 'src', item.file), item.source);
    writeFileSync(join(temporary, 'src/ekf.mec'), kernel);
    // Reuse the repository's dependency resolutions and compilation profile.
    // Cargo adjusts only this disposable consumer's copy of the lockfile.
    copyFileSync(join(repo, 'Cargo.lock'), join(temporary, 'Cargo.lock'));
    const manifest = readFileSync(join(repo, 'Cargo.toml'), 'utf8');
    const profile = manifest.match(/\[profile\.kernel-bench\][\s\S]*?(?=\n\[features\])/);
    assert(profile, 'repository must define the cached native embedding profile');
    const patches = manifest.match(/\[patch\.crates-io\][\s\S]*?(?=\n\[|$)/);
    assert(patches, 'unpublished beta crates require the repository local patches');
    const localPatches = patches[0].replace(/path\s*=\s*"([^"]+)"/g,
      (_, path) => `path = ${JSON.stringify(resolve(repo, path))}`);
    writeFileSync(join(temporary, 'Cargo.toml'), `[package]
name = "mech-blog-rust-examples"
version = "0.0.0"
edition = "2024"
build = false
autobins = false

[workspace]

[dependencies]
mech = { path = ${JSON.stringify(repo)}, default-features = false, features = ["kernel-aot"] }

${rust.map(item => `[[bin]]\nname = "mech-blog-${item.file.slice(0, -3)}"\npath = "src/${item.file}"`).join('\n\n')}

${profile[0]}

${localPatches}
`);
    console.log('Building the exact three downloadable Rust programs against the current blog kernel.');
    // Run Cargo from the repository so its toolchain/configuration (including
    // denied warnings) applies, while every generated manifest stays temporary.
    const build = spawnSync('cargo', ['build', '--offline', '--manifest-path', join(temporary, 'Cargo.toml'),
      '--profile', 'kernel-bench', '--target-dir', target, '--bins', '-j', '2'], {cwd: repo, stdio: 'inherit'});
    assert.ifError(build.error);
    assert.equal(build.status, 0, 'downloadable Rust programs must compile');
    function run(name) {
      const result = spawnSync(join(target, 'kernel-bench', `mech-blog-${name}`), [], {cwd: temporary, encoding: 'utf8'});
      assert.ifError(result.error);
      assert.equal(result.status, 0, `${name} failed: ${result.stderr}`);
      return result.stdout;
    }
    function poses(output) {
      const values = [...output.matchAll(/^filter (\d+): x=([^,]+), y=([^,]+), heading=([^\n]+)$/gm)]
        .map(match => match.slice(1).map(Number));
      assert.equal(values.length, 4, `expected four accepted filter states: ${output}`);
      values.forEach((row, index) => {
        assert.equal(row[0], index);
        assert(row.slice(1).every(Number.isFinite), 'state must be finite');
      });
      return values;
    }
    const jit = poses(run('main'));
    assert(jit.every(row => row.slice(1).some((value, index) => Math.abs(value - [55, 25, 0.4][index]) > 1e-5)),
      'the synchronous turn must update the initial pose');
    assert.match(run('build'), /Saved AOT bundle: ekf\.bundle/);
    assert(existsSync(join(temporary, 'ekf.bundle')), 'AOT builder must save the loadable bundle');
    const aot = poses(run('load'));
    jit.forEach((row, instance) => row.slice(1).forEach((value, coordinate) => {
      const loaded = aot[instance][coordinate + 1];
      assert(Math.abs(value - loaded) <= 1e-4 + 1e-4 * Math.abs(value), 'JIT and loaded SIMD AOT state must agree within f32 tolerance');
    }));
    console.log('PASS: exact Rust downloads execute JIT and AOT build/save/load; four synchronous μ results agree within f32 tolerance.');
  } finally {
    rmSync(temporary, {recursive: true, force: true});
  }
}
