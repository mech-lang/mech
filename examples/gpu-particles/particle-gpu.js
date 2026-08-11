import init, { compileGpuProgram } from "/_mech/pkg/mech_wasm.js";

const MAX_PARTICLES = 2_000_000;
const PARTICLE_COUNTS = [100_000, 500_000, 1_000_000, 2_000_000];

const renderShader = /* wgsl */ `
struct RenderParams {
  point_size: f32,
  pixel_x: f32,
  pixel_y: f32,
  padding: f32,
}

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) local: vec2f,
  @location(1) color: vec3f,
}

@group(0) @binding(0) var<storage, read> positions: array<f32>;
@group(0) @binding(1) var<storage, read> velocities: array<f32>;
@group(0) @binding(2) var<uniform> params: RenderParams;

const corners = array<vec2f, 6>(
  vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
  vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0)
);

@vertex
fn vertex_main(
  @builtin(vertex_index) vertex_index: u32,
  @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
  let offset = instance_index * 2u;
  let particle_position = vec2f(positions[offset], positions[offset + 1u]);
  let particle_velocity = vec2f(velocities[offset], velocities[offset + 1u]);
  let local = corners[vertex_index];
  let size = vec2f(params.pixel_x, params.pixel_y) * params.point_size;
  let speed = clamp(length(particle_velocity) * 2.1, 0.0, 1.0);
  let radius = clamp(length(particle_position), 0.0, 1.0);
  let cold = vec3f(0.23, 0.83, 0.77);
  let warm = vec3f(0.98, 0.49, 0.31);
  let edge = vec3f(0.54, 0.64, 1.0);

  var output: VertexOutput;
  output.position = vec4f(particle_position + local * size, 0.0, 1.0);
  output.local = local;
  output.color = mix(mix(cold, warm, speed), edge, radius * 0.42);
  return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
  let radius = length(input.local);
  if (radius > 1.0) {
    discard;
  }
  let alpha = (1.0 - smoothstep(0.48, 1.0, radius)) * 0.72;
  return vec4f(input.color * alpha, alpha);
}
`;

class ParticleSimulation {
  constructor(manifest, compileMilliseconds) {
    this.manifest = manifest;
    this.compileMilliseconds = compileMilliseconds;
    this.canvas = document.querySelector("#particle-canvas");
    this.status = document.querySelector("#gpu-status");
    this.message = document.querySelector("#canvas-message");
    this.count = MAX_PARTICLES;
    this.activeBuffer = 0;
    this.generation = 0;
    this.paused = false;
    this.benchmarking = false;
    this.running = false;
    this.lastFrame = 0;
    this.statsStart = 0;
    this.statsFrames = 0;
    this.bindControls();
  }

  async initialize() {
    if (!(this.canvas instanceof HTMLCanvasElement)) {
      throw new Error("Particle canvas is missing");
    }
    if (!navigator.gpu) {
      throw new Error("WebGPU is unavailable in this browser");
    }

    this.setStatus("Requesting GPU", "");
    this.adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
    if (!this.adapter) {
      throw new Error("No compatible WebGPU adapter was found");
    }
    const storageBindings = this.manifest.bindings.length;
    if (storageBindings > this.adapter.limits.maxStorageBuffersPerShaderStage) {
      throw new Error(
        `Mech generated ${storageBindings} storage bindings, but this adapter supports `
          + `${this.adapter.limits.maxStorageBuffersPerShaderStage}`,
      );
    }
    this.device = await this.adapter.requestDevice({
      requiredLimits: { maxStorageBuffersPerShaderStage: storageBindings },
    });
    this.device.lost.then((info) => this.fail(new Error(`GPU device lost: ${info.message}`)));
    this.context = this.canvas.getContext("webgpu");
    if (!this.context) {
      throw new Error("The canvas could not create a WebGPU context");
    }
    this.format = navigator.gpu.getPreferredCanvasFormat();

    await this.createPipelines();
    this.createBuffers();
    this.resize();
    this.seedParticles(this.count);
    await this.device.queue.onSubmittedWorkDone();
    this.running = true;
    this.message.textContent = "";
    this.setStatus(`Mech compiled in ${(this.compileMilliseconds / 1000).toFixed(1)} s`, "ready");
    this.updateTelemetry(0);
    this.lastFrame = performance.now();
    this.statsStart = this.lastFrame;
    requestAnimationFrame((time) => this.frame(time));

    const info = this.adapter.info;
    const adapterName = info?.description || info?.device || info?.vendor || "WebGPU adapter";
    document.querySelector("#adapter-name").textContent = adapterName;
  }

  async createPipelines() {
    const computeModule = this.device.createShaderModule({ code: this.manifest.wgsl });
    const computeInfo = await computeModule.getCompilationInfo();
    const computeErrors = computeInfo.messages.filter((message) => message.type === "error");
    if (computeErrors.length > 0) {
      throw new Error(computeErrors.map((message) => message.message).join("\n"));
    }
    const renderModule = this.device.createShaderModule({ code: renderShader });
    this.computePipeline = this.device.createComputePipeline({
      layout: "auto",
      compute: { module: computeModule, entryPoint: "main" },
    });
    this.renderPipeline = this.device.createRenderPipeline({
      layout: "auto",
      vertex: { module: renderModule, entryPoint: "vertex_main" },
      fragment: {
        module: renderModule,
        entryPoint: "fragment_main",
        targets: [{
          format: this.format,
          blend: {
            color: { srcFactor: "one", dstFactor: "one", operation: "add" },
            alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha", operation: "add" },
          },
        }],
      },
      primitive: { topology: "triangle-list" },
    });
  }

  createBuffers() {
    this.stateBuffers = new Map();
    this.fixedBuffers = new Map();
    const stateBindings = this.manifest.bindings.filter(
      (binding) => binding.role === "state-read" || binding.role === "state-write",
    );
    for (const binding of stateBindings) {
      if (this.stateBuffers.has(binding.slot)) {
        continue;
      }
      const size = binding.elements * Float32Array.BYTES_PER_ELEMENT;
      this.stateBuffers.set(binding.slot, [0, 1].map(() => this.device.createBuffer({
        size,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      })));
    }

    for (const binding of this.manifest.bindings) {
      if (binding.role === "state-read" || binding.role === "state-write") {
        continue;
      }
      const buffer = this.device.createBuffer({
        size: Math.max(4, binding.elements * Float32Array.BYTES_PER_ELEMENT),
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      });
      this.fixedBuffers.set(binding.binding, buffer);
      if (binding.role === "input") {
        const values = new Float32Array(binding.elements);
        values.fill(this.inputValue(binding.name));
        this.device.queue.writeBuffer(buffer, 0, values);
      }
    }

    this.computeBindGroups = [0, 1].map((sourceIndex) => this.device.createBindGroup({
      layout: this.computePipeline.getBindGroupLayout(0),
      entries: this.manifest.bindings.map((binding) => ({
        binding: binding.binding,
        resource: { buffer: this.bufferForBinding(binding, sourceIndex) },
      })),
    }));

    this.positionSlot = this.outputSlot("result.0");
    this.velocitySlot = this.outputSlot("result.1");
    this.renderUniformBuffer = this.device.createBuffer({
      size: 16,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    this.renderBindGroups = [0, 1].map((index) => this.device.createBindGroup({
      layout: this.renderPipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: this.stateBuffers.get(this.positionSlot)[index] } },
        { binding: 1, resource: { buffer: this.stateBuffers.get(this.velocitySlot)[index] } },
        { binding: 2, resource: { buffer: this.renderUniformBuffer } },
      ],
    }));
  }

  inputValue(name) {
    const canonical = name.replace(/^host-/, "");
    const inputs = {
      origin: 0,
      attraction: 0.34,
      drag: 0.997,
      dt: 1 / 120,
    };
    if (!(canonical in inputs)) {
      throw new Error(`No browser value is defined for Mech input ${name}`);
    }
    return inputs[canonical];
  }

  bufferForBinding(binding, sourceIndex) {
    if (binding.role === "state-read") {
      return this.stateBuffers.get(binding.slot)[sourceIndex];
    }
    if (binding.role === "state-write") {
      return this.stateBuffers.get(binding.slot)[1 - sourceIndex];
    }
    return this.fixedBuffers.get(binding.binding);
  }

  outputSlot(name) {
    const output = this.manifest.outputs.find((candidate) => candidate.name === name);
    if (!output || !this.stateBuffers.has(output.slot)) {
      throw new Error(`Mech GPU output ${name} is not resident state`);
    }
    return output.slot;
  }

  seedParticles(count) {
    if (!this.device) {
      return;
    }
    const positions = new Float32Array(count * 2);
    const velocities = new Float32Array(count * 2);
    const goldenAngle = Math.PI * (3 - Math.sqrt(5));
    for (let index = 0; index < count; index += 1) {
      const angle = index * goldenAngle;
      const radius = Math.sqrt((index + 0.5) / count) * 0.95 + 0.015;
      const speed = 0.24 + (1 - radius) * 0.18;
      const offset = index * 2;
      positions[offset] = Math.cos(angle) * radius;
      positions[offset + 1] = Math.sin(angle) * radius;
      velocities[offset] = -Math.sin(angle) * speed;
      velocities[offset + 1] = Math.cos(angle) * speed;
    }
    for (const buffer of this.stateBuffers.get(this.positionSlot)) {
      this.device.queue.writeBuffer(buffer, 0, positions);
    }
    for (const buffer of this.stateBuffers.get(this.velocitySlot)) {
      this.device.queue.writeBuffer(buffer, 0, velocities);
    }
    this.activeBuffer = 0;
    this.generation = 0;
    this.updateTelemetry(0);
  }

  resize() {
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    const width = Math.max(1, Math.floor(this.canvas.clientWidth * ratio));
    const height = Math.max(1, Math.floor(this.canvas.clientHeight * ratio));
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
      this.context.configure({
        device: this.device,
        format: this.format,
        alphaMode: "premultiplied",
      });
    }
    const pointSize = this.count >= 1_000_000 ? 1.1 : this.count >= 500_000 ? 1.35 : 1.8;
    this.device.queue.writeBuffer(
      this.renderUniformBuffer,
      0,
      new Float32Array([pointSize, 2 / this.canvas.width, 2 / this.canvas.height, 0]),
    );
  }

  encodeTurn(encoder, count, sourceIndex) {
    const compute = encoder.beginComputePass();
    compute.setPipeline(this.computePipeline);
    compute.setBindGroup(0, this.computeBindGroups[sourceIndex]);
    compute.dispatchWorkgroups(Math.ceil((count * 2) / this.manifest.workgroupSize));
    compute.end();
    return 1 - sourceIndex;
  }

  encodeRender(encoder, count, bufferIndex) {
    const render = encoder.beginRenderPass({
      colorAttachments: [{
        view: this.context.getCurrentTexture().createView(),
        clearValue: { r: 0.018, g: 0.026, b: 0.039, a: 1 },
        loadOp: "clear",
        storeOp: "store",
      }],
    });
    render.setPipeline(this.renderPipeline);
    render.setBindGroup(0, this.renderBindGroups[bufferIndex]);
    render.draw(6, count);
    render.end();
  }

  frame(time) {
    if (!this.running) {
      return;
    }
    if (this.benchmarking) {
      requestAnimationFrame((next) => this.frame(next));
      return;
    }
    this.resize();
    const encoder = this.device.createCommandEncoder();
    let renderedBuffer = this.activeBuffer;
    if (!this.paused) {
      renderedBuffer = this.encodeTurn(encoder, this.count, this.activeBuffer);
    }
    this.encodeRender(encoder, this.count, renderedBuffer);
    this.device.queue.submit([encoder.finish()]);
    if (!this.paused) {
      this.activeBuffer = renderedBuffer;
      this.generation += 1;
    }

    this.statsFrames += 1;
    if (time - this.statsStart >= 500) {
      const fps = (this.statsFrames * 1000) / (time - this.statsStart);
      this.updateTelemetry(fps);
      this.statsFrames = 0;
      this.statsStart = time;
    }
    this.lastFrame = time;
    requestAnimationFrame((next) => this.frame(next));
  }

  async submitTurns(count, turns) {
    const encoder = this.device.createCommandEncoder();
    let active = this.activeBuffer;
    for (let turn = 0; turn < turns; turn += 1) {
      active = this.encodeTurn(encoder, count, active);
    }
    this.device.queue.submit([encoder.finish()]);
    this.activeBuffer = active;
    this.generation += turns;
    await this.device.queue.onSubmittedWorkDone();
  }

  async benchmarkCompute(count) {
    const turns = count <= 100_000 ? 256 : count <= 500_000 ? 128 : count <= 1_000_000 ? 64 : 32;
    this.seedParticles(count);
    await this.submitTurns(count, 8);
    const samples = [];
    for (let sample = 0; sample < 7; sample += 1) {
      const started = performance.now();
      await this.submitTurns(count, turns);
      samples.push((performance.now() - started) / turns);
    }
    return median(samples);
  }

  async benchmarkRendered(count) {
    this.seedParticles(count);
    this.resize();
    const samples = [];
    const frames = count <= 100_000 ? 20 : count <= 500_000 ? 12 : count <= 1_000_000 ? 8 : 5;
    for (let frame = 0; frame < frames + 2; frame += 1) {
      await nextFrame();
      const started = performance.now();
      const encoder = this.device.createCommandEncoder();
      const destination = this.encodeTurn(encoder, count, this.activeBuffer);
      this.encodeRender(encoder, count, destination);
      this.device.queue.submit([encoder.finish()]);
      this.activeBuffer = destination;
      await this.device.queue.onSubmittedWorkDone();
      if (frame >= 2) {
        samples.push(performance.now() - started);
      }
    }
    return median(samples);
  }

  async runBenchmarks(mode = "all") {
    if (this.benchmarking) {
      return;
    }
    this.benchmarking = true;
    const button = document.querySelector("#benchmark-button");
    button.disabled = true;
    const panel = document.querySelector("#benchmark-panel");
    const results = document.querySelector("#benchmark-results");
    panel.hidden = false;
    results.replaceChildren();
    for (const count of PARTICLE_COUNTS) {
      const row = document.createElement("tr");
      row.dataset.count = String(count);
      row.innerHTML = `<td>${shortCount(count)}</td><td class="pending">Waiting</td>`
        + `<td class="pending">Waiting</td><td class="pending">Waiting</td>`;
      results.append(row);
    }
    panel.scrollIntoView({ behavior: "smooth", block: "nearest" });

    try {
      for (const count of PARTICLE_COUNTS) {
        const row = results.querySelector(`[data-count="${count}"]`);
        const cells = row.querySelectorAll("td");
        cells[1].textContent = "Running";
        this.setStatus(`Benchmarking ${shortCount(count)}`, "");
        const computeMs = await this.benchmarkCompute(count);
        cells[1].classList.remove("pending");
        cells[1].textContent = `${computeMs.toFixed(3)} ms/turn`;
        cells[2].classList.remove("pending");
        cells[2].textContent = `${(count / computeMs / 1000).toFixed(1)} M particle-turns/s`;
        if (mode === "compute") {
          cells[3].classList.remove("pending");
          cells[3].textContent = "Not measured";
        } else {
          cells[3].textContent = "Running";
          const renderedMs = await this.benchmarkRendered(count);
          cells[3].classList.remove("pending");
          cells[3].textContent = `${renderedMs.toFixed(2)} ms / ${(1000 / renderedMs).toFixed(1)} Hz`;
        }
      }
      this.setStatus("Benchmark complete", "ready");
    } finally {
      this.count = MAX_PARTICLES;
      this.updateSelectedCount();
      this.seedParticles(this.count);
      this.benchmarking = false;
      button.disabled = false;
      this.statsStart = performance.now();
      this.statsFrames = 0;
    }
  }

  updateTelemetry(fps) {
    this.setText("#particle-count", this.count.toLocaleString());
    this.setText("#frame-rate", `${fps.toFixed(1)} Hz`);
    this.setText("#throughput", `${(this.count * fps / 1_000_000).toFixed(1)} M updates/s`);
    const stateBytes = [...this.stateBuffers.values()].length * 2 * MAX_PARTICLES * 2 * 4;
    this.setText("#resident-state", `${(stateBytes / 1_048_576).toFixed(1)} MB resident`);
  }

  setText(selector, value) {
    const element = document.querySelector(selector);
    if (element) {
      element.textContent = value;
    }
  }

  setStatus(message, kind) {
    this.status.className = `status ${kind}`.trim();
    this.status.innerHTML = "<span aria-hidden=\"true\"></span>";
    this.status.append(document.createTextNode(message));
  }

  fail(error) {
    console.error("Mech GPU particle app failed", error);
    this.running = false;
    this.setStatus("GPU unavailable", "error");
    this.message.textContent = error instanceof Error ? error.message : String(error);
  }

  updateSelectedCount() {
    document.querySelectorAll("[data-particle-count]").forEach((button) => {
      button.setAttribute("aria-pressed", String(Number(button.dataset.particleCount) === this.count));
    });
  }

  bindControls() {
    document.querySelectorAll("[data-particle-count]").forEach((button) => {
      button.addEventListener("click", () => {
        if (!this.device || this.benchmarking) {
          return;
        }
        this.count = Number(button.dataset.particleCount);
        this.updateSelectedCount();
        this.seedParticles(this.count);
      });
    });
    document.querySelector("#pause-button").addEventListener("click", (event) => {
      this.paused = !this.paused;
      event.currentTarget.textContent = this.paused ? "Resume" : "Pause";
    });
    document.querySelector("#reset-button").addEventListener("click", () => {
      if (!this.benchmarking) {
        this.seedParticles(this.count);
      }
    });
    document.querySelector("#benchmark-button").addEventListener("click", () => {
      this.runBenchmarks().catch((error) => this.fail(error));
    });
  }
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

function shortCount(count) {
  return count >= 1_000_000 ? `${count / 1_000_000}M` : `${count / 1000}K`;
}

function nextFrame() {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

async function boot() {
  const status = document.querySelector("#gpu-status");
  const message = document.querySelector("#canvas-message");
  try {
    await init();
    status.lastChild.textContent = "Loading Mech source";
    const response = await fetch("/source/particle-kernel.mec");
    if (!response.ok) {
      throw new Error(`Could not load particle-kernel.mec (${response.status})`);
    }
    const source = await response.text();
    status.lastChild.textContent = "Compiling Mech program";
    message.textContent = "Compiling particle-kernel.mec for 2,000,000 particles";
    await nextFrame();
    const compileStarted = performance.now();
    const manifest = compileGpuProgram(source, MAX_PARTICLES);
    const simulation = new ParticleSimulation(manifest, performance.now() - compileStarted);
    await simulation.initialize();
    globalThis.mechParticleSimulation = simulation;
    const benchmark = new URLSearchParams(location.search).get("benchmark");
    if (benchmark === "all" || benchmark === "compute") {
      await nextFrame();
      simulation.runBenchmarks(benchmark).catch((error) => simulation.fail(error));
    }
  } catch (error) {
    console.error("Mech GPU particle app failed", error);
    status.className = "status error";
    status.lastChild.textContent = "GPU unavailable";
    message.textContent = error instanceof Error ? error.message : String(error);
  }
}

boot();
