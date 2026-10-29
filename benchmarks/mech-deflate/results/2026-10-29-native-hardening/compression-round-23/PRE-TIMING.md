# Budget correctness and lowering before whole-program timing

Production299c70f4ce21eb3cba94dcb47e4d37106cc14b29. The actual unmodified Core crate root, compiled with the existing selected profile's flags/dependencies and warnings as errors, passes **all349 unit tests** (zero ignored/filtered,0.87s). Four budget tests include the two new tests: every2048 subset of11 limits checked against independently constructed complete and first diagnostics, plus None/equality/u64::MAX/u32::MAX cases. Aggregate admission still excludes call-scoped totals. These tests do not grant new budget authority. Initial new test builds had unsuffixed integer inference failures in the mask closure; the typed u16/usize/u64 witness fixes test setup. Both raw failed builds remain.

A separate optimized **code-generation witness**, using the same newly built Core library, compares the existing complete-diagnostics API with the first-only API for two dynamic limits. The complete API's wrapper retains a304-byte stack frame, an external evaluator call and boxed-result retirement. The new API lowers to six ARM64 instructions, zero stack and zero calls:

```text
mov w8, #7
cmp x1, x3
csel x8, x8, xzr, hi
cmp x0, x2
csinc x0, x8, xzr, ls
ret
```

An equivalent clang -O3 C check emits those exact six instructions. Source,assembly,compiler commands and library hash accompany [the receipts](pre-timing/manifest.json). This illustrates cross-crate specialization and first-error order; it is **not** a compressor timing, a claim that every live call has this assembly, or a change to the compression source. Actual full-workload timing/profile follows. Pinned reference load/store/bounds excerpts were reexamined separately; hashed reference symbols are not relabeled as named source functions.

The existing complete evaluator and the first-only sink share the same ordered checks, so diagnostics do not drift between authorities. Eight hot selected call/context sites now use the first-only API where they already discarded later diagnostics. Current arguments,shapes,accounts,owners,work debits,call limits,publication andcleanup remain. IR/work/ABI/bytecode125/93/36/14 remain. Full component/scalar/architecture qualification follows the frozen production build.
