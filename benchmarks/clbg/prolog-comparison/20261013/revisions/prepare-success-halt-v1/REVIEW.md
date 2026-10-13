# Prolog preparation using the qualified success-halt runtime

This is a new, unexecuted 27-file source packet. It derives a new metadata-only
proposal and then, only after review of that actual proposal, performs the
existing serial 20-profile QLF preparation and 156 bounded output checks.
Nothing here admits LOC, workload semantics, full timings or chart data.

The controller is copied from `drafts/private-v1/guarded_prepare.py`. Its six
contract methods already passed in `proposal-seals/private-v1`; the actual
eleven ownership methods belong to the unchanged C3 generic command/probe
service. Neither suite is rerun. The retained receipts have the exact original
method IDs, normal successful terminal evidence, actual source pins and no
skips/errors. This continuation does not claim those tests executed against
the modified controller. Root and an independent source reviewer must review
the adaptation and the exact later commands before execution.

All 20 `sources/*/{compact,optimized}.pl` files, `support/clbg_records.pl`,
`contracts.py` and `profiles.json` are byte copies of the original pinned
packet. The two repaired copies are exactly
`revisions/runtime-success-halt-v1/support/clbg_io.pl` (recorded SHA
`bc6361457988de3b8d7dbce526bf738f0268e96e1c7bb59c35bb4798c71ebdb3`)
and `tools/prepare_qlf.pl` (recorded SHA
`1969d7d1cdb6ac33ffb0135ac9c5335c9b5aaf18b16f75f345e449a495ec147e`).
The only source derivation in those repairs moved successful `halt(0)` outside
the error-catching region; error handlers still exit 1. The QLF tool retains
its original five-argument CLI and initialization directive. Its standalone
main is used; the probe's mainless imported helper is not a compiler entry.
The complete new authored IO helper remains part of every scored source
bundle. The optimized files remain explicit source-identical aliases.

The literal runtime configuration is
`runtime-qualified/success-halt-v1/runtime-config.json`, SHA
`7fa470a9827e60e84eb9c6ad343cc8385e411b400f0ae8a322fbefda488bb448`.
The literal root source/library review is SHA
`eab625231fd424e78de80132566ca3c7fe1bb91b096b50874c0a515271bd6486`.
The actual runtime report `d744e831...71404b`, body terminal, closed durable
terminal, root closed review `dca140a6...2317be1f8`, all 13,399 original pins,
actual feature exit 0 and seven private ARM64 images/35 owned image commands
are rechecked. The same native image closure is reused without executing any
image command, version command, feature probe, download, extraction, mount,
copy, slice or passed fixture again. Every foreign plugin actually observed
in each new compiler inventory must resolve to one of those pinned images.

The controller executes as its own real module in the new packet. Its original
cached-module policy still rejects preloaded benchmark modules outside that
packet. It cold-loads the original pinned generic service/references, then
checks every original guard/harness module path and digest. It does not load
the old Prolog controller, replace `__file__`, spoof old source identities, or
broaden the allowed cached paths. The new contracts module is the verified
byte-identical copy. The common mutex is acquired once and retained throughout
each process; both successful old durable owner leases are separately acquired
nonblocking and held through final pin collection. Exact retained request,
launch, worker-started, command-started and terminal records bind the original
argv/cwd, normal exit 0 and unchanged worker/command identities. The launch
worker parent is allowed to change at daemon detachment; its PID, birth and
complete argv remain exact.

Root must freeze all 27 source files and capture a read-only packet receipt
whose `files` map has exactly the controller's `FILES` relative names. Its
`retained_receipt_pins` must contain exactly these eleven absolute paths:
the old `proposal-seals/private-v1/terminal-pin.json`, plus `request.json`,
`launch.json`, `worker-started.json`, `command-started.json` and `terminal.json`
from each of `continuation/owned-jobs/prolog-private-v1-seal` and
`continuation/owned-jobs/prolog-runtime-success-halt-v1`. Each map value is a
fresh actual SHA captured under the common mutex. No future digest is asserted
in this source note.

The first later command is the isolated, bytecode-disabled bundled Python
entry `-I -S -B -u revisions/prepare-success-halt-v1/guarded_prepare.py --seal
--packet-pins ABSOLUTE_CAPTURE --packet-pins-sha256 ACTUAL_CAPTURE_SHA`.
It writes only the fresh `proposal-seals/prepare-success-halt-v1/` and
`proposals/prepare-success-halt-v1.json`, verifies the original 20 rows and
156-case inventory, and records zero native/fixture/runtime reexecutions.
After that actual closure is reviewed, the same entry with `--prepare` and
`--proposal-sha256 ACTUAL_NEW_PROPOSAL_SHA` writes the exclusive
`qualification/prepare-success-halt-v1/` tree.

The native phase retains the original per-helper/main QLF compile commands,
scored WAM dump, original compiler-bound source inventory classification,
default GC, unbounded GMP integers and uniform 8 GiB resource ceiling. Every
owned compiler must exit natively and finally with integer 0 and clean group
cleanup. QLF application loading stays inside each bounded process. All 156
attempts retain the unchanged twenty qualification gates, complete independent
oracle output checks and raw/supervisor/attempt field equality. Each native
process inherits the common FD and there is one worker. Terminal collection
preserves failed actual bytes and every readable partial pin; it never labels
a stopped attempt successful.

SWI executes the application as bytecode/QLF. Native ARM64 runtime discovery
does not make the application AOT. Full required-work witnesses (especially
tree allocation/retention, seven growable K maps and both Pi divisions),
source-count replay and full 90 measurements remain separate future gates.
Existing runtime failures, successful control tests, old proposals and chart
baseline B088 are retained without edits.
