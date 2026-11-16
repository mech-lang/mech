# Shared synchronous phase-call storage (R229)

Retain the four-file compiler experiment over R228. Sequential calls share two
invocation-local, disjoint argument/result slots. Every call still supplies fresh
inputs and reads current results before continuing. Existing owner/geometry/
capacity guards, fallback, source checks, charges, aliases, errors and recovery
remain. No cached argument, ownership permission or packet contents.

All25 unchanged gates pass; geometric balanced block means improve **3.136%**.
Ratios use matched per-call medians; changes use balanced block means.

| Input | R228 / Roc | R229 / Roc | Block-mean change |
|---|---:|---:|---:|
| reymont-prefix-32769 | 2.344085× | 2.279283× | -2.764% |
| 33027-periodic-window | 2.157944× | 2.054242× | -4.806% |
| repeated-32769 | 2.020157× | 1.933865× | -4.272% |
| random-32769 | 3.006009× | 2.837767× | -5.597% |
| 16384-distribution-change | 2.956251× | 2.805722× | -5.092% |
| eight-literals | 0.515418× | 0.500720× | -2.534% |
| reymont-prefix-16384 | 2.320142× | 2.251860× | -2.943% |

Instruction/cycle medians fall onall25. Random32 instructions fall21.086%
(62.910m→49.645m); natural32 falls10.453% (153.124m→137.118m). Heap requests,
requested bytes, reallocations, incremental usable peaks and all payload/copy/
zero metrics match onall25. Preserve stored-only adverse block means:one-stored
+2.734%,seven-stored-prefix +3.540%. No universal elapsed win is claimed.

## Why the generated entry shrinks

134 phases previously reserved268 argument/result slots,total32168B. The shared
pair needs264B input/248B output. Root stack falls86736→55088B (−31648B).
Root text falls6781056→5649872B (−1131184B,16.68%); whole worker falls
7762616→6623288B (−14.68%). All134 numeric/two map bodies AND metadata remain
byte-identical; native-call count stays50972. One executable handles all25 sizes.

The smaller frame changes ARM64 addressing. Static stack memory instructions
stay441725.147207 memory accesses and238 address adds switch away from register
offsets, accounting for147445 removed `mov` instructions;130541 `movk` and4810
unconditional branches also disappear. These three opcode deltas account exactly
for the1131184B text saving. Static disassembly, including padding/pools, is kept
separate from actual dynamic whole-call counters. The entry is still large;
source functions have not been generally shared or outlined by this change.

## Lifetime proof and regression evidence

Only `compact/emit.rs`,`compact/mod.rs`,`compact/phases.rs` and new
`compact/phase_storage.rs` change. Complete synchronous phase selection excludes
unknown effects, provider boundaries and checked-child callbacks/source exits.
Current inputs are stored after the original gates. Success writes every lane
that its caller reads; failure stores its current origin before reading it.
The first output word is error-only and unused on success. No blanket zero fill.
Results become caller SSA values and ownership settles before the next call.
Array helpers retain backing owners, never packet pointers. Reentrant invocations
and numeric callees use separate frames. Checked extents remain representable.

684 maintained tests pass under the six-feature warnings-as-errors profile,
including26 traversal tests; the same seven inherited exclusions stay explicit.
3800 compiled and2964 ordinary-native controls match parent/candidate and
original/decoded values,errors,origins,work,aliases,finite clone denial,prior
publication,cleanup and retry. All final managed bytes/nodes/private payload
are zero.38 small original/decoded Object/metadata pairs match within each side.

Two new source witnesses cover both packet-size orders,scalar+array results,
changing arguments,recursive visits,unselected paths,late-phase rejection and
immediate recovery. An earlier scalar result stays live across a later smaller
call. Both phase headers actually select in41/50 cases per witness. The first
witness frame falls688→496B. The initial wrong record-type spelling is rejected
by the frozen parent before emission; fixing the test uses existing syntax,
with no parser/compiler gap or language change.

162222 primary calls,3900 separate CPU/heap calls,50 immediate stream smokes and
fresh24514 wall/102369 actual-PC calls verify all25 exact streams,inflation,
held aliases,original capacities and cleanup through131073B. Fixedlevel12,
runtime inputs,200ms warmup,balanced ordering,allocator,clock and full compression/
scratch/transfer/cleanup boundaries remain matched. Stored-only quantization and
all contrary observations stay in the packet. Samples are not instruction shares.

The experiment still omits the ordinary extra exact resource/admission/
accounting/publication/recovery contract; normalIR215 source is unchanged. R225
source bindings stay separate. No LLVM, source algorithm/allocation-site change,
new branch/PR,multi-MB run,fresh ordinary full-compressor timing or parity claim.

## Pins and evidence

Base `3f140950e58a870dd4a0fcab08239395976a491d`; normal `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5` (IR/work/ABI/MBC215/107/39/14). Compiler manifest `fbb7b0ff9c373d173249dce422576d81a60bf32a26bdd5164118053f556c44cd`; Object `240c00f0a1cc298edef2fe32e7074f0b575c9c52b0145b3161232bab70590d6b`. Source/MBC `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` / `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.

[Full source,minimal patch,lifetime proof,all25 timings/counters,current profiles and regression receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r229-shared-phase-packets-b19ae0d44152.tar.gz) — 4688123B,2248 hashed files,SHA-256 `b19ae0d44152f632438ac3a9c028a3cf15e5c62375862c0d97f33022a1430abf`. Local/public hash verification and numerical replay are recorded in the publication receipt. Native binaries,bytecode,full metadata and raw traces are excluded.

Next: separately measure wider matching and inventory remaining caller scratch/address construction with a real source witness. Explicit all25 combined measurement is required before adding R225. Full parity remains open.
