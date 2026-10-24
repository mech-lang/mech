//! Real uncached map access must use its finite call policy during finalization.
use super::*;
use super::super::{bind_native_collection_operation, native_collection_operation_template};
use super::data_footprint_tests::admitted;
use crate::{CanonicalSourceFrontend, ProgramArtifact};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft, ValueDraft};
use mech_core::snapshot::{MapEntryDraft, SnapshotValidationContext};
use mech_syntax::document::{AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision,
    TextSnapshot, parse_canonical_document};

fn variants() -> [ProgramArtifact; 2] {
    let source = "counts := input<{string:u64}>\ncounts{query<string>}\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x414c4c4f), Revision(0), source).unwrap(), ParseConfig::default());
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let artifact = CanonicalSourceFrontend.compile_document(
        &DocumentSyntax::cast(parsed.syntax()).unwrap()).unwrap().compile_artifact().unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap()).unwrap();
    [artifact, decoded]
}

fn fixture(artifact: &ProgramArtifact, memory: &ManagedMemoryBudget)
    -> (NativeCollectionCall, [Option<Value>; 2])
{
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    let template = artifact.nodes().iter().find_map(|node|
        native_collection_operation_template(artifact, node.node, &catalog).ok()
            .filter(|template| template.primitive()
                == crate::resident::native_collection_semantics::NativeCollectionPrimitive::MapKeyAccess))
        .unwrap();
    let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
        panic!("declared map access")
    };
    // One real key suffices to exceed the default comparison profile while
    // keeping this regression small. No missing-key or synthetic cost path.
    let key = "x".repeat(mech_core::RESIDENT_MAX_COMPARISON_WORK as usize + 1);
    let source = ValueDraft { schema: contract.inputs[0].schema, shape_values: Box::new([]),
        data: ValueDataDraft::Map(vec![MapEntryDraft {
            items: vec![ValueDataDraft::String(key.clone().into()), ValueDataDraft::U64(37)]
                .into_boxed_slice(),
        }].into_boxed_slice()),
    }.finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
    let selector = ValueDraft { schema: contract.inputs[1].schema, shape_values: Box::new([]),
        data: ValueDataDraft::String(key.into()),
    }.finalize(&SnapshotValidationContext::new(artifact.schemas())).unwrap();
    let inputs = [Some(admitted(source, memory)), Some(admitted(selector, memory))];
    let parameters = inputs.iter().map(|value| value.as_ref().unwrap().shape().parameter_values())
        .collect::<Vec<_>>();
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu().unwrap().limits;
    limits.max_compute_work = Some(128 * 1024 * 1024);
    limits.max_comparison_work = Some(4 * 1024 * 1024);
    limits.max_temporary_bytes = Some(128 * 1024 * 1024);
    let call = bind_native_collection_operation(artifact, template.node(), &catalog,
        &parameters, &[], limits).unwrap();
    (call, inputs)
}

#[test]
fn uncached_map_lookup_finalization_obeys_actual_finite_allowance_and_root() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let (call, inputs) = fixture(&artifact, &memory);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 10_000_000, &memory).unwrap();
        let mut state = NativePlanCacheState::Uncached;
        let mut output = None;
        call.invoke_cached(&mut state, &inputs, &mut output, &work).unwrap();
        assert!(matches!(state, NativePlanCacheState::Uncached));
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(37)));
        let successful = work.used();
        assert!(successful.1 > mech_core::RESIDENT_MAX_COMPARISON_WORK);

        let prior = output.as_ref().unwrap().clone();
        let mut denied = call.clone();
        denied.memory.target.limits.max_comparison_work = Some(mech_core::RESIDENT_MAX_COMPARISON_WORK);
        let error = denied.invoke_cached(&mut state, &inputs, &mut output, &work).unwrap_err();
        assert!(matches!(error, NativeCollectionInvocationError::Kernel(ResidentKernelError::ResourceLimit {
            dimension: mech_core::MemoryBudgetDimension::ComparisonWork, ..
        }) | NativeCollectionInvocationError::Plan(MemoryPlanError::TargetLimitExceeded {
            violation: mech_core::MemoryBudgetViolation {
                dimension: mech_core::MemoryBudgetDimension::ComparisonWork, ..
            },
        })), "{error:?}");
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert!(work.used().0 >= successful.0 && work.used().1 >= successful.1);

        // A generous call cannot renew a finite whole-invocation allowance.
        let exact = NativeLibraryWorkBudget::new(successful.0, successful.1, &memory).unwrap();
        let mut exact_output = None;
        call.invoke_cached(&mut state, &inputs, &mut exact_output, &exact).unwrap();
        assert_eq!(exact.used(), successful);
        let exact_prior = exact_output.as_ref().unwrap().clone();
        assert!(call.invoke_cached(&mut state, &inputs, &mut exact_output, &exact).is_err());
        assert_eq!(exact.used(), successful);
        assert!(exact_output.as_ref().unwrap().shares_frozen_storage(&exact_prior));
        drop((exact_prior, exact_output, exact, prior, output, state, denied, call, inputs, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
