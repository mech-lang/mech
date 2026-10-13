Additive source-only Clojure grammar v3 candidate. No parser/fixture/hash/replay/
queue has executed. The entire frozen v2 packet, actual29 fixture pass and
original21-source replay/15 diagnostics remain preserved. No benchmark source
or report is changed. All eventual imports/tests/hash/normalization stay under
/tmp/mech-clbg-timing.lock. Parser-pass is an unadmitted count proposal.

The original replay has ten Nbody cascade diagnostics caused by one actually
missing ')' on line26 in both mains. This revision does not waive or change those
reader diagnostics. Crystal's private-v2 scored-source corrections must be
separately bound to actual compiled-input inventory and replayed as new sources;
old bad source/replay bytes stay intact. Do not hardcode original root mains as
if they were the corrected compiled inputs. Each profile counts exact final
main plus the entire authored support namespace once; generated classes remain
provenance only.

The remaining five source diagnostics are addressed in three narrow changes:

K compact28/optimized40: a doseq binding vector contains sort-by's explicit fn.
The old parser diagnosed its frontier while charging the whole control header,
so simply walking the fn would overlap owners. V3 splits that iteration-header
span around the nested fn and retains one outer doseq header plus the actual
fn header, body and close. The outer control owner is independent; ordinary
call/initializer prefix composition does not waive it. Each
additional callback/header/body/close and every
independent effect remains visible and non-overlapping. Other executable
iteration header forms still diagnose; arbitrary do/let/control initializers
are not exempt. The selected comparator definition is not rewritten.

Spectral compact11/optimized12: only actual core aget with a lexically visible
^doubles formal receiver and effect-free scalar index may be a value prefix
before nested control. The reader's enclosing function/binding scopes establish
that type; nearer binding/formal/global def/defn shadows erase or reject it.
Refer-clojure exclusions and require refer names also reject core resolution.
No generic call or scalar-return whitelist applies. Changed type, unknown array,
shadowed aget, rebinding the receiver and mutating index operands remain gated.
Every nested if/branch/close retains ownership. Added evidence records the exact
resolved call AST/span and lexical receiver types for later semantic review.

Pi compact20: the namespace must import java.math.BigInteger and the local
integer helper must be the unique global binding across def/defonce/defn/defn-
and have the exact single body BigInteger/valueOf(long value)
with its written parameter. Resolved base constant seeds and value-producing
.add/.subtract/.multiply/.divide trees establish local receiver/argument types;
type hints or Java method names alone cannot qualify them. Arbitrary Java
methods, wrong namespace/receiver, changed helper body, shadows and unresolved
operands remain diagnosed. The whole helper, methods in source, both independent
quotients and nested final if remain counted/unchanged. This is selected source
value-expression ownership, not a benchmark runtime-work proof or blanket Java
purity claim. Full selected-source review must bind exact caller and runtime
bytes and inspect the closed BigInteger seed/recur flow before admitting counts.
The actual canonical Pi source remains responsible for both extractions.

Primary library evidence: Java SE25's official BigInteger API describes immutable
integers and value-returning arithmetic methods:
https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/math/BigInteger.html
This does not replace actual pinned JDK/runtime/class provenance or source review.

The two original parser/test files are copied additively from v2. Original29
fixture methods remain unchanged. Three new meaningful methods make exact32:
- test_iteration_callback_header_keeps_callable_body_close_and_all_work
- test_typed_array_read_prefix_rejects_shadowed_and_effectful_operands
- test_exact_BigInteger_value_prefix_rejects_changed_helper_type_and_target
They assert actual owner spans/non-overlap and retained bodies/closes, two
callbacks, shadowed/rebound/changed types and imports, mutating operands, and
changed helper/unknown method targets. Expected counts are authored expectations,
not claimed passing. No fixture worker/replay is authorized by this packet.

Before future qualification, root must review these concrete files and exact32
inventory. The separate guarded worker must freeze them with qualified LOC18
language-neutral dependencies, require the actual v2 fixture/replay receipt pins,
preserve raw logs/failed terminal artifacts under the common mutex, and import
only the frozen snapshot. Later replay consumes actual qualified private-v2
Clojure compiler inputs, retaining byte-equal copies/roles/commands; it must not
silently count old Nbody sources or exclude authored support. Source/runtime
binding, complete source ownership review and raw remaining diagnostics remain
mandatory before any canonical LOC overlay or chart admission.

Crystal's concrete additive source-profile manifest is now
clojure-comparison/20261012/revisions/private-v2/profiles.json. Only its two Nbody
main/compiled-source-bundle paths move to revisions/private-v2/sources/clbg/nbody/
{compact,optimized}.clj, preserving namespaces and support/clbg/support.clj.
The actual private-v2 qualification now passed20 profiles/156 bounded cases:
report SHA25669bf7cdb025759e6d10cb8c1ae1c189839e253ced9aca50e4f9cc1e838d36d79.
That consists of24 explicitly retained cases and132 fresh attempts/16 builds,
with all saved pins unchanged. This does not qualify runtime work or full timing.
The separate fourth file qualify_replay.py is now a source-only guarded worker.
It binds these exact declared inputs to that actual qualification, each original
compiler inventory/receipt, copied package and snapshot source, Java/compiler
jar/class artifacts and roles. It rejects extra copied package files, compiler
cleanup faults and missing authored support. There is no fallback to old Nbody
source paths. All original raw replay rows stay preserved under their old identity.

The direct fixture stage acquires the common mutex before prerequisites/imports,
verifies the original29/15 replay and qualified LOC18/Python3.14/Pygments2.19.2,
copies the four-file packet and qualified neutral dependencies to immutable
snapshots, pins exact Python/unittest sources, and rejects cached or mislocated
modules. Actual32 discovered/started IDs, raw logs and zero skip/expected-failure
outcomes are required. Its terminal context retains all available bytes and a
separate failed terminal status even after a saved passed receipt; pin errors are
fatal. No compiler/native process or signal is issued.

Proposed fixture-only command (not queued):
python3.14 -B benchmarks/clbg/loc-normalization/20261012/clojure-grammar-v3/qualify_replay.py --stage fixtures --revision grammar-v3-fixtures-v1

Only after root reviews the actual32 receipt/digest may the same frozen worker
run a distinct --stage replay --revision grammar-v3-source-replay-v1, with
--qualification-report clojure-comparison/20261012/qualification/private-v2/report.json
(an absolute path in the actual command), --qualification-sha256 set to the
full digest above, --fixture-receipt pointing to the actual fixture-only receipt,
and --fixture-sha256 set to its reviewed terminal digest. Replay reuses exact32
fixture evidence without rerunning tests and requires the current packet bytes
to match that frozen receipt. It counts21 exact final authored files into20
main-plus-whole-support proposals, retaining every diagnostic/resolved-prefix
AST fact. Complete selected-source/runtime semantic review remains mandatory.
Canonical LOC and chart admission remain false. No worker is queued here.
