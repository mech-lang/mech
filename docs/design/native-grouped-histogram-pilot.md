# Grouped histogram pilot

The first grouped histogram build improved the complete 81-case paired geometric mean to **0.957847× control**. Five random cases around 32 KiB crossed reference parity, raising the candidate’s count from **47/81 (58.0%) to 52/81 (64.2%)**. One stored case missed both 1.05 gates. The retained compiler remains the [equal-join build](native-equal-join-branch-qualification.md).

| Complete-call input | Candidate / control | Control / Roc | Candidate / Roc | Control / libdeflate | Candidate / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Random 1 MiB | 0.821672× | 1.418155× | 1.165258× | 2.007594× | 1.649584× |
| Natural 8 KiB | 0.983900× | 1.045655× | 1.028820× | 1.296684× | 1.275807× |
| Natural 1 MiB | 0.998251× | 1.181621× | 1.179554× | 0.953217× | 0.951550× |
| Eight literals | 1.000012× | 0.171724× | 0.171726× | 2.180074× | 2.180100× |
| Periodic 1 MiB | 1.002531× | 0.580850× | 0.582320× | 0.983024× | 0.985511× |
| Repeated 65,537 bytes | 1.000648× | 0.567689× | 0.568057× | 1.005255× | 1.005907× |

Both workers beat libdeflate on 16 cases and both peers on seven in the same cohort. The candidate beats either peer on 61/81 versus 56/81 for the control. Sixty-seven paired estimates improve. The nine-case pilot passed both gates and measured 0.979518× control, with random 1 MiB at 0.826708× control.

## Checked work and generated code

The compiler derives a unit-stride selected arm that reads a packed high word, increments one bounded U32 cell with wrapping arithmetic, and returns to the cursor header. The existing vector caller checks current shape and unique writable owners. A once-per-phase check admits nonoverlapping initialized extents. The group reads four consecutive eligible items and performs the original ordered increments. Equal keys observe preceding updates. Insufficient bounds, overlapping extents, or a different item kind use the original route before any grouped store.

The compiler recognizes three matching bodies. Each vector body grows from 596 to 800 bytes and from a 192-byte to a 208-byte frame. The four ordered machine updates and the recurring group checks use registers, with the extra stack saves and loads confined to entry and completion. The general phase bodies and other numeric kernels match byte for byte. The generated root grows 1,060 bytes, its stack changes from 55,360 to 55,344 bytes, and its stored-path register assignments and placement change despite the stored path bypassing the group.

The exact transformed phase replay covers 47 calls and eight proof refusals. It measures 0.196270× scalar phase time on the random 1 MiB captures and 1.033958× on the natural 256 KiB captures. These are separate phase-only comparisons.

The exact candidate profile puts 304 of 5,359 running timer PCs in the grouped random 1 MiB tally and 3,819 in the matcher. Samples include validation. In a separate counter cohort, random 1 MiB instructions fall from 482.0M to 458.6M and cycles from 130.94M to 107.35M. Lifetime physical peak is 53.07 MB before and 53.05 MB after. All six tracked private payload allocation, peak, copy, zero and release metrics agree on all 81 cases.

## Timing gate and qualification

The one-byte stored input took 1.063689× control. Its median block means were 70.89 and 74.88 ns. The independent same-worker confirmation passes both gates on all three stored cases and measures 1.023644× paired control on one byte. Both timing cohorts are preserved. The original full campaign passes 80/81 under both unchanged gates. The next variant moves the group-admission decision to the phase and checks whether the entry code can keep the old layout.

The measured compiler uses parent `467ef97a25a70c1e919d84d1ef408d50d616e143` with the archived patch. The release overlay uses nightly 2026-03-03 with warnings as errors, `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`. The unchanged source and revision-14 bytecode have SHA-256 prefixes `331e659e9218` and `51de51658c1d`. The exact candidate Object starts `ea268f7f322f` and the linked worker `780be96e02fd`.

The original control and original candidate each passed 162 linked output, independent inflate, capacity, alias and cleanup checks. The 245,664 complete-call observations preserve the same runtime inputs, source, level, allocator, validation placement, scratch work and output ownership. This build skipped decoded and managed qualification after the full timing gate miss.

[Measured compiler patch, full timing observations, linked assembly and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-467ef97a25-grouped-histogram-pilot.tar.gz). SHA-256 `88bd310903ce4ecdca7556a594bdd496011a3490ad39d2214b50b455f83aed84`. Run `python3 -B verify-pilot.py` after unpacking.
