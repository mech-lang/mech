# Initial combined admission/shape timing

Measured compiler: `c8b87b305` (full SHA recorded in primary results). Paired baseline is the immutable round-13 executable at production compiler `1ec498a67fc862bac39188fdf5040eaacf6e954c`, binary SHA `59c15cf5021fa2947779334c0e4f8c17f1b393c3ca5d5e3dc955444f38e89bae`.

Roc = 1×. Fresh reference: 0.000290000 seconds, 20000 samples. Eight-byte compression before: 0.072561708 seconds = **250.212786×**. After: 0.061835750 seconds = **213.226724×**. After/before: 0.852181567; **14.782% observed reduction**. Three primary observations each, warm-up and diagnostic observations excluded. These are scoped observations, not a confidence interval or larger-input throughput.

Every timed/diagnostic observation verifies exact ten-byte raw output, independent inflation, retained capacity13, and final zero bytes/nodes. `same_backing=false` remains true before and after. The evaluate_into timer still includes the boundary transport/copy costs. Both executables charge the unchanged complete logical work 527732164684; resource policy, input, pinned source/bytecode, allocator and timing boundaries are unchanged.

This initial patch includes BOTH footprint/accounting specialization and direct current-shape checks. The new user brief arrived while this protected measurement was already in flight and requests that those steps be attributed separately. This result is preserved, not relabeled as footprint-only. The next change will isolate the footprint-only implementation and reuse the established footprint-proof owner more fully before considering direct shape checks again.

Twelve focused frame tests passed; 120 baseline/changed admission cases match exactly. Full component/scalar/linked qualification is pending for the eventual final implementation. The initial release build's unresolved Core-error import and both corrected baseline-fixture mistakes are retained as introduced development failures, not outstanding compiler failures. The historical round-10 scalar ratio 1.0526036 against 1.05 remains unresolved. Current-head review and CI are not claimed complete.

The separate diagnostic sampling run has 65 repetitions, all excluded from the primary measurement. Its original sample and parsed inclusive summary are retained. Inclusive categories overlap. Passive reference ARM64 evidence is retained without inferring a source helper from hashed symbols.
