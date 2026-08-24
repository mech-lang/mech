# Browser DOM static bundle demo

This project is the executable static-web example for `mech bundle-web`. It demonstrates a configured browser DOM host, scoped read/write grants, and a DOM-to-Mech-to-DOM round trip.

Build the WASM package, then create and serve the static bundle:

```sh
python3 scripts/build-wasm.py --profile browser
mech bundle-web examples/browser-dom-demo --out dist/browser-dom-demo
python -m http.server 9000 -d dist/browser-dom-demo
```

Open `http://127.0.0.1:9000/` to see the configured project read `Ada` from the page and write the greeting, output, and status back to the DOM.

The configuration supplies `run.paths`, `serve.paths`, `serve.shim`, and `serve.wasm`. `bundle-web` writes the project config, source manifest, and a relative `./_mech/project.js` bootstrap so the bundle runs from a plain static web server.

For the maintained native/browser runtime example, see `examples/analog-clock/` and run either `mech run examples/analog-clock` or `mech serve examples/analog-clock`.
