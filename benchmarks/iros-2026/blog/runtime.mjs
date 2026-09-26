// The document REPL and workshop kernel share one compiled v0.4 WASM instance.
// Both import this module so concurrent startup cannot initialize it twice.
import initializeWasm from '../_mech/pkg/mech_wasm.js';
export * from '../_mech/pkg/mech_wasm.js';

let initialization;

async function loadRuntime() {
  if (typeof DecompressionStream !== 'undefined') {
    let bytes;
    try {
      const response = await fetch(new URL('../_mech/pkg/mech_wasm_bg.wasm.gz', import.meta.url));
      if (response.ok) {
        bytes = await response.arrayBuffer();
        const magic = new Uint8Array(bytes, 0, Math.min(bytes.byteLength, 2));
        // Fetch may already have decoded a host's Content-Encoding: gzip.
        if (magic[0] === 0x1f && magic[1] === 0x8b) {
          bytes = await new Response(
            new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip')),
          ).arrayBuffer();
        }
      }
    } catch {
      // Hosts without the compressed companion can serve the raw WASM file.
    }
    if (bytes) return initializeWasm({ module_or_path: bytes });
  }
  return initializeWasm();
}

export default function initializeRuntime() {
  initialization ||= loadRuntime();
  return initialization;
}
