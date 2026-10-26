use super::*;
use mech_core::{
    ManagedMemoryBudget, SchemaBody, SchemaDraft, SchemaTableBuilder,
    snapshot::{SnapshotValidationContext, StringOwnershipTransfer, ValueDataDraft, ValueDraft},
};
use std::cell::Cell;

thread_local! { static SCANS: Cell<usize> = const { Cell::new(0) }; }
thread_local! { static HITS: Cell<usize> = const { Cell::new(0) }; }
pub(super) fn scanned() {
    SCANS.with(|count| count.set(count.get() + 1));
}
fn scans() -> usize {
    SCANS.with(Cell::get)
}
pub(super) fn hit() {
    HITS.with(|count| count.set(count.get() + 1));
}
fn hits() -> usize {
    HITS.with(Cell::get)
}

fn string(text: &str) -> Value {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::String,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    ValueDraft {
        schema: built.resolve(handle).unwrap(),
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text.into()),
    }
    .finalize(&SnapshotValidationContext::new(&built.table))
    .unwrap()
}

fn admitted(text: &str, memory: &ManagedMemoryBudget) -> Value {
    let value = string(text);
    let mut claim = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut claim).unwrap()
}

#[test]
fn exact_frozen_identity_reuses_positive_and_negative_facts_without_content_equality() {
    let mut cache = Cache::default();
    for (text, expected) in [
        ("a\0b\r", true),
        ("a\r\nb", false),
        ("éé", false),
        ("ABCD", true),
    ] {
        let value = string(text);
        let before = scans();
        assert_eq!(cache.inspect(&value).unwrap(), expected);
        assert_eq!(cache.inspect(&value.clone()).unwrap(), expected);
        assert_eq!(scans() - before, 1);
        let equal = string(text);
        assert!(!value.shares_frozen_storage(&equal));
        assert_eq!(cache.inspect(&equal).unwrap(), expected);
        assert_eq!(scans() - before, 2);
    }
}

#[test]
fn same_byte_lengths_never_reuse_an_unrelated_ascii_or_crlf_fact() {
    let mut cache = Cache::default();
    for _ in 0..128 {
        for (text, expected) in [("AB", true), ("é", false), ("\r\n", false), ("\r\0", true)] {
            assert_eq!(text.len(), 2);
            assert_eq!(cache.inspect(&string(text)).unwrap(), expected);
        }
    }
}

#[test]
fn weak_fact_releases_payload_and_its_claim_before_cache_metadata_retires() {
    let bytes = Cache::metadata_bytes();
    eprintln!("ASCII_FACT_METADATA_BYTES {bytes}");
    assert_eq!(
        bytes,
        core::mem::size_of::<Cache>() as u64 + Value::frozen_storage_identity_allocation_bytes()
    );
    let short = ManagedMemoryBudget::new(bytes - 1);
    assert!(short.reserve_capacity(bytes).is_err());
    assert_eq!(short.used_bytes(), 0);
    let memory = ManagedMemoryBudget::new(bytes + 1_000_000);
    let metadata = memory.reserve_capacity(bytes).unwrap();
    let value = admitted(&"x".repeat(65_536), &memory);
    let mut cache = Cache::default();
    assert!(cache.inspect(&value).unwrap());
    drop(value);
    assert_eq!(
        memory.used_bytes(),
        bytes,
        "cached identity must not pin payload or its claim"
    );
    assert!(!cache.inspect(&admitted("é", &memory)).unwrap());
    assert_eq!(memory.used_bytes(), bytes);
    drop(cache);
    drop(metadata);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn weak_fact_does_not_block_consuming_ownership_or_identify_the_new_payload() {
    let memory = ManagedMemoryBudget::new(1_000_000);
    let metadata = memory.reserve_capacity(Cache::metadata_bytes()).unwrap();
    let value = admitted("AB", &memory);
    let mut cache = Cache::default();
    assert!(cache.inspect(&value).unwrap());
    let StringOwnershipTransfer::Owned(owned) = value.try_into_owned_string(&memory).unwrap()
    else {
        panic!("weak cache must not turn unique source into shared storage");
    };
    let mut bytes = owned.into_bytes();
    bytes.as_mut_bytes().copy_from_slice("é".as_bytes());
    assert_eq!(bytes.as_bytes(), "é".as_bytes());
    drop(bytes);
    assert!(!cache.inspect(&admitted("é", &memory)).unwrap());
    assert_eq!(memory.used_bytes(), Cache::metadata_bytes());
    drop(cache);
    drop(metadata);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn original_and_decoded_calls_share_only_the_same_source_fact_and_keep_unicode_results() {
    use crate::native_artifact::{
        JitProgramArtifact,
        program_runtime::tests::{artifact_source, limits},
    };
    let source = "+> map\n+> sequence\nhistogram(s<string>, width<u64>) => (out<{string:u64}>) :=\n  n := sequence/length(s)\n  words := [s[(i + 1u64)..=(i + width)] | i <- 0u64..=n, width > 0u64, width <= n - i]\n  out := map/histogram(words).\na := histogram(first<string>, width<u64>)\nb := histogram(first, width)\nc := histogram(second<string>, width)\na{\"AB\"} + b{\"BC\"} + c{\"éé\"}\n";
    let original = artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = artifact
            .inputs()
            .iter()
            .map(|input| {
                let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                let data = match name.as_str() {
                    "first" => ValueDataDraft::String("ABCD".into()),
                    "second" => ValueDataDraft::String("éé".into()),
                    "width" => ValueDataDraft::U64(2),
                    _ => panic!("unexpected input"),
                };
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values: Box::new([]),
                    data,
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut claim = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                value.into_memory_budget(&mut claim).unwrap()
            })
            .collect::<Vec<_>>();
        let before = scans();
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        assert_eq!(
            scans() - before,
            2,
            "one ASCII source hit, then a fresh Unicode source miss"
        );
        assert_eq!(
            outputs
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            ValueDataDraft::U64(3)
        );
        let before_repeat = scans();
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        assert_eq!(
            scans() - before_repeat,
            2,
            "a new invocation must acquire its own fresh facts"
        );
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn collection_counts_and_map_guards_share_facts_without_byte_counting_unicode_or_crlf() {
    use crate::native_artifact::{
        JitProgramArtifact,
        program_runtime::tests::{artifact_source, limits},
    };
    let source = "+> text\n+> map\n+> sequence\nhistogram(s<string>, width<u64>) => (out<{string:u64}>) :=\n  n := sequence/length(s)\n  words := [s[(i + 1u64)..=(i + width)] | i <- 0u64..=n, width > 0u64, width <= n - i]\n  out := map/histogram(words).\nentries := {\"a\": \"A\", \"b\": \"B\", \"é\": \"É\", \"\\r\\n\": \"\\r\\n\"}\nfirst := text/join([entries{ch} | ch <- seed<string>], \"\")\nsecond := text/join([entries{ch} | ch <- seed], \"\")\nh := histogram(first, width<u64>)\nh{key<string>} + sequence/length(second)\n";
    let original = artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(!jit.descriptor().metadata().window_regions.is_empty());
        for (seed, width, key, expected) in [
            ("abab", 2, "AB", 6),
            ("éé", 1, "É", 4),
            ("\r\n\r\n", 1, "\r\n", 4),
        ] {
            let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
            let args = artifact
                .inputs()
                .iter()
                .map(|input| {
                    let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                    let data = match name.as_str() {
                        "seed" => ValueDataDraft::String(seed.into()),
                        "width" => ValueDataDraft::U64(width),
                        "key" => ValueDataDraft::String(key.into()),
                        _ => panic!("unexpected input"),
                    };
                    let value = ValueDraft {
                        schema: input.schema,
                        shape_values: Box::new([]),
                        data,
                    }
                    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                    .unwrap();
                    let mut claim = memory
                        .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                        .unwrap();
                    value.into_memory_budget(&mut claim).unwrap()
                })
                .collect::<Vec<_>>();
            let before = (scans(), hits());
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            assert_eq!(scans() - before.0, 2, "one seed and one mapped root");
            assert!(
                hits() - before.1 >= 3,
                "both sizing and map guards reuse the seed fact"
            );
            assert_eq!(
                outputs
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::U64(expected)
            );
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
