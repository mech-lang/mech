use mech_core::*;

fn resolved(kind: BuiltinScalarKind) -> ResolvedType {
    ResolvedType::new(kind.kind_expr(), Box::new([])).unwrap()
}

fn promoted(left: BuiltinScalarKind, right: BuiltinScalarKind) -> Option<BuiltinScalarKind> {
    plan_numeric_promotion(&resolved(left), &resolved(right))
        .unwrap()
        .map(|plan| match plan.result.kind() {
            KindExpr::Named(id) => BuiltinScalarKind::from_kind_id(*id).unwrap(),
            other => panic!("unexpected promoted kind {other:?}"),
        })
}

fn integer_description(kind: BuiltinScalarKind) -> Option<(bool, u16)> {
    use BuiltinScalarKind as K;
    match kind {
        K::U8 => Some((false, 8)),
        K::U16 => Some((false, 16)),
        K::U32 => Some((false, 32)),
        K::U64 => Some((false, 64)),
        K::U128 => Some((false, 128)),
        K::I8 => Some((true, 8)),
        K::I16 => Some((true, 16)),
        K::I32 => Some((true, 32)),
        K::I64 => Some((true, 64)),
        K::I128 => Some((true, 128)),
        _ => None,
    }
}

fn expected_implicit_scalar(source: BuiltinScalarKind, target: BuiltinScalarKind) -> bool {
    use BuiltinScalarKind as K;
    if source == target {
        return true;
    }
    if let (Some((source_signed, source_width)), Some((target_signed, target_width))) =
        (integer_description(source), integer_description(target))
    {
        return source_signed == target_signed && source_width < target_width
            || !source_signed && target_signed && source_width < target_width;
    }
    match (source, target) {
        (K::F32, K::F64) | (K::C32, K::C64) => true,
        (K::U8 | K::U16 | K::I8 | K::I16, K::F32) => true,
        (K::U8 | K::U16 | K::U32 | K::I8 | K::I16 | K::I32, K::F64) => true,
        (K::U8 | K::U16 | K::U32 | K::I8 | K::I16 | K::I32 | K::I64, K::R64) => true,
        (K::F32, K::C32 | K::C64) | (K::F64, K::C64) => true,
        (integer, K::C32) => expected_implicit_scalar(integer, K::F32),
        (integer, K::C64) => expected_implicit_scalar(integer, K::F64),
        _ => false,
    }
}

fn expected_promotion(
    left: BuiltinScalarKind,
    right: BuiltinScalarKind,
) -> Option<BuiltinScalarKind> {
    use BuiltinScalarKind as K;
    if left == right {
        return Some(left);
    }
    let mut candidates = Vec::new();
    if integer_description(left).is_some() && integer_description(right).is_some() {
        candidates.extend([
            K::U8,
            K::I8,
            K::U16,
            K::I16,
            K::U32,
            K::I32,
            K::U64,
            K::I64,
            K::U128,
            K::I128,
        ]);
    }
    if left == K::R64 || right == K::R64 {
        candidates.push(K::R64);
    }
    if matches!(left, K::F32 | K::F64) || matches!(right, K::F32 | K::F64) {
        candidates.extend([K::F32, K::F64]);
    }
    if matches!(left, K::C32 | K::C64) || matches!(right, K::C32 | K::C64) {
        candidates.extend([K::C32, K::C64]);
    }
    candidates.into_iter().find(|candidate| {
        expected_implicit_scalar(left, *candidate) && expected_implicit_scalar(right, *candidate)
    })
}

#[test]
fn complete_numeric_promotion_matrix_is_symmetric_and_deterministic() {
    let numeric = BuiltinScalarKind::ALL
        .into_iter()
        .filter(|kind| kind.satisfies(BuiltinKindPredicate::Number))
        .collect::<Vec<_>>();
    for left in numeric.iter().copied() {
        for right in numeric.iter().copied() {
            let forward = plan_numeric_promotion(&resolved(left), &resolved(right)).unwrap();
            let reverse = plan_numeric_promotion(&resolved(right), &resolved(left)).unwrap();
            assert_eq!(
                forward.as_ref().map(|plan| plan.result.clone()),
                reverse.as_ref().map(|plan| plan.result.clone()),
                "promotion differs for {left:?}, {right:?}"
            );
            assert_eq!(
                forward.as_ref().map(|plan| match plan.result.kind() {
                    KindExpr::Named(id) => BuiltinScalarKind::from_kind_id(*id).unwrap(),
                    other => panic!("unexpected promoted kind {other:?}"),
                }),
                expected_promotion(left, right),
                "unexpected promotion for {left:?}, {right:?}",
            );
            if let (Some(forward), Some(reverse)) = (forward, reverse) {
                assert_eq!(forward.left.source, resolved(left));
                assert_eq!(forward.right.source, resolved(right));
                assert_eq!(forward.left.target, forward.result);
                assert_eq!(forward.right.target, forward.result);
                assert_eq!(forward.left.cost, reverse.right.cost);
                assert_eq!(forward.right.cost, reverse.left.cost);
            }
        }
    }
}

#[test]
fn numeric_promotion_examples_match_type_system_v1() {
    use BuiltinScalarKind as K;
    for (left, right, expected) in [
        (K::U8, K::U16, Some(K::U16)),
        (K::U8, K::I8, Some(K::I16)),
        (K::U32, K::I32, Some(K::I64)),
        (K::U64, K::I64, Some(K::I128)),
        (K::U128, K::I128, None),
        (K::F32, K::F64, Some(K::F64)),
        (K::U16, K::F32, Some(K::F32)),
        (K::U32, K::F32, Some(K::F64)),
        (K::I64, K::F64, None),
        (K::U32, K::R64, Some(K::R64)),
        (K::U64, K::R64, None),
        (K::R64, K::F64, None),
        (K::F32, K::C32, Some(K::C32)),
        (K::F64, K::C32, Some(K::C64)),
        (K::C32, K::C64, Some(K::C64)),
    ] {
        assert_eq!(promoted(left, right), expected, "{left:?} + {right:?}");
        assert_eq!(promoted(right, left), expected, "{right:?} + {left:?}");
    }
}

#[test]
fn implicit_conversion_is_exactly_the_lossless_table() {
    use BuiltinScalarKind as K;
    for source in BuiltinScalarKind::ALL {
        for target in BuiltinScalarKind::ALL {
            assert_eq!(
                plan_implicit_conversion(&resolved(source), &resolved(target)).is_ok(),
                expected_implicit_scalar(source, target),
                "implicit conversion policy differs for {source:?} -> {target:?}",
            );
        }
    }

    assert!(plan_implicit_conversion(&resolved(K::U8), &resolved(K::U16)).is_ok());
    assert!(plan_implicit_conversion(&resolved(K::U8), &resolved(K::I16)).is_ok());
    assert!(plan_implicit_conversion(&resolved(K::U16), &resolved(K::F32)).is_ok());
    assert!(plan_implicit_conversion(&resolved(K::U32), &resolved(K::F64)).is_ok());
    assert!(plan_implicit_conversion(&resolved(K::F32), &resolved(K::C32)).is_ok());
    assert!(plan_implicit_conversion(&resolved(K::C32), &resolved(K::C64)).is_ok());

    assert!(plan_implicit_conversion(&resolved(K::U64), &resolved(K::F64)).is_err());
    assert!(plan_implicit_conversion(&resolved(K::I64), &resolved(K::F64)).is_err());
    assert!(plan_implicit_conversion(&resolved(K::U32), &resolved(K::F32)).is_err());
    assert!(plan_implicit_conversion(&resolved(K::R64), &resolved(K::F64)).is_err());
    assert!(plan_implicit_conversion(&resolved(K::String), &resolved(K::U8)).is_err());
}

#[test]
fn explicit_cast_policy_is_separate_from_implicit_conversion() {
    use BuiltinScalarKind as K;
    assert!(plan_explicit_cast(&resolved(K::F64), &resolved(K::I32)).is_ok());
    assert!(plan_explicit_cast(&resolved(K::I64), &resolved(K::F64)).is_ok());
    assert!(plan_explicit_cast(&resolved(K::C64), &resolved(K::F64)).is_ok());
    assert!(plan_explicit_cast(&resolved(K::R64), &resolved(K::I64)).is_ok());
    assert!(plan_explicit_cast(&resolved(K::Bool), &resolved(K::String)).is_ok());
    assert!(plan_explicit_cast(&resolved(K::U128), &resolved(K::String)).is_ok());
    assert!(plan_explicit_cast(&resolved(K::String), &resolved(K::F64)).is_err());
    assert!(plan_explicit_cast(&resolved(K::String), &resolved(K::Bool)).is_err());
    assert!(plan_explicit_cast(&resolved(K::F64), &resolved(K::R64)).is_err());
}

#[test]
fn matrix_and_option_plans_preserve_structure() {
    let dimensions =
        vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into_boxed_slice();
    let matrix = |element: BuiltinScalarKind| {
        ResolvedType::new(
            KindExpr::Matrix {
                element: Box::new(element.kind_expr()),
                dimensions: dimensions.clone(),
            },
            Box::new([]),
        )
        .unwrap()
    };
    let plan = plan_implicit_conversion(
        &matrix(BuiltinScalarKind::F32),
        &matrix(BuiltinScalarKind::F64),
    )
    .unwrap();
    assert!(matches!(plan.step, ConversionStep::MatrixElements(_)));

    let option = |element: BuiltinScalarKind| {
        ResolvedType::new(
            KindExpr::Option(Box::new(element.kind_expr())),
            Box::new([]),
        )
        .unwrap()
    };
    assert!(matches!(
        plan_implicit_conversion(
            &option(BuiltinScalarKind::U8),
            &option(BuiltinScalarKind::U16)
        )
        .unwrap()
        .step,
        ConversionStep::OptionPayload(_)
    ));
}
