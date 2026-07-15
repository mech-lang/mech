# Analog clock demo

The native and browser demos use the same Mech project and configuration:

```text
examples/analog-clock/mech.mcfg
examples/analog-clock/clock.mec
```

`clock.mec` reads the wall-clock `time` host, computes the clock values and SVG
presentation scene, sends textual output to `console/output`, and sends the
scene to `scene/frame`. Native `mech run` maps the console host to stdout and
uses a headless scene host. The browser maps the same console output to
`console.log` and renders the same scene data into the SVG target.

Run the native CLI demo from the repository:

```bash
cargo run --bin mech -- run examples/analog-clock
```

Or, with an installed executable:

```bash
mech run examples/analog-clock
```

Build the shared browser package:

```bash
bash scripts/build-mech-browser.sh
```

The build script generates shared files ignored by Git:

```text
examples/pkg/mech_wasm.js
examples/pkg/mech_wasm_bg.wasm
```

Serve the repository with Python:

```bash
python3 -m http.server 8000
```

Open the demo at:

```text
http://localhost:8000/examples/analog-clock/
```

Open the browser developer console to observe the same clock output emitted by
`clock.mec`. SVG rendering is an additional browser presentation of the shared
Mech scene; it is not a separate Mech program and there is no example-specific
JavaScript.

Do not open the page via `file://`; browser ES modules and WASM loading require
an HTTP origin.
