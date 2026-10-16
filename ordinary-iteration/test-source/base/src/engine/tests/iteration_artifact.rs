//! Counted application is durable typed control, independent of source spelling.
#![cfg(feature = "compiler")]

use mech_core::{
    ConstantStoreBuilder, IntegerWidth, OperationContractDeclaration, SchemaBody, SchemaDraft,
    SchemaId, SchemaTable, SchemaTableBuilder,
};
use mech_engine::*;

fn schemas() -> (SchemaTable, SchemaId, SchemaId) {
    let mut builder = SchemaTableBuilder::new();
    let carrier = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::UnsignedInteger(IntegerWidth::W64),
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let other = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::Bool,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    let carrier = built.resolve(carrier).unwrap();
    let other = built.resolve(other).unwrap();
    (built.table, carrier, other)
}

fn identity(
    schema: SchemaId,
    count: u64,
    captures: usize,
) -> IterationDeclaration<OperationContractDeclaration> {
    IterationDeclaration {
        count,
        seed: 0,
        carrier: schema,
        captures: (0..captures)
            .map(|index| ControlCapture {
                input: index as u16 + 1,
                schema,
                freeze_on_suspend: true,
            })
            .collect(),
        body: ControlBlock {
            id: ControlBlockId(0),
            parameters: core::iter::once(ControlParameter {
                source: ControlParameterSource::IterationCarrier,
                schema,
            })
            .chain((0..captures).map(|index| ControlParameter {
                source: ControlParameterSource::Capture(index as u16),
                schema,
            }))
            .collect(),
            operations: Box::new([]),
            yield_value: ControlValue::Parameter {
                block: ControlBlockId(0),
                ordinal: 0,
            },
        },
    }
}

fn build(
    schemas: &SchemaTable,
    control: IterationDeclaration<OperationContractDeclaration>,
    input_schemas: &[SchemaId],
) -> Result<ProgramArtifact, ArtifactBuildError> {
    let carrier = control.carrier;
    let constants = ConstantStoreBuilder::new(schemas)
        .finish()
        .unwrap()
        .into_parts()
        .0;
    let graph = SourceProgram {
        inputs: input_schemas
            .iter()
            .enumerate()
            .map(|(index, schema)| SourceInput {
                name: format!("input{index}"),
                schema: *schema,
            })
            .collect(),
        nodes: vec![SourceNode {
            body: SourceNodeBody::Iteration(control),
            inputs: (0..input_schemas.len())
                .map(|index| SourceValue::Input(index as u32))
                .collect(),
            outputs: vec![SourceNodeOutput::Derived { schema: carrier }].into_boxed_slice(),
        }]
        .into_boxed_slice(),
        outputs: vec![SourceOutput {
            name: "result".to_owned(),
            interactive_symbol: None,
            source: SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            },
            schema: carrier,
        }]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    compile_source_program_with_control_contracts(
        &graph,
        &mut ArtifactBuildContext::new(schemas, &constants),
        &[None],
    )
}

fn invalid(result: Result<ProgramArtifact, ArtifactBuildError>) {
    assert!(
        matches!(result, Err(ArtifactBuildError::InvalidControl { .. })),
        "{result:?}"
    );
}

#[test]
fn iteration_roundtrip_binds_exact_count_without_unrolling() {
    let (schemas, carrier, _) = schemas();
    let mut prior_revision = None;
    let mut zero_bytes = 0;
    for count in [0, 1, 19, 25, u64::MAX] {
        let original = build(&schemas, identity(carrier, count, 0), &[carrier]).unwrap();
        assert_eq!(original.nodes().len(), 1);
        let ExecutableNodeBody::Iteration(control) = &original.nodes()[0].body else {
            panic!("a real iteration node is required")
        };
        assert_eq!(control.count, count);
        assert!(control.body.operations.is_empty());
        let encoded = encode_program_artifact_bytecode_v1(&original).unwrap();
        let decoded = decode_program_artifact_bytecode_v1(&encoded).unwrap();
        assert_eq!(original.nodes(), decoded.nodes());
        assert_eq!(original.revision(), decoded.revision());
        assert_eq!(
            encoded,
            encode_program_artifact_bytecode_v1(&decoded).unwrap()
        );
        if count == 0 {
            zero_bytes = encoded.len();
        }
        assert!(
            encoded.len() <= zero_bytes + 64,
            "literal count changes metadata, never graph cardinality"
        );
        assert_ne!(prior_revision, Some(original.revision()));
        prior_revision = Some(original.revision());
    }
}

#[test]
fn iteration_revision_binds_seed_capture_order_and_body() {
    let (schemas, carrier, _) = schemas();
    let control = identity(carrier, 19, 1);
    let baseline = build(&schemas, control.clone(), &[carrier, carrier])
        .unwrap()
        .revision();
    let mut swapped = control.clone();
    swapped.seed = 1;
    swapped.captures[0].input = 0;
    assert_ne!(
        baseline,
        build(&schemas, swapped, &[carrier, carrier])
            .unwrap()
            .revision()
    );
    let mut capture_yield = control;
    capture_yield.body.yield_value = ControlValue::Parameter {
        block: ControlBlockId(0),
        ordinal: 1,
    };
    assert_ne!(
        baseline,
        build(&schemas, capture_yield, &[carrier, carrier])
            .unwrap()
            .revision()
    );
}

#[test]
fn iteration_rejects_foreign_roles_and_noncanonical_capture_metadata() {
    let (schemas, carrier, other) = schemas();
    let control = identity(carrier, 0, 1);
    let mut wrong = control.clone();
    wrong.body.parameters[0].source = ControlParameterSource::Scrutinee;
    invalid(build(&schemas, wrong, &[carrier, carrier]));
    let mut wrong = control.clone();
    wrong.captures[0].freeze_on_suspend = false;
    invalid(build(&schemas, wrong, &[carrier, carrier]));
    let mut wrong = control.clone();
    wrong.body.parameters[1].schema = other;
    invalid(build(&schemas, wrong, &[carrier, carrier]));
    let mut wrong = control.clone();
    wrong.body.yield_value = ControlValue::Parameter {
        block: ControlBlockId(99),
        ordinal: 0,
    };
    invalid(build(&schemas, wrong, &[carrier, carrier]));
    let mut wrong = control;
    wrong.captures[0].input = 0;
    invalid(build(&schemas, wrong, &[carrier, carrier]));
}

#[test]
fn iteration_zero_count_still_rejects_effects_and_forward_locals() {
    let (schemas, carrier, _) = schemas();
    for body in [
        ControlOperationBody::Recur(0),
        ControlOperationBody::Suspend,
        ControlOperationBody::Publish,
    ] {
        let mut control = identity(carrier, 0, 0);
        control.body.operations = vec![ControlOperation {
            node: 0,
            body,
            inputs: vec![ControlValue::Parameter {
                block: ControlBlockId(0),
                ordinal: 0,
            }]
            .into_boxed_slice(),
            schema: carrier,
        }]
        .into_boxed_slice();
        invalid(build(&schemas, control, &[carrier]));
    }
    let mut control = identity(carrier, 0, 0);
    control.body.operations = vec![ControlOperation {
        node: 0,
        body: ControlOperationBody::Operation {
            operation: OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "assign".to_owned(),
            },
            contract: mech_core::maintained_operation_contract("core/assign", 1, false)
                .unwrap()
                .clone(),
        },
        inputs: vec![ControlValue::Local {
            block: ControlBlockId(0),
            node: 0,
        }]
        .into_boxed_slice(),
        schema: carrier,
    }]
    .into_boxed_slice();
    invalid(build(&schemas, control, &[carrier]));
}

#[test]
fn iteration_nested_lexical_identity_is_validated_before_zero_count() {
    let (schemas, carrier, _) = schemas();
    let mut outer = identity(carrier, 0, 0);
    let mut inner = identity(carrier, u64::MAX, 0);
    inner.body.id = ControlBlockId(1);
    inner.body.yield_value = ControlValue::Parameter {
        block: ControlBlockId(1),
        ordinal: 0,
    };
    outer.body.operations = vec![ControlOperation {
        node: 0,
        body: ControlOperationBody::Iteration(inner),
        inputs: vec![ControlValue::Parameter {
            block: ControlBlockId(0),
            ordinal: 0,
        }]
        .into_boxed_slice(),
        schema: carrier,
    }]
    .into_boxed_slice();
    outer.body.yield_value = ControlValue::Local {
        block: ControlBlockId(0),
        node: 0,
    };
    let original = build(&schemas, outer.clone(), &[carrier]).unwrap();
    let encoded = encode_program_artifact_bytecode_v1(&original).unwrap();
    assert_eq!(
        original.nodes(),
        decode_program_artifact_bytecode_v1(&encoded)
            .unwrap()
            .nodes()
    );
    let ControlOperationBody::Iteration(inner) = &mut outer.body.operations[0].body else {
        unreachable!()
    };
    inner.body.id = ControlBlockId(0);
    inner.body.yield_value = ControlValue::Parameter {
        block: ControlBlockId(0),
        ordinal: 0,
    };
    invalid(build(&schemas, outer, &[carrier]));
}

#[test]
fn zero_count_keeps_all_ordinary_body_contracts_and_operation_references() {
    let (schemas, carrier, _) = schemas();
    let mut control = identity(carrier, 0, 1);
    control.body.operations = (0..2)
        .map(|node| ControlOperation {
            node,
            body: ControlOperationBody::Operation {
                operation: OperationReference {
                    module_path: vec!["core".to_owned()].into_boxed_slice(),
                    operation_name: "assign".to_owned(),
                },
                contract: mech_core::maintained_operation_contract("core/assign", 1, false)
                    .unwrap(),
            },
            inputs: vec![ControlValue::Parameter {
                block: ControlBlockId(0),
                ordinal: node as u16,
            }]
            .into_boxed_slice(),
            schema: carrier,
        })
        .collect();
    // Both nodes are deliberately unused by the yield; zero count cannot erase
    // their declarations or hide an invalid ordinary input/output contract.
    let original = build(&schemas, control.clone(), &[carrier, carrier]).unwrap();
    let bytes = encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        let ExecutableNodeBody::Iteration(iteration) = &artifact.nodes()[0].body else {
            unreachable!()
        };
        assert_eq!(iteration.body.operations.len(), 2);
        assert_eq!(
            artifact.kernel_operation_references(),
            vec![OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "assign".to_owned(),
            }]
        );
        for operation in &iteration.body.operations {
            let ControlOperationBody::Operation { contract, .. } = &operation.body else {
                unreachable!()
            };
            let Some(mech_core::ResolvedOperationContract::Declared(contract)) =
                artifact.contracts().get(*contract)
            else {
                panic!("actual body contract")
            };
            assert_eq!(contract.inputs[0].schema, carrier);
            assert_eq!(contract.outputs[0].schema, carrier);
        }
    }
    control.body.operations[1].schema = SchemaId::new(u32::MAX);
    assert!(build(&schemas, control, &[carrier, carrier]).is_err());
}
