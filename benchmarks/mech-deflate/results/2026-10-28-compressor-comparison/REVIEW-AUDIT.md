# Comparison allocator inventory repair

The strict boundary audit at `f6234366963833c4e4a6be9d37f2ef4c6dc429d0` exposed an introduced omission: the standalone comparison allocator wrapper was absent from the exact inventory. Register that one existing file with its actual package owner and pinned allocator/lifetime obligations. No measured Rust or reference source changes; the recorded source hashes and executable measurements remain applicable.

The strict audit now gets past this new wrapper, then fails on an inherited missing declaration in `hosts/gpu/tests/support/canonical_program_buffer_frames.rs:326`. A clean projection of every tracked Rust file and the original audit/inventory at parent `a82d50823e4140edd862886b0c6b5f813a81411c` reproduces the same failure. The unchanged parent inventory contains 2010 Rust files. Both failure receipts are preserved. The parent problem belongs to the earlier frame-transport tests and will be repaired separately on top of the stack. The complete strict audit is not claimed passing here.

This inventory-only repair is separate from outstanding Huffman and checked-tuple review findings. CI remains canceled, and a fresh review of this commit is required.
