#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::{
    BoundCall, BoundResidentKernel, ExecutionTarget, FunctionCatalogBuilder,
    ImplementationMemoryClass, IntegerWidth, MemorySpace, MemoryTargetKind, PlannedSlotKind,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentValueMut, ScalarMemoryKind,
};
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeScalarKind, NativeScalarOp, NativeScalarValue,
    NativeValueRef, lower_pure_native_artifact,
};
use mech_engine::{ArtifactSource, CanonicalSourceFrontend, ProducerReference, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn compile(source: &str) -> ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4e4154495645), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    CanonicalSourceFrontend
        .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .compile_artifact()
        .unwrap()
}
fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let original = compile(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    [original, decoded]
}

fn expected_reference(
    artifact: &ProgramArtifact,
    mut slot: mech_core::CellSlotId,
) -> NativeValueRef {
    for _ in 0..=artifact.slots().len() {
        match artifact.slots()[slot.get() as usize].producer {
            ProducerReference::Input(input) => return NativeValueRef::Input(input),
            ProducerReference::NodeOutput {
                node,
                output_ordinal: 0,
            } => return NativeValueRef::Local(node),
            ProducerReference::Output {
                source: ArtifactSource::Constant(constant),
                ..
            } => return NativeValueRef::Constant(constant),
            ProducerReference::Output {
                source: ArtifactSource::Slot(source),
                ..
            } => slot = source,
            _ => panic!("unexpected scalar output producer"),
        }
    }
    panic!("invalid output forwarding cycle")
}

#[test]
fn direct_constants_and_live_inputs_are_typed_without_observation() {
    for artifact in artifacts("true\n18446744073709551615u64\nflag<bool>\nvalue<u64>\n") {
        assert!(artifact.nodes().is_empty());
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.program_revision(), artifact.revision());
        assert_eq!(plan.inputs().len(), 2);
        assert_eq!(plan.outputs().len(), artifact.outputs().len());
        assert!(plan.lets().is_empty());
        assert!(
            plan.constants()
                .iter()
                .any(|value| value.value() == NativeScalarValue::Bool(true))
        );
        assert!(
            plan.constants()
                .iter()
                .any(|value| value.value() == NativeScalarValue::U64(u64::MAX))
        );
        for (input, declared) in plan.inputs().iter().zip(artifact.inputs()) {
            assert_eq!(input.input(), declared.input);
            assert_eq!(input.slot(), declared.slot);
            assert_eq!(input.ty().schema(), declared.schema);
        }
        for (output, declared) in plan.outputs().iter().zip(artifact.outputs()) {
            assert_eq!(output.output(), declared.output);
            assert_eq!(output.slot(), declared.source);
            assert_eq!(output.name(), declared.name);
            assert_eq!(
                output.interactive_binding(),
                declared.interactive_binding.as_ref()
            );
            assert_eq!(
                output.value(),
                expected_reference(&artifact, declared.source)
            );
            assert_eq!(output.ty().schema(), declared.schema);
        }
    }
}

#[test]
fn shared_graph_local_has_one_ordered_let_and_multiple_references() {
    for artifact in artifacts("sum := left<u64> + 1u64\nsum + sum\n") {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.lets().len(), artifact.nodes().len());
        assert_eq!(plan.lets().len(), 2);
        let first = &plan.lets()[0];
        let second = &plan.lets()[1];
        assert_eq!(first.op(), NativeScalarOp::U64Add);
        assert_eq!(second.op(), NativeScalarOp::U64Add);
        assert_eq!(second.operands(), &[NativeValueRef::Local(first.node()); 2]);
        assert_eq!(first.ty().kind(), NativeScalarKind::U64);
        assert_eq!(second.ty().kind(), NativeScalarKind::U64);
        assert_eq!(plan.outputs().len(), artifact.outputs().len());
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

#[test]
fn checked_dead_nodes_are_not_folded_executed_or_dropped() {
    // Either live value can be zero; the literal zero guarantees the divide
    // would fail if lowering tried to execute using any placeholder input.
    for artifact in artifacts("unused := left<u64> / 0u64\nright<u64> % 0u64\n42u64\n") {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog())
            .unwrap_or_else(|error| panic!("{error:?}; slots={:?}", artifact.slots()));
        assert_eq!(plan.lets().len(), artifact.nodes().len());
        assert_eq!(
            plan.lets().iter().map(|node| node.op()).collect::<Vec<_>>(),
            [NativeScalarOp::U64Div, NativeScalarOp::U64Rem]
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

#[test]
fn actual_comparison_preserves_bool_result_and_u64_operands() {
    for artifact in artifacts("left<u64> < right<u64>\n") {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.lets().len(), 1);
        assert_eq!(plan.lets()[0].op(), NativeScalarOp::U64Lt);
        assert_eq!(plan.lets()[0].ty().kind(), NativeScalarKind::Bool);
        assert!(
            plan.inputs()
                .iter()
                .all(|input| input.ty().kind() == NativeScalarKind::U64)
        );
    }
}

#[test]
fn ordinary_bool_operations_have_certified_kinds_and_native_bool_slots() {
    for artifact in artifacts("!left<bool>\nleft<bool> == right<bool>\n") {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            plan.lets().iter().map(|node| node.op()).collect::<Vec<_>>(),
            [NativeScalarOp::BoolNot, NativeScalarOp::BoolEq]
        );
        for node in plan.lets() {
            assert_eq!(node.ty().kind(), NativeScalarKind::Bool);
            for storage in node
                .memory_plan()
                .input_storage
                .iter()
                .chain(node.memory_plan().output_storage.iter())
            {
                assert_eq!(
                    storage.slot,
                    PlannedSlotKind::FixedScalar(ScalarMemoryKind::Bool)
                );
                assert_eq!(storage.space, MemorySpace::Host);
            }
        }
    }
}

#[test]
fn native_host_plans_have_real_scalar_storage_and_resident_constructor_stays_resident() {
    let artifact = compile("left<u64> + right<u64>\n");
    let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let call = plan.lets()[0].memory_plan();
    assert_eq!(call.target.kind, MemoryTargetKind::NativeHost);
    assert_eq!(call.bound_call.target(), ExecutionTarget::Native);
    for storage in call.input_storage.iter().chain(call.output_storage.iter()) {
        assert_eq!(storage.space, MemorySpace::Host);
        assert_eq!(
            storage.slot,
            PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64))
        );
    }
    assert!(call.deferred_witnesses.is_empty());
    let bound = &call.bound_call;
    let resident = BoundCall::artifact_operation(
        bound.operation_descriptor().clone(),
        bound.inputs().into(),
        bound.outputs().into(),
        bound.resident_operation().unwrap().clone(),
    )
    .unwrap();
    assert_eq!(resident.target(), ExecutionTarget::ResidentCpu);
    assert_eq!(resident.inputs(), bound.inputs());
    assert!(
        BoundCall::artifact_operation_for_target(
            bound.operation_descriptor().clone(),
            bound.inputs().into(),
            bound.outputs().into(),
            bound.resident_operation().unwrap().clone(),
            ExecutionTarget::DirectRuntime,
        )
        .is_err()
    );
}

#[test]
fn unsupported_lifecycle_control_and_kinds_fail_closed_original_and_decoded() {
    for source in [
        "1u64 / 0u64\n",
        "flag<bool> ? | true => left<u64> | false => right<u64>\n",
        "left<f64> + right<f64>\n",
        "text<string>\n",
        "values<[u64]:1,2>\n",
        "~state := 0u64\nstate\n",
    ] {
        for artifact in artifacts(source) {
            assert!(
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).is_err(),
                "{source}"
            );
        }
    }
    let artifact = compile("1u64 / 0u64\n");
    assert!(matches!(
        lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()),
        Err(NativeArtifactLoweringError::UnsupportedNode {
            feature: "Activation computation",
            ..
        })
    ));
}

static CUSTOM_BINDS: AtomicUsize = AtomicUsize::new(0);
fn forbidden_executor(
    _: &BoundResidentKernel,
    _: &dyn ResidentKernelInputs,
    _: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    panic!("metadata lowering must never execute the selected kernel")
}
fn same_name_custom_factory(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    CUSTOM_BINDS.fetch_add(1, Ordering::SeqCst);
    Ok(BoundResidentKernel::new(forbidden_executor, Box::new([]))
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}
#[test]
fn same_name_contract_and_public_work_policy_do_not_create_semantic_authority() {
    CUSTOM_BINDS.store(0, Ordering::SeqCst);
    let mut catalog = FunctionCatalogBuilder::new();
    catalog
        .insert_resident_factory(
            ["math"],
            "add",
            ImplementationMemoryClass::NoAdditionalScratch,
            same_name_custom_factory,
        )
        .unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in artifacts("left<u64> + right<u64>\n") {
        let node = artifact.nodes()[0].node;
        assert_eq!(
            lower_pure_native_artifact(&artifact, &catalog).unwrap_err(),
            NativeArtifactLoweringError::MissingSemanticWitness { node }
        );
    }
    assert_eq!(CUSTOM_BINDS.load(Ordering::SeqCst), 2);
}
