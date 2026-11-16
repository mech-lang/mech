# Keep the first compact owner slot inline

The compact runtime stores its first registered owner pointer directly in the
invocation heap. Further owners use the existing vector and free list. The
owner itself keeps its stable allocation and six-word descriptor. The compiler
continues to use the same ownership checks and reads the reference count at the
same address. Allocation, writable backing, final publication, cleanup and
worker adoption all consult the same slot mapping. The borrowed input and
output remain live, and the returned byte vector still owns a distinct
allocation at the original capacity.

| Final cohort | Cases | Timed calls | Elapsed / paired control | Pooled / block gates | Faster than Roc | Faster than libdeflate | Faster than both | Faster than either |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| Full | 81 | 245,664 | 0.993514× | 81 / 81 | 63 → 64 | 33 → 32 | 16 → 15 | 80 → 81 |
| Full repeat | 81 | 245,664 | 0.993649× | 81 / 81 | 64 → 64 | 33 → 34 | 16 → 17 | 81 → 81 |

Natural 16 KiB crosses from 1.005540× to 0.999860× in the full run. The prior
published score was 64/81, and the repeat has 64/81 in both arms. The first run
loses periodic 65,536 bytes against libdeflate (0.998430× → 1.000881×). The
repeat gains periodic 65,535 bytes
(1.001173× → 0.996621×). Both changes are near parity. The largest adverse
paired observations are periodic 8 KiB at 1.020772× in the first run and
random 256 bytes at 1.038107× in the repeat.

| Input | Full control → Mech / Roc | Repeat control → Mech / Roc | Full → repeat Mech / libdeflate |
| --- | ---: | ---: | ---: |
| Empty | 2.009834× → 1.678783× | 1.980840× → 1.643507× | 0.013292× → 0.012976× |
| One stored byte | 1.920223× → 1.629506× | 1.907146× → 1.606117× | 0.014474× → 0.014348× |
| Seven stored bytes with prefix | 2.193478× → 1.818098× | 2.195974× → 1.889192× | 0.015263× → 0.015774× |
| Eight literals | 0.162026× → 0.162186× | 0.161606× → 0.162328× | 2.088080× → 2.089284× |
| Natural 16 KiB | 1.005540× → 0.999860× | 0.999967× → 0.993959× | 1.006011× → 1.005196× |
| Natural 32 KiB | 1.042414× → 1.036262× | 1.036491× → 1.033182× | 0.942103× → 0.939237× |
| Held-out slides | 1.064944× → 1.059475× | 1.062986× → 1.058219× | 0.921762× → 0.924413× |
| Natural 1 MiB | 1.059324× → 1.054569× | 1.058815× → 1.056308× | 0.850601× → 0.853586× |

The stored cases improve by 14.0–17.1% elapsed over the two full runs. A
separate allocator cohort confirms 3 → 2 requests and removes 32 requested
bytes for every measured stored case. The reference uses one request. In the
separate CPU cohort each stored case retires 155 fewer instructions. Eight
literals takes 1.000986× and 1.004469× paired elapsed, and its allocation
count stays 228. The three stored cases still lose by balanced block means.
The first build's screen, both full runs and adverse seven-byte CPU cycles
are retained alongside the final runs.

The source, artifact revision 14 and generated Object are pinned to the
[defaulted-read qualification](native-defaulted-read-setup-qualification.md).
Both arms use the same Object
`ea7328a18d060573a5ab1e38ab072e7d4b612f33f028e5c903ed76fc54e0b32c`, allocator,
level, inputs and complete-call boundary. The final runtime-only Rust
worker built with nightly 2026-03-03, optimization level 3 and warnings
denied has SHA-256
`4b63477bfb45387b54c41fd596f5738f1ca066717094f31edcb25e264746eaeb`.
The initial and final links differ. The final linked worker was retimed in two
complete cohorts and its allocator and CPU probes were rebuilt and rerun.
The final runtime belongs to the compact diagnostic entry. The managed compiler
uses its separate runtime and retains its prior qualification.

The final standalone runtime passes 12 tests, including rejection of publication
while an alias is live, retry after releasing that alias, reusing the first slot
with a live spill slot, cleanup and preserved returned bytes. The exact
original and decoded descriptors each pass 162 linked calls. Every timed call
checks exact reference output where applicable, independent inflate, capacity,
both aliases and final cleanup. All six Mech payload counter distributions
agree across both full matrices. Format, resident routing, interactive
architecture, compiler planning and host catalog checks pass. The repository
unsafe-boundary audit reports the existing related-post measurement probe,
with the same failure on the clean benchmark parent `80ddb85f4b`. Hosted CI is outside
this qualification.

## Separate parser trial

The existing admitted short reduction recomputes its biased price index and
uses separate instructions for the entry comparison and span subtraction. The
trial keeps linked addresses fixed and uses the subtraction's flags for the entry branch, carries the first price index
into its item and writes the best cost directly on singleton completion.
Both memory accesses retain their widths and addresses. The continuing loop,
vector instructions and malformed-record check remain byte-identical.

| Input | Screen Mech / Roc | Confirmation Mech / Roc | Screen / control | Confirmation / control |
| --- | ---: | ---: | ---: | ---: |
| Natural 16 KiB | 0.981461× | 0.971609× | 0.983023× | 0.973954× |
| Natural 32 KiB | 1.020130× | 1.017327× | 0.984125× | 0.979907× |
| Held-out slides | 1.045771× | 1.046189× | 0.982999× | 0.982542× |
| Natural 256 KiB | 1.033698× | 1.037512× | 0.989583× | 1.003889× |
| Natural 1 MiB | 1.040128× | 1.040864× | 0.981868× | 0.983786× |

The screen gains three libdeflate wins. The confirmation gains two libdeflate
wins and one Roc win. Both timing gates pass throughout. The 162 complete
linked checks, 68,096 direct reduction positions and 288 boundary pairs pass,
including ties, wrapping cost, rejection origin 15525 and recovery with
unchanged cache and output. Only the bounded machine span changes. Normal
compiler emission and an 81-case acceptance run remain pending for this trial.

Fresh CPU-time samples on the retained executable place 5,079/5,543 running
samples in the matcher and path parser on natural 1 MiB, and 4,736/5,339 on
held-out slides. They include validation and use a separate capture.

[Executable hashes, raw timing observations, linked code and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/native-inline-owner-slot-14660ab8a477.tar.gz).
The archive verifies 704 files and retains the full observation sets. SHA-256
`14660ab8a4776bcc513588fb5d0add563c9093cdf175b24c3887833bfd0e1ffd`.
