#![cfg(feature = "source")]

use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

fn expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x544), Revision(1), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    assert_eq!(parsed.consumed.end.0 as usize, source.len(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("canonical Expression")
}

use mech_core::{
    AliasPolicy, CardinalitySpec, ChangeDetectionPolicy, ExternalInteraction, InputPortLayout,
    IntegerWidth, OutputConstruction, SchemaBody, ShapeRule,
};

fn compile(source: &str) -> mech_engine::CanonicalSourceProgram {
    CanonicalSourceFrontend
        .compile_expression(&expression(source))
        .unwrap()
}

#[test]
fn collection_contracts_and_exact_schemas_reach_artifacts_without_providers() {
    for (source, operation, change) in [
        (
            "{1} ∪ {2}",
            "set/union",
            ChangeDetectionPolicy::AlwaysChanged,
        ),
        (
            "(|a<u8>|1u8|) ⋈ (|a<u8>|1u8|)",
            "table/join",
            ChangeDetectionPolicy::KernelReported,
        ),
    ] {
        let compiled = compile(source);
        let node = compiled.program().nodes.last().unwrap();
        assert_eq!(node.operation.canonical_name(), operation);
        let contract = compiled.contracts().last().unwrap().as_ref().unwrap();
        assert_eq!(
            contract,
            &mech_core::maintained_operation_contract(operation, 2, false).unwrap()
        );
        assert!(matches!(&contract.inputs, InputPortLayout::Fixed(inputs) if inputs.len() == 2));
        assert_eq!(contract.interaction, ExternalInteraction::Pure);
        assert_eq!(contract.outputs.len(), 1);
        assert_eq!(
            contract.outputs[0].construction,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared
            }
        );
        assert_eq!(contract.outputs[0].alias, AliasPolicy::NoAlias);
        assert_eq!(contract.outputs[0].change_detection, change);
        let output = compiled
            .schemas()
            .get(compiled.program().outputs[0].schema)
            .unwrap()
            .body();
        match output {
            SchemaBody::Set {
                element,
                cardinality,
            } => {
                assert_eq!(
                    element.as_ref(),
                    &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
                );
                assert_eq!(
                    cardinality,
                    &CardinalitySpec::Dynamic {
                        upper_bound: Some(mech_core::DimensionExpr::Constant(2))
                    }
                );
            }
            SchemaBody::Table { columns, rows } => {
                assert_eq!(columns.len(), 1);
                assert_eq!(columns[0].name, "a");
                assert_eq!(
                    columns[0].schema,
                    SchemaBody::UnsignedInteger(IntegerWidth::W8)
                );
                assert_eq!(
                    rows,
                    &CardinalitySpec::Exact(mech_core::DimensionExpr::Parameter(
                        mech_core::DimensionParameterId::new(0)
                    ))
                );
            }
            other => panic!("unexpected collection schema: {other:?}"),
        }
        let artifact = compiled.compile_artifact().unwrap();
        assert_eq!(
            artifact
                .schemas()
                .get(artifact.outputs()[0].schema)
                .unwrap()
                .body(),
            output
        );
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let parsed = mech_core::ParsedProgram::from_bytes(&bytes).unwrap();
        let decoded = mech_engine::decode_program_artifact_sections(&parsed.artifact).unwrap();
        assert_eq!(
            decoded
                .schemas()
                .get(decoded.outputs()[0].schema)
                .unwrap()
                .body(),
            output
        );
        assert_eq!(decoded.contracts(), artifact.contracts());
    }
}

#[test]
fn shared_collection_families_have_only_their_declared_binary_arity() {
    for name in [
        "set/union",
        "set/intersection",
        "set/difference",
        "set/symmetric-difference",
        "set/cartesian-product",
        "table/join",
        "table/left-outer-join",
        "table/right-outer-join",
        "table/full-outer-join",
        "table/left-semi-join",
        "table/left-anti-join",
    ] {
        let contract = mech_core::maintained_operation_contract(name, 2, false).unwrap();
        let family = if name.starts_with("set/") {
            "set/union"
        } else {
            "table/join"
        };
        assert_eq!(
            contract,
            mech_core::maintained_operation_contract(family, 2, false).unwrap()
        );
        for arity in [0, 1, 3] {
            assert!(mech_core::maintained_operation_contract(name, arity, false).is_none());
        }
    }
    assert!(mech_core::maintained_operation_contract("set/undeclared", 2, false).is_none());
    assert!(mech_core::maintained_operation_contract("table/undeclared", 2, false).is_none());
}

#[cfg(all(feature = "table", feature = "semantic-compiler"))]
#[test]
fn table_source_catalog_contracts_equal_the_source_authority() {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    for (name, factory) in [
        ("table/join", "TableJoinFxn::Inner"),
        ("table/left-outer-join", "TableJoinFxn::LeftOuter"),
        ("table/right-outer-join", "TableJoinFxn::RightOuter"),
        ("table/full-outer-join", "TableJoinFxn::FullOuter"),
        ("table/left-semi-join", "TableJoinFxn::LeftSemi"),
        ("table/left-anti-join", "TableJoinFxn::LeftAnti"),
    ] {
        let operation = mech_core::OperationId::from_name(name);
        let expected = mech_core::maintained_operation_contract(name, 2, false).unwrap();
        assert_eq!(
            catalog.specializer(operation).unwrap().operation.contract,
            expected
        );
        assert!(
            catalog
                .runtime_entry(mech_core::RuntimeFunctionId::from_name(factory))
                .is_some(),
            "{name} must retain its concrete provider"
        );
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn set_union_roundtrip_executes_canonical_values_and_changed_inputs() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    for (source, expected_inputs) in [("{1} ∪ {2}", 0), ("{signal<f64>} ∪ {2}", 1)] {
        let compiled = compile(source);
        assert_eq!(compiled.program().inputs.len(), expected_inputs, "{source}");
        let artifact = compiled.compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let parsed = mech_core::ParsedProgram::from_bytes(&bytes).unwrap();
        let decoded = mech_engine::decode_program_artifact_sections(&parsed.artifact).unwrap();
        for artifact in [&artifact, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x552, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            assert_eq!(instance.plan.inputs.len(), expected_inputs, "{source}");
            for signal in [1.0, 2.0, 3.0] {
                let data = [signal];
                let inputs = instance
                    .plan
                    .inputs
                    .first()
                    .map(|input| CapturedSignalInput {
                        slot: input.slot,
                        value: ResidentValueRef::F64(&data),
                    })
                    .into_iter()
                    .collect::<Vec<_>>();
                instance.turn(&inputs).unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::Set(values) = output.data() else {
                    panic!("union must return a canonical set")
                };
                let values = values
                    .elements()
                    .iter()
                    .map(|value| match value.data() {
                        ValueData::F64(value) => value.to_f64(),
                        other => panic!("unexpected element: {other:?}"),
                    })
                    .collect::<Vec<_>>();
                let expected = if expected_inputs == 0 || signal == 1.0 {
                    vec![1.0, 2.0]
                } else if signal == 2.0 {
                    vec![2.0]
                } else {
                    vec![2.0, 3.0]
                };
                assert_eq!(values, expected, "{source}");
            }
        }
    }
}
