# Browser DOM resource demo

This example demonstrates browser DOM resources backed by runtime resource providers.

It shows a full DOM -> Mech -> DOM round trip:

- `mech serve` loads `demo.mcfg` and injects a derived host config as `window.__MECH_HOST_CONFIG`.
- `WasmMech.fromHostConfig()` initializes the browser runtime from the host-owned config projection.
- The page fetches `/code/*.mec` encoded compiled payloads and executes them with `WasmMech.evalCompiled()`.
- Mech code binds `@browser := browser://dom/`.
- Mech reads the source input value from `@browser/body/content/mech-sandbox/input/_value`.
- Mech computes `greeting`, `roundtrip`, and `status` strings from that DOM value.
- Mech writes the computed output back to `@browser/body/content/mech-sandbox/output/_value`.
- Mech writes DOM text to `@browser/body/content/mech-sandbox/title` and `@browser/body/content/mech-sandbox/status`.
- Mech writes the `class` attribute through `@browser/body/content/mech-sandbox/status/_class`.
- `denied.mec` attempts to write to the read-only source input and is rejected by config permissions.

## Files

- `index.html` — browser page that loads the WASM package and runs the demo.
- `demo.mcfg` — runtime/browser config and DOM read/write permissions.
- `demo.mec` — allowed program that reads a DOM value, performs string logic in Mech, and writes DOM output.
- `denied.mec` — program that attempts a denied DOM write.

## Running

Build the Mech WASM package, then run the host-owned config flow with `mech serve`:

```text
cd src/wasm
wasm-pack build --target web
cd ../..
# If the server expects the pre-compressed asset, refresh it after rebuilding.
brotli -Z --force src/wasm/pkg/mech_wasm_bg.wasm
cargo run --bin mech -- --config examples/browser-dom-demo/demo.mcfg serve
```

Rebuild `src/wasm/pkg` every time the `WasmMech` browser API changes so the served demo HTML and the wasm-bindgen JavaScript package are from the same commit. Before constructing `WasmMech`, you can verify the host-config constructor is present in DevTools with:

```js
Object.getOwnPropertyNames(WasmMech).includes("fromHostConfig")
```

Open:

```text
http://127.0.0.1:8081/
```

`mech serve` loads the config, uses the `serve` section for the address, port, source paths, shim, and WASM package, and injects a derived host config as `window.__MECH_HOST_CONFIG`.

Change the `Source DOM input` field, click `Run round trip`, and observe that Mech reads the DOM value, computes new strings, and writes the computed result back into the page.
