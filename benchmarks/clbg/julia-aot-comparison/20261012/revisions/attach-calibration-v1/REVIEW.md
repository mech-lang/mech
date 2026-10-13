# Generic attach calibration candidate

Source only, unexecuted, unqualified. This packet compiles one tiny authored C
helper and observes two fresh parent-owned fork children. It runs no Julia,
canary or benchmark, makes no permission changes, and supplies no AOT or chart
admission. Root owns the single launch after source review and the actual
read-only environment diagnostic report/digest review.

The actual read-only diagnostic is now completed at
`jobs/attach-environment-v1/report.json`, root-supplied SHA256
`aa4d725c15347daecf75be5f8110c125f6e1759ed9e2d8d1cd868ae833470e94`.
All twelve queries exited zero with their own groups absent. Developer mode
was reported disabled; the current user belongs to admin and `_developer`.
LLDB/framework/debugserver have Apple signatures; debugserver emitted private
debugger/exception-port/thread-state entitlements. Canary and current Python
have ad hoc signatures, without hardened-runtime flags in the saved metadata;
their entitlements query stdout is empty with a deprecated-colon warning.
These are observed current facts. They do not establish the historical attach
denial, its cause, or historical Python image identity.

The preserved v2 failure reached the pre-attach setup artifacts but timed out
before a successful attach receipt. Its pre-exec child was still Python; its
historical Python kernel image identity was not recorded. Nothing in this
candidate retroactively proves that image, a permission denial, or an observer
sequence defect.

## Exact two cells

Both cells call `SBTarget.AttachToProcessWithID` for a newly owned direct fork
child that has created its own session/process group and stopped with SIGSTOP
before exec. The default cell creates the documented empty target with
`CreateTarget('')` and passes an invalid `SBListener()` sentinel; process events
are read from the debugger's default listener. It sets skip-prologue and the
two symbolic hooks after successful attach, before continuing the stopped
child. The second cell selects the generic helper image, sets skip-prologue
and creates those same hooks before attach, then passes a valid named listener,
matching the ordering/API used by the former observer. This is one comparison
of the combined setup choices, not a factorial experiment or proof that any
single choice caused the historical failure.

The generic target writes a flushed kernel identity marker, calls the
non-inlined `calibration_probe(41)` and exits zero. No target expression is
evaluated. Successful reconciliation requires both actual helper `main` and
`calibration_probe` stops, exact helper module path, zero native exit, the
parent's successful reap/no remaining live group, and a complete post-exec
identity marker. Failed cells retain their raw bytes and bounded failure. The
second cell continues only after an ordinary attach/CLI failure with complete
cleanup; integrity, environment or ownership failures stop the packet.

## Image, birth and ownership evidence

Before attach, an independently owned short helper query uses the shipped
libproc SDK interfaces on the exact stopped Python PID. It captures the
kernel executable path, PID/PPID/PGID and full start-time seconds/microseconds
pair. The pre-exec path must resolve to this actual worker's Python image,
which is separately pinned. After exec, the target itself reads the same
fields. PID, group and birth pair must be unchanged while the executable
path becomes the newly compiled helper. The pre-exec PPID must be this worker.
Post-exec PPID is recorded separately: Darwin ptrace attach can temporarily
reparent the target to debugserver. A changed PPID is not assigned an invented
tracer identity or misclassified as PID reuse. Missing or partial kernel records
stop without substituting a guessed image.

The original `native-observer-v2/suspended_child.py` is freshly loaded from its
actual preserved failure-report pin, without editing its source. It retains
POSIX ownership and a promoted common-lock descriptor across fork/exec. Each
cell journals its independent parent cleanup. An exception in that original
bounded cleanup permits only a bounded SIGKILL/reap after an actual direct-child
wait proves the exact fork child is still unreaped. ECHILD never grants signal
authority. Any cleanup failure remains fatal even if fallback succeeds; failure
to regain/reap the original child after tracing stays unresolved.
Before the original close, a five-second WNOHANG poll can accommodate Darwin's
parent restoration after the owned LLDB command ends. ECHILD during that poll
never causes a signal; actual successful wait ownership/reap is mandatory.

The C3 private-v1 `owned_command` function body is reused unchanged only after
its actual eleven ownership fixtures and original source pins pass. Its old
ps-based identity/group dependencies have no query timeout. The local argument
adapter registers the exact returned Popen object, uses kernel PID/PGID
identity, and supplies new bounded owned ps queries with raw output/receipts.
The adapter copies the function's globals privately; no module/global patch is
made. Group signals require its exact registered leader still live/unreaped;
a remaining group after the leader was reaped is fatal and never signalled by
a stale PID. The metadata ps query also has its own new session, inherited
mutex, bounded cleanup and failure journal.

One common mutex is held before hashing/importing the service, reading
prerequisites, compiling, attaching, and through final reap/evidence pinning.
The held descriptor device/inode must match the named lock immediately after
flock. Both direct targets and all owned CLI/metadata children inherit actual
lock descriptors. Parent SIGINT/SIGTERM/SIGHUP handlers unwind into cleanup.
The compiler has a 60-second command bound, architecture/identity commands
10 seconds, each ps query 3 seconds with bounded cleanup, and each LLDB CLI
45 seconds. Its inner event loop is 20 seconds and at most 32 stops; the outer
bound still applies if the attach or debugger cleanup API blocks. There are
exactly two cells and no automatic retry.

## Inputs and terminal closure

Required actual input is the root-reviewed completed
`attach-environment-diagnostic-v1` report plus its actual SHA256. Its preserved
v1/v2 failure reports, original LLDB executable, original suspended-child
source, saved LLDB Python root, current framework/Python images and read-only
environment bytes are reconciled before execution. Newly captured framework,
SDK-header and helper bytes have current provenance; they are not invented
historical pins. This helper uses the C3 proposal's saved SDK discovery path
and actual Clang22 path, compiled explicitly for ARM64 with `-O0 -g`.
Actual architecture is checked independently. No compiler version probe,
compiler/bootstrap install, signing change, TCC/Developer Tools change or
benchmark control run is introduced.

The exclusive job directory preserves compile/architecture/identity/LLDB
receipts, pre-attach/entered/returned boundaries, event bytes, helper stdout,
parent cleanup and each cell result. Terminal pinning continues across every
available path, retains partial pins/per-path errors, and fails on integrity
failure. Generic success does not imply Julia attach or native AOT capability.
The LLDB CLI freshly executes the verified observer source bytes through its
embedded Python; it does not import a possibly cached observer bytecode file.

## Proposed root command

From repository root, after actual diagnostic receipt review and this packet's
source review/freeze:

```sh
/opt/homebrew/opt/python@3.14/bin/python3.14 -B -u benchmarks/clbg/julia-aot-comparison/20261012/revisions/attach-calibration-v1/calibrate.py --revision attach-calibration-v1 --environment-report /Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/julia-aot-comparison/20261012/jobs/attach-environment-v1/report.json --environment-report-sha256 aa4d725c15347daecf75be5f8110c125f6e1759ed9e2d8d1cd868ae833470e94
```

The actual saved report and root-supplied digest are explicit prerequisites.
Root should launch this command once through its durable owned-job launcher.
No synthetic fixture suite, Julia observer rerun, source counting or timing
command is part of this packet.

Reviewer checklist: verify both target/listener sequences and scope of any
interpretation; exact original-source and current-image provenance; kernel
birth/exec reconciliation; exact Popen/direct-child ownership through failures;
bounded ps/LLDB cleanup; inherited mutex and inode check; and incremental
terminal pin closure. All execution/qualification flags remain false until
actual receipts exist; Julia AOT/chart flags remain false even then.

## Primary API references

- [LLDB SBDebugger](https://lldb.llvm.org/python_api/lldb.SBDebugger.html)
  explicitly shows empty `CreateTarget('')` for PID/name attach.
- [LLDB SBTarget AttachToProcessWithID](https://lldb.llvm.org/python_api/lldb.SBTarget.html#lldb.SBTarget.AttachToProcessWithID)
  documents the invalid-listener sentinel versus a supplied listener.
- [LLDB SBAttachInfo](https://lldb.llvm.org/python_api/lldb.SBAttachInfo.html#lldb.SBAttachInfo.SetListener)
  independently describes default debugger listener behavior.
- [SBListener implementation](https://raw.githubusercontent.com/llvm/llvm-project/main/lldb/source/API/SBListener.cpp)
  supports positive bounded event waits. These current upstream references
  explain the API; the preserved actual LLDB image/package remains measured
  evidence for this calibration.
- Apple [libproc.h](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/libsyscall/wrappers/libproc/libproc.h),
  [proc_info.h](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/sys/proc_info.h)
  and [libproc.c](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/libsyscall/wrappers/libproc/libproc.c)
  define the shipped identity interfaces. Apple calls them private/changeable;
  the actual selected SDK headers are pinned and successful full-size returns
  are mandatory. No undocumented task-port or permission operation is used.
- [XNU fork/exec implementation](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/kern/kern_fork.c)
  preserves the start-time pair across exec. Actual local kernel identity
  observations are mandatory, rather than treating this source as a receipt.
- [LLDB debugserver attach](https://raw.githubusercontent.com/llvm/llvm-project/main/lldb/tools/debugserver/source/MacOSX/MachProcess.mm)
  uses PT_ATTACHEXC; [XNU ptrace handling](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/kern/mach_process.c)
  records the original parent and reparents to the tracer. The kernel PPID
  transition is retained separately from PID/birth/group identity.
- [XNU exit/reap handling](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/kern/kern_exit.c)
  restores/wakes the original parent during traced-child reap or tracer exit.
  Actual successful parent wait/reap remains required here.
