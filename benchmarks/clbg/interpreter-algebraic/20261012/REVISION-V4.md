This unqueued additive draft repairs the opt-in conformance module bridge.
The compile-input-v3 phase successfully built the strict production executable
and retained actual LLVM bitcode/IR and ARM assembly, then stopped with E0433:
`resident` is a private child of `sequence::runtime`, not `sequence` directly.
No conformance execution, CLI case or algebraic variant ran. Every source,
proposal, compiler log, strict binary and failed report remains unchanged.

`guarded_prepare_v4.py` adds a feature-gated `pub(super)` forwarding function
inside the copied runtime module. The public sequence forwarding function
calls that runtime bridge, which can access its private resident child. This
matches the actual module nesting and changes no default production path,
reduction policy, coefficient tape, flags or conformance cases. All three
copied workspaces and targets are fresh in `revisions/module-bridge-v4`.

The metadata phase references the exact five successful v3 include/snapshot
checks without rerunning them; it requires AST equality of every helper except
seal/prepare. It verifies and pins the v3 proposal, successful strict binary,
retained codegen and failed conformance diagnostics as separate lineage. The
old strict executable is not assigned any native/output/performance proof:
there were zero actual native cases. New builds and all33 CLI cases remain
fresh; actual conformance compilation/execution must establish the bridge.

No v4 metadata seal, compiler or runtime has launched. Root reviews this
concrete draft, then its emitted exact proposal before a bounded preparation.
No full samples, performance gain or score admission are claimed.
