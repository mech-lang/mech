# Analog clock demo

Run natively:

```bash
mech run examples/analog-clock
```

Serve in the browser:

```bash
mech serve examples/analog-clock
```

The native and browser paths load the same `mech.mcfg` and `clock.mec`.
`clock.mec` reads the wall-clock time host, computes the clock values and SVG
scene, emits console output, and sends the scene to the generic scene host.

Native scene execution is headless. Browser presentation is driven by the shared
`/_mech/project.js` bootstrap and requestAnimationFrame; there is no
example-specific JavaScript. The scene host coalesces presentation to the latest
completed scene while all application state and calculations remain in Mech.
