/* Shared WebGPU resource and completion protocol for Mech browser hosts. */
globalThis.MechBrowserComputeDevice ||= class MechBrowserComputeDevice {
  static requiredLimits(manifest, supported, requestedOutputNames = []) {
    const requested = new Set(requestedOutputNames);
    const bindingBytes = manifest.bindings.map((binding) =>
      Math.max(4, Number(binding.elements) * Float32Array.BYTES_PER_ELEMENT));
    const readbackBytes = manifest.outputs
      .filter((output) => requested.has(output.name))
      .reduce(
        (total, output) => total + Number(output.elementsPerInstance) * Float32Array.BYTES_PER_ELEMENT,
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
    const requiredLimits = this.requiredLimits(
      manifest,
      adapter.limits,
      requestedOutputNames,
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
    this.disposed = false;
    this.metrics = {
      cpuToGpuInputBytes: 0,
      gpuToCpuReadbackBytes: 0,
      logicalOutputs: requestedOutputNames.length,
      uniquePhysicalOutputBuffers: 0,
    };
    this.createBuffers(requestedOutputNames);
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
    const physical = new Map();
    this.readbackPlan = [];
    let byteLength = 0;
    for (const output of this.manifest.outputs) {
      if (!requested.has(output.name)) continue;
      const key = String(output.slot);
      let item = physical.get(key);
      if (!item) {
        const bytes = output.elementsPerInstance * Float32Array.BYTES_PER_ELEMENT;
        item = { output, offset: byteLength, bytes, aliases: [] };
        physical.set(key, item);
        this.readbackPlan.push(item);
        byteLength += bytes;
      }
      item.aliases.push(output);
    }
    this.metrics.uniquePhysicalOutputBuffers = physical.size;
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

  outputBuffer(index, output) {
    if (this.stateBuffers.has(output.slot)) return this.stateBuffers.get(output.slot)[index];
    const binding = this.manifest.bindings.find(
      (candidate) => candidate.role === "output" && candidate.slot === output.slot,
    );
    if (!binding) throw new Error(`compute output ${output.name} has no physical buffer`);
    return this.fixedBuffers.get(binding.binding);
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
        this.outputBuffer(outputIndex, item.output), 0,
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
    return { outputIndex };
  }

  async finish() {
    if (!this.readback) {
      await this.device.queue.onSubmittedWorkDone();
      this.publishMetrics();
      return { outputs: [], integrity: null };
    }
    await this.readback.mapAsync(GPUMapMode.READ, 0, this.readbackBytes);
    try {
      const mapped = this.readback.getMappedRange(0, this.readbackBytes);
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
        const values = Float32Array.from(
          new Float32Array(mapped, item.offset, item.bytes / Float32Array.BYTES_PER_ELEMENT),
        );
        this.metrics.gpuToCpuReadbackBytes += item.bytes;
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
};
