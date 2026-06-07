# Browser DOM resource demo

This example demonstrates browser DOM resources backed by runtime resource providers.

It shows the intended host flow for browser configuration:

- `mech serve` loads `demo.mcfg` as the host config.
- The `serve` section selects `index.html` as the custom shim, chooses the Mech source files, and points at the WASM package.
- `mech serve` keeps the config host-owned and does not serve it to the browser as a file.
- The custom shim initializes `WasmMech.fromHostConfig()` from host-provided runtime/browser configuration injected by `mech serve`.
- DOM permissions come from the host config loaded by `mech serve`, not an arbitrary app-level fetch of `demo.mcfg`.

The demo performs a full DOM -> Mech -> DOM round trip:

- Mech code binds `@browser := browser://dom/`.
- Mech reads the source input value from `body/content/mech-sandbox/input/_value@browser`.
- Mech computes `greeting`, `roundtrip`, and `status` strings from that DOM value.
- Mech writes the computed output back to `body/content/mech-sandbox/output/_value@browser`.
- Mech writes DOM text to `body/content/mech-sandbox/title@browser` and `body/content/mech-sandbox/status@browser`.
- Mech writes the `class` attribute through `body/content/mech-sandbox/status/_class@browser`.
- `denied.mec` attempts to write to the read-only source input and is rejected by config permissions.

## Files

- `demo.mcfg` — runtime, serve, and browser host config, including DOM read/write permissions.
- `index.html` — custom shim page that loads the WASM package, uses host-provided config, and fetches Mech source through `/code/...`.
- `demo.mec` — allowed program that reads a DOM value, performs string logic in Mech, and writes DOM output.
- `denied.mec` — program that attempts a denied DOM write.

## Running

Build or copy the Mech WASM package into `src/wasm/pkg/` so the configured `serve.wasm` path contains:

```text
src/wasm/pkg/mech_wasm.js
src/wasm/pkg/mech_wasm_bg.wasm.br
```

Then run the demo through `mech serve` with the config file:

```text
cargo run --bin mech -- --config examples/browser-dom-demo/demo.mcfg serve
```

Open `http://127.0.0.1:8081/` in a browser.

`mech serve` reads `examples/browser-dom-demo/demo.mcfg`, resolves `serve.paths`, `serve.shim`, and `serve.wasm` relative to that config file, serves `index.html` at `/`, and keeps the host config internal instead of exposing it as a browser-fetchable file. The custom shim initializes `WasmMech.fromHostConfig()` from host-provided runtime/browser configuration and fetches Mech code through `/code/demo.mec` and `/code/denied.mec`.

Change the `Source DOM input` field, click `Run round trip`, and observe that Mech reads the DOM value, computes new strings, and writes the computed result back into the page.
