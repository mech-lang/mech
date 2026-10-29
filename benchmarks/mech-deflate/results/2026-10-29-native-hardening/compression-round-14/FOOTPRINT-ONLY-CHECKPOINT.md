# Isolate the footprint/accounting change

Base: `aa93a9e3fbba56c54eb8c48648570d54c8da7500`, which retains the initial combined timing.

This commit restores the unchanged original `FrameShapeRecipe` and always runs its constructor-based validation after successful footprint measurement. The footprint preview and exact batched debit are otherwise the same as the measured combined implementation. Supported simple frame-shape eligibility stays the same. Shapes are no longer part of the shortcut; invalid shape relations still follow all admitted field-footprint work and precede output checks.

The optimized selected-operation release build passed. All 12 focused frame tests pass, and the same 120 observations again agree exactly with the baseline, including both artifact forms, real table sizes, work prefixes and error classes. All twelve static architecture/catalog/inventory/warning/format/unsafe contracts pass. The next protected campaign will time three immutable executables with a fresh reference: round13, combined admission+shape, and footprint-only. All use identical source, bytecode, allocator, input and evaluation boundary.

The new brief also requests existing schema-proof reuse and fully prepared fixed scalar footprints. That refactor is a remaining substep; the current isolated implementation still uses Core's bounded DATA visitor for scalar fields. We will not relabel this as the completed prepared-scalar design. The current recipe already avoids runtime schema-support traversal and combines footprint accounting while preserving denied paths. No ABI, IR, bytecode or work-price change is made.
