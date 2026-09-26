#![cfg(feature = "source_default")]

use std::fs;
use std::path::PathBuf;

#[cfg(feature = "resident-artifact")]
use mech_core::snapshot::{ReifiedKind, ReifiedTypeDraft};
use mech_core::{
    CanonicalNominalPath, ChangeDetectionPolicy, IntegerWidth, OutputConstruction, SchemaBody,
    ShapeRule, ValueData,
};
#[cfg(feature = "resident-artifact")]
use mech_core::{
    FunctionCatalogBuilder, KindExpr, ManagedMemoryBudget, ReactiveInstanceId, ResidentValueRef,
    ValueDataDraft,
};
#[cfg(feature = "resident-artifact")]
use mech_engine::__resident::{
    ActivationFacts, CapturedSignalInput, ResidentActivationOptions, activate,
    activate_with_options,
};
use mech_engine::{
    CanonicalSourceFrontend, PHASE_2I_SEMANTIC_RULES, Phase2iSemanticDisposition, SourceValue,
    phase_2i_semantic_disposition,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ExpressionSyntax, ParseConfig, Revision, SyntaxKind,
    SyntaxNode, TextSize, TextSnapshot, VariableDefineSyntax, parse_canonical_document,
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

fn document(source: &str) -> DocumentSyntax {
    let snapshot = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x541), Revision(5), source).unwrap(),
        ParseConfig::default(),
    );
    DocumentSyntax::cast(snapshot.syntax()).expect("canonical Document")
}

fn nominal_origin() -> CanonicalNominalPath {
    CanonicalNominalPath::new(vec!["mech-test".to_owned(), "canonical-source".to_owned()]).unwrap()
}

#[cfg(feature = "resident-artifact")]
fn execute_document<'a>(
    source: &str,
    turns: impl IntoIterator<Item = (Vec<ResidentValueRef<'a>>, ValueDataDraft)>,
) {
    let compiled = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(&document(source), &nominal_origin())
        .unwrap_or_else(|error| panic!("canonical document did not compile: {error:?}"));
    let artifact = compiled.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut source_instance = activate(
        ReactiveInstanceId::new(0x540, 1),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let mut decoded_instance = activate(
        ReactiveInstanceId::new(0x540, 2),
        &decoded,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    for (values, expected) in turns {
        for instance in [&mut source_instance, &mut decoded_instance] {
            assert_eq!(values.len(), instance.plan.inputs.len());
            let captured = values
                .iter()
                .copied()
                .zip(instance.plan.inputs.iter())
                .map(|(value, input)| CapturedSignalInput {
                    slot: input.slot,
                    value,
                })
                .collect::<Vec<_>>();
            instance.turn(&captured).unwrap();
            assert_eq!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected
            );
        }
    }
}

#[test]
fn typed_document_compiles_definition_and_expression_units_in_source_order() {
    let document = document("answer := 40 + 2\nanswer\n");
    let compiled = CanonicalSourceFrontend
        .compile_document(&document)
        .expect("clean canonical document semantics");
    assert_eq!(compiled.program().outputs.len(), 1);
    assert_eq!(compiled.program().nodes.len(), 1);
    assert_eq!(compiled.source_map().nodes[0].operation, "math/add");
    assert_eq!(
        compiled.source_map().outputs[0].document,
        document.syntax().source().document()
    );
    compiled
        .compile_artifact()
        .expect("canonical document produces an artifact");
}

#[test]
fn document_kind_aliases_and_enum_variants_share_the_canonical_type_environment() {
    let alias = CanonicalSourceFrontend
        .compile_document(&document("<count> := <u8>\nx<count> := 1\nx\n"))
        .unwrap();
    assert!(matches!(
        alias
            .schemas()
            .get(alias.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    ));
    alias.compile_artifact().unwrap();

    let shaped_alias = CanonicalSourceFrontend
        .compile_document(&document("<row> := <[f64]>\nx<row> := [1 2]\nx\n"))
        .expect("an open matrix alias specializes at each use");
    shaped_alias.compile_artifact().unwrap();

    for source in [
        "<event> := :idle | :busy\nvalue<*> := :idle\nvalue\n",
        "<event> := :idle | :busy\nvalue<event?> := :idle\nvalue\n",
    ] {
        CanonicalSourceFrontend
            .compile_document_with_nominal_origin(&document(source), &nominal_origin())
            .unwrap_or_else(|error| panic!("permissive enum context {source:?}: {error:?}"))
            .compile_artifact()
            .unwrap();
    }

    let enumeration = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document("<color> := Red | Green\nx := :Red\nx\n"),
            &nominal_origin(),
        )
        .unwrap();
    let schema = enumeration
        .schemas()
        .get(enumeration.program().outputs[0].schema)
        .unwrap();
    let SchemaBody::Enum { variants, .. } = schema.body() else {
        panic!("declared enum output")
    };
    assert_eq!(
        variants
            .iter()
            .map(|variant| variant.name.as_str())
            .collect::<Vec<_>>(),
        ["Red", "Green"]
    );
    let artifact = enumeration.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    assert_eq!(
        mech_engine::decode_program_artifact_bytecode_v1(&bytes)
            .unwrap()
            .revision(),
        artifact.revision()
    );

    let source = document("<color> := :red | :blue\nvalue := :red\nvalue\n");
    let compile_in = |package: &str, module: &str| {
        let origin =
            CanonicalNominalPath::new(vec![package.to_owned(), module.to_owned()]).unwrap();
        let compiled = CanonicalSourceFrontend
            .compile_document_with_nominal_origin(&source, &origin)
            .unwrap();
        let schema = compiled
            .schemas()
            .get(compiled.program().outputs[0].schema)
            .unwrap();
        let SchemaBody::Enum { key, .. } = schema.body() else {
            panic!("declared enum output");
        };
        *key
    };
    assert_ne!(
        compile_in("first-package", "colors"),
        compile_in("second-package", "colors")
    );
    assert_ne!(
        compile_in("first-package", "colors"),
        compile_in("first-package", "other")
    );
    assert_eq!(
        CanonicalSourceFrontend
            .compile_document(&source)
            .err()
            .expect("enum declarations require defining provenance")
            .code,
        "source-semantics/nominal-origin-required"
    );
}

#[test]
fn literal_owned_enum_annotation_conforms_to_optional_kind() {
    let source = "<event> := :idle | :busy\nvalue := :idle<event?>\nvalue\n";
    let compiled = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(&document(source), &nominal_origin())
        .expect("annotated enum atom conforms to its optional kind");
    assert!(matches!(
        compiled
            .schemas()
            .get(compiled.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Option(_)
    ));
    compiled.compile_artifact().unwrap();
}

#[test]
fn contextual_enum_atom_match_pattern_conforms_to_optional_scrutinee() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<event> := :idle | :busy\n\
                 value<event?> := :idle\n\
                 result := value? | :idle => 1 | * => 0.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("the contextual enum atom is wrapped to match the optional scrutinee")
        .compile_artifact()
        .unwrap();
}

#[cfg(feature = "resident-artifact")]
#[test]
fn contextual_enum_atom_match_pattern_executes_after_artifact_roundtrip() {
    execute_document(
        "<event> := :idle | :busy\n\
         value<event?> := :idle\n\
         result := value? | :idle => true | * => false.\n\
         result\n",
        [(vec![], ValueDataDraft::Bool(true))],
    );
}

#[test]
fn unused_invalid_kind_declaration_fails_at_declaration() {
    let error = CanonicalSourceFrontend
        .compile_document(&document("<bad> := <{a<u8>,a<bool>}>\nvalue := 1\nvalue\n"))
        .err()
        .expect("duplicate record fields are invalid even when the alias is unused");
    assert_eq!(error.code, "source-semantics/invalid-kind-declaration");
}

#[test]
fn declared_scalar_aliases_type_literal_values_and_negation() {
    for source in [
        "<count> := <u8>\nx := 1<count>\nx\n",
        "<flag> := <bool>\nx := true<flag>\nx\n",
        "<word> := <string>\nx := \"hi\"<word>\nx\n",
        "<signed> := <i8>\nx := -1<signed>\nx\n",
    ] {
        CanonicalSourceFrontend
            .compile_document(&document(source))
            .unwrap_or_else(|error| panic!("declared literal alias {source:?}: {error:?}"))
            .compile_artifact()
            .unwrap();
    }
}

#[test]
fn declared_scalar_aliases_type_kind_extent_literals() {
    let source = "<count> := <u8>\n<row> := <[f64]:1<count>,3>\n<group> := <{u8}:2<count>>\n<ledger> := <|value<u8>|:2<count>>\nvalue := 1\nvalue\n";
    CanonicalSourceFrontend
        .compile_document(&document(source))
        .expect("declared integer alias is valid in matrix, set, and table extents")
        .compile_artifact()
        .unwrap();
}

#[cfg(feature = "resident-artifact")]
#[test]
fn declared_enum_kind_values_reify_the_nominal_kind() {
    let compiled = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document("<color> := :red | :blue\n<color>\n"),
            &nominal_origin(),
        )
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for program in [&artifact, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x540, 3),
            program,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        let ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(bytes)) = instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap()
        else {
            panic!("declared enum kind must produce a canonical kind value");
        };
        let reified = ReifiedKind::from_canonical_bytes(bytes).unwrap();
        let (kind, dimensions, _) = reified.decoded_closed_kind().unwrap();
        assert!(matches!(kind, KindExpr::Enum(_)));
        assert!(dimensions.is_empty());
    }
}

#[test]
fn contextual_and_qualified_enum_atoms_resolve_exact_nominal_kinds() {
    let contextual = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<first> := :none | :some<f64>\n\
             <second> := :none | :other<f64>\n\
             value<first> := :none\n\
             value\n",
            ),
            &nominal_origin(),
        )
        .unwrap();
    let contextual_schema = contextual
        .schemas()
        .get(contextual.program().outputs[0].schema)
        .unwrap();
    assert!(matches!(contextual_schema.body(), SchemaBody::Enum { .. }));
    contextual.compile_artifact().unwrap();

    let qualified = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document("<color> := :red | :green\nvalue<color> := :color/red\nvalue\n"),
            &nominal_origin(),
        )
        .unwrap();
    let qualified_schema = qualified
        .schemas()
        .get(qualified.program().outputs[0].schema)
        .unwrap();
    assert!(matches!(qualified_schema.body(), SchemaBody::Enum { .. }));
    qualified.compile_artifact().unwrap();
}

#[cfg(feature = "resident-artifact")]
#[test]
fn qualified_nominal_atoms_without_enum_context_retain_their_paths() {
    for source in [":foo/bar", ":foo/bar(1)"] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        compiled.compile_artifact().unwrap();
        let schema = compiled
            .schemas()
            .get(compiled.program().outputs[0].schema)
            .unwrap();
        match source {
            ":foo/bar" => assert!(matches!(schema.body(), SchemaBody::Atom(_))),
            _ => assert!(matches!(schema.body(), SchemaBody::Tuple(_))),
        }
    }

    let compiled = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document("<foo> := <u8>\n:foo/bar\n"),
            &nominal_origin(),
        )
        .unwrap();
    compiled.compile_artifact().unwrap();
    let schema = compiled
        .schemas()
        .get(compiled.program().outputs[0].schema)
        .unwrap();
    assert!(matches!(schema.body(), SchemaBody::Atom(_)));
}

#[cfg(feature = "resident-artifact")]
#[test]
fn payload_free_enum_match_arms_lower_as_nominal_structural_patterns() {
    execute_document(
        "<event> := :idle | :timeout\n\
         value<event> := :timeout\n\
         result := value?\n\
           | :timeout => true\n\
           | * => false.\n\
         result\n",
        [(vec![], ValueDataDraft::Bool(true))],
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn complete_enum_variant_arms_are_exhaustive_without_a_wildcard() {
    execute_document(
        "<event> := :idle | :timeout\n\
         value<event> := :timeout\n\
         result := value?\n\
           | :idle => false\n\
           | :timeout => true.\n\
         result\n",
        [(vec![], ValueDataDraft::Bool(true))],
    );
}

#[test]
fn refutable_enum_payload_arm_does_not_complete_variant_coverage() {
    let source = "<choice> := :some<f64> | :none\n\
                  value<choice> := :choice/none\n\
                  result := value?\n\
                    | :some(0) => 1\n\
                    | :none => 0.\n\
                  result\n";
    let error = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(&document(source), &nominal_origin())
        .err()
        .expect("a payload literal leaves the variant partly uncovered");
    assert_eq!(error.code, "source-semantics/non-exhaustive-match");
}

#[test]
fn narrowed_dynamic_enum_payload_binding_does_not_complete_variant_coverage() {
    let source = "<event> := :data<*>\n\
                  value<event> := :data(true)\n\
                  result := value?\n\
                    | :data(x<f64>) => x.\n\
                  result\n";
    let error = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(&document(source), &nominal_origin())
        .err()
        .expect("a typed binding cannot cover other Dynamic payload schemas");
    assert_eq!(error.code, "source-semantics/non-exhaustive-match");
}

#[test]
fn exact_enum_payload_binding_completes_variant_coverage() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<event> := :data<f64>\n\
                 value<event> := :data(1)\n\
                 result := value? | :data(x) => x.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("an exact payload binding covers the enum variant")
        .compile_artifact()
        .unwrap();
}

#[test]
fn exact_fixed_array_enum_payload_pattern_completes_variant_coverage() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<event> := :data<[f64]:1,2>\n\
                 value<event> := :data([1 2])\n\
                 result := value? | :data([*, *]) => true.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("an exact fixed array payload pattern covers the enum variant")
        .compile_artifact()
        .unwrap();
}

#[test]
fn fixed_array_rest_enum_payload_pattern_completes_variant_coverage() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<event> := :data<[f64]:1,2>\n\
                 value<event> := :data([1 2])\n\
                 result := value? | :data([*, ...]) => true.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("an irrefutable rest payload pattern covers the fixed enum variant")
        .compile_artifact()
        .unwrap();
}

#[test]
fn fixed_array_named_rest_keeps_turn_shape_and_completes_variant_coverage() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<event> := :data<[f64]:1,3>\n\
                 value<event> := :data([1 2 3])\n\
                 result := value? | :data([head | rest]) => head.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("a turn-shaped named rest binding covers the fixed enum payload")
        .compile_artifact()
        .unwrap();
}

#[test]
fn fixed_array_rest_preserves_its_residual_extent_for_nested_patterns() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<event> := :data<[f64]:1,3>\n\
                 value<event> := :data([1 2 3])\n\
                 result := value? | :data([* | [*, *]]) => true.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("the fixed parent determines the nested rest pattern's exact extent")
        .compile_artifact()
        .unwrap();
}

#[test]
fn singleton_enum_payload_pattern_completes_outer_variant_coverage() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<inner> := :only\n\
                 <outer> := :wrap<inner>\n\
                 value<outer> := :wrap(:only)\n\
                 result := value? | :wrap(:only) => true.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("the sole nested enum variant is irrefutable")
        .compile_artifact()
        .unwrap();
}

#[test]
fn enum_payload_coverage_combines_across_match_arms() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<inner> := :left | :right\n\
                 <outer> := :wrap<inner>\n\
                 value<outer> := :wrap(:left)\n\
                 result := value?\n\
                   | :wrap(:left) => 1\n\
                   | :wrap(:right) => 2.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("all nested enum variants collectively cover the outer payload")
        .compile_artifact()
        .unwrap();
}

#[test]
fn boolean_payload_coverage_combines_across_match_arms() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<outer> := :wrap<bool>\n\
                 value<outer> := :wrap(true)\n\
                 result := value?\n\
                   | :wrap(true) => 1\n\
                   | :wrap(false) => 2.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("both Boolean literals collectively cover the outer payload")
        .compile_artifact()
        .unwrap();
}

#[test]
fn tuple_payload_coverage_combines_as_a_finite_product() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<outer> := :wrap<(bool,bool)>\n\
                 value<outer> := :wrap((true,false))\n\
                 result := value?\n\
                   | :wrap((true,*)) => 1\n\
                   | :wrap((false,*)) => 2.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("the tuple arms collectively cover the Boolean product")
        .compile_artifact()
        .unwrap();
}

#[test]
fn tuple_payload_coverage_preserves_field_correlations() {
    let error = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<outer> := :wrap<(bool,bool)>\n\
                 value<outer> := :wrap((true,false))\n\
                 result := value?\n\
                   | :wrap((true,true)) => 1\n\
                   | :wrap((false,false)) => 2.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .err()
        .expect("diagonal tuple cases leave two Boolean combinations uncovered");
    assert_eq!(error.code, "source-semantics/non-exhaustive-match");
}

#[test]
fn fixed_matrix_payload_coverage_combines_as_a_finite_product() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<outer> := :wrap<[bool]:1,1>\n\
                 value<outer> := :wrap([true])\n\
                 result := value?\n\
                   | :wrap([true]) => 1\n\
                   | :wrap([false]) => 2.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("the fixed matrix arms collectively cover the Boolean product")
        .compile_artifact()
        .unwrap();
}

#[test]
fn fixed_matrix_rest_coverage_combines_without_enumerating_the_tail() {
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<outer> := :wrap<[bool]:1,2>\n\
                 value<outer> := :wrap([true false])\n\
                 result := value?\n\
                   | :wrap([true, ...]) => 1\n\
                   | :wrap([false, ...]) => 2.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .expect("the fixed matrix prefix cases cover every finite tail")
        .compile_artifact()
        .unwrap();
}

#[test]
fn fixed_matrix_payload_coverage_preserves_element_correlations() {
    let error = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<outer> := :wrap<[bool]:1,2>\n\
                 value<outer> := :wrap([true false])\n\
                 result := value?\n\
                   | :wrap([true true]) => 1\n\
                   | :wrap([false false]) => 2.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .err()
        .expect("diagonal matrix cases leave two Boolean sequences uncovered");
    assert_eq!(error.code, "source-semantics/non-exhaustive-match");
}

#[cfg(feature = "resident-artifact")]
#[test]
fn optional_enum_payload_pattern_binds_after_artifact_roundtrip() {
    execute_document(
        "<event> := :data<f64> | :idle\n\
         value<event?> := :data(3)\n\
         result := value? | :data(x) => x | * => 0.\n\
         result\n",
        [(
            vec![],
            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(3.0)),
        )],
    );
    for value in [":idle", "_"] {
        execute_document(
            &format!(
                "<event> := :data<f64> | :idle\n\
                 value<event?> := {value}\n\
                 result := value? | :data(x) => x | * => 0.\n\
                 result\n"
            ),
            [(
                vec![],
                ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(0.0)),
            )],
        );
    }
}

#[test]
fn partial_finite_payload_coverage_remains_non_exhaustive() {
    let error = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<inner> := :left | :right\n\
                 <outer> := :wrap<inner>\n\
                 value<outer> := :wrap(:left)\n\
                 result := value? | :wrap(:left) => 1.\n\
                 result\n",
            ),
            &nominal_origin(),
        )
        .err()
        .expect("one nested enum variant leaves the outer payload uncovered");
    assert_eq!(error.code, "source-semantics/non-exhaustive-match");
}

#[test]
fn bare_enum_comprehension_pattern_uses_generator_element_schema() {
    let source = "<first> := :idle | :busy\n\
                  <second> := :idle | :done\n\
                  values := [:first/idle :first/busy]\n\
                  result := [true | :idle <- values]\n\
                  result\n";
    CanonicalSourceFrontend
        .compile_document_with_nominal_origin(&document(source), &nominal_origin())
        .expect("the generator element selects the first enum's idle variant")
        .compile_artifact()
        .unwrap();
}

#[test]
fn declared_annotations_in_comprehension_binding_prepasses_use_the_document_environment() {
    CanonicalSourceFrontend
        .compile_document(&document(
            "<count> := <u8>\n\
             values := [1u8 2u8]\n\
             result := [x | x<count> <- values]\n\
             result\n",
        ))
        .unwrap()
        .compile_artifact()
        .unwrap();
}

#[test]
fn enum_payload_patterns_retain_nominal_identity_through_bytecode() {
    let compiled = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(
            &document(
                "<color> := :red<f64> | :green<f64>\n\
             my-color<color> := :red(300)\n\
             result := my-color?\n\
               | :red(x), x > 100 => x\n\
               | * => 0.\n\
             result\n",
            ),
            &nominal_origin(),
        )
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let match_node = artifact
        .nodes()
        .iter()
        .find_map(|node| match &node.body {
            mech_engine::ExecutableNodeBody::Match(control) => Some(control),
            _ => None,
        })
        .expect("enum source retains canonical match control");
    assert!(matches!(
        &match_node.arms[0].pattern,
        mech_engine::MatchPattern::Structural(mech_engine::CollectionPattern::Enum {
            ordinal: 0,
            payload: Some(_),
        })
    ));
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let decoded_match = decoded
        .nodes()
        .iter()
        .find_map(|node| match &node.body {
            mech_engine::ExecutableNodeBody::Match(control) => Some(control),
            _ => None,
        })
        .expect("decoded enum match control");
    assert_eq!(decoded_match, match_node);
}

#[cfg(feature = "resident-artifact")]
#[test]
fn qualified_enum_payload_pattern_matches_its_declared_variant() {
    execute_document(
        "<color> := :red<f64> | :green<f64>\n\
         value<color> := :color/red(3)\n\
         result := value?\n\
           | :color/red(x) => x\n\
           | * => 0.\n\
         result\n",
        [(
            vec![],
            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(3.0)),
        )],
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn dynamic_enum_payloads_wrap_constant_and_live_values() {
    let samples = [
        (
            "<event> := :data<*>\nvalue<event> := :data(1)\nvalue\n",
            None,
        ),
        (
            "<event> := :data<*>\nvalue<event> := :data(signal<f64>)\nvalue\n",
            Some(7.0),
        ),
    ];
    for (source, input) in samples {
        let compiled = CanonicalSourceFrontend
            .compile_document_with_nominal_origin(&document(source), &nominal_origin())
            .unwrap_or_else(|error| panic!("dynamic enum {source:?}: {error:?}"));
        let artifact = compiled.compile_artifact().unwrap();
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let sample = [input.unwrap_or_default()];
        for artifact in [&artifact, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x540, 8),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            let captured = input
                .map(|_| CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&sample),
                })
                .into_iter()
                .collect::<Vec<_>>();
            instance.turn(&captured).unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::Enum(enumeration) = output.data() else {
                panic!("declared enum output");
            };
            assert_eq!(enumeration.ordinal(), 0);
            let Some(ValueData::Dynamic(dynamic)) = enumeration.payload() else {
                panic!("enum payload must carry a Dynamic envelope");
            };
            let Some(value) = dynamic.value() else {
                panic!("dynamic payload must retain its concrete value");
            };
            assert!(
                matches!(value.data(), ValueData::F64(number) if number.to_f64() == input.unwrap_or(1.0))
            );
        }
    }
}

#[test]
fn enum_payloads_materialize_nested_deferred_constants() {
    for source in [
        "<event> := :data<*?>\nvalue<event> := :data(1)\nvalue\n",
        "<event> := :data<*>\nvalue<event> := :data(1<*?>)\nvalue\n",
    ] {
        let compiled = CanonicalSourceFrontend
            .compile_document_with_nominal_origin(&document(source), &nominal_origin())
            .unwrap_or_else(|error| panic!("nested payload {source:?}: {error:?}"));
        let artifact = compiled.compile_artifact().unwrap();
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn live_enum_payloads_execute_after_artifact_roundtrip() {
    let first = [3.0];
    let second = [9.0];
    execute_document(
        "<color> := :red<f64> | :green<f64>\n\
         my-color<color> := :red(signal<f64>)\n\
         result := my-color?\n\
           | :red(x) => x + 1\n\
           | * => 0.\n\
         result\n",
        [
            (
                vec![ResidentValueRef::F64(&first)],
                ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(4.0)),
            ),
            (
                vec![ResidentValueRef::F64(&second)],
                ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(10.0)),
            ),
        ],
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn live_structural_enum_payload_retains_its_declared_variant() {
    let first = [3.0];
    let second = [9.0];
    let expected = |value| {
        ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
            ordinal: 1,
            payload: Some(Box::new(ValueDataDraft::Tuple(
                vec![
                    ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value)),
                    ValueDataDraft::Bool(true),
                ]
                .into_boxed_slice(),
            ))),
        })
    };
    execute_document(
        "<event> := :idle | :point<(f64,bool)>\n\
         value<event> := :point((signal<f64>,true))\n\
         value\n",
        [
            (vec![ResidentValueRef::F64(&first)], expected(3.0)),
            (vec![ResidentValueRef::F64(&second)], expected(9.0)),
        ],
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn live_enum_publication_rolls_back_after_managed_allocation_failure() {
    let source = "<event> := :idle | :point<(f64,bool)>\n\
                  value<event> := :point((signal<f64>,true))\n\
                  value\n";
    let compiled = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(&document(source), &nominal_origin())
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let memory_budget = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let mut instance = activate_with_options(
        ReactiveInstanceId::new(0x540, 3),
        &decoded,
        &catalog,
        &ActivationFacts::default(),
        ResidentActivationOptions {
            memory_budget: Some(memory_budget.clone()),
            ..ResidentActivationOptions::default()
        },
    )
    .unwrap();
    let turn = |instance: &mut mech_engine::__resident::ReactiveInstance, value: &[f64]| {
        let inputs = [CapturedSignalInput {
            slot: instance.plan.inputs[0].slot,
            value: ResidentValueRef::F64(value),
        }];
        instance.turn(&inputs)
    };

    turn(&mut instance, &[3.0]).unwrap();
    let published = instance.copied_output(0).unwrap();
    let published_epoch = instance.published_epoch();

    memory_budget.inject_snapshot_import_failure_after(0);
    assert!(turn(&mut instance, &[9.0]).is_err());
    assert_eq!(instance.published_epoch(), published_epoch);
    assert_eq!(
        instance.copied_output(0).unwrap().canonical_data_draft(),
        published.canonical_data_draft()
    );

    turn(&mut instance, &[9.0]).unwrap();
    let ValueData::Enum(value) = instance.copied_output(0).unwrap().data().clone() else {
        panic!("live constructor must publish its nominal enum")
    };
    assert_eq!(value.ordinal(), 1);
    let ValueData::Tuple(payload) = value.payload().unwrap() else {
        panic!("point payload must remain structural")
    };
    assert!(
        matches!(payload.as_ref(), [ValueData::F64(value), ValueData::Bool(true)] if value.to_f64() == 9.0)
    );
}

#[test]
fn document_type_environment_rejects_duplicates_cycles_and_unknown_kinds() {
    for (source, code) in [
        (
            "<count> := <u8>\n<count> := <u16>\nx := 1\nx\n",
            "source-semantics/duplicate-kind-declaration",
        ),
        (
            "<left> := <right>\n<right> := <left>\nx := 1\nx\n",
            "source-semantics/cyclic-kind-declaration",
        ),
        (
            "<outer> := <missing>\nx := 1\nx\n",
            "source-semantics/unsupported-kind-annotation",
        ),
        (
            "<color> := Red | Red\nx := 1\nx\n",
            "source-semantics/duplicate-enum-variant",
        ),
        (
            "<u8> := <string>\nx := 1\nx\n",
            "source-semantics/builtin-kind-declaration",
        ),
        (
            "<index> := <u8>\nx := 1\nx\n",
            "source-semantics/builtin-kind-declaration",
        ),
    ] {
        let error = CanonicalSourceFrontend
            .compile_document(&document(source))
            .err()
            .unwrap();
        assert_eq!(error.code, code, "{source}: {error:?}");
    }
}

#[test]
fn pattern_functions_lower_to_ordered_partial_control() {
    for source in [
        "first(n<f64>) => <f64>\n  | n => 42\n  | * => 99.\nfirst(7)\n",
        "plus(x<f64>, y<f64>) => <f64>\n  | (x, y) => x + y.\nplus(y: 2, x: 40)\n",
        "only-zero(n<f64>) => <f64>\n  | 0 => 42.\nonly-zero(0)\n",
        "twice(n<f64>) => <f64>\n  | n => n * 2.\ntwice([1 2 3; 4 5 6])\n",
        "positive(n<f64>) => <bool>\n  | n => n > 0.\npositive([-1 0 2])\n",
    ] {
        let compiled = CanonicalSourceFrontend
            .compile_document(&document(source))
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let artifact = compiled.compile_artifact().unwrap();
        let control = artifact
            .nodes()
            .iter()
            .find_map(|node| match &node.body {
                mech_engine::ExecutableNodeBody::Match(control) => Some(control),
                mech_engine::ExecutableNodeBody::Comprehension(control) => {
                    control.steps.iter().find_map(|step| match step {
                        mech_engine::ComprehensionStep::Operation(operation) => {
                            match &operation.body {
                                mech_engine::ControlOperationBody::Match(control) => Some(control),
                                _ => None,
                            }
                        }
                        _ => None,
                    })
                }
                _ => None,
            })
            .expect("pattern function call lowers to match control");
        assert!(control.partial);
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert_eq!(decoded.revision(), artifact.revision());
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn pattern_functions_apply_per_matrix_element_and_preserve_source_shape() {
    let source = "classify(n<f64>) => <f64>\n\
                    | 0 => 1\n\
                    | n => n.\n\
                  result := classify([0 2; 0 3])\n\
                  result\n";
    let compiled = CanonicalSourceFrontend
        .compile_document(&document(source))
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let output = artifact
        .schemas()
        .get(artifact.outputs()[0].schema)
        .unwrap();
    assert!(matches!(
        output.body(),
        SchemaBody::Matrix { dimensions, .. }
            if dimensions.as_ref()
                == [
                    mech_core::DimensionExpr::Constant(2),
                    mech_core::DimensionExpr::Constant(2),
                ]
    ));
    execute_document(
        source,
        [(
            Vec::new(),
            ValueDataDraft::Matrix(
                [1.0, 2.0, 1.0, 3.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value)))
                    .collect(),
            ),
        )],
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn pattern_function_lifts_conform_each_collection_element() {
    let function = "twice(n<f64>) => <f64>\n\
                    | n => n * 2.\n";
    execute_document(
        &format!("{function}twice([1<u8> 2<u8>])\n"),
        [(
            Vec::new(),
            ValueDataDraft::Matrix(
                [2.0, 4.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value)))
                    .collect(),
            ),
        )],
    );
    execute_document(
        &format!("{function}twice({{1<u8>, 2<u8>}})\n"),
        [(
            Vec::new(),
            ValueDataDraft::Set(
                [2.0, 4.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value)))
                    .collect(),
            ),
        )],
    );
}

#[test]
fn pattern_function_lifts_annotation_compatible_tuple_elements() {
    let source = "classify(x<(*,*)>) => <f64>\n\
                    | (a, b) => 1.\n\
                  classify(signal<[(f64,bool)]>)\n";
    let compiled = CanonicalSourceFrontend
        .compile_document(&document(source))
        .unwrap();
    let schema = compiled
        .schemas()
        .get(compiled.program().outputs[0].schema)
        .unwrap();
    assert!(matches!(
        schema.body(),
        SchemaBody::Matrix { element, .. }
            if matches!(element.as_ref(), SchemaBody::FloatingPoint(_))
    ));
    compiled.compile_artifact().unwrap();
}

#[cfg(feature = "resident-artifact")]
#[test]
fn pattern_function_matrix_parameter_consumes_the_whole_matrix() {
    execute_document(
        "identity(n<[f64]>) => <[f64]>\n\
           | n => n.\n\
         identity([1 2])\n",
        [(
            Vec::new(),
            ValueDataDraft::Matrix(
                [1.0, 2.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value)))
                    .collect(),
            ),
        )],
    );
}

#[test]
fn pattern_function_lifts_keep_dynamic_collection_shape_ownership() {
    for source in [
        "twice(n<f64>) => <f64>\n  | n => n * 2.\ntwice(signal<[f64]>)\n",
        "positive(n<f64>) => <bool>\n  | n => n > 0.\npositive(signal<{f64}>)\n",
    ] {
        CanonicalSourceFrontend
            .compile_document(&document(source))
            .unwrap_or_else(|error| panic!("{source}: {error:?}"))
            .compile_artifact()
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn pattern_function_lift_uses_live_matrix_dimensions_on_each_turn() {
    let source =
        "identity(n<(f64,f64)>) => <(f64,f64)>\n  | n => n.\nidentity(signal<[(f64,f64)]>)\n";
    let compiled = CanonicalSourceFrontend
        .compile_document(&document(source))
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let input_schema = artifact.inputs()[0].schema;
    let schema = artifact.schemas().get(input_schema).unwrap();
    let shape_for = |rows, columns| {
        mech_core::shape_for_schema_components(
            schema,
            &[(
                schema.body(),
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Tuple(
                        vec![
                            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                        ]
                        .into_boxed_slice(),
                    )),
                    dimensions: vec![
                        mech_core::DimensionExpr::Constant(rows),
                        mech_core::DimensionExpr::Constant(columns),
                    ]
                    .into_boxed_slice(),
                },
            )],
            None,
        )
        .unwrap()
    };
    let mut facts = ActivationFacts::default();
    facts
        .slot_shapes
        .insert(artifact.inputs()[0].slot, shape_for(1, 6));
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x540, 16),
        &artifact,
        &catalog,
        &facts,
    )
    .unwrap();
    for (rows, columns) in [(1, 6), (2, 3), (2, 2)] {
        let shape = shape_for(rows, columns);
        let input = mech_core::ValueDraft {
            schema: input_schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                (0..rows * columns)
                    .map(|value| {
                        ValueDataDraft::Tuple(
                            vec![
                                ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(
                                    value as f64,
                                )),
                                ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(1.0)),
                            ]
                            .into_boxed_slice(),
                        )
                    })
                    .collect(),
            ),
        }
        .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
            artifact.schemas(),
        ))
        .unwrap();
        instance
            .turn(&[CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::Snapshot(&[Some(input)]),
            }])
            .unwrap();
        let output = instance.copied_output(0).unwrap();
        let SchemaBody::Matrix { dimensions, .. } = output
            .schemas()
            .unwrap()
            .get(output.schema())
            .unwrap()
            .closed_body(output.shape())
            .unwrap()
        else {
            panic!("lift must publish a matrix")
        };
        assert_eq!(
            dimensions.as_ref(),
            &[
                mech_core::DimensionExpr::Constant(rows),
                mech_core::DimensionExpr::Constant(columns)
            ]
        );
    }
}

#[test]
fn set_lift_rejects_results_the_resident_cannot_canonicalize() {
    let source = "render(n<f64>) => <string>\n  | n => \"item\".\nrender({1, 2})\n";
    let error = CanonicalSourceFrontend
        .compile_document(&document(source))
        .err()
        .expect("string set output cannot enter resident canonicalization");
    assert_eq!(
        error.code,
        "source-semantics/unsupported-lifted-set-result-kind"
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn pattern_function_arms_conform_to_the_declared_output_before_joining() {
    execute_document(
        "convert(n<f64>) => <f64>\n\
           | 0 => 1u8\n\
           | n => n.\n\
         convert(0)\n",
        [(
            Vec::new(),
            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(1.0)),
        )],
    );
}

#[test]
fn refutable_enum_payload_pattern_does_not_make_a_variant_exhaustive() {
    let source = "<choice> := :some<f64> | :none\n\
                  classify(value<choice>) => <f64>\n\
                    | :some(0) => 1\n\
                    | :none => 0.\n\
                  classify(:choice/none)\n";
    let error = CanonicalSourceFrontend
        .compile_document_with_nominal_origin(&document(source), &nominal_origin())
        .err()
        .unwrap();
    assert_eq!(error.code, "source-semantics/non-exhaustive-match");
}

#[cfg(feature = "resident-artifact")]
#[test]
fn pattern_functions_lift_over_sets_with_deduplication_and_distinct_output_kind() {
    let negative = [-2.0];
    let positive = [3.0];
    execute_document(
        "classify(n<f64>) => <bool>\n\
           | n => n > 0.\n\
         result := classify({signal<f64>, 0})\n\
         result\n",
        [
            (
                vec![ResidentValueRef::F64(&negative)],
                ValueDataDraft::Set(vec![ValueDataDraft::Bool(false)].into_boxed_slice()),
            ),
            (
                vec![ResidentValueRef::F64(&positive)],
                ValueDataDraft::Set(
                    vec![ValueDataDraft::Bool(false), ValueDataDraft::Bool(true)]
                        .into_boxed_slice(),
                ),
            ),
        ],
    );
    execute_document(
        "classify(n<f64>) => <bool>\n\
           | n => n > 0.\n\
         empty<{f64}> := {}\n\
         classify(empty)\n",
        [(Vec::new(), ValueDataDraft::Set(Box::new([])))],
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn a_failed_set_lift_discards_the_whole_candidate_and_allows_retry() {
    let source = "only-zero(n<f64>) => <f64>\n\
                    | 0 => 42.\n\
                  result := only-zero({signal<f64>})\n\
                  result\n";
    let compiled = CanonicalSourceFrontend
        .compile_document(&document(source))
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x540, 4),
        &decoded,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let turn = |instance: &mut mech_engine::__resident::ReactiveInstance, value: &[f64]| {
        let inputs = [CapturedSignalInput {
            slot: instance.plan.inputs[0].slot,
            value: ResidentValueRef::F64(value),
        }];
        instance.turn(&inputs)
    };

    turn(&mut instance, &[0.0]).unwrap();
    let published = instance.copied_output(0).unwrap();
    let published_epoch = instance.published_epoch();
    assert!(turn(&mut instance, &[1.0]).is_err());
    assert_eq!(instance.published_epoch(), published_epoch);
    assert_eq!(
        instance.copied_output(0).unwrap().canonical_data_draft(),
        published.canonical_data_draft()
    );
    turn(&mut instance, &[0.0]).unwrap();
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        ValueDataDraft::Set(
            vec![ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(
                42.0
            ))]
            .into_boxed_slice()
        )
    );
}

#[test]
fn typed_document_rejects_recovered_source_before_semantics() {
    let document = document("answer :=\n");
    let error = match CanonicalSourceFrontend.compile_document(&document) {
        Ok(_) => panic!("recovered document must not enter source semantics"),
        Err(error) => error,
    };
    assert_eq!(error.code, "source-semantics/recovered-syntax");
}

#[test]
fn typed_document_rejects_an_assignment_without_a_mutable_definition() {
    let document = document("answer += 1\n");
    let error = match CanonicalSourceFrontend.compile_document(&document) {
        Ok(_) => panic!("assignment without a mutable target must not be skipped"),
        Err(error) => error,
    };
    assert_eq!(error.code, "source-semantics/unknown-assignment-target");
    assert!(error.message.contains("answer"));
}

#[test]
fn ordering_document_state_writers_preserves_semantic_node_references() {
    let compiled = CanonicalSourceFrontend
        .compile_document(&document(
            "~answer := 0\nanswer += 1\nmatched := answer ? | *, true => 1 | * => 2\n",
        ))
        .unwrap();
    let writer = compiled.program().states[0].producer_node as usize;
    assert_eq!(writer, compiled.program().nodes.len() - 1);
    assert_eq!(
        compiled.program().nodes[writer].outputs.as_ref(),
        &[mech_engine::SourceNodeOutput::State(0)]
    );
    let (index, node) = compiled
        .program()
        .nodes
        .iter()
        .enumerate()
        .find(|(_, node)| matches!(node.body, mech_engine::SourceNodeBody::Match(_)))
        .unwrap();
    let mech_engine::SourceNodeBody::Match(control) = &node.body else {
        unreachable!()
    };
    assert_eq!(control.arms.len(), 2);
    assert!(usize::from(control.scrutinee) < node.inputs.len());
    assert_eq!(compiled.source_map().nodes[index].role, "match");
    compiled.compile_artifact().unwrap();
    let compiled = CanonicalSourceFrontend
        .compile_document(&document(
            "~answer := 0\nanswer += 1\n[answer + x | x <- [1 2]]\n",
        ))
        .unwrap();
    let (index, node) = compiled
        .program()
        .nodes
        .iter()
        .enumerate()
        .find(|(_, node)| matches!(node.body, mech_engine::SourceNodeBody::Comprehension(_)))
        .unwrap();
    let mech_engine::SourceNodeBody::Comprehension(control) = &node.body else {
        unreachable!()
    };
    assert_eq!(
        control
            .steps
            .iter()
            .filter(|step| matches!(step, mech_engine::ComprehensionStep::Generator { .. }))
            .count(),
        1
    );
    assert!(
        !node.inputs.is_empty(),
        "the collection captures the current state candidate"
    );
    assert_eq!(compiled.source_map().nodes[index].role, "comprehension");
    compiled.compile_artifact().unwrap();
}

#[test]
fn typed_document_executes_only_eval_inline_mech_code() {
    let display_only = CanonicalSourceFrontend
        .compile_document(&document("Displayed {{x<u8> := 1}}.\n\ny := x\n"))
        .expect("display-only inline Mech must be ignored by execution");
    assert_eq!(display_only.program().inputs.len(), 1);
    assert_eq!(display_only.program().inputs[0].name, "x");
    assert_eq!(
        display_only.program().outputs[0].source,
        SourceValue::Input(0)
    );
    assert!(matches!(
        display_only
            .schemas()
            .get(display_only.program().inputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Dynamic
    ));

    let evaluated = CanonicalSourceFrontend
        .compile_document(&document("Evaluated {1 + 2}.\n"))
        .expect("eval inline Mech must enter document execution");
    assert_eq!(evaluated.source_map().nodes.len(), 1);
    assert_eq!(evaluated.source_map().nodes[0].operation, "math/add");
}

#[test]
fn typed_document_excludes_mika_child_source_from_outer_execution() {
    let compiled = CanonicalSourceFrontend
        .compile_document(&document("~∘~⸢x := 1\n⸥\nx\n"))
        .expect("the enclosing document must compile independently of Mika contents");
    assert_eq!(compiled.program().inputs.len(), 1);
    assert_eq!(compiled.program().inputs[0].name, "x");
    assert_eq!(compiled.program().outputs[0].source, SourceValue::Input(0));
}

#[test]
fn canonical_document_fixture_corpus_has_an_explicit_engine_disposition() {
    let cases = [
        ("compiler.mec", Ok(())),
        ("config.mec", Ok(())),
        ("document.mec", Ok(())),
        ("empty.mec", Err("source-semantics/empty-document")),
        ("executable.mec", Ok(())),
        ("interactive.mec", Ok(())),
        ("malformed.mec", Err("source-semantics/recovered-syntax")),
        ("resolver-index.mec", Ok(())),
        ("wasm-document.mec", Ok(())),
    ];
    for (name, expected) in cases {
        let source = fs::read_to_string(
            repository_root()
                .join("tests/fixtures/syntax-source-boundary")
                .join(name),
        )
        .unwrap();
        let result = CanonicalSourceFrontend.compile_document(&document(&source));
        match (result, expected) {
            (Ok(_), Ok(())) => {}
            (Err(actual), Err(expected)) => assert_eq!(actual.code, expected, "{name}"),
            (Ok(_), Err(expected)) => panic!("{name} unexpectedly compiled; wanted {expected}"),
            (Err(actual), Ok(())) => panic!("{name} unexpectedly failed: {actual:?}"),
        }
    }
}

#[test]
fn resolver_declarations_are_metadata_and_exports_have_artifact_outputs() {
    let compiled = CanonicalSourceFrontend
        .compile_document(&document(
            "+> ./dependency.mec\n@local := @env\nvalue := 42\n<+ value\n",
        ))
        .unwrap();
    assert_eq!(compiled.document_exports().len(), 1);
    let export = &compiled.document_exports()[0];
    assert_eq!(export.name, "value");
    assert_eq!(
        compiled.program().outputs[export.output as usize].name,
        "value"
    );
    assert_eq!(
        compiled.program().outputs[export.output as usize].source,
        compiled.program().outputs[0].source
    );
    compiled.compile_artifact().unwrap();

    let error = CanonicalSourceFrontend
        .compile_document(&document("value := 42\n<+ missing\n"))
        .err()
        .unwrap();
    assert_eq!(error.code, "source-semantics/unknown-export");
    assert_eq!(error.anchor.document, DocumentId(0x541));
    assert_eq!(error.anchor.range.start, TextSize(12));
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
        .compile_expression(&expression("signal<u8> + signal<u8>"))
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
        ("{a: 1, b: 2}", "core/composite-pack"),
        ("{1: 2, 3: 4}", "core/composite-pack"),
        ("{1, 2}", "set/define"),
        ("(1, 2)", "core/composite-pack"),
        ("[1 2]", "matrix/horzcat"),
        ("|a<u8>|1|", "core/composite-pack"),
        ("math/add(left: 1, 2)", "math/add"),
        ("x[1].field", "access/column"),
        ("1..10", "range/exclusive"),
        ("x<bool> ? | * => 1", "match"),
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
fn fsm_pipe_owns_typed_arguments_stages_and_artifact_roundtrip() {
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression(
            "#machine(left: 1, 2) ~> (:next, [:head, :tail], :some(:payload), `some(:x, :y)) -> :ready => :value",
        ))
        .unwrap();
    assert_eq!(compiled.program().nodes.len(), 1);
    let mech_engine::SourceNodeBody::Fsm(fsm) = &compiled.program().nodes[0].body else {
        panic!("FSM source must retain a typed control body")
    };
    assert_eq!(fsm.machine, "machine");
    assert_eq!(fsm.arguments.len(), 2);
    assert_eq!(fsm.arguments[0].name.as_deref(), Some("left"));
    assert_eq!(fsm.arguments[0].input, 0);
    assert_eq!(fsm.arguments[1].name, None);
    assert_eq!(fsm.arguments[1].input, 1);
    assert_eq!(
        fsm.stages
            .iter()
            .map(|stage| stage.kind)
            .collect::<Vec<_>>(),
        [
            mech_engine::FsmStageKind::Async,
            mech_engine::FsmStageKind::State,
            mech_engine::FsmStageKind::Output,
        ]
    );
    assert_eq!(
        fsm.stages[0].value,
        mech_engine::FsmValue::Tuple(
            vec![
                mech_engine::FsmValue::Input(2),
                mech_engine::FsmValue::Array(
                    vec![
                        mech_engine::FsmValue::Input(3),
                        mech_engine::FsmValue::Input(4),
                    ]
                    .into_boxed_slice(),
                ),
                mech_engine::FsmValue::AtomStruct {
                    name: "some".to_owned(),
                    items: vec![mech_engine::FsmValue::Input(5)].into_boxed_slice(),
                },
                mech_engine::FsmValue::TupleStruct {
                    name: "some".to_owned(),
                    items: vec![
                        mech_engine::FsmValue::Input(6),
                        mech_engine::FsmValue::Input(7),
                    ]
                    .into_boxed_slice(),
                },
            ]
            .into_boxed_slice(),
        )
    );
    assert_eq!(fsm.stages[1].value, mech_engine::FsmValue::Input(8));
    assert_eq!(fsm.stages[2].value, mech_engine::FsmValue::Input(9));
    assert_eq!(compiled.program().nodes[0].inputs.len(), 10);
    assert_eq!(compiled.contracts(), &[None]);
    assert_eq!(compiled.source_map().patterns.len(), 3);
    assert!(
        compiled
            .source_map()
            .patterns
            .iter()
            .all(|pattern| pattern.bindings.is_empty()),
        "FSM value-role patterns must not report declaration bindings"
    );

    let artifact = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
        bytes
    );
    assert!(matches!(
        &decoded.nodes()[0].body,
        mech_engine::ExecutableNodeBody::Fsm(decoded) if decoded == fsm
    ));

    let sections = mech_engine::encode_program_artifact_sections(&artifact).unwrap();
    let graph: serde_json::Value = serde_json::from_slice(&sections.nodes).unwrap();
    let identifier_fields = [
        ("machine", "/nodes/0/body/Fsm/machine"),
        ("argument", "/nodes/0/body/Fsm/arguments/0/0"),
        (
            "structured value",
            "/nodes/0/body/Fsm/stages/0/value/Tuple/2/AtomStruct/name",
        ),
    ];
    let mutate_identifier = |role: &str, pointer: &str, replacement: &str| {
        let mut mutated = graph.clone();
        let field = mutated
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("missing FSM {role} field at {pointer}"));
        assert!(field.is_string(), "FSM {role} field must be a string");
        *field = serde_json::Value::String(replacement.to_owned());
        let mut sections = sections.clone();
        sections.nodes = serde_json::to_vec(&mutated).unwrap();
        sections
    };
    // Canonical identifier classes are defined over extended grapheme clusters.
    // U+0600 joins the following '=' into one emoji grapheme whose first scalar
    // admits it in machine, named-argument, and structured-value roles.
    // The bottom-right box terminals are not in the canonical parser's
    // BOX_DRAWING_EMOJI_RULES, so they remain valid identifier emoji too.
    for canonical_identifier in ["\u{0600}=", "┛", "┘"] {
        let canonical_source = format!(
            "#{canonical_identifier}({canonical_identifier}: 1) -> :{canonical_identifier}(:x)"
        );
        CanonicalSourceFrontend
            .compile_expression(&expression(&canonical_source))
            .unwrap()
            .compile_artifact()
            .unwrap();
        for &(role, pointer) in &identifier_fields {
            let valid = mutate_identifier(role, pointer, canonical_identifier);
            mech_engine::decode_program_artifact_sections(&valid).unwrap_or_else(|error| {
                panic!("canonical FSM {role} identifier {canonical_identifier:?}: {error:?}")
            });
        }
    }
    let mut unsupported_revision = sections.clone();
    let mut unsupported_graph = graph.clone();
    unsupported_graph["revision"] = serde_json::Value::from(0);
    unsupported_revision.nodes = serde_json::to_vec(&unsupported_graph).unwrap();
    assert!(mech_engine::decode_program_artifact_sections(&unsupported_revision).is_err());
    // Artifact admission must enforce the complete canonical forbidden-emoji
    // terminal set in all three identifier roles, including a forbidden
    // grapheme after a valid prefix.
    for glyph in [
        '\u{00a0}', '\u{2009}', '\u{27e8}', '\u{27e9}', '\u{2e22}', '\u{2e25}', '╭', '╮', '╰', '╯',
        '┏', '┓', '┗', '┌', '┐', '└', '┼', '─', '├', '┤', '┬', '┴', '│', '┃',
    ] {
        for name in [
            format!("{glyph}bad"),
            format!("bad{glyph}"),
            format!("b{glyph}ad"),
        ] {
            for &(role, pointer) in &identifier_fields {
                let invalid = mutate_identifier(role, pointer, &name);
                assert!(
                    mech_engine::decode_program_artifact_sections(&invalid).is_err(),
                    "forbidden canonical grapheme in FSM {role} identifier: {name:?}"
                );
            }
        }
    }
    for (&(role, pointer), replacement) in
        identifier_fields
            .iter()
            .zip([" ", "bad\0name", "bad\0name"])
    {
        let invalid = mutate_identifier(role, pointer, replacement);
        assert!(
            mech_engine::decode_program_artifact_sections(&invalid).is_err(),
            "noncanonical FSM {role} identifier {replacement:?}"
        );
    }
}

#[test]
fn maximum_depth_structured_fsm_values_roundtrip() {
    let nested_value = |wrappers| {
        let mut value = ":x".to_owned();
        for _ in 0..wrappers {
            value = format!(":some({value})");
        }
        value
    };
    let value = nested_value(31);
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression(&format!("#machine -> {value}")))
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let sections = mech_engine::encode_program_artifact_sections(&artifact).unwrap();
    mech_engine::decode_program_artifact_sections(&sections).unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();

    let beyond_limit = nested_value(32);
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression(&format!("#machine -> {beyond_limit}")))
        .unwrap();
    let result = compiled.compile_artifact();
    assert!(
        matches!(
            result,
            Err(mech_engine::ArtifactBuildError::InvalidControl {
                reason: "FSM value admission limit",
                ..
            })
        ),
        "{result:?}"
    );
}

#[test]
fn fsm_values_predeclare_late_input_annotations() {
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression("#machine -> x -> x<u8>"))
        .unwrap();
    assert_eq!(compiled.program().inputs.len(), 1);
    assert!(matches!(
        compiled
            .schemas()
            .get(compiled.program().inputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    ));
    assert_eq!(
        compiled.program().nodes[0].inputs.as_ref(),
        &[SourceValue::Input(0), SourceValue::Input(0)]
    );
    compiled
        .compile_artifact()
        .expect("FSM value annotations must be occurrence-order independent");
}

#[cfg(feature = "resident-artifact")]
#[test]
fn typed_fsm_fails_closed_until_the_resident_continuation_owner_lands() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId};
    use mech_engine::__resident::{ActivationFacts, ResidentActivationError, activate};

    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression("#machine -> :ready"))
        .unwrap();
    let artifact = compiled.compile_artifact().unwrap();
    let catalog = FunctionCatalogBuilder::new().build().unwrap();

    assert!(matches!(
        activate(
            ReactiveInstanceId::new(0x540, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        ),
        Err(ResidentActivationError::UnsupportedControlLayout { node })
            if node == mech_core::NodeId(0)
    ));
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
        .compile_expression(&expression("math/add(left: 1, 2)"))
        .unwrap();
    let node = call.program().nodes.last().unwrap();
    assert_eq!(
        node.operation()
            .expect("ordinary operation fixture")
            .canonical_name(),
        "math/add"
    );
    assert_eq!(
        call.source_map().nodes.last().unwrap().detail.as_deref(),
        Some("math/add(left,)")
    );
    call.compile_artifact()
        .expect("a declared source call must carry its maintained contract");

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
    assert_eq!(dimensions[1], mech_core::DimensionExpr::Constant(5));
    let dynamic = CanonicalSourceFrontend
        .compile_expression(&expression("start..=3"))
        .unwrap();
    let schema = dynamic
        .schemas()
        .get(dynamic.program().outputs[0].schema)
        .unwrap();
    assert!(!schema.dimension_parameters().is_empty());
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
        .filter(|node| {
            node.operation()
                .expect("ordinary operation fixture")
                .canonical_name()
                == "access/scalar"
        })
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
    let mech_engine::SourceNodeBody::Comprehension(control) =
        &comprehension.program().nodes[0].body
    else {
        panic!("typed collection control")
    };
    assert!(matches!(
        control.steps.as_ref(),
        [mech_engine::ComprehensionStep::Generator {
            source: mech_engine::ComprehensionValue::Input(0),
            pattern: mech_engine::CollectionPattern::Bind { local: 0, .. },
        }]
    ));
    assert_eq!(
        control.yield_value,
        mech_engine::ComprehensionValue::Local(0)
    );
    comprehension.compile_artifact().unwrap();

    let qualified = CanonicalSourceFrontend
        .compile_expression(&expression("[y | x <- xs, y := x, y > 0]"))
        .unwrap();
    let mech_engine::SourceNodeBody::Comprehension(control) = &qualified.program().nodes[0].body
    else {
        panic!("typed qualifiers")
    };
    assert!(matches!(
        control.steps.first(),
        Some(mech_engine::ComprehensionStep::Generator { .. })
    ));
    assert!(matches!(
        control.steps.last(),
        Some(mech_engine::ComprehensionStep::Filter(_))
    ));
    assert_eq!(
        control.yield_value,
        mech_engine::ComprehensionValue::Local(0),
        "immutable alias preserves the generator binding"
    );
    assert!(control.steps.iter().any(|step| matches!(
        step,
        mech_engine::ComprehensionStep::Operation(operation)
            if matches!(
                &operation.body,
                mech_engine::ControlOperationBody::Operation { operation, .. }
                    if operation.canonical_name() == "compare/gt"
            )
    )));
    qualified.compile_artifact().unwrap();

    let destructured = CanonicalSourceFrontend
        .compile_expression(&expression("[a + b | (a<u8>, b<u8>) <- xs]"))
        .unwrap();
    let mech_engine::SourceNodeBody::Comprehension(control) = &destructured.program().nodes[0].body
    else {
        panic!("typed projections")
    };
    let mech_engine::ComprehensionStep::Generator {
        pattern: mech_engine::CollectionPattern::Tuple(fields),
        ..
    } = &control.steps[0]
    else {
        panic!("tuple projection")
    };
    assert!(matches!(
        fields.as_ref(),
        [
            mech_engine::CollectionPattern::Bind { local: 0, .. },
            mech_engine::CollectionPattern::Bind { local: 1, .. }
        ]
    ));
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
    assert_eq!(
        add.inputs.as_ref(),
        &[
            mech_engine::ComprehensionValue::Local(0),
            mech_engine::ComprehensionValue::Local(1)
        ]
    );
    destructured.compile_artifact().unwrap();

    let typed_pattern = CanonicalSourceFrontend
        .compile_expression(&expression("value<bool> ? | y<bool> => !y | * => false"))
        .unwrap();
    let mech_engine::SourceNodeBody::Match(control) = &typed_pattern.program().nodes[0].body else {
        panic!("typed match");
    };
    assert!(matches!(
        control.arms[0].pattern,
        mech_engine::MatchPattern::Structural(mech_engine::CollectionPattern::Bind {
            local: 0,
            ..
        })
    ));
    let parameter = &control.arms[0].body.parameters[0];
    assert_eq!(
        parameter.source,
        mech_engine::ControlParameterSource::PatternBinding(0)
    );
    assert_eq!(
        typed_pattern
            .schemas()
            .get(parameter.schema)
            .unwrap()
            .body(),
        &SchemaBody::Bool
    );
    assert_eq!(control.arms[0].body.operations.len(), 1);
    assert_eq!(typed_pattern.program().inputs.len(), 1);
    typed_pattern.compile_artifact().unwrap();
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
        state.program().nodes[0]
            .operation()
            .expect("ordinary operation fixture")
            .canonical_name(),
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
        .err()
        .expect("unsupported constrained annotations must be diagnosed");
    assert_eq!(
        constrained_optional.code,
        "source-semantics/unsupported-kind-constraint"
    );
    let matrix = CanonicalSourceFrontend
        .compile_expression(&expression("matrix<[u64]>"))
        .expect("dimensionless matrix annotations retain independent extents");
    let schema = matrix
        .schemas()
        .get(matrix.program().inputs[0].schema)
        .unwrap();
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = schema.body()
    else {
        panic!("matrix annotation must retain its matrix schema");
    };
    assert_eq!(
        element.as_ref(),
        &SchemaBody::UnsignedInteger(IntegerWidth::W64)
    );
    assert_eq!(dimensions.len(), 2);
    assert_ne!(dimensions[0], dimensions[1]);
    assert_eq!(schema.dimension_parameters().len(), 2);

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

    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("1u8 === 2u16"))
            .err()
            .expect("strict equality requires identical kinds")
            .code,
        "source-semantics/incompatible-comparison-kinds"
    );

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
        .compile_expression(&expression("-signal<i8>"))
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
        let compiled = CanonicalSourceFrontend
            .compile_definition(&definition(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        assert!(compiled.program().states[0].initializer.is_some());
        compiled.compile_artifact().unwrap();
    }

    let overflow = CanonicalSourceFrontend
        .compile_expression(&expression("1.0e100<f32>"))
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
        .err()
        .expect("unsupported constrained annotations must be diagnosed");
    assert_eq!(
        dynamic_option.code,
        "source-semantics/unsupported-kind-constraint"
    );

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
    assert_eq!(range.inputs[2], SourceValue::Input(0));
    assert_eq!(
        ternary_range
            .schemas()
            .get(ternary_range.program().inputs[0].schema)
            .unwrap()
            .body(),
        &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
    );
    ternary_range.compile_artifact().expect(
        "an unannotated third endpoint must acquire its concrete peer kind before range emission",
    );

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
fn reviewed_source_kind_edges_match_operation_and_literal_contracts() {
    let optional_kind = CanonicalSourceFrontend
        .compile_expression(&expression("<u8><*?>"))
        .unwrap();
    let SourceValue::Constant(id) = optional_kind.program().outputs[0].source else {
        panic!("annotated kind literal did not produce a constant")
    };
    assert!(matches!(
        optional_kind.constants().get(id).unwrap().data(),
        ValueData::Option(Some(value))
            if matches!(value.as_ref(), ValueData::Dynamic(dynamic)
                if matches!(dynamic.value().map(|value| value.data()), Some(ValueData::Type(_))))
    ));
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("<u8><f64>"))
            .err()
            .expect("incompatible reified annotation must fail")
            .code,
        "source-semantics/incompatible-literal-kind"
    );

    let error = CanonicalSourceFrontend
        .compile_expression(&expression("x<bool> ? | threshold + 1 => 2 | * => 3"))
        .err()
        .expect("expected rejection");
    assert_eq!(error.code, "source-semantics/unsupported-match");

    for (source, code) in [
        ("1 && 2", "source-semantics/non-boolean-operator-kind"),
        ("1/2 % 1/3", "source-semantics/invalid-modulus-kind"),
        (
            "true == \"true\"",
            "source-semantics/incompatible-comparison-kinds",
        ),
        (
            "true < false",
            "source-semantics/incompatible-comparison-kinds",
        ),
        ("¬:ready", "source-semantics/non-boolean-negation-kind"),
        ("¬<u8>", "source-semantics/non-boolean-negation-kind"),
        (
            "true + true",
            "source-semantics/non-numeric-arithmetic-kind",
        ),
        (
            "\"a\" * \"b\"",
            "source-semantics/non-numeric-arithmetic-kind",
        ),
        ("-:ready", "source-semantics/non-negatable-kind"),
        ("-<u8>", "source-semantics/non-negatable-kind"),
        (
            "x<bool> ? | *, 1 => 2 | * => 3",
            "source-semantics/non-boolean-operator-kind",
        ),
        ("{}", "source-semantics/unresolved-set-element-kind"),
        ("x<_>", "source-semantics/unsupported-empty-kind-schema"),
    ] {
        assert_eq!(
            CanonicalSourceFrontend
                .compile_expression(&expression(source))
                .err()
                .expect("invalid operator operands must fail")
                .code,
            code,
            "{source:?}"
        );
    }
    for source in [
        "true && false",
        "left && right",
        "1u8 % 2u8",
        "\"a\" < \"b\"",
        ":ready == :ready",
        "limit < 1",
        "value % 2",
        "value + 1",
        "\"a\" + \"b\"",
    ] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        compiled
            .compile_artifact()
            .unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
    }
    let concatenated = CanonicalSourceFrontend
        .compile_expression(&expression("\"a\" + \"b\""))
        .unwrap();
    assert_eq!(
        concatenated
            .program()
            .nodes
            .last()
            .unwrap()
            .operation()
            .expect("ordinary operation fixture")
            .canonical_name(),
        "string/concat"
    );

    let maximum = CanonicalSourceFrontend
        .compile_expression(&expression("340282366920938463463374607431768211455u128"))
        .unwrap();
    let SourceValue::Constant(id) = maximum.program().outputs[0].source else {
        panic!("U128 maximum did not produce a constant")
    };
    assert!(matches!(
        maximum.constants().get(id).unwrap().data(),
        ValueData::U128(value) if *value == u128::MAX
    ));

    let local = CanonicalSourceFrontend
        .compile_expression(&expression("[x<u8> | x := 1]"))
        .unwrap();
    assert!(local.program().inputs.is_empty());
    assert!((0..local.constants().len()).any(|index| {
        matches!(
            local
                .constants()
                .get(mech_core::ConstantId::new(index as u32))
                .unwrap()
                .data(),
            ValueData::U8(1)
        )
    }));
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("[x<u8> | x := :ready]"))
            .err()
            .expect("incompatible local occurrence annotation must fail")
            .code,
        "source-semantics/incompatible-local-kind"
    );

    let transposed = CanonicalSourceFrontend
        .compile_expression(&expression("[1 2]'"))
        .unwrap();
    let transpose_index = transposed
        .program()
        .nodes
        .iter()
        .position(|node| {
            node.operation()
                .expect("ordinary operation fixture")
                .canonical_name()
                == "matrix/transpose"
        })
        .unwrap();
    let contract = transposed.contracts()[transpose_index].as_ref().unwrap();
    assert_eq!(
        contract.outputs[0].construction,
        OutputConstruction::FullWrite {
            shape: ShapeRule::TransposeOf { input: 0 }
        }
    );
    assert_eq!(
        contract.outputs[0].change_detection,
        ChangeDetectionPolicy::KernelReported
    );

    let transposed_range = CanonicalSourceFrontend
        .compile_expression(&expression("(1..3)'"))
        .unwrap();
    let SchemaBody::Matrix { dimensions, .. } = transposed_range
        .schemas()
        .get(transposed_range.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("transposed range did not retain a matrix schema")
    };
    assert_eq!(dimensions[0], mech_core::DimensionExpr::Constant(2));
    assert_eq!(dimensions[1], mech_core::DimensionExpr::Constant(1));
    transposed_range
        .compile_artifact()
        .expect("transposed range must satisfy the resident matrix contract");

    let ordered_matrices = CanonicalSourceFrontend
        .compile_expression(&expression("(1..3) < (2..4)"))
        .unwrap();
    let SchemaBody::Matrix { element, .. } = ordered_matrices
        .schemas()
        .get(ordered_matrices.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("ordered matrix comparison did not retain matrix shape")
    };
    assert!(matches!(element.as_ref(), SchemaBody::Bool));
    ordered_matrices
        .compile_artifact()
        .expect("ordered matrix comparison must satisfy its matrix scheme");

    let equal_matrices = CanonicalSourceFrontend
        .compile_expression(&expression("(1..3) == (1..3)"))
        .unwrap();
    let SchemaBody::Matrix { element, .. } = equal_matrices
        .schemas()
        .get(equal_matrices.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("matrix equality did not retain matrix shape")
    };
    assert!(matches!(element.as_ref(), SchemaBody::Bool));
    assert_eq!(
        equal_matrices
            .contracts()
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .outputs[0]
            .change_detection,
        ChangeDetectionPolicy::KernelReported
    );

    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("1e3"))
            .err()
            .expect("the selected typed-integer suffix must not be reinterpreted")
            .code,
        "source-semantics/unsupported-number-kind-suffix"
    );
    for source in ["1.0e3u8"] {
        let scientific = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap();
        let SourceValue::Constant(id) = scientific.program().outputs[0].source else {
            panic!("scientific literal did not produce a constant")
        };
        assert!(matches!(
            scientific.constants().get(id).unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 1000.0
        ));
    }

    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("300u8<i16>"))
            .err()
            .expect("an out-of-range typed literal must fail before outer conversion")
            .code,
        "source-semantics/invalid-number-literal"
    );
    let converted = CanonicalSourceFrontend
        .compile_expression(&expression("1u8<i16>"))
        .unwrap();
    let SourceValue::Constant(id) = converted.program().outputs[0].source else {
        panic!("converted typed literal did not remain constant")
    };
    assert!(matches!(
        converted.constants().get(id).unwrap().data(),
        ValueData::I16(1)
    ));
}

#[test]
fn reviewed_exact_source_authorities_cover_matches_kinds_logic_and_matrices() {
    let matched = CanonicalSourceFrontend
        .compile_expression(&expression("x<bool> ? | * => 1u8 | * => 2u8"))
        .unwrap();
    assert!(matches!(
        matched
            .schemas()
            .get(matched.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    ));
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("x<bool> ? | * => 1u8 | * => true"))
            .err()
            .expect("incompatible match result schemas must fail")
            .code,
        "source-semantics/incompatible-match-result-kind"
    );

    let negated = CanonicalSourceFrontend
        .compile_expression(&expression("¬[true false]"))
        .unwrap();
    assert!(matches!(
        negated
            .schemas()
            .get(negated.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Matrix { element, .. } if matches!(element.as_ref(), SchemaBody::Bool)
    ));
    assert_eq!(
        negated
            .contracts()
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .outputs[0]
            .change_detection,
        ChangeDetectionPolicy::KernelReported
    );

    for source in [
        "<[u8]>",
        "<[u8]:1_024,2u8>",
        "<{u8:f64}>",
        "<{u8}:10>",
        "<{a<u8>,b<bool?>}>",
        "<(u8,f64)>",
        "<|a<u8>|:10>",
    ] {
        let reified = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let SourceValue::Constant(id) = reified.program().outputs[0].source else {
            panic!("{source:?} did not produce a reified kind constant")
        };
        assert!(matches!(
            reified.constants().get(id).unwrap().data(),
            ValueData::Type(_)
        ));
        if source == "<[u8]>" {
            let ValueData::Type(mech_core::snapshot::ReifiedType::Kind(kind)) =
                reified.constants().get(id).unwrap().data()
            else {
                panic!("matrix kind did not retain a canonical kind value")
            };
            let (kind, dimensions, _) = kind.decoded_closed_kind().unwrap();
            assert_eq!(dimensions.len(), 2);
            assert_eq!(
                dimensions
                    .iter()
                    .map(|dimension| (
                        dimension.origin,
                        dimension.lifetime,
                        dimension.lower_bound.clone(),
                        dimension.upper_bound.clone(),
                    ))
                    .collect::<Vec<_>>(),
                vec![
                    (
                        mech_core::DimensionParameterOrigin::Explicit,
                        mech_core::DimensionLifetime::Activation,
                        mech_core::DimensionExpr::Constant(0),
                        None,
                    );
                    2
                ]
            );
            assert!(matches!(
                kind,
                mech_core::KindExpr::Matrix { dimensions, .. }
                    if dimensions.as_ref()
                        == [mech_core::DimensionExpr::Parameter(
                                mech_core::DimensionParameterId::new(0)),
                            mech_core::DimensionExpr::Parameter(
                            mech_core::DimensionParameterId::new(1))]
            ));
            let artifact = reified.compile_artifact().unwrap();
            let decoded = mech_engine::decode_program_artifact_bytecode_v1(
                &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
            )
            .unwrap();
            assert_eq!(decoded.revision(), artifact.revision());
            for raw in 0..artifact.constants().len() {
                let id = mech_core::ConstantId::new(raw as u32);
                let expected = artifact.constants().get(id).unwrap();
                let actual = decoded.constants().get(id).unwrap();
                assert_eq!(expected.schema_key(), actual.schema_key());
                assert_eq!(
                    expected.value_hash(artifact.schemas()).unwrap(),
                    actual.value_hash(decoded.schemas()).unwrap()
                );
            }
        } else if source == "<[u8]:1_024,2u8>" {
            let ValueData::Type(mech_core::snapshot::ReifiedType::Kind(kind)) =
                reified.constants().get(id).unwrap().data()
            else {
                panic!("matrix kind did not retain a canonical kind value")
            };
            let (kind, dimensions, _) = kind.decoded_closed_kind().unwrap();
            assert!(dimensions.is_empty());
            assert!(matches!(
                kind,
                mech_core::KindExpr::Matrix { dimensions, .. }
                    if dimensions.as_ref()
                        == [mech_core::DimensionExpr::Constant(1_024),
                            mech_core::DimensionExpr::Constant(2)]
            ));
        }
    }

    let blocks = CanonicalSourceFrontend
        .compile_expression(&expression("[(1..3) (4..6)]"))
        .unwrap();
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = blocks
        .schemas()
        .get(blocks.program().outputs[0].schema)
        .unwrap()
        .body()
    else {
        panic!("matrix blocks did not produce a matrix schema")
    };
    assert!(matches!(
        element.as_ref(),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
    ));
    assert_eq!(dimensions.len(), 2);
    assert_eq!(
        blocks
            .program()
            .nodes
            .last()
            .unwrap()
            .operation()
            .expect("ordinary operation fixture")
            .canonical_name(),
        "matrix/horzcat"
    );
    blocks
        .compile_artifact()
        .expect("matrix block concatenation must retain its shared contract");

    let optional = CanonicalSourceFrontend
        .compile_expression(&expression("[1 _]"))
        .unwrap();
    assert!(matches!(
        optional
            .schemas()
            .get(optional.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Matrix { element, dimensions }
            if matches!(element.as_ref(), SchemaBody::Option(payload)
                if matches!(payload.as_ref(), SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)))
                && dimensions.as_ref()
                    == [mech_core::DimensionExpr::Constant(1), mech_core::DimensionExpr::Constant(2)]
    ));
    let node = optional.program().nodes.last().unwrap();
    assert_eq!(
        node.operation()
            .expect("ordinary operation fixture")
            .canonical_name(),
        "matrix/literal"
    );
    assert_eq!(node.inputs.len(), 2);
    assert!(
        optional
            .source_map()
            .nodes
            .iter()
            .all(|node| { node.operation != "source/empty" && node.operation != "convert/kind" })
    );
    assert!(
        optional
            .source_map()
            .nodes
            .iter()
            .all(|node| { node.operation != "source/empty" && node.operation != "convert/kind" })
    );
    let values = node
        .inputs
        .iter()
        .map(|input| {
            let SourceValue::Constant(id) = input else {
                panic!("optional matrix elements must remain constants")
            };
            optional.constants().get(*id).unwrap().data()
        })
        .collect::<Vec<_>>();
    assert!(matches!(values[0], ValueData::Option(Some(_))));
    assert!(matches!(values[1], ValueData::Option(None)));
    optional
        .compile_artifact()
        .expect("optional matrix constants must produce a contracted artifact");

    for source in ["matrix/horzcat([1], [2])", "matrix/vertcat([1], [2])"] {
        let concatenated = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let contract = concatenated.contracts().last().unwrap().as_ref().unwrap();
        assert!(matches!(
            contract.inputs,
            mech_core::InputPortLayout::Variadic { .. }
        ));
        assert!(matches!(
            contract.outputs[0].construction,
            OutputConstruction::Build { .. }
        ));
        assert_eq!(
            contract.outputs[0].change_detection,
            ChangeDetectionPolicy::KernelReported
        );
    }

    CanonicalSourceFrontend
        .compile_expression(&expression("math/abs(-1.0)"))
        .expect(
            "distribution-owned maintained operations must compile without an engine feature gate",
        );
}

#[test]
fn exact_table_columns_and_c32_are_first_class_source_schemas() {
    for (source, expected) in [
        (
            "╭─────────╮\n│ state   │\n├─────────┤\n│ :ready  │\n╰─────────╯",
            "atom",
        ),
        (
            "╭─────────╮\n│ kind    │\n├─────────┤\n│ <u8>    │\n╰─────────╯",
            "kind",
        ),
    ] {
        let table = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let SchemaBody::Table { columns, .. } = table
            .schemas()
            .get(table.program().outputs[0].schema)
            .unwrap()
            .body()
        else {
            panic!("fancy table did not produce a table schema")
        };
        assert!(
            matches!(
                (&columns[0].schema, expected),
                (SchemaBody::Atom(_), "atom") | (SchemaBody::ReifiedType, "kind")
            ),
            "{source:?}: {:?}",
            columns[0].schema
        );
    }

    for source in ["<c32>", "<c32?>"] {
        let reified = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let SourceValue::Constant(id) = reified.program().outputs[0].source else {
            panic!("{source:?} did not produce a reified kind constant")
        };
        assert!(matches!(
            reified.constants().get(id).unwrap().data(),
            ValueData::Type(_)
        ));
    }

    for source in ["1<c32>", "1+2i<c32>"] {
        let value = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let SourceValue::Constant(id) = value.program().outputs[0].source else {
            panic!("{source:?} did not produce a constant")
        };
        assert!(matches!(
            value.constants().get(id).unwrap().data(),
            ValueData::Complex32(_)
        ));
        assert!(matches!(
            value
                .schemas()
                .get(value.program().outputs[0].schema)
                .unwrap()
                .body(),
            SchemaBody::Complex(mech_core::FloatWidth::W32)
        ));
    }

    let optional = CanonicalSourceFrontend
        .compile_expression(&expression("signal<c32?>"))
        .unwrap();
    assert!(matches!(
        optional
            .schemas()
            .get(optional.program().inputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Option(payload)
            if matches!(payload.as_ref(), SchemaBody::Complex(mech_core::FloatWidth::W32))
    ));

    let parameterized_table = CanonicalSourceFrontend
        .compile_expression(&expression(
            "╭──────────╮\n│ values   │\n├──────────┤\n│ (a..3)   │\n╰──────────╯",
        ))
        .unwrap();
    let table_schema = parameterized_table
        .schemas()
        .get(parameterized_table.program().outputs[0].schema)
        .unwrap();
    let SchemaBody::Table { columns, .. } = table_schema.body() else {
        panic!("parameterized fancy table did not produce a table schema")
    };
    assert!(matches!(columns[0].schema, SchemaBody::Matrix { .. }));
    assert!(!table_schema.dimension_parameters().is_empty());

    let matrix = CanonicalSourceFrontend
        .compile_expression(&expression("[1 2]"))
        .unwrap();
    assert_eq!(
        matrix
            .program()
            .nodes
            .last()
            .unwrap()
            .operation()
            .expect("ordinary operation fixture")
            .canonical_name(),
        "matrix/horzcat"
    );
    assert!(matches!(
        matrix
            .schemas()
            .get(matrix.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Matrix { .. }
    ));
    matrix
        .compile_artifact()
        .expect("matrix literals must construct canonical artifacts");

    let set = CanonicalSourceFrontend
        .compile_expression(&expression("{1u8, 2u8}"))
        .unwrap();
    assert!(matches!(
        set.schemas()
            .get(set.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Set { element, cardinality }
            if matches!(element.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
                && *cardinality == mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2))
    ));
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("{1+2i}"))
            .err()
            .expect("non-keyable set elements must be rejected")
            .code,
        "source-semantics/non-keyable-set-element-kind"
    );

    let call = CanonicalSourceFrontend
        .compile_expression(&expression("math/sin(1)"))
        .unwrap();
    assert!(matches!(
        call.schemas()
            .get(call.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
    ));
    assert!(call.contracts().last().unwrap().is_some());
    assert_eq!(
        call.contracts().last().unwrap().as_ref().unwrap().outputs[0].construction,
        OutputConstruction::FullWrite {
            shape: ShapeRule::SameAsInput { input: 0 }
        }
    );
    call.compile_artifact()
        .expect("maintained calls must produce contracted artifacts");
}

#[test]
fn compound_and_maintained_operations_retain_exact_source_schemas() {
    let tuple = CanonicalSourceFrontend
        .compile_expression(&expression("(1u8, (a..3))"))
        .unwrap();
    let tuple_schema = tuple
        .schemas()
        .get(tuple.program().outputs[0].schema)
        .unwrap();
    assert!(matches!(
        tuple_schema.body(),
        SchemaBody::Tuple(items)
            if matches!(items[0], SchemaBody::UnsignedInteger(IntegerWidth::W8))
                && matches!(items[1], SchemaBody::Matrix { .. })
    ));
    assert!(!tuple_schema.dimension_parameters().is_empty());

    let record = CanonicalSourceFrontend
        .compile_expression(&expression("{values: (a..3), ready: true}"))
        .unwrap();
    let record_schema = record
        .schemas()
        .get(record.program().outputs[0].schema)
        .unwrap();
    assert!(matches!(
        record_schema.body(),
        SchemaBody::Record(fields)
            if matches!(fields[0].schema, SchemaBody::Matrix { .. })
                && matches!(fields[1].schema, SchemaBody::Bool)
    ));
    assert!(!record_schema.dimension_parameters().is_empty());

    let map = CanonicalSourceFrontend
        .compile_expression(&expression("{1u8: (a..3), 2u8: (b..4)}"))
        .unwrap();
    let map_schema = map.schemas().get(map.program().outputs[0].schema).unwrap();
    assert!(matches!(
        map_schema.body(),
        SchemaBody::Map {
            key,
            value,
            cardinality
        } if matches!(key.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
            && matches!(value.as_ref(), SchemaBody::Matrix { .. })
            && *cardinality == mech_core::CardinalitySpec::Exact(
                mech_core::DimensionExpr::Constant(2)
            )
    ));
    assert!(!map_schema.dimension_parameters().is_empty());
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("{1u8: true, 2u8: 3}"))
            .err()
            .expect("source compilation must fail")
            .code,
        "source-semantics/incompatible-map-value-kind"
    );
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("{1+2i: true}"))
            .err()
            .expect("source compilation must fail")
            .code,
        "source-semantics/non-keyable-map-key-kind"
    );
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("{:}"))
            .err()
            .expect("empty maps must require explicit kinds")
            .code,
        "source-semantics/unresolved-map-entry-kind"
    );

    for (source, expected_element) in [
        (
            "[1 2] + 3",
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        ),
        ("[1 2] < 3", SchemaBody::Bool),
        ("[true false] && true", SchemaBody::Bool),
        (
            "[1 2] ** [3; 4]",
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        ),
        (
            "[1.0 0.0; 0.0 1.0] \\ [2.0; 3.0]",
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
        ),
    ] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        assert!(matches!(
            compiled
                .schemas()
                .get(compiled.program().outputs[0].schema)
                .unwrap()
                .body(),
            SchemaBody::Matrix { element, .. } if element.as_ref() == &expected_element
        ));
    }

    let promoted_matrix = CanonicalSourceFrontend
        .compile_expression(&expression("[1u8 2u8] + 3u16"))
        .unwrap();
    assert!(matches!(
        promoted_matrix
            .schemas()
            .get(promoted_matrix.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Matrix { element, .. }
            if matches!(element.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W16))
    ));

    let dot = CanonicalSourceFrontend
        .compile_expression(&expression("[1 2] · [3 4]"))
        .unwrap();
    assert!(matches!(
        dot.schemas()
            .get(dot.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
    ));
    let subset = CanonicalSourceFrontend
        .compile_expression(&expression("{1u8} ⊆ {1u8, 2u8}"))
        .unwrap();
    assert!(matches!(
        subset
            .schemas()
            .get(subset.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Bool
    ));

    let cartesian = CanonicalSourceFrontend
        .compile_expression(&expression("set/cartesian-product({1u8}, {true})"))
        .unwrap();
    assert!(matches!(
        cartesian
            .schemas()
            .get(cartesian.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Set { element, .. }
            if matches!(element.as_ref(), SchemaBody::Tuple(items)
                if matches!(items[0], SchemaBody::UnsignedInteger(IntegerWidth::W8))
                    && matches!(items[1], SchemaBody::Bool))
    ));
    let powerset = CanonicalSourceFrontend
        .compile_expression(&expression("set/powerset({1u8, 2u8})"))
        .unwrap();
    assert!(matches!(
        powerset
            .schemas()
            .get(powerset.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Set { element, .. }
            if matches!(element.as_ref(), SchemaBody::Set {
                element: nested,
                cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: Some(_) }
            } if matches!(nested.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8)))
    ));

    let union = CanonicalSourceFrontend
        .compile_expression(&expression("{1u8} ∪ {2u8}"))
        .unwrap();
    assert!(matches!(
        union
            .schemas()
            .get(union.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Set {
            element,
            cardinality: mech_core::CardinalitySpec::Dynamic { .. }
        } if matches!(element.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
    ));

    for source in ["[x | x <- {1u8, 2u8}]", "{x | x <- {1u8, 2u8}}"] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap();
        assert!(
            compiled.contracts().last().unwrap().is_none(),
            "typed collection roots do not fabricate ordinary call contracts"
        );
        assert!(matches!(
            compiled.program().nodes.last().unwrap().body,
            mech_engine::SourceNodeBody::Comprehension(_)
        ));
        compiled.compile_artifact().unwrap();
        let output = compiled
            .schemas()
            .get(compiled.program().outputs[0].schema)
            .unwrap()
            .body();
        if source.starts_with('[') {
            assert!(matches!(
                output,
                SchemaBody::Matrix { element, .. }
                    if matches!(element.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
            ));
        } else {
            assert!(matches!(
                output,
                SchemaBody::Set { element, .. }
                    if matches!(element.as_ref(), SchemaBody::UnsignedInteger(IntegerWidth::W8))
            ));
        }
    }

    let variadic = CanonicalSourceFrontend
        .compile_expression(&expression("matrix/horzcat([1], [2], [3])"))
        .unwrap();
    assert!(matches!(
        variadic
            .schemas()
            .get(variadic.program().outputs[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Matrix { dimensions, .. }
            if dimensions[0] == mech_core::DimensionExpr::Constant(1)
                && dimensions[1] == mech_core::DimensionExpr::Constant(3)
    ));

    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&expression("1'"))
            .err()
            .expect("source compilation must fail")
            .code,
        "source-semantics/incompatible-call-kind"
    );
}

#[test]
fn typed_match_blocks_and_fsm_diagnostics_preserve_owned_roles() {
    let matched = CanonicalSourceFrontend
        .compile_expression(&expression("x<bool> ? | *, true => 1 | * => 2"))
        .unwrap();
    let mech_engine::SourceNodeBody::Match(control) = &matched.program().nodes[0].body else {
        panic!("typed match body");
    };
    assert_eq!(control.arms.len(), 2);
    assert!(control.arms[0].guard.is_some());
    assert!(control.arms[1].guard.is_none());
    matched.compile_artifact().unwrap();

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
    let aggregate_program_path = ["mech_core", "Program"].join("::");
    assert!(!source.contains(&aggregate_program_path));
    let legacy_lower_path = ["document", "lower"].join("::");
    assert!(!source.contains(&legacy_lower_path));
    assert!(!source.contains("parser::parse("));
}

#[test]
fn input_matrix_annotations_share_constraints_in_either_occurrence_order() {
    for source in [
        "(signal<[f64]>, signal<[f64]:2,3>)",
        "(signal<[f64]:2,3>, signal<[f64]>)",
    ] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(compiled.program().inputs.len(), 1);
        let schema = compiled
            .schemas()
            .get(compiled.program().inputs[0].schema)
            .unwrap();
        let SchemaBody::Matrix { dimensions, .. } = schema.body() else {
            panic!("matrix input lost its schema");
        };
        assert_eq!(
            dimensions.as_ref(),
            &[
                mech_core::DimensionExpr::Constant(2),
                mech_core::DimensionExpr::Constant(3)
            ]
        );
    }
    for source in [
        "(signal<[f64]>, signal<[u64]:2,3>)",
        "(signal<[f64]:3,2>, signal<[f64]:2,3>)",
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
fn absent_optional_matrix_annotations_have_valid_shape_witnesses() {
    for source in ["_<[f64]?>", "_<([f64],[u8])?>"] {
        let compiled = CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap();
        let SourceValue::Constant(id) = compiled.program().outputs[0].source else {
            panic!("absent value must be constant");
        };
        let value = compiled.constants().get(id).unwrap();
        assert!(matches!(value.data(), ValueData::Option(None)));
        compiled.compile_artifact().unwrap();
        let schema = compiled.schemas().get(value.schema()).unwrap();
        schema
            .instantiate_shape(value.shape().parameter_values().to_vec().into_boxed_slice())
            .unwrap();
    }
    CanonicalSourceFrontend
        .compile_definition(&definition("x<[f64]?> := _"))
        .unwrap();
}

#[test]
fn matrix_bind_patterns_specialize_dimensions() {
    for source in [
        "[1.0 2.0] ? | y<[f64]> => y",
        "[y | y<[f64]> <- {[1.0 2.0]}]",
    ] {
        CanonicalSourceFrontend
            .compile_expression(&expression(source))
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
    assert!(
        CanonicalSourceFrontend
            .compile_expression(&expression("[1.0 2.0] ? | y<[u64]> => y"))
            .is_err()
    );
}

#[test]
fn static_output_projection_retains_state_dependencies_and_source_identity() {
    use std::collections::BTreeSet;
    let compiled = CanonicalSourceFrontend
        .compile_document_with_planning_contract(
            &document("unrelated := 7 + 8\n~answer := 40\nanswer += 2\nother := 9 + 10\n"),
            std::sync::Arc::new(mech_core::FunctionCatalogBuilder::new().build().unwrap()),
            Default::default(),
            Default::default(),
            &BTreeSet::new(),
            &BTreeSet::from(["answer".to_owned()]),
            &BTreeSet::new(),
        )
        .unwrap();
    let original_count = compiled.program().nodes.len();
    let compiled = compiled
        .retain_static_outputs(&BTreeSet::from(["answer".to_owned()]))
        .unwrap();
    assert!(compiled.program().nodes.len() < original_count);
    assert_eq!(compiled.program().outputs.len(), 1);
    assert_eq!(compiled.source_map().outputs.len(), 1);
    assert_eq!(
        compiled.source_map().nodes.len(),
        compiled.program().nodes.len()
    );
    assert_eq!(compiled.program().states.len(), 1);
    compiled.compile_artifact().unwrap();
    let empty = compiled.retain_static_outputs(&BTreeSet::new()).unwrap();
    assert!(empty.program().nodes.is_empty());
    assert!(empty.program().states.is_empty());
    match empty.retain_static_outputs(&BTreeSet::from(["missing".to_owned()])) {
        Ok(_) => panic!("missing output must fail"),
        Err(error) => assert_eq!(error.code, "source-semantics/unknown-published-binding"),
    }
}
