# R248: equal logical work and controlled scaling through 1 MiB

Retain R245. This round adds measurement coverage and diagnostics; it makes no new speedup claim. R247 remains an archived component with a repeatable natural-text gain and periodic penalty. The normal Object and metadata emitted by the instrumented compiler with diagnostics disabled are byte-identical to R245.

Git parent `5999af0df3862775ff1138660db964198b5c574f`; retained compiler checkpoint `085dce0ee59b2c798a9ed542c62273759414f0cc`. Ordinary R215 remains `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC 215/107/39/14. Ten opt-in diagnostic files change in the complete 816-file GPU compiler slice over archived R245. Shared Engine/Core/runtime/dependency libraries remain frozen. The patch applies to that declared experimental closure, not directly to the maintained tree.

## Main findings

Mech and Roc agree on all 11 listed logical-work categories for all 81 configurations: the original 25 and 56 controlled prefixes. The categories include searches, skips, tree visits, cache headers and matches, path invocations, candidate lengths, reduction completions, node appends, invocation reasons and span histograms. This rules out extra work in those categories as the explanation for the measured gap. It does not equate every internal operation or physical allocation.

The unchanged retained executable completes every new input through 1 MiB, with exact reference bytes and independent decompression. No source allowance, algorithm or input-specific compiler selection changes. This uses the existing diagnostic compilation/entry policy; it is not evidence that the ordinary finite managed entry admits these inputs.

| 1 MiB input | Roc ms | Retained R245 ms | R245 / Roc | Archived R247 / Roc | R245 instructions/byte | Roc instructions/byte |
|---|---:|---:|---:|---:|---:|---:|
| natural | 161.124916 | 361.569563 | 2.244033x | 2.154004x | 4425.437 | 1593.363 |
| random | 28.773104 | 93.100521 | 3.235679x | 3.319774x | 1450.463 | 305.360 |
| periodic | 31.402146 | 56.956958 | 1.813792x | 1.866506x | 1153.802 | 549.308 |
| repeated | 34.347875 | 58.053959 | 1.690176x | 1.739905x | 1148.822 | 549.337 |

The natural-text gain and repetitive/random penalties of phase-only threading persist on the new larger inputs. This is another reason to preserve the component without silently making it the default. The instructions/byte gap remains substantial, especially on random input. CPU-time sample shares are not multiplied into instruction counts.

## Current algorithmic work, independently counted

| Input | Searches | Skips | Tree visits | Cached matches | Path calls | Candidate lengths | Reductions | Node appends |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 33027-periodic-window | 169 | 32,854 | 32,982 | 256 | 2 | 65,460 | 512 | 66,570 |
| reymont-prefix-32769 | 32,763 | 2 | 118,063 | 76,747 | 10 | 1,130,575 | 383,735 | 166,425 |
| random-32769 | 32,765 | 0 | 7,791 | 21 | 3 | 63 | 63 | 99,081 |
| heldout-random-65537 | 65,533 | 0 | 22,931 | 77 | 3 | 231 | 231 | 197,385 |
| prefix-natural-1048576 | 1,048,572 | 0 | 5,592,859 | 2,981,091 | 25 | 39,253,430 | 14,442,759 | 5,092,906 |
| prefix-random-1048576 | 1,048,572 | 0 | 477,320 | 1,967 | 11 | 5,517 | 5,517 | 2,999,990 |
| prefix-periodic-1048576 | 4,106 | 1,044,466 | 1,048,531 | 8,130 | 8 | 2,080,810 | 16,260 | 2,099,216 |
| prefix-repeated-1048576 | 4,067 | 1,044,505 | 1,048,570 | 8,130 | 8 | 2,080,888 | 16,260 | 2,099,216 |

Every number in this table is independently equal on both sides. All 81 complete rows, including per-reason calls and six span bins, are in work-ledger.json. Natural32 retains 235,600 one-length, 47,285 two-length and 26,085 three-length reductions: 80.5% have only one to three candidates. Random32 has only 63 candidate-length evaluations, so eliminating more of that inner arithmetic cannot explain or close its broad per-position gap.

Roc diagnostics record each block, rewind/carryover, path reason and append total. Per block, all invocations agree on E_b, and total candidate work is P_b times E_b. Node appends equal N_b + 258 per invocation. Mech independently agrees on total per-reason invocations, weighted node appends and candidate work. Individual Mech block offsets are not directly sampled; exact per-block cross-language history is not claimed. The current pass census replaces the old assumption that every input performs ten optimizing passes. At 1 MiB, natural text performs 23 optimizing invocations plus one static alternative and one saved-path regeneration; random performs 11 optimizing invocations; periodic and repeated each perform eight.

## Controlled inputs and comparison boundaries

Four frozen streams cover text, deterministic xorshift32 random bytes, the existing periodic seed and the existing repeated byte. Each is tested at 1, 4, 8, 16, 32, 64, 128, 256 and 1,024 KiB, with immediate 32/64 KiB neighbors and the existing 33,027-byte boundary. All hashes and exact prefix relationships are checked. The normal executable is the same at every size; only its runtime input changes.

The official text corpus download matches published MD5 d8f54d78105079775f32d76dc55fc671 and SHA256 0eac0114a3dfe6e2ee1f345a0f79d653cb26c3bc9f0ed79238af4933422b7578. The original text fixtures through 32,769 bytes are exact prefixes. The older held-out 65,537-byte fixture repeats that 32,769-byte prefix; it is intentionally preserved. New larger text sizes use the actual official continuation. Original random32 and held-out random64 also remain separate from the new controlled random stream. No old fixture or expected output is replaced. [Corpus and published checksum](https://sun.aei.polsl.pl/~sdeor/index.php?page=silesia).

All three normal actors—retained R245, archived R247 and the fixed-level12 runtime-input reference—share 24 balanced blocks, all six orders repeated four times, 200 ms warmup, the pinned allocator, identical Rust clock, live input/output aliases, validation placement and output-capacity formula. Full compression, scratch construction, transfer and cleanup are timed; preparation, validation, logging and previous-output drop are excluded. One new primary cohort describes this sweep. CPU and whole-heap probes use separate cohorts. No build, test, diagnostic execution or profile overlaps them. Original25 two-cohort acceptance results remain R247 observations, since no normal machine code changes here.

## All 56 prefix results

| Input | Bytes | R245 / Roc | R247 / Roc | R245 ns/byte | Roc ns/byte | R245 instructions/byte | Roc instructions/byte | Tree visits/byte | Candidate lengths/byte |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| natural | 1024 | 0.927359x | 0.905046x | 304.4028 | 328.2471 | 4006.679 | 2181.561 | 0.745117 | 22.300781 |
| natural | 4096 | 1.732205x | 1.627041x | 276.8860 | 159.8459 | 3757.074 | 1693.808 | 1.815430 | 48.665771 |
| natural | 8192 | 2.000372x | 1.896303x | 274.2818 | 137.1154 | 3836.207 | 1613.025 | 1.913696 | 46.593750 |
| natural | 16384 | 2.182917x | 2.071087x | 289.6105 | 132.6714 | 3969.517 | 1516.140 | 2.614868 | 37.305786 |
| natural | 32767 | 2.223104x | 2.120104x | 297.2255 | 133.6984 | 3960.209 | 1453.093 | 3.602435 | 34.503159 |
| natural | 32768 | 2.222372x | 2.119049x | 297.1312 | 133.7001 | 3960.087 | 1453.064 | 3.602356 | 34.502258 |
| natural | 32769 | 2.221505x | 2.116417x | 297.5684 | 133.9490 | 3967.524 | 1456.073 | 3.602887 | 34.501358 |
| natural | 33027 | 2.223451x | 2.117863x | 298.7802 | 134.3768 | 3970.555 | 1456.750 | 3.615012 | 34.482242 |
| natural | 65535 | 2.234642x | 2.124137x | 365.9638 | 163.7684 | 4649.027 | 1754.654 | 4.559716 | 43.220569 |
| natural | 65536 | 2.246042x | 2.139343x | 365.9074 | 162.9120 | 4649.037 | 1754.688 | 4.559677 | 43.220184 |
| natural | 65537 | 2.241262x | 2.135143x | 366.1624 | 163.3734 | 4652.592 | 1755.512 | 4.559638 | 43.219616 |
| natural | 131072 | 2.202592x | 2.095699x | 374.7492 | 170.1401 | 4766.305 | 1803.527 | 4.948570 | 44.802139 |
| natural | 262144 | 2.204170x | 2.105780x | 381.1573 | 172.9256 | 4843.456 | 1829.174 | 5.175621 | 45.475792 |
| natural | 1048576 | 2.244033x | 2.154004x | 344.8196 | 153.6607 | 4425.437 | 1593.363 | 5.333766 | 37.434988 |
| random | 1024 | 0.747654x | 0.752581x | 225.3218 | 301.3716 | 2918.113 | 1645.748 | 0.005859 | 0.000000 |
| random | 4096 | 1.446860x | 1.401550x | 132.0446 | 91.2628 | 1717.045 | 639.479 | 0.028809 | 0.000732 |
| random | 8192 | 1.794987x | 1.783976x | 99.0855 | 55.2012 | 1481.315 | 460.055 | 0.061523 | 0.001831 |
| random | 16384 | 2.305161x | 2.334046x | 90.4147 | 39.2227 | 1446.098 | 391.304 | 0.124756 | 0.002197 |
| random | 32767 | 2.735059x | 2.786486x | 87.7477 | 32.0826 | 1435.733 | 349.531 | 0.237068 | 0.002930 |
| random | 32768 | 2.740593x | 2.779905x | 88.0254 | 32.1191 | 1435.727 | 349.529 | 0.237061 | 0.002930 |
| random | 32769 | 2.708798x | 2.770201x | 87.8988 | 32.4494 | 1443.052 | 352.529 | 0.237084 | 0.002930 |
| random | 33027 | 2.734139x | 2.771481x | 88.8162 | 32.4842 | 1443.163 | 352.262 | 0.238835 | 0.002907 |
| random | 65535 | 2.984825x | 3.042485x | 89.9182 | 30.1251 | 1457.870 | 333.273 | 0.348562 | 0.004395 |
| random | 65536 | 2.984127x | 3.045634x | 89.7621 | 30.0798 | 1457.898 | 333.276 | 0.348557 | 0.004395 |
| random | 65537 | 2.966156x | 3.032974x | 89.9037 | 30.3098 | 1461.538 | 334.775 | 0.348566 | 0.004394 |
| random | 131072 | 3.109431x | 3.193102x | 91.2078 | 29.3326 | 1482.543 | 327.946 | 0.403664 | 0.004440 |
| random | 262144 | 3.162576x | 3.249457x | 90.6376 | 28.6594 | 1481.815 | 322.549 | 0.432903 | 0.005093 |
| random | 1048576 | 3.235679x | 3.319774x | 88.7876 | 27.4402 | 1450.463 | 305.360 | 0.455208 | 0.005261 |
| periodic | 1024 | 0.660161x | 0.661563x | 210.5713 | 318.9697 | 2644.217 | 1918.709 | 0.956055 | 3.808594 |
| periodic | 4096 | 1.020835x | 1.021173x | 107.1625 | 104.9753 | 1567.617 | 905.299 | 0.989014 | 2.946533 |
| periodic | 8192 | 1.195133x | 1.224742x | 82.4280 | 68.9697 | 1401.481 | 742.871 | 0.994507 | 2.961548 |
| periodic | 16384 | 1.393867x | 1.403244x | 65.7806 | 47.1929 | 1224.384 | 624.945 | 0.997253 | 1.979370 |
| periodic | 32767 | 1.552908x | 1.594756x | 59.0075 | 37.9981 | 1183.801 | 584.572 | 0.998627 | 1.981994 |
| periodic | 32768 | 1.554712x | 1.592894x | 59.2581 | 38.1152 | 1183.806 | 584.572 | 0.998627 | 1.981995 |
| periodic | 32769 | 1.552052x | 1.598976x | 59.5526 | 38.3702 | 1191.143 | 587.572 | 0.998627 | 1.981995 |
| periodic | 33027 | 1.550876x | 1.588684x | 59.6677 | 38.4736 | 1190.765 | 587.233 | 0.998637 | 1.982015 |
| periodic | 65535 | 1.683792x | 1.725236x | 56.6472 | 33.6426 | 1167.234 | 565.892 | 0.999313 | 1.983246 |
| periodic | 65536 | 1.681518x | 1.722979x | 56.6762 | 33.7054 | 1167.206 | 565.892 | 0.999313 | 1.983246 |
| periodic | 65537 | 1.682484x | 1.717511x | 56.9999 | 33.8784 | 1170.865 | 567.391 | 0.999313 | 1.983246 |
| periodic | 131072 | 1.747571x | 1.794815x | 55.1071 | 31.5336 | 1158.971 | 556.572 | 0.999657 | 1.983871 |
| periodic | 262144 | 1.781511x | 1.834278x | 54.4627 | 30.5711 | 1154.841 | 551.905 | 0.999828 | 1.984184 |
| periodic | 1048576 | 1.813792x | 1.866506x | 54.3184 | 29.9474 | 1153.802 | 549.308 | 0.999957 | 1.984415 |
| repeated | 1024 | 0.656157x | 0.655389x | 208.3330 | 317.5049 | 2549.019 | 1918.369 | 0.994141 | 3.960938 |
| repeated | 4096 | 0.994270x | 0.987782x | 106.7911 | 107.4066 | 1543.986 | 905.200 | 0.998535 | 2.975098 |
| repeated | 8192 | 1.152558x | 1.178996x | 83.4808 | 72.4309 | 1389.879 | 742.835 | 0.999268 | 2.975830 |
| repeated | 16384 | 1.339269x | 1.369642x | 66.9454 | 49.9865 | 1218.042 | 624.798 | 0.999634 | 1.984131 |
| repeated | 32767 | 1.470852x | 1.513713x | 60.5252 | 41.1498 | 1180.511 | 584.487 | 0.999817 | 1.984375 |
| repeated | 32768 | 1.473996x | 1.512046x | 60.4897 | 41.0379 | 1180.385 | 584.468 | 0.999817 | 1.984375 |
| repeated | 32769 | 1.478072x | 1.519796x | 60.8553 | 41.1721 | 1187.736 | 587.470 | 0.999817 | 1.984314 |
| repeated | 33027 | 1.486106x | 1.520354x | 61.0933 | 41.1097 | 1187.390 | 587.133 | 0.999818 | 1.984316 |
| repeated | 65535 | 1.591148x | 1.627116x | 58.2634 | 36.6172 | 1165.485 | 565.841 | 0.999908 | 1.984405 |
| repeated | 65536 | 1.594817x | 1.633477x | 57.9898 | 36.3614 | 1165.494 | 565.841 | 0.999908 | 1.984375 |
| repeated | 65537 | 1.593469x | 1.634433x | 58.2549 | 36.5586 | 1169.168 | 567.342 | 0.999908 | 1.984345 |
| repeated | 131072 | 1.664661x | 1.705084x | 57.0693 | 34.2828 | 1158.088 | 556.542 | 0.999954 | 1.984436 |
| repeated | 262144 | 1.684936x | 1.733150x | 56.2549 | 33.3869 | 1154.368 | 551.883 | 0.999977 | 1.984467 |
| repeated | 1048576 | 1.690176x | 1.739905x | 55.3646 | 32.7567 | 1148.822 | 549.337 | 0.999994 | 1.984489 |

These rows distinguish more work per byte from a higher instruction cost. Larger natural text has more tree visits and candidate work per byte, on both implementations. Random input approaches a much lower reference cost per byte while Mech stays near 89 ns/byte at the larger sizes. Increasing relative ratios do not by themselves establish superlinear growth. Detailed blocks, cycles, absolute observations, contrary R247 rows and all memory fields remain in prefix-results.json and the cohort receipts.

### Every prefix configuration currently below the reference

| Input | Bytes | Retained R245 / Roc |
|---|---:|---:|
| natural | 1024 | **0.927359x** |
| random | 1024 | **0.747654x** |
| periodic | 1024 | **0.660161x** |
| repeated | 1024 | **0.656157x** |
| repeated | 4096 | **0.994270x** |

The original25 retain their 11 wins. The new prefix sweep has five below-reference configurations. These are separate coverage groups, with some duplicate input configurations; do not describe this as 16 newly distinct data sets or a compiler speedup. The standing scoreboard retains both groups and their measurement provenance.

## Memory: requested bytes, copies and live peaks are different

| 1 MiB input | Mech heap requests | Roc heap requests | Mech incremental heap peak B | Roc incremental heap peak B | Mech private payload peak B | Mech initialized bytes copied |
|---|---:|---:|---:|---:|---:|---:|
| natural | 1556 | 695 | 10823992 | 10371440 | 10550358 | 40676 |
| random | 914 | 373 | 10824168 | 10371440 | 10550486 | 15316 |
| periodic | 742 | 291 | 10824168 | 10371440 | 10550486 | 8752 |
| repeated | 742 | 291 | 10824168 | 10371440 | 10550486 | 8752 |

The complete normal calls and separate diagnostics have identical payload counters on all 56 prefixes. R245 and R247 also have identical whole-heap medians. Mech private payload peaks near 10.55 MB on 1 MiB input; initialized source-buffer copying is only 8,752–40,676 bytes. This is inconsistent with repeatedly copying the whole input inside compression. It does not count external input preparation or prove that every machine load/store is absent. Requested capacity is not copied data. Input is supplied through a borrowed current view and its alias stays unchanged. RSS/physical/lifetime numbers include loaded code, parsed harness metadata, input preparation and allocator retention; they are retained separately, not substituted for per-call payload or heap peaks.

## Instrumentation and qualification

The R239 aggregate machinery is ported narrowly to the retained compiler. Root and outlined current source sites, ordered-reduction spans and post-settlement returned-owner states are observed only with explicit diagnostic flags plus the phase observer. The normal image imports no observer and exactly restores the R245 Object and metadata. Current read-only source counters identify the two search clones and the separate skip implementation. No observer retains an owner or grants permission.

Fifty fresh full Mech diagnostic calls cover all original25 twice. A separately linked delivery filter avoids irrelevant map/lock work while retaining the same instrumented Object. Its selected records exactly equal the corresponding full records on all25; it then covers all81 twice,162 calls. The independently instrumented reference covers all81 twice,162 calls. Every aggregate repeats exactly, every raw output is exact, independent inflate succeeds, input/output aliases and capacities survive, and final private storage is zero. The 56 normal reference oracles and112 uninstrumented retained prefix checks also pass. Five fresh concrete-runtime/counter tests pass with warnings-as-errors; no broad703-suite rerun or fresh original/decoded small-image suite is claimed for inert diagnostics. Those remain qualified in the parent; original/decoded behavior of the next functional change will run anew.

All benchmark algorithms/tables/depth300/nice258/nearEOF rules, the 262144-entry U16 tree,1500513-word U32 cache,305257-word U64 path capacity, source allocation sites, output13 clear/reuse, allocator and complete-call boundary remain fixed. The largest new input uses the same entry; no allowance increases. The maintained ordinary exact admission/resource/accounting/canonical-publication/recovery contract remains omitted by this standalone path and is not counted as optimized away.

Compilation uses the same six native features, nightly2026-03-03/aarch64, Cranelift0.131.3, optimized compiler/producer and -Dwarnings. Reference source diagnostics add counters in two pinned modules and preserve all original algorithms. Its fresh optimized build exits2 for seven intentional dbg warnings; the binary exists and162 calls qualify it. Primary timings and CPU counters always use the untouched pinned reference worker. Full commands/dependencies are retained in build specifications and receipts.

## Failures and remaining work

Resolved setup failures: the old diagnostic patch needed adaptation after the phase emitter split; an early test link ran before its Object existed; the first reference build could not update its external compiler cache under restricted access and was rebuilt with normal cache access; an overstrict corpus-prefix assertion exposed the intentionally repeated held-out fixture; a source-manifest script tried decoding unchanged binary benchmark fixtures as UTF8 and was corrected to compare bytes first. None required changing an expected output, source algorithm, allowance or warning policy. Exact failed receipts and the fixture relationship are preserved. No unresolved introduced build, semantic or resource failure remains.

The same inherited seven maintained exclusions, unsupported fanout class, resident default-cap denials, experimental module-layout debt and standalone contract omissions remain. No full multi-MB corpus timing, separate ten-workload suite, full Cargo/math rebuild or ordinary-contract performance claim is made. The existing draft PR stays #973; exact-head hosted CI is cancelled by standing instruction and review is requested without waiting for CI.

Fresh random64 profiles are those of R247, including the exact unchanged retained R245 executable:3431/5503 running samples are in its native root, followed by path/tally/search. They remain current by byte identity; no additional profile is relabeled fresh in this diagnostic round. Samples locate work and include harness/validation; they are not per-function instruction partitions.

## Next bounded compiler experiment

Extend the existing short private-write proof only through an owner-free scalar read of the exact current array. First establish a minimal original/decoded read-between-writes witness with bounds/error/alias/retry controls. The real skip head sequence2596/2598 -> read2602 -> store2604 is currently split by that read. Compare all81 configurations after the compiler change; preserve all contrary timings. Phase-success ownership propagation remains a separate unimplemented investigation.

Existing code rejects every owner-using read when building a private interval, although a scalar read of the exact current buffer neither changes identity nor creates an alias. The skip sequence performs the third head store once per skip position:32,854 times on periodic33 and1,044,466 times on the 1 MiB periodic stream. Keep the same bounds-first canonical indexing and original read/error position; shared owners, different views, alias-producing values, calls, growth, unknown effects and joins must still decline. This is a proposed acceptance witness, not an implemented or measured gain. Helper geometry substitution was already tested in R243/R244 with adverse lifetime costs and is not proposed as unexplored work.

Phase-success ownership propagation is not implemented. The current census observes unique returned matching lanes, but that is not a static proof that no later alias or identity change intervenes. The measurement portions of the brief are complete except direct Mech per-block history; the subsequent functional compiler experiment remains outstanding. Parity is not complete.

Main commands (exact flags and expanded dependencies in receipts):

```text
python3 run-spec.py candidate-gpu candidate-producer
python3 run-spec.py census-worker census-tests census-prefix-worker
python3 run-census.py
python3 run-filtered-census.py
python3 run-reference-census.py
python3 prepare-prefix-fixtures.py
python3 smoke-prefix.py
python3 measure-prefix-primary.py
python3 measure-prefix-cpu.py
python3 measure-prefix-memory.py
python3 analyze-work-ledger.py
python3 analyze-prefix.py
python3 replay.py
```

## Pins and replay

- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- normal_object: `f74cd039e908a7bb502a9c4a703c53e3cc0103cc7ae3d6af7f91819a5b62d4c7`.
- diagnostic_object: `c16f9f22dabf3b10df3fa8f0deef781bc8b43f6ce3a9c8942a3645f742050967`.
- retained_worker: `58fa08ee1160db9febd35298f854874572741ba2b6f311b01eb1143d812e44ba`.
- phase_only_worker: `cb3f028b56a1753ba3cab8b51434208fa4b3c58a8dd968742e7cfdf02b620080`.
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`.
- reference_census_worker: `b03bf1ee12fdda7e9d2462ceec4b201be9828c601ea29c108acc4280b28efcfa`.
- full_census_worker: `ab163efe759d87134b45b725503e98d53c0be132695b8ab26f7568bcc7b614f1`.
- filtered_census_worker: `4f351512f82d524495f67ddca54f1eb754c44101c7505892017c427e1104a6f7`.
- runtime: `bbc12169d120eaa60af57904de32f599d594e24c80201f1173991a05746ff066`.
- compiler_patch: `a0c485d30c21f6d30c0246ffb2e2864006bab865b3de22f2a5954206bd6120e5`.
- compiler_source_manifest: `1d495de99171b65107f2135135734bc9d4e74cfcf53dffa72b7fcac440bfd936`.
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
- source_site_map: `55e7035a07c25c7cc00f88bdb24d5eac4ecec1ae2f00a8e09ffe546629fb0695`.
- reference_diagnostic_patch: `29a578afe1caad975d4157e6a7e8135d2dd250189e65a2b4c23281b1d9f1944b`.
- frozen_streams: `823c6a9cfb80999384ea4c0823ff3be12ca592330d02f7e3f2a5edf34f4f16a9`.

The packet contains the tested GPU source slice/patch, separate reference instrumentation, exact commands, frozen input streams/portable prefix fixtures, current aggregate counters, all56 elapsed/CPU/heap observations, source attribution and failure receipts. Native binaries/Objects, bytecode, full executable metadata/descriptors, raw traces/stacks, secrets and private environment exports are excluded. The three analyzers reproduce five byte-identical reports; verify-packet.py checks every archived file hash first. Replaying numerical reports does not build a compiler or execute compression.

## Public evidence

[Complete work census and controlled-prefix evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r248-work-ledger-prefix-scaling-bd9447eb91f2.tar.gz) (`sha256:bd9447eb91f237c11a89fa56af2304f23d9665c938056babb879973d8429c241`, 12630745 bytes). All2,115 public-download hashes, five numerical replays and816-file compiler-slice reconstruction verify.
