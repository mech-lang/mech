# Analog clock demo

The CLI and browser demos share the same platform-neutral Mech model in
`examples/analog-clock/clock.mec`. The model reads time values from a `time`
host and exposes clock-angle symbols; it does not contain CLI, DOM, or SVG
output operations.

Run the native CLI demo:

```bash
cargo run \
  --bin mech-clock \
  --features clock_demo_cli
```

Finite mode:

```bash
cargo run \
  --bin mech-clock \
  --features clock_demo_cli \
  -- \
  --ticks 10
```

Build the browser demo:

```bash
wasm-pack build \
  src/wasm \
  --target web \
  -- \
  --features analog_clock_demo
```

Serve the repository with an HTTP server and open
`examples/analog-clock/index.html` from that server. Do not open the page via
`file://`; browser ES modules and WASM loading require an HTTP origin.
