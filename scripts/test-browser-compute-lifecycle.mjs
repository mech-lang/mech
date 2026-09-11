import assert from "node:assert/strict";

globalThis.GPUMapMode = { READ: 1 };
globalThis.GPUBufferUsage = { COPY_DST: 1, MAP_READ: 2 };
await import("../include/browser-compute.js");

const { Device, ResetTracker, Session, awaitSmokeTargetCompletion } =
  globalThis.MechBrowserCompute;
assert.equal(Object.isFrozen(globalThis.MechBrowserCompute), true);
assert.equal(globalThis.MechComputeSubmissionLifecycle, undefined);
assert.equal(globalThis.MechComputeStateResetLedger, undefined);
assert.equal(globalThis.MechComputeStateResetTracker, undefined);
assert.equal(globalThis.MechBrowserComputeDevice, undefined);

let releaseLateCompletion;
const lateCompletionFailure = new Error("late submitted work failed");
const lateTarget = {
  computeSession: {
    completion: new Promise(resolve => {
      releaseLateCompletion = resolve;
    }),
  },
  computeResource: null,
  bridgeFailure: null,
  stopped: false,
  stop() {
    this.stopped = true;
  },
};
const lateSettlement = awaitSmokeTargetCompletion(lateTarget);
lateTarget.bridgeFailure = lateCompletionFailure;
releaseLateCompletion();
await assert.rejects(lateSettlement, /late submitted work failed/);
assert.equal(
  lateTarget.stopped,
  false,
  "a failed final completion must be reported before smoke teardown publishes success",
);

const successfulTarget = {
  computeSession: { completion: Promise.resolve() },
  computeResource: { disposeCompletion: Promise.resolve() },
  bridgeFailure: null,
  stopped: false,
  stop() {
    this.stopped = true;
  },
};
await awaitSmokeTargetCompletion(successfulTarget);
assert.equal(successfulTarget.stopped, true);

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
    { binding: 0, elements: 15, role: "state-write", memoryObject: 0 },
    { binding: 1, elements: 2, role: "integrity-fault", memoryObject: 1 },
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
    readbackDeviceObject: 2,
    readbackHostObject: 3,
  }],
  memoryAllocations: [
    { object: 0, capacityBytes: "60", space: "device" },
    { object: 1, capacityBytes: "8", space: "device" },
    { object: 2, capacityBytes: "60", space: "device" },
    { object: 3, capacityBytes: "60", space: "host" },
    { object: 4, capacityBytes: "8", space: "device" },
    { object: 5, capacityBytes: "8", space: "host" },
  ],
  integrityReadbackObjects: [4, 5],
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
  60,
  "separate readback and integrity buffers must require only the largest realized capacity",
);
assert.equal(
  Device.requiredLimits(manifest, supported, []).maxBufferSize,
  60,
  "late-bound sampling must be admitted when the WebGPU device is created",
);
const capacityManifest = {
  ...manifest,
  memoryAllocations: manifest.memoryAllocations.map((allocation) =>
    allocation.object === 2 ? { ...allocation, capacityBytes: "96" } : allocation),
};
assert.equal(
  Device.requiredLimits(capacityManifest, { ...supported, maxBufferSize: 96 }).maxBufferSize,
  96,
  "limit admission must use the capacity of the buffer that will be realized",
);
assert.throws(
  () => Device.requiredLimits(capacityManifest, { ...supported, maxBufferSize: 95 }),
  /Mech requires 96 for maxBufferSize, but this adapter supports 95/,
);

const creationManifest = {
  ...capacityManifest,
  planVersion: 1,
  physicalRevision: `sha256:${"a".repeat(64)}`,
  wgsl: "@compute @workgroup_size(1) fn main() {}",
};
let requestedLimits = null;
let deviceDestroyed = 0;
let bufferRealizations = 0;
let shaderRealizations = 0;
const undersizedDevice = {
  limits: { ...supported, maxBufferSize: 95 },
  createBuffer() {
    bufferRealizations += 1;
    throw new Error("device limits must be checked before buffer realization");
  },
  createShaderModule() {
    shaderRealizations += 1;
    throw new Error("device limits must be checked before shader realization");
  },
  destroy() { deviceDestroyed += 1; },
};
await assert.rejects(
  () => Device.create(creationManifest, {
    limits: supported,
    async requestDevice(descriptor) {
      requestedLimits = descriptor.requiredLimits;
      return undersizedDevice;
    },
  }),
  /Mech requires 96 for maxBufferSize, but this device supports 95/,
);
assert.equal(requestedLimits.maxBufferSize, 96);
assert.equal(bufferRealizations, 0);
assert.equal(shaderRealizations, 0);
assert.equal(deviceDestroyed, 1);

function fakeReadback(bytes) {
  let unmapped = 0;
  return {
    async mapAsync() {},
    getMappedRange() { return bytes; },
    unmap() { unmapped += 1; },
    get unmapped() { return unmapped; },
  };
}

function fakeDeviceFor(outputBytes, integrityBytes) {
  const device = Object.create(Device.prototype);
  const outputReadback = fakeReadback(outputBytes);
  const integrityReadback = fakeReadback(integrityBytes);
  const writes = [];
  device.readbackBytes = outputBytes.byteLength + integrityBytes.byteLength;
  device.metrics = {
    cpuToGpuInputBytes: 0,
    gpuToCpuReadbackBytes: 0,
    gpuToCpuOutputBytes: 0,
    logicalOutputs: 2,
    uniquePhysicalOutputBuffers: 1,
  };
  device.integrity = { elements: 2 };
  device.integrityReadback = integrityReadback;
  device.manifest = manifest;
  device.readbackPlan = [{
    output: manifest.outputs[0],
    aliases: manifest.outputs,
    physical: manifest.physicalOutputs[0],
    buffer: outputReadback,
    bytes: 60,
  }];
  device.memory = {
    stageHost(_object, source) { return Uint8Array.from(source); },
    recordWrite(object) { writes.push(object); },
    complete(hold) { hold.active = false; },
  };
  device.publishMetrics = () => {};
  device.testState = { integrityReadback, outputReadback, writes };
  return device;
}

function completedSubmission(writtenObjects = []) {
  return {
    completion: Promise.resolve(),
    queueCompletion: Promise.resolve(),
    mappingSettled: Promise.resolve([]),
    memoryHold: { active: true },
    writtenObjects,
  };
}

const rejectedOutputBytes = new ArrayBuffer(60);
const rejectedIntegrityBytes = new ArrayBuffer(8);
new Uint32Array(rejectedIntegrityBytes).set([1, (7 << 8) | 1]);
const rejectedDevice = fakeDeviceFor(rejectedOutputBytes, rejectedIntegrityBytes);
const rejected = await rejectedDevice.finish(completedSubmission([2, 4]));
assert.deepEqual(rejected, {
  outputs: [],
  integrity: { constraint: "finite-estimate!", instance: 7 },
});
assert.equal(rejectedDevice.metrics.gpuToCpuReadbackBytes, 68);
assert.equal(rejectedDevice.metrics.gpuToCpuOutputBytes, 0);
assert.equal(rejectedDevice.testState.outputReadback.unmapped, 1);
assert.equal(rejectedDevice.testState.integrityReadback.unmapped, 1);

const acceptedOutputBytes = new ArrayBuffer(60);
new Float32Array(acceptedOutputBytes).set(Array.from({ length: 15 }, (_, index) => index));
const acceptedDevice = fakeDeviceFor(acceptedOutputBytes, new ArrayBuffer(8));
const acceptedResult = await acceptedDevice.finish(completedSubmission([2, 4]));
assert.equal(acceptedResult.outputs.length, 2);
assert.equal(acceptedDevice.metrics.gpuToCpuReadbackBytes, 68);
assert.equal(acceptedDevice.metrics.gpuToCpuOutputBytes, 120);

const reportOnly = Object.create(Device.prototype);
reportOnly.readbackPlan = [];
reportOnly.integrity = null;
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
reportOnly.memory = {
  recordWrite() {},
  complete(hold) { hold.active = false; },
};
reportOnly.publishMetrics = () => {};
assert.deepEqual(
  await reportOnly.finish(completedSubmission()),
  { outputs: [], integrity: null },
);
assert.equal(reportOnly.metrics.gpuToCpuReadbackBytes, 0);
await assert.rejects(
  () => reportOnly.finish(),
  /exact submission promise/,
);

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((accept, decline) => {
    resolve = accept;
    reject = decline;
  });
  return { promise, reject, resolve };
}

const queueBoundary = deferred();
const laterQueueBoundary = deferred();
const failedUploadBoundary = deferred();
const slowMapping = deferred();
const firstMappingFailure = new Error("injected first mapping failure");
const queueEvents = [];
const recordedWrites = [];
let memoryCompletions = 0;
let activeMemoryHolds = 0;
let queueCompletionCalls = 0;
let firstUnmaps = 0;
let secondUnmaps = 0;
const queueResource = Object.create(Device.prototype);
queueResource.disposed = false;
queueResource.integrity = null;
queueResource.pipeline = {};
queueResource.bindGroups = [{}];
queueResource.metrics = {
  cpuToGpuInputBytes: 0,
  gpuToCpuReadbackBytes: 0,
  gpuToCpuOutputBytes: 0,
  logicalOutputs: 2,
  uniquePhysicalOutputBuffers: 2,
};
queueResource.manifest = {
  bindings: [{
    binding: 0,
    elements: 1,
    role: "input",
    access: "read-only",
    memoryObject: 10,
  }],
  states: [],
  dispatchElements: 1,
  workgroupSize: 1,
};
queueResource.inputBindings = new Map([["sample", {
  binding: queueResource.manifest.bindings[0],
  buffer: { name: "input" },
}]]);
queueResource.readbackPlan = [
  {
    bytes: 4,
    buffer: {
      mapAsync() {
        queueEvents.push("map-first");
        return Promise.reject(firstMappingFailure);
      },
      unmap() { firstUnmaps += 1; },
    },
    physical: { readbackDeviceObject: 20, readbackHostObject: 21 },
  },
  {
    bytes: 4,
    buffer: {
      mapAsync() {
        queueEvents.push("map-second");
        return slowMapping.promise;
      },
      unmap() { secondUnmaps += 1; },
    },
    physical: { readbackDeviceObject: 22, readbackHostObject: 23 },
  },
];
queueResource.outputBuffer = () => ({ name: "output" });
queueResource.publishMetrics = () => {};
queueResource.memory = {
  begin(objects) {
    queueEvents.push("hold");
    assert.deepEqual(objects, [10, 20, 21, 22, 23]);
    activeMemoryHolds += 1;
    return { active: true };
  },
  complete(hold) {
    assert.equal(hold.active, true);
    hold.active = false;
    activeMemoryHolds -= 1;
    memoryCompletions += 1;
  },
  recordWrite(object) { recordedWrites.push(object); },
};
queueResource.device = {
  createCommandEncoder() {
    return {
      beginComputePass() {
        return {
          setPipeline() {},
          setBindGroup() {},
          dispatchWorkgroups() {},
          end() {},
        };
      },
      copyBufferToBuffer() {},
      finish() {
        queueEvents.push("encode-finish");
        return { name: "command-buffer" };
      },
    };
  },
  queue: {
    writeBuffer() { queueEvents.push("write"); },
    submit() { queueEvents.push("submit"); },
    onSubmittedWorkDone() {
      queueCompletionCalls += 1;
      queueEvents.push(`queue-completion-${queueCompletionCalls}`);
      if (queueCompletionCalls === 1) return queueBoundary.promise;
      if (queueCompletionCalls === 2) return laterQueueBoundary.promise;
      return failedUploadBoundary.promise;
    },
  },
};

const queuedSubmission = queueResource.submit({
  inputs: [{ name: "sample", values: new Float32Array([1]) }],
}, 0);
assert.equal(queueEvents.indexOf("hold") < queueEvents.indexOf("write"), true);
assert.equal(queueEvents.indexOf("write") < queueEvents.indexOf("submit"), true);
assert.equal(queueEvents.indexOf("submit") < queueEvents.indexOf("queue-completion-1"), true);
assert.equal(queueEvents.indexOf("queue-completion-1") < queueEvents.indexOf("map-first"), true);
assert.equal(queuedSubmission.queueCompletion, queueBoundary.promise);
assert.equal(queueCompletionCalls, 1);

// A later presentation submission must not widen the compute ownership hold.
const laterCompletion = queueResource.device.queue.onSubmittedWorkDone();
const finishingQueuedSubmission = queueResource.finish(queuedSubmission);
await Promise.resolve();
await Promise.resolve();
assert.equal(firstUnmaps, 0);
assert.equal(secondUnmaps, 0);
assert.equal(memoryCompletions, 0);
queueBoundary.resolve();
await Promise.resolve();
assert.equal(memoryCompletions, 0, "one pending map must retain every submission pin");
slowMapping.resolve();
await assert.rejects(finishingQueuedSubmission, firstMappingFailure);
assert.equal(firstUnmaps, 1);
assert.equal(secondUnmaps, 1);
assert.equal(memoryCompletions, 1);
assert.deepEqual(
  recordedWrites,
  [10, 20, 22],
  "queue completion records GPU mutations even when readback mapping fails",
);
laterQueueBoundary.resolve();
await laterCompletion;

const preSubmitFailure = new Error("injected queue submission failure");
queueResource.device.queue.submit = () => {
  queueEvents.push("submit-failed");
  throw preSubmitFailure;
};
const failureEventStart = queueEvents.length;
assert.throws(
  () => queueResource.submit({
    inputs: [{ name: "sample", values: new Float32Array([2]) }],
  }, 0),
  preSubmitFailure,
);
const failureEvents = queueEvents.slice(failureEventStart);
assert.equal(failureEvents.indexOf("hold") < failureEvents.indexOf("write"), true);
assert.equal(failureEvents.indexOf("write") < failureEvents.indexOf("submit-failed"), true);
assert.equal(
  failureEvents.indexOf("submit-failed") < failureEvents.indexOf("queue-completion-3"),
  true,
);
assert.equal(memoryCompletions, 1, "an accepted upload must retain its ownership hold");
assert.equal(activeMemoryHolds, 1);

let queueResourceCloses = 0;
let queueResourceDestroys = 0;
queueResource.stateBuffers = new Map();
queueResource.fixedBuffers = new Map();
queueResource.readbackBuffers = new Map();
queueResource.integrityReadback = null;
queueResource.memory.close = () => {
  assert.equal(activeMemoryHolds, 0);
  queueResourceCloses += 1;
};
queueResource.device.destroy = () => { queueResourceDestroys += 1; };
queueResource.dispose();
assert.equal(queueResourceDestroys, 0, "retirement must wait for the accepted upload");
assert.equal(queueResourceCloses, 0);
failedUploadBoundary.resolve();
await queueResource.disposeCompletion;
assert.equal(memoryCompletions, 2);
assert.equal(activeMemoryHolds, 0);
assert.equal(queueResourceCloses, 1);
assert.equal(queueResourceDestroys, 1);
assert.deepEqual(recordedWrites, [10, 20, 22, 10]);

const accountingFailure = new Error("injected content-version exhaustion");
let accountingFailureReleases = 0;
const accountingFailureResource = Object.create(Device.prototype);
accountingFailureResource.readbackPlan = [];
accountingFailureResource.integrity = null;
accountingFailureResource.metrics = {
  cpuToGpuInputBytes: 0,
  gpuToCpuReadbackBytes: 0,
  gpuToCpuOutputBytes: 0,
  logicalOutputs: 0,
  uniquePhysicalOutputBuffers: 0,
};
accountingFailureResource.publishMetrics = () => {};
accountingFailureResource.memory = {
  recordWrite() { throw accountingFailure; },
  complete(hold) {
    hold.active = false;
    accountingFailureReleases += 1;
  },
};
await assert.rejects(
  accountingFailureResource.finish(completedSubmission([99])),
  accountingFailure,
);
assert.equal(
  accountingFailureReleases,
  1,
  "content-version failure must not leak the completed submission hold",
);

const staged = Object.create(Device.prototype);
let readbackDestroyed = 0;
const readback = { destroy() { readbackDestroyed += 1; } };
const previousPlan = [{ previous: true }];
staged.readbackBuffers = new Map([[0, readback]]);
staged.readbackPlan = previousPlan;
staged.readbackBytes = 44;
staged.readbackSignature = "previous";
staged.integrity = { elements: 2 };
staged.manifest = manifest;
staged.metrics = { logicalOutputs: 1, uniquePhysicalOutputBuffers: 1 };
assert.throws(
  () => staged.configureReadback(["missing-output"]),
  /unknown logical output missing-output/,
);
assert.equal(staged.readbackPlan, previousPlan);
assert.equal(staged.readbackBytes, 44);
assert.equal(staged.readbackSignature, "previous");
staged.configureReadback(["estimate", "estimate-alias"]);
assert.equal(staged.readbackPlan.length, 1);
assert.equal(staged.readbackPlan[0].buffer, readback);
assert.equal(staged.metrics.logicalOutputs, 2);
assert.equal(staged.metrics.uniquePhysicalOutputBuffers, 1);
assert.equal(readbackDestroyed, 0);

let integrityReadbackDestroyed = 0;
staged.integrityReadback = { destroy() { integrityReadbackDestroyed += 1; } };
staged.readbackBuffers.set(1, readback);
staged.disposed = false;
staged.stateBuffers = new Map();
staged.fixedBuffers = new Map();
staged.memory = { close() {} };
staged.device = { destroy() {} };
staged.dispose();
assert.equal(readbackDestroyed, 1);
assert.equal(integrityReadbackDestroyed, 1);

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

const submittedHookBoundary = deferred();
const submittedHookFailure = new Error("injected submitted hook failure");
let submittedHookFinishes = 0;
let submittedHookDisposals = 0;
const submittedHookCompletions = [];
const submittedHookSession = new Session({
  generation: 23,
  controller: {
    completeComputeCommand(payload) { submittedHookCompletions.push(payload); },
  },
  resource: {
    physicalRevision: "sha256:submitted-hook-failure",
    device: { lost: new Promise(() => {}) },
    setRequestedOutputs() {},
    submit() {
      return { outputIndex: 1, completion: submittedHookBoundary.promise };
    },
    async finish(submission) {
      submittedHookFinishes += 1;
      await submission.completion;
      return { outputs: [], integrity: null };
    },
    dispose() { submittedHookDisposals += 1; },
  },
});
assert.throws(
  () => submittedHookSession.submit(command("23:1"), {
    onSubmitted() { throw submittedHookFailure; },
  }),
  submittedHookFailure,
);
assert.equal(submittedHookFinishes, 1);
assert.equal(submittedHookSession.pending, true);
assert.deepEqual(submittedHookCompletions, [{
  version: 1,
  token: "23:1",
  status: "failed",
  failure: { reason: submittedHookFailure.message },
}]);
submittedHookSession.retire();
assert.equal(submittedHookDisposals, 0, "retirement must wait for accepted work cleanup");
submittedHookBoundary.resolve();
await submittedHookSession.completion;
await Promise.resolve();
assert.equal(submittedHookSession.pending, false);
assert.equal(submittedHookDisposals, 1);

const terminalRetirementBoundary = deferred();
let terminalRetirementFinishes = 0;
let terminalRetirementDisposals = 0;
const terminalRetirementCompletions = [];
let terminalRetirementSession;
terminalRetirementSession = new Session({
  generation: 24,
  controller: {
    completeComputeCommand(payload) { terminalRetirementCompletions.push(payload); },
  },
  resource: {
    physicalRevision: "sha256:terminal-retirement",
    device: { lost: new Promise(() => {}) },
    setRequestedOutputs() {},
    submit() {
      return { outputIndex: 1, completion: terminalRetirementBoundary.promise };
    },
    async finish(submission) {
      terminalRetirementFinishes += 1;
      await submission.completion;
      return { outputs: [], integrity: null };
    },
    dispose() { terminalRetirementDisposals += 1; },
  },
});
terminalRetirementSession.submit(command("24:1"), {
  onSubmitted() { terminalRetirementSession.retire(); },
});
assert.equal(terminalRetirementSession.pending, true);
assert.equal(terminalRetirementFinishes, 1);
assert.equal(
  terminalRetirementDisposals,
  0,
  "synchronous retirement from the submitted hook must retain accepted work",
);
terminalRetirementBoundary.resolve();
await terminalRetirementSession.completion;
await Promise.resolve();
assert.equal(terminalRetirementSession.pending, false);
assert.equal(terminalRetirementDisposals, 1);
assert.deepEqual(
  terminalRetirementCompletions,
  [],
  "a retired generation must not publish its terminal completion",
);

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
