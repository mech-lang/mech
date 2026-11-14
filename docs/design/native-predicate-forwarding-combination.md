# R254: measure predicate proof followed by identity forwarding

**Archive the combination and retain R253.** The broad R252 forwarding pass still hurts the main large random/text cases after R253 removes impossible branches. Smaller code does not establish faster execution. No benchmark algorithm or compiler source changes in this round; the unchanged R253 producer enables the two existing passes in sequence.

Git parent `63a5fcf0473c9f61346b81911b808bcb4dfb8835`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Ordinary R215 remains `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/MBC 215/107/39/14. The experimental 822-file compiler is reused exactly from the verified [R253 packet](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r253-current-predicates-d1a351896db9.tar.gz), SHA-256 `d1a351896db95a70115194d1583a7b0a3f7271535fe5997ccc2d9a3881b950d7`. `source-reuse.json` pins the complete source manifest and actual producer. No new source patch is required.

## Mechanism and actual code

The combination forwards 913 pure SSA edges using 1,553 selected conditions and 1,011 typed repair parameters. Root code changes 940,264 -> 939,104 bytes, while the frame grows 50,560 -> 52,160 bytes. Complete linked size stays 1,884,504 bytes; static call sites stay 4,215. All 136 original routines, seven selected vector bodies and their metadata are exact. Source-location emission reproduces the complete measured Object/metadata. This is not evidence that the surviving zero-trip transport disappears as a unit.

The pass still stops at loads, stores, calls, checked effects, unsupported shared uses and reentry. It forwards immutable words without supplying memory, ownership or resource permission. The source algorithm, tables, depth 300, nice length 258, near-EOF limits, buffer capacities, allocation sites, pinned allocator and timing boundaries remain unchanged.

## Two matched complete-call confirmations

Two cohorts each use 24 balanced blocks, every control/candidate/reference order four times, and 200 ms warmup per actor/case. Both sides use runtime input, fixed level 12 and a matched live borrowed output. Full compression, scratch construction, output transfer and private cleanup are timed. Validation and logging remain outside. CPU counters and heap are separate cohorts; all builds, source tests, observations and profiles run outside them.

Ratios below use pooled observation medians against the same fresh Roc denominator. Changes use balanced block means. The retained R253 source has not changed: differences from its previous published absolute ratios include a fresh reference denominator and new observations. Counter and elapsed cohorts must not be multiplied or conflated.

| Case | Retained R253 / Roc | Archived R254 / Roc | Elapsed change | Cohort A | Cohort B | Instructions | Cycles |
|---|---:|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.416931x | 0.434926x | +3.9367% | +7.1242% | -0.4249% | -1.1652% | -5.2445% |
| heldout-random-65537 | 2.791868x | 2.862779x | +2.5399% | +2.8069% | +2.3198% | +3.2128% | +2.2053% |
| prefix-natural-32768 | 2.130545x | 2.132447x | +0.0893% | +0.3492% | +0.0345% | +0.8202% | -0.0608% |
| prefix-random-32768 | 2.541059x | 2.603612x | +2.4617% | +2.3359% | +2.9327% | +3.2383% | +2.2067% |
| prefix-periodic-32768 | 1.446918x | 1.437282x | -0.6660% | -1.0189% | -0.5302% | +0.0524% | -1.1757% |
| prefix-repeated-32768 | 1.372578x | 1.357252x | -1.1166% | -1.4003% | -0.6534% | +0.0578% | -0.8347% |
| prefix-natural-1048576 | 2.180404x | 2.187823x | +0.3403% | +0.3330% | +0.3118% | +0.7747% | +0.6574% |
| prefix-random-1048576 | 3.083508x | 3.163219x | +2.5851% | +2.5408% | +2.6086% | +3.1225% | +2.3349% |
| prefix-periodic-1048576 | 1.693255x | 1.681601x | -0.6882% | -0.7089% | -0.5880% | +0.0906% | -0.5697% |
| prefix-repeated-1048576 | 1.583638x | 1.574199x | -0.5961% | -0.6397% | -0.5941% | +0.0906% | -0.6369% |

Across all 81, 31 balanced elapsed medians improve and 50 worsen; descriptive geometric change is +0.7754%. Pooled point gates pass 76/81 and balanced gates 77/81. Cohort A passes 70/81 point and 71/81 balanced; cohort B passes 81/81 of both. No gate or allowance is changed. These introduced performance regressions are the reason for archiving the combination.

Eight bytes has contrary timing confirmations: the first worsens while the second improves slightly. Its independent counter cohort improves both instructions and cycles. Preserve these observations; do not label the pooled elapsed loss a proved uniform instruction regression. Random/text 1 MiB both worsen in both elapsed confirmations and in instructions. The small repetitive-input gains are retained as evidence, not combined with unrelated cohorts to claim a general gain.

The CPU cohort has 16 instruction improvements and 65 increases, with 52 cycle improvements and 29 increases. All 81 payload allocations, requested capacity, initialized copying, zeroing, private peak and listed whole-heap medians remain identical between Mech variants. Requested capacity is not initialized bytes copied or OS RSS.

## Current-image profile and corrected external labels

| Case | Running samples | Root | Numeric phases | Other/external |
|---|---:|---:|---:|---:|
| eight-literals | 5335 | 1004 | 673 | 3658 |
| prefix-natural-1048576 | 5499 | 845 | 4609 | 45 |
| prefix-periodic-1048576 | 5465 | 2226 | 3148 | 91 |
| prefix-random-1048576 | 5567 | 3448 | 2078 | 41 |

Current exact-image PCs and narrow disassembly still show root state movement on random input. Text spends 2,797 samples in path phase 15296; 1,525 lie in its 128-byte window at offset 1,536. That window contains the scalar cost-reduction loop, including consecutive comparisons/selects and its backedge. The long-interval four-lane vector path already exists. Do not propose making this phase native or adding its existing SIMD path as new work. The 1 MiB text invocation enters this phase only 23 times, so rare entry packet loads are not a plausible explanation for its dominant loop samples.

The profile mapper now checks the actual Mach-O text bounds before assigning a local symbol. The earlier mapper assigned out-of-image PCs to the last local symbol, incorrectly labeling 3,404 eight-byte R253 samples as `mi_arena_purge.cold.1`. They are now external/unmapped, not attributed to allocator purge. Replaying the old capture proves every generated-root/numeric count unchanged, including the large-input distributions. Timing and hardware-counter measurements are unaffected. The corrected per-phase PC aggregation and all raw-to-aggregate steps are archived; CPU-time samples are not retired-instruction partitions or removable-time budgets.

Actual pinned Roc entry/prologue inspection confirms its `roc_compress` wrapper calls the hashed main routine. Its symbol span is 21,788 bytes and its visible prologue reserves 4,928 stack bytes including saves, versus Mech root 50,560 retained or 52,160 candidate bytes. These are different function decompositions, not whole-call peak-stack equivalence or a timing explanation. The reference binary is unchanged. Fresh C and Rust shared-comparison witnesses emit one comparison feeding two conditional selects; the actual hot Mech scalar window repeats the same comparison between those selects. This establishes a bounded generic backend opportunity, not its dynamic saving or whole-call benefit. Both assembly witnesses and build commands are archived. R253’s other scalar controls remain inherited; no C/Rust compression timing is claimed.

## Qualification and failure disposition

Fresh: 3,500 source calls across six programs/seven variants and original/decoded forms, including 700 zero trips; all seven small image pairs exact. Controls retain current arguments, fresh visits, input/output aliases, prior publication, failures, cleanup and retry. Another 162 immediate complete smokes cover all 81 exact golden streams and independent inflation. Another 162 observed calls reproduce all 81 R253 source-phase counts. Every timing/counter/profile call also validates exact bytes/inflate/aliases/capacity, and workers finish with zero private payload and owners. Instrumented images are excluded from primary timing.

Unchanged R253 compiler source and its 712-test maintained qualification/same seven exclusions are inherited, not rerun or relabeled fresh. New worker/control builds retain warnings-as-errors, opt3 and pinned nightly-2026-03-03 on Apple ARM64. Full decoded compression images are not emitted this round; decoded behavior is exercised by the seven source pairs. The full source-annotation image is freshly byte-identical. The forwarding-disabled control is the existing pinned R253 image, not a newly claimed emission.

Introduced: measured performance regressions above, resolved by leaving forwarding disabled; no unresolved semantic failure or increased expectation. Inherited diagnostic issue: external-PC label error, repaired and replayed without changing native counts. Inherited: seven maintained exclusions, ordinary extra managed-contract omissions and enclosing child-funding/selection refusal. Ordinary IR215 resource admission/accounting/publication/recovery remains unchanged; private compact aliases/cleanup are not restoration of that extra contract. No completed hosted CI is claimed; cancel exact-head CI under the standing instruction and preserve review workflows.

## Reproduction and next step

`experiment-plan.json` declares the candidate before measurement. `run.py`, `run-spec.py`, worker build specifications, emission/test receipts and fixed fixture manifest preserve exact commands, features, binary hashes and timings. `measure-primary-a.py`, `measure-rest.py` and the three individual scripts run the four isolated cohorts. `profile-compact-pcs.py`, `prepare-profile.py`, `phase-hot-windows.json`, narrow assembly and `audit-R253-profile.py` establish current and corrected historical attribution. The offline analyses regenerate this report from retained aggregates without executing a compiler or compressor.

Keep the broad identity-forwarding candidate closed. Next inspect the current empty/short-reduction control and the enclosing recurrence’s exact funding/cold-handoff requirement. Large random/text throughput remains the target, with every one of the 81 controls and eight-byte <=1x retained. Empty outer reduction, multi-position composition and the single R247 placement control remain open; phase-only isolation is already complete and the funding guard is not weakened.

## All 81 configurations

| Case | Bytes | Retained R253 / Roc | Archived R254 / Roc | Balanced change | Instructions change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | -5.3628% | -0.2079% |
| one-stored | 1 | 1.976190x | 1.976190x | +1.3993% | -0.1256% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | +0.0807% | +1.3861% |
| eight-literals | 8 | 0.416931x | 0.434926x | +3.9367% | -1.1652% |
| eight-overlap | 8 | 0.407556x | 0.432289x | +5.6347% | -1.2326% |
| 320-overlap | 320 | 0.464849x | 0.484281x | +2.5100% | -0.8817% |
| eight-literals-prefix | 8 | 0.417555x | 0.436693x | +3.2921% | -1.1652% |
| 128-biased-dynamic | 128 | 0.423843x | 0.445584x | +5.2414% | -1.0313% |
| 32-high-entropy-stored | 32 | 0.432775x | 0.460897x | +6.6654% | -0.6563% |
| 16384-distribution-change | 16384 | 2.341313x | 2.353894x | +0.5374% | +0.9242% |
| 33027-periodic-window | 33027 | 1.436220x | 1.426367x | -0.6860% | +0.0524% |
| random-64 | 64 | 0.461064x | 0.489083x | +5.8650% | -0.3857% |
| random-256 | 256 | 0.542358x | 0.563540x | +3.1700% | +0.6211% |
| random-1024 | 1024 | 0.639798x | 0.675071x | +4.5818% | +1.3592% |
| repeated-1024 | 1024 | 0.555818x | 0.579535x | +4.4252% | -0.5214% |
| random-32769 | 32769 | 2.513835x | 2.586962x | +2.9089% | +3.2197% |
| repeated-32769 | 32769 | 1.372088x | 1.359102x | -0.9465% | +0.0517% |
| reymont-prefix-1024 | 1024 | 0.800973x | 0.821765x | +2.1123% | +0.3909% |
| reymont-prefix-16384 | 16384 | 2.057270x | 2.061354x | +0.1985% | +0.8079% |
| reymont-prefix-32769 | 32769 | 2.126448x | 2.129332x | +0.1356% | +0.8179% |
| heldout-window-32767 | 32767 | 2.131203x | 2.134625x | +0.1606% | +0.8200% |
| heldout-window-32768 | 32768 | 2.129094x | 2.131727x | +0.1237% | +0.8204% |
| heldout-slides-65537 | 65537 | 2.211736x | 2.215980x | +0.1919% | +0.8075% |
| heldout-random-65537 | 65537 | 2.791868x | 2.862779x | +2.5399% | +3.2128% |
| heldout-mixed-131073 | 131073 | 2.269250x | 2.272002x | +0.1213% | +0.7683% |
| prefix-natural-1024 | 1024 | 0.803887x | 0.818777x | +1.6827% | +0.3909% |
| prefix-natural-4096 | 4096 | 1.572704x | 1.563078x | -0.6121% | +0.8101% |
| prefix-natural-8192 | 8192 | 1.862594x | 1.866224x | +0.1949% | +0.8415% |
| prefix-natural-16384 | 16384 | 2.058570x | 2.060594x | +0.0983% | +0.8005% |
| prefix-natural-32767 | 32767 | 2.132498x | 2.134013x | +0.0710% | +0.8203% |
| prefix-natural-32768 | 32768 | 2.130545x | 2.132447x | +0.0893% | +0.8202% |
| prefix-natural-32769 | 32769 | 2.131449x | 2.133620x | +0.1018% | +0.8187% |
| prefix-natural-33027 | 33027 | 2.124321x | 2.127111x | +0.1313% | +0.8181% |
| prefix-natural-65535 | 65535 | 2.141611x | 2.139072x | -0.1186% | +0.7097% |
| prefix-natural-65536 | 65536 | 2.130765x | 2.130849x | +0.0039% | +0.7084% |
| prefix-natural-65537 | 65537 | 2.162356x | 2.164367x | +0.0930% | +0.7042% |
| prefix-natural-131072 | 131072 | 2.134067x | 2.137101x | +0.1422% | +0.6949% |
| prefix-natural-262144 | 262144 | 2.145937x | 2.152422x | +0.3022% | +0.6855% |
| prefix-natural-1048576 | 1048576 | 2.180404x | 2.187823x | +0.3403% | +0.7747% |
| prefix-random-1024 | 1024 | 0.648466x | 0.678212x | +4.1172% | +1.3605% |
| prefix-random-4096 | 4096 | 1.212611x | 1.243166x | +2.5197% | +2.6654% |
| prefix-random-8192 | 8192 | 1.570189x | 1.615219x | +2.8678% | +3.1487% |
| prefix-random-16384 | 16384 | 2.122976x | 2.156758x | +1.5913% | +3.2477% |
| prefix-random-32767 | 32767 | 2.532465x | 2.605475x | +2.8830% | +3.2383% |
| prefix-random-32768 | 32768 | 2.541059x | 2.603612x | +2.4617% | +3.2383% |
| prefix-random-32769 | 32769 | 2.528863x | 2.592185x | +2.5040% | +3.2209% |
| prefix-random-33027 | 33027 | 2.532118x | 2.589776x | +2.2771% | +3.2199% |
| prefix-random-65535 | 65535 | 2.798722x | 2.864512x | +2.3507% | +3.1467% |
| prefix-random-65536 | 65536 | 2.803080x | 2.865412x | +2.2237% | +3.1467% |
| prefix-random-65537 | 65537 | 2.789249x | 2.848817x | +2.1356% | +3.1383% |
| prefix-random-131072 | 131072 | 2.933092x | 2.998981x | +2.2464% | +2.9654% |
| prefix-random-262144 | 262144 | 2.999043x | 3.067335x | +2.2771% | +3.0125% |
| prefix-random-1048576 | 1048576 | 3.083508x | 3.163219x | +2.5851% | +3.1225% |
| prefix-periodic-1024 | 1024 | 0.582820x | 0.589613x | +0.8903% | -0.4007% |
| prefix-periodic-4096 | 4096 | 0.901405x | 0.900914x | -0.0544% | -0.1667% |
| prefix-periodic-8192 | 8192 | 1.087992x | 1.078040x | -0.9147% | -0.0565% |
| prefix-periodic-16384 | 16384 | 1.273057x | 1.262416x | -0.8359% | +0.0108% |
| prefix-periodic-32767 | 32767 | 1.446102x | 1.428295x | -1.2314% | +0.0525% |
| prefix-periodic-32768 | 32768 | 1.446918x | 1.437282x | -0.6660% | +0.0524% |
| prefix-periodic-32769 | 32769 | 1.435549x | 1.417969x | -1.2247% | +0.0521% |
| prefix-periodic-33027 | 33027 | 1.436424x | 1.425200x | -0.7814% | +0.0524% |
| prefix-periodic-65535 | 65535 | 1.557299x | 1.545913x | -0.7312% | +0.0741% |
| prefix-periodic-65536 | 65536 | 1.556858x | 1.542617x | -0.9148% | +0.0742% |
| prefix-periodic-65537 | 65537 | 1.551953x | 1.539446x | -0.8059% | +0.0739% |
| prefix-periodic-131072 | 131072 | 1.627331x | 1.616391x | -0.6723% | +0.0853% |
| prefix-periodic-262144 | 262144 | 1.662538x | 1.651109x | -0.6875% | +0.0898% |
| prefix-periodic-1048576 | 1048576 | 1.693255x | 1.681601x | -0.6882% | +0.0906% |
| prefix-repeated-1024 | 1024 | 0.556795x | 0.566037x | +0.8693% | -0.5221% |
| prefix-repeated-4096 | 4096 | 0.866526x | 0.854134x | -1.4301% | -0.1798% |
| prefix-repeated-8192 | 8192 | 1.045081x | 1.030149x | -1.4288% | -0.0605% |
| prefix-repeated-16384 | 16384 | 1.230123x | 1.212366x | -1.4435% | +0.0108% |
| prefix-repeated-32767 | 32767 | 1.371927x | 1.359697x | -0.8914% | +0.0523% |
| prefix-repeated-32768 | 32768 | 1.372578x | 1.357252x | -1.1166% | +0.0578% |
| prefix-repeated-32769 | 32769 | 1.369621x | 1.355752x | -1.0126% | +0.0519% |
| prefix-repeated-33027 | 33027 | 1.376601x | 1.360710x | -1.1544% | +0.0523% |
| prefix-repeated-65535 | 65535 | 1.482930x | 1.467044x | -1.0713% | +0.0738% |
| prefix-repeated-65536 | 65536 | 1.480168x | 1.466074x | -0.9521% | +0.0737% |
| prefix-repeated-65537 | 65537 | 1.478054x | 1.465739x | -0.8332% | +0.0736% |
| prefix-repeated-131072 | 131072 | 1.544085x | 1.531743x | -0.7993% | +0.0848% |
| prefix-repeated-262144 | 262144 | 1.576509x | 1.565041x | -0.7275% | +0.0892% |
| prefix-repeated-1048576 | 1048576 | 1.583638x | 1.574199x | -0.5961% | +0.0906% |

## Public evidence

[Evidence packet](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r254-predicate-forwarding-303bd52f963e.tar.gz), 16,531,511 bytes, SHA-256 `303bd52f963e6f4a00c3584acabf46dfa1a793326f1c0a6981871de331c97251`. Downloaded public bytes verify all 1,241 file hashes, five byte-identical offline reports, and the exact 822-file R253 source used without changes. The source itself remains in the linked R253 packet. Native executables/Objects, bytecode, full compressor descriptors/IR/maps and raw traces are excluded.
