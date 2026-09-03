//! Storage-blind schemes for the maintained first-order source operations.

use crate::{
    BuiltinKindPredicate, BuiltinScalarKind, DimensionExpr, DimensionLifetime,
    DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin, InputKindScheme,
    KindConstraint, KindExpr, KindParameter, KindParameterId, KindScheme, ResolvedType,
    SemanticModelError, SourceSchemeTemplate, TableJoinMode,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec, vec::Vec};

fn kind(id: u32) -> KindExpr {
    KindExpr::Parameter(KindParameterId::new(id))
}

fn dim(id: u32) -> DimensionExpr {
    DimensionExpr::Parameter(DimensionParameterId::new(id))
}

fn matrix(element: KindExpr, rows: DimensionExpr, columns: DimensionExpr) -> KindExpr {
    KindExpr::Matrix {
        element: Box::new(element),
        dimensions: vec![rows, columns].into_boxed_slice(),
    }
}

fn set(element: KindExpr, cardinality: DimensionExpr) -> KindExpr {
    KindExpr::Set {
        element: Box::new(element),
        cardinality,
    }
}

fn parameters(count: u32) -> Box<[KindParameter]> {
    (0..count)
        .map(|id| KindParameter {
            id: KindParameterId::new(id),
            upper_bound: None,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn dimensions(count: u32) -> Box<[DimensionParameterDeclaration]> {
    (0..count)
        .map(|id| DimensionParameterDeclaration {
            id: DimensionParameterId::new(id),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn bounded_dimensions(
    input_count: u32,
    output_upper_bound: Option<DimensionExpr>,
) -> Box<[DimensionParameterDeclaration]> {
    let mut declarations = dimensions(input_count).into_vec();
    declarations.push(DimensionParameterDeclaration {
        id: DimensionParameterId::new(input_count),
        origin: DimensionParameterOrigin::Inferred,
        lifetime: DimensionLifetime::Turn,
        lower_bound: DimensionExpr::Constant(0),
        upper_bound: output_upper_bound,
    });
    declarations.into_boxed_slice()
}

fn make(
    kinds: u32,
    dims: u32,
    inputs: Vec<KindExpr>,
    outputs: Vec<KindExpr>,
    constraints: Vec<KindConstraint>,
) -> Result<KindScheme, SemanticModelError> {
    make_with_dimensions(kinds, dimensions(dims), inputs, outputs, constraints)
}

fn make_with_dimensions(
    kinds: u32,
    dimensions: Box<[DimensionParameterDeclaration]>,
    inputs: Vec<KindExpr>,
    outputs: Vec<KindExpr>,
    constraints: Vec<KindConstraint>,
) -> Result<KindScheme, SemanticModelError> {
    KindScheme::new(
        parameters(kinds),
        dimensions,
        InputKindScheme::Fixed(inputs.into_boxed_slice()),
        outputs.into_boxed_slice(),
        constraints.into_boxed_slice(),
    )
}

fn variadic(
    kinds: u32,
    dims: u32,
    repeated: KindExpr,
    minimum: u32,
    output: KindExpr,
    constraints: Vec<KindConstraint>,
) -> Result<KindScheme, SemanticModelError> {
    KindScheme::new(
        parameters(kinds),
        dimensions(dims),
        InputKindScheme::Variadic {
            prefix: Box::new([]),
            repeated,
            min_repetitions: minimum,
        },
        vec![output].into_boxed_slice(),
        constraints.into_boxed_slice(),
    )
}

pub fn exact_unary(input: KindExpr, output: KindExpr) -> Result<KindScheme, SemanticModelError> {
    make(0, 0, vec![input], vec![output], Vec::new())
}

pub fn exact_binary(
    left: KindExpr,
    right: KindExpr,
    output: KindExpr,
) -> Result<KindScheme, SemanticModelError> {
    make(0, 0, vec![left, right], vec![output], Vec::new())
}

pub fn predicate_unary_same(
    predicate: BuiltinKindPredicate,
) -> Result<Vec<KindScheme>, SemanticModelError> {
    let scalar = make(
        1,
        0,
        vec![kind(0)],
        vec![kind(0)],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate,
        }],
    )?;
    let matrix_kind = matrix(kind(0), dim(0), dim(1));
    let matrix = make(
        1,
        2,
        vec![matrix_kind.clone()],
        vec![matrix_kind],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate,
        }],
    )?;
    Ok(vec![scalar, matrix])
}

pub fn promoted_binary_scalar() -> Result<KindScheme, SemanticModelError> {
    make(
        3,
        0,
        vec![kind(0), kind(1)],
        vec![kind(2)],
        vec![
            KindConstraint::Satisfies {
                kind: kind(0),
                predicate: BuiltinKindPredicate::Number,
            },
            KindConstraint::Satisfies {
                kind: kind(1),
                predicate: BuiltinKindPredicate::Number,
            },
            KindConstraint::Promotes {
                left: kind(0),
                right: kind(1),
                output: kind(2),
            },
        ],
    )
}

pub fn promoted_binary_elementwise() -> Result<Vec<KindScheme>, SemanticModelError> {
    let mut schemes = vec![promoted_binary_scalar()?];
    for (left_matrix, right_matrix) in [(true, true), (true, false), (false, true)] {
        let shaped_left = matrix(kind(0), dim(0), dim(1));
        let shaped_right = matrix(kind(1), dim(0), dim(1));
        schemes.push(make(
            3,
            2,
            vec![
                if left_matrix { shaped_left } else { kind(0) },
                if right_matrix { shaped_right } else { kind(1) },
            ],
            vec![matrix(kind(2), dim(0), dim(1))],
            vec![
                KindConstraint::Satisfies {
                    kind: kind(0),
                    predicate: BuiltinKindPredicate::Number,
                },
                KindConstraint::Satisfies {
                    kind: kind(1),
                    predicate: BuiltinKindPredicate::Number,
                },
                KindConstraint::Promotes {
                    left: kind(0),
                    right: kind(1),
                    output: kind(2),
                },
            ],
        )?);
    }
    schemes.push(make(
        3,
        6,
        vec![
            matrix(kind(0), dim(0), dim(1)),
            matrix(kind(1), dim(2), dim(3)),
        ],
        vec![matrix(kind(2), dim(4), dim(5))],
        vec![
            KindConstraint::Satisfies {
                kind: kind(0),
                predicate: BuiltinKindPredicate::Number,
            },
            KindConstraint::Satisfies {
                kind: kind(1),
                predicate: BuiltinKindPredicate::Number,
            },
            KindConstraint::Promotes {
                left: kind(0),
                right: kind(1),
                output: kind(2),
            },
        ],
    )?);
    Ok(schemes)
}

pub fn comparison_exact() -> Result<Vec<KindScheme>, SemanticModelError> {
    let bool_kind = BuiltinScalarKind::Bool.kind_expr();
    let scalar = make(
        1,
        0,
        vec![kind(0), kind(0)],
        vec![bool_kind.clone()],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Equatable,
        }],
    )?;
    let shaped = matrix(kind(0), dim(0), dim(1));
    let matrix = make(
        1,
        2,
        vec![shaped.clone(), shaped],
        vec![matrix(bool_kind, dim(0), dim(1))],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Equatable,
        }],
    )?;
    Ok(vec![scalar, matrix])
}

fn strict_comparison_exact() -> Result<Vec<KindScheme>, SemanticModelError> {
    // Strict equality is the language spelling of exact type equality. It
    // never inserts a conversion and therefore requires one identical,
    // equatable semantic kind on both sides. Matrix extents may be described
    // by different fixed and turn-varying expressions, so retain exact
    // element kinds while proving the two axes compatible.
    let boolean = BuiltinScalarKind::Bool.kind_expr();
    Ok(vec![
        make(
            1,
            0,
            vec![kind(0), kind(0)],
            vec![boolean.clone()],
            vec![KindConstraint::Satisfies {
                kind: kind(0),
                predicate: BuiltinKindPredicate::Equatable,
            }],
        )?,
        make(
            1,
            4,
            vec![
                matrix(kind(0), dim(0), dim(1)),
                matrix(kind(0), dim(2), dim(3)),
            ],
            vec![boolean],
            vec![
                KindConstraint::Satisfies {
                    kind: kind(0),
                    predicate: BuiltinKindPredicate::Equatable,
                },
                KindConstraint::DimensionCompatible(dim(0), dim(2)),
                KindConstraint::DimensionCompatible(dim(1), dim(3)),
            ],
        )?,
    ])
}

pub fn comparison_promoted() -> Result<Vec<KindScheme>, SemanticModelError> {
    comparison_promoted_for_predicate(BuiltinKindPredicate::Number)
}

fn comparison_promoted_for_predicate(
    predicate: BuiltinKindPredicate,
) -> Result<Vec<KindScheme>, SemanticModelError> {
    let mut result = Vec::new();
    for arithmetic in promoted_binary_elementwise()? {
        let output = match arithmetic.outputs().first() {
            Some(KindExpr::Matrix { dimensions, .. }) => matrix(
                BuiltinScalarKind::Bool.kind_expr(),
                dimensions[0].clone(),
                dimensions[1].clone(),
            ),
            _ => BuiltinScalarKind::Bool.kind_expr(),
        };
        let mut constraints = arithmetic.constraints().to_vec();
        constraints.push(KindConstraint::Satisfies {
            kind: kind(0),
            predicate,
        });
        constraints.push(KindConstraint::Satisfies {
            kind: kind(1),
            predicate,
        });
        result.push(KindScheme::new(
            arithmetic.kind_parameters().to_vec().into_boxed_slice(),
            arithmetic
                .dimension_parameters()
                .to_vec()
                .into_boxed_slice(),
            arithmetic.inputs().clone(),
            vec![output].into_boxed_slice(),
            constraints.into_boxed_slice(),
        )?);
    }
    Ok(result)
}

pub fn bool_unary() -> Result<Vec<KindScheme>, SemanticModelError> {
    let bool_kind = BuiltinScalarKind::Bool.kind_expr();
    Ok(vec![
        exact_unary(bool_kind.clone(), bool_kind.clone())?,
        make(
            0,
            2,
            vec![matrix(bool_kind.clone(), dim(0), dim(1))],
            vec![matrix(bool_kind, dim(0), dim(1))],
            Vec::new(),
        )?,
    ])
}

pub fn bool_binary() -> Result<Vec<KindScheme>, SemanticModelError> {
    let bool_kind = BuiltinScalarKind::Bool.kind_expr();
    Ok(vec![
        exact_binary(bool_kind.clone(), bool_kind.clone(), bool_kind.clone())?,
        make(
            0,
            2,
            vec![
                matrix(bool_kind.clone(), dim(0), dim(1)),
                matrix(bool_kind.clone(), dim(0), dim(1)),
            ],
            vec![matrix(bool_kind, dim(0), dim(1))],
            Vec::new(),
        )?,
    ])
}

fn range_scheme(arity: usize) -> Result<KindScheme, SemanticModelError> {
    match arity {
        2 => make(
            3,
            1,
            vec![kind(0), kind(1)],
            vec![matrix(kind(2), DimensionExpr::Constant(1), dim(0))],
            vec![
                KindConstraint::Satisfies {
                    kind: kind(0),
                    predicate: BuiltinKindPredicate::RangeEndpoint,
                },
                KindConstraint::Satisfies {
                    kind: kind(1),
                    predicate: BuiltinKindPredicate::RangeEndpoint,
                },
                KindConstraint::Promotes {
                    left: kind(0),
                    right: kind(1),
                    output: kind(2),
                },
            ],
        ),
        3 => make(
            5,
            1,
            vec![kind(0), kind(1), kind(2)],
            vec![matrix(kind(4), DimensionExpr::Constant(1), dim(0))],
            vec![
                KindConstraint::Satisfies {
                    kind: kind(0),
                    predicate: BuiltinKindPredicate::RangeEndpoint,
                },
                KindConstraint::Satisfies {
                    kind: kind(1),
                    predicate: BuiltinKindPredicate::RangeEndpoint,
                },
                KindConstraint::Satisfies {
                    kind: kind(2),
                    predicate: BuiltinKindPredicate::RangeEndpoint,
                },
                KindConstraint::Promotes {
                    left: kind(0),
                    right: kind(1),
                    output: kind(3),
                },
                KindConstraint::Promotes {
                    left: kind(3),
                    right: kind(2),
                    output: kind(4),
                },
            ],
        ),
        _ => unreachable!("range schemes have two or three endpoints"),
    }
}

pub fn range_binary() -> Result<KindScheme, SemanticModelError> {
    range_scheme(2)
}

pub fn range_ternary() -> Result<KindScheme, SemanticModelError> {
    range_scheme(3)
}

pub fn matrix_transpose() -> Result<KindScheme, SemanticModelError> {
    make(
        1,
        2,
        vec![matrix(kind(0), dim(0), dim(1))],
        vec![matrix(kind(0), dim(1), dim(0))],
        Vec::new(),
    )
}

pub fn matrix_product() -> Result<KindScheme, SemanticModelError> {
    make(
        3,
        3,
        vec![
            matrix(kind(0), dim(0), dim(1)),
            matrix(kind(1), dim(1), dim(2)),
        ],
        vec![matrix(kind(2), dim(0), dim(2))],
        vec![KindConstraint::Promotes {
            left: kind(0),
            right: kind(1),
            output: kind(2),
        }],
    )
}

fn dynamic_matrix_product() -> Result<KindScheme, SemanticModelError> {
    make(
        3,
        4,
        vec![
            matrix(kind(0), dim(0), dim(1)),
            matrix(kind(1), dim(2), dim(3)),
        ],
        vec![matrix(kind(2), dim(0), dim(3))],
        vec![
            KindConstraint::Promotes {
                left: kind(0),
                right: kind(1),
                output: kind(2),
            },
            KindConstraint::DimensionCompatible(dim(1), dim(2)),
        ],
    )
}

pub fn matrix_dot() -> Result<KindScheme, SemanticModelError> {
    make(
        3,
        2,
        vec![
            matrix(kind(0), dim(0), dim(1)),
            matrix(kind(1), dim(0), dim(1)),
        ],
        vec![kind(2)],
        vec![KindConstraint::Promotes {
            left: kind(0),
            right: kind(1),
            output: kind(2),
        }],
    )
}

fn dynamic_matrix_dot() -> Result<KindScheme, SemanticModelError> {
    make(
        3,
        4,
        vec![
            matrix(kind(0), dim(0), dim(1)),
            matrix(kind(1), dim(2), dim(3)),
        ],
        vec![kind(2)],
        vec![KindConstraint::Promotes {
            left: kind(0),
            right: kind(1),
            output: kind(2),
        }],
    )
}

pub fn matrix_solve() -> Result<KindScheme, SemanticModelError> {
    make(
        1,
        2,
        vec![
            matrix(kind(0), dim(0), dim(0)),
            matrix(kind(0), dim(0), dim(1)),
        ],
        vec![matrix(kind(0), dim(0), dim(1))],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::FloatingPoint,
        }],
    )
}

fn dynamic_matrix_solve() -> Result<KindScheme, SemanticModelError> {
    make(
        1,
        6,
        vec![
            matrix(kind(0), dim(0), dim(1)),
            matrix(kind(0), dim(2), dim(3)),
        ],
        vec![matrix(kind(0), dim(4), dim(5))],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::FloatingPoint,
        }],
    )
}

fn instantiate_concatenation_scheme(
    inputs: &[ResolvedType],
    horizontal: bool,
) -> Result<KindScheme, SemanticModelError> {
    if inputs.is_empty() {
        return Err(SemanticModelError::InvalidVariadicKindScheme);
    }
    let mut expected_inputs = Vec::with_capacity(inputs.len());
    let mut common_extents = Vec::with_capacity(inputs.len());
    let mut varying_extents = Vec::with_capacity(inputs.len());
    let mut next_dimension = 0_u32;
    for input in inputs {
        if let KindExpr::Matrix { dimensions, .. } = input.kind() {
            if dimensions.len() != 2 {
                return Err(SemanticModelError::InvalidVariadicKindScheme);
            }
            let rows = dim(next_dimension);
            let columns = dim(next_dimension.checked_add(1).ok_or(
                SemanticModelError::IdentityExhausted {
                    identity: crate::SemanticIdentityKind::DimensionParameterId,
                },
            )?);
            next_dimension =
                next_dimension
                    .checked_add(2)
                    .ok_or(SemanticModelError::IdentityExhausted {
                        identity: crate::SemanticIdentityKind::DimensionParameterId,
                    })?;
            expected_inputs.push(matrix(kind(0), rows.clone(), columns.clone()));
            common_extents.push(if horizontal {
                rows.clone()
            } else {
                columns.clone()
            });
            varying_extents.push(if horizontal { columns } else { rows });
        } else {
            expected_inputs.push(kind(0));
            common_extents.push(DimensionExpr::Constant(1));
            varying_extents.push(DimensionExpr::Constant(1));
        }
    }
    let common = common_extents[0].clone();
    let constraints = common_extents
        .iter()
        .skip(1)
        .map(|extent| KindConstraint::DimensionCompatible(common.clone(), extent.clone()))
        .collect::<Vec<_>>();
    let varying = DimensionExpr::Add(varying_extents.into_boxed_slice());
    let output = if horizontal {
        matrix(kind(0), common, varying)
    } else {
        matrix(kind(0), varying, common)
    };
    make(
        1,
        next_dimension,
        expected_inputs,
        vec![output],
        constraints,
    )
}

pub fn set_membership() -> Result<KindScheme, SemanticModelError> {
    make(
        1,
        1,
        vec![kind(0), set(kind(0), dim(0))],
        vec![BuiltinScalarKind::Bool.kind_expr()],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Keyable,
        }],
    )
}

pub fn set_insert() -> Result<KindScheme, SemanticModelError> {
    make_with_dimensions(
        1,
        bounded_dimensions(
            1,
            Some(DimensionExpr::Add(
                vec![dim(0), DimensionExpr::Constant(1)].into_boxed_slice(),
            )),
        ),
        vec![set(kind(0), dim(0)), kind(0)],
        vec![set(kind(0), dim(1))],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Keyable,
        }],
    )
}

pub fn set_remove() -> Result<KindScheme, SemanticModelError> {
    make_with_dimensions(
        1,
        bounded_dimensions(1, Some(dim(0))),
        vec![set(kind(0), dim(0)), kind(0)],
        vec![set(kind(0), dim(1))],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Keyable,
        }],
    )
}

fn set_binary(output_upper_bound: DimensionExpr) -> Result<KindScheme, SemanticModelError> {
    make_with_dimensions(
        1,
        bounded_dimensions(2, Some(output_upper_bound)),
        vec![set(kind(0), dim(0)), set(kind(0), dim(1))],
        vec![set(kind(0), dim(2))],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Keyable,
        }],
    )
}

pub fn set_union() -> Result<KindScheme, SemanticModelError> {
    set_binary(DimensionExpr::Add(vec![dim(0), dim(1)].into_boxed_slice()))
}

pub fn set_intersection() -> Result<KindScheme, SemanticModelError> {
    set_binary(DimensionExpr::Min(vec![dim(0), dim(1)].into_boxed_slice()))
}

pub fn set_difference() -> Result<KindScheme, SemanticModelError> {
    set_binary(dim(0))
}

pub fn set_symmetric_difference() -> Result<KindScheme, SemanticModelError> {
    set_union()
}

pub fn set_cartesian_product() -> Result<KindScheme, SemanticModelError> {
    make_with_dimensions(
        2,
        bounded_dimensions(
            2,
            Some(DimensionExpr::Multiply(
                vec![dim(0), dim(1)].into_boxed_slice(),
            )),
        ),
        vec![set(kind(0), dim(0)), set(kind(1), dim(1))],
        vec![set(
            KindExpr::Tuple(vec![kind(0), kind(1)].into_boxed_slice()),
            dim(2),
        )],
        Vec::new(),
    )
}

pub fn set_powerset() -> Result<KindScheme, SemanticModelError> {
    let mut extents = dimensions(1).into_vec();
    extents.push(DimensionParameterDeclaration {
        id: DimensionParameterId::new(1),
        origin: DimensionParameterOrigin::Inferred,
        lifetime: DimensionLifetime::Turn,
        lower_bound: DimensionExpr::Constant(0),
        upper_bound: Some(dim(0)),
    });
    extents.push(DimensionParameterDeclaration {
        id: DimensionParameterId::new(2),
        origin: DimensionParameterOrigin::Inferred,
        lifetime: DimensionLifetime::Turn,
        lower_bound: DimensionExpr::Constant(0),
        upper_bound: None,
    });
    make_with_dimensions(
        1,
        extents.into_boxed_slice(),
        vec![set(kind(0), dim(0))],
        vec![set(set(kind(0), dim(1)), dim(2))],
        Vec::new(),
    )
}

pub fn set_relation() -> Result<KindScheme, SemanticModelError> {
    make(
        1,
        2,
        vec![set(kind(0), dim(0)), set(kind(0), dim(1))],
        vec![BuiltinScalarKind::Bool.kind_expr()],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Keyable,
        }],
    )
}

pub fn reduction_scalar() -> Result<KindScheme, SemanticModelError> {
    make(
        1,
        2,
        vec![matrix(kind(0), dim(0), dim(1))],
        vec![kind(0)],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Number,
        }],
    )
}

fn n_choose_k_schemes() -> Result<Vec<KindScheme>, SemanticModelError> {
    let promotion = || KindConstraint::Promotes {
        left: kind(0),
        right: kind(1),
        output: kind(2),
    };
    let numeric = |index| KindConstraint::Satisfies {
        kind: kind(index),
        predicate: BuiltinKindPredicate::Number,
    };
    Ok(vec![
        make(
            3,
            0,
            vec![kind(0), kind(1)],
            vec![kind(2)],
            vec![numeric(0), numeric(1), promotion()],
        )?,
        make(
            3,
            4,
            vec![matrix(kind(0), dim(0), dim(1)), kind(1)],
            vec![matrix(kind(2), dim(2), dim(3))],
            vec![numeric(0), numeric(1), promotion()],
        )?,
    ])
}

fn set_comprehension_schemes() -> Result<Vec<KindScheme>, SemanticModelError> {
    let empty = KindExpr::Tuple(Box::new([]));
    Ok(vec![
        make(
            0,
            1,
            Vec::new(),
            vec![set(empty.clone(), dim(0))],
            vec![KindConstraint::Satisfies {
                kind: empty,
                predicate: BuiltinKindPredicate::Keyable,
            }],
        )?,
        variadic(
            1,
            1,
            kind(0),
            1,
            set(kind(0), dim(0)),
            vec![KindConstraint::Satisfies {
                kind: kind(0),
                predicate: BuiltinKindPredicate::Keyable,
            }],
        )?,
    ])
}

pub fn reduction_axis(column: bool) -> Result<KindScheme, SemanticModelError> {
    let output = if column {
        matrix(kind(0), dim(0), DimensionExpr::Constant(1))
    } else {
        matrix(kind(0), DimensionExpr::Constant(1), dim(1))
    };
    make(
        1,
        2,
        vec![matrix(kind(0), dim(0), dim(1))],
        vec![output],
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Number,
        }],
    )
}

pub fn string_unary() -> Result<KindScheme, SemanticModelError> {
    let string = BuiltinScalarKind::String.kind_expr();
    exact_unary(string.clone(), string)
}

pub fn string_binary() -> Result<KindScheme, SemanticModelError> {
    let string = BuiltinScalarKind::String.kind_expr();
    exact_binary(string.clone(), string.clone(), string)
}

fn absolute_value() -> Result<Vec<KindScheme>, SemanticModelError> {
    let mut schemes = predicate_unary_same(BuiltinKindPredicate::Real)?;
    for (source, target) in [
        (BuiltinScalarKind::C32, BuiltinScalarKind::F32),
        (BuiltinScalarKind::C64, BuiltinScalarKind::F64),
    ] {
        schemes.push(exact_unary(source.kind_expr(), target.kind_expr())?);
        schemes.push(make(
            0,
            2,
            vec![matrix(source.kind_expr(), dim(0), dim(1))],
            vec![matrix(target.kind_expr(), dim(0), dim(1))],
            Vec::new(),
        )?);
    }
    Ok(schemes)
}

fn string_ordering() -> Result<Vec<KindScheme>, SemanticModelError> {
    let string = BuiltinScalarKind::String.kind_expr();
    let boolean = BuiltinScalarKind::Bool.kind_expr();
    Ok(vec![
        exact_binary(string.clone(), string.clone(), boolean.clone())?,
        make(
            0,
            2,
            vec![
                matrix(string.clone(), dim(0), dim(1)),
                matrix(string, dim(0), dim(1)),
            ],
            vec![matrix(boolean, dim(0), dim(1))],
            Vec::new(),
        )?,
    ])
}

pub fn exact_assignment(arity: usize) -> Result<Vec<KindScheme>, SemanticModelError> {
    let constraints = || {
        vec![KindConstraint::Satisfies {
            kind: kind(0),
            predicate: BuiltinKindPredicate::Number,
        }]
    };
    let selector_count = arity.saturating_sub(2) as u32;
    let selectors = || (0..selector_count).map(|id| kind(id + 1));
    let mut scalar_inputs = vec![kind(0), kind(0)];
    scalar_inputs.extend(selectors());
    let sink = matrix(kind(0), dim(0), dim(1));
    let mut broadcast_inputs = vec![sink.clone(), kind(0)];
    broadcast_inputs.extend(selectors());
    let mut matrix_inputs = vec![sink.clone(), matrix(kind(0), dim(2), dim(3))];
    matrix_inputs.extend(selectors());
    Ok(vec![
        make(
            1 + selector_count,
            0,
            scalar_inputs,
            vec![kind(0)],
            constraints(),
        )?,
        make(
            1 + selector_count,
            2,
            broadcast_inputs,
            vec![sink.clone()],
            constraints(),
        )?,
        make(
            1 + selector_count,
            4,
            matrix_inputs,
            vec![sink],
            constraints(),
        )?,
    ])
}

fn table_join(mode: TableJoinMode) -> Result<KindScheme, SemanticModelError> {
    make(
        3,
        1,
        vec![kind(0), kind(1)],
        vec![kind(2)],
        vec![KindConstraint::TableJoin {
            left: kind(0),
            right: kind(1),
            output: kind(2),
            rows: dim(0),
            mode,
        }],
    )
}

pub fn numeric_binary_for_predicate(
    predicate: BuiltinKindPredicate,
) -> Result<Vec<KindScheme>, SemanticModelError> {
    let mut schemes = promoted_binary_elementwise()?;
    schemes = schemes
        .into_iter()
        .map(|scheme| {
            let mut constraints = scheme.constraints().to_vec();
            constraints.push(KindConstraint::Satisfies {
                kind: kind(0),
                predicate,
            });
            constraints.push(KindConstraint::Satisfies {
                kind: kind(1),
                predicate,
            });
            KindScheme::new(
                scheme.kind_parameters().to_vec().into_boxed_slice(),
                scheme.dimension_parameters().to_vec().into_boxed_slice(),
                scheme.inputs().clone(),
                scheme.outputs().to_vec().into_boxed_slice(),
                constraints.into_boxed_slice(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(schemes)
}

pub fn maintained_source_scheme_template(name: &str) -> Option<SourceSchemeTemplate> {
    match name {
        "matrix/horzcat" => Some(SourceSchemeTemplate::HorizontalConcatenation),
        "matrix/vertcat" => Some(SourceSchemeTemplate::VerticalConcatenation),
        "set/define" => Some(SourceSchemeTemplate::SetDefinition),
        _ => None,
    }
}

pub fn instantiate_source_scheme_template(
    template: SourceSchemeTemplate,
    inputs: &[ResolvedType],
) -> Result<Vec<KindScheme>, SemanticModelError> {
    let scheme = match template {
        SourceSchemeTemplate::HorizontalConcatenation => {
            instantiate_concatenation_scheme(inputs, true)?
        }
        SourceSchemeTemplate::VerticalConcatenation => {
            instantiate_concatenation_scheme(inputs, false)?
        }
        SourceSchemeTemplate::SetDefinition => {
            let cardinality =
                u64::try_from(inputs.len()).map_err(|_| SemanticModelError::DimensionOverflowV1)?;
            variadic(
                1,
                0,
                kind(0),
                1,
                set(kind(0), DimensionExpr::Constant(cardinality)),
                vec![KindConstraint::Satisfies {
                    kind: kind(0),
                    predicate: BuiltinKindPredicate::Keyable,
                }],
            )?
        }
    };
    Ok(vec![scheme])
}

/// Returns the explicit storage-blind schemes for one maintained source name.
/// This registry is catalog-construction metadata; execution never inspects
/// operation names to infer a result.
pub fn maintained_source_schemes(
    name: &str,
) -> Result<Option<Vec<KindScheme>>, SemanticModelError> {
    let schemes = match name {
        name if name.contains("-assign") => {
            let arity = if name.ends_with("/range-all") || name.ends_with("/range") {
                3
            } else {
                2
            };
            exact_assignment(arity)?
        }
        "math/add" | "math/sub" | "math/mul" | "math/div" | "math/pow" => {
            promoted_binary_elementwise()?
        }
        "math/mod" => {
            let mut values = numeric_binary_for_predicate(BuiltinKindPredicate::Integer)?;
            values.extend(numeric_binary_for_predicate(
                BuiltinKindPredicate::FloatingPoint,
            )?);
            values
        }
        "math/neg" => predicate_unary_same(BuiltinKindPredicate::Negatable)?,
        "math/abs" => absolute_value()?,
        "math/atan2" | "math/copysign" | "math/fdim" | "math/fmod" | "math/nextafter"
        | "math/remainder" | "math/bessel/jn" | "math/bessel/yn" => {
            numeric_binary_for_predicate(BuiltinKindPredicate::FloatingPoint)?
        }
        name if name.starts_with("math/") => {
            predicate_unary_same(BuiltinKindPredicate::FloatingPoint)?
        }
        "compare/seq" | "compare/sneq" => strict_comparison_exact()?,
        "compare/eq" | "compare/neq" => {
            let mut values = comparison_exact()?;
            values.extend(comparison_promoted()?);
            values
        }
        "compare/gt" | "compare/gte" | "compare/lt" | "compare/lte" => {
            let mut values = comparison_promoted_for_predicate(BuiltinKindPredicate::Ordered)?;
            values.extend(string_ordering()?);
            values
        }
        "compare/max" | "compare/min" => {
            numeric_binary_for_predicate(BuiltinKindPredicate::Ordered)?
        }
        "logic/not" => bool_unary()?,
        "logic/and" | "logic/or" | "logic/xor" => bool_binary()?,
        "range/exclusive" | "range/inclusive" => vec![range_binary()?],
        "range/exclusive-increment" | "range/inclusive-increment" => vec![range_ternary()?],
        "matrix/transpose" => vec![matrix_transpose()?],
        "matrix/matmul" => vec![matrix_product()?, dynamic_matrix_product()?],
        "matrix/dot" => vec![matrix_dot()?, dynamic_matrix_dot()?],
        "matrix/solve" => vec![matrix_solve()?, dynamic_matrix_solve()?],
        "matrix/comprehension" => vec![variadic(
            1,
            2,
            kind(0),
            0,
            matrix(kind(0), dim(0), dim(1)),
            Vec::new(),
        )?],
        "set/comprehension" => set_comprehension_schemes()?,
        "table/join" => vec![table_join(TableJoinMode::Inner)?],
        "table/left-outer-join" => vec![table_join(TableJoinMode::LeftOuter)?],
        "table/right-outer-join" => vec![table_join(TableJoinMode::RightOuter)?],
        "table/full-outer-join" => vec![table_join(TableJoinMode::FullOuter)?],
        "table/left-semi-join" => vec![table_join(TableJoinMode::LeftSemi)?],
        "table/left-anti-join" => vec![table_join(TableJoinMode::LeftAnti)?],
        "set/element-of" | "set/not-element-of" => vec![set_membership()?],
        "set/insert" => vec![set_insert()?],
        "set/remove" => vec![set_remove()?],
        "set/union" => vec![set_union()?],
        "set/intersection" => vec![set_intersection()?],
        "set/difference" => vec![set_difference()?],
        "set/symmetric-difference" => vec![set_symmetric_difference()?],
        "set/cartesian-product" => vec![set_cartesian_product()?],
        "set/powerset" => vec![set_powerset()?],
        "set/equals"
        | "set/not_equals"
        | "set/subset"
        | "set/proper_subset"
        | "set/superset"
        | "set/proper-superset"
        | "set/disjoint" => vec![set_relation()?],
        "set/size" => vec![make(
            1,
            1,
            vec![set(kind(0), dim(0))],
            vec![BuiltinScalarKind::U64.kind_expr()],
            Vec::new(),
        )?],
        "string/concat" => vec![string_binary()?],
        "stats/sum/column" => vec![reduction_axis(true)?],
        "stats/sum/row" => vec![reduction_axis(false)?],
        "combinatorics/n-choose-k" => n_choose_k_schemes()?,
        _ => return Ok(None),
    };
    Ok(Some(schemes))
}
