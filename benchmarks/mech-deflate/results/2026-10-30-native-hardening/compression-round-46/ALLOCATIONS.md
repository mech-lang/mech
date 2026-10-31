# Small allocation census

Two fresh linked diagnostic executions of the retained pre-prefix compiler `b12f6c5afb2c859f6cde29ba566ddf09e70d2f79` agree exactly on steady-state request counts, bytes, constructors and phase/lifetime classifications. Primary timing is unaffected. Each also verifies exact compression, aliases, capacity/work and zero managed cleanup. The prefix candidate's separate allocator cohort confirms the same allocation totals.

**13,994 requests / 98,586,843 bytes** reconcile into **50 requests / 97,129,588 bytes at least 64 KiB**, and **13,944 requests / 1,457,255 bytes below 64 KiB**. The small requests average 104.51 bytes.

| Dynamic phase | Small requests | All requests | All requested bytes |
| --- | ---: | ---: | ---: |
| Invocation setup before linked entry | **1,709** | 1,718 | 3,401,608 |
| Inside generated entry, including its cleanup helpers | **12,232** | 12,273 | 95,185,043 |
| Return/publication teardown after entry | **3** | 3 | 192 |
| Total | **13,944** | **13,994** | **98,586,843** |

The entry wrapper only marks dynamic phases; it changes no allocation operation. Constructors are grouped from resolved full backtraces, retaining raw stacks and function offsets. The first five constructor groups account for **6,398 small requests**:

| Constructor / source | Requests | Bytes | Request sizes | Lifetime at end of evaluate_into |
| --- | ---: | ---: | ---: | --- |
| Core `finalized_value`, snapshot/validation.rs:2628 | **2,022** | **204,896** | 32–160 | 2 survive; 2,020 freed |
| Core `Value::into_memory_budget_inner`, validation.rs:687 | **2,020** | **218,160** | 104 / 112 | 2 survive; 2,018 freed |
| Lazy mutex initialization under the same admission | **1,010** | **64,640** | 64 | 1 survives; 1,009 freed |
| Core `Schema::closed_body`, schema/mod.rs:69 | **673** | **5,384** | 8 | All freed, each after 3 subsequent requests |
| `Vec<DimensionExpr>::into_boxed_slice` during schema closing | **673** | **32,304** | 48 | All freed, each after 3 subsequent requests |

Admission's 2,020 requests split into **1,010 104-byte registration allocations and 1,010 112-byte wrapper allocations**. That establishes the wrapper path's frequency, **not how many roots satisfy exclusive/unadmitted eligibility**. The next pilot must measure eligibility before claiming a removable count.

The first three groups' mean observed ages through their next reallocation/free are 776.32, 6,708.08 and 6,706.58 subsequent requests; maximum ages are 12,235, 13,778 and 13,776. Survivor counts refer to the timer boundary; the existing final drop verifies zero managed bytes/nodes. These request ages describe lifetime without pretending the stack-traced nanosecond durations match uninstrumented execution. Reallocation ends the preceding request's lifetime even when backing does not move.

The cold warmup is marked separately: **14,495 requests**, including 501 extra setup/activation requests. An initial diagnostic-driver assertion mistakenly applied the steady-state expectation to it; raw steady-state samples already matched. The corrected driver preserves the warmup and all raw evidence. No benchmark allowance or expected output changed.

Instrumentation uses fixed C tables, aborts on table/stack overflow, and traces all small Rust allocator requests with the pinned allocator. Its static storage and stack-walking overhead are excluded from all primary memory/time claims. Both traces contain 6,896 distinct full-stack sites; grouping these by constructor avoids splitting the same constructor merely because expanded source call sites differ.

`allocation_attribution.py` reconstructs and cross-checks the groups from archived resolved observations. `reconstruct.py` verifies both repetitions and the complete scorecard. This diagnostic does not establish total bytes copied, unique-root eligibility, or a latency saving from deleting a wrapper; those are the next experiment's acceptance conditions.
