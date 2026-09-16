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
    FloatWidth, InputPortLayout, IntegerWidth, OutputConstruction, SchemaBody, ShapeRule,
};
use mech_engine::{SourceNodeOutput, SourceValue};
fn compile(source: &str) -> mech_engine::CanonicalSourceProgram {
    CanonicalSourceFrontend
        .compile_expression(&expression(source))
        .unwrap_or_else(|error| panic!("{source}: {error}"))
}
fn output(compiled: &mech_engine::CanonicalSourceProgram) -> &SchemaBody {
    compiled
        .schemas()
        .get(compiled.program().outputs[0].schema)
        .unwrap()
        .body()
}
fn definition(source: &str) -> mech_syntax::document::VariableDefineSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x544), Revision(1), source).unwrap(),
        rules::VARIABLE_DEFINE,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    assert_eq!(parsed.consumed.end.0 as usize, source.len(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::VariableDefine)
        .and_then(mech_syntax::document::VariableDefineSyntax::cast)
        .unwrap()
}

#[test]
fn shared_contracts_preserve_variadic_construction_and_matrix_geometry_in_every_producer_profile() {
    for (source, operation) in [
        ("[1 2]", "matrix/horzcat"),
        ("matrix/horzcat(1,2)", "matrix/horzcat"),
        ("matrix/vertcat(1,2)", "matrix/vertcat"),
        ("[1 2] ** [3; 4]", "matrix/matmul"),
        ("[1 0; 0 1] \\ [2; 3]", "matrix/solve"),
        ("{1 2}", "set/define"),
    ] {
        let compiled = compile(source);
        for (node, contract) in compiled.program().nodes.iter().zip(compiled.contracts()) {
            let SourceNodeOutput::Derived { schema } = node.outputs[0] else {
                continue;
            };
            let matrix = matches!(
                compiled.schemas().get(schema).unwrap().body(),
                SchemaBody::Matrix { .. }
            );
            if let Some(expected) = mech_core::maintained_operation_contract(
                &node
                    .operation()
                    .expect("ordinary operation fixture")
                    .canonical_name(),
                node.inputs.len(),
                matrix,
            ) {
                assert_eq!(contract.as_ref(), Some(&expected), "{source}");
            }
        }
        let contract = compiled
            .contracts()
            .last()
            .unwrap()
            .as_ref()
            .unwrap_or_else(|| panic!("missing {source}: {:?}", compiled.source_map().nodes));
        assert_eq!(
            compiled
                .program()
                .nodes
                .last()
                .unwrap()
                .operation()
                .expect("ordinary operation fixture")
                .canonical_name(),
            operation
        );
        if operation.ends_with("cat") {
            assert!(matches!(
                contract.inputs,
                InputPortLayout::Variadic {
                    min_repetitions: 1,
                    ..
                }
            ));
        }
        if operation == "matrix/matmul" {
            assert_eq!(
                contract.outputs[0].construction,
                OutputConstruction::FullWrite {
                    shape: ShapeRule::MatrixProduct { lhs: 0, rhs: 1 }
                }
            );
        }
        compiled.compile_artifact().unwrap();
    }
}

#[test]
fn bindings_inherit_actual_structural_projections_and_keep_local_scope() {
    for source in [
        "{x | (x, *) <- {(1u8, true)}}",
        "{x | (*, (x, *)) <- {(true, (1u8, false))}}",
    ] {
        let compiled = compile(source);
        fn binding(pattern: &mech_engine::CollectionPattern) -> Option<mech_core::SchemaId> {
            match pattern {
                mech_engine::CollectionPattern::Bind { schema, .. } => Some(*schema),
                mech_engine::CollectionPattern::Tuple(fields) => fields.iter().find_map(binding),
                _ => None,
            }
        }
        let control = compiled
            .program()
            .nodes
            .iter()
            .find_map(|node| match &node.body {
                mech_engine::SourceNodeBody::Comprehension(control) => Some(control),
                _ => None,
            })
            .unwrap();
        let schema = control
            .steps
            .iter()
            .find_map(|step| match step {
                mech_engine::ComprehensionStep::Generator { pattern, .. } => binding(pattern),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            compiled.schemas().get(schema).unwrap().body(),
            &SchemaBody::UnsignedInteger(IntegerWidth::W8),
            "{source}"
        );
        compiled.compile_artifact().unwrap();
        assert!(
            compiled.program().inputs.iter().all(
                |input| input.name != "y" && (input.name != "x" || source.starts_with("x<u8>"))
            )
        );
    }
    let explicit_dynamic = CanonicalSourceFrontend
        .compile_expression(&expression("x<*> ? | y => y + 1 | * => 0"))
        .err()
        .unwrap();
    assert_eq!(explicit_dynamic.code, "source-semantics/unsupported-match");
    let inferred = compile("[y | x <- xs, y := x, y > 0]");
    assert_eq!(
        inferred
            .program()
            .inputs
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>(),
        vec!["xs"]
    );
    let mech_engine::SourceNodeBody::Comprehension(control) = &inferred.program().nodes[0].body
    else {
        panic!("typed collection")
    };
    let mech_engine::ComprehensionStep::Generator {
        pattern: mech_engine::CollectionPattern::Bind { schema, .. },
        ..
    } = &control.steps[0]
    else {
        panic!("typed binding")
    };
    assert_eq!(
        inferred.schemas().get(*schema).unwrap().body(),
        &SchemaBody::FloatingPoint(FloatWidth::W64)
    );
    inferred.compile_artifact().unwrap();
    let compiled = compile("{x + y | (x, y) <- {(1u8, 2u8)}}");
    let control = compiled
        .program()
        .nodes
        .iter()
        .find_map(|node| match &node.body {
            mech_engine::SourceNodeBody::Comprehension(control) => Some(control),
            _ => None,
        })
        .unwrap();
    let add = control
        .steps
        .iter()
        .find_map(|step| match step {
            mech_engine::ComprehensionStep::Operation(operation)
                if matches!(
                    &operation.body,
                    mech_engine::ControlOperationBody::Operation { operation, .. }
                        if operation.canonical_name() == "math/add"
                ) =>
            {
                Some(operation)
            }
            _ => None,
        })
        .unwrap();
    let fields = control
        .steps
        .iter()
        .find_map(|step| match step {
            mech_engine::ComprehensionStep::Generator {
                pattern: mech_engine::CollectionPattern::Tuple(fields),
                ..
            } => Some(fields),
            _ => None,
        })
        .unwrap();
    let [
        mech_engine::CollectionPattern::Bind { local: left, .. },
        mech_engine::CollectionPattern::Bind { local: right, .. },
    ] = fields.as_ref()
    else {
        panic!("distinct tuple fields")
    };
    assert_ne!(left, right);
    assert_eq!(
        add.inputs.as_ref(),
        &[
            mech_engine::ComprehensionValue::Local(*left),
            mech_engine::ComprehensionValue::Local(*right)
        ]
    );
    assert!(compiled.program().inputs.is_empty());
    assert!(
        compiled
            .source_map()
            .nodes
            .iter()
            .all(|node| node.operation != "source/bind")
    );
    compiled.compile_artifact().unwrap();
}

#[test]
fn optional_cells_keep_runtime_dependencies_and_absence() {
    for source in [
        "[1 _]",
        "[signal<f64> _]",
        "[signal<f64> + 1 _]",
        "{1 _}",
        "{signal<f64> _}",
    ] {
        let compiled = compile(source);
        let element = match output(&compiled) {
            SchemaBody::Matrix { element, .. } | SchemaBody::Set { element, .. } => element,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            element.as_ref(),
            &SchemaBody::Option(Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)))
        );
        assert!(!compiled.program().nodes.iter().any(|node| {
            node.operation()
                .expect("ordinary operation fixture")
                .canonical_name()
                == "source/empty"
        }));
        if source.contains("signal") {
            assert_eq!(compiled.program().inputs.len(), 1);
            let conversion = compiled
                .program()
                .nodes
                .iter()
                .find(|node| {
                    node.operation()
                        .expect("ordinary operation fixture")
                        .canonical_name()
                        == "option/some"
                })
                .unwrap();
            assert!(!matches!(conversion.inputs[0], SourceValue::Constant(_)));
        }
        compiled.compile_artifact().unwrap();
    }
}

#[test]
fn recursive_definition_annotations_and_typed_dimensions_are_checked() {
    for source in [
        "x<[f64]:1,2> := [1 2]",
        "x<(u8,bool)> := (1u8,true)",
        "x<{f64}> := {1 2}",
        "x<{u8:bool}> := {1u8:true}",
        "x<{a<f64>,b<bool>}> := {a:1,b:true}",
        "x<|a<f64>|> := |a<f64>|1|",
    ] {
        CanonicalSourceFrontend
            .compile_definition(&definition(source))
            .unwrap_or_else(|error| panic!("{source}: {error}"));
    }
    for source in ["x<[f64]:2,2> := [1 2]", "x<[u8]:300u8> := [1u8]"] {
        let error = CanonicalSourceFrontend
            .compile_definition(&definition(source))
            .err()
            .expect(source);
        assert_eq!(error.anchor.document, DocumentId(0x544));
        assert_eq!(error.anchor.revision, Revision(1));
        if source.contains("300u8") {
            assert_eq!(error.code, "source-semantics/unsupported-kind-dimension");
            assert_eq!(
                &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
                "300u8"
            );
        } else {
            assert_eq!(error.code, "source-semantics/incompatible-definition-kind");
            assert_eq!(error.anchor.range.start.0, 0);
            assert_eq!(error.anchor.range.end.0 as usize, source.len());
        }
    }
    for source in [
        "<[u8]:255u8>",
        "<[u8]:127<i8>>",
        "<[u8]:0xff>",
        "<[u8]:1_000u16>",
    ] {
        compile(source);
    }
    for source in [
        "<[u8]:300u8>",
        "<[u8]:128<i8>>",
        "<[u8]:18446744073709551616u128>",
    ] {
        assert!(
            CanonicalSourceFrontend
                .compile_expression(&expression(source))
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn dynamic_range_peers_are_resolved_in_every_endpoint_position() {
    for (source, ordinal, expected_values, operation) in [
        ("limit..10", 0, vec![None, Some(10.0)], "range/exclusive"),
        ("1..limit", 1, vec![Some(1.0), None], "range/exclusive"),
        (
            "limit..2..10",
            0,
            vec![None, Some(2.0), Some(10.0)],
            "range/exclusive-increment",
        ),
        (
            "1..limit..10",
            1,
            vec![Some(1.0), None, Some(10.0)],
            "range/exclusive-increment",
        ),
        (
            "1..2..limit",
            2,
            vec![Some(1.0), Some(2.0), None],
            "range/exclusive-increment",
        ),
        ("limit..=10", 0, vec![None, Some(10.0)], "range/inclusive"),
        (
            "1..limit..=10",
            1,
            vec![Some(1.0), None, Some(10.0)],
            "range/inclusive-increment",
        ),
    ] {
        let compiled = compile(source);
        assert!(
            matches!(output(&compiled), SchemaBody::Matrix { element, .. } if element.as_ref() == &SchemaBody::FloatingPoint(FloatWidth::W64))
        );
        let range = compiled.program().nodes.last().unwrap();
        assert_eq!(
            range
                .operation()
                .expect("ordinary operation fixture")
                .canonical_name(),
            operation
        );
        assert_eq!(range.inputs[ordinal], SourceValue::Input(0));
        assert_eq!(
            compiled
                .schemas()
                .get(compiled.program().inputs[0].schema)
                .unwrap()
                .body(),
            &SchemaBody::FloatingPoint(FloatWidth::W64)
        );
        assert!(compiled.program().nodes.iter().all(|node| {
            node.operation()
                .expect("ordinary operation fixture")
                .canonical_name()
                != "convert/kind"
        }));
        for (input, expected) in range.inputs.iter().zip(expected_values) {
            if let Some(expected) = expected {
                let SourceValue::Constant(id) = input else {
                    panic!("{source}")
                };
                assert!(
                    matches!(compiled.constants().get(*id).unwrap().data(), mech_core::ValueData::F64(value) if value.to_f64() == expected)
                );
            }
        }
        let contract = compiled.contracts().last().unwrap().as_ref().unwrap();
        assert!(
            matches!(&contract.outputs[0].construction, OutputConstruction::Build { postcondition } if postcondition.contract_name == format!("{}-output", operation.strip_prefix("range/").unwrap()))
        );
        compiled.compile_artifact().unwrap();
    }
    for source in ["start..end", "true..10", "1..true..10", "1..limit<*>"] {
        assert!(
            CanonicalSourceFrontend
                .compile_expression(&expression(source))
                .is_err(),
            "{source}"
        );
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn inferred_range_input_preserves_resident_completion_boundary_and_dynamic_cannot_escape() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentKernelBindError};
    use mech_engine::__resident::{ActivationFacts, ResidentActivationError, activate};
    let compiled = compile("1..limit");
    let artifact = compiled.compile_artifact().unwrap();
    let output = &artifact.outputs()[0];
    let mut facts = ActivationFacts::default();
    let shape = artifact
        .schemas()
        .get(output.schema)
        .unwrap()
        .instantiate_shape(vec![3].into_boxed_slice())
        .unwrap();
    for slot in artifact
        .slots()
        .iter()
        .filter(|slot| slot.schema == output.schema)
    {
        facts.slot_shapes.insert(slot.slot, shape.clone());
    }
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    // Existing resident contract requires range cardinality to be activation-fixed.
    // Supplying shape facts does not turn a live endpoint into an immutable value.
    assert!(
        matches!(activate(ReactiveInstanceId::new(0x549,0), &artifact, &catalog.build().unwrap(), &facts),
        Err(ResidentActivationError::KernelBind { node, error: ResidentKernelBindError::UnsupportedLayout }) if node == mech_core::NodeId(0))
    );
    let source = "1..limit<*>";
    let error = CanonicalSourceFrontend
        .compile_expression(&expression(source))
        .err()
        .unwrap();
    assert_eq!(
        error.code,
        "source-semantics/unsupported-dynamic-conversion"
    );
    assert_eq!(error.anchor.document, DocumentId(0x544));
    assert_eq!(error.anchor.revision, Revision(1));
    assert_eq!(error.anchor.range.start.0, 0);
    assert_eq!(error.anchor.range.end.0 as usize, source.len());
}

#[cfg(feature = "resident-artifact")]
#[test]
fn corrected_contracts_activate_and_compute_resident_values() {
    use mech_core::snapshot::SequenceView;
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueData};
    use mech_engine::__resident::{ActivationFacts, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, expected) in [
        ("1..4", vec![1.0, 2.0, 3.0]),
        ("1..2..=5", vec![1.0, 3.0, 5.0]),
        ("[1 2]", vec![1.0, 2.0]),
        ("[1 2] ** [3; 4]", vec![11.0]),
        ("[1 0; 0 1] \\ [2; 3]", vec![2.0, 3.0]),
    ] {
        let compiled = compile(source);
        let artifact = compiled.compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x544, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        instance
            .turn(&[])
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let value = instance.copied_output(0).unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("{source}: {:?}", value.data())
        };
        let SequenceView::F64(values) = matrix.elements() else {
            panic!()
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn input_interfaces_transport_source_identity_without_resolving_contexts() {
    let names = ["x", "@ctx/path", "Δ", "mech-source-input-78"];
    let mut encoded = std::collections::BTreeSet::new();
    for name in names {
        let compiled = compile(name);
        assert_eq!(compiled.program().inputs[0].name, name);
        assert_eq!(compiled.source_map().inputs[0].document, DocumentId(0x544));
        let artifact = compiled.compile_artifact().unwrap();
        let transport = compiled.artifact_input_name(0).unwrap();
        assert_eq!(artifact.inputs()[0].name, transport);
        assert_eq!(
            mech_engine::decode_source_input_name(&transport).as_deref(),
            Some(name)
        );
        assert!(encoded.insert(transport));
    }
}

#[test]
fn recursive_conformance_retains_dimensions_and_independent_nested_parameters() {
    let matrix = CanonicalSourceFrontend
        .compile_definition(&definition("x<[f64]:1,2> := [1 2]"))
        .unwrap();
    assert!(
        matches!(output(&matrix), SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [mech_core::DimensionExpr::Constant(1),mech_core::DimensionExpr::Constant(2)])
    );
    let tuple = CanonicalSourceFrontend
        .compile_definition(&definition("x<(*,*)> := ((1..end-a<f64>),(2..end-b<f64>))"))
        .unwrap();
    let schema = tuple
        .schemas()
        .get(tuple.program().outputs[0].schema)
        .unwrap();
    assert_eq!(schema.dimension_parameters().len(), 2);
    let SchemaBody::Tuple(items) = schema.body() else {
        panic!()
    };
    let dimensions = items
        .iter()
        .map(|body| match body {
            SchemaBody::Matrix { dimensions, .. } => dimensions[1].clone(),
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    assert_ne!(dimensions[0], dimensions[1]);
    assert!(
        dimensions
            .iter()
            .all(|dimension| matches!(dimension, mech_core::DimensionExpr::Parameter(_)))
    );
}

#[test]
fn selected_operation_contracts_travel_with_conversions_and_exact_schemas() {
    use mech_core::{ChangeDetectionPolicy, FloatWidth};
    for (source, operation, shape) in [
        ("x<u8> + 2u16", "math/add", ShapeRule::Declared),
        ("math/atan2(1,2)", "math/atan2", ShapeRule::Declared),
        (
            "math/sin(1)",
            "math/sin",
            ShapeRule::SameAsInput { input: 0 },
        ),
        (
            "math/neg([1 2])",
            "math/neg",
            ShapeRule::SameAsInput { input: 0 },
        ),
        ("logic/not(true)", "logic/not", ShapeRule::Declared),
        (
            "matrix/transpose([1 2])",
            "matrix/transpose",
            ShapeRule::TransposeOf { input: 0 },
        ),
    ] {
        let compiled = compile(source);
        let (index, node) = compiled
            .program()
            .nodes
            .iter()
            .enumerate()
            .find(|(_, node)| {
                node.operation()
                    .expect("ordinary operation fixture")
                    .canonical_name()
                    == operation
            })
            .unwrap();
        let SourceNodeOutput::Derived { schema } = node.outputs[0] else {
            panic!("expected derived output");
        };
        let output = compiled.schemas().get(schema).unwrap().body();
        let matrix = matches!(output, SchemaBody::Matrix { .. });
        let contract = compiled.contracts()[index]
            .as_ref()
            .expect("selected operation has a contract");
        assert_eq!(
            contract,
            &mech_core::maintained_operation_contract(operation, node.inputs.len(), matrix)
                .unwrap()
        );
        assert_eq!(
            contract.outputs[0].construction,
            OutputConstruction::FullWrite { shape }
        );
        assert_eq!(
            contract.outputs[0].change_detection,
            if matrix || operation == "matrix/transpose" {
                ChangeDetectionPolicy::KernelReported
            } else {
                ChangeDetectionPolicy::ExactScalar
            }
        );
        if operation == "math/atan2" {
            assert_eq!(output, &SchemaBody::FloatingPoint(FloatWidth::W64));
        }
        if source == "x<u8> + 2u16" {
            assert_eq!(output, &SchemaBody::UnsignedInteger(IntegerWidth::W16));
            let SourceValue::NodeOutput {
                node: conversion, ..
            } = node.inputs[0]
            else {
                panic!("the selected promotion must survive handoff");
            };
            assert_eq!(
                compiled.program().nodes[conversion as usize]
                    .operation()
                    .expect("ordinary operation fixture")
                    .canonical_name(),
                "convert/kind"
            );
            assert!(compiled.contracts()[conversion as usize].is_some());
        }
        compiled.compile_artifact().unwrap();
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn shared_math_contracts_reach_resident_results() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueData};
    use mech_engine::__resident::{ActivationFacts, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, expected) in [
        ("math/atan2(1,1)", std::f64::consts::FRAC_PI_4),
        ("math/sin(0)", 0.0),
        ("1 + 2 * 3", 7.0),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x551, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::F64(actual) = output.data() else {
            panic!("{source}: {:?}", output.data());
        };
        assert!((actual.to_f64() - expected).abs() < 1e-12, "{source}");
    }
}

#[test]
fn unresolved_empty_and_unknown_functions_are_source_diagnostics() {
    for (source, code, offending) in [
        ("_", "source-semantics/unresolved-empty-expression", "_"),
        ("(_)", "source-semantics/unresolved-empty-expression", "_"),
        ("1 + _", "source-semantics/unresolved-empty-expression", "_"),
        ("-_", "source-semantics/unresolved-empty-expression", "_"),
        ("!_", "source-semantics/unresolved-empty-expression", "_"),
        (
            "math/sin(_)",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        (
            "(1, _)",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        (
            "{a: _}",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        (
            "{_: 1}",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        ("foo(1)", "source-semantics/unknown-function", "foo"),
        (
            "math/not-declared(1)",
            "source-semantics/unknown-function",
            "math/not-declared",
        ),
    ] {
        let error = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .err()
            .expect(source);
        assert_eq!(error.code, code, "{source}");
        assert_eq!(error.anchor.document, DocumentId(0x544));
        assert_eq!(error.anchor.revision, Revision(1));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            offending,
            "{source}"
        );
    }
    let error = CanonicalSourceFrontend
        .compile_definition(&definition("x := _"))
        .err()
        .expect("an untyped binding cannot retain unresolved empty");
    assert_eq!(error.code, "source-semantics/unresolved-empty-expression");
    assert_eq!(
        error.anchor.range,
        mech_syntax::document::TextRange::new(
            mech_syntax::document::TextSize(5),
            mech_syntax::document::TextSize(6)
        )
    );
}

#[test]
fn contextual_empty_resolves_only_to_exact_optional_constants() {
    use mech_core::ValueData;
    let programs = [
        CanonicalSourceFrontend
            .compile_definition(&definition("x<u8?> := _"))
            .unwrap(),
        CanonicalSourceFrontend
            .compile_definition(&definition("x<(u8,bool)?> := _"))
            .unwrap(),
        CanonicalSourceFrontend
            .compile_expression(&expression("_<(u8,bool)?>"))
            .unwrap(),
    ];
    for (index, program) in programs.iter().enumerate() {
        assert!(
            program.program().nodes.is_empty(),
            "resolving absence creates no operation node"
        );
        let SourceValue::Constant(id) = program.program().outputs[0].source else {
            panic!("absence must become a typed constant");
        };
        assert!(matches!(
            program.constants().get(id).unwrap().data(),
            ValueData::Option(None)
        ));
        let SchemaBody::Option(payload) = output(program) else {
            panic!("absence requires an Option schema");
        };
        if index == 0 {
            assert!(matches!(
                payload.as_ref(),
                SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8)
            ));
        } else {
            assert!(
                matches!(payload.as_ref(), SchemaBody::Tuple(items) if items.as_ref() == [SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8), SchemaBody::Bool])
            );
        }
        program
            .compile_artifact()
            .expect("resolved optional absence must compile");
    }
    for source in ["x<u8> := _", "x<*> := _"] {
        let error = CanonicalSourceFrontend
            .compile_definition(&definition(source))
            .err()
            .expect(source);
        assert_eq!(
            error.code, "source-semantics/unresolved-empty-expression",
            "{source}"
        );
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            "_"
        );
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn resolved_optional_absence_executes_without_empty_operation_nodes() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueData};
    use mech_engine::resident::{ActivationFacts, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for source in ["_<u8?>", "_<(u8,bool)?>", "[_ 1u8]", "[(_) 1u8; 2u8 (_)]"] {
        let program = compile(source);
        assert!(
            program
                .source_map()
                .nodes
                .iter()
                .all(|node| node.operation != "source/empty")
        );
        let artifact = program.compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x548, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            let value = instance.copied_output(0).unwrap();
            if source.starts_with('_') {
                assert!(matches!(value.data(), ValueData::Option(None)), "{source}");
            } else {
                let ValueData::Matrix(matrix) = value.data() else {
                    panic!("{source}: {:?}", value.data());
                };
                let values = matrix.elements().to_values();
                if source == "[_ 1u8]" {
                    assert!(
                        matches!(values.as_slice(), [ValueData::Option(None), ValueData::Option(Some(one))] if matches!(one.as_ref(), ValueData::U8(1)))
                    );
                } else {
                    assert!(
                        matches!(values.as_slice(), [ValueData::Option(None), ValueData::Option(Some(one)), ValueData::Option(Some(two)), ValueData::Option(None)] if matches!(one.as_ref(), ValueData::U8(1)) && matches!(two.as_ref(), ValueData::U8(2)))
                    );
                }
            }
        }
    }
}

#[test]
fn latest_review_hexadecimal_suffix_digits_and_large_rational_reductions_compile() {
    for (source, expected) in [
        ("0xf64", mech_core::ValueDataDraft::I64(0xf64)),
        ("0xf32", mech_core::ValueDataDraft::I64(0xf32)),
        ("0xc32", mech_core::ValueDataDraft::I64(0xc32)),
        ("0xc64", mech_core::ValueDataDraft::I64(0xc64)),
        (
            "12f64",
            mech_core::ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(12.0)),
        ),
        (
            "170141183460469231731687303715884105728/170141183460469231731687303715884105728",
            mech_core::ValueDataDraft::Rational64 {
                numerator: 1,
                denominator: 1,
            },
        ),
    ] {
        let compiled = compile(source);
        let artifact = compiled.compile_artifact().unwrap();
        let SourceValue::Constant(id) = compiled.program().outputs[0].source else {
            panic!("{source}")
        };
        let value = artifact.constants().get(id).unwrap();
        assert_eq!(value.canonical_data_draft().unwrap(), expected, "{source}");
    }
}

#[test]
fn latest_review_id_annotations_use_the_canonical_id_schema() {
    let compiled = compile("signal<id>");
    assert_eq!(output(&compiled), &SchemaBody::Id);
    compiled.compile_artifact().unwrap();
    let compiled = CanonicalSourceFrontend
        .compile_definition(&definition("x<id> := signal<id>"))
        .unwrap();
    assert_eq!(output(&compiled), &SchemaBody::Id);
    compiled.compile_artifact().unwrap();
}

#[test]
fn nested_numeric_suffixes_require_a_known_kind() {
    for (source, start, end) in [("1.0e3units", 4, 10), ("1/0xf64", 2, 7)] {
        let error = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .err()
            .unwrap();
        assert_eq!(
            error.code,
            "source-semantics/unsupported-number-kind-suffix"
        );
        assert_eq!(error.anchor.document, DocumentId(0x544));
        assert_eq!(error.anchor.revision, Revision(1));
        assert_eq!(error.anchor.range.start.0, start);
        assert_eq!(error.anchor.range.end.0, end);
    }
}

#[test]
fn one_axis_select_all_has_a_declared_linear_gather_contract_in_every_profile() {
    let compiled = compile("(signal<[f64]:2,3>, signal[:])");
    let gather = compiled
        .program()
        .nodes
        .iter()
        .position(|node| {
            node.operation()
                .expect("ordinary source operation")
                .canonical_name()
                == "access/range"
        })
        .unwrap();
    assert_eq!(compiled.program().nodes[gather].inputs.len(), 1);
    assert_eq!(
        compiled.contracts()[gather].as_ref().unwrap().outputs[0].construction,
        OutputConstruction::FullWrite {
            shape: ShapeRule::Declared
        }
    );
    let SchemaBody::Tuple(items) = output(&compiled) else {
        panic!()
    };
    let SchemaBody::Matrix { dimensions, .. } = &items[1] else {
        panic!()
    };
    assert_eq!(
        dimensions.as_ref(),
        &[
            mech_core::DimensionExpr::Constant(6),
            mech_core::DimensionExpr::Constant(1)
        ]
    );
    compiled.compile_artifact().unwrap();
}

#[cfg(feature = "resident-artifact")]
#[test]
fn select_all_preserves_foreign_dynamic_payloads_after_artifact_roundtrip() {
    use mech_core::snapshot::{
        CompositeSnapshotConstructor, F64Bits, SequenceView, SnapshotValidationContext, ValueData,
        ValueDataDraft as D, ValueDraft,
    };
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, SchemaDraft,
        SchemaTableBuilder,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let original = compile("(payload<*>, signal<[*]:2,2>, signal[:])")
        .compile_artifact()
        .unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut builder = SchemaTableBuilder::new();
    let insert = |builder: &mut SchemaTableBuilder, body| {
        builder
            .insert(
                SchemaDraft {
                    body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
    };
    let dynamic = insert(&mut builder, SchemaBody::Dynamic);
    let tuple = insert(
        &mut builder,
        SchemaBody::Tuple(
            vec![SchemaBody::Bool, SchemaBody::FloatingPoint(FloatWidth::W64)].into_boxed_slice(),
        ),
    );
    let built = builder.finish().unwrap();
    let (dynamic, tuple) = (
        built.resolve(dynamic).unwrap(),
        built.resolve(tuple).unwrap(),
    );
    let (foreign, _) = built.into_parts();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [&original, &decoded] {
        assert!(
            artifact
                .schemas()
                .find_by_key(foreign.entry(tuple).unwrap().key())
                .is_none()
        );
        let dynamic_schema = artifact
            .schemas()
            .find_by_key(foreign.entry(dynamic).unwrap().key())
            .unwrap();
        let matrix_input = artifact
            .inputs()
            .iter()
            .position(|input| {
                matches!(
                    artifact.schemas().get(input.schema).unwrap().body(),
                    SchemaBody::Matrix { .. }
                )
            })
            .unwrap();
        let matrix_schema = artifact.inputs()[matrix_input].schema;
        let mut instance = activate(
            ReactiveInstanceId::new(0x55d, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 2);
        for start in [7., 19.] {
            let values = (0..4)
                .map(|index| {
                    ValueDraft {
                        schema: dynamic,
                        shape_values: Box::new([]),
                        data: D::Dynamic(Some(Box::new(ValueDraft {
                            schema: tuple,
                            shape_values: Box::new([]),
                            data: D::Tuple(
                                vec![
                                    D::Bool(true),
                                    D::F64(F64Bits::from_f64(start + f64::from(index))),
                                ]
                                .into_boxed_slice(),
                            ),
                        }))),
                    }
                    .finalize(&SnapshotValidationContext::new(&foreign))
                    .unwrap()
                })
                .collect::<Vec<_>>();
            let children = values
                .iter()
                .map(|value| (dynamic_schema, value.shape().clone()))
                .collect::<Vec<_>>();
            let constructor = CompositeSnapshotConstructor::bind(
                matrix_schema,
                artifact
                    .schemas()
                    .get(matrix_schema)
                    .unwrap()
                    .instantiate_shape(Box::new([]))
                    .unwrap(),
                &children,
                std::sync::Arc::new(artifact.schemas().clone()),
            )
            .unwrap();
            let payload = [Some(values[0].clone())];
            let matrix = [Some(
                constructor
                    .construct(values.into_boxed_slice(), None)
                    .unwrap(),
            )];
            let inputs = instance
                .plan
                .inputs
                .iter()
                .enumerate()
                .map(|(index, input)| CapturedSignalInput {
                    slot: input.slot,
                    value: ResidentValueRef::Snapshot(if index == matrix_input {
                        &matrix
                    } else {
                        &payload
                    }),
                })
                .collect::<Vec<_>>();
            instance.turn(&inputs).unwrap();
            let output = instance.copied_output(0).unwrap();
            let SchemaBody::Tuple(schema_items) = output
                .schemas()
                .unwrap()
                .get(output.schema())
                .unwrap()
                .closed_body(output.shape())
                .unwrap()
            else {
                panic!()
            };
            assert!(
                matches!(&schema_items[2], SchemaBody::Matrix { element, dimensions } if element.as_ref() == &SchemaBody::Dynamic && dimensions.as_ref() == [mech_core::DimensionExpr::Constant(4), mech_core::DimensionExpr::Constant(1)])
            );
            let ValueData::Tuple(items) = output.data() else {
                panic!()
            };
            let ValueData::Matrix(matrix) = &items[2] else {
                panic!()
            };
            let SequenceView::Values(values) = matrix.elements() else {
                panic!()
            };
            assert_eq!(values.len(), 4);
            for (value, offset) in values.iter().zip([0., 2., 1., 3.]) {
                let ValueData::Dynamic(dynamic) = value else {
                    panic!()
                };
                let payload = dynamic.value().unwrap();
                assert_eq!(payload.schema_key(), foreign.entry(tuple).unwrap().key());
                let ValueData::Tuple(items) = payload.data() else {
                    panic!()
                };
                assert!(
                    matches!(items.as_ref(), [ValueData::Bool(true), ValueData::F64(value)] if value.to_f64() == start + offset)
                );
            }
        }
    }
}

#[test]
fn whole_string_select_all_preserves_its_source_schema() {
    let compiled = compile("(signal<string>, signal[:])");
    assert!(!compiled.program().nodes.iter().any(|node| {
        node.operation()
            .expect("ordinary source operation")
            .canonical_name()
            == "access/range"
    }));
    assert!(
        matches!(output(&compiled), SchemaBody::Tuple(items) if items.as_ref() == [SchemaBody::String, SchemaBody::String])
    );
    compiled.compile_artifact().unwrap();
}

#[cfg(feature = "resident-artifact")]
#[test]
fn whole_string_select_all_preserves_changed_unicode_and_empty_values() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let original = compile("(signal<string>, signal[:])")
        .compile_artifact()
        .unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x55e, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        for text in ["", "a👩🏽‍💻e\u{301}", "changed"] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::String(&[text.to_owned()]),
                }])
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::Tuple(items) = output.data() else {
                panic!()
            };
            assert_eq!(items.len(), 2);
            for value in items {
                assert!(matches!(value, ValueData::String(value) if value.as_ref() == text));
            }
        }
    }
}

#[test]
fn contextual_composite_empty_rejects_incompatible_values_at_their_source() {
    for (source, code, offending) in [
        (
            "x<(f64,{missing<u8?>})> := (1,{missing:\"bad\"})",
            "source-semantics/incompatible-record-field-kind",
            "\"bad\"",
        ),
        (
            "x<[u8?]:1,2> := [_ true]",
            "source-semantics/incompatible-matrix-element-kind",
            "true",
        ),
        (
            "x<{missing<u8>}> := {missing:_}",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        (
            "x<({missing<u8?>},u8)> := ({missing:_},true)",
            "source-semantics/incompatible-tuple-item-kind",
            "true",
        ),
        (
            "x<{u8:{missing<u8?>}}> := {1u8:{missing:\"bad\"}}",
            "source-semantics/incompatible-record-field-kind",
            "\"bad\"",
        ),
        (
            "x := (|value<{missing<u8?>}>|{missing:\"bad\"}|)",
            "source-semantics/incompatible-record-field-kind",
            "\"bad\"",
        ),
        (
            "x<*> := {missing:_}",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        (
            "x<(u8,u8)> := (1u8,_)",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        (
            "x<u8?> := (1u8 + _)",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        (
            "x<[u8?]:2,2> := [_ _]",
            "source-semantics/incompatible-definition-kind",
            "x<[u8?]:2,2> := [_ _]",
        ),
    ] {
        let error = CanonicalSourceFrontend
            .compile_definition(&definition(source))
            .err()
            .expect(source);
        assert_eq!(error.code, code, "{source}: {error:?}");
        assert_eq!(error.anchor.document, DocumentId(0x544));
        assert_eq!(error.anchor.revision, Revision(1));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            offending,
            "{source}: {error:?}"
        );
    }
}

#[test]
fn contextual_all_empty_matrix_blocks_receive_element_expectations() {
    let cases = [
        ("x<[u8?]:1,4> := [[_ _] [_ _]]", 1, 4),
        ("x<[u8?]:2,2> := [[_ _]; [_ _]]", 2, 2),
        ("x<[u8?]:2,4> := [[[_ _] [_ _]]; [[_ _] [_ _]]]", 2, 4),
        ("x<[u8?]:1,4> := [([_ _]) ([_ _])]", 1, 4),
    ];
    let mut failures = Vec::new();
    for (source, rows, columns) in cases {
        match CanonicalSourceFrontend.compile_definition(&definition(source)) {
            Ok(program) => {
                assert_eq!(
                    output(&program),
                    &SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Option(Box::new(
                            SchemaBody::UnsignedInteger(IntegerWidth::W8)
                        ))),
                        dimensions: vec![
                            mech_core::DimensionExpr::Constant(rows),
                            mech_core::DimensionExpr::Constant(columns)
                        ]
                        .into_boxed_slice(),
                    },
                    "{source}"
                );
                program.compile_artifact().unwrap();
            }
            Err(error) => failures.push((source, error)),
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn c32_arithmetic_has_portable_source_schemas_and_maintained_contracts() {
    for (source, operation, matrix) in [
        ("1<c32> + 2<c32>", "math/add", false),
        ("math/add(1<c32>,2<c32>)", "math/add", false),
        ("1<c32> - 2<c32>", "math/sub", false),
        ("1<c32> * 2<c32>", "math/mul", false),
        ("1<c32> / 2<c32>", "math/div", false),
        ("1<c32> ^ 2<c32>", "math/pow", false),
        ("-(signal<c32>)", "math/neg", false),
        ("math/neg(1<c32>)", "math/neg", false),
        ("left<[c32]:1,2> + right<[c32]:1,2>", "math/add", true),
        ("-(signal<[c32]:1,2>)", "math/neg", true),
    ] {
        let compiled = compile(source);
        let expected = if matrix {
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::Complex(FloatWidth::W32)),
                dimensions: vec![
                    mech_core::DimensionExpr::Constant(1),
                    mech_core::DimensionExpr::Constant(2),
                ]
                .into_boxed_slice(),
            }
        } else {
            SchemaBody::Complex(FloatWidth::W32)
        };
        assert_eq!(output(&compiled), &expected, "{source}");
        let node = compiled.program().nodes.last().unwrap();
        assert_eq!(
            node.operation()
                .expect("ordinary source operation")
                .canonical_name(),
            operation
        );
        assert_eq!(
            compiled.contracts().last().unwrap().as_ref().unwrap(),
            &mech_core::maintained_operation_contract(operation, node.inputs.len(), matrix)
                .unwrap()
        );
        let artifact = compiled.compile_artifact().unwrap();
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
        assert_eq!(
            mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
            encoded
        );
        for artifact in [&artifact, &decoded] {
            assert_eq!(
                artifact
                    .schemas()
                    .get(artifact.outputs()[0].schema)
                    .unwrap()
                    .body(),
                &expected
            );
            assert_eq!(
                artifact
                    .nodes()
                    .last()
                    .unwrap()
                    .as_operation()
                    .unwrap()
                    .operation
                    .canonical_name(),
                operation
            );
        }
    }
}

#[test]
fn c32_invalid_kind_combinations_remain_source_semantic_errors() {
    for (source, code) in [
        ("1<c32> % 2<c32>", "source-semantics/invalid-modulus-kind"),
        (
            "1<c32> < 2<c32>",
            "source-semantics/incompatible-comparison-kinds",
        ),
        (
            "1<c32> + true",
            "source-semantics/non-numeric-arithmetic-kind",
        ),
    ] {
        let error = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .err()
            .expect("invalid operands must retain a source semantic error");
        assert_eq!(error.code, code, "{source}: {error}");
    }
}

#[cfg(all(feature = "resident-artifact", feature = "full_source"))]
#[test]
fn c32_arithmetic_availability_is_a_resident_target_capability() {
    use mech_core::{ExecutionTarget, FunctionCatalogBuilder, NodeId, ResidentKernelBindError};
    use mech_engine::resident::{
        ActivationFacts, ResidentActivationError, ResidentActivationOptions, preflight_activation,
        preflight_resident_target,
    };
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    for (source, operation) in [
        ("1<c32> + 2<c32>", "math/add"),
        ("math/add(1<c32>,2<c32>)", "math/add"),
        ("1<c32> - 2<c32>", "math/sub"),
        ("1<c32> * 2<c32>", "math/mul"),
        ("1<c32> / 2<c32>", "math/div"),
        ("1<c32> ^ 2<c32>", "math/pow"),
        ("-(signal<c32>)", "math/neg"),
        ("math/neg(1<c32>)", "math/neg"),
        ("left<[c32]:1,2> + right<[c32]:1,2>", "math/add"),
        ("-(signal<[c32]:1,2>)", "math/neg"),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        for artifact in [&artifact, &decoded] {
            let node = NodeId::new((artifact.nodes().len() - 1) as u32);
            let binding = preflight_activation(
                artifact,
                &catalog,
                &ActivationFacts::default(),
                ResidentActivationOptions::default(),
            )
            .unwrap_err();
            assert_eq!(
                binding,
                ResidentActivationError::KernelBind {
                    node,
                    // Multiplication's final row-form fallback reports its
                    // contract mismatch after rejecting the scalar layout.
                    error: if operation == "math/mul" {
                        ResidentKernelBindError::UnsupportedContract
                    } else {
                        ResidentKernelBindError::UnsupportedLayout
                    }
                },
                "{source}"
            );
            let capability = preflight_resident_target(
                artifact,
                &catalog,
                &ActivationFacts::default(),
                ResidentActivationOptions::default(),
            )
            .unwrap_err();
            assert_eq!(capability.target, ExecutionTarget::ResidentCpu);
            assert_eq!(capability.node, Some(node));
            assert_eq!(capability.operation.unwrap().canonical_name(), operation);
            assert_eq!(capability.reason, format!("{binding:?}"));
        }
    }
}

#[cfg(all(feature = "resident-artifact", feature = "full_source"))]
#[test]
fn resident_supports_c32_literal_storage_and_c64_arithmetic_execution() {
    use mech_core::snapshot::{Complex64Bits, F64Bits, SnapshotValidationContext};
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData, ValueDataDraft,
        ValueDraft,
    };
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentActivationOptions, activate,
        preflight_resident_target,
    };
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    for (source, literal, scale, offset) in [
        ("1+2i<c32>", true, 1.0, 0.0),
        ("signal<c64> + 1<c64>", false, 1.0, 1.0),
        ("signal<c64> * 2<c64>", false, 2.0, 0.0),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
        for artifact in [&artifact, &decoded] {
            preflight_resident_target(
                artifact,
                &catalog,
                &ActivationFacts::default(),
                ResidentActivationOptions::default(),
            )
            .unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x57c, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            assert_eq!(instance.plan.inputs.len(), usize::from(!literal));
            for number in [3.0, 5.0] {
                let value = if literal {
                    None
                } else {
                    Some(
                        ValueDraft {
                            schema: artifact.inputs()[0].schema,
                            shape_values: Box::new([]),
                            data: ValueDataDraft::Complex64(Complex64Bits::new(
                                F64Bits::from_f64(number),
                                F64Bits::from_f64(2.0),
                            )),
                        }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                        .unwrap(),
                    )
                };
                let values = [value];
                let captured = if literal {
                    vec![]
                } else {
                    vec![CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::Snapshot(&values),
                    }]
                };
                instance.turn(&captured).unwrap();
                let result = instance.copied_output(0).unwrap();
                if literal {
                    assert!(
                        matches!(result.data(), ValueData::Complex32(value) if value.real().to_f32() == 1.0 && value.imaginary().to_f32() == 2.0)
                    );
                } else {
                    assert!(
                        matches!(result.data(), ValueData::Complex64(value) if value.real().to_f64() == number * scale + offset && value.imaginary().to_f64() == 2.0 * scale)
                    );
                }
            }
        }
    }
}

#[test]
fn document_subset_never_silently_discards_an_unsupported_semantic_unit() {
    use mech_syntax::document::{DocumentSyntax, GreenBuilder, IdGenerator};
    let first = expression("1");
    let later = expression("#machine(left: 1, 2) ~> :next -> :ready => :value");
    let unsupported = find(later.syntax().clone(), SyntaxKind::FsmPipe).unwrap();
    let mut ids = IdGenerator::default();
    let mut builder = GreenBuilder::new(&mut ids);
    builder.start_node(SyntaxKind::Document);
    builder.start_node(SyntaxKind::Body);
    builder.reuse_node(first.syntax().green().clone()).unwrap();
    builder.token(SyntaxKind::Newline, "\n").unwrap();
    builder.reuse_node(unsupported.green().clone()).unwrap();
    builder.finish_node().unwrap();
    builder.finish_node().unwrap();
    let source = "1\n#machine(left: 1, 2) ~> :next -> :ready => :value";
    let document = DocumentSyntax::cast(SyntaxNode::new_root(
        builder.finish().unwrap(),
        TextSnapshot::new(DocumentId(822), Revision(4), source).unwrap(),
    ))
    .unwrap();
    let error = CanonicalSourceFrontend
        .compile_document(&document)
        .err()
        .expect("unsupported document unit must be rejected");
    assert_eq!(error.code, "source-semantics/unsupported-document-unit");
    assert_eq!(error.anchor.document, DocumentId(822));
    assert_eq!(error.anchor.revision, Revision(4));
    assert_eq!(error.anchor.range.start.0, 2);
    assert_eq!(error.anchor.range.end.0 as usize, source.len());
}

#[test]
fn named_arguments_reject_unknown_duplicate_missing_and_undeclared_bindings() {
    for (source, code) in [
        (
            "math/sub(unknown: 10, unknown: 3)",
            "source-semantics/unknown-call-argument",
        ),
        (
            "math/sub(left: 10, left: 3)",
            "source-semantics/duplicate-call-argument",
        ),
        (
            "math/sub(10, left: 3)",
            "source-semantics/duplicate-call-argument",
        ),
        (
            "math/sub(right: 3)",
            "source-semantics/missing-call-argument",
        ),
        (
            "math/sub(left: 10, 3, 4)",
            "source-semantics/too-many-call-arguments",
        ),
        (
            "math/sin(value: 1)",
            "source-semantics/named-arguments-unavailable",
        ),
    ] {
        let error = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .err()
            .expect(source);
        assert_eq!(error.code, code, "{source}: {error}");
    }
}

#[test]
fn review_simple_string_escapes_preserve_canonical_values() {
    use mech_core::ValueData;
    for (source, expected) in [
        (r#""\a\!\u""#, "a!u"),
        (r#""\n\t\r\0\\\"""#, "\n\t\r\0\\\""),
        // Emoji are ordinary UTF-8 string content, not members of the
        // alpha/symbol/punctuation `escaped-char` production.
        (r#""\é👩🏽‍💻""#, "é👩🏽‍💻"),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert!((0..decoded.constants().len()).filter_map(|index| decoded.constants().get(mech_core::ConstantId::new(index as u32))).any(|value| matches!(value.data(), ValueData::String(value) if value.as_ref() == expected)), "{source}");
    }
}

#[test]
fn review_atom_annotations_keep_exact_nominal_identity() {
    for source in [":ready<:ready>", ":ready<:ready?>", ":ready<*?>"] {
        let compiled = compile(source);
        let expected = output(&compiled);
        assert!(
            matches!(expected, SchemaBody::Atom(_) | SchemaBody::Option(_)),
            "{source}: {expected:?}"
        );
        let artifact = compiled.compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert_eq!(
            mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
            bytes
        );
    }
    for source in [":ready<:other>", ":ready<:other?>", ":ready<u8>"] {
        let error = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .err()
            .unwrap();
        assert_eq!(
            error.code, "source-semantics/incompatible-literal-kind",
            "{source}"
        );
    }
}

#[test]
fn review_duplicate_definitions_fail_at_the_second_name() {
    use mech_syntax::document::{DocumentSyntax, GreenBuilder, IdGenerator};
    for second in ["x := 2", "~x := 2", "x<u8> := 2"] {
        let first = definition("x := 1");
        let later = definition(second);
        let mut ids = IdGenerator::default();
        let mut builder = GreenBuilder::new(&mut ids);
        builder.start_node(SyntaxKind::Document);
        builder.start_node(SyntaxKind::Body);
        builder.reuse_node(first.syntax().green().clone()).unwrap();
        builder.token(SyntaxKind::Newline, "\n").unwrap();
        builder.reuse_node(later.syntax().green().clone()).unwrap();
        builder.finish_node().unwrap();
        builder.finish_node().unwrap();
        let source = format!("x := 1\n{second}");
        let document = DocumentSyntax::cast(SyntaxNode::new_root(
            builder.finish().unwrap(),
            TextSnapshot::new(DocumentId(822), Revision(6), source.as_str()).unwrap(),
        ))
        .unwrap();
        let error = CanonicalSourceFrontend
            .compile_document(&document)
            .err()
            .unwrap();
        assert_eq!(error.code, "source-semantics/variable-already-defined");
        assert_eq!(error.anchor.document, DocumentId(822));
        assert!(error.anchor.range.start.0 >= 7);
    }
}

#[test]
fn review_remaining_set_calls_have_provider_independent_contracts() {
    for source in [
        "set/subset({1}, {1,2})",
        "{1} ⊂ {1,2}",
        "set/superset({1,2}, {1})",
        "set/proper-superset({1,2}, {1})",
        "set/disjoint({1}, {2})",
        "set/equals({1}, {1})",
        "{1} ≠ {2}",
        "set/element-of(1, {1,2})",
        "set/not-element-of(3, {1,2})",
        "set/powerset({1,2})",
        "set/size({1,2})",
        "set/insert({1}, 2)",
        "set/remove({1,2}, 1)",
    ] {
        let artifact = compile(source)
            .compile_artifact()
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert_eq!(
            mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
            bytes
        );
        #[cfg(feature = "resident-artifact")]
        {
            let mut catalog = mech_core::FunctionCatalogBuilder::new();
            mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
            let mut instance = mech_engine::__resident::activate(
                mech_core::ReactiveInstanceId::new(822, 0),
                &decoded,
                &catalog.build().unwrap(),
                &mech_engine::__resident::ActivationFacts::default(),
            )
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            instance.turn(&[]).unwrap();
            assert!(instance.copied_output(0).is_ok());
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn review_optional_matrix_blocks_and_compound_sets_execute() {
    use mech_core::{
        ValueDataDraft as Data,
        snapshot::{F64Bits, OptionDraft},
    };
    let some = |value| {
        Data::Option(OptionDraft {
            present: true,
            value: Some(Box::new(value)),
        })
    };
    let absent = Data::Option(OptionDraft {
        present: false,
        value: None,
    });
    let f = |value| Data::F64(F64Bits::from_f64(value));
    for (source, expected) in [
        (
            "[(1..3) _]",
            Data::Matrix(vec![some(f(1.)), some(f(2.)), absent.clone()].into_boxed_slice()),
        ),
        (
            "[_ (1..3)]",
            Data::Matrix(vec![absent.clone(), some(f(1.)), some(f(2.))].into_boxed_slice()),
        ),
        (
            "[(1..3)' ; _]",
            Data::Matrix(vec![some(f(1.)), some(f(2.)), absent.clone()].into_boxed_slice()),
        ),
        (
            "[(1..3) _; (4..6) _]",
            Data::Matrix(
                vec![
                    some(f(1.)),
                    some(f(2.)),
                    absent.clone(),
                    some(f(4.)),
                    some(f(5.)),
                    absent.clone(),
                ]
                .into_boxed_slice(),
            ),
        ),
        (
            "{(1u8, 2u8) _}",
            Data::Set(
                vec![
                    absent.clone(),
                    some(Data::Tuple(
                        vec![Data::U8(1), Data::U8(2)].into_boxed_slice(),
                    )),
                ]
                .into_boxed_slice(),
            ),
        ),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        let mut catalog = mech_core::FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = mech_engine::__resident::activate(
            mech_core::ReactiveInstanceId::new(822, 0),
            &decoded,
            &catalog.build().unwrap(),
            &mech_engine::__resident::ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            assert_eq!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected,
                "{source}"
            );
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn review_live_optional_matrix_blocks_preserve_rectangular_order() {
    use mech_core::{
        ResidentValueRef, ValueDataDraft as Data,
        snapshot::{F64Bits, OptionDraft},
    };
    let source = "[signal<[f64]:2,2>; _ _]";
    let artifact = compile(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = mech_engine::__resident::activate(
        mech_core::ReactiveInstanceId::new(822, 0),
        &decoded,
        &catalog.build().unwrap(),
        &mech_engine::__resident::ActivationFacts::default(),
    )
    .unwrap();
    for offset in [0., 10.] {
        let input = [1. + offset, 3. + offset, 2. + offset, 4. + offset];
        instance
            .turn(&[mech_engine::__resident::CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::F64(&input),
            }])
            .unwrap();
        let mut expected = (1..=4)
            .map(|n| {
                Data::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(Data::F64(F64Bits::from_f64(
                        f64::from(n) + offset,
                    )))),
                })
            })
            .collect::<Vec<_>>();
        expected.extend([
            Data::Option(OptionDraft {
                present: false,
                value: None,
            }),
            Data::Option(OptionDraft {
                present: false,
                value: None,
            }),
        ]);
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Matrix(expected.into_boxed_slice())
        );
    }
}

#[test]
fn review_scalar_booleans_are_not_logical_masks() {
    for source in [
        "(a<[f64]:1,2>, a[true])",
        "(a<[f64]:1,1>, a[false])",
        "(a<string>, a[true])",
        "(a<[f64]:1,2>, a[true,:])",
    ] {
        let error = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .err()
            .expect(source);
        assert_eq!(
            error.code, "source-semantics/incompatible-selection-kind",
            "{source}: {error}"
        );
        assert_eq!(error.anchor.document, DocumentId(0x544));
        assert_eq!(error.anchor.revision, Revision(1));
    }
    compile("(a<{bool:f64}>, a[true])")
        .compile_artifact()
        .unwrap();
    compile("(a<[f64]:1,2>, a[[true false]])")
        .compile_artifact()
        .unwrap();
}

#[cfg(feature = "resident-artifact")]
#[test]
fn review_annotated_empty_map_roundtrips_and_executes() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueData};
    use mech_engine::resident::{ActivationFacts, activate};
    let source = "x<{u8:bool}> := {:}";
    let compiled = CanonicalSourceFrontend
        .compile_definition(&definition(source))
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(822, 61),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    for _ in 0..2 {
        instance.turn(&[]).unwrap();
        let output = instance.copied_output(0).unwrap();
        assert!(matches!(output.data(), ValueData::Map(map) if map.entries().is_empty()));
    }
    let error = CanonicalSourceFrontend
        .compile_expression(&expression("(1, {:})"))
        .err()
        .unwrap();
    assert_eq!(error.code, "source-semantics/unresolved-map-entry-kind");
    assert_eq!(
        &"(1, {:})"[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
        "{:}"
    );
}

#[test]
fn review_empty_maps_enforce_the_same_keyability_as_populated_maps() {
    for (kind, populated) in [("c32", "{1<c32>: true}"), ("c64", "{1+2i: true}")] {
        let source = format!("x<{{{kind}:bool}}> := {{:}}");
        let error = CanonicalSourceFrontend
            .compile_definition(&definition(&source))
            .err()
            .expect("complex map key must be rejected before emission");
        let populated_error = CanonicalSourceFrontend
            .compile_expression(&expression(populated))
            .err()
            .expect("populated map uses the same keyability boundary");
        assert_eq!(error.code, "source-semantics/non-keyable-map-key-kind");
        assert_eq!(populated_error.code, error.code);
        assert_eq!(error.anchor.document, DocumentId(0x544));
        assert_eq!(error.anchor.revision, Revision(1));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            "{:}"
        );
    }
    for kind in ["u8", "bool", "string"] {
        let source = format!("x<{{{kind}:c64}}> := {{:}}");
        CanonicalSourceFrontend
            .compile_definition(&definition(&source))
            .unwrap()
            .compile_artifact()
            .unwrap();
    }
}

#[test]
fn review_signed_minima_decode_before_positive_magnitude_admission() {
    use mech_core::ValueDataDraft as Data;
    for (source, expected) in [
        ("-128<i8>", Data::I8(i8::MIN)),
        ("-(128<i8>)", Data::I8(i8::MIN)),
        ("-0x80<i8>", Data::I8(i8::MIN)),
        ("-32768<i16>", Data::I16(i16::MIN)),
        ("-2147483648<i32>", Data::I32(i32::MIN)),
        ("-9223372036854775808<i64>", Data::I64(i64::MIN)),
        (
            "-170141183460469231731687303715884105728<i128>",
            Data::I128(i128::MIN),
        ),
    ] {
        let compiled = compile(source);
        let SourceValue::Constant(id) = compiled.program().outputs[0].source else {
            panic!("{source}")
        };
        let artifact = compiled.compile_artifact().unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        assert_eq!(
            decoded
                .constants()
                .get(id)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected,
            "{source}"
        );
    }
    for source in ["-129<i8>", "-170141183460469231731687303715884105729<i128>"] {
        assert_eq!(
            CanonicalSourceFrontend
                .compile_expression(&expression(source))
                .err()
                .unwrap()
                .code,
            "source-semantics/invalid-number-literal"
        );
    }
}

#[test]
fn review_complex_separators_distinguish_hex_digits_from_decimal_exponents() {
    use mech_core::ValueData;
    for (source, real, imaginary) in [
        ("0xE-2i", 14.0, -2.0),
        ("0xe+2i", 14.0, 2.0),
        ("0xFE-2i", 254.0, -2.0),
    ] {
        let compiled = compile(source);
        let SourceValue::Constant(id) = compiled.program().outputs[0].source else {
            panic!("{source}")
        };
        let artifact = compiled.compile_artifact().unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        let ValueData::Complex64(value) = decoded.constants().get(id).unwrap().data() else {
            panic!("{source}")
        };
        assert_eq!(value.real().to_f64(), real, "{source}");
        assert_eq!(value.imaginary().to_f64(), imaginary, "{source}");
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn review_annotated_empty_sets_execute_and_share_keyability_admission() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueData};
    use mech_engine::resident::{ActivationFacts, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for kind in ["u8", "bool", "string"] {
        let source = format!("x<{{{kind}}}> := {{}}");
        let artifact = CanonicalSourceFrontend
            .compile_definition(&definition(&source))
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 65),
            &decoded,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            assert!(
                matches!(instance.copied_output(0).unwrap().data(), ValueData::Set(items) if items.elements().is_empty())
            );
        }
    }
    for kind in ["c32", "c64"] {
        let source = format!("x<{{{kind}}}> := {{}}");
        let error = CanonicalSourceFrontend
            .compile_definition(&definition(&source))
            .err()
            .unwrap();
        assert_eq!(error.code, "source-semantics/non-keyable-set-element-kind");
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            "{}"
        );
    }
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("(1, {})"))
            .err()
            .unwrap()
            .code,
        "source-semantics/unresolved-set-element-kind"
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn review_maintained_calls_infer_peers_after_argument_binding_and_execute() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, scale, offset) in [
        ("math/add(signal, 1)", 1.0, 1.0),
        ("math/add(1, signal)", 1.0, 1.0),
        ("math/sub(left: signal, right: 10)", 1.0, -10.0),
        ("math/sub(right: signal, left: 10)", -1.0, 10.0),
    ] {
        let compiled = compile(source);
        assert_eq!(
            compiled
                .schemas()
                .get(compiled.program().inputs[0].schema)
                .unwrap()
                .body(),
            &SchemaBody::FloatingPoint(FloatWidth::W64)
        );
        let artifact = compiled.compile_artifact().unwrap();
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 66),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for value in [3.0, -7.0, 11.0] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&[value]),
                }])
                .unwrap();
            assert!(
                matches!(instance.copied_output(0).unwrap().data(), ValueData::F64(result) if result.to_f64() == value * scale + offset),
                "{source}"
            );
        }
    }
    for source in [
        "math/add(signal<*>, 1)",
        "math/add(left, right)",
        "matrix/solve(signal, [1; 2])",
    ] {
        assert!(
            CanonicalSourceFrontend
                .compile_expression(&expression(source))
                .is_err(),
            "{source}"
        );
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn review_all_set_relations_infer_either_peer_and_execute() {
    use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData, ValueDataDraft,
        ValueDraft,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (name, equal_result) in [("equals", true), ("disjoint", false)] {
        for arguments in ["signal, {1}", "{1}, signal"] {
            let source = format!("set/{name}({arguments})");
            let compiled = compile(&source);
            let artifact = compiled.compile_artifact().unwrap();
            let artifact = mech_engine::decode_program_artifact_bytecode_v1(
                &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
            )
            .unwrap();
            let schema = artifact.inputs()[0].schema;
            assert!(
                matches!(artifact.schemas().get(schema).unwrap().body(), SchemaBody::Set { element, .. } if element.as_ref() == &SchemaBody::FloatingPoint(FloatWidth::W64))
            );
            let mut instance = activate(
                ReactiveInstanceId::new(822, 67),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            for (number, expected) in [
                (1.0, equal_result),
                (2.0, !equal_result),
                (1.0, equal_result),
            ] {
                let input = ValueDraft {
                    schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::Set(
                        vec![ValueDataDraft::F64(F64Bits::from_f64(number))].into_boxed_slice(),
                    ),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                instance
                    .turn(&[CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::Snapshot(&[Some(input)]),
                    }])
                    .unwrap();
                assert!(
                    matches!(instance.copied_output(0).unwrap().data(), ValueData::Bool(actual) if *actual == expected),
                    "{source}, {number}"
                );
            }
        }
        for (arguments, code) in [
            (
                "signal<*>, {1}",
                "source-semantics/unsupported-dynamic-conversion",
            ),
            ("signal, other", "source-semantics/unresolved-call-kind"),
        ] {
            let source = format!("set/{name}({arguments})");
            assert_eq!(
                CanonicalSourceFrontend
                    .compile_expression(&expression(&source))
                    .err()
                    .unwrap()
                    .code,
                code,
                "{source}"
            );
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn review_stats_and_combinatorics_contracts_roundtrip_and_execute() {
    use mech_core::snapshot::F64Bits;
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft as Data,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let f = |number| Data::F64(F64Bits::from_f64(number));
    // Rectangular input makes the two reduction axes observably different.
    for (source, inputs, expected) in [
        (
            "stats/sum/column(signal<[f64]:2,3>)",
            vec![
                vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
                vec![2.0, 8.0, 4.0, 10.0, 6.0, 12.0],
            ],
            vec![
                Data::Matrix(vec![f(6.0), f(15.0)].into_boxed_slice()),
                Data::Matrix(vec![f(12.0), f(30.0)].into_boxed_slice()),
            ],
        ),
        (
            "stats/sum/row(signal<[f64]:2,3>)",
            vec![
                vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
                vec![2.0, 8.0, 4.0, 10.0, 6.0, 12.0],
            ],
            vec![
                Data::Matrix(vec![f(5.0), f(7.0), f(9.0)].into_boxed_slice()),
                Data::Matrix(vec![f(10.0), f(14.0), f(18.0)].into_boxed_slice()),
            ],
        ),
        (
            "combinatorics/n-choose-k(signal<f64>,2)",
            vec![vec![4.0], vec![5.0]],
            vec![f(6.0), f(10.0)],
        ),
        (
            "combinatorics/n-choose-k([1 2 3],2)",
            vec![vec![], vec![]],
            vec![
                Data::Matrix(
                    vec![f(1.0), f(1.0), f(2.0), f(2.0), f(3.0), f(3.0)].into_boxed_slice()
                );
                2
            ],
        ),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 68),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        for (input, expected) in inputs.iter().zip(expected) {
            let captured = instance
                .plan
                .inputs
                .iter()
                .map(|port| CapturedSignalInput {
                    slot: port.slot,
                    value: ResidentValueRef::F64(input),
                })
                .collect::<Vec<_>>();
            instance
                .turn(&captured)
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            assert_eq!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected,
                "{source}"
            );
        }
    }
    for (name, arity) in [
        ("stats/sum/column", 1),
        ("stats/sum/row", 1),
        ("combinatorics/n-choose-k", 2),
    ] {
        for matrix in [false, true] {
            assert!(mech_core::maintained_operation_contract(name, arity - 1, matrix).is_none());
            assert!(mech_core::maintained_operation_contract(name, arity + 1, matrix).is_none());
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn review_numeric_peer_inference_uses_the_maintained_math_family() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, expected) in [
        ("math/copysign(signal, -1)", [-3.0, -0.5]),
        ("math/fdim(signal, 1)", [2.0, 0.0]),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 69),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for (input, expected) in [3.0, 0.5].into_iter().zip(expected) {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&[input]),
                }])
                .unwrap();
            assert!(
                matches!(instance.copied_output(0).unwrap().data(), ValueData::F64(actual) if actual.to_f64() == expected),
                "{source}: {input}"
            );
        }
    }
}

#[test]
fn review_ordered_extrema_infer_their_peer_at_the_artifact_boundary() {
    for source in ["compare/min(signal, 1)", "compare/max(1, signal)"] {
        let artifact = compile(source).compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert_eq!(artifact.revision(), decoded.revision());
        assert_eq!(
            artifact
                .schemas()
                .get(artifact.inputs()[0].schema)
                .unwrap()
                .body(),
            &SchemaBody::FloatingPoint(FloatWidth::W64)
        );
    }
}

#[test]
fn fixed_matrix_snapshots_bind_to_inferred_input_dimensions() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_core::{DimensionExpr, SchemaDraft, SchemaTableBuilder};
    for (rows, columns) in [(2, 2), (1, 4), (3, 1)] {
        let mut table = SchemaTableBuilder::new();
        let handle = table
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                        dimensions: vec![
                            DimensionExpr::Constant(rows),
                            DimensionExpr::Constant(columns),
                        ]
                        .into_boxed_slice(),
                    },
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = table.finish().unwrap();
        let schema = built.resolve(handle).unwrap();
        let (table, _) = built.into_parts();
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                (0..rows * columns)
                    .map(ValueDataDraft::U64)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&table))
        .unwrap();
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression("matrix<[u64]>"))
            .unwrap()
            .bind_input_constants(&[(0, value)])
            .unwrap();
        assert!(compiled.program().inputs.is_empty());
        let SourceValue::Constant(id) = compiled.program().outputs[0].source else {
            panic!("bound input must become constant");
        };
        let value = compiled.constants().get(id).unwrap();
        assert_eq!(value.shape().parameter_values(), &[rows, columns]);
    }
}
