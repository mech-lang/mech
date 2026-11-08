# Shared call metadata and checked count updates

IR285 composes IR283 immutable call metadata sharing and borrowed operation identity validation with IR284 same-cell checked increment proofs, including fresh local proof scopes after growth. It retains work model 116, runtime ABI 41, and image version 11. No measured speedup is claimed before a matched composition cohort.

The latest upstream DEFLATE branch at 9e38c3346fd856b9b0d45824af21002640011676 adds only a stronger child-funding test and documentation after the inherited production implementation; the complete fallible-chunk offset test is carried here. Production code is unchanged by that upstream update.

Finite resource admission, checked overflow and bounds failures, private first-change ownership, and publication behavior remain at their original visits. The direct scalar entry and Cranelift remain the maintained production path. Optional LLVM checks are retained.
