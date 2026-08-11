(() => {
  "use strict";

  const simulations = new Map();

  const computeShader = /* wgsl */ `
struct Particle {
  position: vec2f,
  velocity: vec2f,
}

struct Params {
  delta_time: f32,
  time: f32,
  particle_count: u32,
  gravity: f32,
  drag: f32,
  point_size: f32,
  pixel_x: f32,
  pixel_y: f32,
}

@group(0) @binding(0) var<storage, read> source: array<Particle>;
@group(0) @binding(1) var<storage, read_write> destination: array<Particle>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3u) {
  let index = id.x;
  if (index >= params.particle_count) {
    return;
  }

  var particle = source[index];
  let center = -particle.position;
  let radius = max(length(center), 0.025);
  let radial = center / radius;
  let tangent = vec2f(-radial.y, radial.x);
  let pulse = sin(params.time * 0.37 + f32(index % 4096u) * 0.0017);
  let acceleration = radial * (params.gravity / (0.16 + radius * radius))
    + tangent * (0.09 + pulse * 0.018);
  let damping = pow(params.drag, params.delta_time * 60.0);

  particle.velocity = (particle.velocity + acceleration * params.delta_time) * damping;
  particle.position = particle.position + particle.velocity * params.delta_time;

  if (abs(particle.position.x) > 1.08) {
    particle.position.x = clamp(particle.position.x, -1.08, 1.08);
    particle.velocity.x = -particle.velocity.x * 0.72;
  }
  if (abs(particle.position.y) > 1.08) {
    particle.position.y = clamp(particle.position.y, -1.08, 1.08);
    particle.velocity.y = -particle.velocity.y * 0.72;
  }

  destination[index] = particle;
}
`;

  const renderShader = /* wgsl */ `
struct Particle {
  position: vec2f,
  velocity: vec2f,
}

struct Params {
  delta_time: f32,
  time: f32,
  particle_count: u32,
  gravity: f32,
  drag: f32,
  point_size: f32,
  pixel_x: f32,
  pixel_y: f32,
}

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) local: vec2f,
  @location(1) color: vec3f,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: Params;

const corners = array<vec2f, 6>(
  vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
  vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0)
);

@vertex
fn vertex_main(
  @builtin(vertex_index) vertex_index: u32,
  @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
  let particle = particles[instance_index];
  let local = corners[vertex_index];
  let size = vec2f(params.pixel_x, params.pixel_y) * params.point_size;
  let speed = clamp(length(particle.velocity) * 1.6, 0.0, 1.0);
  let radius = clamp(length(particle.position), 0.0, 1.0);
  let cold = vec3f(0.25, 0.82, 0.78);
  let warm = vec3f(0.98, 0.47, 0.35);
  let edge = vec3f(0.52, 0.62, 1.0);

  var output: VertexOutput;
  output.position = vec4f(particle.position + local * size, 0.0, 1.0);
  output.local = local;
  output.color = mix(mix(cold, warm, speed), edge, radius * 0.38);
  return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
  let radius = length(input.local);
  if (radius > 1.0) {
    discard;
  }
  let alpha = (1.0 - smoothstep(0.52, 1.0, radius)) * 0.78;
  return vec4f(input.color * alpha, alpha);
}
`;

  class ParticleSimulation {
    constructor(instance, selector, maxParticles) {
      this.instance = instance;
      this.selector = selector;
      this.maxParticles = maxParticles;
      this.canvas = document.querySelector(selector);
      if (!(this.canvas instanceof HTMLCanvasElement)) {
        throw new Error(`GPU particle selector ${selector} is not a canvas`);
      }
      this.status = document.querySelector("#gpu-status");
      this.message = document.querySelector("#canvas-message");
      this.control = null;
      this.device = null;
      this.activeBuffer = 0;
      this.initializing = null;
      this.running = false;
      this.paused = false;
      this.lastFrame = 0;
      this.statsStart = 0;
      this.statsFrames = 0;
      this.frameHandle = 0;
      this.generation = 0;
      this.bindControls();
    }

    configure(control) {
      this.control = this.validatedControl(control);
      this.updateSelectedCount();
      if (!this.initializing) {
        this.initializing = this.initialize().catch((error) => this.fail(error));
      } else if (this.device) {
        this.seedParticles();
      }
    }

    validatedControl(control) {
      const particleCount = Math.min(
        this.maxParticles,
        Math.max(1, Math.trunc(Number(control.particleCount))),
      );
      return {
        particleCount,
        gravity: Number(control.gravity),
        drag: Number(control.drag),
        pointSize: Number(control.pointSize),
        timeScale: Number(control.timeScale),
      };
    }

    async initialize() {
      this.setStatus("Requesting GPU", "");
      if (!navigator.gpu) {
        throw new Error("WebGPU is unavailable in this browser");
      }
      const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
      if (!adapter) {
        throw new Error("No compatible WebGPU adapter was found");
      }
      this.device = await adapter.requestDevice();
      this.context = this.canvas.getContext("webgpu");
      if (!this.context) {
        throw new Error("The canvas could not create a WebGPU context");
      }
      this.format = navigator.gpu.getPreferredCanvasFormat();
      this.createPipelines();
      this.createBuffers();
      this.resize();
      this.seedParticles();
      this.running = true;
      this.setStatus("GPU resident", "ready");
      this.message.textContent = "";
      this.lastFrame = performance.now();
      this.statsStart = this.lastFrame;
      this.frameHandle = requestAnimationFrame((time) => this.frame(time));
    }

    createPipelines() {
      const computeModule = this.device.createShaderModule({ code: computeShader });
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
      const stateBytes = this.maxParticles * 16;
      this.particleBuffers = [0, 1].map(() => this.device.createBuffer({
        size: stateBytes,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      }));
      this.uniformBuffer = this.device.createBuffer({
        size: 32,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      });
      this.computeBindGroups = [
        this.computeBindGroup(0, 1),
        this.computeBindGroup(1, 0),
      ];
      this.renderBindGroups = [0, 1].map((index) => this.device.createBindGroup({
        layout: this.renderPipeline.getBindGroupLayout(0),
        entries: [
          { binding: 0, resource: { buffer: this.particleBuffers[index] } },
          { binding: 1, resource: { buffer: this.uniformBuffer } },
        ],
      }));
    }

    computeBindGroup(source, destination) {
      return this.device.createBindGroup({
        layout: this.computePipeline.getBindGroupLayout(0),
        entries: [
          { binding: 0, resource: { buffer: this.particleBuffers[source] } },
          { binding: 1, resource: { buffer: this.particleBuffers[destination] } },
          { binding: 2, resource: { buffer: this.uniformBuffer } },
        ],
      });
    }

    seedParticles() {
      if (!this.device || !this.control) {
        return;
      }
      const count = this.control.particleCount;
      const state = new Float32Array(count * 4);
      let seed = 0x6d2b79f5;
      const random = () => {
        seed = Math.imul(seed ^ (seed >>> 15), seed | 1);
        seed ^= seed + Math.imul(seed ^ (seed >>> 7), seed | 61);
        return ((seed ^ (seed >>> 14)) >>> 0) / 4294967296;
      };
      for (let index = 0; index < count; index += 1) {
        const angle = random() * Math.PI * 2;
        const radius = Math.sqrt(random()) * 0.96 + 0.018;
        const speed = Math.sqrt(Math.max(this.control.gravity, 0.02) / (0.42 + radius)) * 0.44;
        const offset = index * 4;
        state[offset] = Math.cos(angle) * radius;
        state[offset + 1] = Math.sin(angle) * radius;
        state[offset + 2] = -Math.sin(angle) * speed + (random() - 0.5) * 0.018;
        state[offset + 3] = Math.cos(angle) * speed + (random() - 0.5) * 0.018;
      }
      this.device.queue.writeBuffer(this.particleBuffers[0], 0, state);
      this.device.queue.writeBuffer(this.particleBuffers[1], 0, state);
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
    }

    writeUniforms(deltaSeconds, timeSeconds) {
      const bytes = new ArrayBuffer(32);
      const view = new DataView(bytes);
      view.setFloat32(0, deltaSeconds, true);
      view.setFloat32(4, timeSeconds, true);
      view.setUint32(8, this.control.particleCount, true);
      view.setFloat32(12, this.control.gravity, true);
      view.setFloat32(16, this.control.drag, true);
      view.setFloat32(20, this.control.pointSize, true);
      view.setFloat32(24, 2 / this.canvas.width, true);
      view.setFloat32(28, 2 / this.canvas.height, true);
      this.device.queue.writeBuffer(this.uniformBuffer, 0, bytes);
    }

    frame(time) {
      if (!this.running) {
        return;
      }
      this.resize();
      const elapsed = Math.min((time - this.lastFrame) / 1000, 0.05);
      this.lastFrame = time;
      const delta = this.paused ? 0 : elapsed * this.control.timeScale;
      this.writeUniforms(delta, time / 1000);

      const destination = 1 - this.activeBuffer;
      const encoder = this.device.createCommandEncoder();
      if (!this.paused) {
        const compute = encoder.beginComputePass();
        compute.setPipeline(this.computePipeline);
        compute.setBindGroup(0, this.computeBindGroups[this.activeBuffer]);
        compute.dispatchWorkgroups(Math.ceil(this.control.particleCount / 256));
        compute.end();
      }

      const renderedBuffer = this.paused ? this.activeBuffer : destination;
      const render = encoder.beginRenderPass({
        colorAttachments: [{
          view: this.context.getCurrentTexture().createView(),
          clearValue: { r: 0.018, g: 0.026, b: 0.039, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        }],
      });
      render.setPipeline(this.renderPipeline);
      render.setBindGroup(0, this.renderBindGroups[renderedBuffer]);
      render.draw(6, this.control.particleCount);
      render.end();
      this.device.queue.submit([encoder.finish()]);

      if (!this.paused) {
        this.activeBuffer = destination;
        this.generation += 1;
      }
      this.statsFrames += 1;
      if (time - this.statsStart >= 500) {
        const fps = (this.statsFrames * 1000) / (time - this.statsStart);
        this.updateTelemetry(fps);
        this.statsFrames = 0;
        this.statsStart = time;
      }
      this.frameHandle = requestAnimationFrame((next) => this.frame(next));
    }

    updateTelemetry(fps) {
      const count = this.control ? this.control.particleCount : 0;
      this.setText("#particle-count", count.toLocaleString());
      this.setText("#frame-rate", `${fps.toFixed(1)} Hz`);
      this.setText("#throughput", `${(count * fps / 1_000_000).toFixed(1)} M updates/s`);
      this.setText("#resident-state", `${(this.maxParticles * 32 / 1_048_576).toFixed(1)} MB resident`);
    }

    setText(selector, value) {
      const element = document.querySelector(selector);
      if (element) {
        element.textContent = value;
      }
    }

    setStatus(message, kind) {
      if (!this.status) {
        return;
      }
      this.status.className = `status ${kind}`.trim();
      this.status.innerHTML = "<span aria-hidden=\"true\"></span>";
      this.status.append(document.createTextNode(message));
    }

    fail(error) {
      console.error("Mech GPU particle host failed", error);
      this.running = false;
      this.setStatus("GPU unavailable", "error");
      this.message.textContent = error instanceof Error ? error.message : String(error);
    }

    updateSelectedCount() {
      document.querySelectorAll("[data-particle-count]").forEach((button) => {
        button.setAttribute(
          "aria-pressed",
          String(Number(button.dataset.particleCount) === this.control.particleCount),
        );
      });
    }

    bindControls() {
      document.querySelectorAll("[data-particle-count]").forEach((button) => {
        button.addEventListener("click", () => {
          if (!this.control) {
            return;
          }
          this.control.particleCount = Math.min(
            this.maxParticles,
            Number(button.dataset.particleCount),
          );
          this.updateSelectedCount();
          this.seedParticles();
        });
      });
      document.querySelector("#pause-button")?.addEventListener("click", (event) => {
        this.paused = !this.paused;
        event.currentTarget.textContent = this.paused ? "Resume" : "Pause";
      });
      document.querySelector("#reset-button")?.addEventListener("click", () => this.seedParticles());
    }
  }

  globalThis.MechGpuParticles = Object.freeze({
    configure(instance, selector, maxParticles, control) {
      let simulation = simulations.get(instance);
      if (!simulation) {
        simulation = new ParticleSimulation(instance, selector, maxParticles);
        simulations.set(instance, simulation);
      }
      simulation.configure(control);
      return true;
    },
  });
})();
