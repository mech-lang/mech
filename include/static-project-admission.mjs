// Native bundle publication runs the supplied browser package's own compiler.
// This probe never constructs a live project or starts a host/input driver.
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { join } from 'node:path';

try {
  if (Number.parseInt(process.versions.node, 10) < 22) {
    throw new Error('bundle-web source admission requires Node.js 22 or newer');
  }
  let input = '';
  for await (const chunk of process.stdin) input += chunk;
  const request = JSON.parse(input);
  const packagePath = process.argv[1];
  const { default: init, WasmProject } = await import(pathToFileURL(join(packagePath, 'mech_wasm.js')));
  if (typeof WasmProject?.validateStaticSources !== 'function' ||
      typeof WasmProject?.supportsServedDocumentProvenance !== 'function') {
    throw new Error('rebuild serve.wasm with static source admission and nominal provenance support');
  }
  await init({ module_or_path: await readFile(join(packagePath, 'mech_wasm_bg.wasm')) });
  if (WasmProject.supportsServedDocumentProvenance() !== true) {
    throw new Error('rebuild serve.wasm with nominal provenance support');
  }
  if (WasmProject.validateStaticSources(request.config, request.sources, request.roots,
      request.resolutions, request.provenance) !== true) {
    throw new Error('browser package did not admit the retained source closure');
  }
} catch (error) {
  console.error(String(error));
  process.exitCode = 1;
}
