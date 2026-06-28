# Robot-arm examples

These examples belong to the `mech-host-robot-arm` package. They are not part of the generic top-level Mech examples because the generic runtime, browser host, and wasm package do not depend on the robot-arm host.

The examples require a composition build or runner that registers `RobotArmHostFactory` alongside any other required host factories.

- `robot-arm-demo/` demonstrates the robot-arm host directly.
- `browser-robot-arm-demo/` demonstrates a browser UI plus a robot-arm host. It requires an explicit composition layer that registers both browser and robot-arm hosts.
