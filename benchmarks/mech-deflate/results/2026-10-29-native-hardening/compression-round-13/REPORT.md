# Round 13: private mixed numeric results

Compiler `1ec498a67fc862bac39188fdf5040eaacf6e954c`, base `1087e7b8160894f542b81d46b887eb32a450125c`.
The unchanged eight-byte level-12 pair is **253.03× → 250.01× Roc**
(Roc =1×), **1.19% less elapsed**. Mech medians are 0.073378958 →
0.072503125 seconds, with a fresh 0.000290-second reference
from20000 observations. This is a small observed gain, not a large total win or
large-input throughput. Three primary observations per side are retained;
earlier round references and pairs remain separate. The before compiler is the
retained budget-correct round-12 executable, not round11.

All exact raw bytes, independent inflation, capacity13 and zero terminal bytes /
nodes pass. Physical output backing changes on both sides. The pinned source /
bytecode, real tree/cache, algorithm, tables, search/EOF limits, allocation sites,
mimalloc2.1.7 and evaluate_into-only timing boundary are unchanged. No tests or
builds overlap primary timing or the separate56-repetition after profile.

The typed Scalar / Array / private Frame result recipe is derived from existing
exact nonescaping BufferFrames fields and selected factory/dataflow facts. The
Frame class admits exactly one current unsigned owner and supported scalar
fields. Return freshness is checked on every reaching path. Actual terminal
scalar lanes, which may differ from carrier lanes, are spilled and synchronized
once after private cells freeze. Existing admitted frame lanes own the result;
no extra pin is added before a consuming write, and no public aggregate ABI or
Core ownership implementation changes. IR118/work policy93 identifies the new
recipe and bounded terminal synchronization; canonical bytecode14 / ABI35 remain.
Records, multiple array results, nested/escaping composites, growth and child
native regions keep the maintained path.

The required unchanged huff-assign-symbols selects beneath huff-assign-lengths,
as does the independent array-plus-index leaf beneath its ordinary parent. Its
terminal index is checked through the next symbol run. The generic terminal
returns i+17, making synchronization from a carrier lane observably wrong.
At256 assignments, System/probe requests / bytes / ordinary visits change:

| Witness | Before | After |
| --- | --- | --- |
| Generic two-call parent |9251 /646577 /9811|1619 /172265 /51|
| Unchanged assign-lengths parent |4918 /303186 /9651|340 /52138 /108|

Positive iteration counts have flat after allocations and visits. These component
observations are separate from complete mimalloc timing. The large component
changes do not imply a large whole-program gain.

The16 distinct focused integration checks (including actual linked execution
and restored proof rejection) pass across the retained cohorts. All4 actual
runtime units pass, including16 observed mutated-prefix exits for the new mixed
carrier, retained old write/read-only fuel checks and adversarial prepaid
publication. Current fields, input aliases, prior publication identity, cleanup
and successful retry are verified. All12 static contracts pass. Full existing
component qualification, protected scalar timing/allocation controls and complete
Object inventory are still pending at this timing checkpoint. The historical
round-10 decoded-U64 ratio1.0526036 against1.05 remains unresolved.

The secondary unchanged cumulative loop remains excluded by the current-version
check (`leaf returns an older array version`). This patch preserves predicate-
blind intersection and does not weaken freshness to force selection. Its actual
post-change candidates are retained in the focused log. A predicate-correlation
proof is a separate follow-on.

`CHECKPOINT.md` and the initial/final qualification directories retain each failed
attempt and correction. In particular, the mixed fuel fixture originally lacked
an exact private-use frame and then denied later public packing. The existing
observer now captures native exit work to force genuine native fuel denial;
production limits are unchanged. Two other fixture assumptions were corrected:
an invalid partial library policy, and a resident oracle exceeding its inherited
65536 comparison ceiling. That ceiling is unchanged; an independent full-array
model verifies the large retry case. Existing component expectations and
allowances are unchanged. No remaining production failure is known in the
executed scope. Broader checks remain pending, not claimed passing.

`timing-inventory.json` records exact commands, library/executable/source hashes,
profiles, outputs and observations. Large binaries stay private. The timing
packet is committed before another optimization. No larger input is queued.
