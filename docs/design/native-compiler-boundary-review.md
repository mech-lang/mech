# Checked Program compilation boundary review

Baseline F is `3a66a014c853e082ad907ed4db1673c9dadf0c69`. The tested and measured
candidate C is `79e500d0ace0388c7de1e31f7918e80bb90252fe`.
[Exact M1 commands, outcomes and raw logs](https://github.com/mech-lang/benchmarks/blob/8c0fef89bac508e7cc7508c9255e2f90f5bb21f6/experiments/mech/mechdeflate-cranelift-v1/stabilization/route-memory/79e500d0ace0388c7de1e31f7918e80bb90252fe/checked-compilation-boundary-m1/validation.md)

`LoweredProgramCompilation` owns the checked plan and borrows its exact
artifact, catalog and captured inputs. Its plan accessor is read-only.
Preparation consumes the owner and uses the existing prepared representation.
Each phase converts its error at the ownership boundary.

| Compiler path | Preserved sequence |
| --- | --- |
| `JitProgramArtifact::compile_with_options` | Host check and capture, lowering, JIT module creation, pointer and count preflight, preparation, emission and finalization. |
| `emit_program_object_with_options` | Host check and capture, lowering, Object module creation, pointer and count preflight, preparation, emission and finalization. |
| `compact_native_layout_for_diagnostic` | Capture, lowering and plan inventory. The proof-inventory setting selects preparation. |
| `compile_compact` | Host check, measured `native_plan` and `prepare` stages, compact flag validation, then a fresh Object module for each emission attempt. |
| `emit_minimal_numeric_object_for_diagnostic` | Host check and capture, lowering with the existing definition-table allowance, preparation, then module creation and emission. |

The compact direct-literal attempt and its retry borrow the same prepared
Program and the coordinator's one profile cache, including cached read failures.
The shared owner is the production caller of `prepare_with_inputs`.
Runtime reconstruction continues through its own `prepare()` interface.

The independent-producer test executed original and decoded artifacts from
source and from a direct draft. Each JIT produced 21, reported the correct
addition or division error, retained the previous output and recovered its
accounted memory. The same JIT then produced 2. Object checks inspected the
descriptors and typed interfaces. The public k-nucleotide check exercised
actual linked source and bytecode builds. The fixed behavior and feature checks
passed on F and C, and the coordinator's full-encoder test remained ignored.
[Independent-producer check](https://github.com/mech-lang/benchmarks/blob/8c0fef89bac508e7cc7508c9255e2f90f5bb21f6/experiments/mech/mechdeflate-cranelift-v1/stabilization/route-memory/79e500d0ace0388c7de1e31f7918e80bb90252fe/checked-compilation-boundary-m1/candidate/independent-producer.log)
· [Linked execution](https://github.com/mech-lang/benchmarks/blob/8c0fef89bac508e7cc7508c9255e2f90f5bb21f6/experiments/mech/mechdeflate-cranelift-v1/stabilization/route-memory/79e500d0ace0388c7de1e31f7918e80bb90252fe/checked-compilation-boundary-m1/candidate/public-linked-knucleotide.log)

The compact M1 rebuild reproduced `program.o`, `metadata.json`, and
`program.mecb` byte for byte. All 26 runtime-support tests passed.
The four compiler-cost pairs had a median C minus F difference of
**+4.854 ms** elapsed and **+96 KiB** process peak RSS.
[Artifact hashes and sizes](https://github.com/mech-lang/benchmarks/blob/8c0fef89bac508e7cc7508c9255e2f90f5bb21f6/experiments/mech/mechdeflate-cranelift-v1/stabilization/route-memory/79e500d0ace0388c7de1e31f7918e80bb90252fe/checked-compilation-boundary-m1/artifact-preservation.json)
· [Four pairs and warmups](https://github.com/mech-lang/benchmarks/blob/8c0fef89bac508e7cc7508c9255e2f90f5bb21f6/experiments/mech/mechdeflate-cranelift-v1/stabilization/route-memory/79e500d0ace0388c7de1e31f7918e80bb90252fe/checked-compilation-boundary-m1/compiler-cost/compiler-cost.json)

The direct M1 report, original tooling and offline replay are published in
the benchmarks repository. Replay matched all 1,116 case rows and all
three recorded individual-leg exceptions.
[Direct comparison and replay](https://github.com/mech-lang/benchmarks/blob/c5815aaf65cd66440ab2a6abc4862b90cd62129d/experiments/mech/mechdeflate-cranelift-v1/stabilization/route-memory/README.md#direct-m1-comparison)
