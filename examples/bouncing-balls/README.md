# Mech Bouncing Balls

Run natively:

```bash
mech run examples/bouncing-balls
```

Serve in the browser:

```bash
mech serve examples/bouncing-balls
```

The timer host emits fixed 120 Hz simulation packets. The scene host stores the latest completed scene and the browser presents it from the shared requestAnimationFrame project loop. Native scene execution is headless.
