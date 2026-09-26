/**
 * await verifyKernel({ WasmKernel, source, adapter?, instances: 256 })
 *
 * Initialize the WASM package and load include/browser-compute.js first.
 * This owns a separate kernel/device; it never advances the displayed demo.
 * `passed` requires real WebGPU execution. `status` is passed/failed/unsupported;
 * unsupported may still have cpuPassed=true. No measured GPU error is reported
 * when there were no GPU comparisons. The exact paper source hash is enforced.
 *
 * Twenty deterministic turns compare lane zero, with elementwise f32 tolerance
 * abs=1e-4, rel=1e-4. Test-only full GPU readbacks then prove bitwise whole-batch
 * rollback and compare every instance after recovery. These transfers are not
 * part of the demo's FPS measurement. CPU and GPU matrix layouts are reconciled
 * through the existing Device output-layout conversion, not EKF code in JS.
 */
const PAPER_SHA256 = "f18e37effb2fa63fadca69639f3a8eed218b73218decf65419e78a60b62bb46b";
const NAMES = ["μ", "Σ"];
const WIDTHS = { μ: 3, Σ: 9 };
const TURNS = 20;
const TOLERANCE = Object.freeze({ absolute: 1e-4, relative: 1e-4 });

function requireThat(condition, message) {
  if (!condition) throw new Error(message);
}

function words(values) {
  return new Uint32Array(values.buffer, values.byteOffset, values.byteLength / 4).slice();
}

function equalWords(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function emptyErrors() {
  return { valuesCompared: 0, maxAbsolute: null, maxRelative: null, maxToleranceRatio: null, failures: 0 };
}

function compare(reference, actual, expectedLength, statistics, context) {
  requireThat(reference.length === expectedLength && actual?.length === expectedLength,
    `${context}: expected ${expectedLength} values in both outputs`);
  for (let index = 0; index < expectedLength; index += 1) {
    const left = reference[index], right = actual[index];
    requireThat(Number.isFinite(left) && Number.isFinite(right),
      `${context}[${index}]: non-finite CPU/GPU value`);
    const difference = Math.abs(left - right);
    const magnitude = Math.max(Math.abs(left), Math.abs(right));
    const tolerance = TOLERANCE.absolute + TOLERANCE.relative * magnitude;
    statistics.valuesCompared += 1;
    statistics.maxAbsolute = Math.max(statistics.maxAbsolute ?? 0, difference);
    statistics.maxRelative = Math.max(statistics.maxRelative ?? 0, magnitude ? difference / magnitude : 0);
    statistics.maxToleranceRatio = Math.max(statistics.maxToleranceRatio ?? 0, difference / tolerance);
    if (difference > tolerance) statistics.failures += 1;
  }
}

function deterministicInputs(turn, instances) {
  // Deterministic sensor/control packets, not an EKF implementation. Per-lane
  // variation ensures the faulted last lane is not interchangeable with lane 0.
  return {
    bearing: Float32Array.from({ length: instances }, (_, lane) =>
      -0.55 + 0.008 * Math.sin(turn * 0.17) + lane * 0.000001),
    v: [1 + 0.08 * Math.sin(turn * 0.11)],
    w: [0.015 + 0.003 * Math.cos(turn * 0.13)],
  };
}

function cpuSnapshot(kernel, instances) {
  return Object.fromEntries(NAMES.map(name => {
    const values = kernel.state(name);
    requireThat(values.length === WIDTHS[name] * instances, `CPU ${name}: wrong full-batch width`);
    return [name, words(values)];
  }));
}

async function gpuSnapshot(resource, manifest, exported, activeBuffer, instances) {
  const buffers = [];
  const mappings = [];
  let queueCompletion;
  try {
    const encoder = resource.device.createCommandEncoder();
    for (const name of NAMES) {
      const state = manifest.states.find(value => Number(value.slot) === Number(exported[name].slot));
      requireThat(state && Number(state.elements) === WIDTHS[name] * instances,
        `GPU ${name}: wrong full-batch state width`);
      const size = Number(state.elements) * 4;
      const buffer = resource.device.createBuffer({
        size, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
      });
      buffers.push({ name, buffer, size });
      encoder.copyBufferToBuffer(resource.outputBuffer(activeBuffer, { slot: state.slot }), 0, buffer, 0, size);
    }
    resource.device.queue.submit([encoder.finish()]);
    queueCompletion = resource.device.queue.onSubmittedWorkDone();
    for (const { buffer, size } of buffers) mappings.push(buffer.mapAsync(GPUMapMode.READ, 0, size));
    await Promise.all([queueCompletion, ...mappings]);
    return Object.fromEntries(buffers.map(({ name, buffer, size }) =>
      [name, new Uint32Array(buffer.getMappedRange(0, size)).slice()]));
  } finally {
    // Even failed mappings may have accepted work; settle every operation before
    // releasing the temporary readbacks or allowing the owning Device to close.
    await Promise.allSettled([queueCompletion, ...mappings].filter(Boolean));
    for (const { buffer } of buffers) {
      if (buffer.mapState === "mapped") buffer.unmap();
      buffer.destroy();
    }
  }
}

export async function verifyKernel({ WasmKernel, source, adapter, instances = 256 } = {}) {
  const started = performance.now();
  const report = {
    supported: false, passed: false, status: "failed", reason: null,
    sourceSha256: null, instances, turns: TURNS, tolerance: TOLERANCE,
    cpuPassed: false, gpuExecuted: false, comparedTurns: 0,
    errors: Object.fromEntries(NAMES.map(name => [name, emptyErrors()])),
    rollback: null, recovery: null, elapsedMs: null,
  };
  let kernel, resource, manifest, exported, activeBuffer = 0;
  let unsupportedReason = null;
  let stage = "initialization";
  try {
    requireThat(Number.isInteger(instances) && instances >= 2 && instances <= 4096,
      "instances must be an integer from 2 through 4096");
    requireThat(typeof source === "string", "source must be the complete paper EKF document");
    requireThat(typeof WasmKernel?.fromSource === "function", "WasmKernel is not initialized");
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source));
    report.sourceSha256 = Array.from(new Uint8Array(digest), value => value.toString(16).padStart(2, "0")).join("");
    requireThat(report.sourceSha256 === PAPER_SHA256, "source differs from the complete paper EKF document");

    stage = "source compilation";
    kernel = WasmKernel.fromSource(source, deterministicInputs(0, instances), NAMES);
    requireThat(kernel.instances() === instances, "compiled batch extent differs from requested instances");
    for (const name of NAMES) requireThat(kernel.stateWidth(name) === WIDTHS[name], `unexpected ${name} shape`);
    manifest = kernel.computeManifest();
    exported = Object.fromEntries(manifest.exports.map(value => [value.name, value]));
    for (const name of NAMES) {
      requireThat(typeof exported[name]?.outputName === "string", `GPU export ${name} is missing`);
      requireThat(manifest.outputs.some(value => value.name === exported[name].outputName), `GPU output ${name} is missing`);
    }
    requireThat(manifest.bindings.length === 8, "three-input paper kernel must use eight GPU storage bindings");

    stage = "WebGPU availability";
    if (adapter === undefined) adapter = await globalThis.navigator?.gpu?.requestAdapter();
    if (!adapter) {
      unsupportedReason = "WebGPU adapter unavailable; only CPU checks ran.";
    } else {
      const Device = globalThis.MechBrowserCompute?.Device;
      requireThat(typeof Device?.create === "function", "MechBrowserCompute.Device is not loaded");
      try {
        Device.requiredLimits(manifest, adapter.limits);
      } catch (error) {
        const message = String(error);
        if (!/but this adapter supports|this WebGPU adapter does not report/.test(message)) throw error;
        unsupportedReason = message;
      }
      if (!unsupportedReason) {
        stage = "GPU device and shader creation";
        resource = await Device.create(manifest, adapter, NAMES.map(name => exported[name].outputName));
        report.supported = true;
      }
    }

    const gpuTurn = async updates => {
      const submission = resource.submit({ inputs: kernel.gpuInputs(updates) }, activeBuffer);
      const result = await resource.finish(submission);
      report.gpuExecuted = true;
      if (!result.integrity) activeBuffer = submission.outputIndex;
      return result;
    };
    const compareSample = (result, turn) => {
      requireThat(!result.integrity, `valid GPU turn ${turn} was rejected: ${JSON.stringify(result.integrity)}`);
      requireThat(result.outputs.length === NAMES.length, `turn ${turn}: missing or duplicate GPU outputs`);
      for (const name of NAMES) {
        const descriptor = manifest.outputs.find(value => value.name === exported[name].outputName);
        const expected = globalThis.MechBrowserCompute.Device.logicalOutputValues(descriptor, kernel.stateSample(name, 0));
        const actual = result.outputs.find(value => value.name === exported[name].outputName)?.values;
        compare(expected, actual, WIDTHS[name], report.errors[name], `${name}, turn ${turn}, lane 0`);
      }
      report.comparedTurns += 1;
    };

    stage = "20 deterministic turns";
    const initial = cpuSnapshot(kernel, instances);
    for (let turn = 1; turn <= TURNS; turn += 1) {
      const updates = deterministicInputs(turn, instances);
      kernel.turn(updates);
      if (resource) compareSample(await gpuTurn(updates), turn);
    }
    const beforeCpu = cpuSnapshot(kernel, instances);
    requireThat(!equalWords(initial['μ'], beforeCpu['μ']), "valid CPU turns did not change state");
    const beforeGpu = resource ? await gpuSnapshot(resource, manifest, exported, activeBuffer, instances) : null;
    const beforeActive = activeBuffer;
    const invalid = deterministicInputs(TURNS + 1, instances);
    invalid.bearing[instances - 1] = NaN;

    stage = "last-lane NaN rejection and whole-batch rollback";
    let cpuError = null;
    try { kernel.turn(invalid); } catch (error) { cpuError = String(error); }
    const cpuFaultInstance = Number(cpuError?.match(/instance:\s*(\d+)/)?.[1]);
    requireThat(cpuError?.includes("finite-candidate!") && cpuFaultInstance === instances - 1,
      `CPU did not report the intended last-lane finite-candidate rejection: ${cpuError}`);
    const afterCpu = cpuSnapshot(kernel, instances);
    const cpuUnchanged = NAMES.every(name => equalWords(beforeCpu[name], afterCpu[name]));
    requireThat(cpuUnchanged, "CPU published part of the rejected batch");
    report.rollback = { cpu: { rejected: true, instance: cpuFaultInstance, allStateBitsUnchanged: cpuUnchanged }, gpu: null };
    if (resource) {
      const rejected = await gpuTurn(invalid);
      requireThat(rejected.integrity?.constraint === "finite-candidate!" && rejected.integrity.instance === instances - 1,
        `GPU did not report the intended last-lane finite-candidate rejection: ${JSON.stringify(rejected.integrity)}`);
      requireThat(rejected.outputs.length === 0, "GPU exposed candidate outputs from a rejected turn");
      const afterGpu = await gpuSnapshot(resource, manifest, exported, activeBuffer, instances);
      const gpuUnchanged = NAMES.every(name => equalWords(beforeGpu[name], afterGpu[name]));
      requireThat(beforeActive === activeBuffer && gpuUnchanged, "GPU published part of the rejected batch");
      report.rollback.gpu = { rejected: true, ...rejected.integrity, allStateBitsUnchanged: gpuUnchanged, activeBufferUnchanged: true };
    }

    stage = "valid recovery turn";
    const valid = deterministicInputs(TURNS + 2, instances);
    kernel.turn(valid);
    const recoveredCpu = cpuSnapshot(kernel, instances);
    requireThat(!equalWords(beforeCpu['μ'], recoveredCpu['μ']), "CPU did not advance after rejection");
    requireThat(kernel.attemptedTurns() === TURNS + 2 && kernel.faultCount() === 1, "unexpected CPU turn or fault count");
    report.recovery = { cpu: true, gpu: false, comparedInstances: 0 };
    report.cpuPassed = true;
    if (resource) {
      const failuresBeforeRecovery = NAMES.reduce((sum, name) => sum + report.errors[name].failures, 0);
      compareSample(await gpuTurn(valid), TURNS + 2);
      const recoveredGpu = await gpuSnapshot(resource, manifest, exported, activeBuffer, instances);
      for (const name of NAMES) {
        // These full-buffer snapshots both have physical column-major layout.
        compare(new Float32Array(recoveredCpu[name].buffer), new Float32Array(recoveredGpu[name].buffer),
          WIDTHS[name] * instances, report.errors[name], `${name}, recovered full batch`);
      }
      report.recovery.gpu = NAMES.reduce((sum, name) => sum + report.errors[name].failures, 0) === failuresBeforeRecovery;
      report.recovery.comparedInstances = instances;
    }
    report.passed = report.supported && report.gpuExecuted && report.cpuPassed &&
      NAMES.every(name => report.errors[name].valuesCompared > 0 && report.errors[name].failures === 0);
    report.status = report.passed ? "passed" : unsupportedReason ? "unsupported" : "failed";
    report.reason = report.passed ? null : unsupportedReason || "CPU/GPU numerical tolerance exceeded.";
  } catch (error) {
    report.status = "failed";
    report.passed = false;
    report.reason = `${stage}: ${String(error)}`;
  } finally {
    try {
      if (resource) {
        resource.dispose();
        await resource.disposeCompletion;
        if (resource.queueCleanupFailure) throw resource.queueCleanupFailure;
      }
    } catch (error) {
      report.passed = false;
      report.status = "failed";
      report.reason = `${report.reason ? `${report.reason}; ` : ""}GPU cleanup: ${String(error)}`;
    } finally {
      try {
        kernel?.free();
      } catch (error) {
        report.passed = false;
        report.status = "failed";
        report.reason = `${report.reason ? `${report.reason}; ` : ""}WASM cleanup: ${String(error)}`;
      }
      report.elapsedMs = performance.now() - started;
    }
  }
  return report;
}
