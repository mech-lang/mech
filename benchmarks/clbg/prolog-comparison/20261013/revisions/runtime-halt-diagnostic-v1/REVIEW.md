# Prolog shutdown diagnosis and two bounded controls

Source-only, mutable, unexecuted, unhashed and unfrozen. This two-file packet
contains a prospective diagnostic helper and this note. It adds no retry or
native execution authorization. The frozen runtime-probe-v1 packet, failed
outputs, current chart and datasets remain unchanged.

The actual latest failure is closed in
`continuation/jobs/prolog-runtime-probe-closed-review-v1/review.json`.
Root verified 13,213 recorded pins. Its actual durable terminal is
`3d2240941209d7557bc4a4dc99e6dab87dd7e06ae06efdd31af78c57c0eba58e`,
report `f143a7b6627af26bc46741ac287c50ef80980d355411be468b1104ded09cfe66`,
report terminal
`1f534082e617e804a300e5f3abe887746b8a2e0d20511e0ce525f037d3c7f878`,
complete raw result
`5e1ffe68eccc864e4ba45fcd562f700eb9e07af37f2ec76f639ff709555a86af`,
and distinct owned feature receipt
`835221d6c6ba1754a40e12bc6cfff3590f270fe62e062345781a953e5052f165`.
The command finished normally with exit1 and final owned exit1 in about35ms.
Native stdout/stderr are empty. No image closure ran; no runtime qualification
or chart admission follows from the raw JSON. Removing the imported compiler
initializer did not resolve exit1, so that earlier hypothesis is not a fix.

## Concrete source route and remaining uncertainty

The retained official SWI-Prolog10.0.2 source below
`.build/swi-runtime-directory-v1/official-source/swipl-10.0.2` explains a route
that matches these observations:

1. Frozen `runtime_probe.pl:36` calls halt(0) after setup_call_cleanup returns.
   Its main/0 wraps the whole checked/0 goal in a catch whose recovery prints
   the exception and calls halt(1).
2. `src/pl-prims.c:5270` calls PL_halt with PL_HALT_WITH_EXCEPTION.
   `src/pl-fli.c:5058` and `src/pl-pro.c:608` raise unwind(halt(0)) when the
   outer query handles halt exceptions. `src/pl-pro.c:170` opens that query
   with PL_Q_EXCEPT_HALT.
3. "Uncatchable" does not skip recovery: `src/pl-vmi.c:5192` wraps a matching
   recovery in $recover_and_rethrow, and `boot/init.pl:650` defines that as
   call_cleanup(Recover, throw(OriginalException)). Recovery therefore still
   executes. The official manual's unwind-exceptions section in
   `man/builtin.doc:2957` documents this behavior.
4. `boot/messages.pl:665` translates unwind(halt(_)) to an empty message.
   `boot/messages.pl:2169` discards empty message lines. The probe's
   print_message(error, Error) can consequently leave stderr empty even
   though its recovery was reached.
5. Recovery calls halt(1), creating an equally urgent EXCEPT_HALT exception.
   `src/pl-wam.c:866` saves the pending exception while invoking cleanup;
   `src/pl-attvar.c:626` restores the saved exception when its class is >=
   the cleanup's class. Thus rethrowing the original halt0 during cleanup
   can preserve the already pending halt1. The outer query's
   `src/pl-pro.c:184` then calls halt_from_exception with that status.

This is a source-supported explanation, not an observation that the actual
failed process entered recovery. That command has no before-halt/recovery
markers. A whole JSON file also does not independently prove that its writer
and cleanup returned successfully. The JSON writer's object path finishes
with write(Stream, '}') in `packages/json/json.pl:725`; no later object
serializer step was found. A silent logical failure would instead reach
`boot/toplevel.pl:759`'s failed -g branch, which is silent under -q and calls
halt1. Ordinary at_halt hook errors/failures are reported and continue in
`boot/init.pl:4607`; only cancel_halt cancels. Foreign on_halt callbacks return
warnings rather than changing exit status in `src/pl-init.c:1734`. No such
shutdown hook was found in the two actually loaded JSON/PCRE foreign sources,
and actual empty stderr does not establish the precise shutdown branch.

The relevant official/native source files and frozen probe are already pinned
in the actual failed report. Frozen probe SHA is
`af5d3ac1d36c5eade5d8f87df085a9cfa219d295208841a5cbaca89b47373c85`;
prepare_without_main SHA is
`270dcb356288c365b66f0d6a33fb176be9edf3de17ae526874cc9c49021cf9d4`.
This read did not calculate any digest or execute the runtime.

## Prospective minimal control plan

Root/peer review first. A fresh owned preparation-only diagnostic worker must
revalidate the complete latest closed lease/report/input closure under common,
freeze and capture exact pins for this helper/note, and use the same recorded
private ARM64 binary and original prefix. No download, extraction, version,
fixture, image closure, compiler, benchmark or feature operation is repeated.
The helper only writes markers and calls halt. The frozen feature probe is
loaded to preserve the actual library/startup environment; neither its main/0
nor checked/0, nor the compiler helper, is called.

Use one fresh exclusive output directory with distinct event/owned-receipt/
stdout/stderr paths. The parent must require each event path absent before its
command, bind the exact native binary image to the prior actual report, record
PID/birth/argv and hold the common mutex through both terminals and review.
Use the existing finite owned-command service; do not build a fixture system.
An exit1 diagnostic is recorded as the expected control failure, never passed
off as runtime qualification; the service's failure envelope must be preserved.

Let BINARY be the exact already pinned private Contents/MacOS/swipl, PROBE the
frozen revisions/runtime-probe-v1/runtime_probe.pl, DIAGNOSTIC this freshly
reviewed diagnose_halt.pl, and DIRECT_EVENTS/WRAPPED_EVENTS two fresh output
files. Proposed two sequential native argv arrays are:

```
[BINARY, -q, -O, -f, none, --stack-limit=8g,
 -l, PROBE, -l, DIAGNOSTIC,
 -g, clbg_halt_diagnostic:direct, --, DIRECT_EVENTS]
[BINARY, -q, -O, -f, none, --stack-limit=8g,
 -l, PROBE, -l, DIAGNOSTIC,
 -g, clbg_halt_diagnostic:wrapped, --, WRAPPED_EVENTS]
```

Both controls call the same observed_halt body. Direct has no catch recovery.
Wrapped adds the frozen probe's exact print_message+halt1 recovery, with two
sidecar markers. setup_call_catcher_cleanup records the actual unwind reason;
it neither suppresses nor changes the exception. A normal return is marked
unexpected_return and exits3. Any missing marker, timeout, signal, cleanup
error or changed prerequisite stops interpretation; no automatic retry.

Expected direct events are started(direct), before_halt(0), and
cleanup(exception(unwind(halt(0)))), with normal native/final exit0.
Expected wrapped events add recovery(unwind(halt(0))) and
before_recovery_halt(1), with normal native/final exit1. Compare exact parsed
event terms and actual owned terminals in a root-reviewed gate under common;
do not infer success from a file's existence or status labels alone.

If that exact0/1 pair occurs, it confirms the catch recovery's exit-status
replacement in this actual binary. It does not turn the previous failed
feature run into success. A later separately reviewed repair can make the
feature body return to a main that calls halt0 outside its ordinary-error
catch, or restrict recovery to ordinary error exceptions and rethrow unwind
unchanged. One fresh actual feature+image qualification would still be needed
after that source review. If the pair differs, preserve all diagnostics and
trace the observed cleanup/recovery/native exit branch before proposing any
further native attempt. No prospective repair or launch is included here.
