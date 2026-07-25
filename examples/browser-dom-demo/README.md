# Browser DOM static bundle demo

This project is the executable static-web example for `mech bundle-web`. It demonstrates a configured browser DOM host, scoped read/write grants, a successful DOM-to-Mech-to-DOM round trip, and a denied write.

Build the WASM package, then create and serve the static bundle:

```sh
cd src/wasm
wasm-pack build --target web
cd ../..
mech bundle-web examples/browser-dom-demo --out dist/browser-dom-demo
python -m http.server 9000 -d dist/browser-dom-demo
```

Open `http://127.0.0.1:9000/` and use the two buttons to run the allowed and denied programs.

The configuration supplies the `serve.paths`, `serve.shim`, and `serve.wasm` values required by `bundle-web`. Its shim deliberately uses relative `./pkg/...` and `./code/...` URLs so it works from a plain static web server.

For the maintained native/browser runtime example, see `examples/analog-clock/` and run either `mech run examples/analog-clock` or `mech serve examples/analog-clock`.
