# Compiler-profile evidence, 2026-11-14

Seven complete evidence packets are tracked here at the user's request. They include raw commands, observations, source snapshots and patches, test logs, benchmark Objects and metadata, rejected attempts, adverse counters and pure offline replay scripts. Compiler executables are excluded and hash-pinned inside each packet. The archives total 70,673,714 bytes (70.7 MB).

The [machine-readable index](index.json) pins every archive SHA-256 and source revision. Each receipt also pins the internal manifest and replay output. All seven repository copies have been extracted, verified and replayed byte-identically against their compact summaries.

| Packet | Decision | Archive | Receipt / result | Files |
| --- | --- | --- | --- | ---: |
| Ownership-join SSA forwarding | Rejected: repeated program slowdowns | [mech-ssa-forwarding-rejected-20261114.tar.gz](mech-ssa-forwarding-rejected-20261114.tar.gz) | [Receipt](../../native-profile/ssa-forwarding-rejected-apple-m3-20261114.archive.json) / [result](../../native-profile/ssa-forwarding-rejected-apple-m3-20261114.json) | 4,114 |
| Reusable visitation storage | Retained: identical program bytes; higher RSS | [mech-thread-visits-20261114.tar.gz](mech-thread-visits-20261114.tar.gz) | [Receipt](../../native-profile/thread-visits-compile-apple-m3-20261114.archive.json) / [result](../../native-profile/thread-visits-compile-apple-m3-20261114.json) | 339 |
| Selected liveness definitions | Retained: identical program bytes; higher RSS | [mech-live-definitions-20261114.tar.gz](mech-live-definitions-20261114.tar.gz) | [Receipt](../../native-profile/live-definitions-compile-apple-m3-20261114.archive.json) / [result](../../native-profile/live-definitions-compile-apple-m3-20261114.json) | 364 |
| Current pipeline memory and integration | Diagnostic: current runtime integration and frontend RSS peak | [mech-current-pipeline-memory-20261114.tar.gz](mech-current-pipeline-memory-20261114.tar.gz) | [Receipt](../../native-profile/current-pipeline-memory-apple-m3-20261114.archive.json) / [result](../../native-profile/current-pipeline-memory-apple-m3-20261114.json) | 64 |
| Borrowed schema finalization | Retained: identical program bytes; lower time, instructions and RSS | [mech-borrowed-schema-20261114.tar.gz](mech-borrowed-schema-20261114.tar.gz) | [Receipt](../../native-profile/borrowed-schema-compile-apple-m3-20261114.archive.json) / [result](../../native-profile/borrowed-schema-compile-apple-m3-20261114.json) | 371 |
| Query-local liveness graphs and backend trials | Retained: indexed liveness graphs; identical current program bytes. Includes two rejected backend prototypes. | [mech-indexed-liveness-20261114.tar.gz](mech-indexed-liveness-20261114.tar.gz) | [Receipt](../../native-profile/indexed-liveness-compile-apple-m3-20261114.archive.json) / [result](../../native-profile/indexed-liveness-compile-apple-m3-20261114.json) | 771 |
| Independent constant bounds and fresh CPU profile | Retained: independent constant bounds; unchanged validation and program bytes; lower time, instructions and memory. | [mech-constant-bounds-20261114.tar.gz](mech-constant-bounds-20261114.tar.gz) | [Receipt](../../native-profile/constant-bounds-compile-apple-m3-20261114.archive.json) / [result](../../native-profile/constant-bounds-compile-apple-m3-20261114.json) | 404 |

The latest qualified complete source-to-Object process takes **23.508–23.517 seconds**. Canonicalizing independent constant bounds saves 8.51% of compiler time and 42.09% of frontend time, with 10.21% fewer instructions and 14.41% lower peak RSS. Every generated program byte remains unchanged. This cohort uses the same 55 flags for both subjects, including the independently qualified terminal-append runtime change. Earlier configurations retain their own pins and are not pooled.

Rejected evidence stays here: SSA forwarding repeatedly slows the compressor, and two backend-reuse approaches fail to establish a useful overall compiler saving. All adverse measurements and transient corrections remain in their packets.

To verify and replay a packet from a checkout, use a fresh extraction directory. This example needs only Python and repository files; it does not compile or execute the benchmark:

```sh
python3 benchmarks/mech-deflate/experiment.py verify benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-constant-bounds-20261114.tar.gz --receipt benchmarks/mech-deflate/native-profile/constant-bounds-compile-apple-m3-20261114.archive.json --extract /tmp/mech-constant-bounds-retrieved
python3 /tmp/mech-constant-bounds-retrieved/replay.py > /tmp/mech-constant-bounds-replayed.json
cmp /tmp/mech-constant-bounds-replayed.json benchmarks/mech-deflate/native-profile/constant-bounds-compile-apple-m3-20261114.json
```

Use the corresponding archive, receipt and result from the table to replay another packet. The maintained verifier rejects unsafe archive paths, duplicate entries and non-file entries, then checks the archive, manifest, every file SHA-256 and Git blob identity. Offline replay independently reconstructs the numerical summaries and checks retained output bytes and logs; it does not rerun original measurements.

The earlier archives are preserved byte-for-byte. Their internal snapshots may describe local publication as it stood when sealed; the repository receipts and this index record the current destination. Historical peer profiles and earlier compiler packets remain available through their existing release receipts. See the [pipeline comparison](../../../../docs/design/native-compiler-process-profile.md) and [experiment index](../../EXPERIMENTS.md).
