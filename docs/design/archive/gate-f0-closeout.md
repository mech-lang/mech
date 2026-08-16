# Gate F0 closeout

F0 qualified the v0.4 resident executor product tree at:

```text
39125894b400e2960b6ecf188682d4289435e629
```

The executor architecture and retained v0.4 product behavior were accepted with no known release-blocking correctness finding. F0 did not authorize further executor deletion, a direct-AST compiler rewrite, or removal of the private `semantic-compiler` planning interpreter.

## Final validation

Both final validation tiers passed on exact commit `30f329cb9df90cc6f93072ee21ac4eb3e34dbf23`:

- Selected CI: [run 31960975736](https://github.com/mech-lang/mech/actions/runs/31960975736), 20 jobs passed.
- Full CI: [run 31961306024](https://github.com/mech-lang/mech/actions/runs/31961306024), 71 jobs passed.

The full run covered the permanent architecture contracts, language and bytecode suites, runtime and host suites, browser/WASM, native generation, Windows and macOS packaging, distribution contracts, and compiler/runtime feature boundaries.

## Supplementary performance evidence

The final controlled attempt was [run 31959700947](https://github.com/mech-lang/mech/actions/runs/31959700947) at commit `7085b06cb89e7c8406ea32bec9d1d32e170619f3`. Preconditioning and environment checks passed, but Chain 1 stopped before B2 measurement because the qualification harness invocation failed to compile with missing `MechFunctionCompiler`, `BytecodeCompilerContext`, and `Register` imports. No canonical B2→D2→D3 evidence was produced. The retained session digest was `02493a502990e1c7d6aa6cebd348581d927cfed162a2660827fcb913b98a0171`.

The v0.4 closeout policy therefore treats historical and absolute performance comparisons as advisory. Measurements remain visible, while semantic correctness, allocation behavior, publication, replay, effects, bounded history and epoch behavior, and permanent reachability contracts remain release-blocking.

Temporary F0 workflows, orchestration, manifests, provenance rules, and product-tree tooling were removed after qualification. Durable benchmark protections remain: exact sample protocols, exact probe identities, isolated NumPy execution, separated fresh and historical D2 streams, isolated Cargo targets, and benchmark correctness checks.

This closeout does not merge or promote the branch. Durable F0 changes must first land on `integration/value-executor-v0.4`; promotion from that integration branch to `main` is a separate release action.
