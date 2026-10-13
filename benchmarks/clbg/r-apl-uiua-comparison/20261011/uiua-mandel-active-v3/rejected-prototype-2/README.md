# Additive Uiua active-pixel Mandelbrot candidate

This 14 LOC optimized candidate keeps only the pixels that have not escaped
in its recurrence vectors. Every requested pixel starts at zero, performs
the exact binary64 recurrence until its first escape or 50 iterations, and
contributes its canonical PBM bit. The prior qualified 11 LOC application
and all existing measurements remain unchanged and selected. This additional
candidate has no measured result; measurements determine eventual endpoints.

After each recurrence, ordinary `Where` converts the inside mask into a
stable index vector. Ordinary native `Select` gathers the surviving real and
imaginary states, original pixel indices, and real coordinates. Imaginary
coordinates are common to the row and stay scalar. There is no callback per
pixel and no pairwise membership matrix. At the end, ordinary `Under Select`
sets the remaining original indices to one in a zero vector of exactly the
requested width. The unchanged native weight/row-reduction packer pads only
the final PBM byte with zeros.

All helpers are counted application source. The general runtime array
operations are unmodified. There is no native benchmark kernel, analytic
interior test, input cache, approximate floating point, or skipped requested
pixel. The application operates on one row at a time with linear storage.
This source draft is pending validation. Its final evidence, source audit and
manifest are replaced atomically while the validator holds the shared lock;
no completed candidate metadata is published before all checks pass.

Preparation and bounded validation are isolated in this directory. Every
executable probe and independent reference calculation must acquire
`/tmp/mech-clbg-timing.lock`; no active full trial is interrupted. No refreeze
or full timing is permitted without root approval. Validation will compare
complete PBMs, all active pixel identities/coordinates, every recurrence
state including the escape iteration, generic select/scatter operations,
and odd-width byte padding.
