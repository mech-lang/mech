# Native compiler integration qualification

Source qualification ran at `d4d35e078a1a992d9020357d82ca5e3a2557f13c`. The same source is stacked on the published report commit `2c2e98b4ba3c8a733b254f7ee6bc9f64ed323171` as `f303cd63137eb32431ab03bf7f8a63d86fbda118`. The tree comparison differs only in the earlier timing report. The compact and managed measurements use `a101b7ff4f1274bc46f067112b8e8378654d0f64`. The [evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-d4d35e07-integration-and-addressing-pilot.tar.gz) carries the exact commands, source patches, failures, timing observations, and independent replay. Its SHA-256 is `4d77b460d72a38479ace69cc78e1897cc85eee3d04506135a985bb237bb13ad3`.

| Review finding | Commit on the published stack | Validation |
| --- | --- | --- |
| Canonical comparisons through widening | `730da720ee` | Boolean cleanup and widening run on the same maintained compiler fixture. |
| Shared unsigned reads in checked append | `af75255a1c` | The actual shared loader exercises width, flags, offsets, and refusal cases. The compiler suite covering both findings passed 87 tests. |
| Same-thread worker fuel | `8aaa01c9b8` | 44 worker and two prepared-schema tests passed. The inherited sequence feature errors are addressed by `10d6140a75`. |
| Backend overrides for standalone builds | `283c5184df` | Three generated applications resolved the vendored dependencies in the maintained Cargo tests. |
| Hosted compiled-module bindings | `83b7ccb806` and `0515f5a133` | The fixture import regression is repaired in the integration harness. The policy kernel passed all 13 selected tests at a101. |

The five hosted `StaleHandle` failures reproduced on the clean earlier parent. Prepared text line length sites now reach the private-view handler in `99e7a16af1`. The merge also introduced four runtime-only dead-code errors and two missing fixture exports. The compile gates are in `f7368a202b` and the fixture exports are in `f303cd6313`. The packet distinguishes the clean-parent reproduction from the source-history diagnosis of the two build failures.

| Profile at the qualified source | Result |
| --- | --- |
| Ordinary hosted contracts | 47 passed. The explicit linked test is listed separately. |
| Policy hosted contracts | 48 passed. The explicit linked test is listed separately. |
| Standalone ordinary linked | 1 passed with original and decoded artifacts. |
| Standalone release policy linked | 1 passed with original and decoded artifacts and the production LTO control. |
| Generated launcher replay | 8 passed under warnings-as-errors. |
| Minimal runtime-only library | Cargo check passed under warnings-as-errors. |

The separate optional full-input control and hosted CI completion are outside these local receipts. The archive includes the previously failing diagnostics and the terminal passing results.

The compact timing rerun used all 81 pinned level-12 cases with runtime input, 24 balanced four-actor orders per case, and 245,664 measured calls. Original and decoded workers each passed 162 output, inflate, capacity, alias, and cleanup checks. Fresh a101 emissions independently matched the retained Object and metadata in both forms. The timed compact calls cover compression, scratch construction, output transfer, and cleanup. The hosted tests cover ordinary resource admission, publication, and recovery.

| Result | Retained R355 | Repaired baseline |
| --- | ---: | ---: |
| Faster than Roc | 46/81 | 47/81 (58.0%) |
| Faster than libdeflate | 7/81 | 10/81 (12.3%) |
| Faster than both | 0/81 | 1/81 |

These are counts from the same fresh comparison. The repaired baseline's paired geometric mean was **0.957836× R355** and all 81 paired estimates were below 1× R355. The raw replay retains all 6,792 zero timer samples. Every block mean is positive.

| Input | Mech median | Mech/Roc | Mech/libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.049709 ms | 0.173554× | 2.215658× |
| Natural 8 KiB | 1.175667 ms | 1.042720× | 1.304794× |
| Natural 1 MiB | 192.514875 ms | 1.203077× | 0.964789× |
| Repeated 256 KiB | 5.393271 ms | 0.617698× | 1.003770× |
| Repeated 1 MiB | 21.062271 ms | 0.614834× | 0.965169× |

The ratios are geometric means of paired blocks. The independent counter cohorts cover ten cases, and the exact timed executable has six profiles. Eight literals retired 626,894 instructions versus 1,361,942 for Roc, with lifetime physical peaks of 44.8 MB and 8.46 MB respectively. Natural 1 MiB retired 1.281× Roc instructions. Its sampled PCs split 53.0% in the composed parser and 40.7% in the matcher, with validation present in the profile cohort. Private allocation, initialized-copy and zeroing metrics matched R355 in all ten counter cases.

The managed related-post guard completed 31 primary and 13 confirmation pairs with 88 exact outputs. The pinned calculation timers were **0.714771×** and **0.687365× Rust**. Whole-program wall-clock ratios were 3.099× and 3.049× Rust. All 13 files in the freshly generated managed bundle matched the previous bundle byte for byte.
