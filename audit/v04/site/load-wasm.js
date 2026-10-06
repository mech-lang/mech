import {fetchAuditWasm} from './wasm-transport.js';
export async function loadAuditWasm(initialize) {
  const [artifactResponse, wasmResponse] = await Promise.all([
    fetch('artifact.json'),
    fetchAuditWasm(),
  ]);
  if (!artifactResponse.ok || !wasmResponse.ok) {
    throw new Error('Executable package or provenance record unavailable');
  }
  const artifact = await artifactResponse.json();
  const bytes = await wasmResponse.arrayBuffer();
  const hash = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)))
    .map(value => value.toString(16).padStart(2, '0')).join('');
  if (hash !== artifact.artifacts['pkg/mech_wasm_bg.wasm'].sha256) {
    throw new Error('Loaded WASM bytes differ from the recorded executable');
  }
  await initialize({module_or_path: bytes});
  return {...artifact, loaded_wasm_sha256: hash};
}
