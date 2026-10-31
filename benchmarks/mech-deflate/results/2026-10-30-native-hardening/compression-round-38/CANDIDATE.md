# Retire completed buffer-return aliases at their exact read visits

Starting published checkpoint: `ba198160dc51162da49d962866fe4a19e0aa051c`.
No performance gain is claimed before qualification and paired measurement.

The original read-only census observes 68 real large-buffer helper calls. Of the
38 additional path allocations, 32 appends have no usable last-use proof, four
first appends retain the completed callee's return slot, and two clears retain
private-frame aliases. Four tree copies retain another arena slot. All bytes,
aliases, work 44443887150 and final zero cleanup agree with the unobserved call.

The exact pinned path-guard component reproduces a narrow part of that problem.
Raising capacity 288→305257 changes requested bytes by 4,879,504: **two payloads**,
where the guard function followed by one append requires one backing. Existing
values, real sizes, source recurrence, native-loop selection, allowances and
allocation-slope checks are retained. The added one-backing assertion fails on
the frozen round-37 compiler. A separate original/decoded control passes there:
it keeps the first result as a future call seed, checks both arrays, preserves
assertion-before-division precedence and publication on rejection, and retries.

Private frame retirement already maps loop seed/next/return reads to their exact
initializer/latch/finish visits. Buffer-call retirement instead collapsed every
one into an opaque fusion read and could never release a completed return slot.
Move the existing read-site mapping into the shared old-owner graph query, reuse
it from frame retirement, and use it for unsigned buffer candidates. Collections,
publication and effects retain their independent opaque frontiers. Reachable
future reads, reads before a redefinition and a frontier's own terminator remain
authoritative. Runtime identity checks, real alias refusal and the existing
consuming handoff remain unchanged.

No new owner, executor, language construct, source specialization or resource
allowance. The unused cached fusion-read field/method is now gated by its real
text/map consumers, preserving warnings-as-errors for buffer-only configurations.
This is a general completed-call handoff improvement, not permission to retire
the remaining live frame or control-dependent owners. Their exact witnesses stay
recorded for the next bounded step.
