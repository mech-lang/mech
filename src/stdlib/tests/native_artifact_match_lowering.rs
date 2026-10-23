#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
//! Metadata-only canonical Match lowering and selected implementation authority.
use mech_core::{
    BoundResidentKernel, FunctionCatalogBuilder, ImplementationMemoryClass,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentValueMut,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeMatchAction, NativeMatchTerminator, NativeScalarKind,
    NativeScalarOp, lower_scalar_match_artifact,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::atomic::{AtomicUsize, Ordering};
fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d41544d), Revision(1), source).unwrap(),
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
    [original, decoded]
}
#[test]
fn lexical_captures_alias_exact_scalar_values_and_keep_all_certified_operations() {
    let source = "captured := offset<u64> + 1u64\n(flag<bool>? | true => captured + value<u64> | false => (value<u64>? | 0u64 => captured | * => captured + 2u64))\n";
    for artifact in artifacts(source) {
        let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.program_revision(), artifact.revision());
        assert_eq!(plan.outputs().len(), artifact.outputs().len());
        let ops = plan
            .blocks()
            .iter()
            .filter_map(|block| match block.action() {
                NativeMatchAction::Primitive {
                    op,
                    operands,
                    output,
                    memory_plan,
                } => Some((block.origin(), *op, operands, *output, memory_plan)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(ops.len(), 3);
        assert!(
            ops.iter()
                .all(|(_, op, _, _, _)| *op == NativeScalarOp::U64Add)
        );
        let captured = ops
            .iter()
            .find(|(at, _, _, _, _)| at.block().is_none())
            .unwrap()
            .3;
        assert_eq!(
            ops.iter()
                .filter(|(at, _, args, _, _)| at.block().is_some() && args.contains(&captured))
                .count(),
            2
        );
        for (_, _, _, _, memory) in ops {
            assert_eq!(memory.target.kind, mech_core::MemoryTargetKind::NativeHost);
            assert_eq!(
                memory.bound_call.target(),
                mech_core::ExecutionTarget::Native
            );
        }
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
        assert!(
            plan.values()
                .iter()
                .all(|ty| matches!(ty.kind(), NativeScalarKind::Bool | NativeScalarKind::U64))
        );
        assert!(plan.blocks().iter().any(|block| matches!(
            block.terminator(),
            NativeMatchTerminator::LiteralBranch { .. }
        )));
    }
}
static BINDS: AtomicUsize = AtomicUsize::new(0);
fn never(
    _: &BoundResidentKernel,
    _: &dyn ResidentKernelInputs,
    _: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    panic!("metadata must not execute a kernel")
}
fn fake(_: &ResidentKernelBindRequest<'_>) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    BINDS.fetch_add(1, Ordering::SeqCst);
    Ok(BoundResidentKernel::new(never, Box::new([]))
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}
#[test]
fn public_work_tags_and_same_operation_names_do_not_authorize_a_match_primitive() {
    BINDS.store(0, Ordering::SeqCst);
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
    for artifact in artifacts("(flag<bool>? | true => value<u64> + 1u64 | false => 0u64)\n") {
        assert!(matches!(
            lower_scalar_match_artifact(&artifact, &catalog),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
    }
    assert_eq!(BINDS.load(Ordering::SeqCst), 2);
}
#[test]
fn partial_match_and_lifecycle_or_aggregate_programs_are_rejected_whole() {
    for source in [
        "only(n<u64>) => <u64>\n  | 0u64 => 42u64.\nonly(signal<u64>)\n",
        "~value := 1u64\nvalue\n",
        "signal<[f64]:1,2>\n",
        "[value + 1u64 | value <- signal<[u64]:1,2>]\n",
        "#Deferred(value<u64>) => <u64>\n  | :Start\n  | :Later.\n#Deferred(value) -> :Start\n  :Start ~> :Later\n  :Later => value.\n#Deferred(signal<u64>)\n",
    ] {
        for artifact in artifacts(source) {
            assert!(
                lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).is_err(),
                "{source}"
            );
        }
    }
}

#[test]
fn wider_scalar_matches_share_the_selected_cast_witness_and_exact_types() {
    for (source, op, kind) in [
        (
            "result<i64> := value<u64>\n(flag<bool>? | true => result | false => result)\n",
            NativeScalarOp::U64ToI64,
            NativeScalarKind::I64,
        ),
        (
            "result<u64> := value<i64>\n(flag<bool>? | true => result | false => result)\n",
            NativeScalarOp::I64ToU64,
            NativeScalarKind::U64,
        ),
        (
            "result<f64> := value<u64>\n(flag<bool>? | true => result | false => result)\n",
            NativeScalarOp::U64ToF64,
            NativeScalarKind::F64,
        ),
        (
            "result<f64> := value<i64>\n(flag<bool>? | true => result | false => result)\n",
            NativeScalarOp::I64ToF64,
            NativeScalarKind::F64,
        ),
        (
            "result<u64> := value<f64>\n(flag<bool>? | true => result | false => result)\n",
            NativeScalarOp::F64ToU64,
            NativeScalarKind::U64,
        ),
        (
            "result<i64> := value<f64>\n(flag<bool>? | true => result | false => result)\n",
            NativeScalarOp::F64ToI64,
            NativeScalarKind::I64,
        ),
    ] {
        for artifact in artifacts(source) {
            let plan =
                lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(plan.blocks().iter().any(|block| matches!(block.action(),
                NativeMatchAction::Primitive { op: actual, memory_plan, .. }
                    if *actual == op && memory_plan.bound_call.target() == mech_core::ExecutionTarget::Native)));
            assert!(
                plan.outputs()
                    .iter()
                    .any(|output| output.ty().kind() == kind)
            );
        }
    }
    for artifact in artifacts("(value<f64>? | 0.0 => 7.0 | found => found)\n") {
        let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(
            plan.blocks()
                .iter()
                .any(|block| matches!(block.terminator(),
            NativeMatchTerminator::LiteralBranch { left, right, .. }
                if plan.values()[left.get() as usize].kind() == NativeScalarKind::F64
                    && plan.values()[left.get() as usize] == plan.values()[right.get() as usize]))
        );
    }
}

#[test]
fn counted_application_is_rejected_whole_by_the_match_only_plan() {
    let source = "inc(x<u64>) => <u64> := x + 1u64\nseed<u64> |[2]> inc\n";
    for artifact in artifacts(source) {
        assert!(
            artifact
                .nodes()
                .iter()
                .any(|node| matches!(&node.body, mech_engine::ExecutableNodeBody::Iteration(_)))
        );
        assert!(lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).is_err());
    }
}
