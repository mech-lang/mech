//! Metadata-only dense F64 lowering requires actual selected scalar semantics.
#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{
    BoundResidentKernel, ExecutionTarget, FloatWidth, FunctionCatalogBuilder,
    ImplementationMemoryClass, MemorySpace, MemoryTargetKind, PlannedSlotKind,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentValueMut, ScalarMemoryKind,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeScalarKind, NativeScalarOp, NativeScalarValue,
    lower_pure_native_artifact,
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
fn dense_f64_scalar_ops_have_exact_native_host_memory_and_bool_results() {
    use NativeScalarOp::*;
    for (source, op, kind) in [
        ("left<f64> + right<f64>\n", F64Add, NativeScalarKind::F64),
        ("left<f64> - right<f64>\n", F64Sub, NativeScalarKind::F64),
        ("left<f64> * right<f64>\n", F64Mul, NativeScalarKind::F64),
        ("left<f64> / right<f64>\n", F64Div, NativeScalarKind::F64),
        ("-left<f64>\n", F64Neg, NativeScalarKind::F64),
        (
            "+> math\nmath/sqrt(left<f64>)\n",
            F64Sqrt,
            NativeScalarKind::F64,
        ),
        ("left<f64> == right<f64>\n", F64Eq, NativeScalarKind::Bool),
        ("left<f64> != right<f64>\n", F64Ne, NativeScalarKind::Bool),
        ("left<f64> < right<f64>\n", F64Lt, NativeScalarKind::Bool),
        ("left<f64> <= right<f64>\n", F64Le, NativeScalarKind::Bool),
        ("left<f64> > right<f64>\n", F64Gt, NativeScalarKind::Bool),
        ("left<f64> >= right<f64>\n", F64Ge, NativeScalarKind::Bool),
    ] {
        for artifact in artifacts(source) {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.lets().len(), 1);
            let node = &plan.lets()[0];
            assert_eq!(node.op(), op);
            assert_eq!(node.ty().kind(), kind);
            assert!(
                plan.inputs()
                    .iter()
                    .all(|input| input.ty().kind() == NativeScalarKind::F64)
            );
            let call = node.memory_plan();
            assert_eq!(call.target.kind, MemoryTargetKind::NativeHost);
            assert_eq!(call.bound_call.target(), ExecutionTarget::Native);
            assert!(call.deferred_witnesses.is_empty());
            for storage in call.input_storage.iter() {
                assert_eq!(storage.space, MemorySpace::Host);
                assert_eq!(
                    storage.slot,
                    PlannedSlotKind::FixedScalar(ScalarMemoryKind::Floating(FloatWidth::W64))
                );
            }
            assert_eq!(
                call.output_storage[0].slot,
                PlannedSlotKind::FixedScalar(if kind == NativeScalarKind::Bool {
                    ScalarMemoryKind::Bool
                } else {
                    ScalarMemoryKind::Floating(FloatWidth::W64)
                })
            );
        }
    }
}

#[test]
fn f64_direct_constants_and_dead_ieee_nodes_remain_exact_and_ordered() {
    for artifact in artifacts("1.5\nvalue<f64>\n") {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(plan.lets().is_empty());
        assert!(plan.constants().iter().any(|constant| constant.value()
            == NativeScalarValue::F64(mech_core::snapshot::F64Bits::from_bits(
                0x3ff8000000000000
            ))));
        assert_eq!(plan.outputs().len(), artifact.outputs().len());
        assert!(
            plan.outputs()
                .iter()
                .all(|output| output.ty().kind() == NativeScalarKind::F64)
        );
    }
    for artifact in artifacts("+> math\nunused := left<f64> / 0.0\nmath/sqrt(right<f64>)\n") {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            plan.lets().iter().map(|node| node.op()).collect::<Vec<_>>(),
            [NativeScalarOp::F64Div, NativeScalarOp::F64Sqrt]
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
    panic!("metadata lowering never executes an implementation")
}
fn replacement(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    CUSTOM_BINDS.fetch_add(1, Ordering::SeqCst);
    Ok(BoundResidentKernel::new(forbidden_executor, Box::new([]))
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}

#[test]
fn f64_unsupported_work_and_lifecycle_or_replacement_impls_remain_rejected() {
    for source in [
        "left<f64> % right<f64>\n",
        "left<f64> ^ right<f64>\n",
        "left<f32> + right<f32>\n",
        "left<[f64]:1,1>\n",
        "1.0 / 0.0\n",
        "~state := 0.0\nstate\n",
        "flag<bool>? | true => left<f64> | false => right<f64>\n",
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
            "sqrt",
            ImplementationMemoryClass::NoAdditionalScratch,
            replacement,
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    CUSTOM_BINDS.store(0, Ordering::SeqCst);
    for artifact in artifacts("+> math\nmath/sqrt(value<f64>)\n") {
        assert_eq!(
            lower_pure_native_artifact(&artifact, &catalog).unwrap_err(),
            NativeArtifactLoweringError::MissingSemanticWitness {
                node: artifact.nodes()[0].node
            }
        );
    }
    assert_eq!(CUSTOM_BINDS.load(Ordering::SeqCst), 2);
}
