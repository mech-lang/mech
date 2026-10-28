//! Function carriers give free axes local, first-use identities on every visit.
use super::*;

pub(super) fn embed(
    draft: &SchemaDraft,
    output: &mut Vec<DimensionParameterDeclaration>,
    anchor: SourceSemanticAnchor,
) -> Result<SchemaBody, SourceSemanticError> {
    let mut local = draft.clone();
    // Projection reifies axes as Explicit, while append creates an Inferred
    // axis. Carrying both otherwise renumbers the unchanged argument ahead of
    // the updated argument on a recursive back-edge. These axes are anonymous
    // within the function carrier; preserve their relationships, but let the
    // canonicalizer order them by use in both the entry and recursive tuple.
    // Bounds and persistent dimensions retain the existing embedding path.
    if local.dimension_parameters.iter().all(|parameter| {
        parameter.lifetime == DimensionLifetime::Turn
            && parameter.lower_bound == DimensionExpr::Constant(0)
            && parameter.upper_bound.is_none()
    }) {
        for parameter in local.dimension_parameters.iter_mut() {
            parameter.origin = DimensionParameterOrigin::Inferred;
        }
    }
    embed_schema_draft(&local, output, anchor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor() -> SourceSemanticAnchor {
        SourceSemanticAnchor {
            document: DocumentId(1),
            revision: Revision(1),
            range: TextRange::empty(mech_syntax::document::TextSize::ZERO),
        }
    }
    fn buffer(origin: DimensionParameterOrigin, repeated: bool) -> SchemaDraft {
        SchemaDraft {
            dimension_parameters: (0..2)
                .map(|id| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(id),
                    origin,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                })
                .collect(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8)),
                dimensions: Box::new([
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(if repeated {
                        0
                    } else {
                        1
                    })),
                ]),
            },
        }
    }
    fn carrier(
        second: DimensionParameterOrigin,
        repeated: bool,
        normalized: bool,
    ) -> mech_core::Schema {
        let mut dimensions = Vec::new();
        let fields = [
            buffer(DimensionParameterOrigin::Inferred, false),
            buffer(second, repeated),
        ]
        .iter()
        .map(|schema| {
            if normalized {
                embed(schema, &mut dimensions, anchor()).unwrap()
            } else {
                embed_schema_draft(schema, &mut dimensions, anchor()).unwrap()
            }
        })
        .collect();
        SchemaDraft {
            dimension_parameters: dimensions.into_boxed_slice(),
            body: SchemaBody::Tuple(fields),
        }
        .finalize()
        .unwrap()
    }
    #[test]
    fn fresh_and_projected_arguments_have_the_same_free_axis_order() {
        use DimensionParameterOrigin::{Explicit, Inferred};
        assert_ne!(
            carrier(Inferred, false, false),
            carrier(Explicit, false, false)
        );
        assert_eq!(
            carrier(Inferred, false, true),
            carrier(Explicit, false, true)
        );
        assert_eq!(carrier(Inferred, true, true), carrier(Explicit, true, true));
        assert_ne!(
            carrier(Explicit, true, true),
            carrier(Explicit, false, true)
        );
    }
    #[test]
    fn constrained_or_persistent_arguments_keep_their_declarations() {
        for kind in 0..3 {
            let mut schema = buffer(DimensionParameterOrigin::Explicit, false);
            match kind {
                0 => schema.dimension_parameters[0].lower_bound = DimensionExpr::Constant(1),
                1 => {
                    schema.dimension_parameters[1].upper_bound = Some(DimensionExpr::Constant(320))
                }
                _ => schema.dimension_parameters[1].lifetime = DimensionLifetime::Activation,
            }
            let mut dimensions = Vec::new();
            assert_eq!(
                embed(&schema, &mut dimensions, anchor()).unwrap(),
                schema.body
            );
            assert_eq!(dimensions.as_slice(), schema.dimension_parameters.as_ref());
        }
    }
}
