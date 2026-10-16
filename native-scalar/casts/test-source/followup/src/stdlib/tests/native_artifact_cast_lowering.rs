//! Genuine scalar conversion adapter metadata, without executing a kernel.
#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{
    BoundResidentKernel, ChangeDetectionPolicy, ExecutionTarget, FunctionCatalogBuilder,
    OutputConstruction, ResidentKernelBindError, ResidentKernelBindRequest,
    ResolvedOperationContract, ShapeRule,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeScalarKind, NativeScalarOp, lower_pure_native_artifact,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn compile(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x43415354), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    [original, decoded]
}
#[test]
fn six_actual_scalar_casts_keep_exact_contract_and_native_host_storage() {
    use NativeScalarKind::{F64, I64, U64};
    use NativeScalarOp::*;
    for (source, input, output, op) in [
        ("answer<i64> := value<u64>\nanswer\n", U64, I64, U64ToI64),
        ("answer<u64> := value<i64>\nanswer\n", I64, U64, I64ToU64),
        ("answer<f64> := value<u64>\nanswer\n", U64, F64, U64ToF64),
        ("answer<f64> := value<i64>\nanswer\n", I64, F64, I64ToF64),
        ("answer<u64> := value<f64>\nanswer\n", F64, U64, F64ToU64),
        ("answer<i64> := value<f64>\nanswer\n", F64, I64, F64ToI64),
    ] {
        for artifact in compile(source) {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.inputs().len(), 1);
            assert_eq!(plan.inputs()[0].ty().kind(), input);
            assert_eq!(plan.lets().len(), 1);
            assert_eq!(plan.lets()[0].op(), op);
            assert_eq!(plan.lets()[0].ty().kind(), output);
            let memory = plan.lets()[0].memory_plan();
            assert_eq!(memory.target.kind, mech_core::MemoryTargetKind::NativeHost);
            assert_eq!(memory.bound_call.target(), ExecutionTarget::Native);
            let storage = |kind| {
                mech_core::PlannedSlotKind::FixedScalar(match kind {
                    NativeScalarKind::U64 => {
                        mech_core::ScalarMemoryKind::Unsigned(mech_core::IntegerWidth::W64)
                    }
                    NativeScalarKind::I64 => {
                        mech_core::ScalarMemoryKind::Signed(mech_core::IntegerWidth::W64)
                    }
                    NativeScalarKind::F64 => {
                        mech_core::ScalarMemoryKind::Floating(mech_core::FloatWidth::W64)
                    }
                    _ => panic!(),
                })
            };
            assert_eq!(memory.input_storage[0].slot, storage(input));
            assert_eq!(memory.output_storage[0].slot, storage(output));
            assert!(memory.deferred_witnesses.is_empty());
            let ResolvedOperationContract::Declared(contract) = artifact
                .contracts()
                .get(artifact.nodes()[0].as_operation().unwrap().contract)
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(
                contract.outputs[0].construction,
                OutputConstruction::FullWrite {
                    shape: ShapeRule::SameAsInput { input: 0 }
                }
            );
            assert_eq!(
                contract.outputs[0].change_detection,
                ChangeDetectionPolicy::KernelReported
            );
            assert!(
                plan.outputs()
                    .iter()
                    .all(|value| value.ty().kind() == output)
            );
        }
    }
}
fn unreviewed(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    Ok(BoundResidentKernel::new(
        |_, _, _| panic!("no executor may run during metadata binding"),
        Box::new([]),
    )
    .with_work_accounting(mech_core::ResidentKernelWorkAccounting::RuntimeMetered))
}
#[test]
fn same_name_or_public_cost_policy_never_certifies_a_cast() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["convert"],
            "kind",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            unreviewed,
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    for artifact in compile("answer<f64> := value<u64>\nanswer\n") {
        assert!(matches!(
            lower_pure_native_artifact(&artifact, &catalog),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
    }
}
#[test]
fn other_cast_domains_and_computed_activation_stay_unsupported() {
    for source in [
        "answer<u64> := value<f32>\nanswer\n",
        "answer<[f64]> := value<[u64]:1,1>\nanswer\n",
        "answer<string> := value<u64>\nanswer\n",
        "~answer<u64> := 0u64\nanswer\n",
        "unused := 1u64/0u64\nanswer<f64> := value<u64>\nanswer\n",
    ] {
        for artifact in compile(source) {
            assert!(
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).is_err(),
                "{source}"
            );
        }
    }
}
