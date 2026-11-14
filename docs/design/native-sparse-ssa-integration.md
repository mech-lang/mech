# Restore sparse SSA storage for large native programs

The merged compiler at `a278be55aca7d6b50cc1e73fe4a6178ccb1f53ff` selected registry Cranelift frontend 0.131.3. Its SSA builder grows a dense block-indexed vector for each variable and caches definitions in every intervening single-predecessor block. The complete level-12 compressor exposed this during construction of its 41,039-block source CFG.

The dense emitter ran 661.53 seconds, reached source-site checkpoint 28,672, and exited with signal 9 before writing an Object. The kernel memory-pressure log reports 40,045 MB for the killed process. The separate interval allocator trial also selected the dense frontend and was stopped before its root emission.

The workspace now selects a vendored copy of the same frontend revision with the retained sparse map change. Each variable records actual block definitions and caches only the queried block. The existing predecessor traversal, phi insertion, undefined handling and source checks remain in the same pipeline.

The single-emitter retry completed original Object generation in 614.47 seconds. The OS timing wrapper could not read its kernel clock counter after the Object was written. The linked executable independently validates the emitted Object. The retry provides elapsed time and no fresh peak-RSS receipt.

Twelve SSA tests pass, including a new long-chain check that retains only the entry and queried blocks. Four probes using the same Cranelift backend verify byte-identical IR. The 2,049-variable probe reduces tracked peak requested allocations from 9,241,188 to 1,299,084 bytes. The full compressor matcher proof, pre-allocation IR and allocated VCode match the dense attempt byte for byte.

The warnings-as-errors build uses nightly-2026-03-03, release, offline and locked dependencies. The linked original artifact passes two exact stream calls on each of the 81 fixtures with independent inflation, retained capacities, input and output aliases and final ownership cleanup. The producer checks the canonical bytecode decode and encode round trip. The independent decoded Object and elapsed benchmark are pending.

Compressor source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. Source-derived bytecode SHA-256 `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`. Original Object SHA-256 `ae0fea7ba276a55a04e12c2f2578e8bb743a4bfaf8503f09c522ac640d608aa7`. Linked worker SHA-256 `f1c709468b15559887cb1896acca8d1ae56db2016cb50fd04a0d85f6442c35d0`.

[Verified compiler receipts and linked checks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/sparse-ssa-evidence-f197994270b6.tar.gz). SHA-256 `f197994270b6d70b1d03bda44c8db24a39c9b6ade030e94aaa515fecfc435fb2`.
