import assert from "node:assert/strict";

globalThis.GPUMapMode = { READ: 1 };
await import("../include/browser-compute.js");

const Lifecycle = globalThis.MechComputeSubmissionLifecycle;
const Device = globalThis.MechBrowserComputeDevice;

const beforeSubmission = new Lifecycle(7);
assert.equal(beforeSubmission.canAutoFallback(), true);
const constructionFailure = new Error("device lost before submission");
assert.equal(beforeSubmission.markFailed(constructionFailure), constructionFailure);
assert.equal(beforeSubmission.canAutoFallback(), true);

const inFlight = new Lifecycle(8);
inFlight.markSubmitted("8:1");
assert.equal(inFlight.canAutoFallback(), false);
const loss = new Error("device lost after submission");
assert.equal(inFlight.markFailed(loss), loss);
assert.equal(inFlight.canAutoFallback(), false);
assert.throws(() => inFlight.markAccepted("8:1"), loss);

const accepted = new Lifecycle(9);
accepted.markSubmitted("9:1");
accepted.markAccepted("9:1");
assert.equal(accepted.canAutoFallback(), false);
const acceptedLoss = accepted.markFailed(new Error("device lost after acceptance"));
assert.match(acceptedLoss.message, /after acceptance/);
assert.equal(accepted.canAutoFallback(), false);

const identity = new Lifecycle(10);
identity.markSubmitted("10:2");
assert.throws(
  () => identity.markAccepted("10:1"),
  /does not match the in-flight submission/,
);
identity.markAccepted("10:2");

const firstFailure = new Lifecycle(11);
const first = firstFailure.markFailed(new Error("first"));
assert.equal(firstFailure.markFailed(new Error("second")), first);
assert.equal(firstFailure.failure.message, "first");

const manifest = {
  bindings: [
    { elements: 15, role: "state-write" },
    { elements: 2, role: "integrity-fault" },
  ],
  outputs: [
    {
      name: "estimate",
      sampleDimensions: [15],
      physicalLayout: "row-major",
    },
    {
      name: "estimate-alias",
      sampleDimensions: [15],
      physicalLayout: "row-major",
    },
  ],
  physicalOutputs: [{
    id: 0,
    aliases: ["estimate", "estimate-alias"],
    sampleElements: 15,
  }],
  constraints: [{ code: 1, name: "finite-estimate!" }],
  dispatchElements: 1,
  workgroupSize: 64,
};
const supported = {
  maxStorageBuffersPerShaderStage: 8,
  maxComputeWorkgroupsPerDimension: 65535,
  maxStorageBufferBindingSize: 1024,
  maxBufferSize: 1024,
};
assert.equal(
  Device.requiredLimits(manifest, supported, ["estimate", "estimate-alias"]).maxBufferSize,
  68,
  "logical aliases must not multiply the physical readback allocation",
);
assert.equal(
  Device.requiredLimits(manifest, supported, []).maxBufferSize,
  68,
  "late-bound sampling must be admitted when the WebGPU device is created",
);

function fakeReadback(bytes) {
  return {
    async mapAsync() {},
    getMappedRange() { return bytes; },
    unmap() {},
  };
}

function fakeDeviceFor(bytes) {
  const device = Object.create(Device.prototype);
  device.readback = fakeReadback(bytes);
  device.readbackBytes = bytes.byteLength;
  device.metrics = {
    cpuToGpuInputBytes: 0,
    gpuToCpuReadbackBytes: 0,
    gpuToCpuOutputBytes: 0,
    logicalOutputs: 2,
    uniquePhysicalOutputBuffers: 1,
  };
  device.integrity = { elements: 2 };
  device.integrityOffset = 60;
  device.manifest = manifest;
  device.readbackPlan = [{
    output: manifest.outputs[0],
    aliases: manifest.outputs,
    offset: 0,
    bytes: 60,
  }];
  device.publishMetrics = () => {};
  return device;
}

const rejectedBytes = new ArrayBuffer(68);
new Uint32Array(rejectedBytes, 60, 2).set([1, (7 << 8) | 1]);
const rejectedDevice = fakeDeviceFor(rejectedBytes);
const rejected = await rejectedDevice.finish();
assert.deepEqual(rejected, {
  outputs: [],
  integrity: { constraint: "finite-estimate!", instance: 7 },
});
assert.equal(rejectedDevice.metrics.gpuToCpuReadbackBytes, 68);
assert.equal(rejectedDevice.metrics.gpuToCpuOutputBytes, 0);

const acceptedBytes = new ArrayBuffer(68);
new Float32Array(acceptedBytes, 0, 15).set(Array.from({ length: 15 }, (_, index) => index));
const acceptedDevice = fakeDeviceFor(acceptedBytes);
const acceptedResult = await acceptedDevice.finish();
assert.equal(acceptedResult.outputs.length, 2);
assert.equal(acceptedDevice.metrics.gpuToCpuReadbackBytes, 68);
assert.equal(acceptedDevice.metrics.gpuToCpuOutputBytes, 120);

const reportOnly = Object.create(Device.prototype);
reportOnly.readback = null;
reportOnly.device = { queue: { async onSubmittedWorkDone() {} } };
reportOnly.metrics = {
  cpuToGpuInputBytes: 0,
  gpuToCpuReadbackBytes: 0,
  gpuToCpuOutputBytes: 0,
  logicalOutputs: 0,
  uniquePhysicalOutputBuffers: 0,
};
reportOnly.publishMetrics = () => {};
assert.deepEqual(await reportOnly.finish(), { outputs: [], integrity: null });
assert.equal(reportOnly.metrics.gpuToCpuReadbackBytes, 0);

console.log("browser compute submission lifecycle tests passed");
