#![cfg(feature = "source_default")]

use std::sync::Arc;

use mech_core::{
    AccessMode, AliasPolicy, BuiltinScalarKind, CanonicalFunctionSpecializer,
    ChangeDetectionPolicy, DeliveryMode, ExternalInteraction, FunctionCatalog,
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, FunctionTypeDeclaration,
    InputKindScheme, InputPortLayout, InputPortPolicy, KindExpr, KindParameter, KindParameterId,
    KindScheme, MResult, OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
    ShapeRule, SpecializationContext, SpecializationInvocation, SpecializedFunction, ValueData,
};
use mech_engine::{
    CanonicalSourceFrontend, CanonicalSourceProgram, SourceSemanticError, SourceValue,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

struct MetadataOnly;

impl CanonicalFunctionSpecializer for MetadataOnly {
    fn specialize_invocation(
        &self,
        _invocation: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        panic!("source metadata tests must not invoke an execution factory")
    }
}

fn kind(kind: BuiltinScalarKind) -> KindExpr {
    KindExpr::Named(kind.kind_id())
}

fn fixed(inputs: Vec<KindExpr>, output: KindExpr) -> KindScheme {
    KindScheme::new(
        Box::new([]),
        Box::new([]),
        InputKindScheme::Fixed(inputs.into_boxed_slice()),
        Box::new([output]),
        Box::new([]),
    )
    .unwrap()
}

fn register(
    builder: &mut FunctionCatalogBuilder,
    item: &str,
    schemes: Vec<KindScheme>,
    names: Option<&[&str]>,
) {
    let contracts = schemes
        .iter()
        .map(|scheme| {
            let InputKindScheme::Fixed(inputs) = scheme.inputs() else {
                unreachable!()
            };
            OperationContractDeclaration {
                inputs: InputPortLayout::Fixed(
                    vec![
                        InputPortPolicy {
                            access: AccessMode::Read,
                            delivery: DeliveryMode::Signal
                        };
                        inputs.len()
                    ]
                    .into_boxed_slice(),
                ),
                outputs: Box::new([OutputPortPolicy {
                    access: AccessMode::Write,
                    delivery: DeliveryMode::Signal,
                    construction: OutputConstruction::FullWrite {
                        shape: ShapeRule::Declared,
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: ChangeDetectionPolicy::ExactScalar,
                }]),
                interaction: ExternalInteraction::Pure,
            }
        })
        .collect();
    let mut declaration = FunctionTypeDeclaration::from_schemes(schemes);
    declaration.parameter_names = names.map(|names| {
        names
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>()
            .into_boxed_slice()
    });
    let canonical_name = format!("fixture/{item}");
    let operation = builder
        .insert_canonical_specializer_with_contracts(
            &canonical_name,
            declaration,
            contracts,
            Arc::new(MetadataOnly),
        )
        .unwrap();
    builder
        .insert_export(FunctionExport {
            operation,
            canonical_name,
            module: Some("fixture".into()),
            item: Some(item.into()),
            exposure: FunctionExposure::ModuleOnly,
        })
        .unwrap();
}

fn catalog() -> Arc<FunctionCatalog> {
    use BuiltinScalarKind::{Bool, F32, F64, I64, String as Text, U64};
    let mut builder = FunctionCatalogBuilder::new();
    for (item, scalar) in [
        ("unsigned", U64),
        ("signed", I64),
        ("string", Text),
        ("boolean", Bool),
    ] {
        register(
            &mut builder,
            item,
            vec![fixed(vec![kind(scalar)], kind(scalar))],
            Some(&["value"]),
        );
    }
    register(
        &mut builder,
        "shared",
        vec![
            fixed(vec![kind(F32), kind(U64)], kind(F32)),
            fixed(vec![kind(F64), kind(U64)], kind(F64)),
        ],
        Some(&["value", "precision"]),
    );
    register(
        &mut builder,
        "ambiguous",
        vec![
            fixed(vec![kind(U64)], kind(U64)),
            fixed(vec![kind(F64)], kind(F64)),
        ],
        None,
    );
    register(
        &mut builder,
        "arity",
        vec![
            fixed(vec![kind(U64)], kind(U64)),
            fixed(vec![kind(F64), kind(F64)], kind(F64)),
        ],
        None,
    );
    let parameter = KindParameterId::new(0);
    let generic = KindScheme::new(
        Box::new([KindParameter {
            id: parameter,
            upper_bound: None,
        }]),
        Box::new([]),
        InputKindScheme::Fixed(Box::new([KindExpr::Parameter(parameter)])),
        Box::new([KindExpr::Parameter(parameter)]),
        Box::new([]),
    )
    .unwrap();
    register(&mut builder, "generic", vec![generic], None);
    register(&mut builder, "first", vec![fixed(vec![], kind(U64))], None);
    register(&mut builder, "second", vec![fixed(vec![], kind(F64))], None);
    Arc::new(builder.build().unwrap())
}

fn compile(source: &str) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let source = format!("+> fixture\n{source}");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0xC011), Revision(0), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{source}");
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    CanonicalSourceFrontend.compile_document_with_catalog(&document, catalog())
}

fn argument(compiled: &CanonicalSourceProgram, ordinal: usize) -> &ValueData {
    let SourceValue::Constant(id) = compiled.program().nodes.last().unwrap().inputs[ordinal] else {
        panic!("expected a literal argument")
    };
    compiled.constants().get(id).unwrap().data()
}

#[test]
fn concrete_argument_context_is_generic_and_checked_before_default_f64_lowering() {
    let compiled = compile("x := fixture/unsigned(18446744073709551615)\n").unwrap();
    assert!(matches!(argument(&compiled, 0), ValueData::U64(u64::MAX)));
    let compiled = compile("x := fixture/signed(-9223372036854775808)\n").unwrap();
    assert!(matches!(argument(&compiled, 0), ValueData::I64(i64::MIN)));
    for value in [
        "-1",
        "-0",
        "18446744073709551616",
        "9.0",
        "1e1",
        "9<f64>",
        "9<i64>",
    ] {
        assert!(
            compile(&format!("x := fixture/unsigned({value})\n")).is_err(),
            "{value}"
        );
    }
}

#[test]
fn contexts_use_arity_consensus_and_skip_ambiguous_or_generic_parameters() {
    let compiled = compile("x := fixture/arity(9)\n").unwrap();
    assert!(matches!(argument(&compiled, 0), ValueData::U64(9)));
    for item in ["ambiguous", "generic"] {
        let compiled = compile(&format!("x := fixture/{item}(9)\n")).unwrap();
        assert!(matches!(argument(&compiled, 0), ValueData::F64(value) if value.to_f64() == 9.0));
    }
    for item in ["string", "boolean"] {
        assert!(compile(&format!("x := fixture/{item}(9)\n")).is_err());
    }
}

#[test]
fn aliases_and_bound_names_share_the_same_parameter_context() {
    for source in [
        "x := fixture/shared(1.25, 9)\n",
        "x := fixture/shared(precision: 9, value: 1.25)\n",
        "+> chosen := fixture/shared\nx := chosen(precision: 9, 1.25)\n",
    ] {
        let compiled = compile(source).unwrap();
        assert!(matches!(argument(&compiled, 0), ValueData::F64(value) if value.to_f64() == 1.25));
        assert!(matches!(argument(&compiled, 1), ValueData::U64(9)));
    }
    let compiled = compile("x := fixture/shared(1.25<f32>, 9)\n").unwrap();
    assert!(matches!(argument(&compiled, 0), ValueData::F32(value) if value.to_f32() == 1.25));
    assert!(compile("x := fixture/shared(1.25, 9<f64>)\n").is_err());
}

#[test]
fn named_argument_reordering_preserves_source_operation_order() {
    let compiled =
        compile("x := fixture/shared(precision: fixture/first(), value: fixture/second())\n")
            .unwrap();
    let nodes = &compiled.program().nodes;
    assert_eq!(
        nodes
            .iter()
            .map(|node| node.operation().unwrap().canonical_name())
            .collect::<Vec<_>>(),
        ["fixture/first", "fixture/second", "fixture/shared"]
    );
    assert!(matches!(
        nodes[2].inputs[0],
        SourceValue::NodeOutput {
            node: 1,
            output_ordinal: 0
        }
    ));
    assert!(matches!(
        nodes[2].inputs[1],
        SourceValue::NodeOutput {
            node: 0,
            output_ordinal: 0
        }
    ));
    assert_eq!(compiled.constants().len(), 0);
}
