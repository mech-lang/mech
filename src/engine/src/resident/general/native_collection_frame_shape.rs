//! Immutable shape checks for flat private frames. No previous-visit extents.
//! Unsupported relations retain the ordinary composite constructor's path.
use super::InvokeError;
use mech_core::{
    DimensionExpr, DimensionLifetime, SchemaBody, SchemaId, SchemaTable, ShapeInstance,
};

#[derive(Debug)]
pub(super) struct FrameShapeRecipe {
    output: SchemaId,
    inputs: Box<[SchemaId]>,
    parameters: usize,
    axes: Box<[Axis]>,
}
#[derive(Debug)]
struct Axis {
    input: usize,
    expected: DimensionExpr,
    actual: DimensionExpr,
}

impl FrameShapeRecipe {
    pub(super) fn derive(
        output: SchemaId,
        inputs: &[SchemaId],
        schemas: &SchemaTable,
    ) -> Option<Self> {
        let definition = schemas.get(output)?;
        // This is this consumer's eligibility policy, not a new type relation.
        // Bounded/persistent/computed output axes use the general constructor.
        if !definition.dimension_parameters().iter().all(|p| {
            p.lifetime() == DimensionLifetime::Turn
                && p.lower_bound() == &DimensionExpr::Constant(0)
                && p.upper_bound().is_none()
        }) {
            return None;
        }
        let fields = match definition.body() {
            SchemaBody::Tuple(fields) => fields.iter().collect::<Vec<_>>(),
            SchemaBody::Record(fields) => fields.iter().map(|field| &field.schema).collect(),
            _ => return None,
        };
        if fields.len() != inputs.len() {
            return None;
        }
        let mut axes = Vec::new();
        let mut bound = vec![false; definition.dimension_parameters().len()];
        for (input, (expected, actual)) in fields.iter().zip(inputs).enumerate() {
            let actual = schemas.get(*actual)?.body();
            match (expected, actual) {
                (
                    SchemaBody::Matrix {
                        element: expected,
                        dimensions: ed,
                    },
                    SchemaBody::Matrix {
                        element: actual,
                        dimensions: ad,
                    },
                ) if expected == actual && ed.len() == ad.len() => {
                    // Nested elements must be closed; no parameter relationship
                    // inside a record/table element is silently generalized.
                    if !(mech_core::SchemaDraft {
                        body: expected.as_ref().clone(),
                        dimension_parameters: Box::new([]),
                    })
                    .finalize()
                    .is_ok()
                    {
                        return None;
                    }
                    for (expected, actual) in ed.iter().zip(ad) {
                        match expected {
                            DimensionExpr::Constant(_) => {}
                            DimensionExpr::Parameter(id) => {
                                *bound.get_mut(id.get() as usize)? = true
                            }
                            _ => return None,
                        }
                        axes.push(Axis {
                            input,
                            expected: expected.clone(),
                            actual: actual.clone(),
                        });
                    }
                }
                _ if *expected == actual => {}
                _ => return None,
            }
        }
        if bound.iter().any(|value| !value) {
            return None;
        }
        Some(Self {
            output,
            inputs: inputs.into(),
            parameters: bound.len(),
            axes: axes.into(),
        })
    }

    /// Run only after the caller admits its existing normalization envelope.
    /// Every input's current shape is checked in its own parameter arena.
    /// The single output-parameter allocation replaces child descriptions and
    /// repeated closed-schema construction/binding, not any live shape check.
    pub(super) fn validate<'a>(
        &self,
        schemas: &SchemaTable,
        current: impl Fn(usize) -> Option<&'a ShapeInstance>,
    ) -> Result<(), InvokeError> {
        for (index, schema) in self.inputs.iter().enumerate() {
            let schema = schemas.get(*schema).ok_or(InvokeError::InvalidInput)?;
            let parameters = current(index).map_or(&[][..], ShapeInstance::parameter_values);
            schema
                .instantiate_shape(parameters.to_vec().into_boxed_slice())
                .map_err(|_| InvokeError::InvalidInput)?;
        }
        let mut values = vec![0; self.parameters];
        for axis in &self.axes {
            let shape = current(axis.input).ok_or(InvokeError::InvalidInput)?;
            let actual = shape
                .resolve_dimension(&axis.actual)
                .map_err(|_| InvokeError::InvalidInput)?;
            if let DimensionExpr::Parameter(id) = axis.expected {
                values[id.get() as usize] = actual;
            }
        }
        let shape = schemas
            .get(self.output)
            .ok_or(InvokeError::InvalidInput)?
            .instantiate_shape(values.into_boxed_slice())
            .map_err(|_| InvokeError::InvalidInput)?;
        // Recheck all axes, including constants and repeated parameters; a later
        // child cannot replace an earlier child's current extent silently.
        for axis in &self.axes {
            let actual = current(axis.input)
                .ok_or(InvokeError::InvalidInput)?
                .resolve_dimension(&axis.actual)
                .map_err(|_| InvokeError::InvalidInput)?;
            if shape
                .resolve_dimension(&axis.expected)
                .map_err(|_| InvokeError::InvalidInput)?
                != actual
            {
                return Err(InvokeError::InvalidInput);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        DimensionParameterDeclaration, DimensionParameterId as Id, DimensionParameterOrigin,
        IntegerWidth, SchemaDraft, SchemaTableBuilder,
    };

    fn parameters(count: u32) -> Box<[DimensionParameterDeclaration]> {
        (0..count)
            .map(|n| DimensionParameterDeclaration {
                id: Id::new(n),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            })
            .collect()
    }
    fn matrix(dimensions: Vec<DimensionExpr>) -> SchemaBody {
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: dimensions.into(),
        }
    }
    fn output() -> SchemaDraft {
        let buffer = matrix(vec![
            DimensionExpr::Constant(1),
            DimensionExpr::Parameter(Id::new(0)),
        ]);
        SchemaDraft {
            body: SchemaBody::Tuple(vec![buffer.clone(), buffer].into()),
            dimension_parameters: parameters(1),
        }
    }

    #[test]
    fn frame_recipe_agrees_with_constructor_for_fresh_fixed_and_repeated_axes() {
        let mut builder = SchemaTableBuilder::new();
        let input = builder
            .insert(
                SchemaDraft {
                    body: matrix(vec![
                        DimensionExpr::Parameter(Id::new(0)),
                        DimensionExpr::Parameter(Id::new(1)),
                    ]),
                    dimension_parameters: parameters(2),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let result = builder.insert(output().finalize().unwrap()).unwrap();
        let built = builder.finish().unwrap();
        let input = built.resolve(input).unwrap();
        let result = built.resolve(result).unwrap();
        let recipe = FrameShapeRecipe::derive(result, &[input, input], &built.table).unwrap();
        for (a, b, accepted) in [
            (vec![1, 0], vec![1, 0], true),
            (vec![1, 5], vec![1, 5], true),
            (vec![1, 3], vec![1, 3], true),
            (vec![1, 3], vec![1, 5], false),
            (vec![2, 5], vec![1, 5], false),
            (vec![1, 5], vec![2, 5], false),
            (vec![1, 0], vec![1, 0], true),
        ] {
            let shapes = [
                built
                    .table
                    .get(input)
                    .unwrap()
                    .instantiate_shape(a.into())
                    .unwrap(),
                built
                    .table
                    .get(input)
                    .unwrap()
                    .instantiate_shape(b.into())
                    .unwrap(),
            ];
            let children = [(input, shapes[0].clone()), (input, shapes[1].clone())];
            let ordinary = mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                result,
                &children,
                &built.table,
            )
            .and_then(|shape| {
                mech_core::snapshot::CompositeSnapshotConstructor::bind(
                    result,
                    shape,
                    &children,
                    built.table.clone().into(),
                )
            });
            assert_eq!(ordinary.is_ok(), accepted, "{children:?}");
            assert_eq!(
                recipe.validate(&built.table, |i| Some(&shapes[i])).is_ok(),
                ordinary.is_ok()
            );
        }
    }

    #[test]
    fn frame_recipe_declines_bounded_persistent_and_computed_output_relations() {
        for policy in 0..3 {
            let mut draft = output();
            match policy {
                0 => draft.dimension_parameters[0].upper_bound = Some(DimensionExpr::Constant(5)),
                1 => draft.dimension_parameters[0].lifetime = DimensionLifetime::Activation,
                _ => {
                    let SchemaBody::Tuple(fields) = &mut draft.body else {
                        unreachable!()
                    };
                    for field in fields {
                        let SchemaBody::Matrix { dimensions, .. } = field else {
                            unreachable!()
                        };
                        dimensions[1] = DimensionExpr::Add(
                            vec![
                                DimensionExpr::Parameter(Id::new(0)),
                                DimensionExpr::Constant(1),
                            ]
                            .into(),
                        );
                    }
                }
            }
            let mut builder = SchemaTableBuilder::new();
            let result = builder.insert(draft.finalize().unwrap()).unwrap();
            let built = builder.finish().unwrap();
            let result = built.resolve(result).unwrap();
            assert!(FrameShapeRecipe::derive(result, &[], &built.table).is_none());
        }
    }
}
