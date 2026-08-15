# Gate F0 handoff: evidence-only final qualification

F0 begins from the exact final E4 head. It owns evidence refresh and release
qualification only; it is not another implementation or cleanup phase.

## Authorized scope

F0 must:

1. refresh controlled Gate B evidence against the exact final stack;
2. refresh controlled Gate D evidence where required;
3. remove the controlled stale-evidence allowance from CI;
4. run the strict value-system checker with zero findings;
5. pin Chrome, ChromeDriver, wasm-pack, Rust, and every other qualification tool;
6. run final release, distribution, and retained-product qualification;
7. produce final zero-interpreter and zero-fallback reachability evidence; and
8. record the exact final SHA and immutable evidence manifests.

## Explicitly out of scope

F0 must not contain runtime, compiler, host, or bytecode fixes; executor
deletion; migration cleanup; or known product failures. Any such finding means
E4 is not complete and must be corrected on PR #762 before F0 begins.

The private semantic compiler may continue using `Interpreter` and
`LegacyValue` as ephemeral planning coordinates. Replacing those internals with
a direct AST-to-artifact compiler is future, non-gating compiler modernization.
