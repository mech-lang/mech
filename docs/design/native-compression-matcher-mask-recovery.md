# Recover matcher bounds proofs

**R355 remains the retained comparison during recovery.**

The candidate beats Roc on 47/81 cases (58.02%) and libdeflate on 10/81. In the same run R355 wins 47/81 against Roc and 9/81 against libdeflate. The paired candidate-to-R355 geometric mean is 0.964412×. The archive preserves all 11 paired point estimates above 1×, 3 above 1.05×, and 6,819 zero timer observations.

| Input | R355/Roc | Candidate/Roc | R355/libdeflate | Candidate/libdeflate | Candidate/R355 |
| --- | ---: | ---: | ---: | ---: | ---: |
| eight-literals | 0.183094× | 0.172409× | 2.298759× | 2.164610× | 0.941643× |
| reymont-prefix-32769 | 1.185094× | 1.185253× | 1.078299× | 1.078443× | 1.000134× |
| prefix-natural-32768 | 1.186129× | 1.184494× | 1.080889× | 1.079399× | 0.998621× |
| prefix-natural-131072 | 1.195409× | 1.205012× | 0.980352× | 0.988227× | 1.008033× |
| prefix-natural-262144 | 1.200211× | 1.212183× | 0.970190× | 0.979868× | 1.009976× |
| prefix-natural-1048576 | 1.211190× | 1.218622× | 0.976196× | 0.982185× | 1.006135× |
| prefix-random-1048576 | 1.478095× | 1.420023× | 2.090022× | 2.007909× | 0.960712× |
| prefix-periodic-1048576 | 0.638044× | 0.596969× | 1.077612× | 1.008240× | 0.935624× |
| prefix-repeated-1048576 | 0.646520× | 0.606277× | 1.016746× | 0.953458× | 0.937755× |

## Direct comparison with the append recovery

The seven-case attribution screen measures exact R355, append recovery and the matcher candidate in the same blocks. It retains 41,832 complete calls.

| Input | Append/R355 | Matcher/R355 | Matcher/append |
| --- | ---: | ---: | ---: |
| eight-literals | 0.979287× | 0.965946× | 0.986377× |
| seven-stored-prefix | 1.101388× | 1.124136× | 1.020654× |
| prefix-natural-262144 | 1.043063× | 1.002382× | 0.960998× |
| prefix-periodic-262144 | 1.011601× | 0.931919× | 0.921232× |
| prefix-periodic-1048576 | 1.010236× | 0.935028× | 0.925555× |
| prefix-repeated-1048576 | 0.986683× | 0.934229× | 0.946838× |
| prefix-random-1048576 | 0.991032× | 0.955052× | 0.963694× |

## Independent counters and samples

| Input | Retired instructions / append | CPU cycles / append | Matcher samples |
| --- | ---: | ---: | ---: |
| prefix-natural-262144 | 0.962617× | 0.967775× | 34.9% |
| prefix-periodic-1048576 | 0.964269× | 0.927255× | 61.1% |
| prefix-repeated-1048576 | 0.963857× | 0.942588× | 54.9% |
| prefix-random-1048576 | 0.940286× | 0.967302× | 58.8% |
| eight-literals | 0.977802× | 0.986372× | 13.5% |

The counter companion uses the exact candidate Object in a separate set of 20 balanced calls per actor per case. The profile uses the exact linked elapsed-timing worker in its own acquisition. Sample percentages include validation.

## Generated code and qualification

A low-bit mask of an exact I8 integer comparison preserves that comparison. The bounds and selected-edge analyzers now recognize the identity. Lossy masks and arbitrary byte values retain their checks. Six focused controls passed, including successful, rejected and retry paths.

The phase-only comparison reproduces the exact previous matcher body. The changed matcher combines 17 readable windows, proves eight scalar spans, and retains all 60 wide loads and the six widening headers. Four successful byte comparison paths shrink from 12 or 10 instructions to six. The first search mismatch bypasses an 18-instruction corridor on its no-write edge. Other predecessors keep the required cache work.

Of 351 reported generated bodies, 188 change. The linked matcher is byte-identical to the changed phase-only body, with code size 11,292 to 10,616 bytes and stack use 592 to 576 bytes. All four checked append variants still carry four words and retain both entry proofs.

Compiler base `ef3beb1985bbbf8e17e9436d1013ebb4a7eb215f` plus patch SHA-256 `d8263537791901f8cc348b3f798ad1252a9189c0cc06541fb3bf0aac5f9f068b`. Benchmark source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. Original Object `a0a0452e95b8f9ae7b06a5b6c02896817613ee40903976cad50a46579ab59179`. Linked worker `88792945bb8610122b119c3a57036747259449404a5e7b8e50e566b9411e012b`.

The original artifact passed 162 linked calls with exact output, independent inflation, alias preservation and cleanup. The full timing retains 24 rotated actor orders and 200 ms warmup per actor per case, including full compression, scratch construction, output transfer and private cleanup. Decoded and repeated recovery qualification remain separate acceptance steps.

[Paired measurements, generated-code evidence and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/compression-matcher-mask-0b6c5c5a2b72.tar.gz).
