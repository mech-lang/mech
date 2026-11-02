// Test-only observer for a real emitted static bundle. The shipping bootstrap,
// pointer driver, frame loop, selected backend, readback plan and completion
// publication remain authoritative. Do not construct a substitute project.
import { WasmProject } from "../pkg/mech_wasm.js";
import "./browser-compute.js";

const bootstrap = [...document.querySelectorAll('script[type="module"][src]')]
  .find(script => new URL(script.src, document.baseURI).href === import.meta.url);
const expected = bootstrap.dataset.mechBackend === "wgpu" ? "wgpu" : "cpu-scalar";
const state = { project: null, session: null, started: false, pointerInputs: 0,
  submitted: [], accepted: [], errors: [], adapter: null };
const recordError = error => state.errors.push(String(error?.stack || error));
window.addEventListener("error", event => recordError(event.error || event.message));
window.addEventListener("unhandledrejection", event => recordError(event.reason));
const originalError = console.error;
console.error = (...args) => { recordError(args.map(String).join(" ")); originalError(...args); };

const construct = WasmProject.fromServedDocuments;
WasmProject.fromServedDocuments = function (...args) {
  const project = construct.apply(this, args);
  if (state.project) throw new Error("static client constructed more than one project");
  if (project.computeBackend() !== expected) {
    throw new Error(`selected backend ${project.computeBackend()} instead of ${expected}`);
  }
  state.project = project;
  return project;
};
const start = WasmProject.prototype.start;
WasmProject.prototype.start = function (...args) {
  const result = start.apply(this, args);
  if (this === state.project) state.started = true;
  return result;
};
const pointerInput = WasmProject.prototype.pointerInput;
WasmProject.prototype.pointerInput = function (...args) {
  const result = pointerInput.apply(this, args);
  if (this === state.project) state.pointerInputs += 1;
  return result;
};

const { Device, Session } = globalThis.MechBrowserCompute;
const create = Device.create;
Device.create = async function (manifest, adapter, ...rest) {
  state.adapter = adapter.info ? {
    vendor: adapter.info.vendor, architecture: adapter.info.architecture,
    device: adapter.info.device, description: adapter.info.description,
  } : null;
  return create.call(this, manifest, adapter, ...rest);
};
const submit = Session.prototype.submit;
Session.prototype.submit = function (command, hooks = {}) {
  if (!command.requestedOutputs?.includes("result")) {
    throw new Error("source interface did not request its result readback");
  }
  if (state.submitted.includes(command.dispatchToken)) {
    throw new Error("duplicate accepted command token");
  }
  state.session = this;
  return submit.call(this, command, {
    ...hooks,
    onSubmitted(value) {
      state.submitted.push(value.dispatchToken);
      hooks.onSubmitted?.(value);
    },
    onAccepted(value) {
      if (value.integrity) throw new Error(`integrity rejection: ${JSON.stringify(value.integrity)}`);
      const result = value.outputs.find(output => output.name === "result");
      if (!result) throw new Error("missing actual completed result readback");
      state.accepted.push({ token: value.dispatchToken, values: Array.from(result.values) });
      hooks.onAccepted?.(value);
    },
    onFailure(error) { recordError(error); hooks.onFailure?.(error); },
  });
};

window.staticComputeProbe = {
  snapshot() {
    return {
      url: window.location.href,
      ready: state.started && state.project.hasPointerInput(),
      backend: state.project?.computeBackend() || null,
      answer: state.project?.renderedSymbol("answer")?.inlineHtml || null,
      pointerInputs: state.pointerInputs,
      submitted: state.submitted, accepted: state.accepted,
      pending: state.session?.pending || false,
      adapter: state.adapter, errors: state.errors,
    };
  },
};
const actual = document.createElement("script");
actual.type = "module";
actual.src = bootstrap.dataset.mechOriginal;
for (const [name, value] of Object.entries(bootstrap.dataset)) {
  actual.dataset[name] = value;
}
document.body.appendChild(actual);
