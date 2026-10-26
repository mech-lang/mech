//! Observe the actual emitted map's storage mode, not a replacement kernel.
use super::*;
use crate::native_artifact::JitProgramArtifact;
use crate::native_artifact::program_runtime::tests::{artifact_source, limits};
use mech_core::{
    ManagedMemoryBudget, ValueData,
    snapshot::{SnapshotValidationContext, ValueDataDraft as Data, ValueDraft},
};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Observation {
    owned: bool,
    aliased: bool,
    source_present: bool,
}
thread_local! { static LAST: Cell<Option<Observation>> = const { Cell::new(None) }; }
pub(super) fn observe(
    ctx: &Invocation<'_, '_, '_>,
    region: &NativeStringMapRegion,
    aliased: bool,
    owned: bool,
) {
    LAST.with(|last| {
        last.set(Some(Observation {
            owned,
            aliased,
            source_present: ctx
                .state
                .arena
                .test_current_value(region.source().get())
                .is_some(),
        }))
    });
}

const JOINED: &str = "+> text\n+> map\n+> sequence\nparts := text/lines(seed<string>)\nsize := sequence/length(parts)\nrecord := text/join(parts[1u64..=size], \"\")\nentries := {\"a\": \"A\", \"b\": \"B\", \"é\": \"É\"}\nnormalized := text/join([entries{ch} | ch <- record], \"\")\n";

fn argument(
    artifact: &mech_engine::ProgramArtifact,
    index: usize,
    text: &str,
    memory: &ManagedMemoryBudget,
) -> Value {
    let input = &artifact.inputs()[index];
    let name = mech_engine::decode_source_input_name(&input.name).unwrap();
    let value = ValueDraft {
        schema: input.schema,
        shape_values: Box::new([]),
        data: if name == "flag" {
            Data::Bool(false)
        } else {
            Data::String(text.into())
        },
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut capacity = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut capacity).unwrap()
}

fn variants(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let original = artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

fn run_case(source: &str, text: &str, expected: &str, mode: Option<Observation>) {
    for artifact in variants(source) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = artifact
            .inputs()
            .iter()
            .enumerate()
            .map(|(index, _)| argument(&artifact, index, text, &memory))
            .collect::<Vec<_>>();
        let identities = args
            .iter()
            .map(Value::frozen_storage_identity)
            .collect::<Vec<_>>();
        let mut outputs = vec![None; artifact.outputs().len()];
        LAST.with(|last| last.set(None));
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        assert_eq!(LAST.with(Cell::take), mode);
        assert!(
            matches!(outputs.last().unwrap().as_ref().unwrap().data(), ValueData::String(actual) if actual.as_ref() == expected)
        );
        for (input, identity) in args.iter().zip(&identities) {
            assert!(identity.matches(input));
        }
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn emitted_joined_string_map_reuses_backing_without_a_canonical_source_slot() {
    run_case(
        &format!("{JOINED}normalized\n"),
        "ab\nba",
        "ABBA",
        Some(Observation {
            owned: true,
            aliased: true,
            source_present: false,
        }),
    );
}

#[test]
fn complete_unchanged_knucleotide_body_uses_the_owned_emitted_map() {
    let source = include_str!("../../tests/fixtures/native/knucleotide-idiomatic.mec")
        .lines()
        .filter(|line| !line.starts_with("+> @in :=") && !line.starts_with("+> @out :="))
        .map(|line| {
            line.strip_prefix("@out/text <- ")
                .unwrap_or(line)
                .replace("@in/text", "seed<string>")
        })
        .collect::<Vec<_>>()
        .join("\n");
    run_case(
        &source,
        ">ONE\nignored\n>THREE description\nac\ngt\n>FOUR\nignored\n",
        "A 25.000\nC 25.000\nG 25.000\nT 25.000\n\nAC 33.333\nCG 33.333\nGT 33.333\n\n0\tGGT\n0\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n",
        Some(Observation {
            owned: true,
            aliased: true,
            source_present: false,
        }),
    );
}

#[test]
fn observed_or_cold_source_consumers_retain_the_immutable_copy_path() {
    run_case(
        &format!("{JOINED}normalized + \"|\" + record\n"),
        "ab\nba",
        "ABBA|abba",
        Some(Observation {
            owned: false,
            aliased: false,
            source_present: true,
        }),
    );
    run_case(
        &format!("{JOINED}flag<bool>?\n  | true => record\n  | false => normalized\n"),
        "ab\nba",
        "ABBA",
        Some(Observation {
            owned: false,
            aliased: false,
            source_present: true,
        }),
    );
}

#[test]
fn unicode_source_keeps_the_original_grapheme_route() {
    run_case(&format!("{JOINED}normalized\n"), "é\né", "ÉÉ", None);
}

#[test]
fn external_source_aliases_keep_original_snapshot_identity_and_bytes() {
    let source = "+> text\n+> map\nentries := {\"a\": \"A\", \"b\": \"B\"}\ntext/join([entries{ch} | ch <- seed<string>], \"\")\n";
    run_case(
        source,
        "abba",
        "ABBA",
        Some(Observation {
            owned: false,
            aliased: false,
            source_present: true,
        }),
    );
}

#[test]
fn repeated_parent_visits_do_not_consume_a_lexically_single_use_source() {
    let source = "+> text\n+> map\n+> sequence\nnormalize(value<string>) => (result<string>) := result := text/join([entries{ch} | ch <- value], \"\").\nparts := text/lines(seed<string>)\nsize := sequence/length(parts)\nrecord := text/join(parts[1u64..=size], \"\")\nentries := {\"a\": \"A\", \"b\": \"B\"}\ntext/join([normalize(record) | repeat <- [1u64 2u64]], \"|\")\n";
    run_case(
        source,
        "ab\nba",
        "ABBA|ABBA",
        Some(Observation {
            owned: false,
            aliased: false,
            source_present: true,
        }),
    );
}

#[test]
fn owned_map_missing_key_is_atomic_and_releases_transferred_capacity() {
    for artifact in variants(&format!("{JOINED}normalized\n")) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = artifact
            .inputs()
            .iter()
            .enumerate()
            .map(|(index, _)| argument(&artifact, index, "ab\nba", &memory))
            .collect::<Vec<_>>();
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        drop(args);
        let previous = outputs.clone();
        let bad = artifact
            .inputs()
            .iter()
            .enumerate()
            .map(|(index, _)| argument(&artifact, index, "ab\n?", &memory))
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        LAST.with(|last| last.set(None));
        let error = jit
            .evaluate_into(&bad, &mut outputs, limits(), &memory)
            .unwrap_err();
        assert!(format!("{error:?}").contains("InvalidInput"));
        assert_eq!(
            LAST.with(Cell::take),
            Some(Observation {
                owned: true,
                aliased: true,
                source_present: false
            })
        );
        for (output, previous) in outputs.iter().zip(&previous) {
            assert!(
                output
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(previous.as_ref().unwrap())
            );
        }
        assert_eq!(memory.used_bytes(), before);
        drop((bad, outputs, previous));
        assert_eq!(memory.used_bytes(), 0);
    }
}
