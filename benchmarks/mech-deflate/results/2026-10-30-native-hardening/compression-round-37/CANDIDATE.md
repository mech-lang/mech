# Outline canonical helper fallbacks

Starting published head: `4686620853914517474a23e9d8a6ff1cf11db5cf`.
Status: candidate implementation; no performance gain claimed yet.

The round-36 profile has 60 exclusive weights directly in the shared invocation
helper. Its release disassembly contains about 6300 instructions and allocates
2416 bytes of Rust stack on entry, including on the selected frame paths. The
complete callback census reaches 9156 helpers. Private frame operations are
frequent, while canonical materialization and collection/output fallback bodies
share that same large function.

The first probe investigated ordinary action cleanup instead. The real Huffman
witness reaches 33 actions, 16 retirement entries and only ONE multi-entry
group, repeated identically in original/decoded forms. Output [3,1,3,2], work
91452918, seven lexical batches and zero cleanup remain exact. That is too little
evidence to widen cleanup; no such optimization is enabled. The observation-only
instrumentation is removed, with source and immediately committed receipts
preserved in history.

Instead, outline four existing bodies into non-inlined Rust functions: canonical
structural observation, unselected library materialization, collection execution
and typed output. Bodies retain their original validations, error precedence,
funding, publication and owner handling. All selected paths and common callback
validation remain at their original sites. There is no new native eligibility,
metadata, interpreter, owner, allowance, bytecode or ABI. This is a measured code
layout experiment, not permission to bypass a source operation.

Require the maintained runtime/accounting/value tests, protected scalar and
managed-frame controls, unchanged generated program bytes, exact total work and
a balanced complete-program comparison. Inspect the resulting Rust helper's
stack/code footprint and obtain a fresh profile. Reject an actual regression.

Keep the compressor source, generated program, allocator and timing boundary
unchanged. Compare complete eight-byte compression against the frozen round-36
worker, report both the common fresh reference and historical 0.300 ms ruler,
and commit every observation cohort immediately. No larger-input timing.
