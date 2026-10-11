# APL FASTA: modular affine-prefix prototype

All 55 normalized lines, including the local generic affine function, are application code. This separate source replaces per-draw interpreter dispatch with an ordinary doubling prefix scan. It materializes every actual seed and binary64 CDF comparison, without a period or output cache. Original frozen sources remain unchanged.

`python3 validate.py` uses the common mutex for complete canonical output, chunk-tail cases and independent generic recurrence fixtures. No full measurements are performed. `../apl-fasta-v3` additionally hoists seed-independent coefficients across chunks and is the intended optimized successor if validation passes; this source stays as preparation evidence.
