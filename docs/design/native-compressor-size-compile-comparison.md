# Compressor source size and compilation

The compression implementations contain **5,485 physical lines in libdeflate,
6,187 in Roc and 1,930 in Mech**, including comments and blanks, with benchmark
wrappers excluded. All three now have fresh compilation measurements on this
Apple M3 Air, using native Object/archive output and excluding host linking,
compiler bootstrap and source/tool downloads.

| Implementation | Physical lines | Fresh whole-process compile | Boundary |
| --- | ---: | ---: | --- |
| libdeflate 1.26 | 5,485 | 0.557 s | Encoder translation unit to Object, Clang O2 |
| Roc, pinned reference | 6,187 | 2.249 s | Fixed level 12 with runtime input, `--no-cache --opt=speed --jobs=1`, static archive |
| Mech, latest qualified flags | 1,930 | 37.144–37.963 s | Two fresh source-to-Object processes, compact compiler |

The Mech compiler's own phase totals are **36.728–36.876 seconds** after the
[checked tail-finish lookup change](native-tail-role-compile-profile.md),
which reduces preparation by 4.3% and complete compilation by 1.6% in its
matched cohort. The preceding
[numeric forest reachability change](native-numeric-reachability-compile-profile.md)
reduces preparation by 17.1% and complete compilation by 4.6%. The preceding
[inline dimension-remapping change](native-frontend-affine-compile-profile.md)
reduces frontend time by 21.6% and complete compilation by 4.5%. The preceding
[scoped numeric-facts change](native-numeric-facts-compile-profile.md)
reduces native compilation by 5.1%. The preceding
[sparse SSA lookup change](native-ssa-sparse-compile-profile.md) reduces
source CFG construction by 67.9%. The preceding
[bounded ownership-query change](native-owner-read-compile-profile.md)
reduces preparation by 19.7%. These changes preserve every emitted program byte. The earlier
parser optimization is [qualified separately](native-parser-prediction-profile.md):
parsing falls from 3.43–3.44 seconds to 2.45–2.49 seconds with identical program
output. The [preceding parser change](native-parser-compile-profile.md) reduced
the original 10.13–10.29-second parse. The full
libdeflate static library also builds in **1.500 seconds**; it includes
decompression, checksums and zlib/gzip wrappers. Earlier individual encoder
measurements ranged from 0.373 to 0.557 seconds. These are individual builds,
with different implementation scopes and optimization strategies.

| Compiler | Where measured compilation time goes |
| --- | --- |
| Clang | Frontend 55 ms; LLVM optimization 197 ms and code-generation passes 234 ms within a 432 ms backend |
| Roc | Type checking 65 ms, including 6 ms parsing; shared lowering/evaluation 333 ms; specialization 1,235 ms, including 1,164 ms LIR passes; LLVM optimize/emit 642 ms |
| Mech | Parsing 2.44–2.49 s; frontend 6.78–6.85 s; native compilation 27.30–27.58 s, including 8.53–8.55 s preparation, 4.69–4.70 s numeric generation, 2.45–2.53 s source CFG construction and 5.01–5.09 s cleanup |

Nested entries in this table must not be added twice. Clang's instrumented
process takes 0.549 seconds; Roc's takes 2.313 seconds. Profiled and unprofiled
outputs match byte-for-byte for both peers. Roc's short CPU sample includes
`use_order` and `range_prove` paths; its built-in timings establish the complete
phase costs. Mech's samples show substantial schema canonicalization/cloning,
reachability and SSA work before machine-code emission.

The emitted `__text` sizes are **41,112 bytes for C**, **57,696 for Roc** and
**1,101,332 for the latest Mech configuration**. Maximum resident memory in the
clean process measurements is **75,038,720 bytes**, **445,988,864 bytes** and
**7,758,872,576 bytes** in both observations, respectively. Mech's expanded native work and retained
compiler data deserve attention; fewer source lines do not imply less compiler
work. Runtime speed comparisons retain their own linked-worker qualification.

The source scopes differ. C and Roc support other compression levels, while
this Mech implementation is level 12 only. The C count covers the encoder,
three matchfinders, common and ARM matchfinder helpers, and internal DEFLATE
headers/constants: eight files. Roc counts twelve compression modules, excluding
`Inflate.roc` and the one-line package declaration. The shared `Deflate` API also
imports the decoder, so the measured Roc frontend checks `Inflate` even though
the emitted entry compresses at level 12. Transitive compiler builtins, generic
library headers, platform code and benchmark wrappers are outside the counts.

Mech's 25 component files have **1,905 lines**. Assembly adds 25 separators and
one benchmark entry call: **1,931 lines** supplied to the compiler, or **1,930**
excluding that call. The [earlier C comparison](native-libdeflate-comparison.md)
used a narrower 4,779-line C scope. Formatting and source scope make these counts
unsuitable as a normalized measure of algorithm complexity.

Pins are unchanged: libdeflate
`92e6a0db9fa848d742f9eb286c92afc60f2c3dda`, Roc compression source
`671077457ff7ef3a3d44caf715a3fc595addf379`, and release-fast Roc compiler
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`. The exact Roc compiler was built
locally with Zig 0.17.0; its SHA-256 is
`fafc7e230a14f7165be49fdd46dec10a98373e39acd4e484d95424e16a5426bf`.
The static archive excludes the Rust host and allocator inputs used by the linked
runtime benchmark. Compiler build time is recorded outside program timings.

The [current Mech result](../../benchmarks/mech-deflate/native-profile/tail-role-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/tail-role-compile-apple-m3-20261113.archive.json)
pin its latest qualified cohort. The [peer compact result](../../benchmarks/mech-deflate/native-profile/parser-peer-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/parser-peer-compile-apple-m3-20261113.archive.json)
pin the commands, sources, traces, phase reports, samples and offline replay.
The [original count receipt](../../benchmarks/mech-deflate/native-profile/compressor-size-compile-20261113.archive.json)
retains the earlier 6.101–6.434-second M1 Roc source-to-executable builds and their
different embedded-input/runtime-level entry. Those historical values do not
measure hardware speedup against this runtime-input, fixed-level M3 archive.
