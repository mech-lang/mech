# R329 — Bound indices with immutable array payloads

**Keep R327 with the R272 runtime.** The 256 KiB natural witness changes -0.195% in the primary screen and +0.138% in the four-actor comparison. Both 1.05 gates pass 15/15. The 15-case primary geometric mean changes +0.001%. Five priority natural cases around 32 and 64 KiB move down in both screens. The established 256 KiB witness needs agreement across cohorts.

Measured parent ba8f02f61b3735a2160cc6177e76c6f7cc59cf7b. Published parent fe067e26041272e256a64f728268342cfe34837c. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The experiment uses the compact diagnostic entry. The source, search limits, table contents, capacities, allocator and complete-call boundary are pinned.

## Proof and machine code

Preparation follows full view descriptors through copies, projections, packs and tail transport. Every origin must reach immutable validated literal data. A runtime input, incomplete pack, or unsupported definition loses the payload fact. CLIF uses the actual scalar loads and joins every incoming range, including the original zero fallback. Nonwrapping range endpoints establish the unsigned bounds without changing source arithmetic. The 0–15 payload and 0–7 shift prove a 0–29 index for every U64 offset.

The checked no-growth call admits the length of its current cost view at phase entry. The immutable input packet and its disjoint output packet supply the proof, including the packet reload on the alternate parser entry. Read-only lengths on every incoming edge resolve to the same packet field. A short view selects the original general body before a source action. The first slot lookup keeps its bounds and default path.

One comparison disappears in each of four parser bodies. In the inspected linked body, the successful route from the combined index to the cost load drops three instructions, including the cost length reload and branch. The parser frame shrinks from 256 to 240 bytes, static stack reads from 22 to 18, and code from 1552 to 1520 bytes. The caller grows from 979,776 to 999,064 code bytes. 342 of 347 bodies are byte-identical. Disabling the pass reproduces R327 Object and metadata exactly.

## Matched results

| Input | R327 / Roc | R329 / Roc | R327 / libdeflate | R329 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.154610× | 2.176221× | 0.016917× | 0.017087× | +0.000% |
| one-stored | 1.976966× | 2.015052× | 0.017899× | 0.018244× | +0.000% |
| seven-stored-prefix | 2.253980× | 2.624599× | 0.018594× | 0.021651× | +0.000% |
| eight-literals | 0.181730× | 0.182565× | 2.317054× | 2.327696× | +0.714% |
| reymont-prefix-32769 | 1.290149× | 1.284935× | 1.173981× | 1.169236× | -0.140% |
| heldout-window-32768 | 1.281922× | 1.278755× | 1.169377× | 1.166488× | -0.167% |
| prefix-natural-32767 | 1.282875× | 1.279630× | 1.170289× | 1.167328× | -0.082% |
| prefix-natural-32768 | 1.286730× | 1.285287× | 1.172222× | 1.170907× | -0.385% |
| prefix-natural-65537 | 1.298738× | 1.296533× | 1.093177× | 1.091322× | -0.835% |
| prefix-natural-131072 | 1.295508× | 1.297032× | 1.070910× | 1.072170× | -0.028% |
| prefix-natural-262144 | 1.310640× | 1.312445× | 1.058134× | 1.059591× | -0.195% |
| prefix-natural-1048576 | 1.309703× | 1.313758× | 1.057835× | 1.061111× | +0.171% |
| prefix-random-16384 | 1.064922× | 1.057748× | 2.879526× | 2.860129× | +0.720% |
| prefix-periodic-1048576 | 0.707875× | 0.709567× | 1.202293× | 1.205168× | +0.246% |
| prefix-repeated-1048576 | 0.711547× | 0.713386× | 1.113411× | 1.116290× | +0.002% |

The four-actor comparison contains 154,656 balanced complete calls. Level 12, runtime input, original capacities, fresh C state, and validation placement match.

## Counters and validation

The separate 256 KiB CPU cohort measures 745,126,587.0 to 727,771,539.5 retired instructions (-2.329%). Cycles change +0.015%. All six tracked heap counters agree on all 15 screen cases. The candidate profile assigns 3,104 of 5,352 running samples to the parser. Profiles include validation and the CPU counters come from separate invocations.

The warnings-as-errors build passes 61 CFG tests and 504 focused JIT pairs. 120 focused linked calls on original and decoded artifacts cover changed literal data, short current cost views, current cost-content changes, rebound slot payloads, original fallbacks, and unchanged held outputs. The original and decoded compressor objects and metadata match. 324 linked compressor calls verify exact streams, independent inflate, capacities, held aliases, and zero live owners.

The experiment stops at the 15-case performance screen. The next pilot composes the slot and cost tables within each current cost snapshot to remove the dependent load.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r329-immutable-payload-range-07ff92b59276.tar.gz). SHA-256 07ff92b592768a2f8e1ee812e19c0c1d034a8505b08e346a1a81e499b76a5ce3.
