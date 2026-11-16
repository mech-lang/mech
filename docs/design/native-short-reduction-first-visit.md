# R276 — Evaluate the first short reduction visit directly

**Retain R276 with the R272 runtime.** Paired elapsed changes -0.289% across 81 cases. The measured control has 38/81 cases faster than Roc and the candidate has 38/81 (46.91%). The full pooled and block gates pass 81/81 and 81/81. The separate libdeflate comparison gives 3/81 candidate wins.

Paired elapsed changes -0.286% and -0.293% in the two cohorts. Cases crossing below parity include none. Cases crossing above include none. The largest paired slowdown is +0.791% on random-32769.

Pooled gate misses include none. Block gate misses include none.

Measured and published parent 781de7de7b0c94a73038acb2ce124e1866e04f26. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 plus the R275 source closure and its verified format-only overlay. The Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14, fixed level 12, compressor source and scratch capacities stay pinned.

## Experiment

The source-derived complete-child proof admits the same read geometry, checked arithmetic and work price. The existing no-growth capacity and owner gates admit the scope where the result carries the selected item. The change applies inside that scope after the same original admissions. Every winner must have strictly lower U32 cost, and ties preserve the incoming item. The selected-item expression uses the existing total extension on losing edges. The existing terminal-return recipe publishes the resulting minimum, item and cursor.

The first build adds a one-item dispatch after the long-interval test. It bypasses scalar loop state for a singleton and adds a conditional branch to the longer scalar entry. The revised build computes the first short visit once and enters the scalar loop only when another visit remains. The continuing path then forms the next cursor. The long vector entry retains the same threshold and the same vector remainder. Neither build changes the source proof or input.

The Object audit verifies 347 machine bodies. Both variants change exactly four no-growth bodies. Each grows from 1,408 to 1,488 bytes and stays at a 256-byte frame. The root body and its 57,072-byte frame match the control. The matcher changes previously adopted in R275 remain in the same bodies.

| One candidate through the next outer visit | Instructions | Conditional branches | Stack accesses |
|---|---:|---:|---:|
| retained | 37 | 4 | 2 |
| singleton | 34 | 4 | 0 |
| peeled | 33 | 4 | 0 |

The pinned reference's recurring reduction has 11 instructions and shares one comparison across two conditional selects. The independently compiled Rust control also shares comparison flags. The libdeflate scalar loop branches on the strict win. Both Mech variants still compare twice for cost and item. The static route table includes surrounding Mech admission and terminal instructions, so it has a wider boundary than those recurring peer visits.

| 23-case screen | All-case elapsed change | Natural 32 KiB | Natural 1 MiB | Pooled and block misses |
|---|---:|---:|---:|---|
| singleton | +0.109% | +1.507% | +1.699% | none / none |
| peeled | +0.045% | -0.611% | -0.330% | none / none |
| peeled-confirmation | -0.205% | -0.382% | -0.348% | none / none |

The independent six-case counter screen records 23,559,672 fewer natural 1 MiB instructions (-0.679%). Both timing screens and the separate six-input library comparison are kept alongside the full matrix.

## Matched frontier

The two ratio columns for each peer use the same four-actor comparison. The elapsed-change column uses the two primary timing cohorts. Both include the complete compression call and validation uses the exact expected stream and independent inflation.

| Case | R275 / Roc | Candidate / Roc | R275 / libdeflate | Candidate / libdeflate | Elapsed change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.197751× | 0.198232× | 2.520126× | 2.526258× | +0.335% |
| prefix-repeated-131072 | 1.012775× | 1.012343× | 1.699752× | 1.699029× | -0.029% |
| prefix-repeated-1048576 | 1.034131× | 1.035177× | 1.626138× | 1.627783× | -0.118% |
| prefix-periodic-1048576 | 1.062649× | 1.066074× | 1.801070× | 1.806875× | +0.034% |
| prefix-natural-32768 | 1.478091× | 1.471887× | 1.341623× | 1.335991× | -0.235% |
| prefix-natural-1048576 | 1.508769× | 1.499167× | 1.219665× | 1.211903× | -0.492% |
| prefix-random-1048576 | 2.117142× | 2.124698× | 2.999997× | 3.010703× | +0.432% |

## Correctness, counters and profiles

- The original and decoded final images match in Object and metadata. The final candidate passed 324 complete compression checks and the first candidate passed 162. Both preserve the exact expected stream, independent inflation, allocation capacities, aliases and zero remaining private owners.
- The original and decoded linked builds passed 126 targeted controls. They cover one, two, three, 15, 16 and 17 candidates, incoming and later ties, valid and invalid geometry, finite capacities, a rejected allocation, first assertion origin, nonzero seeds and publication and alias behavior.
- The dedicated samples were taken from the two exact candidate executables after the screens and before the full measurements. The profiler validates output during its independent session. Sample shares describe observed instruction locations and never serve as retired-instruction counts.
- The six logical heap families agree for the 81 cases. The library run includes 245,664 timed calls.

| Case | Control instructions | Candidate instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,454 | 812,377 | -0.009% | +3.606% |
| prefix-repeated-131072 | 89,528,028 | 89,525,889 | -0.002% | +0.128% |
| prefix-repeated-1048576 | 713,792,258 | 713,753,890 | -0.005% | +0.030% |
| prefix-periodic-1048576 | 706,233,912 | 706,195,264 | -0.005% | +0.037% |
| prefix-natural-32768 | 95,186,055 | 94,513,398 | -0.707% | +0.050% |
| prefix-natural-1048576 | 3,470,883,028 | 3,447,191,371 | -0.683% | -0.219% |
| prefix-random-1048576 | 874,125,529 | 874,097,814 | -0.003% | -0.204% |

| Candidate and input | Running samples | Path-parser body | Matcher body |
|---|---:|---:|---:|
| singleton, prefix-natural-1048576 | 5569 | 2701 | 2484 |
| peeled, prefix-natural-1048576 | 5366 | 2552 | 2424 |
| peeled, prefix-natural-32768 | 5355 | 2497 | 2183 |
| peeled, prefix-periodic-1048576 | 5349 | 1205 | 3380 |
| peeled, prefix-random-1048576 | 5358 | 871 | 3234 |

## All primary timings

| Input | Bytes | R275 / Roc | Candidate / Roc | Candidate / R275 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.139686× | 2.112515× | 0.987301× | pass/pass |
| one-stored | 1 | 2.066057× | 2.055192× | 0.994742× | pass/pass |
| seven-stored-prefix | 7 | 2.324711× | 2.282936× | 0.982030× | pass/pass |
| eight-literals | 8 | 0.196852× | 0.197512× | 1.003349× | pass/pass |
| eight-overlap | 8 | 0.186327× | 0.186672× | 1.001853× | pass/pass |
| 320-overlap | 320 | 0.225491× | 0.225759× | 1.001186× | pass/pass |
| eight-literals-prefix | 8 | 0.197775× | 0.197961× | 1.000944× | pass/pass |
| 128-biased-dynamic | 128 | 0.266946× | 0.267776× | 1.003110× | pass/pass |
| 32-high-entropy-stored | 32 | 0.268486× | 0.267354× | 0.995781× | pass/pass |
| 16384-distribution-change | 16384 | 1.678881× | 1.669233× | 0.994253× | pass/pass |
| 33027-periodic-window | 33027 | 0.923577× | 0.914155× | 0.989799× | pass/pass |
| random-64 | 64 | 0.284383× | 0.284409× | 1.000092× | pass/pass |
| random-256 | 256 | 0.346258× | 0.346285× | 1.000079× | pass/pass |
| random-1024 | 1024 | 0.433791× | 0.434408× | 1.001423× | pass/pass |
| repeated-1024 | 1024 | 0.297396× | 0.297266× | 0.999561× | pass/pass |
| random-32769 | 32769 | 1.747546× | 1.761374× | 1.007913× | pass/pass |
| repeated-32769 | 32769 | 0.890137× | 0.884835× | 0.994044× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.544777× | 0.543069× | 0.996864× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.430594× | 1.420236× | 0.992760× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.479832× | 1.475368× | 0.996983× | pass/pass |
| heldout-window-32767 | 32767 | 1.479287× | 1.471579× | 0.994789× | pass/pass |
| heldout-window-32768 | 32768 | 1.476489× | 1.471177× | 0.996402× | pass/pass |
| heldout-slides-65537 | 65537 | 1.518651× | 1.512431× | 0.995904× | pass/pass |
| heldout-random-65537 | 65537 | 1.921727× | 1.917626× | 0.997866× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.642940× | 1.637692× | 0.996805× | pass/pass |
| prefix-natural-1024 | 1024 | 0.545807× | 0.543775× | 0.996276× | pass/pass |
| prefix-natural-4096 | 4096 | 1.109456× | 1.097786× | 0.989481× | pass/pass |
| prefix-natural-8192 | 8192 | 1.319534× | 1.303037× | 0.987498× | pass/pass |
| prefix-natural-16384 | 16384 | 1.432240× | 1.422715× | 0.993350× | pass/pass |
| prefix-natural-32767 | 32767 | 1.473739× | 1.467507× | 0.995771× | pass/pass |
| prefix-natural-32768 | 32768 | 1.475656× | 1.472185× | 0.997648× | pass/pass |
| prefix-natural-32769 | 32769 | 1.478529× | 1.471175× | 0.995026× | pass/pass |
| prefix-natural-33027 | 33027 | 1.478886× | 1.471626× | 0.995091× | pass/pass |
| prefix-natural-65535 | 65535 | 1.486696× | 1.479093× | 0.994886× | pass/pass |
| prefix-natural-65536 | 65536 | 1.483951× | 1.476915× | 0.995258× | pass/pass |
| prefix-natural-65537 | 65537 | 1.477627× | 1.470518× | 0.995189× | pass/pass |
| prefix-natural-131072 | 131072 | 1.491319× | 1.477074× | 0.990448× | pass/pass |
| prefix-natural-262144 | 262144 | 1.482803× | 1.479936× | 0.998066× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.509950× | 1.502515× | 0.995076× | pass/pass |
| prefix-random-1024 | 1024 | 0.438096× | 0.438716× | 1.001416× | pass/pass |
| prefix-random-4096 | 4096 | 0.789260× | 0.784700× | 0.994224× | pass/pass |
| prefix-random-8192 | 8192 | 1.042887× | 1.038525× | 0.995818× | pass/pass |
| prefix-random-16384 | 16384 | 1.442617× | 1.441450× | 0.999191× | pass/pass |
| prefix-random-32767 | 32767 | 1.743864× | 1.744720× | 1.000491× | pass/pass |
| prefix-random-32768 | 32768 | 1.749505× | 1.742006× | 0.995713× | pass/pass |
| prefix-random-32769 | 32769 | 1.740957× | 1.737026× | 0.997742× | pass/pass |
| prefix-random-33027 | 33027 | 1.748351× | 1.745025× | 0.998097× | pass/pass |
| prefix-random-65535 | 65535 | 1.947258× | 1.940210× | 0.996381× | pass/pass |
| prefix-random-65536 | 65536 | 1.946768× | 1.943494× | 0.998318× | pass/pass |
| prefix-random-65537 | 65537 | 1.938248× | 1.931931× | 0.996741× | pass/pass |
| prefix-random-131072 | 131072 | 2.052549× | 2.046925× | 0.997260× | pass/pass |
| prefix-random-262144 | 262144 | 2.100325× | 2.098105× | 0.998943× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.119479× | 2.128626× | 1.004316× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.363033× | 0.363354× | 1.000886× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.555422× | 0.549179× | 0.988759× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.673853× | 0.672266× | 0.997646× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.791894× | 0.789195× | 0.996591× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.902306× | 0.900405× | 0.997893× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.906272× | 0.905832× | 0.999515× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.911612× | 0.908218× | 0.996277× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.915530× | 0.912054× | 0.996203× | pass/pass |
| prefix-periodic-65535 | 65535 | 0.987004× | 0.983384× | 0.996332× | pass/pass |
| prefix-periodic-65536 | 65536 | 0.989031× | 0.984854× | 0.995777× | pass/pass |
| prefix-periodic-65537 | 65537 | 0.988749× | 0.984506× | 0.995708× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.032119× | 1.030987× | 0.998904× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.051223× | 1.050596× | 0.999403× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.062554× | 1.062913× | 1.000338× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.297058× | 0.297647× | 1.001983× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.496881× | 0.494942× | 0.996097× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.625758× | 0.625584× | 0.999721× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.746284× | 0.745723× | 0.999249× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.862849× | 0.860664× | 0.997468× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.864688× | 0.860273× | 0.994894× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.880858× | 0.879722× | 0.998710× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.905896× | 0.906171× | 1.000303× | pass/pass |
| prefix-repeated-65535 | 65535 | 0.961567× | 0.961362× | 0.999787× | pass/pass |
| prefix-repeated-65536 | 65536 | 0.966234× | 0.963770× | 0.997450× | pass/pass |
| prefix-repeated-65537 | 65537 | 0.964977× | 0.967756× | 1.002880× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.010223× | 1.009931× | 0.999711× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.029534× | 1.028950× | 0.999433× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.036653× | 1.035425× | 0.998815× | pass/pass |

[Verified patches, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r276-short-reduction-first-visit-0893a086bfb4.tar.gz). SHA-256 `0893a086bfb49236f9876b04adf6be081fab9a80174c70de579c14414fa61ed4`.
