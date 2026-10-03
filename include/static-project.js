import init, { WasmProject } from "../pkg/mech_wasm.js";

function findBootstrapScript(ownerDocument, moduleUrl) {
  const resolvedModuleUrl = new URL(moduleUrl, ownerDocument.baseURI).href;
  for (const candidate of ownerDocument.querySelectorAll('script[type="module"][src]')) {
    if (new URL(candidate.getAttribute("src"), ownerDocument.baseURI).href === resolvedModuleUrl) {
      return candidate;
    }
  }
  throw new Error(`unable to find static mech bootstrap script for ${resolvedModuleUrl}`);
}

function readBootstrapOptions(script, locationUrl) {
  const projectBase = new URL(script.dataset.mechProject || ".", locationUrl);
  const rawMaxInputs = script.dataset.mechMaxInputs || "8";
  const maxInputsPerFrame = Number.parseInt(rawMaxInputs, 10);
  if (!Number.isFinite(maxInputsPerFrame) || maxInputsPerFrame <= 0 || `${maxInputsPerFrame}` !== rawMaxInputs.trim()) {
    throw new Error("data-mech-max-inputs must be a positive integer");
  }
  return { projectBase, maxInputsPerFrame };
}

const script = findBootstrapScript(document, import.meta.url);
const { projectBase, maxInputsPerFrame } = readBootstrapOptions(script, window.location.href);
let project;
let running = false;
let releasePointerInput = () => {};

function initializePointerInput() {
  if (typeof project.hasPointerInput !== "function" || !project.hasPointerInput()) return;
  if (typeof project.pointerInput !== "function") {
    throw new Error("static bundle WASM profile is missing the live pointer input API");
  }
  let pressed = false;
  let timestamp = null;
  const submit = event => {
    if (!running) return;
    const bounds = document.documentElement.getBoundingClientRect();
    if (bounds.width <= 0 || bounds.height <= 0 ||
        !Number.isFinite(event.clientX) || !Number.isFinite(event.clientY) ||
        !Number.isFinite(event.timeStamp)) return;
    const x = Math.max(-1, Math.min(1, ((event.clientX - bounds.left) / bounds.width) * 2 - 1));
    const y = Math.max(-1, Math.min(1, 1 - ((event.clientY - bounds.top) / bounds.height) * 2));
    const deltaSeconds = timestamp === null ? 0 : Math.max(0, Math.min(1, (event.timeStamp - timestamp) / 1000));
    timestamp = event.timeStamp;
    try {
      project.pointerInput(x, y, pressed, deltaSeconds);
    } catch (error) {
      stopProject(error);
    }
  };
  const down = event => {
    if (event.button !== 0) return;
    pressed = true;
    submit(event);
  };
  const release = event => {
    if (!pressed) return;
    pressed = false;
    submit(event);
  };
  const up = event => {
    if (event.button === 0) release(event);
  };
  const listeners = [["pointermove", submit], ["pointerdown", down], ["pointerup", up], ["pointercancel", release]];
  for (const [name, listener] of listeners) window.addEventListener(name, listener);
  releasePointerInput = () => {
    for (const [name, listener] of listeners) window.removeEventListener(name, listener);
    releasePointerInput = () => {};
  };
}

function stopProject(error) {
  running = false;
  releasePointerInput();
  if (project) {
    try { project.stop(); } catch (stopError) { console.error(stopError); }
  }
  if (error) console.error(error);
}

async function fetchText(path) {
  const response = await fetch(new URL(path, projectBase));
  if (!response.ok) {
    throw new Error(`failed to fetch ${path}: ${response.status} ${response.statusText}`);
  }
  return await response.text();
}

async function readProjectSourceManifest(moduleUrl) {
  const response = await fetch(new URL("project-sources.json", moduleUrl));
  if (!response.ok) {
    throw new Error(`failed to fetch project source manifest: ${response.status} ${response.statusText}`);
  }

  let manifest;
  try {
    manifest = await response.json();
  } catch {
    throw new Error("invalid project source manifest");
  }

  if (
    manifest?.version !== 4 ||
    !Array.isArray(manifest.roots) ||
    manifest.roots.length === 0 ||
    manifest.roots.some(root => typeof root !== "string") ||
    !Array.isArray(manifest.sources) ||
    manifest.sources.some(
      source =>
        typeof source?.specifier !== "string" ||
        typeof source?.url !== "string" ||
        (source.documentUrl !== undefined && typeof source.documentUrl !== "string") ||
        (source.nominalOrigin !== undefined &&
          (!Array.isArray(source.nominalOrigin?.segments) ||
           source.nominalOrigin.segments.length === 0 ||
           source.nominalOrigin.segments.some(segment => typeof segment !== "string"))) ||
        (source.nominalPackageId !== undefined &&
          (source.nominalOrigin === undefined || typeof source.nominalPackageId !== "string")),
    ) ||
    !Array.isArray(manifest.resolutions) ||
    manifest.resolutions.some(
      resolution =>
        typeof resolution?.referrer !== "string" ||
        typeof resolution?.specifier !== "string" ||
        typeof resolution?.target !== "string",
    )
  ) {
    throw new Error("invalid project source manifest");
  }

  for (const root of manifest.roots) {
    if (!manifest.sources.some(source => source.specifier === root && typeof source.documentUrl === "string")) {
      throw new Error(`static bundle root document is missing: ${root}`);
    }
  }
  return manifest;
}

async function main() {
  await init();
  if (
    typeof WasmProject.fromServedDocuments !== "function" ||
    typeof WasmProject.supportsServedAuthority !== "function" ||
    WasmProject.supportsServedAuthority() !== true ||
    typeof WasmProject.supportsServedDocumentResolutions !== "function" ||
    WasmProject.supportsServedDocumentResolutions() !== true ||
    typeof WasmProject.supportsServedDocumentProvenance !== "function" ||
    WasmProject.supportsServedDocumentProvenance() !== true
  ) {
    throw new Error("static bundle WASM profile mismatch: rebuild with browser_project support");
  }
  const config = await fetchText("mech.mcfg");
  const manifest = await readProjectSourceManifest(import.meta.url);
  const sources = Object.create(null);
  const documents = Object.create(null);
  const provenance = Object.create(null);

  for (const source of manifest.sources) {
    sources[source.specifier] = await fetchText(source.url);
    if (source.documentUrl !== undefined) {
      documents[source.specifier] = await fetchText(source.documentUrl);
    }
    if (source.nominalOrigin !== undefined) {
      provenance[source.specifier] = {
        nominalOrigin: source.nominalOrigin,
        nominalPackageId: source.nominalPackageId ?? null,
      };
    }
  }

  if (!Object.prototype.hasOwnProperty.call(window, "__MECH_HOST_CONFIG")) {
    throw new Error("static bundle is missing injected browser host authority");
  }

  project = WasmProject.fromServedDocuments(
    config,
    sources,
    documents,
    manifest.roots,
    manifest.resolutions,
    provenance,
  );
  project.start();
  running = true;
  initializePointerInput();
  requestAnimationFrame(frame);
}

function frame() {
  if (!running || !project) {
    return;
  }
  try {
    project.frame(maxInputsPerFrame);
  } catch (error) {
    stopProject(error);
    return;
  }
  requestAnimationFrame(frame);
}

window.addEventListener("beforeunload", () => {
  stopProject();
});

main().catch((error) => {
  stopProject(error);
});
