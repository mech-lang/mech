/* Shared WebGPU resource and completion protocol for Mech browser hosts. */
globalThis.MechBrowserCompute ||= (() => {
let resourceSequence = 0;
let pipelineBuildCount = 0;

function validateSupportedLimits(required, supported, source) {
  for (const [name, value] of Object.entries(required)) {
    if (!Number.isSafeInteger(value) || value <= 0) {
      throw new Error(`Mech computed an invalid ${name} requirement: ${value}`);
    }
    const limit = Number(supported?.[name]);
    if (!Number.isFinite(limit) || limit <= 0) {
      throw new Error(`this WebGPU ${source} does not report the required ${name} limit`);
    }
    if (value > limit) {
      throw new Error(`Mech requires ${value} for ${name}, but this ${source} supports ${limit}`);
    }
  }
}

function validateInputs(resource, command) {
  const inputs = [];
  for (const input of command.inputs) {
    const target = resource.inputBindings.get(input.name);
    if (!target) throw new Error(`Mech wrote undeclared GPU input ${input.name}`);
    if (input.values.length !== target.binding.elements) {
      throw new Error(
        `Mech wrote ${input.values.length} values to ${input.name}; expected ${target.binding.elements}`,
      );
    }
    inputs.push({ input, target });
  }
  return inputs;
}

function writeInputs(resource, inputs, onWritten = null) {
  for (const { input, target } of inputs) {
    resource.device.queue.writeBuffer(target.buffer, 0, input.values);
    resource.metrics.cpuToGpuInputBytes += input.values.byteLength;
    onWritten?.(target.binding.memoryObject);
  }
}

function captureQueueCompletion(device) {
  try {
    return Promise.resolve(device.queue.onSubmittedWorkDone());
  } catch (error) {
    return Promise.reject(error);
  }
}

async function releaseAcceptedQueueWork(resource, completion, memoryHold, writtenObjects) {
  const [queueResult] = await Promise.allSettled([completion]);
  try {
    if (queueResult.status === "fulfilled") {
      writtenObjects.forEach((object, index) => {
        if (writtenObjects.indexOf(object) === index) resource.memory.recordWrite(object);
      });
    }
  } finally {
    resource.memory.complete(memoryHold);
  }
}

function trackQueueCleanup(resource, cleanup) {
  const tracked = Promise.resolve(cleanup).catch((error) => {
    resource.queueCleanupFailure ||= error instanceof Error ? error : new Error(String(error));
  });
  resource.pendingQueueCleanups ||= new Set();
  resource.pendingQueueCleanups.add(tracked);
  tracked.finally(() => resource.pendingQueueCleanups.delete(tracked));
}

function destroyDeviceResource(resource) {
  for (const buffers of resource.stateBuffers.values()) {
    buffers.forEach((buffer) => buffer.destroy());
  }
  for (const buffer of resource.fixedBuffers.values()) buffer.destroy();
  const destroyedReadbacks = new Set();
  for (const buffer of resource.readbackBuffers.values()) {
    if (!destroyedReadbacks.has(buffer)) buffer.destroy();
    destroyedReadbacks.add(buffer);
  }
  resource.integrityReadback?.destroy();
  resource.memory.close();
  resource.device.destroy();
}

class SubmissionLifecycle {
  constructor(generation) {
    this.generation = String(generation);
    this.phase = "ready";
    this.submitted = false;
    this.inFlight = null;
    this.failure = null;
  }

  markSubmitted(identity) {
    if (this.phase === "failed") {
      throw this.failure;
    }
    if (this.inFlight !== null) {
      throw new Error("a checked document compute dispatch is already in flight");
    }
    this.submitted = true;
    this.inFlight = String(identity);
    this.phase = "in-flight";
  }

  markAccepted(identity) {
    if (this.phase === "failed") {
      throw this.failure;
    }
    if (this.inFlight !== String(identity)) {
      throw new Error("compute completion does not match the in-flight submission");
    }
    this.inFlight = null;
    this.phase = "ready";
  }

  markFailed(reason) {
    if (this.phase !== "failed") {
      this.failure = reason instanceof Error ? reason : new Error(String(reason));
      this.phase = "failed";
    }
    return this.failure;
  }

  canAutoFallback() {
    return !this.submitted;
  }
}

class StateResetLedger {
  constructor() {
    this.transitions = new Set();
    this.count = 0;
  }

  record(previousGeneration, nextGeneration, previousRevision, nextRevision) {
    const previous = String(previousRevision || "none");
    const next = String(nextRevision || "none");
    if (previous === next) return null;
    const transition = [
      String(previousGeneration || "none"),
      String(nextGeneration || "none"),
      previous,
      next,
    ].join(":");
    if (this.transitions.has(transition)) return null;
    this.transitions.add(transition);
    this.count += 1;
    return { previousRevision: previous, nextRevision: next, resetCount: this.count };
  }
}

// Tracks the physical compute identity independently of the WebGPU transport.
// Scalar compute intentionally has no DocumentComputeBridge, but it still owns
// persistent resident state and must report incompatible replacement exactly
// like WebGPU does.
class ResetTracker {
  constructor(ledger = new StateResetLedger()) {
    this.ledger = ledger;
    this.previous = null;
  }

  advance(identity) {
    const next = {
      present: identity?.present === true,
      generation: String(identity?.generation || "none"),
      revision: String(identity?.revision || "none"),
    };
    const previous = this.previous;
    this.previous = next;
    if (!previous?.present) return null;
    return this.ledger.record(
      previous.generation,
      next.generation,
      previous.revision,
      next.revision,
    );
  }
}

class ManagedMemory {
  constructor(manifest) {
    this.closed = false;
    this.lost = false;
    this.records = new Map();
    for (const allocation of manifest.memoryAllocations || []) {
      const id = Number(allocation.object);
      const capacity = Number(allocation.capacityBytes);
      if (!Number.isSafeInteger(id) || id < 0 || !Number.isSafeInteger(capacity) || capacity < 0) {
        throw new Error("browser GPU memory plan contains an invalid object or capacity");
      }
      if (this.records.has(id)) throw new Error(`browser GPU memory plan repeats object ${id}`);
      this.records.set(id, {
        id,
        capacity,
        space: allocation.space,
        lifetime: allocation.lifetime,
        backing: allocation.space === "host" ? new ArrayBuffer(capacity) : null,
        attached: allocation.space !== "device",
        inFlight: 0,
        contentVersion: 0,
      });
    }
  }

  record(id) {
    const record = this.records.get(Number(id));
    if (!record) throw new Error(`browser GPU memory plan has no object ${id}`);
    return record;
  }

  attachDevice(id, buffer) {
    const record = this.record(id);
    if (record.space !== "device") throw new Error(`memory object ${id} is not device storage`);
    if (record.attached) throw new Error(`device memory object ${id} was attached twice`);
    if (Number(buffer?.size) !== record.capacity) {
      throw new Error(
        `device memory object ${id} has ${Number(buffer?.size)} bytes; planned ${record.capacity}`,
      );
    }
    record.attached = true;
    record.buffer = buffer;
  }

  begin(objectIds) {
    if (this.closed) throw new Error("browser GPU memory domain is closed");
    if (this.lost) throw new Error("browser GPU device ownership is lost");
    const records = [...new Set(objectIds.map(Number))]
      .sort((left, right) => left - right)
      .map((id) => this.record(id));
    for (const record of records) {
      if (!record.attached) throw new Error(`memory object ${record.id} is not realized`);
      if (record.inFlight === Number.MAX_SAFE_INTEGER) {
        throw new Error("browser GPU submission pin count exhausted");
      }
    }
    for (const record of records) record.inFlight += 1;
    return { records, active: true };
  }

  complete(hold) {
    if (!hold?.active) throw new Error("browser GPU submission hold was already completed");
    for (const record of hold.records) {
      if (record.inFlight <= 0) throw new Error("browser GPU submission accounting underflow");
    }
    for (const record of hold.records) record.inFlight -= 1;
    hold.active = false;
  }

  recordWrite(id) {
    const record = this.record(id);
    if (record.contentVersion === Number.MAX_SAFE_INTEGER) {
      throw new Error(`device content version exhausted for object ${id}`);
    }
    record.contentVersion += 1;
  }

  stageHost(id, source) {
    const record = this.record(id);
    if (record.space !== "host" || !(record.backing instanceof ArrayBuffer)) {
      throw new Error(`memory object ${id} is not a managed host transfer`);
    }
    if (source.byteLength > record.capacity) {
      throw new Error(
        `host transfer ${id} needs ${source.byteLength} bytes; planned ${record.capacity}`,
      );
    }
    const target = new Uint8Array(record.backing, 0, source.byteLength);
    target.set(source);
    return target;
  }

  markLost() {
    this.lost = true;
  }

  close() {
    if ([...this.records.values()].some((record) => record.inFlight !== 0)) {
      throw new Error("browser GPU memory domain closed with a submission in flight");
    }
    this.closed = true;
    this.records.clear();
  }
}

class Device {
  static logicalOutputValues(output, physicalValues) {
    const dimensions = (output.sampleDimensions || []).map(Number);
    if (output.physicalLayout !== "column-major" || dimensions.length < 2) {
      return Float32Array.from(physicalValues);
    }
    const elements = dimensions.reduce((product, dimension) => product * dimension, 1);
    if (elements !== physicalValues.length) {
      throw new Error(
        `compute output ${output.name} has ${physicalValues.length} physical values; expected ${elements}`,
      );
    }
    const logical = new Float32Array(elements);
    const columnMajorStrides = [];
    let columnMajorStride = 1;
    for (const dimension of dimensions) {
      columnMajorStrides.push(columnMajorStride);
      columnMajorStride *= dimension;
    }
    for (let rowMajorIndex = 0; rowMajorIndex < elements; rowMajorIndex += 1) {
      let remaining = rowMajorIndex;
      let columnMajorIndex = 0;
      for (let axis = dimensions.length - 1; axis >= 0; axis -= 1) {
        const coordinate = remaining % dimensions[axis];
        remaining = Math.floor(remaining / dimensions[axis]);
        columnMajorIndex += coordinate * columnMajorStrides[axis];
      }
      logical[rowMajorIndex] = physicalValues[columnMajorIndex];
    }
    return logical;
  }

  static requiredLimits(manifest, supported) {
    const bindingBytes = manifest.bindings.map((binding) =>
      Math.max(4, Number(binding.elements) * Float32Array.BYTES_PER_ELEMENT));
    // Output sampling is selected per turn, after device creation. Reserve
    // enough address space for every legal readback buffer up front so a later
    // sample request cannot exceed the limits admitted for this device. R6
    // realizes one buffer per device memory object, not one combined readback.
    const readbackBytes = (manifest.physicalOutputs || []).map((output) =>
      Number(output.sampleElements) * Float32Array.BYTES_PER_ELEMENT);
    const integrityBytes = Number(
      manifest.bindings.find((binding) => binding.role === "integrity-fault")?.elements || 0,
    ) * Uint32Array.BYTES_PER_ELEMENT;
    const plannedDeviceBytes = (manifest.memoryAllocations || [])
      .filter((allocation) => allocation.space === "device")
      .map((allocation) => Number(allocation.capacityBytes));
    const required = {
      maxStorageBuffersPerShaderStage: manifest.bindings.length,
      maxComputeWorkgroupsPerDimension: Math.ceil(
        manifest.dispatchElements / manifest.workgroupSize,
      ),
      maxStorageBufferBindingSize: Math.max(...bindingBytes, 4),
      maxBufferSize: Math.max(
        ...bindingBytes,
        ...readbackBytes,
        ...plannedDeviceBytes,
        integrityBytes,
        4,
      ),
    };
    validateSupportedLimits(required, supported, "adapter");
    return required;
  }

  static async create(manifest, adapter, requestedOutputNames = []) {
    if (Number(manifest.planVersion) !== 1) {
      throw new Error(
        `unsupported GPU execution plan version ${manifest.planVersion}; expected 1`,
      );
    }
    if (!/^sha256:[0-9a-f]{64}$/.test(String(manifest.physicalRevision || ""))) {
      throw new Error("GPU execution plan omitted its stable physical revision");
    }
    const requiredLimits = this.requiredLimits(
      manifest,
      adapter.limits,
    );
    const device = await adapter.requestDevice({ requiredLimits });
    try {
      validateSupportedLimits(requiredLimits, device.limits, "device");
      const module = device.createShaderModule({ code: manifest.wgsl });
      const compilation = await module.getCompilationInfo();
      const errors = compilation.messages.filter((message) => message.type === "error");
      if (errors.length) {
        throw new Error(errors.map((message) => message.message).join("\n"));
      }
      const descriptor = {
        layout: "auto",
        compute: { module, entryPoint: "main" },
      };
      const pipeline = typeof device.createComputePipelineAsync === "function"
        ? await device.createComputePipelineAsync(descriptor)
        : device.createComputePipeline(descriptor);
      return new this(manifest, device, pipeline, requestedOutputNames);
    } catch (error) {
      device.destroy();
      throw error;
    }
  }

  constructor(manifest, device, pipeline, requestedOutputNames = []) {
    this.manifest = manifest;
    this.device = device;
    this.pipeline = pipeline;
    this.physicalRevision = String(manifest.physicalRevision || "");
    resourceSequence += 1;
    pipelineBuildCount += 1;
    this.resourceIdentity = String(resourceSequence);
    this.deviceIdentity = `device-${this.resourceIdentity}`;
    this.pipelineIdentity =
      `pipeline-${pipelineBuildCount}`;
    this.stateIdentity = `state-${this.resourceIdentity}`;
    this.pipelineBuildCount = pipelineBuildCount;
    this.disposed = false;
    this.pendingQueueCleanups = new Set();
    this.queueCleanupFailure = null;
    this.disposeCompletion = null;
    this.memory = new ManagedMemory(manifest);
    this.metrics = {
      cpuToGpuInputBytes: 0,
      gpuToCpuReadbackBytes: 0,
      gpuToCpuOutputBytes: 0,
      logicalOutputs: requestedOutputNames.length,
      uniquePhysicalOutputBuffers: 0,
    };
    this.createBuffers(requestedOutputNames);
  }

  compatibleWith(manifest) {
    return !this.disposed && this.physicalRevision !== "" &&
      this.physicalRevision === String(manifest?.physicalRevision || "");
  }

  adoptManifest(manifest) {
    if (!this.compatibleWith(manifest)) {
      throw new Error("cannot adopt an incompatible GPU execution plan");
    }
    this.manifest = manifest;
  }

  createBuffers(requestedOutputNames) {
    this.stateBuffers = new Map();
    for (const state of this.manifest.states) {
      const buffers = [0, 1].map(() => this.device.createBuffer({
        size: Math.max(4, state.elements * Float32Array.BYTES_PER_ELEMENT),
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
      }));
      if (!Array.isArray(state.memoryObjects) || state.memoryObjects.length !== 2) {
        throw new Error(`GPU state ${state.slot} has no managed double-buffer identity`);
      }
      this.memory.attachDevice(state.memoryObjects[0], buffers[0]);
      this.memory.attachDevice(state.memoryObjects[1], buffers[1]);
      this.device.queue.writeBuffer(buffers[0], 0, state.initialValues);
      this.stateBuffers.set(state.slot, buffers);
    }
    this.fixedBuffers = new Map();
    this.inputBindings = new Map();
    for (const binding of this.manifest.bindings) {
      if (binding.role === "state-read" || binding.role === "state-write") continue;
      const buffer = this.device.createBuffer({
        size: Math.max(4, binding.elements * Float32Array.BYTES_PER_ELEMENT),
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
      });
      this.memory.attachDevice(binding.memoryObject, buffer);
      if (binding.initialValues) this.device.queue.writeBuffer(buffer, 0, binding.initialValues);
      this.fixedBuffers.set(binding.binding, buffer);
      if (binding.role === "input") this.inputBindings.set(binding.name, { binding, buffer });
    }
    this.bindGroups = [0, 1].map((sourceIndex) => this.device.createBindGroup({
      layout: this.pipeline.getBindGroupLayout(0),
      entries: this.manifest.bindings.map((binding) => ({
        binding: binding.binding,
        resource: { buffer: this.bufferForBinding(binding, sourceIndex) },
      })),
    }));
    this.integrity = this.manifest.bindings.find(
      (binding) => binding.role === "integrity-fault",
    );
    this.readbackBuffers = new Map();
    const readbackBuffersByObject = new Map();
    for (const physical of this.manifest.physicalOutputs || []) {
      let buffer = readbackBuffersByObject.get(physical.readbackDeviceObject);
      if (!buffer) {
        const capacity = this.memory.record(physical.readbackDeviceObject).capacity;
        buffer = this.device.createBuffer({
          size: capacity,
          usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
        });
        this.memory.attachDevice(physical.readbackDeviceObject, buffer);
        readbackBuffersByObject.set(physical.readbackDeviceObject, buffer);
      }
      this.readbackBuffers.set(physical.id, buffer);
    }
    this.integrityReadback = null;
    if (this.integrity) {
      const objects = this.manifest.integrityReadbackObjects;
      if (!Array.isArray(objects) || objects.length !== 2) {
        throw new Error("GPU integrity binding has no managed transfer identity");
      }
      const bytes = this.integrity.elements * Uint32Array.BYTES_PER_ELEMENT;
      this.integrityReadback = this.device.createBuffer({
        size: bytes,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
      });
      this.memory.attachDevice(objects[0], this.integrityReadback);
    }
    this.configureReadback(requestedOutputNames);
  }

  configureReadback(requestedOutputNames) {
    const requested = new Set(requestedOutputNames);
    for (const name of requested) {
      if (!this.manifest.outputs.some((output) => output.name === name)) {
        throw new Error(`GPU readback requested unknown logical output ${name}`);
      }
    }
    const readbackPlan = [];
    const planned = new Set();
    let byteLength = 0;
    for (const physical of this.manifest.physicalOutputs || []) {
      const aliases = physical.aliases
        .filter((name) => requested.has(name))
        .map((name) => this.manifest.outputs.find((output) => output.name === name));
      if (!aliases.length) continue;
      if (aliases.some((output) => !output)) {
        throw new Error(`GPU physical output ${physical.id} names an unknown logical alias`);
      }
      const bytes = physical.sampleElements * Float32Array.BYTES_PER_ELEMENT;
      readbackPlan.push({
        output: aliases[0],
        physical,
        buffer: this.readbackBuffers.get(physical.id),
        bytes,
        aliases,
      });
      aliases.forEach((output) => planned.add(output.name));
      byteLength += bytes;
    }
    for (const name of requested) {
      if (!planned.has(name)) {
        throw new Error(`GPU logical output ${name} has no physical readback allocation`);
      }
    }
    if (this.integrity) byteLength += this.integrity.elements * Uint32Array.BYTES_PER_ELEMENT;
    this.readbackPlan = readbackPlan;
    this.readbackBytes = byteLength;
    this.readbackSignature = [...requestedOutputNames].sort().join("\u0000");
    this.metrics.logicalOutputs = requestedOutputNames.length;
    this.metrics.uniquePhysicalOutputBuffers = readbackPlan.length;
  }

  setRequestedOutputs(requestedOutputNames) {
    const signature = [...requestedOutputNames].sort().join("\u0000");
    if (signature !== this.readbackSignature) this.configureReadback(requestedOutputNames);
  }

  bufferForBinding(binding, sourceIndex) {
    if (binding.role === "state-read") return this.stateBuffers.get(binding.slot)[sourceIndex];
    if (binding.role === "state-write") return this.stateBuffers.get(binding.slot)[1 - sourceIndex];
    return this.fixedBuffers.get(binding.binding);
  }

  outputBuffer(index, physical) {
    if (this.stateBuffers.has(physical.slot)) return this.stateBuffers.get(physical.slot)[index];
    if (!Number.isInteger(physical.binding)) {
      throw new Error(`compute physical output ${physical.id} has no buffer binding`);
    }
    return this.fixedBuffers.get(physical.binding);
  }

  applyInputs(command) {
    writeInputs(this, validateInputs(this, command));
  }

  submit(command, activeBuffer) {
    if (this.disposed) throw new Error("the WebGPU compute device is disposed");
    const inputs = validateInputs(this, command);
    const outputIndex = 1 - activeBuffer;
    const writtenObjects = inputs.map(({ target }) => target.binding.memoryObject);
    for (const binding of this.manifest.bindings) {
      if (
        binding.access === "read-write" &&
        binding.role !== "state-read" &&
        binding.role !== "state-write"
      ) {
        writtenObjects.push(binding.memoryObject);
      }
    }
    for (const state of this.manifest.states || []) {
      writtenObjects.push(state.memoryObjects[outputIndex]);
    }
    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.bindGroups[activeBuffer]);
    pass.dispatchWorkgroups(Math.ceil(
      this.manifest.dispatchElements / this.manifest.workgroupSize,
    ));
    pass.end();
    for (const item of this.readbackPlan) {
      encoder.copyBufferToBuffer(
        this.outputBuffer(outputIndex, item.physical), 0,
        item.buffer, 0, item.bytes,
      );
    }
    if (this.integrity) {
      encoder.copyBufferToBuffer(
        this.fixedBuffers.get(this.integrity.binding), 0,
        this.integrityReadback, 0,
        this.integrity.elements * Uint32Array.BYTES_PER_ELEMENT,
      );
    }
    const submissionObjects = this.manifest.bindings.map((binding) => binding.memoryObject);
    for (const state of this.manifest.states || []) {
      submissionObjects.push(...state.memoryObjects);
    }
    for (const item of this.readbackPlan) {
      submissionObjects.push(
        item.physical.readbackDeviceObject,
        item.physical.readbackHostObject,
      );
      writtenObjects.push(item.physical.readbackDeviceObject);
    }
    if (this.integrity) {
      submissionObjects.push(...this.manifest.integrityReadbackObjects);
      writtenObjects.push(this.manifest.integrityReadbackObjects[0]);
    }
    const commandBuffer = encoder.finish();
    const memoryHold = this.memory.begin(submissionObjects);
    const queuedWrites = [];
    try {
      writeInputs(this, inputs, (object) => queuedWrites.push(object));
      if (this.integrity) {
        this.device.queue.writeBuffer(
          this.fixedBuffers.get(this.integrity.binding),
          0,
          new Uint32Array([0, 0xffffffff]),
        );
        queuedWrites.push(this.integrity.memoryObject);
      }
      this.device.queue.submit([commandBuffer]);
    } catch (error) {
      if (queuedWrites.length) {
        // writeBuffer is queue work in its own right. A later upload or the
        // compute submit may fail, but earlier accepted uploads must keep
        // their storage live through the boundary captured here.
        const uploadCompletion = captureQueueCompletion(this.device);
        trackQueueCleanup(
          this,
          releaseAcceptedQueueWork(this, uploadCompletion, memoryHold, queuedWrites),
        );
      } else {
        this.memory.complete(memoryHold);
      }
      throw error;
    }
    // Capture completion at the compute submission boundary. Callers may
    // submit unrelated presentation work to the same queue immediately after
    // this method returns; resident acknowledgement must not wait for it.
    const queueCompletion = captureQueueCompletion(this.device);
    const mappings = [];
    let mappingSetupFailure = null;
    try {
      for (const item of this.readbackPlan) {
        mappings.push(Promise.resolve(
          item.buffer.mapAsync(GPUMapMode.READ, 0, item.bytes),
        ));
      }
      if (this.integrity) {
        mappings.push(Promise.resolve(
          this.integrityReadback.mapAsync(
            GPUMapMode.READ,
            0,
            this.integrity.elements * Uint32Array.BYTES_PER_ELEMENT,
          ),
        ));
      }
    } catch (error) {
      // Submission has already succeeded. Surface setup failure through the
      // completion channel so the bridge records the accepted submission
      // before applying its terminal-failure policy.
      mappingSetupFailure = error;
    }
    // Do not let one rejected map abort cleanup of the remaining buffers. The
    // aggregate rejects only after every mapping attempt has settled, so
    // finish() can safely unmap every buffer it submitted.
    const mappingSettled = Promise.allSettled(mappings);
    const mappingCompletion = mappingSettled.then((results) => {
      if (mappingSetupFailure) throw mappingSetupFailure;
      const rejected = results.find((result) => result.status === "rejected");
      if (rejected) throw rejected.reason;
      return results.map((result) => result.value);
    });
    const completion = mappings.length || mappingSetupFailure
      ? Promise.all([queueCompletion, mappingCompletion]).then(([, values]) => values)
      : queueCompletion;
    return {
      outputIndex,
      completion,
      queueCompletion,
      mappingSettled,
      memoryHold,
      writtenObjects,
    };
  }

  async finish(submission) {
    const {
      completion,
      queueCompletion = completion,
      mappingSettled = Promise.resolve([]),
      memoryHold,
      writtenObjects = [],
    } = submission || {};
    if (!completion || typeof completion.then !== "function") {
      throw new Error("compute completion requires its exact submission promise");
    }
    if (!queueCompletion || typeof queueCompletion.then !== "function") {
      throw new Error("compute completion requires its exact queue promise");
    }
    try {
      await completion;
      if (!this.readbackPlan.length && !this.integrity) {
        this.publishMetrics();
        return { outputs: [], integrity: null };
      }
      // This is the physical mapped transfer. Count it exactly once per
      // accepted mapping, including integrity metadata and mappings whose
      // integrity check rejects the candidate state.
      this.metrics.gpuToCpuReadbackBytes += this.readbackBytes;
      if (this.integrity) {
        const mapped = this.integrityReadback.getMappedRange(
          0,
          this.integrity.elements * Uint32Array.BYTES_PER_ELEMENT,
        );
        const staged = this.memory.stageHost(
          this.manifest.integrityReadbackObjects[1],
          new Uint8Array(mapped),
        );
        const words = new Uint32Array(
          staged.buffer,
          staged.byteOffset,
          this.integrity.elements,
        );
        if (words[0] !== 0) {
          const packed = words[1];
          const code = packed & 0xff;
          const instance = packed >>> 8;
          const constraint = (this.manifest.constraints || [])
            .find((candidate) => candidate.code === code);
          const result = {
            outputs: [],
            integrity: { constraint: constraint?.name || String(code), instance },
          };
          this.publishMetrics();
          return result;
        }
      }
      const outputs = [];
      for (const item of this.readbackPlan) {
        const mapped = item.buffer.getMappedRange(0, item.bytes);
        const staged = this.memory.stageHost(
          item.physical.readbackHostObject,
          new Uint8Array(mapped),
        );
        const values = Device.logicalOutputValues(
          item.output,
          new Float32Array(
            staged.buffer,
            staged.byteOffset,
            item.bytes / Float32Array.BYTES_PER_ELEMENT,
          ),
        );
        this.metrics.gpuToCpuOutputBytes += item.bytes * item.aliases.length;
        for (const output of item.aliases) outputs.push({ name: output.name, values });
      }
      this.publishMetrics();
      return { outputs, integrity: null };
    } finally {
      const [queueResult] = await Promise.allSettled([queueCompletion]);
      try {
        await mappingSettled;
        const unmapped = new Set();
        for (const item of this.readbackPlan) {
          if (!unmapped.has(item.buffer)) item.buffer.unmap();
          unmapped.add(item.buffer);
        }
        if (this.integrity && !unmapped.has(this.integrityReadback)) {
          this.integrityReadback.unmap();
        }
        if (queueResult.status === "fulfilled") {
          writtenObjects.forEach((object, index) => {
            if (writtenObjects.indexOf(object) === index) this.memory.recordWrite(object);
          });
        }
      } finally {
        this.memory.complete(memoryHold);
      }
    }
  }

  publishMetrics() {
    const root = document.documentElement;
    root.dataset.mechComputeCpuToGpuInputBytes = String(this.metrics.cpuToGpuInputBytes);
    root.dataset.mechComputeGpuToCpuReadbackBytes = String(this.metrics.gpuToCpuReadbackBytes);
    root.dataset.mechComputeGpuToCpuOutputBytes = String(this.metrics.gpuToCpuOutputBytes);
    root.dataset.mechComputeLogicalOutputs = String(this.metrics.logicalOutputs);
    root.dataset.mechComputePhysicalOutputBuffers = String(
      this.metrics.uniquePhysicalOutputBuffers,
    );
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    const pending = [...(this.pendingQueueCleanups || [])];
    if (pending.length) {
      this.disposeCompletion = Promise.all(pending).then(() => destroyDeviceResource(this));
      this.disposeCompletion.catch((error) => {
        this.queueCleanupFailure ||= error instanceof Error ? error : new Error(String(error));
      });
      return;
    }
    destroyDeviceResource(this);
    this.disposeCompletion = Promise.resolve();
  }
}

class Session {
  constructor({
    controller,
    resource,
    generation = "0",
    activeBuffer = 0,
    acceptedDispatches = 0,
    lastDispatchToken = null,
    ownsResource = true,
    isCurrent = () => true,
  }) {
    this.controller = controller;
    this.resource = resource;
    this.generation = String(generation);
    this.physicalRevision = String(resource?.physicalRevision || "");
    this.activeBuffer = activeBuffer;
    this.acceptedDispatches = acceptedDispatches;
    this.lastAcceptedDispatchToken = null;
    this.lastDispatchToken = lastDispatchToken;
    this.ownsResource = ownsResource;
    this.isOwnedGeneration = isCurrent;
    this.pending = false;
    this.retired = false;
    this.failure = null;
    this.lifecycle = new SubmissionLifecycle(this.generation);
    this.resource?.device?.lost.then((info) => {
      if (!this.isCurrent()) return;
      this.resource?.memory?.markLost();
      const reason = info?.message || info?.reason || "unknown reason";
      const failure = new Error(`GPU device lost: ${reason}`);
      failure.mechDeviceLost = true;
      this.failure = this.lifecycle.markFailed(failure);
    });
  }

  isCurrent() {
    return !this.retired && this.isOwnedGeneration();
  }

  canTransferTo(manifest) {
    return !this.retired && !this.pending && !this.failure && this.ownsResource &&
      this.resource?.compatibleWith(manifest);
  }

  adoptFrom(previous, manifest) {
    if (!previous?.canTransferTo(manifest) || previous.resource !== this.resource) {
      throw new Error("the compatible GPU resource is no longer transferable");
    }
    previous.ownsResource = false;
    previous.retired = true;
    this.resource.adoptManifest(manifest);
    this.physicalRevision = String(manifest.physicalRevision || "");
    this.ownsResource = true;
  }

  validateCommand(command) {
    if (!command?.dispatch) return false;
    if (this.failure) throw this.failure;
    if (!this.isCurrent()) {
      throw new Error("a retired browser compute session received a dispatch");
    }
    if (this.pending) {
      throw new Error("a checked browser compute dispatch is already in flight");
    }
    if (
      command.acknowledgementRequired !== true ||
      typeof command.dispatchToken !== "string" ||
      !/^[1-9][0-9]*:[1-9][0-9]*$/.test(command.dispatchToken)
    ) {
      throw new Error("the browser compute command has no valid completion identity");
    }
    return true;
  }

  complete(payload) {
    this.controller.completeComputeCommand({ version: 1, ...payload });
  }

  reject(dispatchToken, error) {
    const failure = error instanceof Error ? error : new Error(String(error));
    try {
      this.complete({
        token: dispatchToken,
        status: "failed",
        failure: { reason: failure.message },
      });
    } catch (completionError) {
      const detail = completionError instanceof Error
        ? completionError.message
        : String(completionError);
      return new Error(
        `${failure.message}; rejecting dispatch ${dispatchToken} also failed: ${detail}`,
      );
    }
    return failure;
  }

  submit(command, hooks = {}) {
    if (!this.validateCommand(command)) return null;
    const dispatchToken = command.dispatchToken;
    let submission;
    try {
      // Readback allocation is part of claiming this dispatch. Any failure
      // must complete the claimed command before control leaves this method.
      this.resource.setRequestedOutputs(command.requestedOutputs || []);
      submission = this.resource.submit(command, this.activeBuffer);
      // queue.submit() has succeeded. From here onward a device loss is a
      // terminal failure, never permission to replay this turn on the CPU.
      this.lifecycle.markSubmitted(dispatchToken);
      this.lastDispatchToken = dispatchToken;
      this.pending = true;
      hooks.onSubmitted?.({ dispatchToken, ...submission });
    } catch (error) {
      if (submission) {
        // The queue operation was accepted even if local lifecycle bookkeeping
        // or an observation hook failed afterward. Drive the exact submission
        // through resource cleanup, and keep retirement from destroying its
        // buffers until that cleanup reaches completion or device loss.
        let cleanup;
        try {
          cleanup = this.resource.finish(submission);
        } catch (cleanupError) {
          cleanup = Promise.reject(cleanupError);
        }
        this.pending = true;
        this.completion = Promise.resolve(cleanup)
          .catch(() => {})
          .finally(() => { this.pending = false; });
      } else {
        this.pending = false;
      }
      this.failure = this.lifecycle.markFailed(this.reject(dispatchToken, error));
      try {
        hooks.onFailure?.(this.failure);
      } catch (hookError) {
        console.error("browser compute failure hook failed", hookError);
      }
      throw this.failure;
    }
    this.completion = this.finish(dispatchToken, submission, hooks);
    return submission;
  }

  async finish(dispatchToken, submission, hooks) {
    let completionSent = false;
    try {
      const { outputs, integrity } = await this.resource.finish(submission);
      if (!this.isCurrent()) return;
      if (integrity) {
        this.complete({
          token: dispatchToken,
          status: "integrity-rejected",
          integrity: {
            constraint: integrity.constraint,
            instance: integrity.instance,
          },
        });
      } else {
        this.complete({ token: dispatchToken, status: "completed", outputs });
      }
      completionSent = true;
      this.lifecycle.markAccepted(dispatchToken);
      this.acceptedDispatches += 1;
      this.lastAcceptedDispatchToken = dispatchToken;
      if (!integrity) this.activeBuffer = submission.outputIndex;
      hooks.onAccepted?.({
        dispatchToken,
        outputIndex: submission.outputIndex,
        outputs,
        integrity,
      });
    } catch (error) {
      if (!this.isCurrent()) return;
      const failure = completionSent ? error : this.reject(dispatchToken, error);
      this.failure = this.lifecycle.markFailed(failure);
      try {
        hooks.onFailure?.(this.failure);
      } catch (hookError) {
        console.error("browser compute failure hook failed", hookError);
      }
    } finally {
      this.pending = false;
    }
  }

  retire() {
    this.retired = true;
    if (this.ownsResource && this.resource) {
      const resource = this.resource;
      if (this.pending && this.completion) {
        Promise.resolve(this.completion).finally(() => resource.dispose());
      } else {
        resource.dispose();
      }
    }
    this.ownsResource = false;
  }
}

async function awaitSmokeTargetCompletion(target) {
  const completion = target.computeSession?.completion;
  if (completion) {
    await Promise.resolve(completion);
  }
  if (target.bridgeFailure) {
    throw target.bridgeFailure;
  }
  target.stop();
  const disposal = target.computeResource?.disposeCompletion;
  if (disposal) {
    await Promise.resolve(disposal);
  }
  if (target.bridgeFailure) {
    throw target.bridgeFailure;
  }
}

return Object.freeze({ Device, ResetTracker, Session, awaitSmokeTargetCompletion });
})();
