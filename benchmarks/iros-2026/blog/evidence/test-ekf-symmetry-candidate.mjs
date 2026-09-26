#!/usr/bin/env node
// Diagnostic-only trial. All EKF arithmetic is executed by the retained WASM.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';

const [modulePath, binaryPath, instanceText = '65536', turnText = '1000', auditText = 'false', sourcePath] = process.argv.slice(2);
assert(modulePath && binaryPath, 'usage: node test-ekf-symmetry-candidate.mjs WASM_JS WASM_BINARY [INSTANCES] [TURNS] [AUDIT_RAW=false] [SOURCE_PATH]');
const instances = Number(instanceText), turns = Number(turnText), auditRaw = auditText === 'true';
assert(Number.isSafeInteger(instances) && instances > 0 && instances <= 65536);
assert(Number.isSafeInteger(turns) && turns > 0);
const source = readFileSync(sourcePath ?? new URL('./ekf-symmetry-candidate.mec', import.meta.url), 'utf8');
const unicode = source.includes('~μ<');
const stateName = unicode ? 'μ' : 'state', covarianceName = unicode ? 'Σ' : 'covariance';
const binary = readFileSync(binaryPath);
const hash = value => createHash('sha256').update(value).digest('hex');
const {default: initialize, WasmKernel} = await import(pathToFileURL(resolve(modulePath)).href);
await initialize({module_or_path: binary});
const bits = a => new Uint32Array(a.buffer, a.byteOffset, a.length);
const same = (a, b) => a.length === b.length && bits(a).every((v, i) => v === bits(b)[i]);
const names = [stateName, covarianceName];
const initial = n => ({bearing: new Float32Array(n).fill(-0.55), v: [1], w: [0.015]});
const create = (text, n, exports = names) => WasmKernel.fromSource(text, initial(n), exports);
const snapshot = kernel => names.map(name => kernel.state(name));
const assertSnapshot = (kernel, before, message) => names.forEach((name, i) => assert(same(kernel.state(name), before[i]), `${message}: ${name}`));

function observation(truth, turn, count) {
  const [x, y, h] = truth;
  const next = [x + 0.1 * Math.cos(h), y + 0.1 * Math.sin(h), h + 0.015 * 0.1];
  const bearing = Math.atan2(12 - next[1], 140 - next[0]) - next[2];
  return {next, inputs: {bearing: Float32Array.from({length: count}, (_, lane) => bearing + 0.02 * Math.sin(turn * 1.73 + lane * 0.37)), v: [1], w: [0.015]}};
}
function publicationChecks(kernel) {
  const cov = kernel.state(covarianceName), words = bits(cov);
  let maxScale = 0, minDiagonal = Infinity;
  for (let base = 0; base < cov.length; base += 9) {
    for (let i = 0; i < 9; i++) {
      assert(Number.isFinite(cov[base + i]), 'nonfinite published covariance');
      maxScale = Math.max(maxScale, Math.abs(cov[base + i]));
    }
    for (const i of [0, 4, 8]) {
      assert(cov[base + i] > 0, 'nonpositive published diagonal');
      minDiagonal = Math.min(minDiagonal, cov[base + i]);
    }
    for (const [i, j] of [[1, 3], [2, 6], [5, 7]]) assert.equal(words[base + i], words[base + j], 'published pair must be bitwise symmetric');
  }
  for (const value of kernel.state(stateName)) assert(Number.isFinite(value), 'nonfinite published mean');
  return {maxScale, minDiagonal};
}

function invalidInputChecks(kernel, input) {
  const results = [];
  for (const value of [NaN, Infinity, -Infinity]) {
    const before = snapshot(kernel);
    const bearing = input.bearing.slice();
    bearing[instances - 1] = value;
    let error;
    try { kernel.turn({...input, bearing}); } catch (caught) { error = String(caught); }
    assert(error?.includes('Integrity'), `expected integrity rejection for ${value}`);
    assertSnapshot(kernel, before, `rollback after ${value}`);
    results.push({input: String(value), lane: instances - 1, rejected: true, wholeBatchRollbackBitIdentical: true, error});
  }
  return results;
}

function grossAsymmetryCheck() {
  const original = unicode ? "Σraw := A ** Σ̄ ** A' + (K ** K') * R" : "Praw := A ** P0 ** A' + (K ** K') * R";
  assert.equal(source.split(original).length, 2);
  const injected = source.replace(original, `${original} + [0f32 1f32 0f32; 0f32 0f32 0f32; 0f32 0f32 0f32]`);
  const kernel = create(injected, 1);
  try {
    const before = snapshot(kernel);
    let error;
    try { kernel.turn(observation([55, 25, 0.4], 1, 1).inputs); } catch (caught) { error = String(caught); }
    assert(error?.includes('symmetric-covariance'), 'raw asymmetric candidate must not be hidden by projection');
    assertSnapshot(kernel, before, 'gross-asymmetry rollback');
    return {injectedRawPairGap: 1, rejectedByRawSymmetryGuard: true, rollbackBitIdentical: true, sourceSha256: hash(injected), error};
  } finally { kernel.free(); }
}

let instrumented;
if (auditRaw) {
  // CPU-only measurement fixture: extra persistent exports record the raw
  // residual and budget. Every accepted mean/covariance is compared bitwise
  // with the uninstrumented candidate. This is not the GPU trial source.
  const instrumentedSource = source.replace('(2) Time Update',
    '~diagnostic-residual<[f32]:3,1> := [0; 0; 0]\n~diagnostic-budget<[f32]:3,1> := [0; 0; 0]\n\n(2) Time Update')
    .replace(unicode ? 'μ = μ₊' : 'state = mu1', unicode
      ? 'diagnostic-residual = ΔΣ\ndiagnostic-budget = τ\nμ = μ₊'
      : 'diagnostic-residual = D\ndiagnostic-budget = T\nstate = mu1');
  instrumented = create(instrumentedSource, instances, [...names, 'diagnostic-residual', 'diagnostic-budget']);
}

const report = {
  sourceSha256: hash(source), wasmSha256: hash(binary), instances, turns,
  controls: {velocity: 1, omega: 0.015, noise: 0.02, timeStep: 0.1},
  policy: {absolute: 1e-4, relativePerOperand: 1e-6, pairBudget: 'absolute + relative*abs(a) + relative*abs(b)', projection: 'raw*0.5 + transpose(raw)*0.5'},
  accepted: 0, publicationCheckedEveryTurn: true, maxCovarianceMagnitude: 0, minDiagonal: Infinity,
  rawAudit: auditRaw ? {comparedEveryAcceptedStateBit: true, maxGap: 0, maxGapToBudget: 0} : null,
  grossAsymmetry: grossAsymmetryCheck(),
};
const start = performance.now(), kernel = create(source, instances);
try {
  let truth = [55, 25, 0.4];
  const first = observation(truth, 1, instances);
  report.initialInvalidInputs = invalidInputChecks(kernel, first.inputs);
  for (let turn = 1; turn <= turns; turn++) {
    const sample = observation(truth, turn, instances);
    kernel.turn(sample.inputs);
    if (instrumented) {
      instrumented.turn(sample.inputs);
      assertSnapshot(instrumented, snapshot(kernel), `instrumented comparison at turn ${turn}`);
      const residual = instrumented.state('diagnostic-residual'), budget = instrumented.state('diagnostic-budget');
      for (let i = 0; i < residual.length; i++) {
        report.rawAudit.maxGap = Math.max(report.rawAudit.maxGap, Math.abs(residual[i]));
        report.rawAudit.maxGapToBudget = Math.max(report.rawAudit.maxGapToBudget, Math.abs(residual[i]) / budget[i]);
      }
    }
    const checked = publicationChecks(kernel);
    report.maxCovarianceMagnitude = Math.max(report.maxCovarianceMagnitude, checked.maxScale);
    report.minDiagonal = Math.min(report.minDiagonal, checked.minDiagonal);
    report.accepted++;
    truth = sample.next;
    if (turn % 100 === 0) console.error(JSON.stringify({progress: turn, instances, elapsedMs: performance.now() - start}));
  }
  const recovery = observation(truth, turns + 1, instances);
  report.finalInvalidInputs = invalidInputChecks(kernel, recovery.inputs);
  kernel.turn(recovery.inputs);
  report.recovery = {accepted: true, ...publicationChecks(kernel)};
  if (instrumented) {
    instrumented.turn(recovery.inputs);
    assertSnapshot(instrumented, snapshot(kernel), 'recovery must match uninterrupted reference');
    report.recovery.matchesUninterruptedReferenceBits = true;
  }
  report.finalLane0 = {mean: Array.from(kernel.stateSample(stateName, 0)), covarianceColumnMajor: Array.from(kernel.stateSample(covarianceName, 0))};
  report.status = 'passed';
} catch (error) {
  report.status = 'failed'; report.error = String(error); process.exitCode = 1;
} finally {
  report.elapsedMs = performance.now() - start;
  kernel.free(); instrumented?.free();
  console.log(JSON.stringify(report, null, 2));
}
