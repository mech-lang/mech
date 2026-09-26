#!/usr/bin/env node
// Diagnostic only. Runs the actual WASM CPU kernel; JavaScript supplies inputs
// and inspects results, but does not implement the EKF equations.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';

const usage = `Usage: node replay-ekf-symmetry.mjs \\
  --source PATH --wasm-js PATH --wasm PATH \\
  [--instances 4096] [--turns 600] [--state state] [--covariance covariance]

Use matching retained source/WASM artifacts for a historical replay. The
current Unicode source uses --state μ --covariance Σ. Progress goes to stderr;
the final JSON report goes to stdout. This helper never edits source files.
`;
const args = process.argv.slice(2);
if (args.includes('--help')) {
  console.log(usage);
  process.exit(0);
}
const allowed = new Set(['source', 'wasm-js', 'wasm', 'instances', 'turns', 'state', 'covariance']);
const options = {};
for (let i = 0; i < args.length; i += 2) {
  const key = args[i].replace(/^--/, '');
  if (!args[i].startsWith('--') || !allowed.has(key) || !args[i + 1] || key in options) {
    throw new Error(`Invalid or duplicate option ${args[i]}\n${usage}`);
  }
  options[key] = args[i + 1];
}
for (const key of ['source', 'wasm-js', 'wasm']) {
  if (!options[key]) throw new Error(`Missing --${key}\n${usage}`);
}
const instances = Number(options.instances ?? 4096);
const limit = Number(options.turns ?? 600);
assert(Number.isSafeInteger(instances) && instances > 0 && instances <= 65536, 'instances must be 1..65536');
assert(Number.isSafeInteger(limit) && limit > 0, 'turns must be a positive integer');
const stateName = options.state ?? 'state';
const covarianceName = options.covariance ?? 'covariance';
const source = readFileSync(options.source, 'utf8');
const wasmBytes = readFileSync(options.wasm);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const wasmModule = await import(pathToFileURL(resolve(options['wasm-js'])).href);
await wasmModule.default({module_or_path: wasmBytes});
const {WasmKernel} = wasmModule;
const bits = values => new Uint32Array(values.buffer, values.byteOffset, values.length);
const bitEqual = (left, right) => left.length === right.length && bits(left).every((x, i) => x === bits(right)[i]);

function covarianceStatistics(values) {
  assert.equal(values.length, 9, 'expected one 3×3 covariance');
  const pairs = [[1, 3], [2, 6], [5, 7]].map(([i, j]) => ({
    indices: [i, j],
    values: [values[i], values[j]],
    difference: Math.abs(values[i] - values[j]),
    // These samples have same-sign mirrored entries. Bit distance is only an
    // ULP count under that condition; do not label opposite-sign distances.
    sameSignUlpDistance: Math.sign(values[i]) === Math.sign(values[j])
      ? Math.abs(bits(values)[i] - bits(values)[j]) : null,
  }));
  const maxAsymmetry = Math.max(...pairs.map(pair => pair.difference));
  const scale = Math.max(...Array.from(values, Math.abs));
  const [xx, yy, zz] = [values[0], values[4], values[8]];
  const [xy, xz, yz] = pairs.map(pair => (pair.values[0] + pair.values[1]) / 2);
  return {
    covarianceColumnMajor: Array.from(values),
    finite: Array.from(values).every(Number.isFinite),
    diagonal: [xx, yy, zz],
    maxAsymmetry,
    scale,
    relativeToScale: maxAsymmetry / scale,
    pairs,
    // Descriptive f64 analysis of the returned f32 values, not an EKF update.
    // Positive leading principal minors establish positivity of this
    // symmetric part; they do not certify every turn or the raw nonsymmetric
    // matrix as a valid covariance.
    symmetrizedLeadingMinors: [
      xx,
      xx * yy - xy * xy,
      xx * yy * zz + 2 * xy * xz * yz - xx * yz * yz - yy * xz * xz - zz * xy * xy,
    ],
  };
}

function observation(truth, turn, count, selectedLane) {
  const [x, y, heading] = truth;
  const next = [x + 0.1 * Math.cos(heading), y + 0.1 * Math.sin(heading), heading + 0.015 * 0.1];
  const bearing = Math.atan2(12 - next[1], 140 - next[0]) - next[2];
  return {
    next,
    inputs: {
      bearing: Float32Array.from({length: count}, (_, lane) =>
        bearing + 0.02 * Math.sin(turn * 1.73 + (selectedLane ?? lane) * 0.37)),
      v: [1],
      w: [0.015],
    },
  };
}

function createKernel(text, count) {
  return WasmKernel.fromSource(text, {
    bearing: new Float32Array(count).fill(-0.55), v: [1], w: [0.015],
  }, [stateName, covarianceName]);
}

function diagnosticUncheckedSource(text) {
  let result = text;
  for (const name of ['finite-candidate', 'positive-covariance', 'symmetric-covariance']) {
    const declaration = new RegExp(`^${name}! :=[^\\r\\n]*(?:\\r?\\n|$)`, 'gm');
    assert.equal(Array.from(result.matchAll(declaration)).length, 1,
      `expected exactly one single-line ${name} integrity declaration`);
    result = result.replace(declaration, '');
  }
  return result;
}

// Candidate observation is a separate in-memory diagnostic. The checked
// production source is untouched, and the original batch's rejection is not
// retried or accepted. This technique is CPU-only: removing GPU checks can
// alter shader optimization and has been observed to change results at turn 1.
function inspectCpuCandidate(fault, batch) {
  const uncheckedSource = diagnosticUncheckedSource(source);
  const checked = createKernel(source, 1);
  const unchecked = createKernel(uncheckedSource, 1);
  try {
    let truth = [55, 25, 0.4];
    for (let turn = 1; turn <= fault.turn; turn++) {
      const sample = observation(truth, turn, 1, fault.lane);
      unchecked.turn(sample.inputs);
      if (turn < fault.turn) {
        checked.turn(sample.inputs);
        for (const name of [stateName, covarianceName]) {
          assert(bitEqual(checked.state(name), unchecked.state(name)),
            `checked/unchecked CPU prefix differs at turn ${turn}, ${name}`);
        }
      } else {
        for (const name of [stateName, covarianceName]) {
          assert(bitEqual(checked.state(name), batch.stateSample(name, fault.lane)),
            `single-lane and batch last accepted values differ for ${name}`);
        }
        let error;
        try { checked.turn(sample.inputs); } catch (caught) { error = String(caught); }
        assert(error?.includes('symmetric-covariance'), 'selected CPU lane must reject on the same turn');
        fault.selectedLaneRejection = error;
      }
      truth = sample.next;
    }
    return {
      method: 'CPU-only unchecked in-memory source, with checked single-lane and full-batch comparisons',
      uncheckedSourceSha256: hash(uncheckedSource),
      checkedAndUncheckedPrefixBitIdentical: true,
      checkedSingleLaneMatchesBatchLastAccepted: true,
      checkedSingleLaneRejectedSameTurn: true,
      ...covarianceStatistics(unchecked.state(covarianceName)),
      mean: Array.from(unchecked.state(stateName)),
    };
  } finally {
    checked.free();
    unchecked.free();
  }
}

const start = performance.now();
const report = {
  diagnostic: 'default-input EKF symmetry rejection; no production modifications',
  runtime: 'actual WASM CPU kernel',
  sourceSha256: hash(source),
  wasmSha256: hash(wasmBytes),
  wasmJsSha256: hash(readFileSync(options['wasm-js'])),
  instances,
  limit,
  controls: {velocity: 1, omega: 0.015, noise: 0.02, timeStep: 0.1},
  exports: [stateName, covarianceName],
  firstFailure: null,
};
const kernel = createKernel(source, instances);
try {
  let truth = [55, 25, 0.4];
  for (let turn = 1; turn <= limit; turn++) {
    const beforeState = kernel.state(stateName);
    const beforeCovariance = kernel.state(covarianceName);
    const sample = observation(truth, turn, instances);
    try {
      kernel.turn(sample.inputs);
      truth = sample.next;
    } catch (error) {
      const match = String(error).match(/instance:\s*(\d+)/);
      assert(match, `expected an integrity fault with an instance index: ${error}`);
      assert(String(error).includes('symmetric-covariance'), `unexpected rejection: ${error}`);
      const lane = Number(match[1]);
      const rollbackAllStateBitsUnchanged = bitEqual(beforeState, kernel.state(stateName))
        && bitEqual(beforeCovariance, kernel.state(covarianceName));
      assert(rollbackAllStateBitsUnchanged, 'rejected batch must preserve all published state bits');
      report.firstFailure = {
        turn,
        accepted: turn - 1,
        lane,
        error: String(error),
        rollbackAllStateBitsUnchanged,
        lastAccepted: covarianceStatistics(kernel.stateSample(covarianceName, lane)),
      };
      report.firstFailure.candidate = inspectCpuCandidate(report.firstFailure, kernel);
      break;
    }
    if (turn % 100 === 0) console.error(JSON.stringify({progress: turn, instances, elapsedMs: performance.now() - start}));
  }
  report.status = report.firstFailure ? 'reproduced' : 'no-rejection-within-limit';
  report.elapsedMs = performance.now() - start;
  console.log(JSON.stringify(report, null, 2));
} finally {
  kernel.free();
}
