# Inline the existing work-meter wrapper

Status: **measured and retained**; see [the closure packet](CLOSURE.md).

Starting compiler/review head: `6123fdf46b749c5b5793591be7f9ed92f38269da`.
The review receipt for that head is issue comment 6526753449 on PR 973.

The immediately preceding complete eight-byte profile has 27 exclusive samples
in NativeWorkMeter::charge, 56 in the maintained library debit and 51 in TLS
lookup. Frame retirement alone has 58 exclusive samples. These are samples,
not exact elapsed fractions. The actual baseline disassembly shows the frame
retirement helper calling the 86-instruction work-meter wrapper, which calls
the maintained library debit. The existing wrapper returns a large error enum;
its caller tests the result before continuing. The pinned reference's hot
function contains direct arithmetic and loops without this Mech wrapper.

The sole production change is an ordinary `#[inline]` hint on the existing
NativeWorkMeter::charge method. It allows the Rust optimizer to fold the wrapper
into its callers. No prices, validation, scopes, control flow, limits, source
programs, owners or executor contracts change. No new semantic specialization,
ABI revision, source-size cutoff or instruction allowance is introduced.

This is a measured candidate, not a claimed improvement. Compare fresh balanced
eight-byte complete compression with the retained previous executable, using
the unchanged worker, artifact, pinned allocator and evaluation-only boundary.
Retain every observation and commit each reference/cohort immediately. Check
raw bytes, independent inflation, aliases, capacity 13, identical work and zero
cleanup. Run the existing accounting/runtime tests and protected scalar/frame
controls. Inspect the changed machine code and collect a separate fresh profile.
Keep a demonstrated modest gain; retain counterevidence if it does not help.

The baseline binary inspection is archived beside this note. Its reference
function is a whole large function, so raw instruction/call totals must not be
compared as equivalent function boundaries or runtime counts.
