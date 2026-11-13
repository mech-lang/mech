# Compressor source size and compilation

These counts exclude benchmark harnesses and count physical lines, including
comments and blanks. They describe the compression implementations, rather than
the compiler repositories or entire libraries.

| Implementation | Physical lines | Compile elapsed | Machine and boundary |
| --- | ---: | ---: | --- |
| libdeflate 1.26 | 5,485 | 0.373 s | M3, encoder source to Object, Clang O2 |
| Roc, pinned reference | 6,187 | 6.101–6.434 s | Historical M1, 12 source-to-executable builds, `--no-cache --opt=speed` |
| Mech, current benchmark flags | 1,930 | 57.933–58.188 s | M3, fresh source to Object, including canonical encoding |

The full libdeflate static library also builds in **1.500 s** on this M3. It
includes decompression, checksums and zlib/gzip wrappers; the 0.373-second
measurement compiles only the raw DEFLATE encoder translation unit. Tool builds,
source downloads and host/application builds are excluded from the M3 timings.
These are individual builds, not a statistical timing campaign.

The source scopes differ. C and Roc support other compression levels, while
this Mech implementation is level 12 only. The C count covers
`deflate_compress.c`, its three matchfinders, common and ARM matchfinder helpers,
and the internal DEFLATE header/constants: eight files. Roc counts its twelve
compression package modules, excluding `Inflate.roc` and the one-line package
declaration. Shared public interfaces can contain a small decompression wrapper;
the decoder implementation is excluded. Neither is a complete transitive count
of compiler builtins, generic library headers or runtime code.

Mech's 25 component files have **1,905 lines**. Their assembled source has
25 additional separators and one benchmark entry call: **1,931 lines** total,
or **1,930** excluding that call. This explains the 1,931-line count used in the
[earlier C comparison](native-libdeflate-comparison.md), whose narrower C count
was 4,779 lines for the encoder and level-12 matchfinder headers. Line counts are
sensitive to formatting and are not a normalized measure of algorithm complexity.

The Roc timings were recovered from the original checked build receipt at Git
snapshot `6ab75037bc5d4ec138d735c7dcfe1c4b653d31fa`, using source
`671077457ff7ef3a3d44caf715a3fc595addf379` and release-fast compiler
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`. Those applications embed the inputs
and select the level at runtime. They are not fresh M3 timings of the later
fixed-level, runtime-input worker. The exact compiler is absent on this M3 and
its successful CI run exposes no compiler artifact; a current release was not
substituted. Machine and compilation boundaries therefore prevent a controlled
language-speed ratio.

The M3 libdeflate build uses the unchanged benchmark's pinned upstream source
`92e6a0db9fa848d742f9eb286c92afc60f2c3dda`, Apple Clang, ARM64, O2 and NDEBUG.
All fourteen Roc package hashes agree with the maintained reference pins. The
[Mech compile study](native-compile-under-minute-profile.md) records exact native
output preservation, qualifications and the separately measured 55.704-second
default configuration.

The [compact result](../../benchmarks/mech-deflate/native-profile/compressor-size-compile-20261113.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/compressor-size-compile-20261113.archive.json)
pin [the counted sources, build commands, historical receipt and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-compressor-size-compile-20261113.tar.gz):
38 files, SHA-256 `f337bdddcd215bb053855675b7c99c9cf96c54a16f17e1168a011a2d669aafd1`.
No compiler or application executable is included. The packet's `replay.py`
checks every counted source and reconstructs the table without compiling.
