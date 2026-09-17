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
                &node.operation.canonical_name(),
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
                .operation
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
        "x<u8> ? | y => y | * => 0u8",
        "{x | (x, *) <- {(1u8, true)}}",
        "{x | (*, (x, *)) <- {(true, (1u8, false))}}",
    ] {
        let compiled = compile(source);
        let bindings = compiled
            .program()
            .nodes
            .iter()
            .filter(|node| node.operation.canonical_name() == "source/bind")
            .collect::<Vec<_>>();
        assert_eq!(bindings.len(), 1, "{source}");
        let SourceNodeOutput::Derived { schema } = bindings[0].outputs[0] else {
            panic!()
        };
        assert_eq!(
            compiled.schemas().get(schema).unwrap().body(),
            &SchemaBody::UnsignedInteger(IntegerWidth::W8),
            "{source}"
        );
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
    assert_eq!(
        explicit_dynamic.code,
        "source-semantics/unsupported-dynamic-conversion"
    );
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
    let binding = inferred
        .program()
        .nodes
        .iter()
        .find(|node| node.operation.canonical_name() == "source/bind")
        .unwrap();
    let SourceNodeOutput::Derived { schema } = binding.outputs[0] else {
        panic!()
    };
    assert_eq!(
        inferred.schemas().get(schema).unwrap().body(),
        &SchemaBody::FloatingPoint(FloatWidth::W64)
    );
    assert!(
        inferred.compile_artifact().is_err(),
        "the unresolved generator remains an explicit intermediate source/bind boundary"
    );
    let compiled = compile("{x + y | (x, y) <- {(1u8, 2u8)}}");
    let add = compiled
        .program()
        .nodes
        .iter()
        .find(|node| node.operation.canonical_name() == "math/add")
        .unwrap();
    assert_ne!(add.inputs[0], add.inputs[1]);
    assert!(compiled.program().inputs.is_empty());
    let details = compiled
        .source_map()
        .nodes
        .iter()
        .filter(|node| node.operation == "source/bind")
        .map(|node| node.detail.as_deref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        details,
        [
            "pattern=0;binding=0;name=x;path=[0]",
            "pattern=0;binding=1;name=y;path=[1]"
        ]
    );
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
        assert!(
            !compiled
                .program()
                .nodes
                .iter()
                .any(|node| node.operation.canonical_name() == "source/empty")
        );
        if source.contains("signal") {
            assert_eq!(compiled.program().inputs.len(), 1);
            let conversion = compiled
                .program()
                .nodes
                .iter()
                .find(|node| node.operation.canonical_name() == "option/some")
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
        assert_eq!(range.operation.canonical_name(), operation);
        assert_eq!(range.inputs[ordinal], SourceValue::Input(0));
        assert_eq!(
            compiled
                .schemas()
                .get(compiled.program().inputs[0].schema)
                .unwrap()
                .body(),
            &SchemaBody::FloatingPoint(FloatWidth::W64)
        );
        assert!(
            compiled
                .program()
                .nodes
                .iter()
                .all(|node| node.operation.canonical_name() != "convert/kind")
        );
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
        .compile_definition(&definition("x<(*,*)> := ((1..3),(2..4))"))
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
