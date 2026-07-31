import init, { WasmProject } from '/_mech/pkg/mech_wasm.js';

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

async function main() {
  await init();
  const config = await fetchText('mech.mcfg');
  const manifest =
    await readProjectSourceManifest(import.meta.url);

  const sourceEntries =
    manifest?.sources ??
    Array.from(WasmProject.requiredPaths(config), path => ({
      specifier: path,
      url: path,
    }));

  const sources = {};

  for (const source of sourceEntries) {
    sources[source.specifier] =
      await fetchText(source.url);
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
});
