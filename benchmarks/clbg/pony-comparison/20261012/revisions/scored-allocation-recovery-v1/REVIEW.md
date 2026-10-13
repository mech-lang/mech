# Pony scored allocator evidence recovery proposal

Source-only proposal. No recovery import, hash, fixture, compiler, disassembler,
binary execution, freeze or launch has run. C3/website publication keeps priority
over every substantial receipt/IR read and diagnostic command. All substantial
work requires the existing common timing mutex. The failed work-v5 report and
both completed actual tree observations are immutable historical evidence.

The failed owned job is
`continuation/owned-jobs/pony-two-tree-reviewed-next-stage-v2`. Its native worker
first retained the four previously completed Pi/map traces, then recorded the
two new actual binary-tree observations and mutation rejections. The original
top-level report remains stopped; it must never be changed to passed, resumed,
or have those two observations rerun. Every original trace/build/image/source/
binary/environment/proof and terminal pin must be verified under common before
any new metadata projection can reuse them.

The failure is in frozen work-v5 `allocation_evidence.py`: after an owned
successful compact diagnostic IR compile, it expects exactly one `.ll` file.
The saved command instead emitted 16 files: one surviving `.main.ll` and 15
package modules. Its stderr writes the main filename twice; the surviving
header identifies `compact.builtin`, rather than the authored compact module.
No file can be selected by glob position or filename alone.

The exact saved compiler version is 0.74.0-95ad180, LLVM 23.1.2. Its primary
[IR emitter](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/genobj.c)
uses the same main suffix for every null package symbol. The saved duplicate
write plus surviving builtin header therefore indicate that the original
authored module was overwritten. The
[bitcode emitter](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/gensplit.cc)
distinguishes the entry package by index and later unnamed packages by their
package filename. The
[compiler route](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/genexe.cc)
emits bitcode before returning at that pass, and
[pass names](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/pass/pass.c)
name this option `--pass bitcode`. This is a diagnostic extraction route; the
scored compiler, sources, executable images, flags and timings stay unchanged.

The first released recovery step must only capture and pin the failed durable
closure, manifest, qualification, actual six trace records and complete 16-file
partial artifact inventory under common. It must retain the exact duplicate
write sequence and each module's header/defined-symbol catalogue, with no
automatic choice or allocator conclusion. That produces an additive ownership
review packet, never a replacement failed report or an admitted work verdict.

Only after the actual ownership/pin packet and concrete source recovery worker
are independently reviewed can missing scored evidence be extracted:

- Reuse the complete existing compact scored-binary ARM disassembly after its
  original owned receipt and binary/source/production-command pins verify.
- Emit fresh diagnostic bitcode for both original scored tree source packages,
  with the exact original production compile command except the exclusive
  output path and explicit stopping pass. Preserve every emitted package file
  and every actual compiler output record; reject missing, duplicate or unknown
  module outputs and incomplete compiler/source ownership bindings.
- Use a separately pinned and actually qualified LLVM23 disassembler to decode
  the unambiguous authored entry bitcode. The currently available candidate is
  `/opt/homebrew/Cellar/llvm/23.1.3_1/bin/llvm-dis`; its bytes, version and loaded
  image closure are not yet qualified. That current reader is separate from the
  original Pony compiler's recorded LLVM23.1.2 backend; no retrospective reader
  pin or same-image claim is allowed.
- Capture only the missing optimized scored-binary ARM disassembly. These are
  compiler/disassembler/otool evidence commands, with owned bounded lifecycles,
  zero tree/application executions and zero full timing samples.

The authored module must be selected from actual recorded package/output
lineage and source-matched defined Node/Worker/Main symbols, with corresponding
original scored ARM symbols. Retain all vendor modules and reject ambiguity,
overwritten output, missing ownership or a different source/target/compiler.
Neither a builtin header nor mere presence of an allocation name is sufficient.

New raw evidence keeps `leaf_path_allocates_fresh_node`,
`interior_path_allocates_fresh_node`, `no_static_or_shared_tree_graph` and
`long_root_storage_retained_through_worker_joins` pending until independent
source/ARM/IR semantic review. No boolean conclusion is synthesized from a
trace copy, artifact filename, prepared status or successful compiler exit.
The scored binaries must always be the unchanged original images, never the
diagnostic executable or observer copy.

Every output and recovery receipt is a new exclusive revision. Original failed
report, stopped terminal, 16 IR artifacts, six trace records, source and timing
bytes remain unchanged. All early/late failures preserve their raw error and
readable partial evidence pins while common is held. Native fixture or tree
reexecution, automatic retry, blanket ambiguity waiver, benchmark timing,
source mutation and chart admission remain outside this recovery scope.
