# Consolidate prepared footprints in their existing budget owner

Base: `2427adf7b52e9957ea2b50a24e3a52ed872aa731` (published measured footprint-only checkpoint).

`FrameGeometry` still owns the immutable admission recipe. Its field measurements now reuse the existing `BorrowedFootprintSchemaProof` and live in the shared budget/footprint owner. The call-specific module shrinks from 185 to 85 lines. There is no independent schema-support classifier or work-unit constant in the call module.

Closed scalar footprints and their exact existing work are prepared through `measure_native_scalar_frame_footprint` once. Runtime words retain exact bit-range validation and return that fixed logical footprint without a schema/DATA visitor or canonical wrapper. Managed scalar measurements verify the current table entry's cached semantic key. Packed fields retain current storage-kind checks and Core's bounded DATA metadata visitor, including actual retained capacity and current shape-wrapper storage. Runtime does not call `Schema::key`, retain owners or reuse previous arguments.

The original shape constructor remains unchanged. The exact serial success preview/batched debit, original metadata reservations and denied incremental path remain unchanged. Logical prices, typed IR118/work93, ABI35 and bytecode14 remain unchanged. No compressor source, algorithm, allocation site, buffer, search rule, runtime policy or timing boundary changes.

Optimized selected-operation release build passes. The actual Engine test root compiled with warnings denied: 100 selected budget/worker/projection/shape tests pass, including three new differential footprint tests. Twenty scalar-word cases include invalid widths, signed extrema, negative zero and a NaN payload. Twenty current packed-buffer cases cover U8/U16/U32/U64, changing rectangular and zero extents, aliases and actual retained capacities including the real U16/U32 tables. Each footprint and its compute/node debit agrees with the maintained ordinary measurement.

All 12 maintained frame tests pass. Their 144 admission cases include all 120 exact baseline observations plus 24 competing resource/invalid-scalar/output errors. Original/decoded artifacts, current shapes, prior aliases, cleanup and retry remain covered. Full component/scalar/linked qualification is still pending. The next protected campaign will compare this implementation with both retained prior controls, retaining the original shape constructor on both footprint implementations.
