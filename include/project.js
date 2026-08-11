import init, * as mech from '/_mech/pkg/mech_wasm.js';

const { WasmProject } = mech;

export function findBootstrapScript(ownerDocument, moduleUrl) {
  const resolvedModuleUrl = new URL(moduleUrl, ownerDocument.baseURI).href;
  for (const candidate of ownerDocument.querySelectorAll('script[type="module"][src]')) {
    if (new URL(candidate.getAttribute('src'), ownerDocument.baseURI).href === resolvedModuleUrl) {
      return candidate;
    }
  }
  throw new Error(`unable to find mech browser bootstrap script for ${resolvedModuleUrl}`);
}

export function readBootstrapOptions(script, locationUrl) {
  const projectBase = new URL(script.dataset.mechProject || '.', locationUrl);
  const rawMaxInputs = script.dataset.mechMaxInputs || '8';
  const maxInputsPerFrame = Number.parseInt(rawMaxInputs, 10);
  if (!Number.isFinite(maxInputsPerFrame) || maxInputsPerFrame <= 0 || `${maxInputsPerFrame}` !== rawMaxInputs.trim()) {
    throw new Error('data-mech-max-inputs must be a positive integer');
  }
  return { projectBase, maxInputsPerFrame };
}

const script = findBootstrapScript(document, import.meta.url);
const { projectBase, maxInputsPerFrame } = readBootstrapOptions(script, window.location.href);
let project;
let running = false;

async function fetchText(path) {
  const response = await fetch(new URL(path, projectBase));
  if (!response.ok) {
    throw new Error(`failed to fetch ${path}: ${response.status} ${response.statusText}`);
  }
  return await response.text();
}

async function readProjectSourceManifest(moduleUrl) {
  const response = await fetch(
    new URL('project-sources.json', moduleUrl),
  );

  if (response.status === 404) {
    return null;
  }

  if (!response.ok) {
    throw new Error(
      `failed to fetch project source manifest: ` +
      `${response.status} ${response.statusText}`,
    );
  }

  let manifest;

  try {
    manifest = await response.json();
  } catch {
    throw new Error('invalid project source manifest');
  }

  if (
    (manifest?.version !== 1 && manifest?.version !== 2) ||
    !Array.isArray(manifest.sources) ||
    (manifest.version === 2 &&
      (!Array.isArray(manifest.roots) ||
        !Array.isArray(manifest.resolutions))) ||
    manifest.sources.some(
      source =>
        typeof source?.specifier !== 'string' ||
        typeof source?.url !== 'string',
    )
  ) {
    throw new Error('invalid project source manifest');
  }

  if (manifest.version === 2) {
    const sourceSpecifiers = new Set(
      manifest.sources.map(source => source.specifier),
    );
    if (manifest.roots.some(
      specifier =>
        typeof specifier !== 'string' ||
        !specifier.trim() ||
        !sourceSpecifiers.has(specifier)
    )) {
      throw new Error('invalid project root source identity');
    }
  }

  return manifest;
}

const pointRendererShader = /* wgsl */ `
struct RenderParams {
  point_size: f32,
  pixel_x: f32,
  pixel_y: f32,
  interleaved: f32,
  sample_stride: f32,
  _padding_0: f32,
  _padding_1: f32,
  _padding_2: f32,
}

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) local: vec2f,
  @location(1) color: vec3f,
}

@group(0) @binding(0) var<storage, read> points: array<f32>;
@group(0) @binding(1) var<uniform> params: RenderParams;

const corners = array<vec2f, 6>(
  vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
  vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0)
);

@vertex
fn vertex_main(
  @builtin(vertex_index) vertex_index: u32,
  @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
  let count = arrayLength(&points) / 2u;
  let point_index = min(instance_index * u32(params.sample_stride), count - 1u);
  var point: vec2f;
  if (params.interleaved > 0.5) {
    let offset = point_index * 2u;
    point = vec2f(points[offset], points[offset + 1u]);
  } else {
    point = vec2f(points[point_index], points[count + point_index]);
  }
  let local = corners[vertex_index];
  let size = vec2f(params.pixel_x, params.pixel_y) * params.point_size;
  let radius = clamp(length(point), 0.0, 1.0);

  var output: VertexOutput;
  output.position = vec4f(point + local * size, 0.0, 1.0);
  output.local = local;
  output.color = mix(vec3f(0.20, 0.82, 0.74), vec3f(0.54, 0.64, 1.0), radius);
  return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4f {
  let radius = length(input.local);
  if (radius > 1.0) {
    discard;
  }
  let alpha = (1.0 - smoothstep(0.45, 1.0, radius)) * 0.72;
  return vec4f(input.color * alpha, alpha);
}
`;

class BrowserGpuProject {
  static async fromSources(sourceEntries, sources) {
    if (typeof mech.compileGpuProgram !== 'function') {
      throw new Error('WASM build-profile mismatch: the configured GPU executor is unavailable');
    }
    if (sourceEntries.length !== 1) {
      throw new Error(`the browser GPU executor currently requires one Mech source, found ${sourceEntries.length}`);
    }
    const source = sources[sourceEntries[0].specifier];
    const compileStarted = performance.now();
    const manifest = mech.compileGpuProgram(source);
    return new BrowserGpuProject(manifest, performance.now() - compileStarted);
  }

  constructor(manifest, compileMilliseconds) {
    this.manifest = manifest;
    this.compileMilliseconds = compileMilliseconds;
    this.canvas = document.querySelector('canvas[data-mech-gpu-renderer="points2d"]');
    this.status = document.querySelector('[data-mech-gpu-status]');
    this.message = document.querySelector('[data-mech-gpu-message]');
    this.activeBuffer = 0;
    this.stopped = false;
    this.statsStarted = 0;
    this.statsFrames = 0;
  }

  async start() {
    if (!(this.canvas instanceof HTMLCanvasElement)) {
      throw new Error('the GPU executor needs a points2d canvas target');
    }
    if (!navigator.gpu) {
      throw new Error('WebGPU is unavailable in this browser');
    }
    const outputName = this.canvas.dataset.mechGpuOutput;
    this.output = this.manifest.outputs.find((candidate) => candidate.name === outputName);
    if (!this.output) {
      throw new Error(`Mech GPU output ${outputName} does not exist`);
    }
    if (
      this.output.dimensions.length !== 2 ||
      (this.output.dimensions[0] !== 2 && this.output.dimensions[1] !== 2)
    ) {
      throw new Error(`points2d requires an N x 2 or 2 x N f32 matrix, got [${this.output.dimensions.join(', ')}]`);
    }
    // Mech matrices use logical row-major order at this host boundary.
    this.interleavedPoints = this.output.dimensions[1] === 2;
    this.itemCount = this.interleavedPoints
      ? this.output.dimensions[0]
      : this.output.dimensions[1];
    const maxRenderedPoints = 250_000;
    this.sampleStride = Math.max(1, Math.ceil(this.itemCount / maxRenderedPoints));
    this.renderItemCount = Math.ceil(this.itemCount / this.sampleStride);
    this.setStatus('Requesting GPU', '');

    this.adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
    if (!this.adapter) {
      throw new Error('no compatible WebGPU adapter was found');
    }
    const storageBindings = this.manifest.bindings.length;
    if (storageBindings > this.adapter.limits.maxStorageBuffersPerShaderStage) {
      throw new Error(
        `Mech generated ${storageBindings} storage bindings, but this adapter supports ` +
        `${this.adapter.limits.maxStorageBuffersPerShaderStage}`,
      );
    }
    const workgroups = Math.ceil(this.manifest.dispatchElements / this.manifest.workgroupSize);
    if (workgroups > this.adapter.limits.maxComputeWorkgroupsPerDimension) {
      throw new Error(
        `Mech needs ${workgroups} workgroups, but this adapter supports ` +
        `${this.adapter.limits.maxComputeWorkgroupsPerDimension}`,
      );
    }
    this.device = await this.adapter.requestDevice({
      requiredLimits: { maxStorageBuffersPerShaderStage: storageBindings },
    });
    this.device.lost.then((info) => {
      this.stopped = true;
      this.setStatus(`GPU device lost: ${info.message}`, 'error');
    });
    this.context = this.canvas.getContext('webgpu');
    if (!this.context) {
      throw new Error('the canvas could not create a WebGPU context');
    }
    this.format = navigator.gpu.getPreferredCanvasFormat();
    await this.createPipelines();
    this.createBuffers();
    this.resize();
    await this.device.queue.onSubmittedWorkDone();
    this.message.textContent = '';
    this.setStatus(`Mech compiled in ${(this.compileMilliseconds / 1000).toFixed(1)} s`, 'ready');
    this.setText('[data-mech-gpu-item-count]', this.itemCount.toLocaleString());
    this.setText('[data-mech-gpu-render-count]', this.renderItemCount.toLocaleString());
    const timings = this.manifest.compileTimings;
    if (timings) {
      this.setText(
        '[data-mech-gpu-compile-time]',
        `${timings.parsing.toFixed(0)} ms parse / ` +
        `${timings.sourceExecution.toFixed(0)} ms init / ` +
        `${(timings.artifactCompilation / 1000).toFixed(1)} s artifact`,
      );
      console.table({
        'catalog setup': timings.catalogSetup,
        'source parsing': timings.parsing,
        'source execution + initialization': timings.sourceExecution,
        'artifact compilation': timings.artifactCompilation,
        'GPU lowering': timings.gpuLowering,
        'input capture': timings.inputCapture,
        'manifest encoding': timings.manifestEncoding,
      });
    }
    const residentBytes = [...this.stateBuffers.values()]
      .reduce((total, buffers) => total + buffers[0].size + buffers[1].size, 0);
    this.setText('[data-mech-gpu-state-size]', `${(residentBytes / 1_048_576).toFixed(1)} MB resident`);
    this.statsStarted = performance.now();
  }

  async createPipelines() {
    const computeModule = this.device.createShaderModule({ code: this.manifest.wgsl });
    const compilation = await computeModule.getCompilationInfo();
    const errors = compilation.messages.filter((message) => message.type === 'error');
    if (errors.length > 0) {
      throw new Error(errors.map((message) => message.message).join('\n'));
    }
    this.computePipeline = this.device.createComputePipeline({
      layout: 'auto',
      compute: { module: computeModule, entryPoint: 'main' },
    });
    const renderModule = this.device.createShaderModule({ code: pointRendererShader });
    this.renderPipeline = this.device.createRenderPipeline({
      layout: 'auto',
      vertex: { module: renderModule, entryPoint: 'vertex_main' },
      fragment: {
        module: renderModule,
        entryPoint: 'fragment_main',
        targets: [{
          format: this.format,
          blend: {
            color: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
          },
        }],
      },
      primitive: { topology: 'triangle-list' },
    });
  }

  createBuffers() {
    this.stateBuffers = new Map();
    for (const state of this.manifest.states) {
      const buffers = [0, 1].map(() => this.device.createBuffer({
        size: state.elements * Float32Array.BYTES_PER_ELEMENT,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      }));
      this.device.queue.writeBuffer(buffers[0], 0, state.initialValues);
      state.initialValues = null;
      this.stateBuffers.set(state.slot, buffers);
    }

    this.fixedBuffers = new Map();
    for (const binding of this.manifest.bindings) {
      if (binding.role === 'state-read' || binding.role === 'state-write') {
        continue;
      }
      const buffer = this.device.createBuffer({
        size: Math.max(4, binding.elements * Float32Array.BYTES_PER_ELEMENT),
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      });
      if (binding.initialValues) {
        this.device.queue.writeBuffer(buffer, 0, binding.initialValues);
        binding.initialValues = null;
      }
      this.fixedBuffers.set(binding.binding, buffer);
    }

    this.computeBindGroups = [0, 1].map((sourceIndex) => this.device.createBindGroup({
      layout: this.computePipeline.getBindGroupLayout(0),
      entries: this.manifest.bindings.map((binding) => ({
        binding: binding.binding,
        resource: { buffer: this.bufferForBinding(binding, sourceIndex) },
      })),
    }));
    this.renderUniform = this.device.createBuffer({
      size: 32,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    this.renderBindGroups = [0, 1].map((index) => this.device.createBindGroup({
      layout: this.renderPipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: this.outputBuffer(index) } },
        { binding: 1, resource: { buffer: this.renderUniform } },
      ],
    }));
  }

  bufferForBinding(binding, sourceIndex) {
    if (binding.role === 'state-read') {
      return this.stateBuffers.get(binding.slot)[sourceIndex];
    }
    if (binding.role === 'state-write') {
      return this.stateBuffers.get(binding.slot)[1 - sourceIndex];
    }
    return this.fixedBuffers.get(binding.binding);
  }

  outputBuffer(index) {
    if (this.stateBuffers.has(this.output.slot)) {
      return this.stateBuffers.get(this.output.slot)[index];
    }
    const binding = this.manifest.bindings.find(
      (candidate) => candidate.role === 'output' && candidate.slot === this.output.slot,
    );
    if (!binding) {
      throw new Error(`GPU output ${this.output.name} has no physical buffer`);
    }
    return this.fixedBuffers.get(binding.binding);
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
        alphaMode: 'premultiplied',
      });
    }
    const pointSize = this.itemCount >= 1_000_000 ? 1.1 : 1.6;
    this.device.queue.writeBuffer(
      this.renderUniform,
      0,
      new Float32Array([
        pointSize,
        2 / width,
        2 / height,
        this.interleavedPoints ? 1 : 0,
        this.sampleStride,
        0,
        0,
        0,
      ]),
    );
  }

  frame() {
    if (this.stopped) {
      return;
    }
    this.resize();
    const encoder = this.device.createCommandEncoder();
    const compute = encoder.beginComputePass();
    compute.setPipeline(this.computePipeline);
    compute.setBindGroup(0, this.computeBindGroups[this.activeBuffer]);
    compute.dispatchWorkgroups(
      Math.ceil(this.manifest.dispatchElements / this.manifest.workgroupSize),
    );
    compute.end();
    const renderedBuffer = 1 - this.activeBuffer;
    const render = encoder.beginRenderPass({
      colorAttachments: [{
        view: this.context.getCurrentTexture().createView(),
        clearValue: { r: 0.018, g: 0.026, b: 0.039, a: 1 },
        loadOp: 'clear',
        storeOp: 'store',
      }],
    });
    render.setPipeline(this.renderPipeline);
    render.setBindGroup(0, this.renderBindGroups[renderedBuffer]);
    render.draw(6, this.renderItemCount);
    render.end();
    this.device.queue.submit([encoder.finish()]);
    this.activeBuffer = renderedBuffer;

    this.statsFrames += 1;
    const now = performance.now();
    if (now - this.statsStarted >= 500) {
      const rate = (this.statsFrames * 1000) / (now - this.statsStarted);
      this.setText('[data-mech-gpu-frame-rate]', `${rate.toFixed(1)} Hz`);
      this.setText(
        '[data-mech-gpu-throughput]',
        `${(this.itemCount * rate / 1_000_000).toFixed(1)} M updates/s`,
      );
      this.statsStarted = now;
      this.statsFrames = 0;
    }
  }

  stop() {
    this.stopped = true;
  }

  setText(selector, value) {
    const target = document.querySelector(selector);
    if (target) {
      target.textContent = value;
    }
  }

  setStatus(message, kind) {
    if (!this.status) {
      return;
    }
    this.status.className = `status ${kind}`.trim();
    this.status.innerHTML = '<span aria-hidden="true"></span>';
    this.status.append(document.createTextNode(message));
  }
}

async function main() {
  await init();
  const config = await fetchText('mech.mcfg');
  const manifest =
    await readProjectSourceManifest(import.meta.url);

  const requiredPaths = typeof WasmProject?.requiredPaths === 'function'
    ? WasmProject.requiredPaths(config)
    : mech.requiredGpuPaths(config);
  const sourceEntries =
    manifest?.sources ??
    Array.from(requiredPaths, path => ({
      specifier: path,
      url: path,
    }));

  const sources = {};

  for (const source of sourceEntries) {
    sources[source.specifier] =
      await fetchText(source.url);
  }
  const executor = typeof mech.configuredExecutor === 'function'
    ? mech.configuredExecutor(config)
    : null;
  if (executor?.provider === 'gpu') {
    project = await BrowserGpuProject.fromSources(sourceEntries, sources);
    await project.start();
    running = true;
    requestAnimationFrame(frame);
    return;
  }
  const hasServedAuthority = Object.prototype.hasOwnProperty.call(window, '__MECH_HOST_CONFIG');
  if (hasServedAuthority) {
    const supported = typeof WasmProject.supportsServedAuthority === 'function' && WasmProject.supportsServedAuthority() === true;
    const hasGraphConstructor =
      manifest?.version === 2 &&
      typeof WasmProject.fromServedSourcesWithResolutions === 'function';
    if (!supported || (manifest?.version === 2
      ? !hasGraphConstructor
      : typeof WasmProject.fromServedSources !== 'function')) {
      throw new Error('WASM build-profile mismatch: served project authority was injected by the server, but this mech_wasm artifact was not compiled with served_project_authority support');
    }
    project = manifest?.version === 2
      ? WasmProject.fromServedSourcesWithResolutions(
          config,
          sources,
          manifest.resolutions,
        )
      : WasmProject.fromServedSources(config, sources);
  } else {
    if (
      manifest?.version === 2 &&
      typeof WasmProject.fromSourcesWithResolutions !== 'function'
    ) {
      throw new Error('WASM build-profile mismatch: source resolution graph support is unavailable');
    }
    project = manifest?.version === 2
      ? WasmProject.fromSourcesWithResolutions(config, sources, manifest.resolutions)
      : WasmProject.fromSources(config, sources);
  }
  project.start();
  globalThis.__MECH_RUNTIME_INFO__ = () => project.runtimeInfo();
  globalThis.__MECH_LAST_FRAME__ = null;
  globalThis.__MECH_STOP__ = () => {
    running = false;
    project.stop();
  };
  running = true;
  requestAnimationFrame(frame);
}

function frame() {
  if (!running || !project) {
    return;
  }
  try {
    globalThis.__MECH_LAST_FRAME__ = project.frame(maxInputsPerFrame);
  } catch (error) {
    running = false;
    try {
      project.stop();
    } catch (stopError) {
      console.error(stopError);
    }
    console.error(error);
    return;
  }
  requestAnimationFrame(frame);
}

window.addEventListener('beforeunload', () => {
  running = false;
  if (project) {
    try {
      project.stop();
    } catch (error) {
      console.error(error);
    }
  }
});

main().catch((error) => {
  running = false;
  console.error(error);
  const status = document.querySelector('[data-mech-gpu-status]');
  if (status) {
    status.className = 'status error';
    status.innerHTML = '<span aria-hidden="true"></span>';
    status.append(document.createTextNode('GPU unavailable'));
  }
  const message = document.querySelector('[data-mech-gpu-message]');
  if (message) {
    message.textContent = error instanceof Error ? error.message : String(error);
  }
});
