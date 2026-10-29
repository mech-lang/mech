# Introduced fallback-selection regression

Production e721febbcd557040ce8866e2ebbf61f09a9e4b5b selects a private child beside an opaque canonical record with dynamic dimensions, then rejects preparation with Backend("private composition has unsupported shape relations"). This is an introduced eligibility error. The source must retain the established canonical constructor fallback. The focused regression compiles successfully and fails at JIT preparation, before input construction. It is a correctness repair within nested private-frame composition, not another compressor operation or source rewrite.

The already committed initial full timing remains106.602389×→99.868246×. All12 initial scalar pairs pass≤1.05(max1.010359388), with equal requests/bytes and zero loop allocation slope. These controls are committed before repairing production. The fixed compiler will be measured again; the initial timing is not relabeled as the fixed result.
