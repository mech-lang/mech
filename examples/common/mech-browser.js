import init, { WasmProject } from '../pkg/mech_wasm.js';

const script = document.currentScript;
const projectBase = new URL(script?.dataset?.mechProject || '.', window.location.href);
const maxInputsPerFrame = Number.parseInt(script?.dataset?.mechMaxInputs || '8', 10);
let project;
let running = false;

async function fetchText(path) {
  const response = await fetch(new URL(path, projectBase));
  if (!response.ok) {
    throw new Error(`failed to fetch ${path}: ${response.status} ${response.statusText}`);
  }
  return await response.text();
}

async function main() {
  if (!Number.isFinite(maxInputsPerFrame) || maxInputsPerFrame <= 0) {
    throw new Error('data-mech-max-inputs must be a positive integer');
  }
  await init();
  const config = await fetchText('mech.mcfg');
  const paths = WasmProject.requiredPaths(config);
  const sources = {};
  for (const path of paths) {
    sources[path] = await fetchText(path);
  }
  project = WasmProject.fromSources(config, sources);
  project.start();
  running = true;
  requestAnimationFrame(frame);
}

function frame() {
  if (!running || !project) {
    return;
  }
  try {
    project.frame(maxInputsPerFrame);
  } catch (error) {
    running = false;
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
});
