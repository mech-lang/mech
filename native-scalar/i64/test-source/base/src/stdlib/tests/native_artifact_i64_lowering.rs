//! Metadata-only signed scalar lowering uses the selected private binder proof.
#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{
    BoundResidentKernel, ExecutionTarget, FunctionCatalogBuilder, ImplementationMemoryClass,
    IntegerWidth, MemorySpace, MemoryTargetKind, PlannedSlotKind, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentValueMut, ScalarMemoryKind,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeScalarKind, NativeScalarOp, NativeScalarValue,
    NativeValueRef, lower_pure_native_artifact,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x494e4154495645), Revision(1), source).unwrap(),
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
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .compile_artifact()
        .unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        encoded,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [original, decoded]
}

#[test]
fn all_signed_scalar_binders_have_exact_typed_native_host_plans() {
    use NativeScalarOp::*;
    for (source, expected, result) in [
        ("left<i64> + right<i64>\n", I64Add, NativeScalarKind::I64),
        ("left<i64> - right<i64>\n", I64Sub, NativeScalarKind::I64),
        ("left<i64> * right<i64>\n", I64Mul, NativeScalarKind::I64),
        ("left<i64> / right<i64>\n", I64Div, NativeScalarKind::I64),
        ("left<i64> % right<i64>\n", I64Rem, NativeScalarKind::I64),
        ("-left<i64>\n", I64Neg, NativeScalarKind::I64),
        ("left<i64> == right<i64>\n", I64Eq, NativeScalarKind::Bool),
        ("left<i64> != right<i64>\n", I64Ne, NativeScalarKind::Bool),
        ("left<i64> < right<i64>\n", I64Lt, NativeScalarKind::Bool),
        ("left<i64> <= right<i64>\n", I64Le, NativeScalarKind::Bool),
        ("left<i64> > right<i64>\n", I64Gt, NativeScalarKind::Bool),
        ("left<i64> >= right<i64>\n", I64Ge, NativeScalarKind::Bool),
    ] {
        for artifact in artifacts(source) {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.lets().len(), 1);
            let node = &plan.lets()[0];
            assert_eq!(node.op(), expected);
            assert_eq!(node.ty().kind(), result);
            assert!(
                plan.inputs()
                    .iter()
                    .all(|input| input.ty().kind() == NativeScalarKind::I64)
            );
            assert_eq!(
                node.operands().len(),
                if expected == I64Neg { 1 } else { 2 }
            );
            let call = node.memory_plan();
            assert_eq!(call.target.kind, MemoryTargetKind::NativeHost);
            assert_eq!(call.bound_call.target(), ExecutionTarget::Native);
            assert!(call.deferred_witnesses.is_empty());
            for storage in call.input_storage.iter() {
                assert_eq!(storage.space, MemorySpace::Host);
                assert_eq!(
                    storage.slot,
                    PlannedSlotKind::FixedScalar(ScalarMemoryKind::Signed(IntegerWidth::W64))
                );
            }
            let output = &call.output_storage[0];
            assert_eq!(output.space, MemorySpace::Host);
            assert_eq!(
                output.slot,
                PlannedSlotKind::FixedScalar(if result == NativeScalarKind::Bool {
                    ScalarMemoryKind::Bool
                } else {
                    ScalarMemoryKind::Signed(IntegerWidth::W64)
                })
            );
        }
    }
}

#[test]
fn signed_constants_aliases_and_dead_checked_nodes_keep_their_artifact_identity() {
    for artifact in artifacts("value := 9223372036854775807i64\nvalue\nlive<i64>\n") {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(plan.lets().is_empty());
        assert!(
            plan.constants()
                .iter()
                .any(|constant| constant.value() == NativeScalarValue::I64(i64::MAX))
        );
        assert!(
            plan.inputs()
                .iter()
                .all(|input| input.ty().kind() == NativeScalarKind::I64)
        );
        assert_eq!(plan.outputs().len(), artifact.outputs().len());
        for output in plan.outputs() {
            assert_eq!(output.ty().kind(), NativeScalarKind::I64);
            assert!(matches!(
                output.value(),
                NativeValueRef::Constant(_) | NativeValueRef::Input(_)
            ));
        }
    }
    for artifact in artifacts("unused := left<i64> / 0i64\n-right<i64>\n42i64\n") {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.lets().len(), artifact.nodes().len());
        assert_eq!(
            plan.lets().iter().map(|node| node.op()).collect::<Vec<_>>(),
            [NativeScalarOp::I64Div, NativeScalarOp::I64Neg]
        );
        assert_eq!(
            plan.lets()
                .iter()
                .map(|node| node.node())
                .collect::<Vec<_>>(),
            artifact
                .nodes()
                .iter()
                .map(|node| node.node)
                .collect::<Vec<_>>()
        );
    }
}

static CUSTOM_BINDS: AtomicUsize = AtomicUsize::new(0);
fn forbidden_executor(
    _: &BoundResidentKernel,
    _: &dyn ResidentKernelInputs,
    _: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    panic!("metadata lowering must never execute a selected kernel")
}
fn replacement_factory(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    CUSTOM_BINDS.fetch_add(1, Ordering::SeqCst);
    Ok(BoundResidentKernel::new(forbidden_executor, Box::new([]))
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}

#[test]
fn signed_extension_preserves_scope_kind_and_selected_implementation_boundaries() {
    for source in [
        "1i64 / 0i64\n",
        "~state := 0i64\nstate\n",
        "left<i32> + right<i32>\n",
        "left<f64> + right<f64>\n",
        "values<[i64]:1,1>\n",
        "flag<bool>? | true => left<i64> | false => right<i64>\n",
    ] {
        for artifact in artifacts(source) {
            assert!(
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).is_err(),
                "{source}"
            );
        }
    }
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["math"],
            "add",
            ImplementationMemoryClass::NoAdditionalScratch,
            replacement_factory,
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    CUSTOM_BINDS.store(0, Ordering::SeqCst);
    for artifact in artifacts("left<i64> + right<i64>\n") {
        assert_eq!(
            lower_pure_native_artifact(&artifact, &catalog).unwrap_err(),
            NativeArtifactLoweringError::MissingSemanticWitness {
                node: artifact.nodes()[0].node
            }
        );
    }
    assert_eq!(CUSTOM_BINDS.load(Ordering::SeqCst), 2);
}
