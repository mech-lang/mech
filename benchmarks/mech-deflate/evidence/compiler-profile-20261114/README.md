# Compiler-profile evidence, 2026-11-14

Five complete evidence packets are tracked here at the user's request. They include raw commands, observations, source snapshots and patches, test logs, benchmark Objects and metadata, rejected attempts, adverse counters and pure offline replay scripts. Compiler executables are excluded and hash-pinned inside each packet. The archives total 45,591,124 bytes (45.6 MB).

The [machine-readable index](index.json) pins every archive SHA-256 and source revision. Each receipt also pins the internal manifest and replay output. All five repository copies have been extracted, verified and replayed byte-identically against their compact summaries.

| Packet | Decision | Archive | Receipt / result | Files |
| --- | --- | --- | --- | ---: |
| Ownership-join SSA forwarding | Rejected: repeated program slowdowns | [mech-ssa-forwarding-rejected-20261114.tar.gz](mech-ssa-forwarding-rejected-20261114.tar.gz) | [Receipt](../../native-profile/ssa-forwarding-rejected-apple-m3-20261114.archive.json) / [result](../../native-profile/ssa-forwarding-rejected-apple-m3-20261114.json) | 4,114 |
| Reusable visitation storage | Retained: identical program bytes; higher RSS | [mech-thread-visits-20261114.tar.gz](mech-thread-visits-20261114.tar.gz) | [Receipt](../../native-profile/thread-visits-compile-apple-m3-20261114.archive.json) / [result](../../native-profile/thread-visits-compile-apple-m3-20261114.json) | 339 |
| Selected liveness definitions | Retained: identical program bytes; higher RSS | [mech-live-definitions-20261114.tar.gz](mech-live-definitions-20261114.tar.gz) | [Receipt](../../native-profile/live-definitions-compile-apple-m3-20261114.archive.json) / [result](../../native-profile/live-definitions-compile-apple-m3-20261114.json) | 364 |
| Current pipeline memory and integration | Diagnostic: current runtime integration and frontend RSS peak | [mech-current-pipeline-memory-20261114.tar.gz](mech-current-pipeline-memory-20261114.tar.gz) | [Receipt](../../native-profile/current-pipeline-memory-apple-m3-20261114.archive.json) / [result](../../native-profile/current-pipeline-memory-apple-m3-20261114.json) | 64 |
| Borrowed schema finalization | Retained: identical program bytes; lower time, instructions and RSS | [mech-borrowed-schema-20261114.tar.gz](mech-borrowed-schema-20261114.tar.gz) | [Receipt](../../native-profile/borrowed-schema-compile-apple-m3-20261114.archive.json) / [result](../../native-profile/borrowed-schema-compile-apple-m3-20261114.json) | 371 |

The latest qualified complete source-to-Object process takes **28.764–28.769 seconds**. Borrowed schema finalization saves 4.08% of compiler time and 24.33% of frontend time, lowers peak RSS 7.62%, and preserves all emitted bytes. The rejected SSA trial stays here because its compiler saving came with repeated compressor runtime slowdowns. Cohorts retain their individual source, runtime configuration and observation pins; they are not pooled.

To verify and replay a packet from a checkout, use a fresh extraction directory. This example needs only Python and the repository files; it does not compile or execute the benchmark:

```sh
python3 benchmarks/mech-deflate/experiment.py verify benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-borrowed-schema-20261114.tar.gz --receipt benchmarks/mech-deflate/native-profile/borrowed-schema-compile-apple-m3-20261114.archive.json --extract /tmp/mech-borrowed-schema-retrieved
python3 /tmp/mech-borrowed-schema-retrieved/replay.py > /tmp/mech-borrowed-schema-replayed.json
cmp /tmp/mech-borrowed-schema-replayed.json benchmarks/mech-deflate/native-profile/borrowed-schema-compile-apple-m3-20261114.json
```

Use the corresponding archive, receipt and result from the table to replay another packet. The maintained verifier rejects unsafe archive paths, duplicate entries and non-file entries, then checks the archive, manifest, every file SHA-256 and Git blob identity. Offline replay independently reconstructs the numerical summaries and checks retained output bytes and logs; it does not rerun the original measurements.

The four earlier archives are preserved byte-for-byte. Their internal snapshots may describe local publication as it stood when sealed; the repository receipts and this index record the current destination. Historical peer profiles and earlier compiler packets remain available through their existing release receipts. See the [pipeline comparison](../../../../docs/design/native-compiler-process-profile.md) and [experiment index](../../EXPERIMENTS.md).
