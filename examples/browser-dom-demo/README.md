# Browser DOM resource demo

This example demonstrates browser DOM resources backed by runtime resource providers.

It shows a full DOM -> Mech -> DOM round trip:

- `WasmMech.fromConfig(...)` loads browser DOM grants from `demo.mcfg`.
- Mech code binds `@browser := browser://dom/`.
- Mech reads the source input value from `body/content/mech-sandbox/input/_value@browser`.
- Mech computes `greeting`, `roundtrip`, and `status` strings from that DOM value.
- Mech writes the computed output back to `body/content/mech-sandbox/output/_value@browser`.
- Mech writes DOM text to `body/content/mech-sandbox/title@browser` and `body/content/mech-sandbox/status@browser`.
- Mech writes the `class` attribute through `body/content/mech-sandbox/status/_class@browser`.
- `denied.mec` attempts to write to the read-only source input and is rejected by config permissions.

## Files

- `index.html` — browser page that loads the WASM package and runs the demo.
- `demo.mcfg` — runtime/browser config and DOM read/write permissions.
- `demo.mec` — allowed program that reads a DOM value, performs string logic in Mech, and writes DOM output.
- `denied.mec` — program that attempts a denied DOM write.

## Running

Build or copy the Mech WASM package into `examples/browser-dom-demo/pkg/` so the page can import:

```text
./pkg/mech_wasm.js
./pkg/mech_wasm_bg.wasm
```

Then serve this directory with any static file server. For example:

```text
cd examples/browser-dom-demo
python3 -m http.server 8080
```

Open `http://localhost:8080/` in a browser.

Change the `Source DOM input` field, click `Run round trip`, and observe that Mech reads the DOM value, computes new strings, and writes the computed result back into the page.
