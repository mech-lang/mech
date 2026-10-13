# Explicit intrinsic type contracts

This additive proposal updates only `c_type_environment.py` and its focused fixtures. It does not change shared normalizer/canonical code, sources, timing records, or chart admission. The new fixture methods have not been executed by this source review.

GNU/Clang `__int128`, `signed __int128`, and `unsigned __int128` are explicit compiler primitive spellings. They are recognized without guessing arbitrary identifiers with an `__int128` prefix.

Intrinsic vectors require semantic include metadata proving an exact owned header token. The added contracts are deliberately limited:

| Explicit header | Proven type spellings |
| --- | --- |
| `xmmintrin.h` | `__m128` |
| `emmintrin.h` | `__m128`, `__m128d`, `__m128i` |
| `immintrin.h` | `__m128`, `__m128d`, `__m128i`, `__m256`, `__m256d`, `__m256i`, `__m512`, `__m512d`, `__m512i` |

Static local LLVM headers supplied semantic evidence: `/Library/Developer/CommandLineTools/usr/lib/clang/15.0.0/include/xmmintrin.h:21` defines `__m128`; `emmintrin.h:17–20` includes the SSE header and defines the SSE2 vectors; `avxintrin.h:34–36` defines the AVX vectors; `avx512fintrin.h:33–35` defines the AVX512 vectors. `immintrin.h` includes those headers at lines 26, 31, 66, and 104 in the GNU/Clang path. Header bytes were not hashed or matched to original benchmark builds, so this remains semantic evidence rather than a build certificate.

`xmmintrin.h:3013–3015` has a conditional compatibility inclusion of the SSE2 header. The narrow `xmmintrin.h` contract therefore does not infer `__m128d` or `__m128i` from that target-dependent inclusion. Wider vectors and invented names such as `__m999i` remain gated. Isolated vector declaration fixtures need an explicit include row; the include remains one source unit, followed by independent declarator units.

The shared legacy expression also recognizes `intN_t`, `uintN_t`, `size_t`, and `ssize_t` without source import ownership. These are not new builtin shortcuts in this helper: fixed-width integers require its existing exact `stdint.h` contract, and `size_t`/`ptrdiff_t` have the exact `stddef.h` contract. No broad standard-header contract expansion was made in this pass. Any newly exposed count gap needs actual source/import evidence in the full guarded replay.
