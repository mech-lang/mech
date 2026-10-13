The first preparation stopped in the strict compiler on a missing compile-time
input, `tests/fixtures/resident-ekf/ekf-input-v1.bin`. Its original source pins
covered Rust and Cargo files but omitted this binary include. The compiler
returned101 before any conformance or actual CLI case. Original proposal,
source copies, partial target, build logs and failed report remain unchanged.

This additive revision changes snapshot inputs and output locations only.
`guarded_prepare_v2.py` retains every literal `include_str!`/`include_bytes!`
target from the copied dependency source roots, including test-only inclusions
present in those files. It also recognizes the existing package-relative
`concat!(env!("CARGO_MANIFEST_DIR"), literal)` form. A missing, escaping or
unrecognized inclusion fails metadata before any compiler. This is an explicit
source-root inventory for the integration driver's local dependencies; it
does not claim a general Rust macro resolver or copy unrelated CLI/wasm builds.

The metadata proposal records every inclusion site and hashes every selected
input under the common mutex. Snapshotting merges these inputs with all prior
1490 source pins and verifies each copied byte before builds. Five temporary
file checks cover literal binary/text paths, package-relative paths, failure
boundaries, exact binary snapshot bytes and unchanged prior helper functions.
They execute once through the existing owned child guard with inherited mutex.

The three source variants, coefficient tape, conformance matrix, eleven actual
CLI sizes, native ownership, compiler/SDK/library pins, codegen flags and
pre-execution conformance binary pins are unchanged. There is no prior binary
or output proof to reuse. Every build and all33 CLI probes are fresh in
`revisions/compile-input-v2/{.build,preparation}`. The additive proposal is
`revisions/compile-input-v2/proposal.json`; old preparation is never resumed.

No v2 metadata fixture, seal, compiler or probe has run. Root reviews this
concrete repair before one metadata phase, and the emitted proposal digest
before one bounded preparation. No full performance sample or chart admission
is authorized.
