# Gleam current candidate: corrected launch preparation

The v1 attempt stopped at the read-only source gate before binding inputs or creating a candidate. Root had hashed the accepted source packet without setting its read-only permissions. The actual failure is retained in jobs/gleam-current-candidate-v1 and its closed review.

This additive worker is byte-for-byte v1 except its fresh exclusive output directory (gleam-current-candidate-v2) and this review filename. All data validation, original fixture reuse, actual metadata and published baseline checks are unchanged. The previous complete source review remains applicable. Before launch root will set the worker, unchanged packet, original inventory helper, and this note read-only, then capture actual hashes. No native or fixture rerun occurs; live data is not modified. Original failures remain intact.
