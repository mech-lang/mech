# Distribution profile baselines

These committed profile snapshots predate `logic/all`. The operation adds one
runtime factory to profiles that select the complete logic operation set and,
in source profiles, one named specializer and one module-only export. It does
not add a prelude export. Selected profiles that omit logic are unchanged.

The stdlib profile tests verify this addition explicitly and check every older
entry against the historical profile fingerprints. Catalog diagnostic output
reports current counts and digests, including the addition. The JSON snapshots
retain their recorded values until an explicit distribution-report refresh;
their old digests are not current-catalog measurements.
