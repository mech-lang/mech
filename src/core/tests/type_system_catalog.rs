#![cfg(feature = "functions")]

use std::sync::{Arc, LazyLock};

use mech_core::{
    AccessMode, AliasPolicy, BuiltinKindPredicate, BuiltinScalarKind, CanonicalFunctionSpecializer,
    ChangeDetectionPolicy, DeliveryMode, DimensionExpr, DimensionLifetime,
    DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin,
    ExternalInteraction, FunctionCatalogBuilder, FunctionTypeDeclaration, FunctionTypeOverload,
    InputKindScheme, InputPortLayout, KindConstraint, KindExpr, KindScheme, MResult,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ResolvedOutputSchemaRule,
    ResolvedType, ShapeRule, SourceInputKind, SourceSchemeTemplate, SourceTypeAuthority,
    SpecializationContext, SpecializationInvocation, SpecializedFunction,
    TypeConstraintEnvironment, TypeConstraintOrigin, instantiate_source_scheme_template,
    maintained_source_type_declaration,
};

static TEST_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(Box::new([])),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

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
        .insert_canonical_specializer_with_contract(
            "math/add",
            maintained_source_type_declaration("math/add").unwrap(),
            TEST_CONTRACT.clone(),
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
        .insert_canonical_intrinsic_specializer(
            "assign",
            TEST_CONTRACT.clone(),
            Arc::new(NeverSpecialize),
        )
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
            output_schema_rules: vec![ResolvedOutputSchemaRule::FromResolvedType]
                .into_boxed_slice(),
        }]
        .into_boxed_slice(),
        template: None,
    };
    let mut builder = FunctionCatalogBuilder::new();
    let error = builder
        .insert_canonical_specializer_with_contract(
            "test/malformed",
            malformed,
            TEST_CONTRACT.clone(),
            Arc::new(NeverSpecialize),
        )
        .unwrap_err();
    assert_eq!(error.kind_name(), "FunctionCatalogInvalidTypeDeclaration");

    let overload = FunctionTypeOverload {
        id: 7,
        input_layout: Box::new([]),
        scheme: nullary_scheme(),
        output_schema_rules: vec![ResolvedOutputSchemaRule::FromResolvedType].into_boxed_slice(),
    };
    let duplicate = FunctionTypeDeclaration {
        overloads: vec![overload.clone(), overload].into_boxed_slice(),
        template: None,
    };
    let error = builder
        .insert_canonical_specializer_with_contract(
            "test/duplicate",
            duplicate,
            TEST_CONTRACT.clone(),
            Arc::new(NeverSpecialize),
        )
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

fn fixed_set(kind: BuiltinScalarKind, cardinality: u64) -> ResolvedType {
    ResolvedType::new(
        KindExpr::Set {
            element: Box::new(kind.kind_expr()),
            cardinality: DimensionExpr::Constant(cardinality),
        },
        Box::new([]),
    )
    .unwrap()
}

fn turn_row_matrix(kind: BuiltinScalarKind, columns: u64) -> ResolvedType {
    turn_axis_matrix(kind, 0, columns)
}

fn turn_axis_matrix(kind: BuiltinScalarKind, axis: usize, fixed: u64) -> ResolvedType {
    let rows = DimensionParameterId::new(0);
    let mut dimensions = vec![DimensionExpr::Constant(fixed); 2];
    dimensions[axis] = DimensionExpr::Parameter(rows);
    ResolvedType::new(
        KindExpr::Matrix {
            element: Box::new(kind.kind_expr()),
            dimensions: dimensions.into_boxed_slice(),
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
    assert!(
        fallback
            .constraints()
            .iter()
            .any(|constraint| matches!(constraint, KindConstraint::DimensionCompatible(_, _)))
    );
    assert!(matches!(
        &fallback.outputs()[0],
        KindExpr::Matrix { dimensions, .. }
            if dimensions[0] == DimensionExpr::Parameter(mech_core::DimensionParameterId::new(0))
                && dimensions[1] == DimensionExpr::Parameter(mech_core::DimensionParameterId::new(3))
    ));
}

#[test]
fn matrix_numeric_operations_reject_string_elements_semantically() {
    let strings = [
        fixed_matrix(BuiltinScalarKind::String, 2, 2),
        fixed_matrix(BuiltinScalarKind::String, 2, 2),
    ];
    for operation in ["matrix/matmul", "matrix/dot"] {
        let declaration = maintained_source_type_declaration(operation).unwrap();
        assert!(declaration.overloads.iter().all(|overload| {
            TypeConstraintEnvironment::new(TypeConstraintOrigin::new(operation, None))
                .solve_scheme(&overload.scheme, &strings, None)
                .is_err()
        }));
    }
}

#[test]
fn dynamic_dot_and_solve_schemes_enforce_and_preserve_dimensions() {
    let dot = maintained_source_type_declaration("matrix/dot").unwrap();
    let dot_fallback = &dot.overloads[1].scheme;
    assert!(
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("matrix/dot", None))
            .solve_scheme(
                dot_fallback,
                &[
                    fixed_matrix(BuiltinScalarKind::F64, 2, 3),
                    fixed_matrix(BuiltinScalarKind::F64, 2, 4),
                ],
                None,
            )
            .is_err()
    );

    let solve = maintained_source_type_declaration("matrix/solve").unwrap();
    let solve_fallback = &solve.overloads[1].scheme;
    let solved = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("matrix/solve", None))
        .solve_scheme(
            solve_fallback,
            &[
                fixed_matrix(BuiltinScalarKind::F64, 2, 2),
                fixed_matrix(BuiltinScalarKind::F64, 2, 3),
            ],
            None,
        )
        .unwrap();
    assert!(matches!(
        solved.outputs[0].kind(),
        KindExpr::Matrix { dimensions, .. }
            if dimensions.as_ref() == [DimensionExpr::Constant(2), DimensionExpr::Constant(3)]
    ));
    assert!(
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("matrix/solve", None))
            .solve_scheme(
                solve_fallback,
                &[
                    fixed_matrix(BuiltinScalarKind::F64, 2, 3),
                    fixed_matrix(BuiltinScalarKind::F64, 2, 1),
                ],
                None,
            )
            .is_err()
    );
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

#[test]
fn set_union_closes_bound_input_dimensions_inside_the_output_upper_bound() {
    let declaration = maintained_source_type_declaration("set/union").unwrap();
    let inputs = [
        fixed_set(BuiltinScalarKind::F64, 2),
        fixed_set(BuiltinScalarKind::F64, 2),
    ];
    let solved = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("set/union", None))
        .solve_scheme(&declaration.overloads[0].scheme, &inputs, None)
        .unwrap();
    let output = &solved.outputs[0];

    assert_eq!(output.dimension_parameters().len(), 1);
    assert_eq!(
        output.dimension_parameters()[0].upper_bound,
        Some(DimensionExpr::Constant(4))
    );
}

#[test]
fn set_membership_keeps_the_candidate_schema_independent() {
    let set = ResolvedType::new(
        KindExpr::Set {
            element: Box::new(BuiltinScalarKind::F64.kind_expr()),
            cardinality: DimensionExpr::Constant(3),
        },
        Box::new([]),
    )
    .unwrap();
    let declaration = maintained_source_type_declaration("set/element-of").unwrap();
    let solved = declaration
        .overloads
        .iter()
        .find_map(|overload| {
            TypeConstraintEnvironment::new(TypeConstraintOrigin::new("set/element-of", None))
                .solve_scheme(
                    &overload.scheme,
                    &[fixed_matrix(BuiltinScalarKind::F64, 1, 1), set.clone()],
                    None,
                )
                .ok()
        })
        .expect("a candidate with a different schema is a valid non-member");
    assert_eq!(solved.outputs.len(), 1);
    assert_eq!(
        solved.outputs[0].kind(),
        &BuiltinScalarKind::Bool.kind_expr()
    );
}

fn resolve_named_overload(
    name: &str,
    inputs: &[ResolvedType],
) -> Result<mech_core::ResolvedOverload, mech_core::TypeResolutionError> {
    resolve_named_overload_with_expected(name, inputs, None)
}

fn resolve_named_overload_with_expected(
    name: &str,
    inputs: &[ResolvedType],
    expected_outputs: Option<&[ResolvedType]>,
) -> Result<mech_core::ResolvedOverload, mech_core::TypeResolutionError> {
    let declaration = maintained_source_type_declaration(name).unwrap();
    let candidates = declaration
        .overloads
        .iter()
        .map(|overload| mech_core::TypeOverloadCandidate {
            id: u64::from(overload.id),
            scheme: &overload.scheme,
        })
        .collect::<Vec<_>>();
    mech_core::resolve_type_overloads(
        TypeConstraintOrigin::new(name, None),
        &candidates,
        inputs,
        expected_outputs,
    )
}

#[test]
fn ordinary_matrix_equality_wins_over_whole_aggregate_equality() {
    for name in ["compare/eq", "compare/neq"] {
        for element in [
            BuiltinScalarKind::F64,
            BuiltinScalarKind::Bool,
            BuiltinScalarKind::String,
        ] {
            let matrix = fixed_matrix(element, 2, 3);
            let resolved = resolve_named_overload(name, &[matrix.clone(), matrix]).unwrap();
            assert_eq!(
                resolved.outputs.as_ref(),
                &[fixed_matrix(BuiltinScalarKind::Bool, 2, 3)]
            );
            assert!(resolved.conversions.iter().all(|plan| plan.cost == 0));
        }
        let aggregate = ResolvedType::new(
            KindExpr::Tuple(
                vec![
                    BuiltinScalarKind::F64.kind_expr(),
                    BuiltinScalarKind::Bool.kind_expr(),
                ]
                .into_boxed_slice(),
            ),
            Box::new([]),
        )
        .unwrap();
        let resolved = resolve_named_overload(name, &[aggregate.clone(), aggregate]).unwrap();
        assert_eq!(
            resolved.outputs.as_ref(),
            &[scalar(BuiltinScalarKind::Bool)]
        );
    }
    let matrix = fixed_matrix(BuiltinScalarKind::F64, 2, 3);
    let strict = resolve_named_overload("compare/seq", &[matrix.clone(), matrix]).unwrap();
    assert_eq!(strict.outputs.as_ref(), &[scalar(BuiltinScalarKind::Bool)]);
}

#[test]
fn equality_broadcasts_preserve_equatable_kinds_and_axes() {
    for name in ["compare/eq", "compare/neq"] {
        for element in [BuiltinScalarKind::Bool, BuiltinScalarKind::String] {
            let matrix = fixed_matrix(element, 2, 3);
            for other in [
                scalar(element),
                fixed_matrix(element, 2, 1),
                fixed_matrix(element, 1, 3),
            ] {
                for inputs in [
                    [matrix.clone(), other.clone()],
                    [other.clone(), matrix.clone()],
                ] {
                    let resolved = resolve_named_overload(name, &inputs).unwrap();
                    assert_eq!(
                        resolved.outputs.as_ref(),
                        &[fixed_matrix(BuiltinScalarKind::Bool, 2, 3)]
                    );
                    assert!(resolved.conversions.iter().all(|plan| plan.cost == 0));
                }
            }
            for other in [
                fixed_matrix(element, 3, 1),
                fixed_matrix(element, 1, 4),
                fixed_matrix(element, 3, 2),
            ] {
                assert!(resolve_named_overload(name, &[matrix.clone(), other]).is_err());
            }
        }
    }
}

#[test]
fn promoted_elementwise_schemes_admit_only_supported_broadcast_axes() {
    for (name, output_element) in [
        ("math/add", BuiltinScalarKind::F64),
        ("compare/gt", BuiltinScalarKind::Bool),
        ("compare/max", BuiltinScalarKind::F64),
    ] {
        let matrix = fixed_matrix(BuiltinScalarKind::F32, 2, 3);
        for other in [
            scalar(BuiltinScalarKind::F64),
            fixed_matrix(BuiltinScalarKind::F64, 2, 1),
            fixed_matrix(BuiltinScalarKind::F64, 1, 3),
        ] {
            for inputs in [
                [matrix.clone(), other.clone()],
                [other.clone(), matrix.clone()],
            ] {
                let resolved = resolve_named_overload(name, &inputs).unwrap();
                assert_eq!(
                    resolved.outputs.as_ref(),
                    &[fixed_matrix(output_element, 2, 3)],
                    "{name} returned the wrong broadcast shape"
                );
            }
        }
        for other in [
            fixed_matrix(BuiltinScalarKind::F64, 3, 1),
            fixed_matrix(BuiltinScalarKind::F64, 1, 4),
            fixed_matrix(BuiltinScalarKind::F64, 3, 2),
        ] {
            assert!(
                resolve_named_overload(name, &[matrix.clone(), other]).is_err(),
                "{name} admitted incompatible elementwise matrix dimensions"
            );
        }
    }
}

#[test]
fn mixed_fixed_and_live_broadcast_axes_have_one_semantic_result() {
    for (name, input_element, output_element) in [
        ("math/add", BuiltinScalarKind::F64, BuiltinScalarKind::F64),
        ("math/mul", BuiltinScalarKind::F64, BuiltinScalarKind::F64),
        (
            "compare/gt",
            BuiltinScalarKind::F64,
            BuiltinScalarKind::Bool,
        ),
        (
            "compare/eq",
            BuiltinScalarKind::String,
            BuiltinScalarKind::Bool,
        ),
        (
            "logic/and",
            BuiltinScalarKind::Bool,
            BuiltinScalarKind::Bool,
        ),
    ] {
        for axis in 0..2 {
            for extent in [0, 1, 10] {
                let live = turn_axis_matrix(input_element, axis, 1);
                let (rows, columns) = if axis == 0 { (extent, 1) } else { (1, extent) };
                let fixed = fixed_matrix(input_element, rows, columns);
                let expected = if extent == 1 {
                    // A singleton broadcasts without requiring the live axis
                    // itself to become one. Empty live axes are valid too.
                    turn_axis_matrix(output_element, axis, 1)
                } else {
                    fixed_matrix(output_element, rows, columns)
                };
                for inputs in [[live.clone(), fixed.clone()], [fixed.clone(), live.clone()]] {
                    let resolved = resolve_named_overload(name, &inputs).unwrap_or_else(|error| {
                        panic!("{name}, axis {axis}, extent {extent}: {error:?}")
                    });
                    assert_eq!(resolved.outputs.as_ref(), &[expected.clone()], "{name}");
                    for (conversion, original) in resolved.conversions.iter().zip(&inputs) {
                        // Result normalization must not rewrite the rigid input
                        // type or convert a live extent into a fixed one.
                        assert_eq!(&conversion.source, original);
                        assert_eq!(&conversion.target, original);
                        assert_eq!(conversion.cost, 0);
                    }
                }
            }
        }
    }
}

#[test]
fn distinct_live_broadcast_axes_do_not_require_rigid_input_equality() {
    for name in ["math/add", "math/mul"] {
        let mut live = turn_row_matrix(BuiltinScalarKind::F64, 1);
        let bounded = ResolvedType::new(
            live.kind().clone(),
            vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Constant(64)),
            }]
            .into_boxed_slice(),
        )
        .unwrap();
        assert_ne!(live, bounded);
        for other in [bounded, live.clone()] {
            for inputs in [[live.clone(), other.clone()], [other.clone(), live.clone()]] {
                let resolved = resolve_named_overload(name, &inputs).unwrap();
                assert_eq!(resolved.outputs.len(), 1);
                assert!(resolved.conversions.iter().all(|plan| plan.cost == 0));
            }
            // Check both declaration orders without making them equal.
            live = other;
        }
    }
}

#[test]
fn live_broadcast_normalization_preserves_explicit_expected_output_authority() {
    for axis in 0..2 {
        let live = turn_axis_matrix(BuiltinScalarKind::F64, axis, 1);
        let fixed = if axis == 0 {
            fixed_matrix(BuiltinScalarKind::F64, 10, 1)
        } else {
            fixed_matrix(BuiltinScalarKind::F64, 1, 10)
        };
        for inputs in [[live.clone(), fixed.clone()], [fixed.clone(), live.clone()]] {
            for expected in [live.clone(), fixed.clone()] {
                let resolved = resolve_named_overload_with_expected(
                    "math/mul",
                    &inputs,
                    Some(core::slice::from_ref(&expected)),
                )
                .unwrap();
                assert_eq!(resolved.outputs.as_ref(), core::slice::from_ref(&expected));
                assert!(resolved.conversions.iter().all(|plan| plan.cost == 0));
            }
        }
    }
}

#[test]
fn whole_value_assignment_requires_exact_matrix_dimensions() {
    let matrix = fixed_matrix(BuiltinScalarKind::F64, 2, 3);
    let scalar = scalar(BuiltinScalarKind::F64);
    for name in [
        "math/add-assign",
        "math/sub-assign",
        "math/mul-assign",
        "math/div-assign",
    ] {
        let scalar_assignment =
            resolve_named_overload(name, &[matrix.clone(), scalar.clone()]).unwrap();
        assert_eq!(scalar_assignment.outputs.as_ref(), &[matrix.clone()]);

        let matrix_assignment =
            resolve_named_overload(name, &[matrix.clone(), matrix.clone()]).unwrap();
        assert_eq!(matrix_assignment.outputs.as_ref(), &[matrix.clone()]);

        assert!(
            resolve_named_overload(
                name,
                &[matrix.clone(), fixed_matrix(BuiltinScalarKind::F64, 3, 2)],
            )
            .is_err(),
            "{name} admitted an incompatible whole-matrix source"
        );
    }
}

#[test]
fn boolean_broadcast_overloads_preserve_axes_in_both_operand_orders() {
    for name in ["logic/and", "logic/or", "logic/xor"] {
        let matrix = fixed_matrix(BuiltinScalarKind::Bool, 2, 3);
        for other in [
            scalar(BuiltinScalarKind::Bool),
            fixed_matrix(BuiltinScalarKind::Bool, 2, 1),
            fixed_matrix(BuiltinScalarKind::Bool, 1, 3),
        ] {
            for inputs in [
                [matrix.clone(), other.clone()],
                [other.clone(), matrix.clone()],
            ] {
                let resolved = resolve_named_overload(name, &inputs).unwrap();
                assert_eq!(resolved.outputs.as_ref(), &[matrix.clone()]);
                assert!(resolved.conversions.iter().all(|plan| plan.cost == 0));
            }
        }
        for other in [
            fixed_matrix(BuiltinScalarKind::Bool, 3, 1),
            fixed_matrix(BuiltinScalarKind::Bool, 1, 4),
            fixed_matrix(BuiltinScalarKind::Bool, 3, 2),
        ] {
            assert!(resolve_named_overload(name, &[matrix.clone(), other]).is_err());
        }
        let changing_matrix = turn_row_matrix(BuiltinScalarKind::Bool, 3);
        let changing_column = turn_row_matrix(BuiltinScalarKind::Bool, 1);
        for inputs in [
            [changing_matrix.clone(), changing_column.clone()],
            [changing_column, changing_matrix.clone()],
        ] {
            let resolved = resolve_named_overload(name, &inputs).unwrap();
            assert_eq!(resolved.outputs.as_ref(), &[changing_matrix.clone()]);
        }
    }
}

#[test]
fn complex_absolute_value_matches_the_maintained_runtime_result_kind() {
    let complex = scalar(BuiltinScalarKind::C64);
    let resolved = resolve_named_overload("math/abs", core::slice::from_ref(&complex)).unwrap();
    assert_eq!(resolved.outputs.as_ref(), core::slice::from_ref(&complex));

    let matrix = fixed_matrix(BuiltinScalarKind::C64, 2, 3);
    let resolved = resolve_named_overload("math/abs", core::slice::from_ref(&matrix)).unwrap();
    assert_eq!(resolved.outputs.as_ref(), core::slice::from_ref(&matrix));
}

#[test]
fn rational_power_selects_its_exact_integral_exponent() {
    let rational = scalar(BuiltinScalarKind::R64);
    let exponent = scalar(BuiltinScalarKind::I32);
    let resolved =
        resolve_named_overload("math/pow", &[rational.clone(), exponent.clone()]).unwrap();
    assert_eq!(resolved.outputs.as_ref(), &[rational.clone()]);
    assert_eq!(resolved.conversions[0].target, rational);
    assert_eq!(resolved.conversions[1].target, exponent);
    assert_eq!(resolved.conversion_count, 0);
    assert!(
        resolved
            .conversions
            .iter()
            .all(|plan| matches!(plan.step, mech_core::ConversionStep::Identity))
    );
}
