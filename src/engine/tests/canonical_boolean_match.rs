#![cfg(feature = "source")]

use mech_core::{BindingId, CellSlotId, ConstantId, InputId, NodeId, OutputId, SchemaBody};
use mech_engine::*;
use mech_syntax::document::parser::{canonical::parse_canonical_phase_2i_rule_for_test, rules};
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

fn find(node: SyntaxNode) -> Option<ExpressionSyntax> {
    if node.kind() == SyntaxKind::Expression {
        return ExpressionSyntax::cast(node);
    }
    node.children().find_map(find)
}

fn compile(source: &str) -> CanonicalSourceProgram {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x4d41544348), Revision(1), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source}");
    assert_eq!(parsed.consumed.end.0 as usize, source.len());
    CanonicalSourceFrontend
        .compile_expression(&find(parsed.syntax()).unwrap())
        .unwrap()
}

fn fixture() -> ProgramArtifactDraft {
    let base = compile("(flag<bool>, -11,22)").compile_artifact().unwrap();
    let boolean = base.inputs()[0].schema;
    let scalar = (0..base.schemas().len())
        .map(|id| mech_core::SchemaId::new(id as u32))
        .find(|id| {
            matches!(
                base.schemas().get(*id).unwrap().body(),
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
            )
        })
        .unwrap();
    let constants = (0..base.constants().len())
        .map(|id| ConstantId::new(id as u32))
        .filter(|id| base.constants().get(*id).unwrap().schema() == scalar)
        .collect::<Vec<_>>();
    assert_eq!(constants.len(), 2);
    let block = |id, constant| ControlBlock {
        id: ControlBlockId(id),
        parameters: Box::new([]),
        operations: Box::new([]),
        yield_value: ControlValue::Constant(constant),
    };
    ProgramArtifactDraft {
        schemas: base.schemas().clone(),
        constants: base.constants().clone(),
        contracts: base.contracts().clone(),
        requirements: ApplicationRequirementTable::default(),
        inputs: vec![InputDeclaration {
            input: InputId(0),
            name: "flag".into(),
            slot: CellSlotId(0),
            schema: boolean,
        }]
        .into_boxed_slice(),
        slots: vec![
            SlotDeclaration {
                slot: CellSlotId(0),
                schema: boolean,
                role: SlotRole::Input,
                producer: ProducerReference::Input(InputId(0)),
                initializer: None,
            },
            SlotDeclaration {
                slot: CellSlotId(1),
                schema: scalar,
                role: SlotRole::Derived,
                producer: ProducerReference::NodeOutput {
                    node: NodeId(0),
                    output_ordinal: 0,
                },
                initializer: None,
            },
            SlotDeclaration {
                slot: CellSlotId(2),
                schema: scalar,
                role: SlotRole::Output,
                producer: ProducerReference::Output {
                    output: OutputId(0),
                    source: ArtifactSource::Slot(CellSlotId(1)),
                },
                initializer: None,
            },
        ]
        .into_boxed_slice(),
        nodes: vec![NodeDeclaration {
            node: NodeId(0),
            body: ExecutableNodeBody::BooleanMatch(BooleanMatchDeclaration {
                scrutinee: 0,
                captures: Box::new([]),
                arms: vec![
                    BooleanMatchArm {
                        pattern: BooleanPattern::Literal(true),
                        guard: None,
                        body: block(0, constants[0]),
                    },
                    BooleanMatchArm {
                        pattern: BooleanPattern::Literal(false),
                        guard: None,
                        body: block(1, constants[1]),
                    },
                ]
                .into_boxed_slice(),
            }),
            input_bindings: 0..1,
            output_bindings: 1..2,
        }]
        .into_boxed_slice(),
        bindings: vec![
            BindingDeclaration::Input {
                id: BindingId(0),
                node: NodeId(0),
                port_ordinal: 0,
                source: ArtifactSource::Slot(CellSlotId(0)),
            },
            BindingDeclaration::Output {
                id: BindingId(1),
                node: NodeId(0),
                port_ordinal: 0,
                target: CellSlotId(1),
            },
        ]
        .into_boxed_slice(),
        outputs: vec![OutputDeclaration {
            output: OutputId(0),
            name: "result".into(),
            interactive_binding: None,
            source: CellSlotId(2),
            schema: scalar,
        }]
        .into_boxed_slice(),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    }
}

fn control(draft: &mut ProgramArtifactDraft) -> &mut BooleanMatchDeclaration {
    let ExecutableNodeBody::BooleanMatch(control) = &mut draft.nodes[0].body else {
        panic!()
    };
    control
}

#[test]
fn typed_match_roundtrip_and_revision_own_all_branch_semantics() {
    let draft = fixture();
    let artifact = draft.clone().finalize().unwrap();
    let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(artifact.revision(), decoded.revision());
    assert_eq!(artifact.nodes(), decoded.nodes());
    for change in 0..3 {
        let mut changed = draft.clone();
        let matched = control(&mut changed);
        match change {
            0 => {
                matched.arms[0].pattern = BooleanPattern::Literal(false);
                matched.arms[1].pattern = BooleanPattern::Literal(true);
            }
            1 => {
                let value = matched.arms[0].body.yield_value;
                matched.arms[0].body.yield_value = matched.arms[1].body.yield_value;
                matched.arms[1].body.yield_value = value;
            }
            _ => matched.arms[1].pattern = BooleanPattern::Wildcard,
        }
        assert_ne!(artifact.revision(), changed.finalize().unwrap().revision());
    }
}

#[test]
fn typed_match_rejects_invalid_scope_coverage_schema_and_writer() {
    for mutation in 0..8 {
        let mut draft = fixture();
        let boolean = draft.inputs[0].schema;
        match mutation {
            0 => control(&mut draft).arms[1].pattern = BooleanPattern::Literal(true),
            1 => control(&mut draft).arms[0].body.id = ControlBlockId(9),
            2 => {
                control(&mut draft).arms[0].body.yield_value = ControlValue::Local {
                    block: ControlBlockId(1),
                    node: 0,
                }
            }
            3 => {
                control(&mut draft).arms[0].body.yield_value = ControlValue::Parameter {
                    block: ControlBlockId(0),
                    ordinal: 0,
                }
            }
            4 => control(&mut draft).scrutinee = 9,
            5 => draft.slots[1].schema = boolean,
            6 => draft.slots[1].role = SlotRole::State,
            _ => {
                let guard = control(&mut draft).arms[0].body.clone();
                control(&mut draft).arms[0].guard = Some(guard);
                control(&mut draft).arms[0].body.id = ControlBlockId(1);
                control(&mut draft).arms[1].body.id = ControlBlockId(2);
            }
        }
        assert!(draft.finalize().is_err(), "mutation {mutation}");
    }
}

#[test]
fn typed_match_codec_admits_exact_bounds_and_rejects_unknown_tags() {
    let artifact = fixture().finalize().unwrap();
    let sections = encode_program_artifact_sections(&artifact).unwrap();
    let exact = ArtifactDecodeLimits {
        max_nodes: 1,
        max_control_arms: 2,
        max_control_blocks: 2,
        max_control_operations: 0,
        max_control_operands: 0,
        ..ArtifactDecodeLimits::default()
    };
    assert_eq!(
        decode_program_artifact_sections_with_limits(&sections, exact)
            .unwrap()
            .revision(),
        artifact.revision()
    );
    for limits in [
        ArtifactDecodeLimits {
            max_control_arms: 1,
            ..exact
        },
        ArtifactDecodeLimits {
            max_control_blocks: 1,
            ..exact
        },
        ArtifactDecodeLimits {
            max_nodes: 0,
            ..exact
        },
    ] {
        assert!(decode_program_artifact_sections_with_limits(&sections, limits).is_err());
    }
    for key in ["revision", "pattern"] {
        let mut sections = sections.clone();
        let text = String::from_utf8(sections.nodes.clone()).unwrap();
        let text = if key == "revision" {
            text.replace("\"revision\":2", "\"revision\":9")
        } else {
            text.replace("\"pattern\":1", "\"pattern\":9")
        };
        sections.nodes = text.into_bytes();
        assert!(
            decode_program_artifact_sections(&sections).is_err(),
            "{key}"
        );
    }
}

#[test]
fn every_local_operation_requires_its_exact_ordinary_contract() {
    let base = compile("-11").compile_artifact().unwrap();
    let operation = base.nodes()[0].as_operation().unwrap().operation.clone();
    for mutation in 0..5 {
        let mut draft = fixture();
        let scalar = draft.slots[1].schema;
        let contract = draft
            .contracts
            .iter()
            .find(|contract| {
                matches!(contract,
            mech_core::ResolvedOperationContract::Declared(contract)
                if contract.inputs.len() == 1 && contract.inputs[0].schema == scalar
                    && contract.outputs.len() == 1 && contract.outputs[0].schema == scalar)
            })
            .unwrap()
            .clone();
        let mut contracts = mech_core::OperationContractTableBuilder::new();
        let handle = contracts.insert(contract).unwrap();
        let built = contracts.finish().unwrap();
        let id = built.resolve(handle).unwrap();
        draft.contracts = built.into_parts().0;
        let block = &mut control(&mut draft).arms[0].body;
        block.operations = vec![ControlOperation {
            node: 0,
            operation: operation.clone(),
            contract: id,
            inputs: vec![block.yield_value].into_boxed_slice(),
            schema: scalar,
        }]
        .into_boxed_slice();
        block.yield_value = ControlValue::Local {
            block: block.id,
            node: 0,
        };
        match mutation {
            0 => {}
            1 => block.operations[0].contract = mech_core::OperationContractId::new(u32::MAX),
            2 => block.operations[0].node = 1,
            3 => block.operations[0].inputs = Box::new([]),
            _ => block.operations[0].inputs[0] = block.yield_value,
        }
        let result = draft.finalize();
        if mutation == 0 {
            result.unwrap();
        } else {
            assert!(result.is_err(), "mutation {mutation}");
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn resident_typed_match_switches_true_false_true_without_eager_blocks() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let artifact = fixture().finalize().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d, 1),
        &decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap(),
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    assert!(!instance.plan.has_only_kernel_steps());
    let matched = match &artifact.nodes()[0].body {
        ExecutableNodeBody::BooleanMatch(matched) => matched,
        _ => panic!(),
    };
    let expected = matched
        .arms
        .iter()
        .map(|arm| match arm.body.yield_value {
            ControlValue::Constant(id) => match artifact.constants().get(id).unwrap().data() {
                ValueData::F64(value) => value.to_f64(),
                _ => panic!(),
            },
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    let input = instance.plan.inputs[0].clone();
    for (index, flag) in [1u8, 0, 1].into_iter().enumerate() {
        instance
            .turn(&[CapturedSignalInput {
                slot: input.slot,
                value: ResidentValueRef::Bool(&[flag]),
            }])
            .unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!()
        };
        assert_eq!(value.to_f64(), expected[usize::from(index == 1)]);
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn canonical_match_blocks_roundtrip_and_execute_captures_binding_and_guards() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for source in [
        "flag<bool> ? | true => signal<f64> + 1 | false => signal<f64> + 2",
        "flag<bool> ? | x, x => signal<f64> + 1 | * => signal<f64> + 2",
    ] {
        let compiled = compile(source);
        assert_eq!(compiled.program().inputs.len(), 2);
        assert_eq!(
            compiled.program().nodes.len(),
            1,
            "arm operations must be block-owned"
        );
        assert!(matches!(
            compiled.program().nodes[0].body,
            SourceNodeBody::BooleanMatch(_)
        ));
        let artifact = compiled.compile_artifact().unwrap();
        let decoded = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        for artifact in [&artifact, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x4d, 2),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            let input_slots = instance
                .plan
                .inputs
                .iter()
                .map(|input| input.slot)
                .collect::<Vec<_>>();
            for (flag, signal, expected) in [(1u8, 10.0, 11.0), (0, 20.0, 22.0), (1, 30.0, 31.0)] {
                instance
                    .turn(&[
                        CapturedSignalInput {
                            slot: input_slots[0],
                            value: ResidentValueRef::Bool(&[flag]),
                        },
                        CapturedSignalInput {
                            slot: input_slots[1],
                            value: ResidentValueRef::F64(&[signal]),
                        },
                    ])
                    .unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::F64(value) = output.data() else {
                    panic!()
                };
                assert_eq!(value.to_f64(), expected, "{source}");
            }
        }
    }
}

#[cfg(feature = "resident-artifact")]
mod lazy_execution {
    use super::*;
    use mech_core::{
        BoundResidentKernel, FunctionCatalogBuilder, ManagedMemoryBudget, ReactiveInstanceId,
        ResidentKernelError, ResidentKernelInputs, ResidentValueMut, ResidentValueRef, ValueData,
    };
    use mech_engine::resident::{
        ActivatedTurnStep, ActivationFacts, CapturedSignalInput, ResidentActivationOptions,
        activate, activate_with_options,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    static FAILING_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn partial_failure(
        _: &BoundResidentKernel,
        _: &dyn ResidentKernelInputs,
        output: ResidentValueMut<'_>,
    ) -> Result<bool, ResidentKernelError> {
        FAILING_CALLS.fetch_add(1, Ordering::SeqCst);
        let ResidentValueMut::F64(output) = output else {
            return Err(ResidentKernelError::InvalidOutput);
        };
        output[0] = 999.0;
        Err(ResidentKernelError::Arithmetic)
    }

    fn stateful_artifact(guarded: bool) -> ProgramArtifact {
        let compiled = compile(if guarded {
            "flag<bool> ? | x, x => signal<f64> + 1 | * => signal<f64> + 2"
        } else {
            "flag<bool> ? | true => signal<f64> + 1 | false => signal<f64> + 2"
        });
        let mut graph = compiled.program().clone();
        let schema = graph.outputs[0].schema;
        let initial = (0..compiled.constants().len())
            .map(|id| ConstantId::new(id as u32))
            .find(|id| compiled.constants().get(*id).unwrap().schema() == schema)
            .unwrap();
        graph.states = vec![SourceState {
            schema,
            initializer: Some(initial),
            producer_node: 1,
            producer_output_ordinal: 0,
        }]
        .into_boxed_slice();
        let mut nodes = graph.nodes.into_vec();
        nodes.push(SourceNode {
            body: SourceNodeBody::Operation {
                operation: OperationReference {
                    module_path: vec!["core".to_owned()].into_boxed_slice(),
                    operation_name: "assign".to_owned(),
                },
                requirement: None,
            },
            inputs: vec![SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            }]
            .into_boxed_slice(),
            outputs: vec![SourceNodeOutput::State(0)].into_boxed_slice(),
        });
        graph.nodes = nodes.into_boxed_slice();
        graph.outputs[0].source = SourceValue::State(0);
        let assign = mech_core::maintained_operation_contract("core/assign", 1, false).unwrap();
        compile_source_program_with_control_contracts(
            &graph,
            &mut ArtifactBuildContext::new(compiled.schemas(), compiled.constants()),
            &[None, Some(&assign)],
        )
        .unwrap()
    }

    fn output(instance: &mech_engine::resident::ReactiveInstance) -> f64 {
        let value = instance.copied_output(0).unwrap();
        let ValueData::F64(value) = value.data() else {
            panic!()
        };
        value.to_f64()
    }

    fn state(instance: &mech_engine::resident::ReactiveInstance, slot: CellSlotId) -> f64 {
        let mech_engine::resident::ResidentValueBorrow::F64 {
            values: [value],
            shape,
        } = instance.state_borrow(slot).unwrap()
        else {
            panic!()
        };
        assert_eq!(shape, mech_core::ResidentShape::SCALAR);
        *value
    }

    #[test]
    fn inactive_failure_guard_laziness_selected_failure_and_abort_preserve_state() {
        for guarded in [false, true] {
            FAILING_CALLS.store(0, Ordering::SeqCst);
            let mut catalog = FunctionCatalogBuilder::new();
            install_intrinsic_resident(&mut catalog).unwrap();
            let catalog = catalog.build().unwrap();
            let artifact = stateful_artifact(guarded);
            let artifact = decode_program_artifact_bytecode_v1(
                &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
            )
            .unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x4d, 3),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            let slots = instance
                .plan
                .inputs
                .iter()
                .map(|input| input.slot)
                .collect::<Vec<_>>();
            let selected = instance
                .plan
                .steps
                .iter()
                .find_map(|step| match step {
                    ActivatedTurnStep::BooleanMatch(matched) => {
                        Some(matched.arms[0].body.kernels.start as usize)
                    }
                    _ => None,
                })
                .unwrap();
            let original = instance.plan.replace_kernel_for_test(
                selected,
                BoundResidentKernel::new(partial_failure, Box::new([])),
            );
            let signal = [10.0];
            let capture = |flag| {
                [
                    CapturedSignalInput {
                        slot: slots[0],
                        value: ResidentValueRef::Bool(flag),
                    },
                    CapturedSignalInput {
                        slot: slots[1],
                        value: ResidentValueRef::F64(&signal),
                    },
                ]
            };
            instance.turn(&capture(&[0])).unwrap();
            assert_eq!(
                FAILING_CALLS.load(Ordering::SeqCst),
                0,
                "a false guard must not execute its body"
            );
            assert_eq!(output(&instance), 12.0);
            let epoch = instance.published_epoch();
            let state_slot = instance
                .plan
                .slots
                .iter()
                .find(|slot| slot.role == SlotRole::State)
                .unwrap()
                .artifact_id;
            let prior_state = state(&instance, state_slot);
            assert!(instance.turn(&capture(&[1])).is_err());
            assert_eq!(FAILING_CALLS.load(Ordering::SeqCst), 1);
            assert_eq!(instance.published_epoch(), epoch);
            assert_eq!(output(&instance), 12.0);
            assert_eq!(state(&instance, state_slot), prior_state);
            instance.plan.replace_kernel_for_test(selected, original);
            instance.prepare_turn(&capture(&[1])).unwrap().abort();
            assert_eq!(instance.published_epoch(), epoch);
            assert_eq!(output(&instance), 12.0);
            assert_eq!(state(&instance, state_slot), prior_state);
            instance.turn(&capture(&[1])).unwrap();
            assert_eq!(output(&instance), 11.0);
            instance.turn_without_summary(&capture(&[0])).unwrap();
            assert_eq!(output(&instance), 12.0);
        }
    }

    #[test]
    fn exact_backing_budget_admits_both_arms_and_one_below_rejects() {
        let mut catalog = FunctionCatalogBuilder::new();
        install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let artifact = stateful_artifact(true);
        let activate_budget = |budget: &ManagedMemoryBudget| {
            activate_with_options(
                ReactiveInstanceId::new(0x4d, 4),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
                ResidentActivationOptions {
                    memory_budget: Some(budget.clone()),
                    ..ResidentActivationOptions::default()
                },
            )
        };
        let mut lower = 0;
        let mut upper = 1u64;
        while activate_budget(&ManagedMemoryBudget::new(upper)).is_err() {
            lower = upper;
            upper = upper.checked_mul(2).unwrap();
        }
        while upper - lower > 1 {
            let middle = lower + (upper - lower) / 2;
            if activate_budget(&ManagedMemoryBudget::new(middle)).is_ok() {
                upper = middle;
            } else {
                lower = middle;
            }
        }
        let under = ManagedMemoryBudget::new(upper - 1);
        assert!(activate_budget(&under).is_err());
        assert_eq!(under.used_bytes(), 0);
        let exact = ManagedMemoryBudget::new(upper);
        let mut instance = activate_budget(&exact).unwrap();
        let retained = exact.used_bytes();
        let slots = instance
            .plan
            .inputs
            .iter()
            .map(|input| input.slot)
            .collect::<Vec<_>>();
        for turn in 0..100 {
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: slots[0],
                        value: ResidentValueRef::Bool(&[(turn % 2) as u8]),
                    },
                    CapturedSignalInput {
                        slot: slots[1],
                        value: ResidentValueRef::F64(&[10.0]),
                    },
                ])
                .unwrap();
            assert_eq!(exact.used_bytes(), retained);
            let Some(mech_engine::resident::ResidentValueBorrow::F64 {
                values: [value],
                shape,
            }) = instance.output_borrow(0)
            else {
                panic!("native scalar output");
            };
            assert_eq!(shape, mech_core::ResidentShape::SCALAR);
            assert_eq!(*value, if turn % 2 == 0 { 12.0 } else { 11.0 });
        }
        drop(instance);
        assert_eq!(exact.used_bytes(), 0);
    }
}

#[test]
fn source_only_match_has_typed_capture_and_binding_authority() {
    for source in [
        "flag<bool> ? | true => signal<f64> + 1 | false => signal<f64> + 2",
        "flag<bool> ? | x, !x => signal<f64> + 2 | * => signal<f64> + 1",
        "flag<bool> ? | x => x",
    ] {
        let compiled = compile(source);
        assert_eq!(compiled.program().nodes.len(), 1);
        assert_eq!(compiled.contracts(), &[None]);
        let SourceNodeBody::BooleanMatch(control) = &compiled.program().nodes[0].body else {
            panic!()
        };
        assert!(!control.arms.is_empty());
        assert!(
            compiled
                .source_map()
                .nodes
                .iter()
                .all(|node| node.operation != "source/match" && node.operation != "source/bind")
        );
        let artifact = compiled.compile_artifact().unwrap();
        let decoded = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        assert_eq!(artifact.revision(), decoded.revision());
        assert_eq!(artifact.nodes(), decoded.nodes());
    }
}

#[test]
fn captured_source_identity_and_guard_changes_change_revision() {
    let original = compile("flag<bool> ? | x, x => left<f64> + 1 | * => right<f64> + 2")
        .compile_artifact()
        .unwrap();
    for source in [
        "flag<bool> ? | x, !x => left<f64> + 1 | * => right<f64> + 2",
        "flag<bool> ? | x, x => right<f64> + 1 | * => left<f64> + 2",
    ] {
        assert_ne!(
            original.revision(),
            compile(source).compile_artifact().unwrap().revision()
        );
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn portable_scalar_match_target_capability_is_separate_from_artifact_validity() {
    let compiled = compile("flag<bool> ? | true => 1u8 | false => 2u8");
    let artifact = compiled.compile_artifact().unwrap();
    let artifact = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    assert_eq!(
        artifact
            .schemas()
            .get(artifact.outputs()[0].schema)
            .unwrap()
            .body(),
        &SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8)
    );
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let result = mech_engine::resident::preflight_resident_target(
        &artifact,
        &catalog.build().unwrap(),
        &mech_engine::resident::ActivationFacts::default(),
        mech_engine::resident::ResidentActivationOptions::default(),
    )
    .err()
    .expect("expected rejection");
    assert_eq!(result.node, Some(NodeId(0)));
    assert_eq!(result.target, mech_core::ExecutionTarget::ResidentCpu);
    assert!(result.reason.contains("UnsupportedControlLayout"));
}

#[cfg(feature = "resident-artifact")]
#[test]
fn local_operations_have_capability_witnesses_and_report_the_enclosing_owner() {
    use mech_engine::resident::{
        ActivationFacts, ResidentActivationOptions, preflight_resident_target,
    };
    let artifact = compile("flag<bool> ? | x, !x => signal<f64> + 2 | * => signal<f64> + 1")
        .compile_artifact()
        .unwrap();
    let empty = mech_core::FunctionCatalogBuilder::new().build().unwrap();
    let error = preflight_resident_target(
        &artifact,
        &empty,
        &ActivationFacts::default(),
        ResidentActivationOptions::default(),
    )
    .err()
    .expect("expected rejection");
    assert_eq!(
        error.node,
        Some(NodeId(0)),
        "local physical IDs must not escape as source owners"
    );
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let witness = preflight_resident_target(
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
        ResidentActivationOptions::default(),
    )
    .unwrap();
    assert_eq!(witness.concrete_cases.len(), 3);
    assert!(
        witness
            .concrete_cases
            .iter()
            .all(|case| case.node == NodeId(0))
    );
    assert_eq!(
        witness
            .concrete_cases
            .iter()
            .filter(|case| case.operation.canonical_name() == "math/add")
            .count(),
        2
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn scalar_layouts_and_guard_operations_match_fresh_instances() {
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentActivationOptions, ResidentIntegrityMode,
        ResidentValueBorrow, activate_with_options,
    };
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (kind, source) in [
        (0, "flag<bool> ? | x, !x => false | * => true"),
        (1, "flag<bool> ? | x, !x => left<index> | * => right<index>"),
        (2, "flag<bool> ? | x, !x => 2.0 | * => 1.0"),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let artifact = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        for integrity in [
            ResidentIntegrityMode::Checked,
            ResidentIntegrityMode::Unchecked,
        ] {
            let activate = || {
                activate_with_options(
                    mech_core::ReactiveInstanceId::new(822, 5),
                    &artifact,
                    &catalog,
                    &ActivationFacts::default(),
                    ResidentActivationOptions {
                        integrity,
                        ..Default::default()
                    },
                )
                .unwrap()
            };
            let mut instance = activate();
            for flag in [1u8, 0, 1] {
                let mut fresh = activate();
                for instance in [&mut instance, &mut fresh] {
                    let flag_values = [flag];
                    let inputs = instance
                        .plan
                        .inputs
                        .iter()
                        .enumerate()
                        .map(|(ordinal, input)| CapturedSignalInput {
                            slot: input.slot,
                            value: match ordinal {
                                0 => mech_core::ResidentValueRef::Bool(&flag_values),
                                1 => mech_core::ResidentValueRef::Index(&[2]),
                                2 => mech_core::ResidentValueRef::Index(&[1]),
                                _ => panic!("input layout"),
                            },
                        })
                        .collect::<Vec<_>>();
                    instance.turn(&inputs).unwrap();
                    match (kind, instance.output_borrow(0).unwrap()) {
                        (
                            0,
                            ResidentValueBorrow::Bool {
                                values: [actual], ..
                            },
                        ) => assert_eq!(*actual, flag),
                        (
                            1,
                            ResidentValueBorrow::Index {
                                values: [actual], ..
                            },
                        ) => assert_eq!(*actual, if flag == 0 { 2 } else { 1 }),
                        (
                            2,
                            ResidentValueBorrow::F64 {
                                values: [actual], ..
                            },
                        ) => assert_eq!(*actual, if flag == 0 { 2.0 } else { 1.0 }),
                        _ => panic!("exact native scalar layout"),
                    }
                }
            }
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn match_integrity_failure_preserves_state_and_published_epoch() {
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentValueBorrow, activate,
    };
    let compiled = compile("flag<bool> ? | true => true | false => false");
    let mut graph = compiled.program().clone();
    let schema = graph.outputs[0].schema;
    let initial = (0..compiled.constants().len())
        .map(|id| ConstantId::new(id as u32))
        .find(|id| {
            matches!(
                compiled.constants().get(*id).unwrap().data(),
                mech_core::ValueData::Bool(true)
            )
        })
        .unwrap();
    graph.states = vec![SourceState {
        schema,
        initializer: Some(initial),
        producer_node: 1,
        producer_output_ordinal: 0,
    }]
    .into_boxed_slice();
    let mut nodes = graph.nodes.into_vec();
    nodes.push(SourceNode {
        body: SourceNodeBody::Operation {
            operation: OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "assign".to_owned(),
            },
            requirement: None,
        },
        inputs: vec![SourceValue::NodeOutput {
            node: 0,
            output_ordinal: 0,
        }]
        .into_boxed_slice(),
        outputs: vec![SourceNodeOutput::State(0)].into_boxed_slice(),
    });
    graph.nodes = nodes.into_boxed_slice();
    graph.outputs[0].source = SourceValue::State(0);
    graph.constraints = vec![SourceIntegrityConstraint {
        name: "selected result must hold".to_owned(),
        operation: OperationReference {
            module_path: vec!["integrity".to_owned()].into_boxed_slice(),
            operation_name: "assert".to_owned(),
        },
        inputs: vec![SourceValue::State(0)].into_boxed_slice(),
    }]
    .into_boxed_slice();
    let assign = mech_core::maintained_operation_contract("core/assign", 1, false).unwrap();
    let artifact = compile_source_program_with_control_contracts(
        &graph,
        &mut ArtifactBuildContext::new(compiled.schemas(), compiled.constants()),
        &[None, Some(&assign)],
    )
    .unwrap();
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(822, 6),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let input = instance.plan.inputs[0].slot;
    let state = instance
        .plan
        .slots
        .iter()
        .find(|slot| slot.role == SlotRole::State)
        .unwrap()
        .artifact_id;
    instance
        .turn(&[CapturedSignalInput {
            slot: input,
            value: mech_core::ResidentValueRef::Bool(&[1]),
        }])
        .unwrap();
    let epoch = instance.published_epoch();
    assert!(
        instance
            .turn(&[CapturedSignalInput {
                slot: input,
                value: mech_core::ResidentValueRef::Bool(&[0])
            }])
            .is_err()
    );
    assert_eq!(instance.published_epoch(), epoch);
    assert!(matches!(
        instance.output_borrow(0),
        Some(ResidentValueBorrow::Bool { values: [1], .. })
    ));
    assert!(matches!(
        instance.state_borrow(state),
        Some(ResidentValueBorrow::Bool { values: [1], .. })
    ));
    instance
        .turn(&[CapturedSignalInput {
            slot: input,
            value: mech_core::ResidentValueRef::Bool(&[1]),
        }])
        .unwrap();
}

#[test]
fn unfinished_control_forms_are_explicit_source_errors() {
    for source in ["x<u8> ? | y => y | * => 0u8", "x<*> ? | y => y | * => 0"] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(822), Revision(1), source).unwrap(),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean());
        assert_eq!(parsed.consumed.end.0 as usize, source.len());
        let error = CanonicalSourceFrontend
            .compile_expression(&find(parsed.syntax()).unwrap())
            .err()
            .expect("expected rejection");
        assert_eq!(error.code, "source-semantics/unsupported-boolean-match");
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn long_blocks_keep_local_slots_outside_the_artifact_schedule() {
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentValueBorrow, activate,
    };
    let source = format!(
        "flag<bool> ? | true => signal<f64>{} | false => 0",
        " + 1".repeat(80)
    );
    let compiled = compile(&source);
    assert_eq!(compiled.program().nodes.len(), 1);
    let artifact = compiled.compile_artifact().unwrap();
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(822, 7),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    assert_eq!(instance.plan.steps.len(), 81);
    let slots = instance
        .plan
        .inputs
        .iter()
        .map(|input| input.slot)
        .collect::<Vec<_>>();
    for flag in [1u8, 0, 1] {
        instance
            .turn(&[
                CapturedSignalInput {
                    slot: slots[0],
                    value: mech_core::ResidentValueRef::Bool(&[flag]),
                },
                CapturedSignalInput {
                    slot: slots[1],
                    value: mech_core::ResidentValueRef::F64(&[5.0]),
                },
            ])
            .unwrap();
        let Some(ResidentValueBorrow::F64 {
            values: [actual], ..
        }) = instance.output_borrow(0)
        else {
            panic!("scalar result");
        };
        assert_eq!(*actual, if flag == 1 { 85.0 } else { 0.0 });
    }
}
