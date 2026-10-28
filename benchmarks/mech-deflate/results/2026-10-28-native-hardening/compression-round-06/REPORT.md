# Round 06: consuming scalar indexed stores

Starting published head: `d9c2b9a3c62a7e29177918bd045a81023d9d8a01`.
Protocol and regression: `dfaa03115656ba0e2ed4bf84db024dffa2cd8e5a`.
Fixture correction: `151d5a0b5507b57b0f7cad0aee8784788a9b9bfe`.
Runtime implementation: `5929c0bff0a7e1b4b14aec2f684b0b8c634975ad`.
Exact resource tests: `5bd4cb352ba01e0dfb34982dc98d035b3473fffb`.
Stack parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

An ordinary plain unsigned scalar replacement now has a prepared selected
strategy. Index and incoming word remain in native lanes. The existing
same-plan last-use facts can retire matching dead arena slots; the existing
core transfer then verifies actual same-account frozen exclusivity. Private
backing transfers, shared backing makes an immutable copy, and unchanged writes
return the exact old owner. The changed flag compares one selected word, without
full snapshot equality. Source kind/shape/index, budgets and backing admission
precede mutation. Fresh wrapping/owner construction and atomic program publication
keep aliases and failed evaluations safe. Capacity and logical shape are retained.

The strategy uses the original constructor-private MatrixIndexedReplace factory,
shared JIT/Object preparation, independent image strategy derivation and existing
native runtime. It adds no opcode, representation, intrinsic or executor. The
original packed index mapping is reused for column-major selection over row-major
storage. Unsupported selected kinds keep their canonical route. Existing frame
retirement suffices for the measured recursion; no loop/capture exception or
field-sensitive ownership extension was needed to pass that witness.

## Small whole-compression timing

Apple M1, eight-byte `abcdefgh`, unchanged level 12, **Roc = 1x**:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned reference, 20000 observations | 0.000286 | 1x |
| Before consuming stores | 0.465164708 | 1626.45x |
| After consuming stores | 0.164188708 | 574.09x |

Mech is **2.8331x faster** than this round's Mech baseline, using **64.70% less
elapsed time**. The previous 1631.52x result used a different reference cohort;
this comparison reruns the retained before executable against the same fresh
reference as the candidate. Three retained observations per subject, excluded
warmup/extra calls and all raw rows are archived. The separate two-second profile
and its 25 repetitions are excluded from primary statistics. No compilation or
other qualification overlaps primary or protected scalar timing. No larger
complete input was timed.

Before executable SHA-256:
`8ad35661bca41e0a3ab1c79bbe006c94b0474f8782d222b27050e5d4835ec33c`.
After executable SHA-256:
`8b2a5b0c09ee3f2af07232c930b61d741b47db03eafed1f3597629298adebf33`.
The selected catalog, pinned mimalloc 2.1.7 and evaluate_into-only harness are
unchanged. Every call passes exact raw bytes, independent inflation, aliases,
capacity 13 and zero terminal managed bytes/nodes. All 25 components, source,
artifact, algorithm, tables, source allocation sites, buffers, search/EOF limits
and six deliberately unused Huffman initializers are unchanged.

Reported work changes from **941491317987 to 888390537256**, a delta of
-53100780731. These are logical accounting units, not machine instructions.
Actual copy/transport accounting changes; the conservative prepared metadata
envelope remains. The U16 store witness's logical work rises while U32 falls;
neither fact should be interpreted as an instruction count or elapsed-time claim.
No existing qualification allowance is increased.

## Copy-complexity and actual collector evidence

The new regression runs both original/decoded artifacts, internal initialization
and retained external aliases, sizes 288 and the real 262144 U16 / 1500513 U32,
and zero/one/eight reached recursive stores. All 48 observations are retained.
Before production changes, the test collects every observation, then fails:
eight externally aliased U16 stores scale allocation by 4189696 bytes versus
the required one-payload increment of 523712 bytes. The same regression body is
byte-identical before and after. Afterward all size deltas equal exactly one
payload: the internal initial allocation, or the first necessary external copy.
Subsequent stores do not allocate additional payloads.

Representative eight-store original-artifact diagnostics:

| Recursive case | Before requested bytes | After requested bytes | Before/after requests |
| --- | ---: | ---: | ---: |
| U16, external immutable alias | 5193242 | 570554 | 9081 / 522 |
| U32, external immutable alias | 49015354 | 6048318 | 9081 / 522 |
| U16, internally initialized | 5670153 | 571921 | 9035 / 523 |
| U32, internally initialized | 54970029 | 6049685 | 9035 / 523 |

The unchanged straight-line 24-observation real-buffer witness also passes.
Eight stores fall from 8833 to 274 allocation requests. U16 requested bytes fall
from 5177904 to 555312; U32 from 49000016 to 6033076. Retained diagnostic medians
are in `word-comparison.json`; they are instrumented System-allocator observations,
not primary complete-compression timings.

The actual permanent collector runs Mech initialization, recursive argument
frames and consecutive search/skip calls at depth 300 and nice 258, with the
full U16 tree and U32 cache. Both original/decoded forms complete six positions
on 320 bytes and match all golden table/cache words, headers, hashes and state.
The same finite policies and expected values pass in retained before and new
binaries. Original-form requests fall **25682 to 8362**, requested bytes
**79457091 to 35045914**, diagnostic elapsed **0.0093455 to 0.004712125 seconds**.
Decoded results agree as well. These single instrumented component observations
show that the actual carrier benefits; they are not a controlled large-input
benchmark. Additional copies remain in that path. Initialization and collector
cleanup checks pass, with no resource denial.

The reference compression entry contains direct indexed word load/add/store
instructions. The new Mech selected store helper contains width/range checks and
native byte/halfword/word stores, with no payload loop or copy call inside that
helper. Admission, ownership transfer and shared copy happen outside it. Exact
commands, binary hashes and compressed assembly are archived. The opaque
reference symbol is not independently source mapped; no stronger equivalence
is inferred from those instructions.

## Qualification and remaining gates

**84 scoped Rust tests pass:** 71 component tests, eight native scalar tests,
three bit tests and two actual collector tests. New tests cover U8/U16/U32/U64,
rectangular selection, high bits, exact changed flag, exact backing identity,
external/no-op aliases, zero-copy denial, invalid index, successful recovery and
cleanup. Exact/one-short output elements, output bytes, clone bytes, temporary
bytes, retained nodes, compute and comparison work all pass. Existing component
bodies and allowances remain unchanged. Original/decoded forms are covered.

The selected release consumer passes locked offline checking (31.74 s) and
build (454.37 s), reusing every machine library including math. All twelve
architecture/catalog/routing/inventory/warning/boundary/format commands pass.
Protected original/decoded Bool/U64/I64/F64 allocation counts/bytes match exactly;
median ratios 0.9982–1.0164 pass the predeclared 1.05 gate. Zero/64-visit scalar
loop allocation slopes stay zero. Actual toolchain, features, linked libraries,
commands and hashes are retained.

One initial new-fixture build has a Rust macro grouping error, corrected before
the real red test. Its receipt is separate from the expected allocation failure.
The final tests use the clean test head; production libraries are from 5929c0bf
and differ only by later test additions. No executed production failure remains.
Fresh source-derived artifact regeneration was last checked in round 05; the
source/artifact hashes are verified again for timing and this round changes
only native planning/runtime paths. Broader linked/generated applications and
the route unit-test suite are not fresh executions. Final block/window/full-file
coverage and inherited unsupported forms retain their earlier disposition.

The new profile has 1356 samples: budget evaluation, TLS, control-work charging
and runtime visits are now prominent. Ordered schema precharge accounts for
212 inclusive samples. Allocation/free is much smaller. Sampling is diagnostic;
raw rows retain unresolved/deduplicated/JIT symbols without invented attribution.
Use this profile to choose the next substantive reduction in repeated runtime
work. CI is cancelled by request, #973 remains draft and unmerged, and parity
is not achieved. No cancelled long campaign was restarted.
