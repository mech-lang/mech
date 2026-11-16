# Reuse header setup and narrow the previous-node read

The retained composed parser recomputes the current header ordinal at the bounds test, indexed load, first-record calculation and record-loop test. It reads a complete U64 node to use its low U32 cost, then constructs a tagged literal through two U32 masks. The pinned reference also holds a current record ordinal across child visits and uses U32 loads for prior costs. The earlier header-ordinal trial added a branch after every ordinary store. This trial keeps the existing store and continuation addresses.

The fixed-layout candidate uses the current cursor register for the ordinal through record completion. It tests the prior node count for zero and reads only the low word at the same initialized cell, using the original zero default on an empty view. The literal field comes from the high 16 bits of a U32 load and needs at most 25 bits after the shift and tag. Three zero defaults branch away from the readable route. The cache and literal bounds checks still select their original reads in order. The readable setup through the first record dispatch takes 21 instructions instead of 30. Each completed record saves one more instruction. The scalar reduction and append store retain their existing instructions and addresses.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.164251× → 0.162872× | 0.160813× → 0.160828× | 2.061556× |
| Natural 16 KiB | 1.036098× → 1.014497× | 1.034319× → 1.014611× | 1.026006× |
| Reymont 16 KiB | 1.040033× → 1.015182× | 1.033511× → 1.014715× | 1.022624× |
| Reymont 32,769 bytes | 1.061347× → 1.051752× | 1.064028× → 1.054233× | 0.957110× |
| Natural 256 KiB | 1.081523× → 1.066203× | 1.082344× → 1.066943× | 0.861790× |

The 18-case screen and eight-case confirmation take **0.991476×** and **0.990011×** paired control. Both timing gates pass throughout, with unchanged peer win sets. The two identical 16 KiB inputs improve by 1.8–2.4% across both cohorts. Random 1 MiB takes 1.002303× control in the screen and 0.997835× in confirmation. In the independent counter cohort, natural 16 KiB retires 28,425,849 → 27,509,258 instructions in 7,176,946 → 7,048,468 cycles. Natural 256 KiB retires 544,378,871 → 526,142,436 instructions in 156,216,519 → 154,281,102 cycles. Eight-literal cycles rise from 171,049 to 188,949 in that cohort while retired instructions are essentially unchanged. Its matched elapsed changes in opposite directions across the primary screen and confirmation.

The fixed-layout Object `45df0dec03e3e9480a9f7bc19ae1cd1e22f9806e8ec8bfb854b139f9d586e229` uses the pinned compiler `170401d69aaeded4304b2992763dd5dfd7e9aabf`. Worker `70bf83128f86de220f1be63b897dafc7a35943e27fc52bf7bafaba7feed0dca3` preserves all 1,276 linked addresses and the 1,216-byte body with a 224-byte frame. It passes 162 complete linked checks, 68,096 direct positions, 288 boundary pairs and 14,592 timed calls. The direct controls verify failure origin 15525, prior publication and cache preservation, and successful retry. The three exact-worker profiles place 2,218/4,626, 2,542/5,220 and 3,007/5,370 running timer PCs in the main parser at natural 16 KiB, Reymont 32,769 bytes and natural 256 KiB respectively. Sample counts include validation. The assembly preflight corrected an unsupported scaled narrow-load operand and checked the final fallthrough and branch encodings. The full matrix was skipped for the fixed-layout diagnostic. Archive SHA-256 `98726f769725ca1296dbf60db905e1989defea84dacda51a3d506f0b4c698876`. [Fixed-layout instructions, matched observations, counters and direct witness](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-composed-header-setup.tar.gz).

## Compiler emission from the transformed SSA

The SSA trial puts the biased cursor on every incoming edge of the outer loop, folds a branch whose two effect-free routes reach the same continuation, narrows the previous-node load and the masked tag arithmetic, and marks the original zero defaults cold. The first shape emits a separate compare and branch at the empty previous-node test and four instructions to form the scaled narrow load. A second shape uses a direct zero branch and forms the adjusted base before adding the scaled index. The second output still materializes a separate address adjustment and a cost register transfer. Both 1,200-byte linked bodies retain the 224-byte frame and 1,276 symbol addresses.

| Input or cohort | First shape / control | Second shape / control | First shape repeat / Roc | Second shape repeat / Roc |
| --- | ---: | ---: | ---: | ---: |
| Natural 16 KiB, confirmation | 0.984953× | 0.986652× | 1.023057× | 1.018855× |
| Reymont 16 KiB, confirmation | 0.988578× | 0.988889× | 1.023027× | 1.021295× |
| 18-case screen | 0.995165× | 0.997098× | — | — |
| Eight-case confirmation | 0.991846× | 0.995153× | — | — |

Each SSA worker passes 162 complete linked checks and 14,592 timed calls. Both timing gates pass throughout, with unchanged peer win sets. The second shape shortens the setup but gives back part of the first shape's suite gain. Its first comparison slows Reymont 32,769 bytes from 1.061002× to 1.061722× Roc. The per-worker profiles and all elapsed observations are retained. The SSA trials change a single linked body and skip source-form emission, the full matrix, and hardware counters. They leave the production scoreboard on the previously qualified compiler. Archive SHA-256 `cfd19ae44c15cf5a6c85975d63f60f500201add1e55d1ea5965cd2174386a2bb`. [Both SSA transformations, generated code and profile replays](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-composed-header-ssa.tar.gz).
