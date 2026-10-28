use mech_core::{ManagedMemoryBudget, Value, ValueDataDraft, snapshot::{SnapshotValidationContext, ValueDraft}};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_gpu::native_artifact::{NativeExecutionLimits,NativeProgramExecutionLimits};
use mech_syntax::document::{AstNode,DocumentId,DocumentSyntax,ParseConfig,Revision,TextSnapshot,parse_canonical_document};
#[path = "/private/tmp/mech-deflate-reality-check/src/core/tests/support/allocation_probe.rs"]
pub mod allocation_probe;
#[global_allocator]
static ALLOCATOR:allocation_probe::ProbeAllocator=allocation_probe::ProbeAllocator;
fn artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
    artifacts_mode(source, false)
}
fn artifacts_mode(source: &str, interactive: bool) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x50524f47), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = if interactive {
        CanonicalSourceFrontend
            .compile_interactive_document_with_catalog(&syntax, mech_stdlib::source_catalog())
    } else {
        CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
    }
    .unwrap();
    let last = product
        .document_outputs()
        .iter()
        .rev()
        .find(|out| out.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = product.compile_artifact().unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    ([original, decoded], last)
}
fn limits() -> NativeProgramExecutionLimits {
    let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    // Explicit finite qualification policy, NOT a default-budget/performance claim.
    library.max_compute_work = Some(134_217_728);
    library.max_comparison_work = Some(1_000_000);
    library.max_temporary_bytes = Some(128 * 1024 * 1024);
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000,
            ..NativeExecutionLimits::default()
        },
        library,
    }
}
fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, ValueDataDraft)],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = named
                .iter()
                .find(|(key, _)| *key == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .1
                .clone();
            let shape_values = match &data {
                ValueDataDraft::Matrix(_)
                    if artifact
                        .schemas()
                        .get(input.schema)
                        .unwrap()
                        .dimension_parameters()
                        .is_empty() =>
                {
                    Box::new([])
                }
                ValueDataDraft::Matrix(values) => mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[1, values.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
                _ => Box::new([]),
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

mod measurement;
fn main(){measurement::measure();}
