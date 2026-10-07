// Transport encoding preserves the executable identified by artifact.json.
const manifestURL = new URL('./pkg/transport.json', import.meta.url);
const digest = async bytes => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), value => value.toString(16).padStart(2, '0')).join('');
let payload;
async function loadPayload() {
  const response = await fetch(manifestURL, {cache: 'no-cache'});
  if (!response.ok) throw new Error(`WASM transport manifest: HTTP ${response.status}`);
  const manifest = await response.json();
  if (manifest.encoding !== 'gzip' || !Array.isArray(manifest.chunks) || !manifest.chunks.length) throw new Error('Invalid WASM transport manifest');
  if (typeof DecompressionStream !== 'function') throw new Error('This browser requires gzip DecompressionStream support to load Mech.');
  const chunks = await Promise.all(manifest.chunks.map(async chunk => {
    const response = await fetch(new URL(chunk.path, manifestURL));
    if (!response.ok) throw new Error(`WASM transport chunk: HTTP ${response.status}`);
    const bytes = await response.arrayBuffer();
    if (bytes.byteLength !== chunk.bytes || await digest(bytes) !== chunk.sha256) throw new Error('WASM transport chunk integrity check failed');
    return bytes;
  }));
  if (chunks.reduce((total, bytes) => total + bytes.byteLength, 0) !== manifest.compressed_bytes) throw new Error('WASM transport size check failed');
  const bytes = await new Response(new Blob(chunks).stream().pipeThrough(new DecompressionStream('gzip'))).arrayBuffer();
  if (bytes.byteLength !== manifest.uncompressed_bytes || await digest(bytes) !== manifest.uncompressed_sha256) throw new Error('Reconstructed WASM integrity check failed');
  return bytes;
}
export async function fetchAuditWasm() {
  payload ??= loadPayload().catch(error => { payload = undefined; throw error; });
  return new Response(await payload, {headers: {'Content-Type': 'application/wasm'}});
}
