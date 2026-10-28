# Ordinary function argument frames retain packed buffers

The linear stack continues with #942 above #941. Ordinary recursive argument
transport now retains existing immutable buffer owners, and the real U64 path
and U8 output reservations survive the saved recursive acceptance. This is
compiler progress toward the benchmark; there is still no complete compressor
or compression timing result.

## Exact revisions

Base: `851507304181308ddbadcd96bec24859e8deeed4` (#941).
First implementation: `356e7275cc7e852aeea9713a4f533e8540983b9d`.
Final validated code: `4b722986c7eaa47011c4e36562baec0e1fe87cf3`.
The following packet commit changes evidence and documentation only.

Pinned source: `671077457ff7ef3a3d44caf715a3fc595addf379`.
Pinned reference compiler: `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`.
Native IR / ABI / work policy: **109 / 35 / 88**. Executable bytecode remains
revision **14**. The work policy invalidates older native identities after the
new admission behavior; portable program semantics and codec are unchanged.

`356e7275c` adds the private frame layout, reference transport, current-site
helpers, tests and linked case. `4b722986c` replaces an inadequate arity-only
shape-binding allowance with the core constructor's existing allocation,
node and work witness, adding a red/green quota regression.

## Design and compressor requirement

The parser passes the **262144-entry U16 tree and 1500513-word U32 cache** through
ordinary calls. Its node-building path also passes a growing U64 list after
reserving **305257 entries**. The writer needs **6633832 U8 slots** retained
across repetitions. Canonical tuple construction previously cloned each packed
Vec; cloning dropped spare capacity and allocated full initialized payloads.

A same-plan derived layout selects an entire nonescaping tuple component with
packed unsigned buffers and native scalar leaves. All definitions and reads
must be understood pack/copy/projection uses. Input/output/effect escape or
unsupported shapes retain the canonical route. Activation shape histories also
retain their original bound-factory owner; selected geometry is fixed or Turn.
The layout is not serialized authority: independent descriptor loading rederives
it from the validated plan.

The existing compiled CFG and helper visit protocol remain in charge. Selected
tuples use planned `CanonicalValueHandle` lanes on the same memory account.
Copies clone immutable Value handles; projections retain the same buffer owner.
No second interpreter, source algorithm, pointer serialization or scalar
instruction dispatcher is introduced. Scalar emission sources are byte identical
to the base, and the existing caller-layout envelope remains unchanged.

Same-plan liveness includes the exact loop seed/latch read sites, with reads
before writes and later definitions killing only old visits. It releases
exhausted frame/operand slots after their current reads. The existing unsigned
buffer leaf must still obtain actual exclusive same-account ownership from the
core; external aliases and prior publications remain immutable. Lexical cleanup
also releases frame handles at loop boundaries and on failed invocations.

Tuple shape binding uses `CompositeSnapshotConstructor::binding_cost_for_inputs`,
`shape_for_children` and `bind`. The allocation-free preflight is prepaid; the
resulting temporary bytes, metadata nodes and bounded binding work are admitted
before construction. The selected call's logical output bound remains checked.
The newly added quota test failed before the admission correction, and passes
with rejection, unchanged aliases/account use, and recovery afterward.

The reference lanes and their small owner exist only when selected. Their bytes
participate in transport/worker setup admission and their retained nodes in the
caller census. The owner is boxed to retain the existing scalar caller layout.
They are reference metadata, not a zero-filled replacement for path nodes.
Guest buffers reserve, append and clear at the same source sites as before.

## Measured evidence

All figures below are unoptimized host diagnostics on the same machine and
nightly-2026-03-03, not compression throughput or a language comparison.
Allocation bytes are cumulative requests, not peak memory or measured memcpy.
The before/after real argument test is unchanged in the maintained source and
runs with the same feature profile, data and finite allowances.

| Real tree/cache recursive step | Before (`aad1f6528`, same code as base) | After (`4b722986c`) |
| --- | --- | --- |
| Successful allocation requests | 5508 | 409 |
| Successful requested bytes | 26611790 | 41619 |
| Successful elapsed time | 48.7–51.5 ms | 0.452–0.500 ms |
| Rejected assertion requested bytes | 13315186 | 35593 |
| Published outputs, aliases, cleanup and retry | pass | pass |

The unchanged original pending recursive-capacity acceptance also passes now.
The permanent test exercises 24 original/decoded combinations: U64/U8, small
and real reservations, and 0/1/3 appends. Every capacity remains intact. After
warming equally, the extra requested bytes for the real reservation equal
exactly one extra payload, independent of recursive visits. For three appends,
real U64 uses 495 requests / 2486755 bytes, and real U8 uses 501 / 6679251 bytes.

The real collector prefix still performs substantial work: **6 positions** on a
320-byte overlap input, full physical tree/cache, depth **300**, nice **258** and
original near-EOF limits, in **13.939 / 13.921 seconds** for original/decoded
programs. It requests **259549511 / 259549447 bytes**, with no resource denial.
The complete expected state and all buffers agree with the pinned reference.
This remains one compiled Mech computation. Search and skip remain separate;
skip has no cache argument or recording path.

## Passing validation

The final component/linked/layout command ran at tracked-clean code
`4b722986c`, before this packet was created. Exact commands, environment, head,
dirty state, elapsed time and exits are in `receipts/`; terminal logs are
compressed unchanged under `logs/`, with SHA-256 hashes in `MANIFEST.json`.
Warnings remain errors, incremental compilation is off, and dev/test debug info
is disabled. Cargo uses `--locked --offline -j1`.

| Profile / coverage | Result |
| --- | --- |
| `mech-gpu --no-default-features --features canonical-native-program --lib --test canonical_artifact_program`, selected maintained qualification | **160 passed, 0 failed, 0 ignored, 129 filtered**, 518.00 s integration tests |
| Same command, native caller-layout unit test | **1 passed**, 422 filtered; persistent 1112 bytes, invocation 1288; original envelope / 2704 charged fixed nodes retained |
| Complete final command including build and both binaries | exit 0, **654.730 s** |
| Engine runtime-only, `sequence_buffers` | check pass |
| Engine source-free native, `native-artifact,sequence_buffers` | check pass |
| GPU source-free runtime, `canonical-native-program-runtime` | check pass |
| Formatting, warnings, native build analysis, resident routing, source entrypoints, Rust module layout and native host catalog | pass |

The integration count comprises the unchanged **143 component cases**, the
existing **8 sequence cases**, **7 new frame cases**, and **2 actual linked
process cases**. The additional frame case executes reserve/append recursion
through the generated linked library, valid→invalid→valid inputs, assertion
before arithmetic, unchanged published output on failure and recovery.

New regression coverage includes real allocation/capacity deltas, fresh Boolean
arguments on recursive visits, immutable published aliases, repeated calls and
resource-denial recovery, zero application-payload copy quota for real
arguments, escaping tuple fallback, scalar layout separation, independently
restored descriptor metadata and the core shape-binding admission bound.

The six permanent compressor programs, component expected results and policies,
73 native/bytecode fixture files, scalar emission sources, distribution/catalog
surfaces and warnings policy are unchanged. See the fixture and scalar source
verification JSON. No CI inventory repair or unrelated cleanup is mixed in.
The existing CI profile executes this entire integration target; live inventory
verification passes with zero gaps.

Automated code review completed with no findings at both code revisions,
including `4b722986c`. CI was canceled on this branch as requested. No completed
remote CI result is claimed. Release comparisons, full reymont, complete raw-byte
compression/decompression agreement and expensive ignored full-block diagnostics
were not run here. The earlier 320-byte full collector result remains the largest
completed block collector; it is not a compression result.

## Remaining exact blocker

`record-return-capacity.mec` is the next minimal witness. It returns a reserved
buffer in `{nodes: nodes}`, then appends after projecting `.nodes`. Contents are
correct, but capacity falls **305257→8 for U64** and **6633832→64 for U8**.
All four original/decoded content and cleanup checks pass; the capacity
acceptance fails. The same test fails with identical allocation totals on the
tracked-clean parent `851507304`, so this is an **inherited canonical record
transport limitation**, not introduced by this patch. See
`pending-record-return.rs.txt`, `next-operation.json`, and the paired receipts.
It is explicitly outside the passing count.

This blocks an actual pinned operation: `CompressOptimal.roc` defines
`PathResult` at line 394, clears the reusable nodes at 429, returns them in a
record at 506, reserves once at 720 and reuses `path.nodes` at 1000, 1062, 1158
and 1182. Copying away this reservation would change the target allocation
behavior. The next bounded change is ordinary record return/projection
ownership, starting with this acceptance and preserving aliases and recovery.
No record implementation is included in this patch.

An exploratory counted application of a tuple state is rejected by the existing
source carrier restriction (scalar or flat matrix); it is unqualified and is
not treated as a new compressor work package. The parent/current record witness
also records the existing resident default compute denial separately, with
unchanged allowances. Every failed validation or probe is retained and
classified in `FAILURES.json`.

## Compressor checkpoint

Permanent executing functions remain `bt-get-matches`, `bt-skip`,
`optimal-search-position`, `optimal-skip-run`, `optimal-block-loop`,
`optimal-collect-block`, `optimal-first-block` and `optimal-carry-block`, plus
statistics, block decisions, rewind and sliding. This patch improves their
compiler transport machinery; it does not supply the remaining compressor.

Still required for `compress_into`: default/adjusted costs, `find_min_cost_path`,
code-cost iteration and selection, integration of the qualified Huffman
component's real sizes/kinds/reversal table/allocation sites, block header and
bit writing, flush, orchestration, carryover and repeated output reuse. Connected
small cases must then match exact reference raw bytes and independent
decompression before full reymont and a complete measured run. The goal remains
active.
