# One explicit-goal diagnostic using retained Prolog QLF

This two-file source-only packet preserves the actual failed native preparation
and performs no action before root/independent source review, freeze and exact
launch. It proposes one new bounded `all-replacements` process using the two
QLF files whose compilation and original scored inventory inspection already
passed. It does not recompile either file or run the remaining profiles/cases.

The actual failed report is `qualification/prepare-success-halt-v1/report.json`,
SHA `1b2e0f5c08c3f2105d6f97046145cb18b905220199852915c9dfd4673719da20`.
Its body terminal is `59d7b133c142f7bb1d4010d739bda779bb6fd033aea222d2aab1c608439dc04c`;
the closed durable terminal is `ba6f8f9f2c10e4bc2f1489441e3d98480e9b9b86caf6bc706879c179abd1721a`.
Root's actual closed review records13,471 verified pins, three successful
compiler/inspection commands and one failed output case. The process exited0
but emitted only a newline and a stdin Prolog syntax error. Neither native0 nor
the successful compiler receipts can qualify that output. All failed raw
sampler/supervisor/report bytes remain unchanged.

The exact source cause is supported by the preserved official SWI10.0.2
`boot/toplevel.pl` SHA
`1e135b02ad3f65c680f5f9307b3740d2189fd4c79b4882986046d7117e67717c`.
`user_thread_init/0` first runs explicit `-g` goals, then checks `load_only/0`.
That predicate notices any `-l` in the process argv and skips main/program
initialization globally. The saved command preloads `clbg_io.qlf` with `-l` and
loads `compact.qlf` with `-s`. Its source entry is exactly
`initialization(clbg_io:run(clbg_regexredux:main), main)`, so the suppressed main
leaves the REPL consuming the DNA input. The
[official loading/running documentation](https://us.swi-prolog.org/pldoc/man?section=compilation)
also describes `-l` as loading without executing main/program initialization.

The sole command change is appending two argv entries:
`-g`, `clbg_io:run(clbg_regexredux:main)`. The fixed source module/main declaration,
QLF paths and hashes, full source/helper bundle, runtime/environment/stack/GC,
binary image, compiler receipts, scored WAM and complete loaded-source inventory
are revalidated first. Explicit `-g` invokes the same authored entry after QLF
loading; the repaired `clbg_io:run/1` flushes output and exits0 outside its error
catch. No workload operation, source text or library is changed. For a future
numeric profile, an explicit goal would have to be inserted before its `--`
application arguments; that broader controller change is not performed here.

The actual helper QLF SHA is
`3b1f870708e6843cecf57e634042f414fbe5c938467ac44e6518d9578fbb48ea`;
main QLF SHA is
`204710c5f202e83ccc41666f5057ccdbec64c4040239a937b126ee77731a6272`.
The existing complete oracle output and stdin bytes are retained and checked
against the unchanged pinned independent regex reference before the new case.
The original C3 owned service/probe, guard/harness and six-method contracts are
cold-loaded from their real reviewed paths. The old preparation controller's
`reused_service` is deliberately not called: its original cache rule rejects a
new entry outside the old packet. This explicit direct service loading is
source reviewed; it does not forge `__file__`, widen that old rule or claim
another fixture execution. All original pinned service and fixture evidence
remain required. The source-bound old controller is
`revisions/prepare-success-halt-v1/guarded_prepare.py`, SHA
`8d7cefd54b0342b563aec6131e8cdb443203309c0719b804a26d991e0c227ecb`.

The diagnostic holds the named common mutex and closed failed owner's private
lease through final pin collection. Its actual request/start/worker/terminal
identities, concrete PID31352/parent31348 argv and original37076a/c369 proposal
bindings remain checked. There is no launcher alias assumption. The unchanged
probe owns one native group, drains its output, enforces the original twenty
qualification gates and complete oracle bytes/hash, and preserves raw versus
supervisor versus attempt identity. The diagnostic additionally requires empty
stderr. Default GC, unbounded integers and the8GiB ceiling remain. Successful
source/compiler/runtime/fixture commands are never repeated.

Only exclusive `qualification/explicit-goal-first-case-v1/` evidence is written.
A success means one corrected startup/output case passed; it does not admit20
profiles,156 cases, required work, canonical LOC, full studies or chart data.
The passed first build/inventory, failed native case and any new diagnostic
failure stay separate. Only after the actual diagnostic closure is independently
reviewed should a later additive preparation controller reuse this first build
and corrected case while doing the remaining unstarted native work.

Root must freeze `diagnose.py` and this note, capture their actual digests and
the actual `continuation/jobs/prolog-prepare-native-closed-review-v1/receipt.json`
digest under common, then review the concrete command:

```text
<bundled Python3.12> -I -S -B -u ABSOLUTE/explicit-goal-v1/diagnose.py
  --worker-sha256 ACTUAL_FROZEN_WORKER_SHA
  --review-sha256 ACTUAL_FROZEN_NOTE_SHA
  --closed-review-sha256 ACTUAL_CLOSED_FAILED_PREPARE_REVIEW_SHA
```

No compiler, parser/counter, fixture, hash, native command, freeze or Git command
was executed during authorship. The new native diagnostic requires its own root
authorization and common-lock scheduling; it is not an automatic retry.
