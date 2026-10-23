//! Input-only dimension alignment for already-supported aggregate conversions.
//! Output schemas and real cast eligibility remain with the shared solver/plan.
use super::*;

pub(super) fn input_shape(
    actual: &KindExpr,
    expected: &KindExpr,
    parameters: &mut Vec<DimensionParameterDeclaration>,
) -> Option<KindExpr> {
    Some(match (actual, expected) {
        (
            KindExpr::Matrix {
                element: actual,
                dimensions,
            },
            KindExpr::Set {
                element: expected, ..
            },
        ) => {
            // These axes belong to the source Matrix that disappears during
            // conversion. They must not become the output Set cardinality or
            // capture independently quantified axes of a Matrix-valued key.
            let mut source_axes = Vec::with_capacity(dimensions.len());
            for _ in dimensions {
                let id = DimensionParameterId::new(u32::try_from(parameters.len()).ok()?);
                parameters.push(DimensionParameterDeclaration {
                    id,
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                });
                source_axes.push(DimensionExpr::Parameter(id));
            }
            KindExpr::Matrix {
                element: Box::new(input_shape(actual, expected, parameters)?),
                dimensions: source_axes.into_boxed_slice(),
            }
        }
        (
            KindExpr::Matrix {
                element: actual, ..
            },
            KindExpr::Matrix {
                element: expected,
                dimensions,
            },
        ) => KindExpr::Matrix {
            element: Box::new(input_shape(actual, expected, parameters)?),
            dimensions: dimensions.clone(),
        },
        (KindExpr::Option(actual), KindExpr::Option(expected)) => {
            KindExpr::Option(Box::new(input_shape(actual, expected, parameters)?))
        }
        // OptionPresent constructs a target wrapper, not an input dimension.
        (_, KindExpr::Option(expected)) => input_shape(actual, expected, parameters)?,
        // Existing non-conversion shape patterns, including exact aggregate
        // cardinalities, keep their old matching behavior.
        _ => expected.clone(),
    })
}
