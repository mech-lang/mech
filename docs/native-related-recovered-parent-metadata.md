# Recovered parent-frame metadata borrowing

The native loader now carries its private compiled-frame reader through static
parent composition records. Genuine bound modules borrow the immutable
compiler-only field and scalar-path records; independent image readers retain
eager ownership and the same complete structural validation. Parent runtime
lanes, outputs, finite admission and publication remain unchanged.

The exact field count is retained during that same checked structural read. The
existing element-limit predicate uses it without lazily constructing compiler
vectors. This composes the earlier IR253 borrowing and completed IR255 consumer
fix with IR280 grouped-key metadata, scalar-home masks and current DEFLATE work.
All explicit grouped-reader record names remain identical.

Identity: IR282, work116, ABI41, image11, Cranelift. No Mech source or algorithm
change. Runtime admission and errors remain checked. Scalar and DEFLATE controls
and both current benchmark sources must qualify before performance measurement.
Whole-process timings remain reported, with no mandatory noninferiority gate.
