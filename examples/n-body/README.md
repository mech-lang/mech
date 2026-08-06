# Solar-system orbit viewer

Run natively:

```bash
mech run examples/n-body
```

Serve the browser visualization:

```bash
mech serve examples/n-body
```

The native and browser paths load the same `mech.mcfg` and `n-body.mec`.
A timer host advances a ten-body circular-orbit model at 15 turns per second.
The scene host rotates each planet around its SVG orbit, while the console host
prints simulation time, Earth and Jupiter phase, total angular momentum, and
total energy.

This configured example is an orbital visualization, not yet the pairwise
n-body integrator. The current configured native runner does not register the
`state/commit` primitive needed to retain the integrator's positions and
velocities between timer turns. Keeping that distinction explicit prevents the
closed-form display model from being mistaken for a numerical n-body solve.

The orbital model uses astronomical units and years. The browser uses a
square-root radial projection only for display, allowing Mercury and Pluto to
share one legible view. The static SVG orbit guides use the same projection.
