# Boolean loop facts in the related-post compiler

IR256 integrates PR973 commit
`b6a42fde80dbc011dfefa9de5d9659be334411b0` on the frozen IR252 related-post
compiler. The same zero-or-one SSA transfer proof feeds selected-edge ranges,
including current loop block arguments. Unsupported entry words, fresh loads,
arithmetic updates and unknown incoming values remain unproved. It grants no
memory permissions and removes no source effects, work debits or error paths.

The integration preserves the related-post compiler's early identities,
Boolean conditions, demand-liveness ablation, extent-proof alias records,
checked source errors and finite admission. Its IR is distinct from both
upstream IR206 and related-post IR252; work115, ABI41 and image9 are unchanged.
The CI inventory includes the three actual upstream execution tests in addition
to the existing 389 names. Both the inventory and every selected test must run.

The separate IR253 parent-frame metadata experiment is not included. It saves
startup allocations but adds 66 late compiler-vector allocations and fails the
joint latency gate; a separate fix targets their exact runtime field-count
consumer. Neither rejected timings nor upstream DEFLATE observations establish
a speed improvement for this integrated compiler.

Qualification and matched Rust benchmarks must identify this exact compiler.
Source variants and timer boundaries remain frozen for its first comparison.

The initial IR254 integration passed the full 62-case Rust edge checks but its
upstream proof-coverage test reported no selected branch fold. Existing direct
Boolean lowering had inserted a low-bit test; selected-edge inversion did not
carry facts back through that proven Boolean identity. IR256 composes the two
passes only when independent SSA ranges prove the original word is zero or one.
Unrestricted odd/even words never inherit a singleton value. The unchanged
upstream assertion and a new four-width rejection/publication test cover this
interaction. Original failed receipts remain; no expected value is weakened.
