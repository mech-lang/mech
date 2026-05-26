# Runtime module smoke example

`main.mec` imports `math.mec`.

`math.mec` defines and exports `tau`.

`main.mec` uses `tau` after the runtime resolves, builds, and runs dependency modules first.

The automated runtime test verifies this example shape.
