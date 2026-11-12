# Four packed literal tally visits

The census reads the current node window at the linked tally entry in the reverse cursor image from `aa26a947138681dcc6a0d21434ae68b98ce248b0`. A debugger-only signature was applied to a scratch copy after the unentitled launch was denied. The copy's entire 1,552,756-byte executable text section matches the timed worker. The debugger samples each phase entry once and reconstructs the exact itinerary from the current packed words. Two complete invocations per case have identical entry receipts and pass exact reference, independent inflate, live alias, and cleanup checks.

| Input | Tally visits | Literal updates | Match updates | Literal runs | Updates in groups of four | Share of all visits |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Random 1 MiB | 2,995,430 | 2,994,580 | 850 | 858 | 2,993,312 | 99.93% |
| Natural 256 KiB | 233,863 | 76,162 | 157,701 | 48,891 | 9,756 | 4.17% |

The diagnostic uses the exact tally CLIF and preserves the original match and scalar literal branches. At a selected literal, it checks that four source visits fit the current window and end, and that the node reads and updated bins are disjoint. It reads the next three packed lengths before any store. Four unit-length items execute four wrapping U32 increments in original order, so repeated keys see preceding writes. Any refusal enters the original scalar literal arm. Exact tally CLIF and VCode took 154 seconds to capture. The diagnostic producer was stopped after writing the phase.

The JIT checks compare status, output packet, and all backing contents for 21 real phase inputs and 26 controls. These include repeated keys with wrapping, empty and short tails, near-limit indices, a later checked histogram failure, and a live alias whose first increment changes the next node. The baseline compiles to the same 596-byte tally body size as the linked worker. The prototype grows to 924 bytes and increases the frame from 192 to 240 bytes. The generated group route rematerializes the invariant geometry checks each time it attempts a group.

| Isolated tally phase | Scalar median | Group median | Group/scalar paired elapsed |
| --- | ---: | ---: | ---: |
| Random 1 MiB | 8.807 ms | 2.705 ms | **0.307461×** |
| Natural 256 KiB | 1.073 ms | 1.231 ms | **1.148343×** |

Each row sums that input's actual phase invocations per order. The measurement uses 24 alternating pairs and excludes packet reset and validation. The next compiler experiment is to carry the admitted geometry through the phase and reduce the failed-group cost on mixed data. The current result measures the isolated tally function. Full compression timing requires the compiler to select and emit the operation from the proved source shape.

[Census, phase CLIF, diagnostic patch, and timing replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-aa26a94713-tally-phase-pilot.tar.gz). Archive SHA-256 `27573c553ce525af5b70e7091123dfb9ee4e2f5b106b4fcf8388d3fbf2a83bcb`.

## Admit geometry at the phase entry

The separate entry experiment checks the same current node and bin extents once per call, before dispatch to the grouped body. It selects the original scalar entry if a length, checked address extent, or disjointness test fails. The grouped body continues to test the current cursor, window, and four lengths before updates. The original 47 differential controls pass, including the alias case whose first store changes the next node and the late bounds error.

| Isolated tally phase | Scalar median | Entry-admitted group median | Group/scalar paired elapsed |
| --- | ---: | ---: | ---: |
| Random 1 MiB | 8.779 ms | 1.721 ms | **0.195524×** |
| Natural 256 KiB | 1.068 ms | 1.087 ms | **1.019425×** |

Both order halves agree on the direction in the 24-pair follow-up. The timer includes the once-per-phase dispatch. Each experiment has its own direct scalar control. These receipts give the compiler integration a concrete boundary to reproduce through the existing private buffer and view admission. The next complete-call measurement can then compare the resulting source-derived fast path across the 81 cases.

[Entry admission patch, test receipts, and timing replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-aa26a94713-tally-entry-admitted.tar.gz). Archive SHA-256 `e5a907b4951dea225a0383c5a12f06836b0087ffb396920b8e33867cf57d2924`.
