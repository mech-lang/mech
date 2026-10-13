# Pony runtime-merged target diagnosis and remaining evidence plan

This additive plan and its three-file executable source packet are source-only,
mutable and unexecuted. They change
no existing worker, failed report, emitted bitcode, decoded IR, scored binary or
trace. It grants no execution and does not relax the old failed literal check.
Root and an independent peer must review the concrete continuation worker and
exact launch separately.

The actual v2 recovery ended normally with exit1 at
2026-10-13T22:53:42.940422Z, after exactly two successful owned commands: compact
production-argv bitcode stop and compact entry decode. Root's closed review
`continuation/jobs/pony-scored-recovery-v2-closed-review/receipt.json` verified
10,142 pins and preserves the failure. Its body terminal SHA is
0f3248e5a2f135906c6f66588e989fcf567a88954c407fa1842fed1a0a0dcb39;
durable terminal SHA17904a3e495a5b6c328b25679d14d2ff75c464bdbd50b555b2984d2ff7eb46b8.
Every digest in PLAN.json is copied from those actual records, not computed
during this source review. Root must capture the closed-review digest later.

The retained entry header is exactly `arm64-apple-macosx26.0.0`. Its source
filename names the distinct emitted `.main.bc`, and it defines authored
Node/Worker/Main methods. The original overwritten `.main.ll` names
`compact.builtin` and has the requested `aarch64-apple-macosx14.4.0` triple.
Both actual commands retain the exact same compiler image, requested triple,
14.4 SDK and source package as production; the diagnostic changes only output
and the stop pass. This is not a decoder rewriting target metadata.

The pinned compiler identifies itself as Pony0.74.0-95ad180 compiled with
LLVM23.1.2 and AppleClang21 ARM64. In the matching official compiler source,
[`codegen.c`](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/codegen.c)
initializes each module from the requested triple, then links `libponyrt.bc`
into the current main module. Its later target-attribute stamping sets CPU and
feature attributes, without restoring that module triple. In
[`genexe.cc`](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/genexe.cc)
this occurs before either diagnostic stop and the production bitcode/link path.
[`gensplit.cc`](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/gensplit.cc)
then writes each unchanged module and distinct main/builtin filename. The
[`package.c`](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/pkg/package.c)
installation search puts the compiler's own `../lib/<arch>` before custom
`--path` roots. The actual v6 installation contains one `libponyrt.bc`, under
`lib/armv8-a`, with original pinned SHA95e60711be95591e4b5f5e3b35e7daef1c28de30fb8bb5b46c74d89d2309438a.

The matching official LLVM23.1.2
[`IRMover.cpp`](https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-23.1.2/llvm/lib/Linker/IRMover.cpp)
sets the linked destination triple from `SrcTriple.merge(DstTriple)`.
[`Triple.cpp`](https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-23.1.2/llvm/lib/TargetParser/Triple.cpp)
selects the Apple triple with the larger OS version, retaining its spelling.
This provides a precise explanation for a runtime26 main and untouched14.4
other packages. It is still a source-supported inference about the runtime
input header: that pinned vendor bitcode has not yet been decoded in the
retained evidence. The first proposed command must establish its actual26
header before the new gate can accept the main26 header. Another runtime
triple, image, source or input set rejects; no arbitrary triple equivalence or
header rewriting is allowed.

The original scored compact production stderr independently records
`Merging runtime`, the same17 bitcode partitions and final linking. Its pinned
native image receipt actually reports `arm64`. The original scored
`otool -tvV` output contains ARM64 instructions and matching Node/Worker/Main
symbols, including recursive `_Node_ref_create_pZo` and allocation calls.
Thus this diagnostic stage follows the same production merge/partition route
and agrees with the scored architecture. A14.4 command does not independently
prove every linked runtime function supports macOS14.4. The linker source
derives its explicit Mach platform argument from the original requested triple;
the runtime26 module is nevertheless part of its bitcode input. No new minimum
OS compatibility or full native allocation verdict is claimed here. The
existing complete ARM, final ThinLTO effects and the actual merged IR still
need independent semantic review before source-work qualification.

The next direct continuation uses the existing owned command service and
qualified LLVM23 reader. It holds common inode79859134 and all original closed
owner leases continuously, including the new failed v2 owner. Before any
command it verifies root's actual closed review, exact failed request/launch/
worker-started/command-started/terminal identity and normal1/zero stops, all
10,142 actual expected and available pins, the original successful two native
receipts, original reader image/receipt/terminal/lease and full original
qualification/capture/six-trace/compact ARM closure. It must cold-load verified
raw source from real module paths; no cached-module rule or identity is forged.

PLAN.json fixes six missing command steps: one vendor runtime decode; compact
builtin decode from existing bitcode; optimized exact-production bitcode stop;
optimized main and builtin decodes; original optimized scored ARM capture.
The compact compile and entry decode are not repeated. No applications,
fixtures, reader qualification or six passed traces rerun. The source package
and all original pinned library input sets remain exact. Each stage uses fresh
output, owned native/final0, fully drained bytes and clean group/reap/cleanup.
The compact17 surviving bitcodes and original entry IR remain in their old
directory. Final new evidence only references them with unchanged hashes.

The new result must disclose both requested and actual runtime-merged triples,
record every command and source image, preserve the failures and carry null
allocation/retention verdicts, source_work_qualified=false, LOC/chart false.
All original and newly available pins are rechecked before the final receipt,
with partial failure journals retained. This plan is not a native run or an
implicit retry. No parser, decoder, compiler, hashing, benchmark, fixture,
freeze, signal or Git command ran during authorship.

`continue.py` implements this exact six-command plan using the frozen v2
utility functions by literal worker/review digests. It never calls that old
worker's `main`, `recover` or `reader`, so neither completed compact command nor
reader qualification can repeat. It consumes the actual failed root review,
body and durable terminal with exact PID33585/33589 command identity and all
actual pins. The original capture/reader CLI values are fixed to their actual
completed values. All four closed private leases stay held with common until
the new exclusive terminal, including readable failure artifacts.

Before accepting the retained authored entry it requires the vendor decode's
actual target to be exactly arm64-apple-macosx26.0.0 and a real runtime allocator
definition, with no authored Node/Worker/Main definition. The current earlier
compiler-bin runtime shadow must be absent. The pinned requested14.4 argv and
SDK stay exact; all original Pony input filename sets remain checked. Both
compact native receipts are actual0 with clean ownership and retained hashes.
Each17-module write inventory is exact; authored versus builtin filenames,
headers, data layout and symbols retain the narrow checks. Final evidence
reports both triples and still requires final native semantic review.

Freeze/capture all three files and the actual root10,142-pin closed review;
then review this one fresh durable command before root release:

```text
<bundled-python3.12> -I -S -B -u ABSOLUTE/runtime-target-continuation-v1/continue.py
  --worker-sha256 ACTUAL_WORKER_SHA --review-sha256 ACTUAL_NOTE_SHA
  --plan-sha256 ACTUAL_PLAN_SHA --closed-review-sha256 ACTUAL_ROOT_CLOSED_REVIEW_SHA
```

The exclusive output is fixed to `jobs/remaining-scored-evidence-v1`. There is
no resume/overwrite option, benchmark run, qualification shortcut, data write,
reader retry or fixture execution. This source was not executed or hashed.
