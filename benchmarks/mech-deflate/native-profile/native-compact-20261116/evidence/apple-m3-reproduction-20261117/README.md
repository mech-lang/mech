# PR #993 DEFLATE reproduction on Apple M3

The qualified 71,192-byte Mech encoder was rebuilt from source on Apple M3.
Its complete object and metadata match the pinned Apple M1 artifacts exactly.
The locally rebuilt Mech worker and the unchanged archived M1 worker are
measured separately against the same pinned Roc and libdeflate 1.26 level-12
executables. This is a machine/environment reproduction, not a compiler
promotion campaign.

## Results

| Machine / executable | Roc wins, forward / reverse | libdeflate 12 wins, forward / reverse |
| --- | ---: | ---: |
| Apple M1, qualified archived worker | 81 / 81 | 61 / 61 |
| Apple M3, source-rebuilt worker | **71 / 69** | **57 / 58** |
| Apple M3, unchanged M1 worker | 68 / 68 | 58 / 57 |

All counts are out of 81. Both M3 campaigns pass raw-observation replay, each
covering 1,965,312 timed calls. The original 61/81 M1 result does not reproduce
on this M3 with either executable. These orders do not isolate hardware,
operating system, and within-machine variation; they do not establish a
host-rebuild regression. [Machine-readable results](results.json) ·
[Win counts](win-counts.csv)

Each count is a point-estimate win across 81 named cases. The inventory contains
74 distinct input/prefix/capacity contracts; duplicate named cases remain in the
published 81-case comparison. The raw-data replay also reports precision-supported
wins and distinct-contract counts.

## Source and identity

- Compiler checkout: `ed99ade92836761bebd2f7357bf46817204433d4`.
- Recorded native runtime source: `3ff13b6ab4834770aa422fa26887c7ff8cef6b0e`,
  recovered and hash-checked by `rebuild.py`.
- Mech source SHA-256: `653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299`.
- Qualified object SHA-256: `96696eeed083f0f11b51a8b26a01fe9077735821f65e7f08decea56881c6329b`.
- Metadata SHA-256: `6d35fb7d9a46193a9febef97427729921739ed7f914abaefc45e16ff0b091437`.
- Encoder text: **71,192 bytes**.

[Encoder rebuild receipt](encoder-rebuild.json) and
[control rebuild receipt](control-rebuild.json) record the compiler construction,
source compilation, runtime construction, final links, flags, and exact checks.
Both rebuilt workers passed 162 complete checked calls before measurement.
Their worker executables differ from the archived M1 executables because the
benchmark support and links were rebuilt; encoder-object identity alone is not
whole-executable identity.

## Measurement boundary and environment

Apple M3, arm64, macOS 26.2 (25C56), AC power. The original Apple M1 campaign
used macOS 15.6.1. This comparison does not isolate hardware from operating-system
effects. CPU frequency was not fixed, and throttling was not disabled.

The boundary is the native compress-into invocation through private-heap cleanup,
inside a persistent worker. Process startup, fixture loading, JSON transport,
and independent inflation are outside that interval. A fresh libdeflate compressor
is allocated inside each measured call. These are function-runtime measurements,
not application startup or whole-process timings. The returned output is retained
for validation and dropped after timing. As in the original #993 diagnostic
contract, ordinary resource admission/accounting and publication recovery are
omitted; these are not production entry-point measurements.

For each actor and case, warmup lasts at least 200 ms. Every case uses all
24 permutations of four actors four times: 96 balanced blocks. Blocks batch
512 calls for the three stored cases, 64 for other inputs up to 1,024 bytes,
and one otherwise. Each complete order contains **982,656 timed calls**.
The reverse run reverses the case and initial actor order. All actor executions
are serial. The shared campaign lock was held across both orders, with no build
or correctness-preflight process running alongside accepted acquisition.

Each observation checks independent inflation, unchanged input and borrowed-output
aliases, and expected output identity/capacity. All 81 libdeflate output streams
are byte-identical to the pinned reference. Cleanup checks release all private
payload and owner records. The original #993 verifier replays every timed call,
recomputes timing summaries, and checks actor order balance and pinned identities.

CSV seconds are medians of individual calls. The comparison columns are the
geometric means of paired per-block mean-time ratios used by the original campaign;
they need not equal ratios of the separate medians. A ratio below 1 favors Mech.

## Rebuild

Check out the pinned compiler revision and make the recorded runtime commit
available in Git history. On an Apple Silicon Mac with the pinned Rust toolchain,
run the following from the checkout root. `PACKET` is the extracted evidence
archive listed in [evidence.json](evidence.json), which supplies the shared,
hash-pinned allocator object.

```sh
git fetch origin 3ff13b6ab4834770aa422fa26887c7ff8cef6b0e
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-pr993-source-built --variant frame-copies \
  --compiler-opt-level 3 --direct-source --preselect-root-inlining \
  --skip-refused-general-ssa --mimalloc-object "$PACKET/allocator/mimalloc.o" \
  --target-dir /tmp/mech-pr993-compiler
```

This constructs the compiler, compiles the Mech source, checks the emitted object
and complete metadata against their expected hashes, builds runtime/benchmark
support, links a worker, and runs 162 exact checks. The allocator is a fixed
comparison dependency. No transferred Mech executable is used by this rebuild.

The compiler construction took 866.87 seconds in a partially populated cache;
that is not a clean build benchmark. A single source-through-object observation
was 3.50563 seconds, and the benchmark-host link was 0.70014 seconds. These are
unbalanced operational receipts, not comparable to the matched Apple M1
2.54442-second deployment-build median, which uses a different host/link recipe.

## Evidence

[Source-built timings](source-built/timings.csv) contain raw seconds and paired
multiples of both peers for every case/order, along with the corresponding M1
ratios. [Source-built replay](source-built/replay.json) records complete validation.
[Download all raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/pr993-deflate-apple-m3-source-rebuild-20261117-20caefffbb31.tar.gz) (38618306 bytes; SHA-256
`20caefffbb3166eb43af59774b47b717aa3faec644478867b1774ca343ed78f2`).
The archive contains the frozen plans, timer and verifier sources, every raw
observation, cleanup logs, pinned executables/objects, source-build receipts,
and the separately labeled archived-binary diagnostic.
[Archived-worker timings](archived-binary/timings.csv) and
[its replay](archived-binary/replay.json) retain that control separately.

The archive is self-contained for raw-data verification. After extraction, run:

```sh
python3 -B "$PACKET/replay-m3.py" --canonical-fixtures "$PACKET/canonical-fixtures"
python3 -B "$PACKET/archived-binary-diagnostic/replay-m3.py" \
  --canonical-fixtures "$PACKET/canonical-fixtures"
```

Both relocated-fixture replay commands were verified successfully. The archive
manifest pins every included file. Re-running a timing acquisition requires a
fresh output directory and a newly pinned plan; the archived orders are immutable.

A partial setup trial began before the control correctness preflight had exited.
It was stopped, preserved as `setup-overlap-aborted-forward`, and excluded before
the clean timing orders. Its 13 case summaries and pre-exclusion plan remain
in the archive. No failed or unfavorable accepted run was removed.

Two descriptive labels in the frozen source-built setup predate its final build:
`environment.clock` says unchanged M1 executables, and the top-level peer-pins
`qualified_worker_sha256` names the archived worker. These labels were not used
as actor identities. `plan.artifacts`, each acquired run's actor pins, and the
verifier use the actual rebuilt worker hash
`6828360cbf38c42a27e7c0c24bf07af7c2f0e407f5bcaf11e82f911f180512d2`.
The frozen records are retained without retroactive modification.
