# Shared-debit inlining: measured regression, hint removed

The public library-debit inline hint made complete eight-byte compression
**2.46176468% slower**, with every independently prepared worker pair regressing.
It is removed. The preceding work-meter wrapper hint and its qualified 2.35%
gain remain. Production source again matches reviewed head
`a4b9ac5f4312cf822102dd394f9ea52c3add08b6` exactly.

## Matched evidence

| Same-session measurement | Retained compiler | Rejected candidate |
| --- | ---: | ---: |
| Complete eight-byte primary median | 4.921104 ms | 5.042250 ms |
| Fresh common reference = 1x, 0.285 ms | 17.26703158x | 17.69210526x |
| Fixed older 0.300 ms ruler | 16.40368000x | 16.80750000x |

Candidate/retained ratio **1.02461765**. Pair regressions: **2.97%, 2.74%,
2.85%, 1.70%**. Only **5/64** descriptive blocks improve; median block ratio
1.02776674, geometric ratio 1.02710742. Both evaluation-order strata regress:
before/after median 1.02912552 (2/32 blocks improve), after/before 1.02695426
(3/32 improve). The blocks share four pairs, not 64 independent experiments.

All **3984** observations are retained, including **3968** primary calls.
Fresh reference: **20000** observations. Each reference and worker cohort was
committed immediately: `80f7863d1`, `ff18287b3`, `08b8b3c34`, `78b6a1ae0`,
`2a48b26c1`; derived summary `c898f8cd9`. No partial or superseded primary
cohort exists in this round. No builds, tests, audits, JIT preparation or
profiles overlapped primary evaluation; the peer worker waited outside timing.

The retained compiler here is the same frozen executable that achieved
4.917375 ms against a 0.279 ms reference in the preceding round. The fresh
4.921104 ms measurement is compatible with that latency. Its lower normalized
figure uses a different reference timing; it is not another optimization gain.
The preceding 2.35% gain remains a separate matched result. No gain is claimed
for this round, and parity remains unachieved.

Every observation checks exact ten raw bytes, independent inflation, input
contents and aliases, capacity 13 and final zero cleanup. Work is exactly
**44443887150 in both variants**, for every observation. Source, algorithm,
tables, depth 300, nice length 258, near-EOF limits, real 262144-entry U16 tree
and 1500513-word U32 cache, allocation sites and clear/rebind/timing boundaries
are unchanged. No larger-input compression timing was attempted.

## Revisions and pins

- PR base `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
- Starting reviewed head `a4b9ac5f4312cf822102dd394f9ea52c3add08b6`;
  clean review is PR 973 comment `6527236792`.
- Rejected compiler `fbb538f6a1fd659bb62d5b5eef58b9c0418ad5f1`.
- Hint removed in **`e6d0622b444bc27f9a624c9d22d719126345512b`**.
  `git diff --exit-code a4b9ac5f4312cf822102dd394f9ea52c3add08b6 HEAD -- src hosts machines`
  has zero output and succeeds. The retained production implementation is the
  preceding `8c60a8cee62c01257b2c282dea95755a141307c8`.
- IR/work/ABI/bytecode remain **131/95/37/14**.
- Source SHA256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- Artifact SHA256 `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- Reference executable SHA256 `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.
- Pinned allocator SHA256 `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
- Retained worker SHA256 `09d6b7aeb11c66c4721d32e8f805646f3be424ace75b5d176184d5d7260dd20a`.
- Rejected worker SHA256 `cd886cf6239d63a7f85bbd1d75836a12123640a3df0a9ff43e57a097eecab4aa`.
- Unchanged worker source SHA256 `a0062c6170db0005d96eba239232cfdbdd41054945c2e4a574afa8acb8939575`.

The only proposed production edit was an ordinary inline hint on
`NativeLibraryWorkBudget::charge_compute_with_limit`. Its body, prices,
overflow/ceiling precedence, scope checks and worker/reserved-fuel semantics
were never changed. The preceding GPU work-wrapper hint remains present.

## Build and test qualification

Same nightly 2026-03-03, aarch64 macOS, release opt 3, locked/offline, jobs 1,
incremental disabled, warnings as errors. GPU retains the same six features;
Engine the same 178 feature flags. Five unrelated direct-library hashes match.
Engine, GPU and standard-library hashes change. Their dependent compute/runtime
crates and consumer binaries also rebuild; operation selection stays unchanged.
Production build takes **624.25 seconds**. The actual Engine unit root opt 1
takes **267.87 seconds**, overlapping only untimed compilation. The GPU unit
root opt 1 takes **29.30 seconds**. Exact commands, environments, dependency
hashes, build stdout/stderr and test receipts are in `qualification/`.

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review --features mech-runtime/compiler
engine-tests resident::budget:: --nocapture --test-threads=1
# 102 passed, 0 failed; 1026 filtered out
gpu-unit-tests native_artifact::program::runtime:: --skip complete_unchanged_knucleotide_body_uses_the_owned_emitted_map --nocapture --test-threads=1
# 308 passed, 0 failed; 131 filtered out
gpu-unit-tests native_artifact::work:: native_artifact::memory:: --nocapture --test-threads=1
# 13 passed, 0 failed; 426 filtered out
```

**423 maintained tests pass for the candidate.** They cover exact and one-short
work, overflow/ceiling/foreign-scope precedence, nested calls, reservations,
concurrent worker debit atomicity, scope restoration and caught unwind, current
frames, unchanged publication/aliases, cleanup and retry. Intentional caught
panic messages in stderr are successful unwind fixtures. No test or expected
value changed for this experiment.

All **12 scalar controls** pass the unchanged 1.05 ceiling, maximum ratio
**1.010103210**. Requests/bytes do not increase, pure scalar counts/bytes match
and scalar-loop slopes remain zero. All **eight managed-frame controls** retain
requests, bytes, work and zero slopes. Controls committed immediately in
`32ee496c8` and `471271bff`. These passing controls did not override the actual
complete-program slowdown.

The reverted source equals the already qualified preceding source, whose 321
maintained tests and 2.35% gain remain retained. Its frozen executable also
passes every fresh observation in this round's four baseline workers. A local
rebuild of the restored source is recorded separately in `restored-build/` when
complete; at this checkpoint that rebuild is running, with no new compression
timing underway. No completed-rebuild claim is made here.

## Machine code, Object and profile

The candidate folds the outer ceiling check into callers but retains a separate
thread-local closure helper. Frame retirement grows **155 to 190 static
instructions**. Linked worker Rust text grows **48187088 to 48212532 bytes**
(+25444). These are static measurements; the full slowdown is established by
paired elapsed results, not attributed to a single instruction-count change.

Fresh candidate compressor Object, metadata and canonical descriptor are
**byte-identical** to the retained compiler's output: Object 13802344 bytes,
native code 13801616, stack 108560. Object SHA256
`d2c0185ac19d2279c477386cb9f51b080bb4ea159735bbae61101930bfe2cf13`;
descriptor SHA256
`7b6cfa95f6735e77b12842bbd2e9aa94388d6f04b29c8417b30e01f0cb21520e`.

Fresh candidate profile: **695 diagnostic compressions**, all excluded from
primary, correct work/output and zero cleanup. Raw profile committed immediately
in `c644699df`. Disjoint exclusive weights before/after: frame actions 332/337,
ordinary visits 237/216, other helpers 287/303, selected calls 141/145,
remaining 357/347; totals 1354/1348. The new thread-local closure helper has 40
exclusive samples, shared debit 58, frame retirement 56, TLS lookup 48 and
ordinary visits 49. Sampling is not a paired time comparison or proof of the
specific cause of the slowdown. Account and retirement work remain prominent.

## Disposition and next step

No build, maintained test or protected control failed. The performance candidate
failed its intended improvement criterion and is removed. A diagnostic JSON
inspection probe assumed an object where one old receipt held a command list;
that probe was corrected and did not affect a build, test or measurement.

The inherited reduced-catalog whole-knucleotide statistics exclusion remains.
Other unit/integration targets, broad full-feature suites, linked/AOT recovery,
the component cohort, selected not/xor and larger-input timing were not rerun.
Warnings-as-errors, factory/catalog checks, safety boundaries and resource
allowances are preserved. Final formatting/diff/source-restoration checks are
recorded separately. No new unsafe code or semantic compiler change remains.

The next bounded target is repeated retirement accounting: establish the exact
reached cleanup/debit inventory, then evaluate paying the same successful
cleanup charge together while retaining the original ordered path on tight
funding or other unsupported state. It requires a real compressor witness,
exact-boundary and failure-prefix tests before enabling behavior. No such
semantic implementation is included in this round.

The PR remains draft, CI is cancelled by user direction and review is requested
on the evidence/restoration update. Parity remains the active goal.
