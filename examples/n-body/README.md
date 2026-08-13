# Resident solar-system orbit viewer

Serve the browser visualization with one command:

```bash
mech serve examples/n-body
```

The project configuration selects the resident runtime, starts its 60 Hz timer,
and publishes one dense ten-body positions matrix to the SVG scene for every
accepted turn. The numerical source preserves the original fixed-Sun Keplerian
orbit equations, lowered to one resident numeric graph with no legacy turns.
