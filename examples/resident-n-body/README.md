# Resident Mech N-Body

This project runs the frozen ten-body recurrence through the production
resident runtime. A 60 Hz timer supplies the deterministic step input and one
dense `N × 3` matrix is published to the SVG scene after each accepted turn.

Run it natively with:

```text
mech run examples/resident-n-body --resident --runtime-info --max-live-turns 120
```

Or serve the directory to run the same source through `WasmProject` in a
browser.
