//! Identity annotations can independently number the same free buffer axes.
//! This does not admit reshapes, coupled axes, bounds, or element conversion.
use mech_core::{DimensionExpr, DimensionLifetime, SchemaBody, SchemaDraft};

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
