# Cost length in the composed scorer

The composed caller already checks that its cost view has at least 545
entries. The experimental scorer uses that bound on the common path. An
index at or above the admitted floor rechecks the original full length
from the caller's immutable packet and takes the original default when
out of range. The same packet field must prove both bounds. The native
boundary harness checks 102 inputs on each of three generated entries,
including the admitted boundary, larger valid indexes, out-of-range
indexes, wrapping offsets, and unrelated packet fields.
[Source patch](compiler.patch.gz) · [Boundary results](boundary/result.json)

The comparison uses the exact earlier 71,096 B bottom-tested scorer as
its control. Only the placed composed scorer changes in the candidate.
The checked root has the same bytes and relocations. The candidate
emits **71,128 B**, or **1.918× libdeflate** and **1.233× Roc**.
It is **64 B smaller** than the maintained 71,192 B frame-copy encoder.
The 162 linked observations and six checked failures agree, with a
measured stack high-water of **5,280 B** for all three workers.
[Object and link identities](build-comparison.json.gz) · [Linked checks](linked/result.json)

On natural 8 KiB the placed scorer executes **54 stack loads** across
six calls, down from **43,074**. Total dynamic scorer instructions fall
from 7,523,934 to 7,483,578. The stack-load change is observed with
a separate counted worker. Timings use the original executable.
[Instruction counts](placed-counts.json) · [Counter inputs](counts/)

| Case | Order | Control vs libdeflate | Candidate vs libdeflate |
| --- | --- | ---: | ---: |
| `320-overlap` | Forward | 0.98921× | 0.99041× |
| `320-overlap` | Reverse | 0.98581× | 0.98831× |
| `prefix-natural-4096` | Forward | 1.09129× | 1.08963× |
| `prefix-natural-4096` | Reverse | 1.09508× | 1.09218× |
| `prefix-natural-8192` | Forward | 0.99702× | 0.99426× |
| `prefix-natural-8192` | Reverse | 1.00016× | 1.00101× |

Both workers take **15/15 Roc** in both orders and **13/15, 12/15
libdeflate**, with the same won cases. The natural 8 KiB paired change
is **-0.10%** against the bottom-tested control
with a 90% interval of **-0.33% to +0.15%**.
The reverse result above parity leaves the candidate **Unresolved**
under the v3 qualification. Complete timing orders for this arm
remain pending. [Focus schedule and results](focus/screen.json) ·
[Raw forward observations](focus/forward.tar.gz) · [Raw reverse observations](focus/reverse.tar.gz)

For a subsequent scorer experiment, the remaining time needs a different
mechanism from the removed reload. The 4 KiB target is still about 9%
above parity in each focused order.
