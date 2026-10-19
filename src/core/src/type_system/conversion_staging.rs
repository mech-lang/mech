//! Pure layout/work facts shared by source and resident conversion admission.
use super::ConversionStep;
use crate::{
    CardinalitySpec, DimensionExpr, PlannedExecutionDemand, Schema, SchemaBody, ValueData,
    ValueDataDraft, WorkDemand,
};
use core::mem::size_of;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConversionSchemaStagingBound {
    pub temporary_bytes: u64,
    pub setup_clone_bytes: u64,
    pub metadata_nodes: u64,
    pub setup_work: u64,
    pub shape_clone_bytes: u64,
    pub shape_work: u64,
    pub plan_bytes: u64,
    pub plan_nodes: u64,
    pub plan_work: u64,
}

#[derive(Clone, Copy, Default)]
struct Geometry {
    nodes: u64,
    body_nodes: u64,
    depth: u64,
    dimensions: u64,
    search: bool,
}
impl Geometry {
    fn child(&mut self, child: Self) -> Option<()> {
        self.nodes = self.nodes.checked_add(child.nodes)?;
        self.body_nodes = self.body_nodes.checked_add(child.body_nodes)?;
        self.depth = self.depth.max(child.depth.checked_add(1)?);
        self.dimensions = self.dimensions.checked_add(child.dimensions)?;
        self.search |= child.search;
        Some(())
    }
}
fn dimension(value: &DimensionExpr) -> Option<Geometry> {
    let mut g = Geometry {
        nodes: 1,
        depth: 1,
        dimensions: 1,
        ..Geometry::default()
    };
    match value {
        DimensionExpr::Add(values)
        | DimensionExpr::Multiply(values)
        | DimensionExpr::Min(values)
        | DimensionExpr::Max(values) => {
            for value in values {
                g.child(dimension(value)?)?;
            }
        }
        // The exact published core unique-witness solver also searches a
        // bare Parameter. Constants and parameter-free expressions do not.
        DimensionExpr::Parameter(_) => g.search = true,
        _ => {}
    }
    Some(g)
}
fn cardinality(value: &CardinalitySpec) -> Option<Geometry> {
    match value {
        CardinalitySpec::Exact(value)
        | CardinalitySpec::Dynamic {
            upper_bound: Some(value),
        } => dimension(value),
        CardinalitySpec::Dynamic { upper_bound: None } => Some(Geometry {
            nodes: 1,
            depth: 1,
            ..Geometry::default()
        }),
    }
}
fn body(value: &SchemaBody) -> Option<Geometry> {
    let mut g = Geometry {
        nodes: 1,
        body_nodes: 1,
        depth: 1,
        ..Geometry::default()
    };
    match value {
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            g.child(body(element)?)?;
            for value in dimensions {
                g.child(dimension(value)?)?;
            }
        }
        SchemaBody::Option(value) => {
            g.child(body(value)?)?;
        }
        SchemaBody::Set {
            element,
            cardinality: count,
        } => {
            g.child(body(element)?)?;
            g.child(cardinality(count)?)?;
        }
        SchemaBody::Map {
            key,
            value,
            cardinality: count,
        } => {
            g.child(body(key)?)?;
            g.child(body(value)?)?;
            g.child(cardinality(count)?)?;
        }
        SchemaBody::Tuple(values) => {
            for value in values {
                g.child(body(value)?)?;
            }
        }
        SchemaBody::Record(fields)
        | SchemaBody::Table {
            columns: fields, ..
        } => {
            for field in fields {
                g.child(body(&field.schema)?)?;
            }
            if let SchemaBody::Table { rows, .. } = value {
                g.child(cardinality(rows)?)?;
            }
        }
        SchemaBody::Enum { variants, .. } => {
            for variant in variants {
                g.nodes = g.nodes.checked_add(1)?;
                if let Some(value) = &variant.payload {
                    g.child(body(value)?)?;
                }
            }
        }
        _ => {}
    }
    Some(g)
}

/// Bounds actual Rust schema/evidence/component layouts, not encoded lengths.
/// Source closing, semantic-kind/evidence construction and component solving
/// happen once per invocation. Current cardinality refresh only copies tapes.
pub fn conversion_schema_staging_bound(
    source: &Schema,
    target: &Schema,
) -> Option<ConversionSchemaStagingBound> {
    let mut g = Geometry {
        nodes: 1,
        depth: 1,
        ..Geometry::default()
    };
    for schema in [source, target] {
        g.child(body(schema.body())?)?;
        for parameter in schema.dimension_parameters() {
            g.child(dimension(parameter.lower_bound())?)?;
            if let Some(bound) = parameter.upper_bound() {
                g.child(dimension(bound)?)?;
            }
        }
    }
    let layout = source
        .clone_allocation_bound_bytes()?
        .checked_add(target.clone_allocation_bound_bytes()?)?
        .checked_add((2 * size_of::<Schema>()) as u64)?;
    // At most depth ancestor projections per node; the 32 populations cover
    // both schema/kind/evidence trees, closed components, witness arrays and
    // shape/normalization scratch simultaneously, plus fixed carrier headers.
    let temporary_bytes = layout
        .checked_mul(g.depth)?
        .checked_mul(32)?
        .checked_add(4096)?;
    let setup_clone_bytes = temporary_bytes.checked_mul(u64::from(g.nodes.max(1).ilog2()) + 1)?;
    let metadata_nodes = g
        .nodes
        .checked_mul(g.depth)?
        .checked_mul(32)?
        .checked_add(32)?;
    // Schema/evidence searches may be quadratic; semantic-name sorting adds
    // log(body nodes). These trees are NOT rebuilt on each solver round.
    let evidence_work = layout
        .checked_mul(g.depth)?
        .checked_mul(g.body_nodes.checked_add(1)?)?
        .checked_mul(u64::from(g.body_nodes.max(1).ilog2()) + 1)?;
    let parameters = source
        .dimension_parameters()
        .len()
        .max(target.dimension_parameters().len());
    let shape_clone_bytes = u64::try_from(parameters)
        .ok()?
        .checked_mul((2 * size_of::<u64>()) as u64)?;
    // The per-data preserving solver may revisit an expression at each nested
    // operator; tape validation also visits all declared bound expressions.
    // It does not run the unique-witness binary search or rebuild kinds.
    let dimensions = g.dimensions.checked_add(1)?;
    let shape_work = dimensions.checked_mul(dimensions)?.checked_mul(64)?;
    // The unique-witness solver has at most P+1 fixing sweeps and 64 binary
    // evaluations. Only dimension AST/tape work repeats, not schema clones.
    let parameters = source
        .dimension_parameters()
        .len()
        .checked_add(target.dimension_parameters().len())?;
    let rounds = if g.search { u64::from(u64::BITS) } else { 0 };
    let dimension_work = shape_work.checked_mul(
        u64::try_from(parameters)
            .ok()?
            .checked_add(1)?
            .checked_add(rounds)?,
    )?;
    let setup_work = setup_clone_bytes
        .checked_add(evidence_work)?
        .checked_add(dimension_work)?;
    // Recursive semantic plans retain full resolved/evidence owners at every
    // conversion step. Step depth is bounded by the sum of source and target
    // schema depths, not merely one root's temporary reconstruction.
    let plan_depth = g.depth.checked_mul(2)?.checked_add(1)?;
    let plan_bytes = temporary_bytes.checked_mul(plan_depth)?;
    let plan_nodes = metadata_nodes.checked_mul(plan_depth)?;
    let plan_work = setup_work.checked_mul(plan_depth)?;
    Some(ConversionSchemaStagingBound {
        temporary_bytes,
        setup_clone_bytes,
        metadata_nodes,
        setup_work,
        shape_clone_bytes,
        shape_work,
        plan_bytes,
        plan_nodes,
        plan_work,
    })
}

pub fn conversion_materialization_phases(mut step: &ConversionStep) -> u64 {
    let mut phases = 0u64;
    loop {
        step = match step {
            ConversionStep::MatrixElements(inner) | ConversionStep::MatrixToSet(inner) => {
                phases = phases.checked_add(1).unwrap_or(u64::MAX);
                &inner.step
            }
            ConversionStep::OptionPayload(inner) | ConversionStep::OptionPresent(inner) => {
                &inner.step
            }
            ConversionStep::Identity | ConversionStep::Scalar(_) => return phases.max(1),
        };
    }
}

/// Maximum new Option boxes along this single semantic conversion path.
pub fn conversion_option_present_depth(mut step: &ConversionStep) -> u64 {
    let mut wrappers = 0u64;
    loop {
        step = match step {
            ConversionStep::OptionPresent(inner) => {
                wrappers = wrappers.checked_add(1).unwrap_or(u64::MAX);
                &inner.step
            }
            ConversionStep::OptionPayload(inner)
            | ConversionStep::MatrixElements(inner)
            | ConversionStep::MatrixToSet(inner) => &inner.step,
            ConversionStep::Identity | ConversionStep::Scalar(_) => return wrappers,
        };
    }
}

fn sum(values: &[u64]) -> Option<u64> {
    values.iter().try_fold(0u64, |n, v| n.checked_add(*v))
}
fn mul(values: &[u64]) -> Option<u64> {
    values.iter().try_fold(1u64, |n, v| n.checked_mul(*v))
}

/// Complete private draft/normalization staging, independent of input/previous
/// owner pins, final publication, sort comparisons and output reservation.
/// Callers must fund those independent obligations as well.
pub fn conversion_staging_demand(
    metadata: ConversionSchemaStagingBound,
    expanded: u64,
    payload: u64,
    phases: u64,
) -> Option<PlannedExecutionDemand> {
    let passes = phases.checked_add(1)?;
    let visits = mul(&[expanded, passes])?;
    let shape_calls = visits.checked_add(8)?;
    let nodes = mul(&[expanded, phases])?;
    let staging = mul(&[
        nodes,
        (3 * size_of::<ValueDataDraft>()
            + 2 * size_of::<ValueData>()
            + size_of::<crate::snapshot::CanonicalKeyValue>()) as u64,
    ])?;
    Some(PlannedExecutionDemand {
        temporary_bytes: sum(&[
            mul(&[payload, 5, phases])?,
            staging,
            metadata.temporary_bytes,
            mul(&[metadata.shape_clone_bytes, passes])?,
        ])?,
        cloned_bytes: sum(&[
            mul(&[payload, 3, phases])?,
            metadata.setup_clone_bytes,
            mul(&[metadata.shape_clone_bytes, shape_calls])?,
        ])?,
        retained_nodes: sum(&[
            mul(&[nodes, 4])?,
            metadata.metadata_nodes,
            mul(&[metadata.shape_clone_bytes / size_of::<u64>() as u64, passes])?,
        ])?,
        work: WorkDemand {
            compute: sum(&[
                mul(&[payload, 4, phases])?,
                mul(&[nodes, 32])?,
                mul(&[visits, 64])?,
                metadata.setup_work,
                mul(&[metadata.shape_work, visits])?,
            ])?,
            ..WorkDemand::default()
        },
    })
}
