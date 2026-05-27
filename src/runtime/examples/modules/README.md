# Runtime module smoke example

`main.mec` imports `math.mec` as module namespace `math`.

`math.mec` defines and exports `tau`.

`main.mec` accesses the exported value as `math/tau`.

Dependency/file imports do not mean wildcard import.
