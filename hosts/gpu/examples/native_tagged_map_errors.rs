//! Same-source semantic/error reference, compatible with pre-tagged compilers.
//! Run the old closure with `canonical`, and the new closure with `tagged`;
//! their JSON lines must be identical. Work, policy and timing are excluded.
use mech_core::{
    ManagedMemoryBudget, Value, ValueDataDraft as Data,
    snapshot::{MapEntryDraft, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const SOURCE: &str = include_str!("../tests/fixtures/native/private-tagged-map.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    steps: u64,
    seed: u64,
    allowed: bool,
    divisor: u64,
    query: &str,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let data = match name.as_str() {
                "keys" => Data::Matrix([Data::String("a".into())].into()),
                "initial" => Data::Map(
                    [MapEntryDraft {
                        items: [Data::String("old".into()), Data::U64(3)].into(),
                    }]
                    .into(),
                ),
                "steps" => Data::U64(steps),
                "seed" => Data::U64(seed),
                "allowed" => Data::Bool(allowed),
                "divisor" => Data::U64(divisor),
                "marker" => Data::U64(2),
                "query" => Data::String(query.into()),
                _ => panic!("unexpected input {name}"),
            };
            let shape_values: Box<[u64]> = if let Data::Matrix(values) = &data {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(port.schema).unwrap(),
                    &[1, values.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .into()
            } else {
                Box::new([])
            };
            let value = ValueDraft {
                schema: port.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut claim = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut claim).unwrap()
        })
        .collect()
}

fn contents(values: &[Value]) -> Vec<Data> {
    values
        .iter()
        .map(|value| value.canonical_data_draft().unwrap())
        .collect()
}

fn run() {
    let expected_tagged = match std::env::args().nth(1).as_deref() {
        Some("canonical") => false,
        Some("tagged") => true,
        _ => panic!("expected canonical or tagged"),
    };
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x54414745), Revision(0), SOURCE).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for (form, artifact) in ["original", "decoded"].into_iter().zip([original, decoded]) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let frames = jit.descriptor().plan().facts().buffer_frames();
        let tagged = jit
            .descriptor()
            .plan()
            .blocks()
            .iter()
            .enumerate()
            .any(|(site, _)| {
                frames.action(site).is_some_and(|action| {
                    format!("{:?}", action.operation()).starts_with("EnumPack")
                })
            });
        assert_eq!(
            tagged, expected_tagged,
            "the reference must exercise the requested real carrier route"
        );
        let memory = mode.memory_budget();
        let mut outputs = vec![None; artifact.outputs().len()];
        for (case, (steps, seed, allowed, divisor, query, expected)) in [
            (1, 5, true, 1, "old", Some(11)),
            (2, 5, false, 0, "missing", None),
            (2, 5, true, 0, "missing", None),
            (2, 5, true, 1, "old", None),
            (1, u64::MAX, true, 1, "old", None),
            (0, 13, true, 1, "missing", None),
            (0, 13, false, 0, "old", Some(18)),
            (1, 17, true, 1, "a", Some(37)),
        ]
        .into_iter()
        .enumerate()
        {
            let args = arguments(&artifact, &memory, steps, seed, allowed, divisor, query);
            let aliases = args.clone();
            let before = contents(&args);
            let prior = outputs.clone();
            let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let result = jit.evaluate_into(&args, &mut outputs, mode.execution(), &memory);
            let error = result.err().map(|error| format!("{error:?}"));
            let actual = outputs
                .iter()
                .map(|value| value.as_ref().unwrap().canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            if let Some(expected) = expected {
                assert!(error.is_none());
                assert_eq!(actual.last(), Some(&Data::U64(expected)));
            } else {
                assert!(error.is_some());
                assert!(outputs.iter().zip(&prior).all(|(actual, old)| {
                    actual
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    held
                );
            }
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
            println!(
                "{}",
                serde_json::json!({"witness":"tagged-map-semantic-error","form":form,"case":case,"error":error,"outputs":actual,"inputs_and_aliases_unchanged":true,"old_publication_preserved_on_error":true,"failure_claims_released":true})
            );
        }
        drop(outputs);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn main() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
