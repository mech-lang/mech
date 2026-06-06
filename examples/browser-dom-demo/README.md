# Browser DOM resource demo

This example demonstrates browser DOM resources backed by runtime resource providers.

It shows:

- `WasmMech.fromConfig(...)` loading browser DOM grants from `demo.mcfg`.
- Mech code binding `@browser := browser://dom/`.
- A DOM read from `body/content/mech-sandbox/input/_value@browser`.
- DOM writes to `body/content/mech-sandbox/title@browser`, `body/content/mech-sandbox/status@browser`, and `body/content/mech-sandbox/status/_class@browser`.
- A denied write from `denied.mec` because the config grants read, but not write, on the input value.

## Files

- `index.html` — browser page that loads the WASM package and runs the demo.
- `demo.mcfg` — runtime/browser config and DOM permissions.
- `demo.mec` — allowed program that reads an input value and writes DOM output.
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
