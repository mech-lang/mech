# Fuse integer comparisons into ARM64 selections

The trial takes **0.995219×** paired control time across 81 cases. Both 1.05 timing gates pass on every case. Repeated 65,536 and 65,537 bytes beat both peers in three measurements and at the control's original function placement. Natural 8 KiB slows in all four measurements, including by 1.35% in the placement control, where it crosses above parity. Keep the [composed countdown baseline](native-composed-countdown-qualification.md) while isolating the two changed selection sites.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 1.004943× | 0.170442× | 0.171284× | 2.189658× | 2.200482× |
| Natural 8 KiB | 1.008558× | 1.006474× | 1.015087× | 1.244483× | 1.255133× |
| Natural 256 KiB | 1.001399× | 1.145482× | 1.147085× | 0.928218× | 0.929517× |
| Natural 1 MiB | 1.000883× | 1.154316× | 1.155335× | 0.931700× | 0.932523× |
| Repeated 32 KiB | 0.977564× | 0.511072× | 0.499605× | 0.997001× | 0.974631× |
| Repeated 65,535 bytes | 0.986889× | 0.563903× | 0.556510× | 0.996059× | 0.982999× |
| Repeated 65,536 bytes | 0.980097× | 0.571359× | 0.559987× | 1.007534× | 0.987481× |
| Repeated 65,537 bytes | 0.985670× | 0.567578× | 0.559445× | 1.002726× | 0.988357× |
| Periodic 1 MiB | 0.981514× | 0.581649× | 0.570897× | 0.984893× | 0.966686× |

The matched Roc result remains **52/81 (64.2%)**. The libdeflate result goes from 16 to **18/81 (22.2%)**; wins over both go from seven to nine. Wins over either remain 61/81. The 16-case screen takes 0.993474× and the eight-case confirmation 0.992721×. In confirmation the two new wins reach 0.993315× and 0.997764× libdeflate. Natural 8 KiB takes 1.003644× control and 1.045319× Roc; the contemporaneous control takes 1.041524× Roc. The worst full paired elapsed result is empty at 1.008827×. The eight-literal result is adverse in the full run, favorable in the screen and confirmation, and 1.001243× at fixed placement. All blocks are retained.

The opt-in ARM64 rule reuses the conditional branch's existing integer comparison chain proof and emits a conditional select from the final flags. It admits an I64 selection whose condition is a bounded tree of pure integer comparisons and zero/one preserving operations, with one direct user of each condition value within the selection's block. The selection result may be shared. Unrelated condition uses in other blocks still lower independently. No side-effecting operands are sunk. Register allocation moves and spill code between comparisons and the selection preserve flags. There are no compressor identities, field names, or source changes in the rule.

Only two of 357 measured bodies differ. The scalar matcher grows from 30,424 to 30,480 bytes and its frame shrinks from 896 to 880. The vector matcher shrinks from 10,188 to 10,156 bytes with the same 576-byte frame. Its repeated-window endpoint selection replaces materialized Boolean words and `tst` with `cmp; ccmp; ccmp; csel`, removing six instructions at the hot site. Another selection changes in a different route. The earlier profile attributes 294 timer samples to the endpoint `csel` in the retained repeated 65,535-byte run. That is a sample count, not the number of executed comparisons. The root Object body is identical. The natural link preserves 866 of 1,348 text symbol keys at their original addresses.

The placement diagnostic replaces only the relocation-free vector body with exact candidate bytes and eight unreachable padding instructions. All linked function addresses agree with control, and the rest of the executable is identical apart from UUID and code signature. It takes 0.997602× paired control across its eight cases. Repeated 65,536 and 65,537 bytes still win over libdeflate, at 0.992270× and 0.987607×. Natural 8 KiB takes 1.013517× control and crosses from 0.997531× to 1.011014× Roc. Random 8 KiB takes 1.011205× control. This isolates the effect to the changed vector body; internal register allocation, instruction selection and layout still vary.

| Separate hardware counter cohort | Instructions before | Instructions after | After / before cycles |
| --- | ---: | ---: | ---: |
| Eight literals | 626,967 | 626,959 | 1.018720× |
| Natural 8 KiB | 14,706,078 | 14,708,107 | 1.004599× |
| Natural 256 KiB | 565,692,697.5 | 565,719,138.5 | 1.000614× |
| Repeated 65,535 bytes | 24,040,817 | 23,585,222 | 0.983517× |
| Repeated 65,537 bytes | 24,285,408 | 23,829,844 | 0.981918× |
| Periodic 1 MiB | 374,686,073.5 | 367,387,475.5 | 0.981859× |

In the exact candidate's timer profiles, the vector matcher accounts for 1,365/4,842 running samples on natural 8 KiB, 2,748/5,222 on repeated 65,537 bytes, and 3,190/5,365 on periodic 1 MiB. The private payload metrics agree across the matrix. Process lifetime peak physical footprint in the separate counter cohort falls by 16,384 bytes for all eight cases; it includes linked image and runtime, so it does not measure private payload bytes.

The measured parent is `9cf7b6fcdaa6ed5792a61375a0fa39552c27b111` and the compiler patch SHA-256 is `df199200c556a25fb98f53e37cda7cce5914a957c1b9a8c3bb75ae5a8f571c89`. Nightly 2026-03-03 builds with `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` and `-D warnings`. The unchanged revision-14 artifact emits Object `cdf3677e6dc21770e2014f62344e36175adc7418bd4da1dbf06ada97d085d25e`; the linked worker is `2a66f4463e729d62abdfff8d7e7e53fabe5ce3c6e9a5e3b4045a68906a009ad6`. The observed invocation takes 573.57 s. The source frontend takes 0.11 s and the interval between the operation recipe and source CFG markers is 445.31 s. All 1,280 producer-thread observations in one five-second sample land under `OwnerFlow::transfers` and `last_use_with_guards`. The sample identifies a concrete compilation cost; it is not a partition of the entire interval.

The focused ARM64 JIT test passes 157,464 executions covering shared selection results, independently used conditions, raw Boolean words and mixed signed and unsigned widths. The naturally linked decoded image passes 324 linked checks across two descriptors and the unchanged phase passes its 68,096-position direct oracle with rejection and recovery. The fixed-placement worker passes 162 linked checks. The screen, full, confirmation and placement cohorts retain 56,640, 245,664, 55,872 and 55,872 timed calls. Reference output, independent inflate, capacities, aliases, and zero final owners are checked. Original source emission, the managed performance guard, and CI qualification were skipped for the declined patch.

[Compiler patch, paired observations, assembly and verifier](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-9cf7b6fcda-conditional-select.tar.gz). SHA-256 `7fe599756a8a1c71397afa098a1bd609d3263d961805c7cf27ac1d6e78f109eb`. Run `python3 -B verify-round.py` after unpacking.
