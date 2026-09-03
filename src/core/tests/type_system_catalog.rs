#![cfg(feature = "functions")]

use std::sync::Arc;

use mech_core::{
    BuiltinKindPredicate, BuiltinScalarKind, CanonicalFunctionSpecializer, DimensionExpr,
    DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, FunctionCatalogBuilder, FunctionTypeDeclaration,
    FunctionTypeOverload, InputKindScheme, KindConstraint, KindExpr, KindScheme, MResult,
    ResolvedType, SourceInputKind, SourceSchemeTemplate, SourceTypeAuthority,
    SpecializationContext, SpecializationInvocation, SpecializedFunction,
    TypeConstraintEnvironment, TypeConstraintOrigin, instantiate_source_scheme_template,
    maintained_source_type_declaration,
};

struct NeverSpecialize;

impl CanonicalFunctionSpecializer for NeverSpecialize {
    fn specialize_invocation(
        &self,
        _: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        unreachable!("catalog declaration tests never execute physical specializers")
    }
}

fn nullary_scheme() -> KindScheme {
    KindScheme::new(
        Box::new([]),
        Box::new([]),
        InputKindScheme::Fixed(Box::new([])),
        vec![KindExpr::Index].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap()
}

#[test]
fn named_specializers_are_scheme_authoritative() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_canonical_specializer(
            "math/add",
            maintained_source_type_declaration("math/add").unwrap(),
            Arc::new(NeverSpecialize),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let entry = catalog.all_specializers().next().unwrap();
    let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
        panic!("a named operation cannot be syntax-directed")
    };
    assert!(!declaration.overloads.is_empty());
}

#[test]
fn parser_intrinsics_are_explicit_and_never_named_exports() {
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_canonical_intrinsic_specializer("assign", Arc::new(NeverSpecialize))
        .unwrap();
    let catalog = builder.build().unwrap();
    let entry = catalog.intrinsic_specializer_entries().next().unwrap();
    assert_eq!(
        entry.type_authority,
        SourceTypeAuthority::SyntaxDirectedIntrinsic
    );
    assert!(catalog.all_specializers().next().is_none());
    assert!(catalog.all_exports().next().is_none());
}

#[test]
fn malformed_and_duplicate_overloads_are_rejected() {
    let malformed = FunctionTypeDeclaration {
        overloads: vec![FunctionTypeOverload {
            id: 1,
            input_layout: vec![SourceInputKind::Value].into_boxed_slice(),
            scheme: nullary_scheme(),
        }]
        .into_boxed_slice(),
        template: None,
    };
    let mut builder = FunctionCatalogBuilder::new();
    let error = builder
        .insert_canonical_specializer("test/malformed", malformed, Arc::new(NeverSpecialize))
        .unwrap_err();
    assert_eq!(error.kind_name(), "FunctionCatalogInvalidTypeDeclaration");

    let overload = FunctionTypeOverload {
        id: 7,
        input_layout: Box::new([]),
        scheme: nullary_scheme(),
    };
    let duplicate = FunctionTypeDeclaration {
        overloads: vec![overload.clone(), overload].into_boxed_slice(),
        template: None,
    };
    let error = builder
        .insert_canonical_specializer("test/duplicate", duplicate, Arc::new(NeverSpecialize))
        .unwrap_err();
    assert_eq!(error.kind_name(), "FunctionCatalogInvalidTypeDeclaration");
}

#[test]
fn maintained_declarations_are_deterministic_and_ids_are_unique() {
    const OPERATIONS: &[&str] = &[
        "math/add",
        "math/mod",
        "math/neg",
        "compare/seq",
        "compare/eq",
        "compare/gt",
        "logic/not",
        "logic/and",
        "range/inclusive",
        "range/inclusive-increment",
        "matrix/transpose",
        "matrix/matmul",
        "matrix/dot",
        "matrix/solve",
        "matrix/horzcat",
        "matrix/vertcat",
        "set/element-of",
        "set/insert",
        "set/remove",
        "set/union",
        "set/intersection",
        "set/difference",
        "set/symmetric-difference",
        "set/cartesian-product",
        "set/powerset",
        "set/subset",
        "set/size",
        "string/concat",
        "stats/sum/column",
        "stats/sum/row",
        "combinatorics/n-choose-k",
    ];
    for operation in OPERATIONS {
        let first = maintained_source_type_declaration(operation).unwrap();
        let second = maintained_source_type_declaration(operation).unwrap();
        assert_eq!(first, second, "{operation}");
        let mut ids = first
            .overloads
            .iter()
            .map(|overload| overload.id)
            .collect::<Vec<_>>();
        let original = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), original, "{operation}");
    }
}

fn scalar(kind: BuiltinScalarKind) -> ResolvedType {
    ResolvedType::new(kind.kind_expr(), Box::new([])).unwrap()
}

fn fixed_matrix(kind: BuiltinScalarKind, rows: u64, columns: u64) -> ResolvedType {
    ResolvedType::new(
        KindExpr::Matrix {
            element: Box::new(kind.kind_expr()),
            dimensions: vec![
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        },
        Box::new([]),
    )
    .unwrap()
}

fn turn_row_matrix(kind: BuiltinScalarKind, columns: u64) -> ResolvedType {
    let rows = DimensionParameterId::new(0);
    ResolvedType::new(
        KindExpr::Matrix {
            element: Box::new(kind.kind_expr()),
            dimensions: vec![
                DimensionExpr::Parameter(rows),
                DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        },
        vec![DimensionParameterDeclaration {
            id: rows,
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
    )
    .unwrap()
}

#[test]
fn strict_matrix_equality_accepts_compatible_turn_extents_without_conversion() {
    let declaration = maintained_source_type_declaration("compare/seq").unwrap();
    let compatible = [
        turn_row_matrix(BuiltinScalarKind::String, 1),
        fixed_matrix(BuiltinScalarKind::String, 15, 1),
    ];
    let solved = declaration
        .overloads
        .iter()
        .find_map(|overload| {
            TypeConstraintEnvironment::new(TypeConstraintOrigin::new("compare/seq", None))
                .solve_scheme(&overload.scheme, &compatible, None)
                .ok()
        })
        .expect("strict equality must defer a turn-varying extent to the runtime shape contract");
    assert_eq!(
        solved.outputs[0].kind(),
        &BuiltinScalarKind::Bool.kind_expr()
    );
    assert!(solved.conversions.iter().all(|plan| plan.cost == 0));

    let incompatible = [
        fixed_matrix(BuiltinScalarKind::String, 1, 2),
        fixed_matrix(BuiltinScalarKind::String, 1, 3),
    ];
    assert!(declaration.overloads.iter().all(|overload| {
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("compare/seq", None))
            .solve_scheme(&overload.scheme, &incompatible, None)
            .is_err()
    }));
}

#[test]
fn matrix_product_preserves_outer_axes_and_rejects_fixed_inner_mismatch() {
    let declaration = maintained_source_type_declaration("matrix/matmul").unwrap();
    let valid = [
        fixed_matrix(BuiltinScalarKind::F64, 2, 3),
        fixed_matrix(BuiltinScalarKind::F64, 3, 4),
    ];
    let solved = declaration
        .overloads
        .iter()
        .find_map(|overload| {
            TypeConstraintEnvironment::new(TypeConstraintOrigin::new("matrix/matmul", None))
                .solve_scheme(&overload.scheme, &valid, None)
                .ok()
        })
        .expect("one matrix-product scheme must accept compatible inputs");
    assert!(matches!(
        solved.outputs[0].kind(),
        KindExpr::Matrix { dimensions, .. }
            if dimensions.as_ref() == [DimensionExpr::Constant(2), DimensionExpr::Constant(4)]
    ));

    let invalid = [
        fixed_matrix(BuiltinScalarKind::F64, 1, 2),
        fixed_matrix(BuiltinScalarKind::F64, 3, 1),
    ];
    assert!(declaration.overloads.iter().all(|overload| {
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("matrix/matmul", None))
            .solve_scheme(&overload.scheme, &invalid, None)
            .is_err()
    }));

    let fallback = &declaration.overloads[1].scheme;
    assert!(matches!(
        &fallback.constraints()[1],
        KindConstraint::DimensionCompatible(_, _)
    ));
    assert!(matches!(
        &fallback.outputs()[0],
        KindExpr::Matrix { dimensions, .. }
            if dimensions[0] == DimensionExpr::Parameter(mech_core::DimensionParameterId::new(0))
                && dimensions[1] == DimensionExpr::Parameter(mech_core::DimensionParameterId::new(3))
    ));
}

#[test]
fn concatenation_templates_accept_more_than_thirty_two_inputs() {
    let inputs = vec![scalar(BuiltinScalarKind::F64); 40];
    for (template, expected) in [
        (
            SourceSchemeTemplate::HorizontalConcatenation,
            [DimensionExpr::Constant(1), DimensionExpr::Constant(40)],
        ),
        (
            SourceSchemeTemplate::VerticalConcatenation,
            [DimensionExpr::Constant(40), DimensionExpr::Constant(1)],
        ),
    ] {
        let scheme = instantiate_source_scheme_template(template, &inputs)
            .unwrap()
            .remove(0);
        let solved = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("concat", None))
            .solve_scheme(&scheme, &inputs, None)
            .unwrap();
        assert!(matches!(
            solved.outputs[0].kind(),
            KindExpr::Matrix { dimensions, .. } if dimensions.as_ref() == expected
        ));
    }
}

#[test]
fn set_definition_cardinality_and_keyability_are_semantic() {
    let inputs = vec![scalar(BuiltinScalarKind::String); 3];
    let scheme = instantiate_source_scheme_template(SourceSchemeTemplate::SetDefinition, &inputs)
        .unwrap()
        .remove(0);
    let solved = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("set/define", None))
        .solve_scheme(&scheme, &inputs, None)
        .unwrap();
    assert!(matches!(
        solved.outputs[0].kind(),
        KindExpr::Set {
            cardinality: DimensionExpr::Constant(3),
            ..
        }
    ));
    assert!(scheme.constraints().iter().any(|constraint| matches!(
        constraint,
        KindConstraint::Satisfies {
            predicate: BuiltinKindPredicate::Keyable,
            ..
        }
    )));

    let unkeyable = vec![scalar(BuiltinScalarKind::C64)];
    let scheme =
        instantiate_source_scheme_template(SourceSchemeTemplate::SetDefinition, &unkeyable)
            .unwrap()
            .remove(0);
    assert!(
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("set/define", None))
            .solve_scheme(&scheme, &unkeyable, None)
            .is_err()
    );

    let comprehension = maintained_source_type_declaration("set/comprehension").unwrap();
    assert!(comprehension.template.is_none());
    assert!(comprehension.overloads.iter().all(|overload| {
        overload.scheme.constraints().iter().any(|constraint| {
            matches!(
                constraint,
                KindConstraint::Satisfies {
                    predicate: BuiltinKindPredicate::Keyable,
                    ..
                }
            )
        })
    }));
}
