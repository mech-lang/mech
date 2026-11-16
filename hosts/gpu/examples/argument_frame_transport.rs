//! Allocation diagnostic for ordinary checked function argument transport.
//! Input creation, compilation, and result inspection are outside each probe.
use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{ManagedMemoryBudget, Value, ValueDataDraft as Data, ValueDraft};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionLimits,
};
use mech_syntax::document::AstNode;
use mech_syntax::document::{
    DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot, parse_canonical_document,
};

#[path = "../../../src/core/tests/support/allocation_probe.rs"]
pub mod allocation_probe;

#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

fn artifact(checked: bool) -> (ProgramArtifact, usize) {
    let check = if checked {
        "  accepted! := allowed\n"
    } else {
        ""
    };
    let source = format!(
        "+> sequence\nsize(table<[u16]>, cache<[u32]>, allowed<bool>) => (out<u64>) :=\n{check}  out := sequence/length(table) + sequence/length(cache) + (allowed? | true => 0u64 | false => 1u64).\nsize(table<[u16]>, cache<[u32]>, allowed<bool>)\n"
    );
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4652414d45), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
        .unwrap();
    let output = product
        .document_outputs()
        .iter()
        .rev()
        .find(|out| out.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    (product.compile_artifact().unwrap(), output)
}

fn args(
    artifact: &ProgramArtifact,
    table: usize,
    cache: usize,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (data, count) = match name.as_str() {
                "table" => (
                    Data::Matrix((0..table).map(|i| Data::U16(i as u16)).collect()),
                    Some(table),
                ),
                "cache" => (
                    Data::Matrix((0..cache).map(|i| Data::U32(i as u32)).collect()),
                    Some(cache),
                ),
                "allowed" => (Data::Bool(true), None),
                _ => panic!("unexpected input {name}"),
            };
            let shape_values = count.map_or_else(
                || Box::new([]) as Box<[u64]>,
                |count| {
                    mech_core::shape_for_resolved_extents(
                        artifact.schemas().get(input.schema).unwrap(),
                        &[1, count as u64],
                    )
                    .unwrap()
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice()
                },
            );
            let value = ValueDraft {
                schema: input.schema,
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

fn policy() -> NativeProgramExecutionLimits {
    let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    // Explicit diagnostic policy, matching the finite component work/temporary limits.
    // It declares the real cache extent; no production default is changed.
    library.max_output_elements = Some(1_500_513);
    library.max_compute_work = Some(10_000_000_000);
    library.max_comparison_work = Some(10_000_000_000);
    library.max_temporary_bytes = Some(128 * 1024 * 1024);
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 10_000_000_000,
            ..NativeExecutionLimits::default()
        },
        library,
    }
}

fn main() {
    let mut records = Vec::new();
    for (table, cache) in [(16, 31), (262_144, 1_500_513)] {
        for checked in [false, true] {
            let (original, selected) = artifact(checked);
            let decoded = mech_engine::decode_program_artifact_bytecode_v1(
                &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
            )
            .unwrap();
            for (form, artifact) in [("original", original), ("decoded", decoded)] {
                let jit =
                    JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let inputs = args(&artifact, table, cache, &memory);
                let aliases = inputs.clone();
                let mut outputs = vec![None; artifact.outputs().len()];
                for visit in 0..2 {
                    let retained_before = memory.used_bytes();
                    let (result, allocations, allocated_bytes) =
                        allocation_probe::measured_with_bytes(|| {
                            jit.evaluate_into(&inputs, &mut outputs, policy(), &memory)
                        });
                    if result.is_ok() {
                        assert_eq!(
                            outputs[selected]
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64((table + cache) as u64)
                        );
                    }
                    assert!(
                        inputs
                            .iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                    records.push(serde_json::json!({
                        "table_elements":table,"cache_elements":cache,"input_payload_bytes":table*2+cache*4,
                        "checked":checked,"artifact":form,"visit":visit,
                        "allocations":allocations,"requested_allocation_bytes":allocated_bytes,
                        "retained_before":retained_before,"retained_after":memory.used_bytes(),
                        "result":format!("{result:?}"),"success":result.is_ok(),
                    }));
                }
                drop((outputs, aliases, inputs));
                assert_eq!(memory.used_bytes(), 0, "diagnostic leaked retained memory");
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "scope":"ordinary argument/frame allocation diagnostic; no compression timing",
            "policy":format!("{:?}",policy()),"records":records,
        }))
        .unwrap()
    );
}
