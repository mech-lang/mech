//! Identity annotations can independently number the same free buffer axes.
//! This does not admit reshapes, coupled axes, bounds, or element conversion.
use mech_core::{DimensionExpr, DimensionLifetime, SchemaBody, SchemaDraft};

/// A fresh record constructor binds its result dimensions from its fields.
/// Unlike an existing snapshot, it has no positional shape to preserve yet.
/// Give independent free axes occurrence order, keeping every field's geometry
/// and shared-axis relationships. Bounded or persistent axes retain their order.
pub(super) fn fresh_record(mut draft: SchemaDraft) -> SchemaDraft {
    if !matches!(draft.body, SchemaBody::Record(_)) {
        return draft;
    }
    let original = draft.clone();
    for parameter in draft.dimension_parameters.iter_mut() {
        parameter.origin = mech_core::DimensionParameterOrigin::Inferred;
    }
    if equivalent(&original, &draft) {
        draft
    } else {
        original
    }
}

pub(super) fn equivalent(actual: &SchemaDraft, expected: &SchemaDraft) -> bool {
    let (Ok(actual), Ok(expected)) = (actual.clone().finalize(), expected.clone().finalize())
    else {
        return false;
    };
    if actual == expected {
        return true;
    }
    let count = actual.dimension_parameters().len();
    if count == 0 || count != expected.dimension_parameters().len() {
        return false;
    }
    // Only unconstrained turn-local axes can be renamed here. All other
    // annotation checks continue through the maintained conversion planner.
    if actual
        .dimension_parameters()
        .iter()
        .chain(expected.dimension_parameters())
        .any(|p| {
            p.lifetime() != DimensionLifetime::Turn
                || p.lower_bound() != &DimensionExpr::Constant(0)
                || p.upper_bound().is_some()
        })
    {
        return false;
    }
    let mut forward = vec![None; count];
    let mut reverse = vec![None; count];
    same_body(actual.body(), expected.body(), &mut forward, &mut reverse)
        && forward.iter().all(Option::is_some)
        && reverse.iter().all(Option::is_some)
}

fn same_body(
    actual: &SchemaBody,
    expected: &SchemaBody,
    forward: &mut [Option<usize>],
    reverse: &mut [Option<usize>],
) -> bool {
    match (actual, expected) {
        // Closed matrix fields (including fixed arrays of scalar records)
        // carry no free axes that participate in this renaming.
        (
            SchemaBody::Matrix {
                element: a,
                dimensions: ad,
            },
            SchemaBody::Matrix {
                element: b,
                dimensions: bd,
            },
        ) if a == b
            && ad == bd
            && ad.iter().all(|d| matches!(d, DimensionExpr::Constant(_)))
            && (SchemaDraft {
                body: a.as_ref().clone(),
                dimension_parameters: Box::new([]),
            })
            .finalize()
            .is_ok() =>
        {
            true
        }
        (
            SchemaBody::Matrix {
                element: a,
                dimensions: ad,
            },
            SchemaBody::Matrix {
                element: b,
                dimensions: bd,
            },
        ) if a == b && matches!(a.as_ref(), SchemaBody::UnsignedInteger(_)) => {
            ad.len() == bd.len()
                && ad.iter().zip(bd).all(|(a, b)| match (a, b) {
                    (DimensionExpr::Constant(a), DimensionExpr::Constant(b)) => a == b,
                    (DimensionExpr::Parameter(a), DimensionExpr::Parameter(b)) => {
                        let (a, b) = (a.get() as usize, b.get() as usize);
                        if forward[a].is_some_and(|value| value != b)
                            || reverse[b].is_some_and(|value| value != a)
                        {
                            return false;
                        }
                        forward[a] = Some(b);
                        reverse[b] = Some(a);
                        true
                    }
                    _ => false,
                })
        }
        (SchemaBody::Tuple(a), SchemaBody::Tuple(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(a, b)| same_body(a, b, forward, reverse))
        }
        (SchemaBody::Record(a), SchemaBody::Record(b)) => {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.name == b.name && same_body(&a.schema, &b.schema, forward, reverse)
                })
        }
        (SchemaBody::Bool | SchemaBody::UnsignedInteger(_), _) => actual == expected,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin,
        IntegerWidth, SchemaField,
    };

    fn buffers(lens: u32, words: u32) -> SchemaDraft {
        let buffer = |width, parameter| SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(width)),
            dimensions: Box::new([
                DimensionExpr::Constant(1),
                DimensionExpr::Parameter(DimensionParameterId::new(parameter)),
            ]),
        };
        SchemaDraft {
            dimension_parameters: (0..2)
                .map(|id| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(id),
                    // Structural projections reify their existing parameter order.
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                })
                .collect(),
            body: SchemaBody::Record(Box::new([
                SchemaField {
                    name: "lens".into(),
                    schema: buffer(IntegerWidth::W8, lens),
                },
                SchemaField {
                    name: "codewords".into(),
                    schema: buffer(IntegerWidth::W32, words),
                },
            ])),
        }
    }

    #[test]
    fn fresh_record_uses_field_order_without_changing_axis_relationships() {
        let actual = buffers(1, 0);
        let expected = buffers(0, 1).finalize().unwrap();
        assert_eq!(fresh_record(actual).finalize().unwrap(), expected);
        let shared = buffers(0, 0);
        assert_eq!(
            fresh_record(shared.clone()).finalize().unwrap(),
            shared.finalize().unwrap()
        );
    }

    #[test]
    fn fresh_record_retains_bounded_and_persistent_axes() {
        for case in 0..3 {
            let mut actual = buffers(1, 0);
            match case {
                0 => actual.dimension_parameters[0].lower_bound = DimensionExpr::Constant(1),
                1 => actual.dimension_parameters[0].upper_bound = Some(DimensionExpr::Constant(32)),
                _ => actual.dimension_parameters[0].lifetime = DimensionLifetime::Activation,
            }
            assert_eq!(fresh_record(actual.clone()), actual);
        }
    }

    #[test]
    fn fixed_record_array_fields_must_really_be_closed() {
        let add_field = |mut draft: SchemaDraft, element| {
            let SchemaBody::Record(fields) = draft.body else {
                panic!()
            };
            let mut fields = fields.into_vec();
            fields.push(SchemaField {
                name: "sequences".into(),
                schema: SchemaBody::Matrix {
                    element: Box::new(element),
                    dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(1)]),
                },
            });
            draft.body = SchemaBody::Record(fields.into_boxed_slice());
            draft
        };
        let element = SchemaBody::Record(Box::new([SchemaField {
            name: "length".into(),
            schema: SchemaBody::UnsignedInteger(IntegerWidth::W32),
        }]));
        let actual = add_field(buffers(1, 0), element.clone());
        let expected = add_field(buffers(0, 1), element).finalize().unwrap();
        assert_eq!(fresh_record(actual).finalize().unwrap(), expected);
        let open = SchemaBody::Record(Box::new([SchemaField {
            name: "buffer".into(),
            schema: SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W32)),
                dimensions: Box::new([
                    DimensionExpr::Constant(1),
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                ]),
            },
        }]));
        let actual = add_field(buffers(1, 0), open);
        assert_eq!(fresh_record(actual.clone()), actual);
    }

    #[test]
    fn independent_buffer_axes_are_identity_under_a_bijection() {
        let actual = buffers(1, 0);
        let expected = buffers(0, 1);
        assert_ne!(
            actual.clone().finalize().unwrap(),
            expected.clone().finalize().unwrap()
        );
        assert!(equivalent(&actual, &expected));
        assert!(equivalent(&expected, &actual));
    }

    #[test]
    fn independent_axes_cannot_become_coupled_or_change_constants_or_elements() {
        let actual = buffers(1, 0);
        assert!(!equivalent(&actual, &buffers(0, 0)));
        assert!(!equivalent(&buffers(0, 0), &actual));
        let mut expected = buffers(0, 1);
        let SchemaBody::Record(fields) = &mut expected.body else {
            panic!()
        };
        let SchemaBody::Matrix { dimensions, .. } = &mut fields[0].schema else {
            panic!()
        };
        dimensions[0] = DimensionExpr::Constant(2);
        assert!(!equivalent(&actual, &expected));
        let SchemaBody::Record(fields) = &mut expected.body else {
            panic!()
        };
        let SchemaBody::Matrix {
            dimensions,
            element,
        } = &mut fields[0].schema
        else {
            panic!()
        };
        dimensions[0] = DimensionExpr::Constant(1);
        *element = Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W16));
        assert!(!equivalent(&actual, &expected));
    }

    #[test]
    fn bounded_and_persistent_axes_keep_the_conversion_path() {
        let actual = buffers(1, 0);
        let mut expected = buffers(0, 1);
        expected.dimension_parameters[0].lower_bound = DimensionExpr::Constant(1);
        assert!(!equivalent(&actual, &expected));
        expected.dimension_parameters[0].lower_bound = DimensionExpr::Constant(0);
        expected.dimension_parameters[0].upper_bound = Some(DimensionExpr::Constant(32));
        assert!(!equivalent(&actual, &expected));
        expected.dimension_parameters[0].upper_bound = None;
        expected.dimension_parameters[0].lifetime = DimensionLifetime::Activation;
        assert!(!equivalent(&actual, &expected));
    }

    #[test]
    fn repeated_axes_must_keep_the_same_relationship() {
        let mut actual = buffers(0, 1);
        let mut expected = buffers(0, 1);
        let SchemaBody::Record(fields) = &actual.body else {
            panic!()
        };
        let mut third = fields[0].clone();
        third.name = "again".into();
        let SchemaBody::Record(fields) = &mut actual.body else {
            panic!()
        };
        *fields = fields.iter().cloned().chain([third.clone()]).collect();
        let SchemaBody::Matrix { dimensions, .. } = &mut third.schema else {
            panic!()
        };
        dimensions[1] = DimensionExpr::Parameter(DimensionParameterId::new(1));
        let SchemaBody::Record(fields) = &mut expected.body else {
            panic!()
        };
        *fields = fields.iter().cloned().chain([third]).collect();
        assert!(!equivalent(&actual, &expected));
        assert!(!equivalent(&expected, &actual));
    }

    #[test]
    fn unused_projection_axes_keep_existing_identity_behavior() {
        let actual = buffers(0, 0);
        let expected = buffers(1, 1);
        assert!(equivalent(&actual, &expected));
    }
}
