# Analog clock demo

The CLI and browser demos share the same platform-neutral Mech model in
`examples/analog-clock/clock.mec`. The model reads time values from a `time`
host and exposes clock-angle symbols; it does not contain CLI, DOM, or SVG
output operations.

Run the native CLI demo from the repository:

```bash
cargo run --bin mech -- run examples/analog-clock
```

Or, with an installed executable:

```bash
mech run examples/analog-clock
```

Build the browser demo:

```bash
bash scripts/build-analog-clock-web.sh
```

The build script generates these local files, which are ignored by Git:

```text
examples/analog-clock/pkg/mech_wasm.js
examples/analog-clock/pkg/mech_wasm_bg.wasm
```

Serve the repository with Python:

```bash
python3 -m http.server 8000
```

Open the demo at:

```text
http://localhost:8000/examples/analog-clock/
```

Do not open the page via `file://`; browser ES modules and WASM loading require
an HTTP origin.
