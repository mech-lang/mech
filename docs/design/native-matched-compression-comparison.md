# Matched fixed-level runtime-input comparison (R224)

The eight-byte standalone-native win survives the corrected comparator:0.528194× Roc by per-call median and0.542406× by balanced block means. This is a benchmark confirmation, not a new compiler speedup. Ten original cases have lower block means; natural1024 is borderline (median0.9994×,block mean1.0126×) and is not counted as a clear win. All20 original cases plus five new held-out cases through131073 bytes match exact reference bytes and independent decompression.

## What is matched

Both programs now know level12 at compile time and receive input bytes at runtime. One Mech image and one Roc executable each handle every case; no fixture bytes are compiled into either compressor. The existing level12 source, search depth300,nice258,passes10,threshold1bit,static cutoff10000 and≤7-byte stored bypass remain. The real262144-U16 tree,1500513-U32 cache,305257-U64 paths and all allocation sites stay.

Both workers persist across cases and alternate in24 balanced order blocks. Each gets200ms of explicit wall-time warmup per case, executing the same complete operation and validations without retaining per-warmup logs. Both use Rust std::time::Instant from nightly2026-03-03 and the same frozen miniz validation library. Each timed call is followed by exact byte comparison, independent inflate and live input/output alias checks. Validation, clear/prefix, argument/reference setup, printing and old-output drop remain outside timing. Complete compression, actual scratch allocation/initialization and cleanup remain inside.

The output mode is explicitly BORROWED on both sides. The Mech input/prepared output have live caller backing; the Roc host holds an actual reference while passing a second owned reference to its pure exported compressor. Saved prefix/capacity and distinct returned backing are checked. The eight-byte output capacity remains13. Both clear and reuse the previous output capacity; neither supplies a fresh prepared buffer each repetition. The Roc list's encoded capacity is verified with a shape probe; a logical capacity13 is represented by26. Input loading/representation preparation remains excluded. Both retain the complete input; no streaming or complete RSS-equivalence claim is made.

The historical reference is retained separately:runtime level,embedded per-fixture input,consumed output,its old clock/warmup/validation and separate order. It is run freshly for256 samples per original case. The historical column below divides the SAME currently measured Mech median by that explicitly different denominator. Earlier published checkpoints remain intact; old and new ratios are not compounded.

This standalone execution comparison intentionally omits Mech's extra exact resource/accounting/publication/recovery contract, which the reference does not supply. Restoring that contract is not a prerequisite. Source indexing,checked arithmetic,alias semantics and valid accesses remain required; R223's canonical matrix repair is retained. Maintained normalIR215 is separate and unchanged.

## Revisions and evidence

Compiler checkpoint `3c2fadb93ab8254eef8bcd943bbf3b9c76d2876c`; normal compiler `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`; normal IR/work/ABI/MBC215/107/39/14. Roc package671077457ff7ef3a3d44caf715a3fc595addf379 and compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d. All14 package hashes are reverified. Pinned mimalloc2.1.7 is unchanged. Rust static-host compilation uses warnings-as-errors and frozen dependencies; the math library is not rebuilt.

- mech_source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- mech_bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- mech_object_sha256: `4ed01e640ecc30de6ec4e55efe84ff17a184e8e45d52621067a8c418d96d39f6`
- mech_worker_sha256: `e48aaf6e5f3a34ee3d0acfb3c57f7ba662c873ceb37efa5eb91846876250bbac`
- reference_worker_sha256: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- reference_compiler_sha256: `fdd590c62a2783295c7bfe7e8e5a19e83de508418a95b48373ab5b189f87ff94`
- allocator_sha256: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

The Mech compression Object is byte-identical to R223. Harness changes add only explicit warmup; reference changes supply a fixed-level pure entry and runtime RC-backed arguments. Fresh108,148 complete primary calls/108,048 retained observations plus1,168,522 verified warmup calls and5120 historical reference calls are retained. All25 cases complete. Mech private payload and owner records finish at zero; the reference's host input/output roots are released. A full reference allocation census is not inferred from root cleanup.

## All25 results

Times are per-call medians in microseconds. Block means and5th/95th percentiles are separately archived; the quantiles use sorted floor indices. Tiny reference times are now nonzero with the matched clock, so all20 original ratios are measurable. Their three former zero-denominator cases are not silently relabeled historical wins.

| Input | Bytes | Mech µs | Roc µs | Matched median ratio | Matched block-mean ratio | Same Mech / historical reference |
|---|---:|---:|---:|---:|---:|---:|
| reymont-prefix-32769 | 32769 | 10711.354 | 4343.312 | 2.466172× | 2.466172× | 2.458989× |
| 33027-periodic-window | 33027 | 2682.917 | 1266.750 | 2.117953× | 2.117953× | 2.144618× |
| repeated-32769 | 32769 | 2667.312 | 1351.771 | 1.973199× | 1.973199× | 1.994998× |
| random-32769 | 32769 | 3187.187 | 1055.958 | 3.018290× | 3.018290× | 3.038310× |
| 16384-distribution-change | 16384 | 2439.542 | 826.479 | 2.951727× | 2.951727× | 3.088028× |
| eight-literals | 8 | 152.208 | 288.167 | 0.528194× | 0.542406× | 0.526671× |
| reymont-prefix-16384 | 16384 | 5143.541 | 2158.229 | 2.383223× | 2.383223× | 2.407462× |
| empty | 0 | 0.083 | 0.042 | 1.976190× | 2.319189× | unresolved |
| one-stored | 1 | 0.083 | 0.042 | 1.976190× | 2.536237× | unresolved |
| seven-stored-prefix | 7 | 0.084 | 0.042 | 2.000000× | 2.758622× | unresolved |
| eight-overlap | 8 | 150.792 | 288.709 | 0.522298× | 0.531088× | 0.519972× |
| 320-overlap | 320 | 181.646 | 298.375 | 0.608784× | 0.619445× | 0.603475× |
| eight-literals-prefix | 8 | 152.459 | 288.084 | 0.529217× | 0.539731× | 0.522120× |
| 128-biased-dynamic | 128 | 156.417 | 289.583 | 0.540146× | 0.553795× | 0.537515× |
| 32-high-entropy-stored | 32 | 164.562 | 289.083 | 0.569257× | 0.575632× | 0.561647× |
| random-64 | 64 | 177.833 | 292.041 | 0.608933× | 0.624093× | 0.621795× |
| random-256 | 256 | 210.438 | 296.833 | 0.708942× | 0.718816× | 0.706166× |
| random-1024 | 1024 | 249.938 | 307.250 | 0.813466× | 0.821391× | 0.816789× |
| repeated-1024 | 1024 | 242.895 | 329.188 | 0.737864× | 0.744280× | 0.749677× |
| reymont-prefix-1024 | 1024 | 338.291 | 338.479 | 0.999445× | 1.012601× | 1.018950× |
| heldout-window-32767 | 32767 | 10799.229 | 4348.458 | 2.483461× | 2.483461× | unresolved |
| heldout-window-32768 | 32768 | 10729.709 | 4337.917 | 2.473471× | 2.473471× | unresolved |
| heldout-slides-65537 | 65537 | 21729.750 | 8852.417 | 2.454669× | 2.454669× | unresolved |
| heldout-random-65537 | 65537 | 6474.562 | 1934.230 | 3.347360× | 3.347360× | unresolved |
| heldout-mixed-131073 | 131073 | 23257.916 | 8389.000 | 2.772430× | 2.772430× | unresolved |

Eight-byte5th–95th percentiles are147.375–177.250µs Mech and281.167–300.541µs Roc. The current natural1024 point is too close to parity for a clear-win claim, and its block mean is adverse. The three stored-only cases remain slower (roughly2.3–2.8× by block means), although the absolute time is sub-microsecond. They remain in the regression matrix.

## Held-out coverage and remaining work

New inputs cover32767/32768-byte window boundaries,65537-byte repeated-text and deterministic random data, and131073-byte mixed text/random/periodic data with a prefix. They are generated independently of compiler selection. The fixed reference generates their complete expected streams at runtime, then independent inflate verifies them. The SAME Mech image matches every stream. All input/prefix/expected bytes and hashes are archived for reproducibility.

No new compiler optimization or source rewrite is included in R224. No fresh matched-reference instruction/cycle/allocation cohort is claimed. R223's counters/profiles remain historical; its compression Object is unchanged, while the reference and outer harness differ. Actual complete-executable section sizes and current reference entry disassembly are archived separately. No new scalar/ordinary678-test suite or completed hosted CI is claimed. Same seven inherited ordinary exclusions; no new compiler failure or resource denial. The PR remains draft and ordinary exact-head CI stays cancelled by instruction.

Harness failures are preserved:the platform requires a local forwarding entry; the first hand-written list used an unencoded capacity and crashed; corrected tagged capacity passes the explicit shape and full eight-byte probes. A debugger was unavailable in this noninteractive environment, so actual disassembly and controlled probes established the layout instead. A hash-format helper initially failed to compile and was replaced by supported per-byte formatting. These are resolved new harness defects, not compiler performance findings. No failed primary cohort is replaced.

Next:the isolated two-binding source control for repeated packed-cache/header reads is already compiling. Measure it independently on all25 cases, with new instruction/allocation cohorts and current executable profiles, before the generic phase range/shape proof-reuse experiment. Private-write simplification, shared phase packet storage and wider matching remain separate later experiments. No additional phase eligibility expansion precedes that evidence.

## Replay

`analyze.py` recomputes every timing median,block mean,ratio,order and count from raw complete-call observations. `fixture-bytes.json` preserves independently hashed runtime inputs,prefixes and expected streams. `measurement-pins.json` records exact binaries,source,bytecode,allocator and dependencies. Native reproduction needs the frozen toolchain/dependencies and R223 Object. Public evidence excludes native binaries/Objects,bytecode,raw traces,credentials and private environment exports.

[Verified public evidence and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r224-matched-level12-135ea1ec2c87.tar.gz) (1558397 bytes;190 files), SHA256 `135ea1ec2c87129c64ad8af8ad66626c881ae804ef4fdc61476199878e4720ec`. Downloaded file hashes and numerical replay match.
