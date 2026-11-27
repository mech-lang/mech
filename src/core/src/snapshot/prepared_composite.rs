//! Immutable schema relations for current flat aggregate construction.
//! Unsupported relations retain the general shape solver and binder.
use super::{CompositeSnapshotConstructor, SnapshotValueError};
use crate::{
    DimensionExpr, DimensionLifetime, SchemaBody, SchemaId, SchemaKey, SchemaTable, ShapeInstance,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct CompositeSnapshotRecipe {
    schema: SchemaId,
    schemas: Arc<SchemaTable>,
    inputs: Box<[(SchemaId, SchemaKey, bool)]>,
    axes: Box<[Axis]>,
    parameters: usize,
}

#[derive(Clone, Debug)]
struct Axis {
    input: usize,
    expected: DimensionExpr,
    actual: DimensionExpr,
}

impl CompositeSnapshotRecipe {
    /// Compile only structural relations. Each later binding validates fresh
    /// shapes; neither current values nor dimensions are preparation facts.
    pub fn derive(
        schema: SchemaId,
        inputs: &[SchemaId],
        schemas: Arc<SchemaTable>,
    ) -> Option<Self> {
        let definition = schemas.get(schema)?;
        if !definition.dimension_parameters().iter().all(|p| {
            p.lifetime() == DimensionLifetime::Turn
                && p.lower_bound() == &DimensionExpr::Constant(0)
                && p.upper_bound().is_none()
        }) {
            return None;
        }
        let fields: Vec<_> = match definition.body() {
            SchemaBody::Tuple(fields) => fields.iter().collect(),
            SchemaBody::Record(fields) => fields.iter().map(|field| &field.schema).collect(),
            _ => return None,
        };
        if fields.len() != inputs.len() {
            return None;
        }
        let closed = |body: &SchemaBody| {
            (crate::SchemaDraft {
                body: body.clone(),
                dimension_parameters: Box::new([]),
            })
            .finalize()
            .is_ok()
        };
        let mut axes = Vec::new();
        let mut bound = vec![false; definition.dimension_parameters().len()];
        let mut children = Vec::with_capacity(inputs.len());
        for (input, (&id, expected)) in inputs.iter().zip(fields).enumerate() {
            let actual = schemas.entry(id)?;
            match (expected, actual.schema().body()) {
                (
                    SchemaBody::Matrix {
                        element: e,
                        dimensions: ed,
                    },
                    SchemaBody::Matrix {
                        element: a,
                        dimensions: ad,
                    },
                ) if e == a && ed.len() == ad.len() && closed(e) => {
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
                _ if expected == actual.schema().body() && closed(expected) => {}
                _ => return None,
            }
            children.push((id, actual.key(), matches!(expected, SchemaBody::Dynamic)));
        }
        if bound.iter().any(|bound| !bound) {
            return None;
        }
        let parameters = bound.len();
        Some(Self {
            schema,
            schemas,
            inputs: children.into(),
            axes: axes.into(),
            parameters,
        })
    }

    /// Bind current canonical child headers without closing and cloning their
    /// schemas. The private recipe proves the unchanged structural relation;
    /// this call still checks every current bound, extent and shared axis.
    /// Callers admit the ordinary binding envelope before invoking this method.
    pub fn bind<'a>(
        &self,
        count: usize,
        current: impl Fn(usize) -> Option<(SchemaId, &'a ShapeInstance)>,
    ) -> Result<CompositeSnapshotConstructor, SnapshotValueError> {
        let output = self
            .schemas
            .entry(self.schema)
            .expect("prepared output remains present");
        let mismatch =
            || SnapshotValueError::SnapshotSchemaDefinitionMismatch { key: output.key() };
        if count != self.inputs.len() {
            return Err(mismatch());
        }
        // Snapshot each caller-supplied header once. Even a stateful Fn cannot
        // substitute a different shape after validation or dimension checks.
        let children = self
            .inputs
            .iter()
            .enumerate()
            .map(|(index, &(id, key, dynamic))| {
                let (actual, shape) = current(index).ok_or_else(mismatch)?;
                if actual != id {
                    return Err(mismatch());
                }
                self.schemas
                    .get(id)
                    .expect("prepared input remains present")
                    .validate_shape(shape)
                    .map_err(|_| mismatch())?;
                Ok((key, shape.clone(), dynamic))
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?
            .into_boxed_slice();
        let mut parameters = vec![0; self.parameters];
        for axis in &self.axes {
            let shape = &children[axis.input].1;
            let actual = shape
                .resolve_dimension(&axis.actual)
                .map_err(|_| mismatch())?;
            if let DimensionExpr::Parameter(id) = axis.expected {
                parameters[id.get() as usize] = actual;
            }
        }
        let shape = output
            .schema()
            .instantiate_shape(parameters.into_boxed_slice())
            .map_err(|_| mismatch())?;
        for axis in &self.axes {
            let child = &children[axis.input].1;
            if child
                .resolve_dimension(&axis.actual)
                .map_err(|_| mismatch())?
                != shape
                    .resolve_dimension(&axis.expected)
                    .map_err(|_| mismatch())?
            {
                return Err(mismatch());
            }
        }
        Ok(CompositeSnapshotConstructor {
            schema: self.schema,
            shape,
            children,
            schemas: Arc::clone(&self.schemas),
        })
    }
}
