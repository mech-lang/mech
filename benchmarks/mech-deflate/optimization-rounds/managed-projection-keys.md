# Round 10: validated keys for managed frame projections

Assigned after the read-only-loop baseline `0a0dae3b5`. The stage-2 implementation
is preserved separately and is not part of this comparison. Compiler baseline
remains `4af4662097dd570f0821fc105a1c4d9de4eb5d89`, executable SHA-256
`0fa38254f8f5773f49749201a07ad2032356d38b5d7f47a44f4c581a6bdc6570`.

Reuse the existing prepared value-slot key at a managed frame projection instead
of `NativeProgramType::schema_key`, which canonical-encodes and hashes the
immutable definition on every projection. Establish exact slot-index, schema,
definition and schema-table-key alignment during preparation and independent
image restoration. Do not accept a key merely because it is valid elsewhere in
the table. Do not add another cache or change mutable geometry/admission.

Keep exact type rejection, original/decoded/restored images, finite admission,
failure publication, aliases, cleanup and retry. Run the relevant established
frame/component tests, protected scalar latency/allocation controls and static
contracts. Existing adversarial Engine definition tests retain their coverage.
Add focused corruption tests for a swapped pair of independently valid type
slots, a wrong key and a changed definition. Test filters must execute tests.

Then compare the unchanged pinned eight-byte complete compressor against the
retained executable with the same selected catalogue, pinned allocator, artifact,
NoLimits diagnostic mode and evaluate_into boundary. Use a fresh common reference,
retain every sample and take the separate profile. Logical work and finite
allowances must remain unchanged. Keep the optional source variant separate.
Commit all observations and publish to the existing draft before resuming the
native numeric leaf work. No cancelled large-input campaign is restarted.
