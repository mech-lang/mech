# Round 08: mixed scalar state through the existing native layout

Starting published head: `8c98f46506e1baf9568cff2a395452f21acc1b07`.
Protocol and first red test: `37ba94f2812ea23f51401cbf7ba4a0346455e1c4`.
Independent signed/enum red tests: `8d1792af42e412a25441d35c6773b9a96d9826ae`.
Engine lowering and regressions: `7692a4453e1b67eba6609eb2f5295deaa565d69c`.
Existing audited image harness: `803e2e83bc76d31d4d2a5d5dad7b28ba04ab9f8b`.
Closed-tail failure regression: `c4681888b9d5e9a315842e5ec9a7d57cb81d1e9d`.
Shared source-failure unwind: `c465f1712fed87e116771a9785207054258ce9ba`.
Status namespace correction / measured head: `4af4662097dd570f0821fc105a1c4d9de4eb5d89`.
Stack parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The existing private word layout now accepts flat tuples and enum payloads of
Bool/U8/U16/U32/U64/I64/F64. It uses the shared scalar kind and bit transport,
exact source/target kinds, original selected operations, whole-component escape
analysis, guarded tags and the existing 32-field boundary. Records, nested
payloads and escaping results retain canonical storage. There is no new schema,
codec, opcode, executor, ownership exemption or runtime representation. Shared
emission serves JIT and Object; images independently derive the same private
metadata. The production Engine patch is 31 added / 15 removed lines.

The real compressor now selects seven I64 pairs, four U32 pairs and its U64/U32
choice tuples/enums, for 91 private values / 310 words. Eight actual optimal-path
Tail carriers select the mixed word layout. The old implementation unconditionally
rejects these non-U64 layouts, and both minimal reproductions fail on the unchanged
old libraries before production edits. This static inventory is not a dynamic
execution count. Existing permanent path/default-cost cases supply execution.

## Small whole-compression timing

Apple M1, unchanged eight-byte `abcdefgh`, level 12, **Roc = 1x**:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned reference, 20000 observations | 0.000285 | 1x |
| Before mixed state | 0.139637125 | 489.95x |
| After mixed state plus failure repair | 0.129644084 | 454.89x |

This is **1.0771x faster** than the paired Mech baseline, or **7.16% less elapsed
time**. It does not deliver the larger performance improvement being sought.
The preceding 495.21x result uses a different reference cohort. This round reruns
the retained before executable against the same fresh reference as the candidate.
Three retained calls per subject, excluded warmup/extra calls, raw rows and a
separate two-second profile with 31 repetitions are archived. No compilation or
qualification overlaps primary or protected scalar timing. No larger complete
input was timed. The uncorrected candidate is not a primary timing observation.

Before executable SHA-256:
`97aa2b05a99ddc3efecc0fe04cda64a817bb8ef21bee0f8d61375e715b00d3d8`.
After executable SHA-256:
`0fa38254f8f5773f49749201a07ad2032356d38b5d7f47a44f4c581a6bdc6570`.
Source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Artifact SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
The selected catalog, pinned mimalloc 2.1.7 and evaluate_into-only boundary stay
fixed. Every call passes exact raw bytes, independent inflation, aliases,
capacity 13 and terminal zero managed bytes/nodes. Logical work changes from
888390537256 to **875610515844**, delta **-12780021412**: fewer canonical state
packing/projection charges. These are accounting units, not instructions. No
existing qualification allowance changes. All 25 components, algorithm, tables,
search/EOF limits, real tree/cache, allocation sites and six unused initializers
are unchanged. No application source rewrite is included in this round.

## Failure-path discovery and correction

The first extended run passes 94 tests and fails the new checked mixed recursion:
a false assertion returns a descriptor error about outstanding compiled String
loans. The active loan is closed-Tail fuel, not a source String. Managed source
failure terminators returned directly, bypassing the existing body-error unwind.
The fix routes both checked assertion and rejection exits through that existing
path before returning their original typed status and origin. It does not discard
an error, suppress an invariant or disable the optimization. Scalar-only entries
without these managed loans keep their existing entry.

A same-shaped all-U64 regression establishes that the problem predates this
optimization. The first attempted parent test used earlier Cargo library paths;
Cargo had reused those paths for the current candidate, so that attempt is
**not clean-parent evidence**. Its receipt and provenance correction are retained.
The definitive reproduction uses the retained, hash-checked **before executable**
97aa2b05 on a 226-byte, unused-result assertion program: it reports the outstanding-
loan descriptor error. The new retained executable reports the correct Integrity
error for the identical bytecode. Canonical source frontend and artifact codec owners are
unchanged since the published parent. These intentional failing invocations are
functional witnesses, not timing rows or cleanup qualifications. The maintained
original/decoded tests then prove repeated rejection, unchanged output identity,
input aliases, complete memory/node cleanup and successful retry on the final
compiler. The cleanup defect is inherited; mixed-state support exposes it on
additional inputs.

## Qualification, configurations and failures

**109 scoped Rust tests pass:** 96 component/state/closed-tail/default-cost/path
tests, eight native scalar tests, three bit tests and two actual collector tests.
This includes the existing 71-test cohort unchanged. The new state coverage
checks every supported scalar kind, high unsigned bits, signed extrema, NaN
payload and negative-zero bits, changing Boolean state, permutations, constants,
original/decoded artifacts, JIT/Object identity and restored metadata. Escaping
results stay canonical and unsupported nested payloads retain the original path.
Twenty original/decoded allocation observations over zero/one/two/63/64 visits
all request exactly 1211 allocations / 230926 bytes: zero per-visit growth.
The checked-state tests preserve assertion/arithmetic error precedence in both
orders, one-short execution denial, prior output identities, cleanup and retry.
Existing pure-U64 scalar, nontermination, width-boundary and native-fuel tests pass.

The actual collector still completes six positions on 320 bytes using the real
262144-entry U16 tree / 1500513-word U32 cache, depth 300, nice 258 and separate
search/skip operations. Golden table/cache/header/hash/state checks and the same
finite policies pass. The separate store witness retains identical allocations,
bytes and work. Actual collector traffic is retained in its own diagnostic rows;
it is not a new large-input timing result.

Protected original/decoded Bool/U64/I64/F64 controls preserve exact allocation
counts and bytes. Median ratios 0.9780–1.0051 pass the declared 1.05 gate; zero/64-
visit scalar-loop allocation slopes stay zero. The initial Engine release build
passes in 458.00 seconds with all machine libraries reused. The subsequent host-
only failure-exit repair rebuild passes in 72.28 seconds with Engine and machines
fresh. Selected checking passes in 20.17 seconds. Full twelve-command static
checks pass on c465f171; final 4af46620 changes only the rejection-status constant's
namespace and passes compilation, formatting and whitespace checks. Actual
commands, features, dependency hashes and toolchain are archived. No warning is
suppressed. Unit budget tests passed in round 07 and are not counted fresh here.

Additional retained failures/dispositions:

- The first image test adds a new unsafe location and fails the strict boundary
  audit. The case moves into the existing audited restoration test, preserving
  the allowlist and exercising both frame and mixed-scalar descriptors. The
  complete corrected audit passes; this is an introduced test-authoring error.
- The first cleanup build references STATUS_REJECT in the wrong ABI module and
  fails E0425. The separate namespace commit repairs this introduced compile error.
- The optional complete-plan inventory initially uses the ordinary 67108864-byte
  definition-table limit and is denied at 120162192 bytes. The benchmark already
  uses Diagnostic NoLimits. The corrected inspection explicitly uses u64::MAX;
  no production allowance changes and no benchmark is blocked by this setup error.
- The all-primitives scalar test needs not/xor absent from the selected profiling
  catalog and remains explicitly skipped. Linked/generated applications, the host
  route unit suite, final block/window and full-file execution are not refreshed.
  Fresh source/artifact regeneration remains round 05; pinned hashes are checked
  again here and frontend/codec source is unchanged. Ignored/cancelled campaigns
  provide no pass and are not restarted.

No executed production failure remains. Published parent 8c98f465 has a completed
review with no findings; this new head will request its own review. CI is cancelled
by request, #973 remains draft/unmerged, and parity is not achieved.

The new profile contains 1370 samples: TLS 144, budget evaluation 93, runtime visit
73, memmove 57 and native work charging 51 are prominent exclusive symbols.
Ordered schema precharge contributes 31 inclusive samples. These diagnostic
counts are not exact time shares and unresolved/JIT addresses remain unresolved.
The permanent default cost-table case still requests about 44000 allocations for
544 independent scalar updates. The next experiment should remove whole chains
of table construction transactions, with any application source optimization
kept separate from this pinned-source compiler baseline and separately measured.
