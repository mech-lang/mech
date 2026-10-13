# Generic attach image repair, source candidate v2

Mutable source candidate, unexecuted and unqualified. Root must read the full
source and the original v1-to-v2 diff, obtain independent source acceptance,
review the original actual report/closed lease/pin closure under the common
mutex, freeze the accepted source, and authorize one fresh durable job before
execution. No hash, import, parser, fixture, compile, LLDB or native command
was run while preparing this packet. It contains `calibrate.py` and this note;
original v1 source and outputs remain unchanged.

## Exact observed failure

The v1 durable job `continuation/owned-jobs/julia-generic-attach-calibration-v1`
exited 1 normally at 2026-10-13T15:42:51.055303+00:00, with no requested stop
signals. The C helper compilation and ARM64 architecture command both exited
zero before the failure. The helper is at
`jobs/generic-attach-calibration-v1/build/generic-helper`, with its report's
recorded SHA256 `d73d21f1772ecdda84659e82a33761694f09b438e8a6dc7d4517994b72b472b0`.
No helper rebuild or architecture-query repetition is proposed.

The first stopped pre-exec child was PID/PGID 47840, direct parent 18542. The
owned helper query returned the full libproc record, BSD stopped status 4,
birth pair 1791906170/468537, and the exact Homebrew Python 3.14.8 Framework
Python.app executable path. The v1 assertion instead compared that kernel
path with `sys.executable`, which resolves to the distinct bin launcher.
The failure happened before the LLDB command directory/request was created.
No second cell began, native target output is empty, and bounded parent
cleanup reaped the first child with no remaining group. The candidate consumes
these actual receipts and the exact failure line; it does not infer an attach
permission failure.

## Narrow new image contract

The new source consumes root's reviewed concrete host descriptor at
`continuation/jobs/python-framework-host-linkage-v1/host-linkage.json`, exact
SHA256 `9a1c0fa4fc330cf0c7c176349c220640ad4714c5e8e081eaead84c00b89729af`.
It accepts only its literal Homebrew 3.14.8 launcher and Framework Python.app
pair and pins their distinct image bytes:

- Bin launcher: `e401ac7a95cda75dee34a0cfe1e4faf0a6df1a501cf67887f177fb78cd4eb60a`.
- Framework image: `b3e2d3162eb789101425bbe76810fc5eb0d412d7cd5867d27568dd180ccf5a22`.

The launcher symlink target, current `sys.executable` resolved path and current
Python version must match that exact descriptor. The original compiled helper
makes one bounded read-only kernel query of the current calibration parent.
Its observed parent PID/PPID/PGID, birth pair and full-size libproc result are
checked; its kernel image must be the literal Framework image. Each fresh
fork child must be stopped, owned by that same parent, have its own PGID,
and have exactly the parent's actual Framework kernel image. This replaces
only the mistaken launcher-equals-kernel-image comparison. The pre-exec child
birth pair still must match its post-exec helper marker; the exact helper image,
zero target exit, main/probe stops, parent reap and empty remaining group remain
mandatory for success.

The descriptor's Framework digest is a current host capture. The saved v1
child path is historical kernel evidence, but v1 did not pin that Framework
image before failing. This candidate does not retroactively claim a historical
parent kernel image hash or same launcher/app bytes. Official startup source
explains the expected launcher reexec:
https://raw.githubusercontent.com/python/cpython/v3.14.8/Mac/Tools/pythonw.c .
Actual new parent/child kernel observations, rather than that source alone,
are required before either new attach attempt.

## Reused closure and unchanged execution scope

The new CLI requires the root-reviewed original generic report's actual SHA256
and the frozen new worker SHA256. It verifies every original report pin,
the old closed private lease, original durable command/cwd/terminal/start
identity, successful exact compilation/architecture receipts, empty errors,
original child query and cleanup, no old attach directory, and no second cell.
The old failed report and successful build are preserved; existing ownership
fixtures are consumed without rerunning them.

The original `attach-calibration-v1/observe_calibration.py` is freshly loaded
from its actual report pin. The suspended-child source and C3 owned-command
function body remain unchanged. Both original listener/target setup sequences,
exact two cells, finite LLDB/group-query bounds, direct ownership signal rules,
post-exec reconciliation, and terminal partial-pin handling are retained. The
new worker holds the same common mutex before hashing/loading any evidence,
through the read-only parent query and new attach cells, until cleanup and
terminal evidence closure. All children inherit the retained common descriptor.

A successful generic calibration still leaves Julia/canary/AOT/chart admission
false. Permission changes, benchmark runs, canary executions, new generic
compilations and repeated architecture queries are zero. The two required
fresh attach observations were never reached in v1; there is no automatic
retry or repeat of a completed attach experiment.

## Deferred concrete entry

Use the original qualified Homebrew Python 3.14 route and fresh revision
`generic-attach-calibration-v2`, with `--environment-report` pointing to the
unchanged `jobs/attach-environment-v1/report.json` and its exact existing
`--environment-report-sha256`
`aa4d725c15347daecf75be5f8110c125f6e1759ed9e2d8d1cd868ae833470e94`.
The two new required arguments are `--original-report-sha256` and
`--worker-sha256`. Their values must be captured/reviewed under common after
source acceptance; this source-only note intentionally supplies no invented
report or source digest. Suggested fresh durable name is
`julia-generic-attach-calibration-v2`. Root owns launch authorization.
