# R330 — Compose offset costs for the current parser pass

**Keep R327 with the R272 runtime.** In the four-actor comparison 256 KiB natural moves from 1.306734× to 1.525092× Roc and 1.057991× to 1.234784× libdeflate. Paired elapsed changes +16.710% there and +17.102% in the separate primary screen. The remaining natural witnesses also regress.

Measured compiler ba8f02f61b3735a2160cc6177e76c6f7cc59cf7b. PR base 92f75db84e2523693648084bea16cd52ad60777a. Published parent 4111148fca1263e9a24f6113b43a5206a407978f. Apple ARM64, IR/work/ABI/bytecode 215/107/39/14. This diagnostic source representation pilot uses the prior exact compiler and runtime.

## Composition

For each selected pass, construct 512 U32 entries from its current cost values and the 256 U8 slot entries. The high half adds 14 to the class before fetching the current cost. Zero and out-of-table offsets retain the original cost defaults. The symbolic check covers all 65,536 values of the packed U16 offset. The pass rebuilds its own table, which keeps saved, static, and dynamic costs current. A 2,048-record work estimate and exact slot length gate select the experiment, with the original parser for sparse calls or other views. The full-call screen includes construction, growth, initialization, use and cleanup. A managed finite-resource admission fallback would be needed for a production version.

## Machine code and counters

The linked 256 KiB parser executes one prepared cost load. Its setup repeats the compare against offset 256 for shift and bank selection. It still checks the 512-entry lexical floor and the combined index before the load. Those checks are visible in the actual linked assembly. The report keeps this source lowering and a possible native preparation pass distinct.

The CPU cohort changes instructions +1.523% and cycles +16.145%. At 256 KiB there are 88 extra payload allocations, 44,880 more requested bytes, and 22,352 more initialized bytes copied. These deltas match 11 constructions growing through eight capacities from 4 to 512 words. Peak payload stays 9,763,141 bytes. Both random cases retain all six baseline heap families.

The composed parser has 3,258 of 5,104 running samples, with samples on both sides of the repeated offset threshold branch. The retained 256 KiB parser previously had 3,103 of 5,349. Profiles include validation and come from separate diagnostic invocations.

## Matched screen

| Input | R327 / Roc | R330 / Roc | R327 / libdeflate | R330 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.107770× | 2.167709× | 0.016592× | 0.017063× | +0.000% |
| one-stored | 1.983853× | 2.036338× | 0.017680× | 0.018147× | +0.000% |
| seven-stored-prefix | 2.230838× | 2.234802× | 0.018592× | 0.018625× | +0.000% |
| eight-literals | 0.184065× | 0.183627× | 2.358960× | 2.353349× | +0.821% |
| reymont-prefix-32769 | 1.289412× | 1.495704× | 1.168584× | 1.355544× | +15.744% |
| heldout-window-32768 | 1.283437× | 1.484779× | 1.172281× | 1.356184× | +15.261% |
| prefix-natural-32767 | 1.285374× | 1.489251× | 1.173862× | 1.360052× | +15.563% |
| prefix-natural-32768 | 1.283964× | 1.475064× | 1.180037× | 1.355669× | +15.052% |
| prefix-natural-65537 | 1.294457× | 1.507882× | 1.092166× | 1.272238× | +17.118% |
| prefix-natural-131072 | 1.299161× | 1.517067× | 1.071462× | 1.251176× | +16.491% |
| prefix-natural-262144 | 1.306734× | 1.525092× | 1.057991× | 1.234784× | +17.102% |
| prefix-natural-1048576 | 1.310876× | 1.519826× | 1.058514× | 1.227238× | +15.601% |
| prefix-random-16384 | 1.102093× | 1.108445× | 2.924554× | 2.941412× | -0.973% |
| prefix-random-1048576 | 1.596530× | 1.597246× | 2.264021× | 2.265036× | +0.075% |
| prefix-periodic-1048576 | 0.706713× | 0.712568× | 1.203500× | 1.213471× | -0.044% |
| prefix-repeated-1048576 | 0.709006× | 0.707982× | 1.120932× | 1.119314× | -0.118% |

154,752 matched timed calls and 162 linked qualification calls. Both timing gates pass 8/16 pooled and 7/16 by block mean. All 81 original streams match and independently inflate, with exact capacities and preserved aliases. The candidate stops at this screen.

## Retained random profiles

| Input | Running samples | Matcher | Tally | Parser |
|---|---:|---:|---:|---:|
| prefix-random-16384 | 5,263 | 2,302 | 1,203 | 495 |
| prefix-random-1048576 | 5,358 | 3,296 | 1,054 | 441 |

These samples come from the exact R327 image. The matcher and tally are the two largest measured regions for random data at both sizes.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r330-composed-cost-e5b0a68594f9.tar.gz). SHA-256 e5b0a68594f905da7266df8750aee53cb49a0ef97a72501c62ec90e31f2c8d83.
