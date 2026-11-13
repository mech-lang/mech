# Reuse the current cache header ordinal

The composed phase forms the current header ordinal by subtracting one from the cursor. It repeats that subtraction at the header load, when forming the first record ordinal, and when comparing each record against the header. The fixed-address trial keeps the result in the existing cursor register until the record walk completes. The cache view, current bounds checks, zero defaults, first record, typed completion, append store, and both outer continuation cursors retain their values. The trial introduces a jump after the store to keep subsequent code at the same addresses.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation control → trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.161477× → 0.161439× | 0.161030× → 0.161083× | 2.063119× → 2.063800× |
| Natural 16 KiB | 1.039574× → 1.042266× | 1.033498× → 1.039022× | 1.042385× → 1.047957× |
| Reymont 16 KiB | 1.033771× → 1.039294× | 1.034928× → 1.039512× | 1.040784× → 1.045394× |
| Reymont 32,769 bytes | 1.064479× → 1.070157× | 1.060861× → 1.072738× | 0.965916× → 0.976731× |
| Natural 256 KiB | 1.081229× → 1.094217× | 1.081257× → 1.095498× | 0.871591× → 0.883070× |

The 18-case screen takes 1.002441× paired control and the eight-case confirmation takes 1.003108×. Both gates pass on all 26 comparisons and the peer win sets agree within both cohorts. Both 16 KiB cases slow in both comparisons. The previous compiler stays retained. The extra jump and changed internal instruction positions are part of this diagnostic's measured effect.

Measured compiler `170401d69aaeded4304b2992763dd5dfd7e9aabf`, Object `a8982ef649c33c742d631b849380a7914d7161de678acb338e5c26e53e4a8c1d` and worker `3ac9ad22c03609a3c5ae997cfb114b206c72dfc538a214c229d723940512e093`. The one 1,216-byte body retains its 224-byte frame. All 1,276 named symbol addresses and the two table relocations agree. The executable passes 162 complete linked checks and 14,592 timed calls with exact streams, independent inflation, capacity, aliases and cleanup. Full-matrix timing, independent counters and a compiler implementation were skipped for this declined trial.

The exact-worker profiles put 2,583/5,289 running timer PCs in the main composed parser at natural 16 KiB, 2,678/5,359 at Reymont 32,769 bytes and 3,040/5,363 at natural 256 KiB. The samples include validation. The public replay verifies 102 files. Archive SHA-256 `a99c8c4a7b2973bd2604210e19393b32d81b60e2008d9ef2075c09e04d5e6e70`. [Timing and profile replay, proof and linked code](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-composed-header-cursor.tar.gz).
