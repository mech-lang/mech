//! Public typed graph fixtures; no artifact mutation or fake native witness.
#![cfg(feature = "native-artifact")]
use mech_core::{
    ConstantStoreBuilder, FloatWidth, FunctionCatalogBuilder, SchemaBody, SchemaDraft,
    SchemaTableBuilder, ValueDataDraft, ValueDraft,
    snapshot::{F64Bits, SnapshotValidationContext},
};
use mech_engine::{
    ArtifactBuildContext, OperationReference, SourceNode, SourceNodeBody, SourceNodeOutput,
    SourceOutput, SourceProgram, SourceValue, compile_source_program_with_contracts,
    native_artifact::{NativeMatrixPlanError, lower_matrix_artifact},
};

fn artifacts(computed_activation: bool) -> [mech_engine::ProgramArtifact; 2] {
    let mut schemas = SchemaTableBuilder::new();
    let handle = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                    dimensions: Box::new([
                        mech_core::DimensionExpr::Constant(2),
                        mech_core::DimensionExpr::Constant(2),
                    ]),
                },
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = schemas.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let schemas = build.into_parts().0;
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            vec![0, 1u64 << 63, 0x7ff8000000001234, 1]
                .into_iter()
                .map(|bits| ValueDataDraft::F64(F64Bits::from_bits(bits)))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    let mut constants = ConstantStoreBuilder::new(&schemas);
    let handle = constants.insert(value).unwrap();
    let build = constants.finish().unwrap();
    let constant = build.resolve(handle).unwrap();
    let constants = build.into_parts().0;
    let mut graph = SourceProgram {
        outputs: vec![
            SourceOutput {
                name: "first".into(),
                interactive_symbol: None,
                source: SourceValue::Constant(constant),
                schema,
            },
            SourceOutput {
                name: "alias".into(),
                interactive_symbol: None,
                source: SourceValue::Constant(constant),
                schema,
            },
        ]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    let contract = mech_core::maintained_operation_contract("matrix/transpose", 1, true).unwrap();
    if computed_activation {
        graph.nodes = Box::new([SourceNode {
            body: SourceNodeBody::Operation {
                operation: OperationReference {
                    module_path: Box::new(["matrix".into()]),
                    operation_name: "transpose".into(),
                },
                requirement: None,
            },
            inputs: Box::new([SourceValue::Constant(constant)]),
            outputs: Box::new([SourceNodeOutput::Derived { schema }]),
        }]);
        graph.outputs[0].source = SourceValue::NodeOutput {
            node: 0,
            output_ordinal: 0,
        };
    }
    let contracts = if computed_activation {
        vec![&contract]
    } else {
        vec![]
    };
    let original = compile_source_program_with_contracts(
        &graph,
        &mut ArtifactBuildContext::new(&schemas, &constants),
        &contracts,
    )
    .unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    [original, decoded]
}
fn catalog() -> mech_core::FunctionCatalog {
    let mut b = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut b).unwrap();
    b.build().unwrap()
}
#[test]
fn exact_constant_output_aliases_keep_one_immutable_typed_value_original_and_decoded() {
    for artifact in artifacts(false) {
        assert!(artifact.nodes().is_empty());
        assert!(artifact.slots().iter().all(|s| s.initializer.is_some()));
        let plan = lower_matrix_artifact(&artifact, &catalog()).unwrap();
        assert!(plan.calls().is_empty());
        assert_eq!(plan.constants().len(), 1);
        assert_eq!(plan.outputs().len(), 2);
        assert_eq!(plan.outputs()[0].value(), plan.outputs()[1].value());
        let value = plan.constants()[0].canonical();
        assert_eq!(value.schema(), artifact.outputs()[0].schema);
        assert!(value.validate_against(artifact.schemas()).is_ok());
    }
}
#[test]
fn constant_operands_do_not_admit_a_computed_activation_node() {
    for artifact in artifacts(true) {
        assert!(matches!(
            lower_matrix_artifact(&artifact, &catalog()),
            Err(NativeMatrixPlanError::Template(_))
        ));
    }
}
