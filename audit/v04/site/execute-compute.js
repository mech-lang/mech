import {WasmMixedComputeProject} from './pkg/mech_wasm.js';

function adapterIdentity(adapter) {
  if (!adapter) return null;
  const i = adapter.info;
  return {
    vendor: i?.vendor, architecture: i?.architecture, device: i?.device,
    description: i?.description,
    isFallbackAdapter: adapter.isFallbackAdapter ?? i?.isFallbackAdapter ?? null,
    hardware_classification: (adapter.isFallbackAdapter ?? i?.isFallbackAdapter) === true ? 'software/fallback adapter' : /swiftshader|software|llvmpipe/i.test(`${i?.description} ${i?.device} ${i?.architecture}`)
      ? 'software' : (i?.vendor || i?.architecture) ? 'hardware adapter reported by browser' : 'unresolved',
    limits: {maxStorageBuffersPerShaderStage: adapter.limits.maxStorageBuffersPerShaderStage,
      maxComputeWorkgroupsPerDimension: adapter.limits.maxComputeWorkgroupsPerDimension},
  };
}

// The production resource owns GPU buffers, dispatch and queue completion.
// Expected outputs are supplied by each workload's independent reference.
export async function executeCompute({source, configuration, backend, inputs, expectedOutputs, tolerance}) {
  const adapter = backend === 'gpu' ? await navigator.gpu?.requestAdapter() : null;
  if (backend === 'gpu' && !adapter) return {outcome:'blocked', requested:backend,
    selected:null, completed:null, reason:'WebGPU adapter unavailable', browser:navigator.userAgent};
  const started = performance.now();
  const project = WasmMixedComputeProject.fromSource(configuration, source, backend, Boolean(adapter), undefined);
  const compile_ms = performance.now() - started;
  const selected = project.backend(), manifest = project.computeManifest();
  let resource = null, active = 0;
  const frames = [];
  try {
    const deviceStart = performance.now();
    if (adapter) resource = await MechBrowserCompute.Device.create(manifest, adapter, ['result']);
    const device_preparation_ms = performance.now() - deviceStart;
    project.start();
    for (let index = 0; index < inputs.length; index++) {
      const input = inputs[index], turnStart = performance.now();
      const command = project.frame(input.x, input.y, input.pressed, input.delta_seconds, 8);
      let actual, completed;
      if (resource) {
        resource.setRequestedOutputs(['result']);
        const submission = resource.submit(command, active);
        const result = await resource.finish(submission);
        if (result.integrity) throw Error(JSON.stringify(result.integrity));
        actual = Array.from(result.outputs.find(o => o.name === 'result').values);
        project.completeComputeCommand({version:1, token:command.dispatchToken, status:'completed',
          outputs:result.outputs.filter(o => (command.requestedOutputs || []).includes(o.name))});
        active = submission.outputIndex;
        completed = {backend:'wgpu', token:command.dispatchToken, queue_completed:true,
          readback_completed:true, runtime_acknowledged:true};
      } else {
        if (command.acknowledgementRequired) throw Error('Unexpected CPU acknowledgement requirement');
        actual = Array.from(project.cpuOutput('result'));
        completed = {backend:'cpu-scalar', synchronous:true, output_read_after_frame:true};
      }
      const expected = expectedOutputs[index];
      const max_absolute_error = Math.max(0, ...actual.map((v,i) => Math.abs(v-expected[i])));
      if (actual.length !== expected.length || actual.some(v => !Number.isFinite(v)) || max_absolute_error > tolerance) {
        throw Error(JSON.stringify({turn:index+1, actual:actual.slice(0,16), expected:expected.slice(0,16), max_absolute_error, tolerance}));
      }
      frames.push({turn:index+1, input, completed, elements:actual.length, actual, expected,
        max_absolute_error, tolerance, compute_completion_readback_ms:performance.now()-turnStart});
    }
    return {outcome:'passed', requested:backend, selected, completed:frames.at(-1).completed.backend,
      selection_scope:'Explicit override admitted by Mech capability selection', adapter:adapterIdentity(adapter),
      browser:navigator.userAgent, source, configuration, compile_ms, device_preparation_ms, frames, manifest};
  } finally {
    project.stop();
    resource?.dispose();
    if (resource?.disposeCompletion) await resource.disposeCompletion;
    project.free();
  }
}
