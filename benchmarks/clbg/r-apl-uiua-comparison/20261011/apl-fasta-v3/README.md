# APL FASTA: native affine-prefix arrays

This is a separate, unmeasured candidate. All benchmark code and its local coefficient function count toward the 55 normalized lines. The frozen original sources and runtime remain unchanged.

The generic coefficient function composes `s -> (a*s+c) mod m` transformations with a doubling prefix scan. It accepts the modulus, multiplier, increment and prefix length. A fixed chunk capacity lets the program reuse seed-independent coefficients, then calculate every actual seed from the current carry. A shorter final chunk uses a prefix slice. It does not reuse generated output, identify the input generator's period, skip positions or replace counts with formulas.

Each generated seed is divided by 139968.0 in binary64, compared with the original unscaled cumulative probabilities, and written with the canonical record headers and 60-character wrapping. The last actual seed carries across chunks and records. The canonical coefficient products fit exactly in binary64 and in signed 64-bit integer arithmetic.

The earlier `../apl-fasta-v2` candidate recomputes the same coefficients for each chunk and remains preserved as preparation evidence. Its output is intended to be identical; no performance claim is made before measurement.

Run `python3 validate.py` for independent canonical output comparisons, record/chunk-tail cases, and arbitrary-coefficient/seed recurrence fixtures. The validator takes the shared benchmark mutex. Only a validated manifest may join the full runner at an explicit stage boundary; three real full trials and fresh Mech controls remain necessary.
