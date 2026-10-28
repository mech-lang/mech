# Six-position collector completes

Compiler `d93c41c08` fixes the selected unsigned-buffer length path on base
`b7f3c611a5f8d635d9d113e203a7a2fadce3053f`. The exact full SHA and fixture hashes
are in [provenance.json](provenance.json). Source remains pinned at
`671077457ff7ef3a3d44caf715a3fc595addf379`. Artifact revision 14, program ABI 35
and typed IR 107 are unchanged; work policy is 83.

The resource rejection at the previous checkpoint was **ComputeWork**, not a
buffer-memory rejection. The operation trace attributed approximately 19% of
helper work to general length-call binding. The [change](../../unsigned-length/README.md)
removes that binding and its canonical result allocation while retaining the
exact selected operation, validation, logical limits and existing scalar lane.
It does not increase a budget or change accounting coefficients to accept the
collector.

The unchanged six-position test now succeeds for original and decoded artifacts.
Both compare every one of the 262144 U16 tree entries and 1500513 U32 cache words
with the pinned expected state. Search depth 300, nice length 258, 320 input
bytes, 10 billion work units, 256 MiB managed and 128 MiB temporary allowances
are unchanged. Search and skip remain separate; skip has no cache parameter.

| Corrected-header witness | Original | Decoded |
| --- | ---: | ---: |
| Completed positions | 6 | 6 |
| Evaluation seconds | 14.531305291 | 14.535073416 |
| Allocation requests | 617047 | 617046 |
| Cumulative requested bytes | 478106375 | 478106311 |
| Work units | 9793861508 | 9793861508 |
| Comparison units | 357598325 | 357598325 |
| Control visits | 19781 | 19781 |

The Rust test host is unoptimized; generated native code uses Cranelift `speed`.
Compilation and full expected-state comparison are outside evaluation timing.
Requested allocation bytes are cumulative, not live or peak memory. The same
buffers remain published after success; final teardown checks managed usage is
zero, and input aliases remain unchanged. The earlier successful run is also
retained; these are qualification observations, not balanced release samples.

Validation uses locked offline dependencies, one build job,
`nightly-2026-03-03`, and warnings as errors. Commands are preserved verbatim in
the JSON receipts, with full logs:

- [Real-buffer command](selected-length-real-collector.json): two tests passed,
  zero failed/ignored; initializer and six positions on both artifact forms.
- [Corrected header and focused command](selected-length-focused-and-collector.json):
  four passed, one failed. Both collector tests and two new length tests passed.
  The direct-call test's final cleanup assertion ran while its 200-byte work
  account metadata was still live. This introduced test-lifetime mistake was
  fixed by dropping the work account before asserting zero managed bytes.
- [Final focused and linked command](selected-length-final-focused.json): four
  passed, zero failed/ignored, 257 filtered out; 17.63 seconds build and 86.34
  seconds execution. All three new length tests pass. The existing Object-based
  constructor/length consumer also builds and runs with only runtime features,
  no parser/JIT, and `-D warnings`.
- `cargo fmt --all --check` and `git diff --check`: passed.

No broad component rerun, feature-profile matrix, CI, or release timing was
started at this checkpoint. The constructor checkpoint's prior evidence is
retained separately. The temporary work profiler was removed before the patch;
only its [summary](operation-work-summary.json) is archived here.

There is still no complete compressor or compression time. Permanent executing
functions remain the matchfinder, shared extension loops, initialization/rebase,
separate binary-tree search/skip, and consecutive-position prefix collector.
The next source work is the actual block collector: statistics before cache
headers, complete skip before block decisions, rewind and carryover. Checked
U32 frequency merging must receive its minimal reproducer and acceptance test
before compiler work expands. Retained-capacity U64/U8 operations, the optimal
parser, block writer, full Huffman integration and `compress_into` remain.
