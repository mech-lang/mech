# Additive platform display correction

Candidate only; the original platform-filter-proposal-v1 and adapter v1 remain frozen. No dataset, frontend hook, source, timing, LOC or language admission has occurred.

The actual platform qualification v2 reconciled 98 of 100 profiles. Its two unresolved Java K profiles lack preserved retime class/JAR artifact pins. Their explicit false backend annotation takes precedence over legacy execution fields, language groups and fixed language overrides. Java's aggregate platform and execution remain Unclassified until every displayed timed profile has compatible verified evidence. Qualified per-profile annotations remain available independently.

`platform.mjs` now reports whether timed profiles contain an authoritative backend annotation. `traits.mjs` uses Unclassified execution when those annotations are unresolved or incomplete; it permits the existing legacy policy only for a language with no measured backend annotations. The candidate fixtures exercise false/invalid backend qualification, stale Java AOT membership, partially annotated and mixed languages, and the Ruby JIT override. The producer separately removes Java from aggregate execution groups for the actual 15 verified / 2 unresolved cohort.

This JS fixture has not run. The four-method Python adapter qualification freezes/pins these candidate bytes but explicitly records zero frontend executions. A separately reviewed frontend check must run under the common mutex before a hook is admitted.

Python candidate: `overview/explorer/platform_metadata_data_v2.py`, its four transaction fixtures, and `continuation/qualify_platform_metadata_data_v2.py`. The guard binds the actual qualified metadata digest `d8920942ab078dc79228d6d77af64391a818354ddc6b5af9cf383ade46dbe8d8`, actual terminal receipt `c49db52205aba82db5e19be6607a03d3934776254bee31506a146800889ff427`, original 100 IDs, all original evidence pins, earlier 16 publication fixtures and four adapter-v1 fixtures. It runs no historical benchmark or runtime-version command and performs no integration.

Proposed fixture-only command, pending root review:

```
python3 -B continuation/qualify_platform_metadata_data_v2.py --output continuation/jobs/platform-metadata-data-v2
```

The four exact actual fixture IDs are:

- `test_platform_metadata_data_v2.PlatformMetadataTransaction.test_complete_actual_overlay_preserves_records_and_object_identities`
- `test_platform_metadata_data_v2.PlatformMetadataTransaction.test_mixed_java_verification_clears_legacy_modes_and_preserves_provenance`
- `test_platform_metadata_data_v2.PlatformMetadataTransaction.test_late_changed_source_report_receipt_or_provenance_leaves_input_unchanged`
- `test_platform_metadata_data_v2.PlatformMetadataTransaction.test_partial_typed_or_foreign_descriptor_evidence_is_transactional`

False rows receive `runtime_policy: Unclassified` and `execution_mode: unknown`. Their exact prior display fields, including field presence and null values, remain in `prior_runtime_metadata` with the pinned original dataset reference and an explicit display-only provenance scope. A conflicting prior metadata object is rejected before any mutation. The adapter stages the entire overlay, prose, groups, source and receipt pins before its object-preserving commit; it performs no reads or validation after committing.

The adapter consumes the exact qualified immutable `snapshot/4-data.json` beside the reviewed metadata output. Its SHA must equal the qualification's original generated `overview/explorer/dist/data.json` SHA and the terminal receipt's saved-baseline SHA. The original path/hash remain provenance; the mutable generated output is never re-read or checked as an immutable prerequisite by the adapter. A later producer run can regenerate that output and add unrelated new-language rows while the exact one hundred reviewed source/report/sample identities still reconcile against the saved baseline. The existing mixed-verification fixture now proves this repeat-build behavior and retained unrelated rows; the late-change method rejects changed baseline bytes without mutation. The fixture guard itself still qualifies once against the currently frozen original inputs before integration is authorized.
