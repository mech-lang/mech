// Validate the exact browser wrapper/module pair before native embedding.
import { readFileSync } from "node:fs";

const [javascript, binary] = process.argv.slice(2);
try {
  const source = readFileSync(javascript, "utf8");
  const module = new WebAssembly.Module(readFileSync(binary));
  const exports = new Map(WebAssembly.Module.exports(module).map(entry => [entry.name, entry.kind]));
  if (![...exports.values()].includes("function")) {
    throw new Error("module has no executable exports");
  }
  if (exports.get("memory") !== "memory") {
    throw new Error("module does not export its browser memory");
  }
  const references = [...source.matchAll(/\bwasm\.([\w$]+)(\s*\()?/g)];
  if (!references.length) throw new Error("wrapper has no module export references");
  for (const [, name, called] of references) {
    if (!exports.has(name) || (called && exports.get(name) !== "function")) {
      throw new Error(`wrapper requires missing or incompatible module export ${name}`);
    }
  }
  // Parsing the actual wrapper also rejects corrupt JavaScript that happens to
  // contain the expected controller method names. wasm-bindgen's web wrapper
  // has no top-level imports; supplying the bytes is unnecessary for this probe.
  const wrapper = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
  for (const [owner, methods] of [
    [wrapper.WasmDocument, ["fromEncodedWithBundle", "fromServedEncodedWithBundle"]],
    [wrapper.WasmProject, ["validateStaticSources", "supportsServedDocumentProvenance"]],
    [wrapper.WasmDocument?.prototype, ["renderedDocumentValue", "renderedProgramOutput", "replInvoke", "replContinueStep"]],
  ]) {
    for (const method of methods) {
      if (typeof owner?.[method] !== "function") throw new Error(`wrapper is missing ${method}`);
    }
  }
} catch (error) {
  console.error(`invalid browser WebAssembly package: ${error.message}`);
  process.exitCode = 1;
}
