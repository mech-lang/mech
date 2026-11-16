# R260: empty-path census and current-baseline fixed-placement control

**Retain R256.** The existing phase threader removes real repeated work, but the complete-call experiment loses on random, periodic and repeated inputs even when every retained function address is preserved. This is an archived control, not a newly adopted compiler baseline.

Published parent: `9dbdf42d2157715941e3b6cd75421eb01442d1d7`. PR base: `92f75db84e2523693648084bea16cd52ad60777a`. Retained compiler source: R256 at `608ed69aedb98bb8c9d61b0bf6364580b304a6d1`; ordinary IR/work/ABI/bytecode remain 215/107/39/14. The complete 830-file candidate source closure reverses exactly to the 825-file R256 closure. The R255 backend and all dependencies remain pinned.

This round changes no compressor source, algorithm, tables, depth300/nice258, original EOF checks, buffer kinds/capacities, allocation sites, allocator or timing boundaries. It retains the real 262144-entry U16 tree, 1500513-word U32 cache and 305257-entry U64 path capacity. Full level12 compression, scratch construction, transfer and cleanup remain inside each evaluate boundary. Runtime input, equal warmups and borrowed output ownership are matched. The historical comparator remains in the earlier comparison reports.

The compact experimental entry still omits the additional ordinary exact managed resource/admission/accounting/publication/recovery contract. The alias/error/cleanup controls below do not restore that contract.

## Fresh eligibility and machine witness

A diagnostic-only callback borrows the fully initialized current phase input packet. The probe reads the existing immutable cache in place, derives the unchanged source loop visits, and discards all pointers before returning. These are packet-derived source eligibility counts, not per-iteration hardware events. All81 cases run twice with identical counts, exact streams, independent inflate, unchanged aliases and zero private residual. A final-source decoded diagnostic Object and metadata match the original exactly. Disabled diagnostics restore the full R256 Object and metadata.

|1MiB input|Path calls|Positions|Empty outer visits|Match reductions|Candidate lengths|Lengths/reduction|
|---|---:|---:|---:|---:|---:|---:|
|natural|25|5,086,456|165,668|14,442,759|39,253,430|2.717862|
|random|11|2,997,152|2,991,660|5,517|5,517|1.000000|
|periodic|8|2,097,152|2,089,022|16,260|2,080,810|127.971095|
|repeated|8|2,097,152|2,089,022|16,260|2,080,888|127.975892|

Random has 2,991,660/2,997,152 =99.816759% empty visits. Natural text retains 14,442,759 short reductions averaging2.717862 candidate lengths. Nonincreasing lengths are zero in all81 current complete traces; the focused semantic controls still include decreasing and empty ranges.

The audited native zero-count route is39 instructions in R256 and21 after threading, from common initialization to the first append-capacity test. Header read, literal-cost computation, source arithmetic, already-decremented header cursor and append remain. The18-instruction route saving predicts53,849,880 fewer random1MiB instructions from empty visits alone. This is a static route exposure estimate, not a complete hardware instruction partition. The actual whole-call decrease is54,904,992 instructions.

## Placement isolation

The exact R247 threader runs on four selected path definitions and their unused vector alternatives. This is an experiment header filter, not input-dependent dispatch or a compression-specific compiler proof. Each original2048-byte body becomes1968 bytes; the same six local repair phis and three threaded edges per body are retained. No global repair argument campaign is added. Frames remain288 bytes.

The padded control appends80 unreachable AArch64 NOP bytes per changed definition. All343 Object function starts and all178 retained linked entry starts remain equal to R256. Root code and symbolic relocations are exact;335 other definitions are unchanged. Padded and unpadded reachable code and relocations match exactly. Thus movement of unrelated functions is removed as a sole explanation. Internal branch placement and register allocation still change.

The assembly also exposes a new cache-cursor stack store/reload. It is a concrete live-state/dependency candidate, not proof that it alone explains every lost cycle. There is no alignment sweep.

## Complete-call matched results

Two independent cohorts use the four persistent actors in all24 balanced orders, equal200ms warmups, fixed original batch sizes and all81 unchanged cases. Candidate/control and candidate/reference ratios are formed **inside each balanced block**, then geometrically summarized over48 blocks. Raw pooled medians and original1.05 gates are retained separately, including contrary tiny-case results. No observations from older cohorts are pooled here.

Across81, padded elapsed is+1.537919%; unpadded is+0.884097%. Both are archived.

|Case|R256/Roc|Fixed placement/Roc|Natural placement/Roc|Fixed/R256 elapsed|Instructions|Cycles|
|---|---:|---:|---:|---:|---:|---:|
|eight-literals|0.418882x|0.436531x|0.427977x|+4.2133%|-0.0495%|-4.2711%|
|prefix-natural-32768|1.841523x|1.751952x|1.752627x|-4.8640%|-6.7499%|-5.1034%|
|prefix-random-32768|1.845227x|1.909470x|1.903934x|+3.4816%|-5.5418%|+3.8016%|
|prefix-periodic-32768|1.099299x|1.133368x|1.133621x|+3.0992%|-4.1351%|+4.0051%|
|prefix-repeated-32768|1.056704x|1.096444x|1.094859x|+3.7607%|-4.1428%|+3.6928%|
|prefix-natural-1048576|1.900042x|1.821964x|1.821439x|-4.1093%|-6.5085%|-4.3877%|
|prefix-random-1048576|2.170635x|2.249037x|2.249241x|+3.6119%|-5.5568%|+3.2761%|
|prefix-periodic-1048576|1.270857x|1.317566x|1.322129x|+3.6754%|-4.3292%|+3.2613%|
|prefix-repeated-1048576|1.225534x|1.275855x|1.275380x|+4.1061%|-4.3255%|+3.2820%|

At1MiB natural text, instructions3847710904.5→3597284339 (−6.5085%) and cycles979769045.5→936779509 (−4.3877%). Random instructions988062167.5→933157175.5 (−5.5568%), while cycles199953481.5→206504068 (+3.2761%). Periodic/repeated similarly retire fewer instructions but need more cycles. This tradeoff survives the placement control.

The eight-byte elapsed result worsens while its independent cycle cohort improves; both contrary measurements remain. Neither independent cohorts nor CPU-time sample shares are multiplied into elapsed or retired-instruction claims.

|Gate|Fixed placement|Natural placement|
|---|---:|---:|
|Combined original point|78/81|80/81|
|Combined block mean|75/81|79/81|
|Cohort A point/block|77/71|see all81 data|
|Cohort B point/block|77/74|see all81 data|

Combined fixed-placement point misses are eight-literals, prefix-periodic-1024 and prefix-repeated-262144. Block misses also include empty, one-stored and eight-overlap. Both variants remain below the eight-byte1x reference ceiling. No gate or allowance is raised.

## Allocation, copying and memory

All81 independent heap cohorts match R256 exactly for all three Mech actors: allocation/reallocation/deallocation requests, requested bytes, incremental live-usable peak, private payload requests/bytes, initialized copying, zeroing and private peak. Final nonreturned private bytes remain zero. The16848 CPU/heap observations contain15552 retained calls. RSS and physical footprint below are whole-worker observations and include metadata/harness memory; they are not isolated compression heap peaks. Requested capacity is not initialized bytes copied.

|Case|Mech requests|Roc requests|Mech requested bytes|Roc requested bytes|Mech private peak|Mech initialized copy bytes|
|---|---:|---:|---:|---:|---:|---:|
|eight-literals|230|96|9,030,708|9,021,944|8,982,372|2,188|
|prefix-natural-1048576|1,556|695|26,677,303|10,390,299|10,550,358|40,676|
|prefix-random-1048576|914|373|26,493,368|10,213,500|10,550,486|15,316|
|prefix-periodic-1048576|742|291|26,451,818|10,174,286|10,550,486|8,752|
|prefix-repeated-1048576|742|291|26,451,710|10,174,178|10,550,486|8,752|

## Current profiles

Six fresh exact-image complete-program CPU-time captures cover eight bytes, natural/random32KiB and natural/random/periodic1MiB. They include validation/harness activity outside the primary timer; sample shares are not instruction or cycle partitions. No old root source map is used, and root samples remain explicitly unattributed. Reference whole-call counters are fresh in the matched cohorts; no new reference CPU-time capture is claimed in this round.

|Case|Running samples|Numeric-phase samples|Root samples|Other samples|
|---|---:|---:|---:|---:|
|eight-literals|5362|793|862|3707|
|prefix-natural-1048576|5382|5255|67|60|
|prefix-natural-32768|5538|5254|154|130|
|prefix-periodic-1048576|5377|5172|72|133|
|prefix-random-1048576|5614|5518|16|80|
|prefix-random-32768|5521|5188|112|221|

## Qualification and limitations

- 722 maintained tests pass under -Dwarnings (715 current-baseline tests plus the seven existing threader tests). The same seven inherited exclusions remain verbatim in maintained-test-command.json; none is newly waived. Nightly2026-03-03/AArch64, compileropt3/test-rootopt1, six features: aot,jit,canonical-native,canonical-native-runtime,canonical-native-program,canonical-native-program-runtime.
- 240 focused original/decoded real-path native calls agree with the unchanged independent oracle and frozen resident receipts. They cover current arguments, empty/decreasing ranges, bounds/shape controls, error precedence, aliases, unchanged publication, cleanup and retry. No fresh resident run is claimed; each form retains the prior34 ordinary default-work denials, four errors and two values.
- 162 complete snapshot calls plus162 uninstrumented smokes pass all81 exact streams, independent inflation and alias checks; both elapsed and counter cohorts independently repeat those checks.
- Diagnostics off restore R256 Object/metadata. Original/decoded diagnostic images match; source annotations and final formatting rebuild reproduce the entire measured candidate Object/metadata. Rustfmt2024 checks pass on the three changed modules without recursive child formatting.
- All case sizes through1MiB are included; full6.6MB, additional ordinary managed-contract restoration and a new LLVM comparison are outside this round.
- Initial compiler/harness mistakes and the normal-environment ps permission denial are fixed and listed in failures-and-exclusions.json. There is no unresolved introduced semantic failure. Performance gate misses are introduced by this experiment and prevent adoption.
- CI is cancelled only for this exact published head under the standing instruction. The PR stays draft and a new exact-head review is requested; no new PR or branch is created.

## Next bounded step

The bounded R259 geometry/window comparison retains its stated proof-use qualification; dynamic packed-site search-versus-skip attribution remains unproved. This round closes the fresh empty-path witness and the current R247 placement control. Do not repeat those completed controls as new work packages. Keep R256 retained and test one internal scalar ordered-reduction state representation: carry its U32 minimum and U32 winning index in one U64 register, leaving selected-item evaluation at the existing finish point. Admission already bounds indices below the reserved U32-max sentinel. Compare the actual moves/spills, unchanged source charges/errors and all81 complete-call outcomes before any combination. This directly targets live-state pressure exposed here; a speedup is not assumed.

## All81 paired elapsed results

|Case|R256/Roc|Fixed/Roc|Natural/Roc|Fixed/R256|Natural/R256|Point gate|Block gate|
|---|---:|---:|---:|---:|---:|---|---|
|empty|2.112093x|2.256623x|2.108357x|1.068430x|0.998231x|pass|FAIL|
|one-stored|2.021584x|2.152654x|2.033245x|1.064835x|1.005768x|pass|FAIL|
|seven-stored-prefix|2.222397x|2.229416x|2.347255x|1.003159x|1.056182x|pass|pass|
|eight-literals|0.418882x|0.436531x|0.427977x|1.042133x|1.021713x|FAIL|FAIL|
|eight-overlap|0.417993x|0.435161x|0.419158x|1.041073x|1.002788x|pass|FAIL|
|320-overlap|0.462171x|0.478339x|0.460288x|1.034982x|0.995927x|pass|pass|
|eight-literals-prefix|0.414811x|0.427962x|0.418608x|1.031702x|1.009153x|pass|pass|
|128-biased-dynamic|0.415394x|0.428319x|0.413217x|1.031114x|0.994759x|pass|pass|
|32-high-entropy-stored|0.424874x|0.436600x|0.423909x|1.027599x|0.997730x|pass|pass|
|16384-distribution-change|1.888921x|1.878060x|1.884687x|0.994250x|0.997758x|pass|pass|
|33027-periodic-window|1.104432x|1.143890x|1.144361x|1.035728x|1.036153x|pass|pass|
|random-64|0.435886x|0.454014x|0.437253x|1.041588x|1.003137x|pass|pass|
|random-256|0.478428x|0.493367x|0.481350x|1.031226x|1.006107x|pass|pass|
|random-1024|0.539261x|0.558063x|0.544637x|1.034866x|1.009968x|pass|pass|
|repeated-1024|0.521331x|0.547819x|0.530583x|1.050809x|1.017747x|pass|pass|
|random-32769|1.827435x|1.894688x|1.887544x|1.036802x|1.032892x|pass|pass|
|repeated-32769|1.064573x|1.101164x|1.102172x|1.034372x|1.035318x|pass|pass|
|reymont-prefix-1024|0.683012x|0.680165x|0.663976x|0.995831x|0.972128x|pass|pass|
|reymont-prefix-16384|1.766272x|1.667876x|1.666053x|0.944292x|0.943260x|pass|pass|
|reymont-prefix-32769|1.834064x|1.746896x|1.745414x|0.952473x|0.951665x|pass|pass|
|heldout-window-32767|1.837162x|1.745637x|1.746574x|0.950182x|0.950691x|pass|pass|
|heldout-window-32768|1.837128x|1.745425x|1.747548x|0.950083x|0.951239x|pass|pass|
|heldout-slides-65537|1.903540x|1.822659x|1.821892x|0.957510x|0.957107x|pass|pass|
|heldout-random-65537|1.986871x|2.060313x|2.063407x|1.036964x|1.038521x|pass|pass|
|heldout-mixed-131073|1.921678x|1.929248x|1.930910x|1.003939x|1.004804x|pass|pass|
|prefix-natural-1024|0.678730x|0.672284x|0.668308x|0.990503x|0.984645x|pass|pass|
|prefix-natural-4096|1.337661x|1.270140x|1.262875x|0.949523x|0.944092x|pass|pass|
|prefix-natural-8192|1.589808x|1.500679x|1.500253x|0.943937x|0.943670x|pass|pass|
|prefix-natural-16384|1.770501x|1.672487x|1.675396x|0.944641x|0.946284x|pass|pass|
|prefix-natural-32767|1.834596x|1.744768x|1.746038x|0.951037x|0.951729x|pass|pass|
|prefix-natural-32768|1.841523x|1.751952x|1.752627x|0.951360x|0.951727x|pass|pass|
|prefix-natural-32769|1.836326x|1.745379x|1.745336x|0.950473x|0.950450x|pass|pass|
|prefix-natural-33027|1.840122x|1.750936x|1.748492x|0.951532x|0.950204x|pass|pass|
|prefix-natural-65535|1.910188x|1.817093x|1.816424x|0.951264x|0.950914x|pass|pass|
|prefix-natural-65536|1.911030x|1.816325x|1.815636x|0.950443x|0.950083x|pass|pass|
|prefix-natural-65537|1.909187x|1.811514x|1.813929x|0.948841x|0.950106x|pass|pass|
|prefix-natural-131072|1.898063x|1.809662x|1.810065x|0.953426x|0.953638x|pass|pass|
|prefix-natural-262144|1.902083x|1.818130x|1.816509x|0.955863x|0.955011x|pass|pass|
|prefix-natural-1048576|1.900042x|1.821964x|1.821439x|0.958907x|0.958631x|pass|pass|
|prefix-random-1024|0.545668x|0.560763x|0.546328x|1.027664x|1.001210x|pass|pass|
|prefix-random-4096|1.004105x|1.017552x|1.008728x|1.013392x|1.004604x|pass|pass|
|prefix-random-8192|1.224774x|1.276609x|1.267313x|1.042322x|1.034732x|pass|pass|
|prefix-random-16384|1.575496x|1.621827x|1.625694x|1.029407x|1.031861x|pass|pass|
|prefix-random-32767|1.836841x|1.902992x|1.898215x|1.036013x|1.033413x|pass|pass|
|prefix-random-32768|1.845227x|1.909470x|1.903934x|1.034816x|1.031816x|pass|pass|
|prefix-random-32769|1.829882x|1.892485x|1.889773x|1.034212x|1.032730x|pass|pass|
|prefix-random-33027|1.832109x|1.899062x|1.890030x|1.036544x|1.031615x|pass|pass|
|prefix-random-65535|2.009643x|2.086617x|2.086247x|1.038302x|1.038118x|pass|pass|
|prefix-random-65536|2.011400x|2.083845x|2.081438x|1.036017x|1.034821x|pass|pass|
|prefix-random-65537|2.000169x|2.071942x|2.073712x|1.035884x|1.036768x|pass|pass|
|prefix-random-131072|2.081787x|2.155897x|2.155942x|1.035599x|1.035621x|pass|pass|
|prefix-random-262144|2.187930x|2.250612x|2.268020x|1.028649x|1.036605x|pass|pass|
|prefix-random-1048576|2.170635x|2.249037x|2.249241x|1.036119x|1.036213x|pass|pass|
|prefix-periodic-1024|0.531431x|0.558832x|0.544680x|1.051560x|1.024931x|FAIL|FAIL|
|prefix-periodic-4096|0.771876x|0.790145x|0.781562x|1.023669x|1.012548x|pass|pass|
|prefix-periodic-8192|0.884143x|0.920178x|0.920316x|1.040756x|1.040913x|pass|pass|
|prefix-periodic-16384|0.991904x|1.024545x|1.020316x|1.032908x|1.028645x|pass|pass|
|prefix-periodic-32767|1.095070x|1.133959x|1.136298x|1.035513x|1.037648x|pass|pass|
|prefix-periodic-32768|1.099299x|1.133368x|1.133621x|1.030992x|1.031222x|pass|pass|
|prefix-periodic-32769|1.097096x|1.139713x|1.133630x|1.038845x|1.033300x|pass|pass|
|prefix-periodic-33027|1.108291x|1.144089x|1.146303x|1.032300x|1.034298x|pass|pass|
|prefix-periodic-65535|1.174817x|1.225080x|1.219138x|1.042783x|1.037726x|pass|pass|
|prefix-periodic-65536|1.179364x|1.223973x|1.222761x|1.037824x|1.036796x|pass|pass|
|prefix-periodic-65537|1.178354x|1.224723x|1.226040x|1.039350x|1.040468x|pass|pass|
|prefix-periodic-131072|1.219499x|1.268691x|1.265635x|1.040338x|1.037832x|pass|pass|
|prefix-periodic-262144|1.238961x|1.310671x|1.305345x|1.057879x|1.053581x|pass|pass|
|prefix-periodic-1048576|1.270857x|1.317566x|1.322129x|1.036754x|1.040345x|pass|pass|
|prefix-repeated-1024|0.528330x|0.541164x|0.536713x|1.024293x|1.015867x|pass|pass|
|prefix-repeated-4096|0.760369x|0.790183x|0.792366x|1.039210x|1.042081x|pass|pass|
|prefix-repeated-8192|0.861461x|0.903201x|0.898464x|1.048453x|1.042954x|pass|pass|
|prefix-repeated-16384|0.962267x|1.002057x|0.993715x|1.041351x|1.032682x|pass|pass|
|prefix-repeated-32767|1.058574x|1.098505x|1.096149x|1.037722x|1.035496x|pass|pass|
|prefix-repeated-32768|1.056704x|1.096444x|1.094859x|1.037607x|1.036107x|pass|pass|
|prefix-repeated-32769|1.068428x|1.103497x|1.101322x|1.032824x|1.030788x|pass|pass|
|prefix-repeated-33027|1.079426x|1.121388x|1.115846x|1.038874x|1.033741x|pass|pass|
|prefix-repeated-65535|1.135934x|1.178617x|1.179334x|1.037576x|1.038206x|pass|pass|
|prefix-repeated-65536|1.138227x|1.181791x|1.182299x|1.038273x|1.038719x|pass|pass|
|prefix-repeated-65537|1.142447x|1.185089x|1.183357x|1.037325x|1.035808x|pass|pass|
|prefix-repeated-131072|1.184263x|1.227348x|1.227805x|1.036381x|1.036767x|pass|pass|
|prefix-repeated-262144|1.222494x|1.291790x|1.301894x|1.056684x|1.064949x|FAIL|FAIL|
|prefix-repeated-1048576|1.225534x|1.275855x|1.275380x|1.041061x|1.040674x|pass|pass|

## Public review packet

[Complete source and measurement evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r260-path-placement-8c7ab1e856aa.tar.gz) (`sha256:8c7ab1e856aa1558517abc71228a212075767880d549552d22d513af06de5f1b`, 10604541 bytes). Fresh public retrieval verifies all1885 file hashes, six byte-identical offline report replays, and exact830-file source reconstruction/reversal to825-file R256. No native binaries, bytecode, full compressor descriptors/IR/maps, raw traces or private environment exports are uploaded.

Compiler source manifest SHA-256: `706a2053854556b63101b34f5b1197820a7bcfa0d3362449d068013a2bd5570f`. Exact build, test, emission, source, backend, allocator, Object and worker pins are in the packet.

## Subsequent review priority

The review of R259 identifies two source-backed inherited compact-compiler defects (packed-read/reuse dominance and metadata-only owner acquisition), plus tracked managed-admission and replay-tool defects. Native reproduction and fixes take priority over the next optimization. R256 is retained as a measurement control, not a general-compiler adoption claim; this packet predates those repairs.
