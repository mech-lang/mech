#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
//! Canonical loop metadata and selected-factory authority, not native execution.
use mech_core::{
    BoundResidentKernel, FunctionCatalogBuilder, ImplementationMemoryClass,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentValueMut,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeIterationAction, NativeIterationValueScope,
    NativeMatchAction, NativeScalarKind, NativeScalarOp, lower_scalar_iteration_artifact,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x49544c4f), Revision(1), source).unwrap(),
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
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    [original, decoded]
}

#[test]
fn loops_preserve_all_four_exact_scalar_kinds_and_constant_counts() {
    for (spelling, kind) in [
        ("bool", NativeScalarKind::Bool),
        ("u64", NativeScalarKind::U64),
        ("i64", NativeScalarKind::I64),
        ("f64", NativeScalarKind::F64),
    ] {
        for count in [0, 1, 25, u64::MAX] {
            let source = format!(
                "identity(x<{spelling}>) => (next<{spelling}>) := next := x.\nseed<{spelling}> |{count}> identity\n"
            );
            for artifact in artifacts(&source) {
                let plan =
                    lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog())
                        .unwrap();
                plan.validate_current_visits().unwrap();
                assert_eq!(plan.program_revision(), artifact.revision());
                assert_eq!(plan.iterations().len(), 1);
                let loop_ = &plan.iterations()[0];
                assert_eq!(loop_.count(), count);
                assert_eq!(plan.values()[loop_.carrier().get() as usize].kind(), kind);
                assert_eq!(
                    plan.value_scopes()[loop_.carrier().get() as usize],
                    NativeIterationValueScope::Carrier { iteration: 0 }
                );
                assert_ne!(loop_.carrier(), loop_.seed());
                for (actual, expected) in plan.outputs().iter().zip(artifact.outputs()) {
                    assert_eq!(actual.output(), expected.output);
                    assert_eq!(actual.slot(), expected.source);
                    assert_eq!(actual.name(), expected.name);
                    assert_eq!(actual.ty().schema(), expected.schema);
                    assert_eq!(
                        actual.interactive_binding(),
                        expected.interactive_binding.as_ref()
                    );
                }
            }
        }
    }
}

#[test]
fn nested_loops_share_cast_and_arithmetic_witnesses_without_evaluating_them() {
    let source = "step(x<u64>) => (next<u64>) := signed<i64> := x\nreal<f64> := signed\nagain<u64> := real\nnext := (choose? | true => x + 1u64 | false => again).\nouter(x<u64>) => (next<u64>) := next := x |3> step.\nchoose := flag<bool>\nseed<u64> |25> outer\n";
    for artifact in artifacts(source) {
        let plan =
            lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        plan.validate_current_visits().unwrap();
        assert_eq!(plan.iterations().len(), 2);
        assert_eq!(plan.iterations()[1].parent(), Some(0));
        let mut operations = Vec::new();
        for block in plan.blocks() {
            if let NativeIterationAction::Scalar(NativeMatchAction::Primitive {
                op,
                operands,
                output,
            }) = block.action()
            {
                operations.push(*op);
                // Compiler-time binding plans are not retained in this CFG.
                // Check the selected opcode's exact typed data flow instead.
                let (arity, input, result) = match op {
                    NativeScalarOp::U64ToI64 => (1, NativeScalarKind::U64, NativeScalarKind::I64),
                    NativeScalarOp::I64ToF64 => (1, NativeScalarKind::I64, NativeScalarKind::F64),
                    NativeScalarOp::F64ToU64 => (1, NativeScalarKind::F64, NativeScalarKind::U64),
                    NativeScalarOp::U64Add => (2, NativeScalarKind::U64, NativeScalarKind::U64),
                    other => panic!("unexpected selected operation: {other:?}"),
                };
                assert_eq!(operands.len(), arity);
                assert!(
                    operands
                        .iter()
                        .all(|arg| plan.values()[arg.get() as usize].kind() == input)
                );
                assert_eq!(plan.values()[output.get() as usize].kind(), result);
                assert!(block.origin().block().is_some());
            }
        }
        for expected in [
            NativeScalarOp::U64ToI64,
            NativeScalarOp::I64ToF64,
            NativeScalarOp::F64ToU64,
            NativeScalarOp::U64Add,
        ] {
            assert!(operations.contains(&expected), "{operations:?}");
        }
    }
}

#[test]
fn signed_and_floating_literal_patterns_keep_exact_constant_types_inside_loops() {
    for (source, kind) in [
        (
            "step(x<i64>) => (next<i64>) := next := (x? | 0i64 => 1i64 | * => x).\nseed<i64> |2> step\n",
            NativeScalarKind::I64,
        ),
        (
            "step(x<f64>) => (next<f64>) := next := (x? | 0.0 => 1.0 | * => x).\nseed<f64> |2> step\n",
            NativeScalarKind::F64,
        ),
    ] {
        for artifact in artifacts(source) {
            let plan =
                lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            plan.validate_current_visits().unwrap();
            assert!(
                plan.constants()
                    .iter()
                    .any(|constant| plan.values()[constant.value().get() as usize].kind() == kind)
            );
        }
    }
}

static BINDS: AtomicUsize = AtomicUsize::new(0);
fn never(
    _: &BoundResidentKernel,
    _: &dyn ResidentKernelInputs,
    _: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    panic!("native metadata lowering must not execute a resident kernel")
}
fn fake(_: &ResidentKernelBindRequest<'_>) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    BINDS.fetch_add(1, Ordering::SeqCst);
    Ok(BoundResidentKernel::new(never, Box::new([]))
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}

#[test]
fn familiar_names_and_public_work_tags_do_not_authorize_a_loop_operation() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["math"],
            "add",
            ImplementationMemoryClass::NoAdditionalScratch,
            fake,
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    BINDS.store(0, Ordering::SeqCst);
    for artifact in
        artifacts("step(x<u64>) => (next<u64>) := next := x + 1u64.\nseed<u64> |25> step\n")
    {
        assert!(matches!(
            lower_scalar_iteration_artifact(&artifact, &catalog),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 2);
}

#[test]
fn matrix_counts_and_lifecycle_are_not_silently_run_by_another_executor() {
    for source in [
        "step(x<u64>) => (next<u64>) := next := x + 1u64.\nseed<u64> |[2]> step\n",
        "step(x<u64>) => (next<u64>) := next := x + 1u64.\nseed<[u64]:1,2> |[2 3]> step\n",
        "~value := 1u64\nvalue\n",
        "#Deferred(value<u64>) => <u64>\n  | :Start\n  | :Later.\n#Deferred(value) -> :Start\n  :Start ~> :Later\n  :Later => value.\n#Deferred(signal<u64>)\n",
    ] {
        for artifact in artifacts(source) {
            assert!(
                lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).is_err(),
                "{source}"
            );
        }
    }
}
