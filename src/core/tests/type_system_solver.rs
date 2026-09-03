use mech_core::*;

fn dimension(
    id: u32,
    lifetime: DimensionLifetime,
    upper: Option<u64>,
) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(id),
        origin: DimensionParameterOrigin::Inferred,
        lifetime,
        lower_bound: DimensionExpr::Constant(0),
        upper_bound: upper.map(DimensionExpr::Constant),
    }
}

fn ranged_dimension(id: u32, lower: u64, upper: Option<u64>) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(id),
        origin: DimensionParameterOrigin::Inferred,
        lifetime: DimensionLifetime::Turn,
        lower_bound: DimensionExpr::Constant(lower),
        upper_bound: upper.map(DimensionExpr::Constant),
    }
}

fn matrix_type(
    dimensions: Vec<DimensionExpr>,
    declarations: Vec<DimensionParameterDeclaration>,
) -> ResolvedType {
    ResolvedType::new(
        KindExpr::Matrix {
            element: Box::new(BuiltinScalarKind::F64.kind_expr()),
            dimensions: dimensions.into_boxed_slice(),
        },
        declarations.into_boxed_slice(),
    )
    .unwrap()
}

fn square_scheme(lifetime: DimensionLifetime) -> KindScheme {
    let axis = DimensionParameterId::new(0);
    KindScheme::new(
        Box::new([]),
        vec![dimension(0, lifetime, Some(64))].into_boxed_slice(),
        InputKindScheme::Fixed(
            vec![KindExpr::Matrix {
                element: Box::new(BuiltinScalarKind::F64.kind_expr()),
                dimensions: vec![
                    DimensionExpr::Parameter(axis),
                    DimensionExpr::Parameter(axis),
                ]
                .into_boxed_slice(),
            }]
            .into_boxed_slice(),
        ),
        vec![BuiltinScalarKind::F64.kind_expr()].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap()
}

#[test]
fn dimension_inequality_requires_guaranteed_interval_endpoints() {
    let dimensions = vec![
        dimension(0, DimensionLifetime::Turn, None),
        dimension(1, DimensionLifetime::Turn, None),
    ];
    let scheme = KindScheme::new(
        Box::new([]),
        dimensions.into_boxed_slice(),
        InputKindScheme::Fixed(
            vec![KindExpr::Matrix {
                element: Box::new(BuiltinScalarKind::F64.kind_expr()),
                dimensions: vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                ]
                .into_boxed_slice(),
            }]
            .into_boxed_slice(),
        ),
        vec![BuiltinScalarKind::Bool.kind_expr()].into_boxed_slice(),
        vec![KindConstraint::DimensionLessEqual(
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
            DimensionExpr::Parameter(DimensionParameterId::new(1)),
        )]
        .into_boxed_slice(),
    )
    .unwrap();

    let overlapping = matrix_type(
        vec![
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
            DimensionExpr::Parameter(DimensionParameterId::new(1)),
        ],
        vec![
            ranged_dimension(0, 0, Some(5)),
            ranged_dimension(1, 4, Some(10)),
        ],
    );
    assert!(
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("interval/order", None))
            .solve_scheme(&scheme, &[overlapping], None)
            .is_err()
    );

    let separated = matrix_type(
        vec![
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
            DimensionExpr::Parameter(DimensionParameterId::new(1)),
        ],
        vec![
            ranged_dimension(0, 0, Some(4)),
            ranged_dimension(1, 5, Some(10)),
        ],
    );
    TypeConstraintEnvironment::new(TypeConstraintOrigin::new("interval/order", None))
        .solve_scheme(&scheme, &[separated], None)
        .unwrap();
}

#[test]
fn dynamic_lower_bound_requires_actual_minimum_above_declared_maximum() {
    let lower = DimensionParameterId::new(0);
    let actual = DimensionParameterId::new(1);
    let scheme = KindScheme::new(
        Box::new([]),
        vec![
            dimension(0, DimensionLifetime::Turn, None),
            DimensionParameterDeclaration {
                id: actual,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Parameter(lower),
                upper_bound: None,
            },
        ]
        .into_boxed_slice(),
        InputKindScheme::Fixed(
            vec![KindExpr::Matrix {
                element: Box::new(BuiltinScalarKind::F64.kind_expr()),
                dimensions: vec![
                    DimensionExpr::Parameter(lower),
                    DimensionExpr::Parameter(actual),
                ]
                .into_boxed_slice(),
            }]
            .into_boxed_slice(),
        ),
        vec![BuiltinScalarKind::Bool.kind_expr()].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap();
    let overlapping = matrix_type(
        vec![
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
            DimensionExpr::Parameter(DimensionParameterId::new(1)),
        ],
        vec![
            ranged_dimension(0, 0, Some(5)),
            ranged_dimension(1, 4, Some(10)),
        ],
    );
    assert!(
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("interval/lower", None))
            .solve_scheme(&scheme, &[overlapping], None)
            .is_err()
    );
}

fn solve(
    scheme: &KindScheme,
    input: ResolvedType,
) -> Result<SchemeResolution, TypeResolutionError> {
    TypeConstraintEnvironment::new(TypeConstraintOrigin::new("matrix/square", None)).solve_scheme(
        scheme,
        &[input],
        None,
    )
}

#[test]
fn square_scheme_rejects_independent_equal_bounded_axes() {
    let input = matrix_type(
        vec![
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
            DimensionExpr::Parameter(DimensionParameterId::new(1)),
        ],
        vec![
            dimension(0, DimensionLifetime::Activation, Some(64)),
            dimension(1, DimensionLifetime::Activation, Some(64)),
        ],
    );
    assert!(matches!(
        solve(&square_scheme(DimensionLifetime::Activation), input),
        Err(TypeResolutionError::Incompatible { ref failures, .. })
            if failures.iter().any(|failure| matches!(failure, TypeConstraintFailure::IncompatibleDimensions { .. }))
    ));
}

#[test]
fn square_scheme_accepts_one_shared_axis_expression() {
    let axis = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let input = matrix_type(
        vec![axis.clone(), axis],
        vec![dimension(0, DimensionLifetime::Activation, Some(64))],
    );
    assert!(solve(&square_scheme(DimensionLifetime::Activation), input).is_ok());
}

#[test]
fn imported_rigid_dimensions_are_never_aliased() {
    square_scheme_rejects_independent_equal_bounded_axes();
}

#[test]
fn alpha_equivalent_types_share_rigid_dimension_identity() {
    let actual = matrix_type(
        vec![DimensionExpr::Parameter(DimensionParameterId::new(0))],
        vec![dimension(0, DimensionLifetime::Activation, Some(64))],
    );
    let scheme_axis = DimensionParameterId::new(0);
    let repeated = KindScheme::new(
        Box::new([]),
        vec![dimension(0, DimensionLifetime::Activation, Some(64))].into_boxed_slice(),
        InputKindScheme::Fixed(
            vec![
                KindExpr::Matrix {
                    element: Box::new(BuiltinScalarKind::F64.kind_expr()),
                    dimensions: vec![DimensionExpr::Parameter(scheme_axis)].into_boxed_slice(),
                },
                KindExpr::Matrix {
                    element: Box::new(BuiltinScalarKind::F64.kind_expr()),
                    dimensions: vec![DimensionExpr::Parameter(scheme_axis)].into_boxed_slice(),
                },
            ]
            .into_boxed_slice(),
        ),
        vec![BuiltinScalarKind::F64.kind_expr()].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap();
    assert!(
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("matrix/repeated", None))
            .solve_scheme(&repeated, &[actual.clone(), actual], None)
            .is_ok()
    );
}

#[test]
fn activation_dimension_rejects_compound_turn_expression() {
    let turn = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let input = matrix_type(
        vec![DimensionExpr::Add(
            vec![turn, DimensionExpr::Constant(1)].into_boxed_slice(),
        )],
        vec![dimension(0, DimensionLifetime::Turn, Some(63))],
    );
    let scheme_axis = DimensionParameterId::new(0);
    let scheme = KindScheme::new(
        Box::new([]),
        vec![dimension(0, DimensionLifetime::Activation, Some(64))].into_boxed_slice(),
        InputKindScheme::Fixed(
            vec![KindExpr::Matrix {
                element: Box::new(BuiltinScalarKind::F64.kind_expr()),
                dimensions: vec![DimensionExpr::Parameter(scheme_axis)].into_boxed_slice(),
            }]
            .into_boxed_slice(),
        ),
        vec![BuiltinScalarKind::F64.kind_expr()].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap();
    assert!(matches!(
        solve(&scheme, input),
        Err(TypeResolutionError::Incompatible { ref failures, .. })
            if failures.iter().any(|failure| matches!(failure, TypeConstraintFailure::InvalidDynamicFixedExtent { .. }))
    ));
}

#[test]
fn activation_dimension_rejects_bare_turn_parameter() {
    let axis = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let input = matrix_type(
        vec![axis.clone(), axis],
        vec![dimension(0, DimensionLifetime::Turn, Some(64))],
    );
    assert!(matches!(
        solve(&square_scheme(DimensionLifetime::Activation), input),
        Err(TypeResolutionError::Incompatible { ref failures, .. })
            if failures.iter().any(|failure| matches!(failure, TypeConstraintFailure::InvalidDynamicFixedExtent { .. }))
    ));
}

#[test]
fn turn_dimension_accepts_fixed_and_activation_expressions() {
    let turn_scheme = square_scheme(DimensionLifetime::Turn);
    let fixed = matrix_type(
        vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)],
        vec![],
    );
    assert!(solve(&turn_scheme, fixed).is_ok());

    let axis = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let activation = matrix_type(
        vec![axis.clone(), axis],
        vec![dimension(0, DimensionLifetime::Activation, Some(64))],
    );
    assert!(solve(&turn_scheme, activation).is_ok());
}

#[test]
fn turn_dimension_accepts_fixed_expression() {
    let input = matrix_type(
        vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)],
        vec![],
    );
    assert!(solve(&square_scheme(DimensionLifetime::Turn), input).is_ok());
}

#[test]
fn turn_dimension_accepts_activation_expression() {
    let axis = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let input = matrix_type(
        vec![axis.clone(), axis],
        vec![dimension(0, DimensionLifetime::Activation, Some(64))],
    );
    assert!(solve(&square_scheme(DimensionLifetime::Turn), input).is_ok());
}

#[test]
fn bounded_turn_rejects_unbounded_turn_expression() {
    let axis = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let input = matrix_type(
        vec![axis.clone(), axis],
        vec![dimension(0, DimensionLifetime::Turn, None)],
    );
    let result = solve(&square_scheme(DimensionLifetime::Turn), input);
    assert!(
        matches!(
            result,
            Err(TypeResolutionError::Incompatible { ref failures, .. })
                if failures.iter().any(|failure| matches!(failure, TypeConstraintFailure::DimensionBoundNotProven { .. }))
        ),
        "{result:?}"
    );
}

#[test]
fn dimension_bounds_are_checked_for_compound_expressions() {
    let axis = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let compound = DimensionExpr::Add(vec![axis, DimensionExpr::Constant(1)].into_boxed_slice());
    let input = matrix_type(
        vec![compound.clone(), compound],
        vec![dimension(0, DimensionLifetime::Turn, Some(64))],
    );
    let result = solve(&square_scheme(DimensionLifetime::Turn), input);
    assert!(
        matches!(
            result,
            Err(TypeResolutionError::Incompatible { ref failures, .. })
                if failures.iter().any(|failure| matches!(failure, TypeConstraintFailure::DimensionBoundNotProven { .. }))
        ),
        "{result:?}"
    );
}

#[test]
fn cyclic_kind_binding_is_structured() {
    let parameter = KindParameterId::new(0);
    let scheme = KindScheme::new(
        vec![KindParameter {
            id: parameter,
            upper_bound: None,
        }]
        .into_boxed_slice(),
        Box::new([]),
        InputKindScheme::Fixed(Box::new([])),
        vec![KindExpr::Parameter(parameter)].into_boxed_slice(),
        vec![KindConstraint::Equal(
            KindExpr::Parameter(parameter),
            KindExpr::Option(Box::new(KindExpr::Parameter(parameter))),
        )]
        .into_boxed_slice(),
    )
    .unwrap();
    let error = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("cyclic/kind", None))
        .solve_scheme(&scheme, &[], None)
        .unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.iter().any(|failure| matches!(failure, TypeConstraintFailure::CyclicKindBinding { .. }))
    ));
}

#[test]
fn cyclic_dimension_binding_is_structured() {
    let axis = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let scheme = KindScheme::new(
        Box::new([]),
        vec![dimension(0, DimensionLifetime::Turn, None)].into_boxed_slice(),
        InputKindScheme::Fixed(Box::new([])),
        vec![KindExpr::Matrix {
            element: Box::new(BuiltinScalarKind::F64.kind_expr()),
            dimensions: vec![axis.clone()].into_boxed_slice(),
        }]
        .into_boxed_slice(),
        vec![KindConstraint::DimensionEqual(
            axis.clone(),
            DimensionExpr::Add(vec![axis, DimensionExpr::Constant(1)].into_boxed_slice()),
        )]
        .into_boxed_slice(),
    )
    .unwrap();
    let error = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("cyclic/dimension", None))
        .solve_scheme(&scheme, &[], None)
        .unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.iter().any(|failure| matches!(failure, TypeConstraintFailure::CyclicDimensionBinding { .. }))
    ));
}

#[test]
fn unresolved_outputs_are_structured() {
    let parameter = KindParameterId::new(0);
    let scheme = KindScheme::new(
        vec![KindParameter {
            id: parameter,
            upper_bound: None,
        }]
        .into_boxed_slice(),
        Box::new([]),
        InputKindScheme::Fixed(Box::new([])),
        vec![KindExpr::Parameter(parameter)].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap();
    let error = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("unresolved", None))
        .solve_scheme(&scheme, &[], None)
        .unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.iter().any(|failure| matches!(failure, TypeConstraintFailure::UnresolvedKindVariable { .. }))
    ));
}

#[test]
fn normalization_is_idempotent() {
    let expression = DimensionExpr::Add(
        vec![
            DimensionExpr::Constant(0),
            DimensionExpr::Constant(2),
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
        ]
        .into_boxed_slice(),
    );
    let normalized = normalize_dimension(&expression, 1).unwrap();
    assert_eq!(normalized, normalize_dimension(&normalized, 1).unwrap());
}

#[test]
fn commutative_dimension_permutations_are_equal() {
    let left = DimensionExpr::Add(
        vec![
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
            DimensionExpr::Constant(3),
        ]
        .into_boxed_slice(),
    );
    let right = DimensionExpr::Add(
        vec![
            DimensionExpr::Constant(3),
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
        ]
        .into_boxed_slice(),
    );
    assert_eq!(
        normalize_dimension(&left, 1).unwrap(),
        normalize_dimension(&right, 1).unwrap(),
    );
}

fn scalar(kind: BuiltinScalarKind) -> ResolvedType {
    ResolvedType::new(kind.kind_expr(), Box::new([])).unwrap()
}

fn generic_scalar_scheme(output: KindExpr, constraints: Vec<KindConstraint>) -> KindScheme {
    KindScheme::new(
        vec![KindParameter {
            id: KindParameterId::new(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
        Box::new([]),
        InputKindScheme::Fixed(
            vec![KindExpr::Parameter(KindParameterId::new(0))].into_boxed_slice(),
        ),
        vec![output].into_boxed_slice(),
        constraints.into_boxed_slice(),
    )
    .unwrap()
}

#[test]
fn exact_concrete_beats_generic() {
    let generic = generic_scalar_scheme(KindExpr::Parameter(KindParameterId::new(0)), vec![]);
    let concrete = exact_unary(
        BuiltinScalarKind::U8.kind_expr(),
        BuiltinScalarKind::U8.kind_expr(),
    )
    .unwrap();
    let result = resolve_type_overloads(
        TypeConstraintOrigin::new("overload/exact", None),
        &[
            TypeOverloadCandidate {
                id: 1,
                scheme: &generic,
            },
            TypeOverloadCandidate {
                id: 2,
                scheme: &concrete,
            },
        ],
        &[scalar(BuiltinScalarKind::U8)],
        None,
    )
    .unwrap();
    assert_eq!(result.candidate_ids.as_ref(), &[2]);
}

#[test]
fn generic_exact_beats_concrete_conversion() {
    let parameter = KindExpr::Parameter(KindParameterId::new(0));
    let generic = generic_scalar_scheme(parameter.clone(), vec![]);
    let converted = generic_scalar_scheme(
        BuiltinScalarKind::U16.kind_expr(),
        vec![KindConstraint::Convertible(
            parameter,
            BuiltinScalarKind::U16.kind_expr(),
        )],
    );
    let result = resolve_type_overloads(
        TypeConstraintOrigin::new("overload/conversion", None),
        &[
            TypeOverloadCandidate {
                id: 1,
                scheme: &converted,
            },
            TypeOverloadCandidate {
                id: 2,
                scheme: &generic,
            },
        ],
        &[scalar(BuiltinScalarKind::U8)],
        None,
    )
    .unwrap();
    assert_eq!(result.candidate_ids.as_ref(), &[2]);
}

#[test]
fn predicate_constrained_beats_unconstrained() {
    let parameter = KindExpr::Parameter(KindParameterId::new(0));
    let unconstrained = generic_scalar_scheme(parameter.clone(), vec![]);
    let constrained = generic_scalar_scheme(
        parameter.clone(),
        vec![KindConstraint::Satisfies {
            kind: parameter,
            predicate: BuiltinKindPredicate::Number,
        }],
    );
    let result = resolve_type_overloads(
        TypeConstraintOrigin::new("overload/predicate", None),
        &[
            TypeOverloadCandidate {
                id: 1,
                scheme: &unconstrained,
            },
            TypeOverloadCandidate {
                id: 2,
                scheme: &constrained,
            },
        ],
        &[scalar(BuiltinScalarKind::I32)],
        None,
    )
    .unwrap();
    assert_eq!(result.candidate_ids.as_ref(), &[2]);
}

#[test]
fn narrower_predicate_beats_broader_predicate() {
    let parameter = KindExpr::Parameter(KindParameterId::new(0));
    let scheme_for = |predicate| {
        generic_scalar_scheme(
            parameter.clone(),
            vec![KindConstraint::Satisfies {
                kind: parameter.clone(),
                predicate,
            }],
        )
    };
    let number = scheme_for(BuiltinKindPredicate::Number);
    let integer = scheme_for(BuiltinKindPredicate::Integer);
    let result = resolve_type_overloads(
        TypeConstraintOrigin::new("overload/narrow", None),
        &[
            TypeOverloadCandidate {
                id: 1,
                scheme: &number,
            },
            TypeOverloadCandidate {
                id: 2,
                scheme: &integer,
            },
        ],
        &[scalar(BuiltinScalarKind::I32)],
        None,
    )
    .unwrap();
    assert_eq!(result.candidate_ids.as_ref(), &[2]);
}

#[test]
fn equal_semantic_alternatives_retain_physical_candidates() {
    let scheme = exact_unary(
        BuiltinScalarKind::U8.kind_expr(),
        BuiltinScalarKind::U8.kind_expr(),
    )
    .unwrap();
    let result = resolve_type_overloads(
        TypeConstraintOrigin::new("overload/physical", None),
        &[
            TypeOverloadCandidate {
                id: 9,
                scheme: &scheme,
            },
            TypeOverloadCandidate {
                id: 4,
                scheme: &scheme,
            },
        ],
        &[scalar(BuiltinScalarKind::U8)],
        None,
    )
    .unwrap();
    assert_eq!(result.candidate_ids.as_ref(), &[4, 9]);
}

#[test]
fn wildcard_inputs_close_as_exact_identity_conversions() {
    let scheme = KindScheme::new(
        Box::new([]),
        Box::new([]),
        InputKindScheme::Fixed(
            vec![KindExpr::Matrix {
                element: Box::new(KindExpr::Wildcard),
                dimensions: vec![DimensionExpr::Constant(2)].into_boxed_slice(),
            }]
            .into_boxed_slice(),
        ),
        vec![BuiltinScalarKind::Bool.kind_expr()].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap();
    let actual = matrix_type(vec![DimensionExpr::Constant(2)], vec![]);
    let result = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("wildcard", None))
        .solve_scheme(&scheme, &[actual.clone()], None)
        .unwrap();

    assert_eq!(result.conversions.len(), 1);
    assert_eq!(result.conversions[0].source, actual);
    assert_eq!(result.conversions[0].source, result.conversions[0].target);
    assert_eq!(result.conversions[0].step, ConversionStep::Identity);
    assert_eq!(result.conversions[0].cost, 0);
}

#[test]
fn different_equal_score_outputs_produce_ambiguity() {
    let first = exact_unary(
        BuiltinScalarKind::U8.kind_expr(),
        BuiltinScalarKind::U8.kind_expr(),
    )
    .unwrap();
    let second = exact_unary(
        BuiltinScalarKind::U8.kind_expr(),
        BuiltinScalarKind::U16.kind_expr(),
    )
    .unwrap();
    assert!(matches!(
        resolve_type_overloads(
            TypeConstraintOrigin::new("overload/ambiguous", None),
            &[
                TypeOverloadCandidate {
                    id: 1,
                    scheme: &first
                },
                TypeOverloadCandidate {
                    id: 2,
                    scheme: &second
                },
            ],
            &[scalar(BuiltinScalarKind::U8)],
            None,
        ),
        Err(TypeResolutionError::Ambiguous { .. })
    ));
}

#[test]
fn registration_order_and_runtime_factory_id_do_not_affect_result() {
    let generic = generic_scalar_scheme(KindExpr::Parameter(KindParameterId::new(0)), vec![]);
    let concrete = exact_unary(
        BuiltinScalarKind::U8.kind_expr(),
        BuiltinScalarKind::U8.kind_expr(),
    )
    .unwrap();
    for candidates in [
        vec![
            TypeOverloadCandidate {
                id: 99,
                scheme: &generic,
            },
            TypeOverloadCandidate {
                id: 2,
                scheme: &concrete,
            },
        ],
        vec![
            TypeOverloadCandidate {
                id: 87,
                scheme: &concrete,
            },
            TypeOverloadCandidate {
                id: 1,
                scheme: &generic,
            },
        ],
    ] {
        let result = resolve_type_overloads(
            TypeConstraintOrigin::new("overload/order", None),
            &candidates,
            &[scalar(BuiltinScalarKind::U8)],
            None,
        )
        .unwrap();
        assert_eq!(result.outputs.as_ref(), &[scalar(BuiltinScalarKind::U8)]);
        assert_eq!(result.candidate_ids.len(), 1);
    }
}

#[test]
fn expected_output_constrains_overloads_without_storage_metadata() {
    let first = exact_unary(
        BuiltinScalarKind::U8.kind_expr(),
        BuiltinScalarKind::U8.kind_expr(),
    )
    .unwrap();
    let second = exact_unary(
        BuiltinScalarKind::U8.kind_expr(),
        BuiltinScalarKind::U16.kind_expr(),
    )
    .unwrap();
    let result = resolve_type_overloads(
        TypeConstraintOrigin::new("overload/expected", None),
        &[
            TypeOverloadCandidate {
                id: 1,
                scheme: &first,
            },
            TypeOverloadCandidate {
                id: 2,
                scheme: &second,
            },
        ],
        &[scalar(BuiltinScalarKind::U8)],
        Some(&[scalar(BuiltinScalarKind::U16)]),
    )
    .unwrap();
    assert_eq!(result.candidate_ids.as_ref(), &[2]);
}

#[test]
fn solver_returns_one_closed_conversion_plan_per_input() {
    let matrix = |element: BuiltinScalarKind| {
        ResolvedType::new(
            KindExpr::Matrix {
                element: Box::new(element.kind_expr()),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                    .into_boxed_slice(),
            },
            Box::new([]),
        )
        .unwrap()
    };
    let scheme = promoted_binary_elementwise().unwrap().remove(1);
    let resolution = TypeConstraintEnvironment::new(TypeConstraintOrigin::new("math/add", None))
        .solve_scheme(
            &scheme,
            &[
                matrix(BuiltinScalarKind::F32),
                matrix(BuiltinScalarKind::F64),
            ],
            None,
        )
        .unwrap();
    assert_eq!(resolution.conversions.len(), 2);
    assert_eq!(
        resolution.conversions[0].target,
        matrix(BuiltinScalarKind::F64)
    );
    assert_eq!(
        resolution.conversions[1].target,
        matrix(BuiltinScalarKind::F64)
    );
    assert!(matches!(
        resolution.conversions[0].step,
        ConversionStep::MatrixElements(_)
    ));
    assert!(matches!(
        resolution.conversions[1].step,
        ConversionStep::Identity
    ));

    let range = range_ternary().unwrap();
    let resolution = TypeConstraintEnvironment::new(TypeConstraintOrigin::new(
        "range/inclusive-increment",
        None,
    ))
    .solve_scheme(
        &range,
        &[
            scalar(BuiltinScalarKind::U8),
            scalar(BuiltinScalarKind::U16),
            scalar(BuiltinScalarKind::U32),
        ],
        None,
    )
    .unwrap();
    assert_eq!(resolution.conversions.len(), 3);
    assert!(
        resolution
            .conversions
            .iter()
            .all(|plan| plan.target == scalar(BuiltinScalarKind::U32))
    );
}

fn table_type(columns: &[(&str, BuiltinScalarKind)], rows: u64) -> ResolvedType {
    ResolvedType::new(
        KindExpr::Table {
            columns: columns
                .iter()
                .map(|(name, kind)| KindField {
                    name: (*name).into(),
                    kind: kind.kind_expr(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            rows: DimensionExpr::Constant(rows),
        },
        Box::new([]),
    )
    .unwrap()
}

#[test]
fn table_join_schemes_close_structural_outputs_without_wildcards() {
    let left = table_type(
        &[
            ("id", BuiltinScalarKind::U64),
            ("left", BuiltinScalarKind::I32),
        ],
        2,
    );
    let right = table_type(
        &[
            ("id", BuiltinScalarKind::U64),
            ("right", BuiltinScalarKind::String),
        ],
        3,
    );
    let declaration = maintained_source_type_declaration("table/full-outer-join").unwrap();
    let scheme = &declaration.overloads[0].scheme;
    let result =
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("table/full-outer-join", None))
            .solve_scheme(scheme, &[left, right], None)
            .unwrap();
    let KindExpr::Table { columns, .. } = result.outputs[0].kind() else {
        panic!("table join must have one closed structural table output")
    };
    assert_eq!(columns.len(), 3);
    assert!(matches!(columns[1].kind, KindExpr::Option(_)));
    assert!(matches!(columns[2].kind, KindExpr::Option(_)));
    assert!(!matches!(result.outputs[0].kind(), KindExpr::Wildcard));

    let incompatible = table_type(&[("id", BuiltinScalarKind::String)], 1);
    let left = table_type(&[("id", BuiltinScalarKind::U64)], 1);
    assert!(
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("table/join", None))
            .solve_scheme(
                &maintained_source_type_declaration("table/join")
                    .unwrap()
                    .overloads[0]
                    .scheme,
                &[left, incompatible],
                None,
            )
            .is_err()
    );
}

#[test]
fn closed_outputs_import_dimension_bound_dependencies_before_their_dependents() {
    let capacity = DimensionParameterId::new(0);
    let cardinality = DimensionParameterId::new(1);
    let resolved = ResolvedType::new(
        KindExpr::Set {
            element: Box::new(BuiltinScalarKind::U8.kind_expr()),
            cardinality: DimensionExpr::Parameter(cardinality),
        },
        vec![
            dimension(0, DimensionLifetime::Turn, None),
            DimensionParameterDeclaration {
                id: cardinality,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Parameter(capacity)),
            },
        ]
        .into_boxed_slice(),
    )
    .unwrap();

    assert_eq!(resolved.dimension_parameters().len(), 2);
    assert_eq!(
        resolved.dimension_parameters()[1].upper_bound,
        Some(DimensionExpr::Parameter(DimensionParameterId::new(0)))
    );
    assert!(matches!(
        resolved.kind(),
        KindExpr::Set {
            cardinality: DimensionExpr::Parameter(id),
            ..
        } if *id == DimensionParameterId::new(1)
    ));
}
