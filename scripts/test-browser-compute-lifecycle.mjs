import assert from "node:assert/strict";

globalThis.GPUMapMode = { READ: 1 };
globalThis.GPUBufferUsage = { COPY_DST: 1, MAP_READ: 2 };
await import("../include/browser-compute.js");

const { Device, ResetTracker, Session } = globalThis.MechBrowserCompute;
assert.equal(Object.isFrozen(globalThis.MechBrowserCompute), true);
assert.equal(globalThis.MechComputeSubmissionLifecycle, undefined);
assert.equal(globalThis.MechComputeStateResetLedger, undefined);
assert.equal(globalThis.MechComputeStateResetTracker, undefined);
assert.equal(globalThis.MechBrowserComputeDevice, undefined);

const resets = new ResetTracker();
assert.equal(
  resets.advance({ present: true, generation: 1, revision: "sha256:same" }),
  null,
);
assert.equal(
  resets.advance({ present: true, generation: 2, revision: "sha256:same" }),
  null,
);
assert.deepEqual(
  resets.advance({ present: true, generation: 3, revision: "sha256:new" }),
  {
    previousRevision: "sha256:same",
    nextRevision: "sha256:new",
    resetCount: 1,
  },
);
assert.equal(
  resets.advance({ present: true, generation: 4, revision: "sha256:new" }),
  null,
  "one physical plan transition must publish exactly one reset",
);

const scalarResets = new ResetTracker();
assert.equal(
  scalarResets.advance({
    present: true,
    generation: 1,
    revision: "sha256:scalar-old",
  }),
  null,
  "the first scalar generation seeds reset tracking",
);
assert.deepEqual(
  scalarResets.advance({
    present: true,
    generation: 2,
    revision: "sha256:scalar-new",
  }),
  {
    previousRevision: "sha256:scalar-old",
    nextRevision: "sha256:scalar-new",
    resetCount: 1,
  },
  "scalar physical replacement must report the same reset as WebGPU",
);
assert.equal(
  scalarResets.advance({
    present: true,
    generation: 3,
    revision: "sha256:scalar-new",
  }),
  null,
  "a compatible scalar replacement must preserve state without a reset",
);

const lifecycle = (generation) => new Session({
  controller: { completeComputeCommand() {} },
  resource: null,
  generation,
}).lifecycle;

const beforeSubmission = lifecycle(7);
assert.equal(beforeSubmission.canAutoFallback(), true);
const constructionFailure = new Error("device lost before submission");
assert.equal(beforeSubmission.markFailed(constructionFailure), constructionFailure);
assert.equal(beforeSubmission.canAutoFallback(), true);

const inFlight = lifecycle(8);
inFlight.markSubmitted("8:1");
assert.equal(inFlight.canAutoFallback(), false);
const loss = new Error("device lost after submission");
assert.equal(inFlight.markFailed(loss), loss);
assert.equal(inFlight.canAutoFallback(), false);
assert.throws(() => inFlight.markAccepted("8:1"), loss);

const accepted = lifecycle(9);
accepted.markSubmitted("9:1");
accepted.markAccepted("9:1");
assert.equal(accepted.canAutoFallback(), false);
const acceptedLoss = accepted.markFailed(new Error("device lost after acceptance"));
assert.match(acceptedLoss.message, /after acceptance/);
assert.equal(accepted.canAutoFallback(), false);

const identity = lifecycle(10);
identity.markSubmitted("10:2");
assert.throws(
  () => identity.markAccepted("10:1"),
  /does not match the in-flight submission/,
);
identity.markAccepted("10:2");

const firstFailure = lifecycle(11);
const first = firstFailure.markFailed(new Error("first"));
assert.equal(firstFailure.markFailed(new Error("second")), first);
assert.equal(firstFailure.failure.message, "first");

const manifest = {
  physicalRevision: "sha256:stable-plan",
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

const reusable = Object.create(Device.prototype);
reusable.disposed = false;
reusable.physicalRevision = manifest.physicalRevision;
reusable.manifest = manifest;
assert.equal(reusable.compatibleWith({ ...manifest }), true);
assert.equal(
  reusable.compatibleWith({ ...manifest, physicalRevision: "sha256:changed-plan" }),
  false,
);
const replacementManifest = { ...manifest };
reusable.adoptManifest(replacementManifest);
assert.equal(reusable.manifest, replacementManifest);
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
const rejected = await rejectedDevice.finish(Promise.resolve());
assert.deepEqual(rejected, {
  outputs: [],
  integrity: { constraint: "finite-estimate!", instance: 7 },
});
assert.equal(rejectedDevice.metrics.gpuToCpuReadbackBytes, 68);
assert.equal(rejectedDevice.metrics.gpuToCpuOutputBytes, 0);

const acceptedBytes = new ArrayBuffer(68);
new Float32Array(acceptedBytes, 0, 15).set(Array.from({ length: 15 }, (_, index) => index));
const acceptedDevice = fakeDeviceFor(acceptedBytes);
const acceptedResult = await acceptedDevice.finish(Promise.resolve());
assert.equal(acceptedResult.outputs.length, 2);
assert.equal(acceptedDevice.metrics.gpuToCpuReadbackBytes, 68);
assert.equal(acceptedDevice.metrics.gpuToCpuOutputBytes, 120);

const reportOnly = Object.create(Device.prototype);
reportOnly.readback = null;
reportOnly.device = {
  queue: {
    async onSubmittedWorkDone() {
      throw new Error("finish must not widen completion to later queue work");
    },
  },
};
reportOnly.metrics = {
  cpuToGpuInputBytes: 0,
  gpuToCpuReadbackBytes: 0,
  gpuToCpuOutputBytes: 0,
  logicalOutputs: 0,
  uniquePhysicalOutputBuffers: 0,
};
reportOnly.publishMetrics = () => {};
assert.deepEqual(
  await reportOnly.finish(Promise.resolve()),
  { outputs: [], integrity: null },
);
assert.equal(reportOnly.metrics.gpuToCpuReadbackBytes, 0);
await assert.rejects(
  () => reportOnly.finish(),
  /exact submission promise/,
);

const staged = Object.create(Device.prototype);
let previousReadbackDestroyed = 0;
const previousReadback = { destroy() { previousReadbackDestroyed += 1; } };
staged.readback = previousReadback;
staged.readbackPlan = [{ previous: true }];
staged.readbackBytes = 44;
staged.readbackSignature = "previous";
staged.integrityOffset = 36;
staged.integrity = { elements: 2 };
staged.manifest = manifest;
staged.metrics = { logicalOutputs: 1, uniquePhysicalOutputBuffers: 1 };
staged.device = {
  createBuffer() { throw new Error("injected readback allocation failure"); },
};
assert.throws(
  () => staged.configureReadback(["missing-output"]),
  /unknown logical output missing-output/,
);
assert.equal(staged.readback, previousReadback);
assert.equal(previousReadbackDestroyed, 0);
assert.throws(
  () => staged.configureReadback(["estimate"]),
  /injected readback allocation failure/,
);
assert.equal(staged.readback, previousReadback);
assert.deepEqual(staged.readbackPlan, [{ previous: true }]);
assert.equal(staged.readbackBytes, 44);
assert.equal(staged.readbackSignature, "previous");
assert.equal(staged.integrityOffset, 36);
assert.equal(previousReadbackDestroyed, 0);

let replacementReadbackDestroyed = 0;
const replacementReadback = { destroy() { replacementReadbackDestroyed += 1; } };
staged.device.createBuffer = () => replacementReadback;
staged.configureReadback(["estimate", "estimate-alias"]);
assert.equal(staged.readback, replacementReadback);
assert.equal(staged.readbackPlan.length, 1);
assert.equal(staged.metrics.logicalOutputs, 2);
assert.equal(staged.metrics.uniquePhysicalOutputBuffers, 1);
assert.equal(previousReadbackDestroyed, 1);
assert.equal(replacementReadbackDestroyed, 0);
staged.disposed = false;
staged.stateBuffers = new Map();
staged.fixedBuffers = new Map();
staged.device.destroy = () => {};
staged.dispose();
assert.equal(replacementReadbackDestroyed, 1);

const command = (token) => ({
  dispatch: true,
  acknowledgementRequired: true,
  dispatchToken: token,
  requestedOutputs: ["estimate"],
  inputs: [],
});

function failingSession(stage, failure) {
  const completions = [];
  let submissions = 0;
  const resource = {
    physicalRevision: "sha256:failure-injection",
    device: { lost: new Promise(() => {}) },
    setRequestedOutputs() {
      if (stage === "readback") throw failure;
    },
    submit() {
      submissions += 1;
      if (stage === "submit") throw failure;
      return { outputIndex: 1, completion: Promise.resolve() };
    },
    async finish() { return { outputs: [], integrity: null }; },
  };
  const session = new Session({
    controller: { completeComputeCommand(payload) { completions.push(payload); } },
    resource,
    generation: 21,
  });
  return { session, completions, submissions: () => submissions };
}

for (const [stage, reason] of [
  ["readback", "requested-output validation or allocation failed"],
  ["submit", "input upload, encoder, or queue submission failed"],
]) {
  const injected = new Error(reason);
  const fixture = failingSession(stage, injected);
  assert.throws(() => fixture.session.submit(command("21:1")), injected);
  assert.equal(fixture.session.pending, false);
  assert.equal(fixture.session.failure, injected);
  assert.equal(fixture.submissions(), stage === "readback" ? 0 : 1);
  assert.deepEqual(fixture.completions, [{
    version: 1,
    token: "21:1",
    status: "failed",
    failure: { reason },
  }]);
  assert.throws(
    () => fixture.session.submit(command("21:2")),
    injected,
    "terminal transport failure must reject later commands without a second completion",
  );
  assert.equal(fixture.completions.length, 1);
}

const publicationAttempts = [];
const publicationSession = new Session({
  generation: 22,
  controller: {
    completeComputeCommand(payload) {
      publicationAttempts.push(payload);
      if (payload.status === "completed") {
        throw new Error("injected completion publication failure");
      }
    },
  },
  resource: {
    physicalRevision: "sha256:publication-failure",
    device: { lost: new Promise(() => {}) },
    setRequestedOutputs() {},
    submit() { return { outputIndex: 1, completion: Promise.resolve() }; },
    async finish() { return { outputs: [], integrity: null }; },
  },
});
publicationSession.submit(command("22:1"));
await publicationSession.completion;
assert.equal(publicationSession.pending, false);
assert.match(publicationSession.failure.message, /completion publication failure/);
assert.deepEqual(publicationAttempts.map(({ status }) => status), ["completed", "failed"]);
assert.throws(() => publicationSession.submit(command("22:2")), publicationSession.failure);

console.log("browser compute submission lifecycle tests passed");
