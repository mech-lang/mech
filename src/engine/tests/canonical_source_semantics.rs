#![cfg(feature = "source")]

use std::fs;
use std::path::PathBuf;

use mech_core::{
    ChangeDetectionPolicy, IntegerWidth, OutputConstruction, SchemaBody, ShapeRule, ValueData,
};
use mech_engine::{
    CanonicalSourceFrontend, PHASE_2I_SEMANTIC_RULES, Phase2iSemanticDisposition,
    SourceSemanticComprehensionQualifierRole, SourceValue, phase_2i_semantic_disposition,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSize,
    TextSnapshot, VariableDefineSyntax,
};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x540), Revision(4), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("canonical Expression")
}

fn recovered_expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x540), Revision(4), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(!parsed.is_strictly_clean(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("recovered Expression")
}

fn definition(source: &str) -> VariableDefineSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x540), Revision(4), source).unwrap(),
        rules::VARIABLE_DEFINE,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::VariableDefine)
        .and_then(VariableDefineSyntax::cast)
        .expect("canonical VariableDefine")
}

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

#[test]
fn semantic_policy_covers_the_exact_generated_component() {
    let schema = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-syntax-schema.tsv"),
    )
    .unwrap();
    let names = schema
        .lines()
        .skip(1)
        .map(|line| line.split('\t').next().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(PHASE_2I_SEMANTIC_RULES.len(), 80);
    assert_eq!(
        PHASE_2I_SEMANTIC_RULES
            .iter()
            .map(|rule| rule.grammar_name)
            .collect::<Vec<_>>(),
        names
    );
    assert!(
        PHASE_2I_SEMANTIC_RULES
            .iter()
            .any(|rule| rule.disposition == Phase2iSemanticDisposition::Executable)
    );
    assert_eq!(
        phase_2i_semantic_disposition("kind"),
        Some(Phase2iSemanticDisposition::CompileTime)
    );
    assert_eq!(phase_2i_semantic_disposition("unknown"), None);
}

#[test]
fn typed_expression_builds_source_program_and_preserves_anchors() {
    let expression = expression("1 + 2 * 3");
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression)
        .unwrap();

    assert_eq!(compiled.program().inputs.len(), 0);
    assert_eq!(compiled.program().outputs.len(), 1);
    assert_eq!(compiled.constants().len(), 3);
    assert_eq!(compiled.program().nodes.len(), 2);
    assert_eq!(compiled.contracts().len(), 2);
    assert_eq!(compiled.source_map().nodes.len(), 2);
    assert_eq!(compiled.source_map().nodes[0].operation, "math/mul");
    assert_eq!(compiled.source_map().nodes[1].operation, "math/add");
    assert!(matches!(
        compiled
            .schemas()
            .get(compiled.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
    ));
    assert_eq!(
        compiled.contracts()[1].as_ref().unwrap().outputs[0].change_detection,
        ChangeDetectionPolicy::ExactScalar
    );
    assert_eq!(
        compiled.contracts()[1].as_ref().unwrap().outputs[0].construction,
        mech_core::OutputConstruction::FullWrite {
            shape: ShapeRule::Declared
        }
    );
    assert_eq!(compiled.source_map().outputs[0].document, DocumentId(0x540));
    assert_eq!(compiled.source_map().outputs[0].revision, Revision(4));
    assert_eq!(
        compiled.source_map().outputs[0].range,
        expression.syntax().range()
    );
    compiled
        .compile_artifact()
        .expect("typed source graph must be a canonical artifact input");
}

#[test]
fn identifiers_are_resolved_once_and_reused_as_source_inputs() {
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression("signal + signal"))
        .unwrap();
    assert_eq!(compiled.program().inputs.len(), 1);
    assert_eq!(compiled.program().inputs[0].name, "signal");
    assert_eq!(compiled.program().nodes.len(), 1);
    assert_eq!(
        compiled.program().nodes[0].inputs.as_ref(),
        &[SourceValue::Input(0), SourceValue::Input(0)]
    );
}

#[test]
fn structures_calls_comprehensions_and_fsm_enter_one_source_graph() {
    for (source, final_operation) in [
        ("{a: 1, b: 2}", "source/record"),
        ("{1: 2, 3: 4}", "source/map"),
        ("{1, 2}", "set/define"),
        ("(1, 2)", "source/tuple"),
        ("[1 2]", "source/matrix"),
        ("|a<u8>|1|", "source/table"),
        ("f(left: 1, 2)", "f"),
        ("x[1].field", "access/index"),
        ("1..10", "range/exclusive"),
        ("x ? | * => 1", "source/match"),
        ("[x | x <- xs]", "matrix/comprehension"),
        ("{x | x <- xs}", "set/comprehension"),
        ("#controller() -> :ready", "source/fsm"),
    ] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        assert_eq!(
            compiled.source_map().nodes.last().unwrap().operation,
            final_operation,
            "{source:?}"
        );
        assert_eq!(compiled.program().nodes.len(), compiled.contracts().len());
    }
}

#[test]
fn recovered_trees_never_construct_partial_semantics() {
    let error = CanonicalSourceFrontend
        .compile_expression(&recovered_expression("1 +"))
        .err()
        .expect("recovered syntax must be rejected");
    assert_eq!(error.code, "source-semantics/recovered-syntax");
    assert_eq!(error.anchor.document, DocumentId(0x540));
    assert_eq!(error.anchor.revision, Revision(4));
    assert_eq!(error.anchor.range.start, TextSize(3));
    assert_eq!(error.anchor.range.end, TextSize(3));
}

#[test]
fn calls_ranges_subscripts_and_patterns_keep_their_canonical_roles() {
    let call = CanonicalSourceFrontend
        .compile_expression(&expression("f(left: 1, 2)"))
        .unwrap();
    let node = call.program().nodes.last().unwrap();
    assert_eq!(node.operation.canonical_name(), "f");
    assert_eq!(
        call.source_map().nodes.last().unwrap().detail.as_deref(),
        Some("f(left,)")
    );
    assert!(matches!(
        call.compile_artifact(),
        Err(mech_engine::ArtifactBuildError::MissingOperationContract { operation, .. })
            if operation.canonical_name() == "f"
    ));

    let range = CanonicalSourceFrontend
        .compile_expression(&expression("1..2..=10"))
        .unwrap();
    assert_eq!(
        range.source_map().nodes.last().unwrap().operation,
        "range/inclusive-increment"
    );
    assert!(matches!(
        range.contracts().last().unwrap().as_ref().unwrap().outputs[0].construction,
        OutputConstruction::Build { ref postcondition }
            if postcondition.module_path.as_ref() == ["range"]
                && postcondition.contract_name == "inclusive-increment-output"
    ));
    assert_eq!(
        range.contracts().last().unwrap().as_ref().unwrap().outputs[0].change_detection,
        ChangeDetectionPolicy::KernelReported
    );
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = range
        .schemas()
        .get(range.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("range did not produce a matrix schema")
    };
    assert!(matches!(element.as_ref(), SchemaBody::FloatingPoint(_)));
    assert_eq!(dimensions[0], mech_core::DimensionExpr::Constant(1));
    assert!(matches!(
        dimensions[1],
        mech_core::DimensionExpr::Parameter(_)
    ));
    range
        .compile_artifact()
        .expect("typed range must be a canonical artifact input");

    let slice = CanonicalSourceFrontend
        .compile_expression(&expression("x[1][2]"))
        .unwrap();
    let accesses = slice
        .program()
        .nodes
        .iter()
        .filter(|node| node.operation.canonical_name() == "access/index")
        .collect::<Vec<_>>();
    assert_eq!(accesses.len(), 2);
    assert!(matches!(
        accesses[1].inputs[0],
        SourceValue::NodeOutput {
            node: _,
            output_ordinal: 0
        }
    ));

    let comprehension = CanonicalSourceFrontend
        .compile_expression(&expression("[x | x <- xs]"))
        .unwrap();
    assert_eq!(
        comprehension
            .program()
            .inputs
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>(),
        vec!["xs"]
    );
    assert_eq!(comprehension.source_map().patterns.len(), 1);
    assert_eq!(
        comprehension.source_map().patterns[0].bindings.as_ref(),
        &["x"]
    );
    assert!(comprehension.program().nodes.iter().all(|node| {
        !node
            .operation
            .canonical_name()
            .starts_with("source/pattern")
    }));
    assert_eq!(comprehension.source_map().comprehension_qualifiers.len(), 1);
    assert!(matches!(
        comprehension.source_map().comprehension_qualifiers[0].role,
        SourceSemanticComprehensionQualifierRole::Generator { pattern: 0 }
    ));

    let qualified = CanonicalSourceFrontend
        .compile_expression(&expression("[y | x <- xs, y := x, y > 0]"))
        .unwrap();
    assert_eq!(qualified.source_map().comprehension_qualifiers.len(), 3);
    assert!(matches!(
        qualified.source_map().comprehension_qualifiers[0].role,
        SourceSemanticComprehensionQualifierRole::Generator { pattern: 0 }
    ));
    assert_eq!(
        qualified.source_map().comprehension_qualifiers[1].role,
        SourceSemanticComprehensionQualifierRole::Definition
    );
    assert_eq!(
        qualified.source_map().comprehension_qualifiers[2].role,
        SourceSemanticComprehensionQualifierRole::Filter
    );
    assert_eq!(
        qualified
            .source_map()
            .comprehension_qualifiers
            .iter()
            .map(|qualifier| qualifier.input_ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let destructured = CanonicalSourceFrontend
        .compile_expression(&expression("[a + b | (a, b) <- xs]"))
        .unwrap();
    let bindings = destructured
        .source_map()
        .nodes
        .iter()
        .filter(|node| node.operation == "source/bind")
        .collect::<Vec<_>>();
    assert_eq!(bindings.len(), 2);
    assert_ne!(bindings[0].detail, bindings[1].detail);
    let add = destructured
        .program()
        .nodes
        .iter()
        .find(|node| node.operation.canonical_name() == "math/add")
        .unwrap();
    assert_ne!(add.inputs[0], add.inputs[1]);
}

#[test]
fn canonical_numeric_kinds_annotations_strings_and_state_are_preserved() {
    for (source, expected) in [
        ("1u8", "u8"),
        ("0x10", "hex-i64"),
        ("0d42", "decimal-i64"),
        ("1/2", "r64"),
        ("1+2i", "c64"),
        ("1<u8>", "u8"),
    ] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let SourceValue::Constant(id) = compiled.program().outputs[0].source else {
            panic!("{source:?} did not produce a constant")
        };
        let value = compiled.constants().get(id).unwrap();
        let actual = match value.data() {
            ValueData::U8(1) => "u8",
            ValueData::I64(16) => "hex-i64",
            ValueData::I64(42) => "decimal-i64",
            ValueData::Rational64(value) if value.numerator() == 1 && value.denominator() == 2 => {
                "r64"
            }
            ValueData::Complex64(_) => "c64",
            other => panic!("unexpected value for {source:?}: {other:?}"),
        };
        assert_eq!(actual, expected, "{source:?}");
    }

    let input = CanonicalSourceFrontend
        .compile_expression(&expression("signal<u8>"))
        .unwrap();
    assert!(matches!(
        input
            .schemas()
            .get(input.program().inputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    ));

    let table = CanonicalSourceFrontend
        .compile_expression(&expression("|count<u8>|1u8|"))
        .unwrap();
    let SchemaBody::Table { columns, rows } = table
        .schemas()
        .get(table.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("annotated table did not produce a table schema")
    };
    assert_eq!(columns[0].name, "count");
    assert_eq!(
        columns[0].schema,
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    );
    assert_eq!(
        rows,
        &mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(1))
    );

    let inferred_table = CanonicalSourceFrontend
        .compile_expression(&expression(
            "╭─────────╮\n│ count   │\n├─────────┤\n│   1     │\n╰─────────╯",
        ))
        .unwrap();
    let SchemaBody::Table { columns, .. } = inferred_table
        .schemas()
        .get(inferred_table.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("inferred table did not produce a table schema")
    };
    assert!(matches!(
        columns[0].schema,
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
    ));

    let string = CanonicalSourceFrontend
        .compile_expression(&expression("\"\\0\\u{41}\""))
        .unwrap();
    let SourceValue::Constant(id) = string.program().outputs[0].source else {
        panic!("string did not produce a constant")
    };
    assert!(
        matches!(string.constants().get(id).unwrap().data(), ValueData::String(value) if value.as_ref() == "\0A")
    );

    let state = CanonicalSourceFrontend
        .compile_definition(&definition("~state<u8> := 1"))
        .unwrap();
    assert_eq!(state.program().states.len(), 1);
    assert!(state.program().states[0].initializer.is_some());
    assert_eq!(
        state.program().nodes[0].operation.canonical_name(),
        "core/assign"
    );
    assert_eq!(state.program().outputs[0].source, SourceValue::State(0));
    assert_eq!(state.program().nodes[0].inputs[0], SourceValue::State(0));
    assert!(matches!(
        state
            .schemas()
            .get(state.program().states[0].schema)
            .unwrap()
            .body(),
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    ));
    state
        .compile_artifact()
        .expect("mutable definition has a resolved state contract");

    let defined = CanonicalSourceFrontend
        .compile_definition(&definition("x<u8> := 1"))
        .unwrap();
    let SourceValue::Constant(id) = defined.program().outputs[0].source else {
        panic!("annotated definition did not produce a constant")
    };
    assert!(matches!(
        defined.constants().get(id).unwrap().data(),
        ValueData::U8(1)
    ));

    let optional_input = CanonicalSourceFrontend
        .compile_expression(&expression("signal<u8?>"))
        .unwrap();
    assert!(matches!(
        optional_input
            .schemas()
            .get(optional_input.program().inputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Option(payload)
            if matches!(payload.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
    ));

    let optional_definition = CanonicalSourceFrontend
        .compile_definition(&definition("x<u8?> := 1"))
        .unwrap();
    let SourceValue::Constant(id) = optional_definition.program().outputs[0].source else {
        panic!("optional definition did not produce a constant")
    };
    assert!(matches!(
        optional_definition.constants().get(id).unwrap().data(),
        ValueData::Option(Some(value)) if matches!(value.as_ref(), ValueData::U8(1))
    ));

    let optional_literal = CanonicalSourceFrontend
        .compile_expression(&expression("1<u8?>"))
        .unwrap();
    let SourceValue::Constant(id) = optional_literal.program().outputs[0].source else {
        panic!("optional literal did not produce a constant")
    };
    assert!(matches!(
        optional_literal.constants().get(id).unwrap().data(),
        ValueData::Option(Some(value)) if matches!(value.as_ref(), ValueData::U8(1))
    ));

    let optional_kind = CanonicalSourceFrontend
        .compile_expression(&expression("<u8?>"))
        .unwrap();
    let SourceValue::Constant(id) = optional_kind.program().outputs[0].source else {
        panic!("optional kind did not produce a constant")
    };
    assert!(matches!(
        optional_kind.constants().get(id).unwrap().data(),
        ValueData::Type(_)
    ));

    let constrained_optional = CanonicalSourceFrontend
        .compile_expression(&expression("signal<u8:1..10?>"))
        .unwrap();
    assert!(matches!(
        constrained_optional
            .schemas()
            .get(constrained_optional.program().inputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Option(payload) if matches!(payload.as_ref(), SchemaBody::Dynamic)
    ));

    let promoted = CanonicalSourceFrontend
        .compile_expression(&expression("1u8 + 2u16"))
        .unwrap();
    assert!(matches!(
        promoted
            .schemas()
            .get(promoted.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::UnsignedInteger(IntegerWidth::W16)
    ));
    let add = promoted.program().nodes.last().unwrap();
    assert!(add.inputs.iter().all(|input| {
        let SourceValue::Constant(id) = input else {
            return false;
        };
        matches!(
            promoted.constants().get(*id).unwrap().data(),
            ValueData::U16(_)
        )
    }));

    let strict = CanonicalSourceFrontend
        .compile_expression(&expression("1u8 === 2u16"))
        .unwrap();
    let strict_inputs = &strict.program().nodes.last().unwrap().inputs;
    assert!(matches!(
        strict
            .constants()
            .get(match strict_inputs[0] {
                SourceValue::Constant(id) => id,
                _ => panic!(),
            })
            .unwrap()
            .data(),
        ValueData::U8(1)
    ));
    assert!(matches!(
        strict
            .constants()
            .get(match strict_inputs[1] {
                SourceValue::Constant(id) => id,
                _ => panic!(),
            })
            .unwrap()
            .data(),
        ValueData::U16(2)
    ));

    let rational_power = CanonicalSourceFrontend
        .compile_expression(&expression("1/2 ^ 2<i32>"))
        .unwrap();
    let power_inputs = &rational_power.program().nodes.last().unwrap().inputs;
    assert!(matches!(
        rational_power
            .constants()
            .get(match power_inputs[0] {
                SourceValue::Constant(id) => id,
                _ => panic!(),
            })
            .unwrap()
            .data(),
        ValueData::Rational64(_)
    ));
    assert!(matches!(
        rational_power
            .constants()
            .get(match power_inputs[1] {
                SourceValue::Constant(id) => id,
                _ => panic!(),
            })
            .unwrap()
            .data(),
        ValueData::I32(2)
    ));

    let negated = CanonicalSourceFrontend
        .compile_expression(&expression("-1<i8>"))
        .unwrap();
    assert!(matches!(
        negated
            .contracts()
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .outputs[0]
            .construction,
        OutputConstruction::FullWrite {
            shape: ShapeRule::SameAsInput { input: 0 }
        }
    ));

    for source in ["~state := signal", "~state := 1 + 2"] {
        let error = CanonicalSourceFrontend
            .compile_definition(&definition(source))
            .err()
            .expect("nonconstant state initializer must be rejected");
        assert_eq!(error.code, "source-semantics/nonconstant-state-initializer");
    }

    let overflow = CanonicalSourceFrontend
        .compile_expression(&expression("1e100<f32>"))
        .err()
        .expect("finite f64 values that overflow f32 must be rejected");
    assert_eq!(overflow.code, "source-semantics/invalid-number-literal");

    let atom = CanonicalSourceFrontend
        .compile_expression(&expression(":ready"))
        .unwrap();
    let SourceValue::Constant(id) = atom.program().outputs[0].source else {
        panic!("atom literal did not produce a constant")
    };
    assert!(matches!(
        atom.constants().get(id).unwrap().data(),
        ValueData::Atom
    ));
    assert!(matches!(
        atom.schemas()
            .get(atom.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Atom(_)
    ));

    let kind = CanonicalSourceFrontend
        .compile_expression(&expression("<u8>"))
        .unwrap();
    let SourceValue::Constant(id) = kind.program().outputs[0].source else {
        panic!("kind literal did not produce a constant")
    };
    assert!(matches!(
        kind.constants().get(id).unwrap().data(),
        ValueData::Type(_)
    ));
    assert!(matches!(
        kind.schemas()
            .get(kind.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::ReifiedType
    ));
}

#[test]
fn semantic_kind_edges_are_resolved_before_graph_emission() {
    let dynamic_option = CanonicalSourceFrontend
        .compile_expression(&expression("1<u8:1..10?>"))
        .unwrap();
    let SourceValue::Constant(id) = dynamic_option.program().outputs[0].source else {
        panic!("constrained optional number did not produce a constant")
    };
    assert!(matches!(
        dynamic_option
            .schemas()
            .get(dynamic_option.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Option(payload) if matches!(payload.as_ref(), SchemaBody::Dynamic)
    ));
    assert!(matches!(
        dynamic_option.constants().get(id).unwrap().data(),
        ValueData::Option(Some(value))
            if matches!(value.as_ref(), ValueData::Dynamic(dynamic)
                if matches!(dynamic.value().map(|value| value.data()), Some(ValueData::F64(_))))
    ));
    dynamic_option
        .compile_artifact()
        .expect("a constrained optional number must finalize as a dynamic payload");

    for (source, numerator, denominator) in [("2/4", 1, 2), ("7/7", 1, 1)] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap();
        let SourceValue::Constant(id) = compiled.program().outputs[0].source else {
            panic!("{source:?} did not produce a constant")
        };
        assert!(matches!(
            compiled.constants().get(id).unwrap().data(),
            ValueData::Rational64(value)
                if value.numerator() == numerator && value.denominator() == denominator
        ));
        compiled
            .compile_artifact()
            .expect("reduced rationals must finalize");
    }

    let empty = CanonicalSourceFrontend
        .compile_expression(&expression("_<u8?>"))
        .unwrap();
    let SourceValue::Constant(id) = empty.program().outputs[0].source else {
        panic!("typed empty did not produce a constant")
    };
    assert!(matches!(
        empty.constants().get(id).unwrap().data(),
        ValueData::Option(None)
    ));
    assert!(matches!(
        empty
            .schemas()
            .get(empty.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Option(payload)
            if matches!(payload.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
    ));
    empty
        .compile_artifact()
        .expect("typed empty must be an absent optional constant");

    let late_annotation = CanonicalSourceFrontend
        .compile_expression(&expression("(signal + 1) + signal<u8>"))
        .unwrap();
    assert!(matches!(
        late_annotation
            .schemas()
            .get(late_annotation.program().inputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    ));
    assert!(late_annotation.program().nodes.iter().all(|node| {
        node.outputs.iter().all(|output| match output {
            mech_engine::SourceNodeOutput::Derived { schema } => matches!(
                late_annotation.schemas().get(*schema).unwrap().body(),
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
            ),
            mech_engine::SourceNodeOutput::State(_) => false,
        })
    }));
    late_annotation
        .compile_artifact()
        .expect("input declarations must be resolved before consumer nodes");

    let unsigned_negation = CanonicalSourceFrontend
        .compile_expression(&expression("-1u8"))
        .err()
        .expect("unsigned negation must be rejected");
    assert_eq!(
        unsigned_negation.code,
        "source-semantics/non-negatable-kind"
    );

    let atom_state = CanonicalSourceFrontend
        .compile_definition(&definition("~state := :ready"))
        .unwrap();
    assert!(matches!(
        atom_state
            .schemas()
            .get(atom_state.program().states[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Atom(_)
    ));
    atom_state
        .compile_artifact()
        .expect("an exact atom initializer must retain its state schema");

    let rational_range = CanonicalSourceFrontend
        .compile_expression(&expression("1/2..3/4"))
        .err()
        .expect("rational range endpoints must be rejected");
    assert_eq!(
        rational_range.code,
        "source-semantics/invalid-range-endpoint-kind"
    );
}

#[test]
fn semantic_annotations_follow_value_roles_and_lexical_scope() {
    for source in ["<*>", "<_>"] {
        let kind = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap();
        let SourceValue::Constant(id) = kind.program().outputs[0].source else {
            panic!("{source:?} did not produce a reified kind constant")
        };
        assert!(matches!(
            kind.constants().get(id).unwrap().data(),
            ValueData::Type(_)
        ));
        kind.compile_artifact()
            .expect("wildcard and empty kinds must have canonical reified values");
    }

    let optional_atom = CanonicalSourceFrontend
        .compile_definition(&definition("x<*?> := :ready"))
        .unwrap();
    let SourceValue::Constant(id) = optional_atom.program().outputs[0].source else {
        panic!("optional atom definition did not produce a constant")
    };
    assert!(matches!(
        optional_atom.constants().get(id).unwrap().data(),
        ValueData::Option(Some(value))
            if matches!(value.as_ref(), ValueData::Dynamic(dynamic)
                if matches!(dynamic.value().map(|value| value.data()), Some(ValueData::Atom)))
    ));
    optional_atom
        .compile_artifact()
        .expect("an exact constant must finalize inside a dynamic option payload");

    let local_annotations = CanonicalSourceFrontend
        .compile_expression(&expression("([x | x<u8> := 1], [x | x<u16> := 2])"))
        .unwrap();
    assert!(local_annotations.program().inputs.is_empty());

    let ternary_range = CanonicalSourceFrontend
        .compile_expression(&expression("1..2..limit"))
        .unwrap();
    let range = ternary_range.program().nodes.last().unwrap();
    assert!(matches!(
        range.inputs[2],
        SourceValue::NodeOutput {
            node: _,
            output_ordinal: 0
        }
    ));
    ternary_range
        .compile_artifact()
        .expect("a dynamic third endpoint must be conformed before range emission");

    let invalid_not = CanonicalSourceFrontend
        .compile_expression(&expression("¬1"))
        .err()
        .expect("logical negation of a number must be rejected");
    assert_eq!(
        invalid_not.code,
        "source-semantics/non-boolean-negation-kind"
    );

    let index_range = CanonicalSourceFrontend
        .compile_expression(&expression("lo<ix>..hi<ix>"))
        .unwrap();
    assert!(index_range.program().inputs.iter().all(|input| matches!(
        index_range.schemas().get(input.schema).unwrap().body(),
        SchemaBody::Index
    )));
    let SchemaBody::Matrix { element, .. } = index_range
        .schemas()
        .get(index_range.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("index range did not produce a matrix")
    };
    assert!(matches!(element.as_ref(), SchemaBody::Index));
    index_range
        .compile_artifact()
        .expect("index endpoints must satisfy the range contract");

    let annotated_atom = CanonicalSourceFrontend
        .compile_expression(&expression(":ready<u8>"))
        .err()
        .expect("an incompatible atom annotation must be rejected");
    assert_eq!(
        annotated_atom.code,
        "source-semantics/incompatible-literal-kind"
    );

    let record = CanonicalSourceFrontend
        .compile_expression(&expression("{ x<u8>: 1, y<bool>: true }"))
        .unwrap();
    let SchemaBody::Record(fields) = record
        .schemas()
        .get(record.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("annotated record did not retain a record schema")
    };
    assert!(matches!(
        fields[0].schema,
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    ));
    assert!(matches!(fields[1].schema, SchemaBody::Bool));
}

#[test]
fn match_and_fsm_metadata_preserve_source_argument_layouts() {
    let matched = CanonicalSourceFrontend
        .compile_expression(&expression("x ? | *, true => 1 | * => 2"))
        .unwrap();
    assert_eq!(matched.source_map().match_arms.len(), 2);
    assert_eq!(matched.source_map().match_arms[0].guard_input, Some(1));
    assert_eq!(matched.source_map().match_arms[0].result_input, 2);
    assert_eq!(matched.source_map().match_arms[1].guard_input, None);
    assert_eq!(matched.source_map().match_arms[1].result_input, 3);

    let fsm = CanonicalSourceFrontend
        .compile_expression(&expression("#controller(left: 1, 2) -> :ready"))
        .unwrap();
    assert_eq!(
        fsm.source_map().nodes.last().unwrap().detail.as_deref(),
        Some("controller(left,)")
    );
}

#[test]
fn the_source_semantic_module_has_no_aggregate_program_boundary() {
    let source =
        fs::read_to_string(repository_root().join("src/engine/src/source_semantics/frontend.rs"))
            .unwrap();
    assert!(!source.contains("mech_core::Program"));
    assert!(!source.contains("document::lower"));
    assert!(!source.contains("parser::parse("));
}
