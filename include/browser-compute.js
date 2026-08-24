/* Shared WebGPU resource and completion protocol for Mech browser hosts. */
globalThis.MechBrowserCompute ||= (() => {
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
    // enough address space for the largest legal readback plan up front so a
    // later sample request cannot exceed the limits admitted for this device.
    const readbackBytes = (manifest.physicalOutputs || [])
      .reduce(
        (total, output) => total + Number(output.sampleElements) * Float32Array.BYTES_PER_ELEMENT,
        0,
      );
    const integrityBytes = Number(
      manifest.bindings.find((binding) => binding.role === "integrity-fault")?.elements || 0,
    ) * Uint32Array.BYTES_PER_ELEMENT;
    const required = {
      maxStorageBuffersPerShaderStage: manifest.bindings.length,
      maxComputeWorkgroupsPerDimension: Math.ceil(
        manifest.dispatchElements / manifest.workgroupSize,
      ),
      maxStorageBufferBindingSize: Math.max(...bindingBytes, 4),
      maxBufferSize: Math.max(...bindingBytes, readbackBytes + integrityBytes, 4),
    };
    for (const [name, value] of Object.entries(required)) {
      if (!Number.isSafeInteger(value) || value <= 0) {
        throw new Error(`Mech computed an invalid ${name} requirement: ${value}`);
      }
      const limit = Number(supported[name]);
      if (!Number.isFinite(limit) || limit <= 0) {
        throw new Error(`this WebGPU adapter does not report the required ${name} limit`);
      }
      if (value > limit) {
        throw new Error(`Mech requires ${value} for ${name}, but this adapter supports ${limit}`);
      }
    }
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
    globalThis.__MECH_COMPUTE_RESOURCE_SEQUENCE__ =
      Number(globalThis.__MECH_COMPUTE_RESOURCE_SEQUENCE__ || 0) + 1;
    globalThis.__MECH_COMPUTE_PIPELINE_BUILD_COUNT__ =
      Number(globalThis.__MECH_COMPUTE_PIPELINE_BUILD_COUNT__ || 0) + 1;
    this.resourceIdentity = String(globalThis.__MECH_COMPUTE_RESOURCE_SEQUENCE__);
    this.deviceIdentity = `device-${this.resourceIdentity}`;
    this.pipelineIdentity =
      `pipeline-${globalThis.__MECH_COMPUTE_PIPELINE_BUILD_COUNT__}`;
    this.stateIdentity = `state-${this.resourceIdentity}`;
    this.pipelineBuildCount = globalThis.__MECH_COMPUTE_PIPELINE_BUILD_COUNT__;
    this.disposed = false;
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
    this.configureReadback(requestedOutputNames);
  }

  configureReadback(requestedOutputNames) {
    this.metrics.logicalOutputs = requestedOutputNames.length;
    this.readback?.destroy();
    const requested = new Set(requestedOutputNames);
    this.readbackPlan = [];
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
      this.readbackPlan.push({
        output: aliases[0], physical, offset: byteLength, bytes, aliases,
      });
      byteLength += bytes;
    }
    this.metrics.uniquePhysicalOutputBuffers = this.readbackPlan.length;
    this.integrityOffset = this.integrity ? byteLength : null;
    if (this.integrity) byteLength += this.integrity.elements * Uint32Array.BYTES_PER_ELEMENT;
    this.readbackBytes = byteLength;
    this.readback = byteLength > 0 ? this.device.createBuffer({
      size: Math.max(4, byteLength),
      usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
    }) : null;
    this.readbackSignature = [...requestedOutputNames].sort().join("\u0000");
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
    for (const input of command.inputs) {
      const target = this.inputBindings.get(input.name);
      if (!target) throw new Error(`Mech wrote undeclared GPU input ${input.name}`);
      if (input.values.length !== target.binding.elements) {
        throw new Error(
          `Mech wrote ${input.values.length} values to ${input.name}; expected ${target.binding.elements}`,
        );
      }
      this.device.queue.writeBuffer(target.buffer, 0, input.values);
      this.metrics.cpuToGpuInputBytes += input.values.byteLength;
    }
  }

  submit(command, activeBuffer) {
    if (this.disposed) throw new Error("the WebGPU compute device is disposed");
    this.applyInputs(command);
    if (this.integrity) {
      this.device.queue.writeBuffer(
        this.fixedBuffers.get(this.integrity.binding),
        0,
        new Uint32Array([0, 0xffffffff]),
      );
    }
    const outputIndex = 1 - activeBuffer;
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
        this.readback, item.offset, item.bytes,
      );
    }
    if (this.integrity) {
      encoder.copyBufferToBuffer(
        this.fixedBuffers.get(this.integrity.binding), 0,
        this.readback, this.integrityOffset,
        this.integrity.elements * Uint32Array.BYTES_PER_ELEMENT,
      );
    }
    this.device.queue.submit([encoder.finish()]);
    // Capture completion at the compute submission boundary. Callers may
    // submit unrelated presentation work to the same queue immediately after
    // this method returns; resident acknowledgement must not wait for it.
    let completion;
    try {
      completion = this.readback
        ? this.readback.mapAsync(GPUMapMode.READ, 0, this.readbackBytes)
        : this.device.queue.onSubmittedWorkDone();
    } catch (error) {
      // Submission has already succeeded. Surface setup failure through the
      // completion channel so the bridge records the accepted submission
      // before applying its terminal-failure policy.
      completion = Promise.reject(error);
    }
    return { outputIndex, completion };
  }

  async finish(completion) {
    if (!completion || typeof completion.then !== "function") {
      throw new Error("compute completion requires its exact submission promise");
    }
    await completion;
    if (!this.readback) {
      this.publishMetrics();
      return { outputs: [], integrity: null };
    }
    try {
      const mapped = this.readback.getMappedRange(0, this.readbackBytes);
      // This is the physical mapped transfer. Count it exactly once per
      // accepted mapping, including integrity metadata and mappings whose
      // integrity check rejects the candidate state.
      this.metrics.gpuToCpuReadbackBytes += this.readbackBytes;
      if (this.integrity) {
        const words = new Uint32Array(mapped, this.integrityOffset, this.integrity.elements);
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
        const values = Device.logicalOutputValues(
          item.output,
          new Float32Array(mapped, item.offset, item.bytes / Float32Array.BYTES_PER_ELEMENT),
        );
        this.metrics.gpuToCpuOutputBytes += item.bytes * item.aliases.length;
        for (const output of item.aliases) outputs.push({ name: output.name, values });
      }
      this.publishMetrics();
      return { outputs, integrity: null };
    } finally {
      this.readback.unmap();
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
    for (const buffers of this.stateBuffers.values()) buffers.forEach((buffer) => buffer.destroy());
    for (const buffer of this.fixedBuffers.values()) buffer.destroy();
    this.readback?.destroy();
    this.device.destroy();
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
    this.resource.setRequestedOutputs(command.requestedOutputs || []);
    let submission;
    try {
      submission = this.resource.submit(command, this.activeBuffer);
      // queue.submit() has succeeded. From here onward a device loss is a
      // terminal failure, never permission to replay this turn on the CPU.
      this.lifecycle.markSubmitted(dispatchToken);
      this.lastDispatchToken = dispatchToken;
      this.pending = true;
      hooks.onSubmitted?.({ dispatchToken, ...submission });
    } catch (error) {
      this.failure = this.lifecycle.markFailed(this.reject(dispatchToken, error));
      hooks.onFailure?.(this.failure);
      throw this.failure;
    }
    this.completion = this.finish(dispatchToken, submission, hooks);
    return submission;
  }

  async finish(dispatchToken, submission, hooks) {
    let completionSent = false;
    try {
      const { outputs, integrity } = await this.resource.finish(submission.completion);
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
      hooks.onFailure?.(this.failure);
    } finally {
      this.pending = false;
    }
  }

  retire() {
    this.retired = true;
    if (this.ownsResource) this.resource?.dispose();
    this.ownsResource = false;
  }
}

return Object.freeze({ Device, ResetTracker, Session });
})();
