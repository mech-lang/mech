//! Allocation/work witness for the existing composite shape normalization path.

use crate::{
    CardinalitySpec, DimensionExpr, Schema, SchemaBody, SchemaId, SchemaKey, ShapeInstance,
};

/// Temporary metadata used while witnessing and binding current child shapes.
/// This excludes canonical child/output payloads, which have separate costs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompositeBindingCost {
    pub temporary_bytes: u64,
    pub metadata_nodes: u64,
    pub compute_work: u64,
}

#[derive(Clone, Copy, Default)]
struct Walk {
    nodes: u64,
    dimensions: u64,
    dimension_nodes: u64,
    work: u64,
}

impl Walk {
    fn add(&mut self, other: Self) -> Option<()> {
        self.nodes = self.nodes.checked_add(other.nodes)?;
        self.dimensions = self.dimensions.checked_add(other.dimensions)?;
        self.dimension_nodes = self.dimension_nodes.checked_add(other.dimension_nodes)?;
        self.work = self.work.checked_add(other.work)?;
        Some(())
    }
    fn extent(&mut self, extent: &CardinalitySpec) -> Option<()> {
        match extent {
            CardinalitySpec::Exact(expression)
            | CardinalitySpec::Dynamic {
                upper_bound: Some(expression),
            } => self.dimension(expression),
            CardinalitySpec::Dynamic { upper_bound: None } => Some(()),
        }
    }
    fn dimension(&mut self, expression: &DimensionExpr) -> Option<()> {
        self.dimensions = self.dimensions.checked_add(1)?;
        let count = dimension_nodes(expression)?;
        self.dimension_nodes = self.dimension_nodes.checked_add(count)?;
        self.work = self.work.checked_add(count)?;
        Some(())
    }
}

fn dimension_nodes(expression: &DimensionExpr) -> Option<u64> {
    match expression {
        DimensionExpr::Add(children)
        | DimensionExpr::Multiply(children)
        | DimensionExpr::Min(children)
        | DimensionExpr::Max(children) => children
            .iter()
            .try_fold(1_u64, |n, child| n.checked_add(dimension_nodes(child)?)),
        _ => Some(1),
    }
}

fn body_walk(body: &SchemaBody) -> Option<Walk> {
    let mut result = Walk {
        nodes: 1,
        work: 1,
        ..Walk::default()
    };
    match body {
        SchemaBody::Option(child) => result.add(body_walk(child)?)?,
        SchemaBody::Tuple(children) => {
            for child in children {
                result.add(body_walk(child)?)?;
            }
        }
        SchemaBody::Record(fields)
        | SchemaBody::Table {
            columns: fields, ..
        } => {
            for field in fields {
                result.work = result.work.checked_add(field.name.len() as u64)?;
                result.add(body_walk(&field.schema)?)?;
            }
            if let SchemaBody::Table { rows, .. } = body {
                result.extent(rows)?;
            }
        }
        SchemaBody::Enum { variants, .. } => {
            for variant in variants {
                result.nodes = result.nodes.checked_add(1)?;
                result.work = result.work.checked_add(variant.name.len() as u64)?;
                if let Some(child) = &variant.payload {
                    result.add(body_walk(child)?)?;
                }
            }
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            result.add(body_walk(element)?)?;
            for dimension in dimensions {
                result.dimension(dimension)?;
            }
        }
        SchemaBody::Set {
            element,
            cardinality,
        } => {
            result.add(body_walk(element)?)?;
            result.extent(cardinality)?;
        }
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => {
            result.add(body_walk(key)?)?;
            result.add(body_walk(value)?)?;
            result.extent(cardinality)?;
        }
        _ => {}
    }
    Some(result)
}

fn schema_work(schema: &Schema) -> Option<u64> {
    schema.dimension_parameters().iter().try_fold(
        body_walk(schema.body())?.work,
        |work, parameter| {
            work.checked_add(dimension_nodes(parameter.lower_bound())?)?
                .checked_add(match parameter.upper_bound() {
                    Some(bound) => dimension_nodes(bound)?,
                    None => 0,
                })
        },
    )
}

fn count_bytes<T>(count: u64) -> Option<u64> {
    count.checked_mul(core::mem::size_of::<T>() as u64)
}

// Result-collect and push-grown Vecs use at most max(4, 2*len) slots. Include
// the final boxed slice as well, so shrinking need not happen in place.
fn grown_bytes<T>(count: u64) -> Option<u64> {
    if count == 0 {
        return Some(0);
    }
    count_bytes::<T>(count.checked_mul(2)?.max(4).checked_add(count)?)
}

fn closure_bytes(body: &SchemaBody) -> Option<u64> {
    // Reuse the canonical schema's actual enum/field/string/dimension layouts.
    // A Result-collected nonempty slice may temporarily use four slots for
    // its first item, plus its boxed result. Five cloned layouts bound that
    // growth for every slice; scalar nodes allocate nothing. Dimension closure
    // removes expression children, never increases this clone bound.
    body.clone_allocation_bound_bytes()?.checked_mul(5)
}

pub(super) fn binding_cost(
    output: &Schema,
    components: &[(&SchemaBody, &Schema)],
) -> Option<CompositeBindingCost> {
    let count = components.len() as u64;
    let output_parameters = output.dimension_parameters().len() as u64;
    let mut child_parameters = 0_u64;
    let mut actual_heap = 0_u64;
    let mut expected_heap = 0_u64;
    let mut actual = Walk::default();
    let mut expected = Walk::default();
    let mut schema_visits = schema_work(output)?;
    for (component, child) in components {
        child_parameters =
            child_parameters.checked_add(child.dimension_parameters().len() as u64)?;
        actual_heap = actual_heap.checked_add(closure_bytes(child.body())?)?;
        actual.add(body_walk(child.body())?)?;
        schema_visits = schema_visits.checked_add(schema_work(child)?)?;
        if !matches!(component, SchemaBody::Dynamic) {
            expected_heap = expected_heap.checked_add(closure_bytes(component)?)?;
            expected.add(body_walk(component)?)?;
        }
    }
    // The aggregate cardinality is a separate solver witness, outside the
    // component bodies. Its expression also participates in fallback solving.
    match output.body() {
        SchemaBody::Table { rows, .. }
        | SchemaBody::Map {
            cardinality: rows, ..
        } => expected.extent(rows)?,
        _ => {}
    }
    // shape_for_children closes each actual once, and bind closes it once
    // again. The solver closes each expected component once; bind closes the
    // full output once. Repeated table/map components occur only in the actual
    // component list, never as output_schema_size * child_count.
    let mut bytes = closure_bytes(output.body())?
        .checked_add(actual_heap.checked_mul(2)?)?
        .checked_add(expected_heap)?;
    let witnesses = expected.dimensions.checked_add(1)?;
    // Current child identities, expected component refs (two calls), closed
    // component pairs, retained binding identities, witness and fixed buffers.
    for addition in [
        grown_bytes::<(SchemaId, ShapeInstance)>(count)?,
        grown_bytes::<&SchemaBody>(count)?.checked_mul(2)?,
        grown_bytes::<(&SchemaBody, SchemaBody)>(count)?,
        grown_bytes::<(SchemaKey, ShapeInstance, bool)>(count)?,
        grown_bytes::<(&DimensionExpr, u64)>(witnesses)?,
        count_bytes::<bool>(output_parameters)?,
        count_bytes::<u64>(
            child_parameters
                .checked_mul(4)?
                .checked_add(output_parameters.checked_mul(3)?)?,
        )?,
    ] {
        bytes = bytes.checked_add(addition)?;
    }
    let output_walk = body_walk(output.body())?;
    // These are actual metadata nodes, not encoded label bytes. All schemas
    // and shape buffers can coexist with the candidate until publication.
    let nodes = output_walk
        .nodes
        .checked_add(output_walk.dimensions)?
        .checked_add(
            actual
                .nodes
                .checked_add(actual.dimensions)?
                .checked_mul(2)?,
        )?
        .checked_add(expected.nodes.checked_add(expected.dimensions)?)?
        .checked_add(count.checked_mul(5)?)?
        .checked_add(witnesses.checked_mul(3)?)?
        .checked_add(
            output_parameters
                .checked_add(child_parameters)?
                .checked_mul(4)?,
        )?;
    // The solver performs at most P+1 witness passes. Reference/injectivity
    // and fallback walks visit at most D^2 expression nodes per pass; each
    // unique witness uses at most 64 bisections plus a final evaluator check.
    // Remaining validation/closure/schema comparison paths make at most eight
    // complete walks, including bounds and field/variant text comparisons.
    let d = expected.dimension_nodes.checked_add(1)?;
    let p = output_parameters;
    let solver_work = d
        .checked_mul(d)?
        .checked_mul(p.checked_add(1)?)?
        .checked_mul(4)?
        .checked_add(d.checked_mul(p)?.checked_mul(65)?)?;
    let work = schema_visits
        .checked_add(expected.work)?
        .checked_mul(8)?
        .checked_add(solver_work)?
        .checked_add(count.checked_mul(4)?)?;
    Some(CompositeBindingCost {
        temporary_bytes: bytes,
        metadata_nodes: nodes,
        compute_work: work,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SchemaDraft, SchemaField};
    #[cfg(feature = "no_std")]
    use alloc::{boxed::Box, vec};

    fn schema(body: SchemaBody) -> Schema {
        SchemaDraft {
            body,
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn wide_tuple_binding_metadata_grows_with_actual_components() {
        let scalar = schema(SchemaBody::Bool);
        let bound = |count| {
            let output = schema(SchemaBody::Tuple(
                vec![SchemaBody::Bool; count].into_boxed_slice(),
            ));
            let components = vec![(&SchemaBody::Bool, &scalar); count];
            binding_cost(&output, &components).unwrap()
        };
        let small = bound(32);
        let large = bound(64);
        assert!(large.temporary_bytes <= small.temporary_bytes * 2);
        assert!(large.metadata_nodes <= small.metadata_nodes * 2);
        assert!(large.compute_work <= small.compute_work * 2);
        assert!(large.temporary_bytes < crate::RESIDENT_MAX_BYTES);
        assert!(large.metadata_nodes < crate::RESIDENT_MAX_RETAINED_NODES);
    }

    #[test]
    fn dynamic_components_charge_their_actual_closed_dimension_slots() {
        let output = schema(SchemaBody::Tuple(
            vec![SchemaBody::Dynamic].into_boxed_slice(),
        ));
        let bound = |rank| {
            let child = schema(SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: vec![DimensionExpr::Constant(1); rank].into_boxed_slice(),
            });
            binding_cost(&output, &[(&SchemaBody::Dynamic, &child)]).unwrap()
        };
        let small = bound(2);
        let large = bound(64);
        assert!(large.metadata_nodes >= small.metadata_nodes + 2 * (64 - 2));
        assert!(large.temporary_bytes > small.temporary_bytes);
    }

    #[test]
    fn field_labels_charge_clone_bytes_and_work_without_becoming_nodes() {
        let scalar = schema(SchemaBody::Bool);
        let bound = |name| {
            let output = schema(SchemaBody::Record(
                vec![SchemaField {
                    name,
                    schema: SchemaBody::Bool,
                }]
                .into_boxed_slice(),
            ));
            binding_cost(&output, &[(&SchemaBody::Bool, &scalar)]).unwrap()
        };
        let small = bound("a".into());
        let large = bound("a".repeat(4097));
        assert!(large.temporary_bytes >= small.temporary_bytes + 4096);
        assert!(large.compute_work >= small.compute_work + 4096);
        assert_eq!(large.metadata_nodes, small.metadata_nodes);
    }
}
