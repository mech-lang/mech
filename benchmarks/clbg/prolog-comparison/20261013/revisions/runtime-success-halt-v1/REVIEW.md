# Successful Prolog shutdown outside error recovery

This additive six-file source packet is mutable, unexecuted, unhashed and
unfrozen. The four Prolog files, controller and this note are released for root/peer source
review. No fixture, native command, version query, compiler, benchmark, hash,
Git operation, source sealing or dataset/UI change occurred while authoring it.
All original drafts, frozen revisions, failed reports and diagnostic receipts
remain unchanged. The all133 diagnosis packet is unchanged as well.

Root's actual closed review is
`continuation/jobs/prolog-halt-controls-closed-review-v1/review.json`. It verifies
13,233 pins and the exact two native controls: direct normal0 with three markers;
wrapped normal1 with five markers including recovery(unwind(halt(0))) followed
by before_recovery_halt(1). Both native streams are empty. Report SHA is
`d43a5f0845b2b0ba7b312905265b1edf22071e3b99cbcc7a0d391af6b1bace29`,
body terminal
`5dd615c6f9e6af4f773f1223a570c0217f3c5a5942524d6c3cf7f7626b69aee3`,
durable terminal
`ebe4960eceb30abe9615384f42301468f7cb949aa5b20dfa769b671555fa0742`.
The wrapped owned exit1 remains an expected failed diagnostic receipt. Neither
the control pair nor complete raw feature JSON qualifies the runtime or relabels
the previous failed native attempts as successful.

The confirmed recovery behavior affects the runtime probe, QLF preparation
entry, and clbg_io's authored application wrapper. All twenty source candidates
invoke clbg_io:run from their main initialization. Each contains the same
success-halt inside a catch region as the diagnosed control. The revision moves
that successful halt after the catch. Ordinary errors retain their existing
print/error/halt1 recovery; a failed checked goal cannot reach the success halt.

## Exact source scope

| New file | Independently recorded original | Exact edit |
| --- | --- | --- |
| tools/runtime_probe.pl | revisions/runtime-probe-v1/runtime_probe.pl, SHA af5d3ac1d36c5eade5d8f87df085a9cfa219d295208841a5cbaca89b47373c85 | checked ends after JSON cleanup; main catches checked then calls halt0 |
| tools/prepare_qlf.pl | drafts/private-v1/tools/prepare_qlf.pl, SHA 1f3a7a79f97bca961b04b504cc339466223470252697ce8aacf6e9e94600eb0c | checked ends after receipt cleanup; main catches checked then calls halt0 |
| tools/prepare_without_main.pl | revisions/runtime-probe-v1/prepare_without_main.pl, SHA 270dcb356288c365b66f0d6a33fb176be9edf3de17ae526874cc9c49021cf9d4 | Same success-halt relocation as QLF entry; prior main-initializer neutralization remains |
| support/clbg_io.pl | drafts/private-v1/support/clbg_io.pl, SHA 29a83894f0acded722fc0eda3244339413f0b7ad04189f6724c95b387cbc8290 | run_checked flushes after Goal succeeds; run catches it then calls halt0 |

Original digests above are read from the actual diagnostic report's input pins;
no new digest calculation supplied these bindings. The two preparation bodies
remain identical except their single startup-initializer directive/comment.
The runtime probe imports the adjacent prepare_without_main, retaining the
previous isolation of the incompatible five-argument compiler initializer from
the one-argument feature CLI. QLF preparation retains its actual compiler main
initialization. No benchmark main, library import, algorithm, primitive test,
source/foreign inventory, WAM writer, output field, argument validation, runtime
flag or compilation option changes.

The exact two replacements for each checked/main file are:

```
close(Out)),
    halt(0).
```

becomes `close(Out)).`, and the original one-line main becomes:

```
main :-
    catch(checked, Error, (print_message(error, Error), halt(1))),
    halt(0).
```

For clbg_io, catch(run_checked(Goal), Error, report_error(Error)) is followed
by halt0 in run/1. The successful branch of run_checked/1 ends at
flush_output(user_output). Its octet encodings, call(Goal) if-then-else,
application_goal_failed exception, report_error/1 and size/1 are exact.
Differences were inspected as source only; no Prolog parser was executed.

## Required continuation contract

Root must first capture exact new source/review pins under common and freeze
the accepted packet. A future bounded worker must consume the actual closed
13,233-pin control review and original failed feature/runtime closures, verify
the complete input/source/native-image identity and exact source derivations
above, and require fresh exclusive outputs with the existing owned-command
service. It must retain native0/final0 as mandatory for a repaired feature probe
and every later compiler/preparation command. Exit1/raw JSON remains failure.
The normal0/1 controls are already complete and need no repeat. No version,
extraction, download, mount, copy, slice or passed synthetic fixture replay is
needed merely to inspect or bind this repair.

The next native phase may use tools/runtime_probe.pl and its adjacent repaired
mainless helper to close the still missing feature and native-image evidence,
after separate root execution authorization. The new loaded helper paths must
be explicitly bound in that evidence; old helper self-pins cannot stand in for
their independently derived new bodies. Runtime discovery continues to leave
source/library semantics and scored workloads pending.

Before compiler/application preparation, a separately reviewed fresh copied
authored tree must preserve every original workload/helper byte except the
explicit support/clbg_io replacement, retaining the existing relative imports.
The repaired tools/prepare_qlf entry is the compiler CLI for that new tree.
The new support file is counted authored application code in every complete
bundle that includes it; its SHA and compiler-input/LOC receipt must reflect the
actual repaired bytes. Original profile helpers and algorithms stay retained;
old complete-bundle pins cannot be relabelled as this new bundle. No existing
compiler/scored output is produced or admitted by this source packet. Future
small semantic/QLF evidence and full-workload timing admission remain separate
root-coordinated stages. There is no new fixture framework or launch controller
in the Prolog source repair.

Root added discover_runtime.py to carry out the next native phase with the
existing owned-command service. It binds the actual closed two-control result,
verifies the unchanged 13,233 input pins and exact four source derivations,
then runs one corrected feature probe with mandatory integer exit 0. Only after
its primitive and loaded-source checks pass does the existing native-image
inspector inspect actual private images. Original successful version, extraction,
slice, controls and fixtures are reused without execution. The command accepts
a separately hashed six-file packet map and closed-review digest. New outputs
are exclusive under .build/swi-runtime-success-halt-v1. No benchmark executes;
runtime/source review and preparation of the repaired authored tree still follow.
