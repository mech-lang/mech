# Shared descriptor experiment (R68, rejected)

Current production retains the original inline resolved type/schema descriptor.
R68 tried sharing their immutable definition while keeping each current shape
independent and validated. Correctness qualification passed and the complete
eight-byte call removed 728 allocation requests, but matched elapsed rose
0.8594% and retired instructions rose 2.4194%. The broad representation change
was therefore reverted. Its source, zero-allocation clone witness, full census,
counters, host assembly and design remain in the R68 receipt packet.

The instruction-window diagnostic attributes almost all of the extra work to
the linked entry and its callbacks, rather than setup/cleanup. It does not yet
identify a precise callback or prove a particular pointer/refcount explanation.

The next experiment should share immutable descriptor arrays at the call-plan
boundary while preserving ordinary descriptor access. It must be generic, keep
current shape/account authority dynamic, and repeat the same full-call timing
and instruction comparison. No gain is assumed before measurement.

The two new shape/lifetime semantic regressions and separately repaired source
contract guards remain. The rejected clone-allocation acceptance witness is
archived with the candidate; production constructor coverage returns to the
original four tests and its strict four-test CI inventory.
