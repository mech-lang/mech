// Build src/wasm with browser_compute_canary, then run:
// node src/wasm/tests/kernel-smoke.mjs [optional-behavior.mec]
// This exercises the generated JavaScript/WASM boundary without mocking Mech.
// Actual WebGPU dispatch is covered by the browser host tests, not this script.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import init, { WasmKernel, WasmRepl } from "../pkg/mech_wasm.js";

await init({ module_or_path: await readFile(new URL("../pkg/mech_wasm_bg.wasm", import.meta.url)) });
const source = await readFile(new URL("./fixtures/paper-ekf.mec", import.meta.url), "utf8");
assert.equal(createHash("sha256").update(source).digest("hex"),
  "a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2");

const kernel = WasmKernel.fromSource(source, {
  bearing: new Float32Array(4).fill(-0.55),
  dt: [0.1], v: [1], w: [0.015], R: [0.25], fmax: [3.402823466e38], eps: [0.0001],
}, ["state", "covariance"]);
try {
  assert.equal(kernel.instances(), 4);
  assert.equal(kernel.stateWidth("state"), 3);
  assert.equal(kernel.stateWidth("covariance"), 9);
  const initial = kernel.state("state");
  assert.equal(initial.length, 12);
  assert.deepEqual(kernel.stateSample("state", 2), initial.slice(6, 9));
  assert.throws(() => kernel.stateSample("state", 4), /outside the compiled batch/);
  assert.throws(() => kernel.state("missing"), /not exported/);

  kernel.turn({ bearing: [-0.54] });
  const accepted = kernel.state("state");
  const acceptedCovariance = kernel.state("covariance");
  assert.notDeepEqual(accepted, initial);
  assert.throws(() => kernel.turn({ bearing: [-0.55, -0.55, -0.55, NaN] }), /finite-candidate!/);
  assert.deepEqual(kernel.state("state"), accepted);
  assert.deepEqual(kernel.state("covariance"), acceptedCovariance);
  assert.equal(Number(kernel.attemptedTurns()), 2);
  assert.equal(Number(kernel.faultCount()), 1);
  assert.throws(() => kernel.turn({ bearing: [-0.5, -0.5] }), /broadcast or/);
  assert.throws(() => kernel.turn({ missing: [1] }), /not a live input/);
  assert.throws(() => kernel.turn({ bearing: ["bad"] }), /non-number/);
  kernel.turn({ bearing: [-0.53] });
  assert.notDeepEqual(kernel.state("state"), accepted);

  kernel.reset();
  assert.deepEqual(kernel.state("state"), initial);
  assert.equal(Number(kernel.attemptedTurns()), 0);
  const manifest = kernel.computeManifest();
  assert.equal(manifest.instances, 4);
  assert.equal(manifest.exports.length, 2);
  assert(manifest.exports.every(value => typeof value.outputName === "string"));
  assert(manifest.wgsl.includes("@compute"));
  const inputs = kernel.gpuInputs({ bearing: [-0.51] });
  assert.equal(inputs.length, 7);
  assert(inputs.every(value => value.values instanceof Float32Array));
  assert.equal(Number(kernel.attemptedTurns()), 0, "GPU packet construction must not run CPU math");
  assert.deepEqual(kernel.state("state"), initial);
  console.log("WasmKernel: CPU turns, single-instance readback, rejection/rollback, reset and WGSL manifest passed.");
} finally {
  kernel.free();
}

if (process.argv[2]) {
  const repl = new WasmRepl();
  try {
    const loaded = repl.submit(await readFile(process.argv[2], "utf8"));
    assert.equal(loaded.result, null);
    assert(!loaded.events.some(envelope =>
      envelope.event.channel === "diagnostic" && envelope.event.event.severity === "error"
    ), JSON.stringify(loaded));
    for (const [mode, event, expected] of [
      [0, 0, 0], [0, 1, 1], [1, 0, 0], [1, 2, 2], [2, 1, 2], [2, 3, 0],
    ]) {
      const response = repl.submit(`#Robot(${mode}, ${event})`);
      assert.equal(Number(response.result?.inlineHtml), expected, JSON.stringify(response));
    }
    console.log("WasmRepl: actual Mech Robot FSM pause, run, reject, latch and reset transitions passed.");
  } finally {
    repl.shutdown();
    repl.free();
  }
}
