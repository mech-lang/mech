# Verification after a no-op phi sweep

The count-only compiler uses the archived shared-owner native trace build.
It exposes the existing constant-phi solver log and verifies the exact source,
bytecode, encoder object, and metadata. The coordinator pass order matches
the archived timed trace. The counts below cover adjacent prune, verify,
constant-phi removal, and verify spans. The elapsed column uses the archived
second verifier spans, including their descendants.
[Counts and matching trace indices](report.json)

| Coordinator scope | Adjacent pairs | Phi sweeps with no edits | Second verification after those sweeps |
| --- | ---: | ---: | ---: |
| Checked root | 20 | 8 | 29.586 ms |
| Numeric phase | 136 | 80 | 5.515 ms |
| Cranelift optimization | 59 | 46 | 3.585 ms |
| Object coordinator | 2 | 0 | 0 ms |

A phi sweep with zero constant formals leaves the function, aliases, branch
arguments, CFG, dominators and instruction order intact. A potential change
can retain the first verification in an adjacent pair and verify again after
a sweep that edits any formals. Keep the standalone pass entry and the first
verification of each pair available for all callers. Confirm the conditions
at each call site before treating a counted adjacent span as reusable.
[Constant-phi transformation](https://github.com/mech-lang/mech/blob/ff7c1dbc9c5059ed84d730b008847c02179fa2cf/vendor/cranelift-codegen/src/remove_constant_phis.rs#L330-L409)
· [Context verification](https://github.com/mech-lang/mech/blob/ff7c1dbc9c5059ed84d730b008847c02179fa2cf/vendor/cranelift-codegen/src/context.rs#L258-L284)

The 38.686 ms coordinator total is an opportunity estimate from prior traced
elapsed time. Verify the changed and unchanged pass cases and the exact
emitted worker before measuring a source change in the ordinary compiler.
[Census command and inputs](build.json)
· [Count log](compile.log.gz) · [Diagnostic trace](native-exclusive.json.gz)
