//! Allocation-count regression for a fixed immutable map in a tagged FSM.
//! Compile this separately from the test binary so its allocator is isolated.
//! Vary transition count while keeping the admitted map and output shape fixed.
use mech_core::{
    ManagedMemoryBudget, Value, ValueData, ValueDataDraft as Data,
    snapshot::{MapEntryDraft, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[allow(dead_code)]
#[path = "../../../src/core/tests/support/allocation_probe.rs"]
mod allocation_probe;
#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

const SOURCE: &str = include_str!("../tests/fixtures/native/private-tagged-map-forward.mec");

fn inputs(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    entries: usize,
    steps: u64,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let data = match name.as_str() {
                "steps" => Data::U64(steps),
                "seed" => Data::U64(7),
                "initial" => Data::Map(
                    (0..entries)
                        .map(|index| MapEntryDraft {
                            items: [
                                Data::String(if index == 0 {
                                    "old".into()
                                } else {
                                    format!("key-{index}")
                                }),
                                Data::U64(19 + index as u64),
                            ]
                            .into(),
                        })
                        .collect(),
                ),
                _ => panic!("unexpected input {name}"),
            };
            let value = ValueDraft {
                schema: port.schema,
                shape_values: Box::new([]),
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

fn run() {
    let expected_tagged = match std::env::args().nth(1).as_deref() {
        Some("canonical") => false,
        Some("tagged") => true,
        _ => panic!("expected canonical or tagged"),
    };
    let mut zero_slope = true;
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5441474d), Revision(0), SOURCE).unwrap(),
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
        let packs = jit
            .descriptor()
            .plan()
            .blocks()
            .iter()
            .enumerate()
            .filter(|(site, _)| {
                frames.action(*site).is_some_and(|action| {
                    format!("{:?}", action.operation()).starts_with("EnumPack")
                })
            })
            .count();
        assert_eq!(
            packs > 0,
            expected_tagged,
            "allocation probe must exercise the requested real carrier route"
        );
        for entries in [1, 64] {
            let mut counts = None;
            for steps in [0, 1, 64, 2048] {
                let memory = mode.memory_budget();
                let args = inputs(&artifact, &memory, entries, steps);
                let aliases = args.clone();
                let before = args
                    .iter()
                    .map(|value| value.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let mut output = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                    .unwrap();
                let mut repeated = None;
                for sample in 0..3 {
                    let ((), requests, bytes) = allocation_probe::measured_with_bytes(|| {
                        for _ in 0..20 {
                            jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                                .unwrap();
                        }
                    });
                    let observed = (requests, bytes);
                    if let Some(expected) = counts {
                        zero_slope &= observed == expected;
                    } else {
                        counts = Some(observed);
                    }
                    if let Some(prior) = repeated {
                        assert_eq!(
                            observed, prior,
                            "repeated warm allocation observation must be stable"
                        );
                    } else {
                        repeated = Some(observed);
                    }
                    println!(
                        "{}",
                        serde_json::json!({"witness":"tagged-map-allocation-slope","form":form,"map_entries":entries,"steps":steps,"sample":sample,"evaluations":20,"allocation_requests":requests,"requested_bytes":bytes,"selected_enum_packs":packs})
                    );
                }
                assert!(
                    matches!(output.last().unwrap().as_ref().unwrap().data(), ValueData::U64(value) if *value == 26 + steps)
                );
                assert_eq!(
                    args.iter()
                        .map(|value| value.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(value, alias)| value.shares_frozen_storage(alias))
                );
                drop((output, args, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"witness":"tagged-map-allocation-summary", "expected_tagged":expected_tagged, "zero_per_transition_allocation_slope":zero_slope})
    );
    assert!(
        !expected_tagged || zero_slope,
        "selected tagged-map state still allocates per transition; retain this failed performance regression explicitly"
    );
}

fn main() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
