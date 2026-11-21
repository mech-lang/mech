# Native CPU sampling

Separate five-second1ms sampling. Compiler code boundaries differ, so shares guide optimization and are not exact matched phase durations. All diagnostic outputs are verified independently of primary timing.

The parser subtracts child samples and restricts every share to encoder-root descendants. Shares sum to100% within each encoder; code boundaries differ across compilers. These are not matched absolute phase times.

## Random1MiB

| Actor | Function | Exclusive samples | Encoder share |
| --- | --- | ---: | ---: |
| mech-selected | `mech_compact_numeric_475_vector` | 2825 /3800 | 74.34% |
| mech-selected | `mech_compact_numeric_3009_no_growth_composed` | 528 /3800 | 13.89% |
| mech-selected | `mech_compact_numeric_3447_grouped` | 283 /3800 | 7.45% |
| mech-selected | `mech_compact_word_collection_10` | 74 /3800 | 1.95% |
| mech-selected | `mech_compact_numeric_2821_vector_bounded_byte_histogram` | 39 /3800 | 1.03% |
| mech-selected | `mech_compact_numeric_3956_vector` | 23 /3800 | 0.61% |
| mech-selected | `mech_compact_closed_checked` | 10 /3800 | 0.26% |
| mech-selected | `mech_compact_numeric_362_vector` | 5 /3800 | 0.13% |
| libdeflate | `deflate_compress_near_optimal` | 2586 /3772 | 68.56% |
| libdeflate | `deflate_find_min_cost_path` | 967 /3772 | 25.64% |
| libdeflate | `deflate_optimize_and_flush_block` | 157 /3772 | 4.16% |
| libdeflate | `deflate_make_huffman_code` | 23 /3772 | 0.61% |
| libdeflate | `do_end_block_check` | 16 /3772 | 0.42% |
| libdeflate | `deflate_near_optimal_merge_stats` | 13 /3772 | 0.34% |
| libdeflate | `_platform_memmove` | 5 /3772 | 0.13% |
| libdeflate | `deflate_precompute_huffman_header` | 2 /3772 | 0.05% |

## Random4KiB

| Actor | Function | Exclusive samples | Encoder share |
| --- | --- | ---: | ---: |
| mech-selected | `mech_compact_numeric_475_vector` | 1637 /3769 | 43.43% |
| mech-selected | `mech_compact_numeric_3447_grouped` | 507 /3769 | 13.45% |
| mech-selected | `mech_compact_numeric_3956_vector` | 448 /3769 | 11.89% |
| mech-selected | `mech_compact_numeric_3009_no_growth_composed` | 420 /3769 | 11.14% |
| mech-selected | `mech_compact_closed_checked` | 164 /3769 | 4.35% |
| mech-selected | `mech_compact_numeric_2821_vector_bounded_byte_histogram` | 148 /3769 | 3.93% |
| mech-selected | `mech_compact_numeric_3872` | 114 /3769 | 3.02% |
| mech-selected | `mech_compact_numeric_362_vector` | 73 /3769 | 1.94% |
| libdeflate | `deflate_compress_near_optimal` | 1338 /3022 | 44.28% |
| libdeflate | `deflate_find_min_cost_path` | 674 /3022 | 22.30% |
| libdeflate | `_platform_memset_pattern16` | 629 /3022 | 20.81% |
| libdeflate | `deflate_make_huffman_code` | 228 /3022 | 7.54% |
| libdeflate | `OUTLINED_FUNCTION_10` | 75 /3022 | 2.48% |
| libdeflate | `deflate_optimize_and_flush_block` | 44 /3022 | 1.46% |
| libdeflate | `_platform_memset` | 12 /3022 | 0.40% |
| libdeflate | `deflate_precompute_huffman_header` | 10 /3022 | 0.33% |

## Earlier C4KiB summary correction

The earlier summary included some helper samples outside the encoding call tree. Recomputing the original report gives188 /2900 Huffman samples (6.48%), and555 /2900 memset samples (19.14%), replacing962 and569. The original elapsed-time observations, encoded outputs and win counts are unaffected. The portable verifier checks this correction against the original report stored in the preceding checkpoint archive.

[Fresh sampled shares](sampling.json) · [Portable derivation](derive_sampling.py) · [Prior correction](prior-sampling-correction.json).
