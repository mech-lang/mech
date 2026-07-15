# Analog clock demo

The native and browser demos share the same Mech project:

```text
examples/analog-clock/mech.mcfg
examples/analog-clock/clock.mec
```

`clock.mec` reads the configured `time` host, computes the clock values, and
sends the same `clock-output` value to `console/output`. The native runtime maps
that console host to process stdout; the browser runtime maps it to
`console.log`.

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

Open the browser developer console to observe the same clock output emitted by
`clock.mec`. The SVG clock hands are an additional browser presentation that
reads the computed angle symbols; they are not a separate Mech program.

Do not open the page via `file://`; browser ES modules and WASM loading require
an HTTP origin.
