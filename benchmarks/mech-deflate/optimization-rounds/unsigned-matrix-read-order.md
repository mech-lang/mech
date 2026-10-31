# Unsigned matrix read review repair

Review of `88b9b89a3a425c9842e1e3cbf33bd914c6f48ebe` found that selected
unsigned reads use a column-major linear ordinal as a direct row-major storage
offset. Minimal reproducer: `xs := values<[u8]:2,3>` followed by
`xs[position<u64>]`, with canonical cells `[1,2,3,4,5,6]` and position 2.
The maintained resident result is 4; the selected native result is 2.
This is an existing fast-path correctness defect, not a new source feature.
The compressor's row buffers have identical linear/storage offsets.

Retain the existing selection, admission, index error and publication paths.
After the maintained scalar index check, reuse the numeric owner's current
matrix-dimension validation and translate the ordinal to canonical storage.
Apply that single translation before all supported unsigned storage branches.
Preserve logical charges, finite limits and zero selector/result cloning.

Correct the existing U64 test to compare against the resident oracle and an
independent literal column-order expectation. Add an original/decoded test for
U8/U16/U32/U64, direct and function-local reads, and repeated current shapes
2x3, 3x2, 1x6, 6x1, 2x2, 0x3 and 3x0. Include invalid/zero/oversized selectors,
unchanged prior publication and aliases, cleanup and successful recovery.
Require the corrected tests to fail before changing production code.

This follows the completed owned-template measurement. Retain its executable
`6a9080cdaa8943c370253555ca33a39e2d001cceb5cc7052cbee875b4604790f`
as the before subject. After the repair, repeat the unchanged eight-byte
comparison, refresh 20000 reference samples and report both subjects with
Roc = 1x. Keep the same catalog, harness, artifact, allocator, resource work
941491317987, capacities, output checks and timing boundary. Run protected
scalar controls and existing component qualification without changing their
allowances. Profile separately after primary timing. No larger input is run.
