# Exact parent-frame field counts without compiler-vector reconstruction

IR255 builds on the unselected IR253 parent-frame borrowing experiment. A fresh
post-JSON allocation trace matches both benchmark resource cohorts exactly:
IR252 uses 3,758 requests/8,012,356B; IR253 uses 3,824/8,014,468B. All 66 added
requests/2,112B occur under `begin_numeric_segment`, where an element-limit
predicate reads `frame.fields.len()` and materializes the compiler-only vectors.

Each frame now retains its exact field count as a runtime word. Owned frames
derive it from the existing vector; compiled frames count the same structurally
validated record items without constructing a vector. This is not inferred from
array-lane count, maximum arity or a serialized count claim. Wire vocabulary,
independent derivation and genuine same-Object authority remain unchanged.

The original finite element-limit comparison reads this word. No limit,
admission, current-owner check, error order or work debit is removed. Compiler
field/path vectors remain available for actual inspection, serialization and
required exits. Tests exercise sparse/empty counts in JSON and native records,
exact images and genuine original/decoded parent-frame lanes without late
materialization. Existing finite denials, publication and retry checks remain.

Native kernels should remain byte-identical, since this changes only restoration
and runtime metadata access. New benchmark/resource cohorts must establish
whether the avoided allocations improve latency; neither count savings nor
the adverse IR253 trial proves a speed gain.
