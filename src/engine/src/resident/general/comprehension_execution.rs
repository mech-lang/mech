use super::*;
use crate::resident::budget::{self, PreparedKernel, ResidentBudgetMeter};
use crate::resident::general::{
    ActivatedCollectionStep, ActivatedComprehensionNode, ActivatedPatternBinding,
    ActivatedPatternValue,
};
use mech_core::snapshot::{
    F64Bits, SnapshotCanonicalizationBudget, SnapshotValidationContext, ValueFootprint,
    canonical_snapshot_data_draft_with_context, dynamic_canonical_allocation_bound_bytes,
};
use mech_core::{
    CurrentMemoryFootprint, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, Schema, SchemaBody, SchemaDraft, SchemaId,
    SchemaKey, SchemaTable, SchemaTableBuilder, SemanticModelError, Value, ValueData,
    ValueDataDraft, ValueDraft,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, Default)]
struct ComprehensionLiveLocalFootprint {
    retained_prefix: usize,
    footprint: ValueFootprint,
}

#[derive(Clone, Copy)]
pub(super) enum Item {
    Bool(bool),
    Index(u64),
    F64(f64),
}

impl Item {
    fn from_draft(data: &ValueDataDraft) -> Option<Self> {
        match data {
            ValueDataDraft::Bool(value) => Some(Self::Bool(*value)),
            ValueDataDraft::Index(value) => Some(Self::Index(*value)),
            ValueDataDraft::F64(value) => Some(Self::F64(value.to_f64())),
            _ => None,
        }
    }
    fn data(self) -> ValueData {
        match self {
            Self::Bool(value) => ValueData::Bool(value),
            Self::Index(value) => ValueData::Index(value),
            Self::F64(value) => ValueData::F64(F64Bits::from_f64(value)),
        }
    }
}

fn collection_len(value: ResidentValueRef<'_>) -> Option<usize> {
    match value {
        ResidentValueRef::Snapshot([Some(value)]) => match value.data() {
            ValueData::Set(set) => Some(set.elements().len()),
            ValueData::Matrix(matrix) => Some(matrix.elements().len()),
            _ => None,
        },
        ResidentValueRef::Bool(value) => Some(value.len()),
        ResidentValueRef::Index(value) => Some(value.len()),
        ResidentValueRef::F64(value) => Some(value.len()),
        ResidentValueRef::String(value) => Some(value.len()),
        _ => None,
    }
}

fn allocation_capacity_bytes<T>(capacity: usize) -> Result<u64, ResidentKernelError> {
    budget::checked_u64(capacity)?
        .checked_mul(core::mem::size_of::<T>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)
}

fn completed_set_shape_values(
    schema: &mech_core::Schema,
    data: &ValueDataDraft,
) -> Result<Box<[u64]>, ResidentKernelError> {
    mech_core::shape_for_value_data(schema, data, &[], None)
        .map(|shape| shape.parameter_values().to_vec().into_boxed_slice())
        .map_err(|_| ResidentKernelError::InvalidShape)
}

fn closed_yield_body(
    value: ResidentValueRef<'_>,
    region: ResidentRegion,
    schema: SchemaId,
    schemas: &SchemaTable,
) -> Result<SchemaBody, ResidentKernelError> {
    let schema = schemas
        .get(schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let shape = match value {
        ResidentValueRef::Snapshot([Some(value)]) => value.shape().clone(),
        _ if schema.dimension_parameters().is_empty() => return Ok(schema.body().clone()),
        _ if matches!(schema.body(), SchemaBody::Matrix { .. }) => {
            mech_core::shape_for_resolved_extents(
                schema,
                &[
                    u64::from(region.shape.rows),
                    u64::from(region.shape.columns),
                ],
            )
            .map_err(|_| ResidentKernelError::InvalidShape)?
        }
        _ => return Err(ResidentKernelError::InvalidShape),
    };
    schema
        .closed_body(&shape)
        .map_err(|_| ResidentKernelError::InvalidShape)
}

fn lower_bound_yield_body(
    schema: SchemaId,
    schemas: &SchemaTable,
) -> Result<SchemaBody, ResidentKernelError> {
    let schema = schemas
        .get(schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let shape = mech_core::shape_for_declared_lower_bounds(schema)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    schema
        .closed_body(&shape)
        .map_err(|_| ResidentKernelError::InvalidShape)
}

#[derive(Clone, Debug)]
pub(super) enum PatternItem {
    Plain(ValueDataDraft),
    Dynamic(Option<Box<ValueDraft>>),
    Component {
        schema: Option<SchemaId>,
        body: SchemaBody,
        shape_values: Box<[u64]>,
        data: ValueDataDraft,
    },
    SourceComponent {
        projection_schema: Option<SchemaId>,
        value_schema: Option<SchemaId>,
        body: SchemaBody,
        shape_values: Box<[u64]>,
        data: ValueDataDraft,
        source_data: Option<ValueData>,
        context: Arc<SourceSchemaContext>,
        contexts: Arc<[Arc<SourceSchemaContext>]>,
    },
}

#[derive(Clone, Debug)]
pub(super) struct SourceSchemaContext {
    owner: Arc<SchemaTable>,
    schemas: Arc<SchemaTable>,
    projections: StructuralProjectionTable,
    binding_schemas: Arc<SchemaTable>,
    binding_schema_index: Arc<[(SchemaKey, SchemaId)]>,
}

pub(super) struct PatternBindingItem {
    pub(super) shape_values: Box<[u64]>,
    pub(super) data: ValueDataDraft,
    pub(super) schemas: Option<std::sync::Arc<SchemaTable>>,
    pub(super) schema_index: Option<Arc<[(SchemaKey, SchemaId)]>>,
    footprint: BindingFootprint,
}

#[derive(Clone, Copy, Debug)]
enum BindingFootprint {
    Selected,
    Concrete,
    DynamicWrap { nested_shape_parameters: usize },
}

struct ResolvedPatternItem {
    projection_schema: Option<SchemaId>,
    value_schema: Option<SchemaId>,
    shape_values: Box<[u64]>,
    body: SchemaBody,
    data: ValueDataDraft,
    schemas: Option<std::sync::Arc<SchemaTable>>,
    source_data: Option<ValueData>,
    source_context: Option<Arc<SourceSchemaContext>>,
    source_contexts: Option<Arc<[Arc<SourceSchemaContext>]>>,
}

fn projection_declarations(
    schema: &Schema,
) -> Result<Box<[DimensionParameterDeclaration]>, SemanticModelError> {
    schema
        .dimension_parameters()
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            Ok(DimensionParameterDeclaration {
                id: DimensionParameterId::new(
                    u32::try_from(index).map_err(|_| SemanticModelError::SchemaIdExhausted)?,
                ),
                // Keep inherited parameter order when one retained parameter
                // bounds another; only the appended rest extent is inferred.
                origin: DimensionParameterOrigin::Explicit,
                lifetime: parameter.lifetime(),
                lower_bound: parameter.lower_bound().clone(),
                upper_bound: parameter.upper_bound().cloned(),
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn projected_schema(parent: &Schema, body: SchemaBody) -> Result<Schema, SemanticModelError> {
    SchemaDraft {
        dimension_parameters: projection_declarations(parent)?,
        body,
    }
    .finalize()
}

fn projected_schema_shape(
    schema: SchemaId,
    closed_body: &SchemaBody,
    inherited_shape_values: &[u64],
    schemas: &SchemaTable,
) -> Option<(SchemaId, Box<[u64]>)> {
    let projected = schemas.get(schema)?;
    let seeded = if projected.dimension_parameters().len() == inherited_shape_values.len() {
        Some(inherited_shape_values.to_vec())
    } else if projected.dimension_parameters().len() == inherited_shape_values.len() + 1 {
        if let SchemaBody::Matrix { dimensions, .. } = closed_body
            && let [DimensionExpr::Constant(1), DimensionExpr::Constant(extent)] =
                dimensions.as_ref()
        {
            let mut values = inherited_shape_values.to_vec();
            values.push(*extent);
            Some(values)
        } else {
            None
        }
    } else {
        None
    };
    if let Some(values) = seeded
        && let Ok(shape) = projected.instantiate_shape(values.into_boxed_slice())
        && projected.closed_body(&shape).ok().as_ref() == Some(closed_body)
    {
        return Some((schema, shape.parameter_values().to_vec().into_boxed_slice()));
    }
    let inferred = mech_core::shape_for_schema_components(
        projected,
        &[(projected.body(), closed_body.clone())],
        None,
    )
    .ok()?;
    Some((
        schema,
        inferred.parameter_values().to_vec().into_boxed_slice(),
    ))
}

fn dynamic_binding_shape(
    binding: &Schema,
    selected_shape_values: Option<&[u64]>,
    source_shape_values: &[u64],
) -> Result<mech_core::ShapeInstance, ResidentKernelError> {
    let parameters = binding.dimension_parameters().len();
    let values = if parameters == 0 {
        &[][..]
    } else {
        selected_shape_values
            .filter(|values| values.len() == parameters)
            .or_else(|| (source_shape_values.len() == parameters).then_some(source_shape_values))
            .ok_or(ResidentKernelError::InvalidInput)?
    };
    binding
        .instantiate_shape(values.to_vec().into_boxed_slice())
        .map_err(|_| ResidentKernelError::InvalidInput)
}

fn projected_rest_schema(
    parent: &Schema,
    element: SchemaBody,
) -> Result<Schema, SemanticModelError> {
    let mut parameters = projection_declarations(parent)?.into_vec();
    let extent = DimensionParameterId::new(
        u32::try_from(parameters.len()).map_err(|_| SemanticModelError::SchemaIdExhausted)?,
    );
    parameters.push(DimensionParameterDeclaration {
        id: extent,
        origin: DimensionParameterOrigin::Inferred,
        lifetime: DimensionLifetime::Turn,
        lower_bound: DimensionExpr::Constant(0),
        upper_bound: None,
    });
    let projected = SchemaDraft {
        dimension_parameters: parameters.into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Parameter(extent)]
                .into_boxed_slice(),
        },
    }
    .finalize()?;
    Ok(projected)
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StructuralProjectionTable {
    entries: Box<[StructuralProjectionEntry]>,
}

#[derive(Clone, Debug, Default)]
struct StructuralProjectionEntry {
    tuple_children: Box<[Option<SchemaId>]>,
    matrix_element: Option<SchemaId>,
    matrix_rest: Option<SchemaId>,
}

impl StructuralProjectionTable {
    fn for_schemas(schemas: &SchemaTable) -> Result<Self, SemanticModelError> {
        let by_key = schemas
            .entries()
            .enumerate()
            .map(|(index, entry)| {
                Ok((
                    entry.key(),
                    SchemaId::new(
                        u32::try_from(index).map_err(|_| SemanticModelError::SchemaIdExhausted)?,
                    ),
                ))
            })
            .collect::<Result<std::collections::BTreeMap<_, _>, SemanticModelError>>()?;
        let entries = schemas
            .entries()
            .map(|entry| {
                let parent = entry.schema();
                let lookup = |schema: Schema| {
                    by_key
                        .get(&schema.key())
                        .copied()
                        .ok_or(SemanticModelError::InvalidSchemaHandleV1)
                };
                Ok(match parent.body() {
                    SchemaBody::Tuple(children) => StructuralProjectionEntry {
                        tuple_children: children
                            .iter()
                            .map(|child| {
                                projected_schema(parent, child.clone())
                                    .and_then(&lookup)
                                    .map(Some)
                            })
                            .collect::<Result<Vec<_>, _>>()?
                            .into_boxed_slice(),
                        ..StructuralProjectionEntry::default()
                    },
                    SchemaBody::Matrix { element, .. } => StructuralProjectionEntry {
                        matrix_element: Some(lookup(projected_schema(
                            parent,
                            element.as_ref().clone(),
                        )?)?),
                        matrix_rest: Some(lookup(projected_rest_schema(
                            parent,
                            element.as_ref().clone(),
                        )?)?),
                        ..StructuralProjectionEntry::default()
                    },
                    SchemaBody::Set { element, .. } => StructuralProjectionEntry {
                        matrix_element: Some(lookup(projected_schema(
                            parent,
                            element.as_ref().clone(),
                        )?)?),
                        ..StructuralProjectionEntry::default()
                    },
                    _ => StructuralProjectionEntry::default(),
                })
            })
            .collect::<Result<Vec<_>, SemanticModelError>>()?
            .into_boxed_slice();
        Ok(Self { entries })
    }

    fn tuple_child(&self, parent: SchemaId, index: usize) -> Option<SchemaId> {
        self.entries
            .get(parent.get() as usize)?
            .tuple_children
            .get(index)
            .copied()
            .flatten()
    }

    fn matrix_element(&self, parent: SchemaId) -> Option<SchemaId> {
        self.entries.get(parent.get() as usize)?.matrix_element
    }

    fn matrix_rest(&self, parent: SchemaId) -> Option<SchemaId> {
        self.entries.get(parent.get() as usize)?.matrix_rest
    }

    fn retained_footprint(&self) -> Option<(u64, u64)> {
        let mut bytes = u64::try_from(self.entries.len())
            .ok()?
            .checked_mul(core::mem::size_of::<StructuralProjectionEntry>() as u64)?;
        let mut nodes = u64::from(!self.entries.is_empty());
        for entry in &self.entries {
            bytes = bytes.checked_add(
                u64::try_from(entry.tuple_children.len())
                    .ok()?
                    .checked_mul(core::mem::size_of::<Option<SchemaId>>() as u64)?,
            )?;
            nodes = nodes.checked_add(u64::from(!entry.tuple_children.is_empty()))?;
        }
        Some((bytes, nodes))
    }
}

fn collect_projection_schemas(
    body: &SchemaBody,
    parameters: &[DimensionParameterDeclaration],
    output: &mut SchemaTableBuilder,
) -> Result<(), SemanticModelError> {
    output.insert(
        SchemaDraft {
            dimension_parameters: parameters.to_vec().into_boxed_slice(),
            body: body.clone(),
        }
        .finalize()?,
    )?;
    match body {
        SchemaBody::Enum { variants, .. } => {
            for payload in variants
                .iter()
                .filter_map(|variant| variant.payload.as_ref())
            {
                collect_projection_schemas(payload, parameters, output)?;
            }
        }
        SchemaBody::Option(child) | SchemaBody::Set { element: child, .. } => {
            collect_projection_schemas(child, parameters, output)?;
        }
        SchemaBody::Tuple(children) => {
            for child in children {
                collect_projection_schemas(child, parameters, output)?;
            }
        }
        SchemaBody::Record(fields)
        | SchemaBody::Table {
            columns: fields, ..
        } => {
            for field in fields {
                collect_projection_schemas(&field.schema, parameters, output)?;
            }
        }
        SchemaBody::Matrix { element, .. } => {
            let mut rest_parameters = parameters.to_vec();
            let extent = DimensionParameterId::new(
                u32::try_from(rest_parameters.len())
                    .map_err(|_| SemanticModelError::SchemaIdExhausted)?,
            );
            rest_parameters.push(DimensionParameterDeclaration {
                id: extent,
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            });
            output.insert(
                SchemaDraft {
                    dimension_parameters: rest_parameters.into_boxed_slice(),
                    body: SchemaBody::Matrix {
                        element: element.clone(),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(extent),
                        ]
                        .into_boxed_slice(),
                    },
                }
                .finalize()?,
            )?;
            collect_projection_schemas(element, parameters, output)?;
        }
        SchemaBody::Map { key, value, .. } => {
            collect_projection_schemas(key, parameters, output)?;
            collect_projection_schemas(value, parameters, output)?;
        }
        SchemaBody::Dynamic
        | SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::IntegerInterval(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_)
        | SchemaBody::ReifiedType => {}
    }
    Ok(())
}

pub(super) fn structural_projection_schema_context(
    schemas: &SchemaTable,
) -> Result<(SchemaTable, StructuralProjectionTable), SemanticModelError> {
    let mut projections = SchemaTableBuilder::new();
    for entry in schemas.entries() {
        let parameters = projection_declarations(entry.schema())?;
        collect_projection_schemas(entry.schema().body(), &parameters, &mut projections)?;
    }
    let schemas = schemas.extend_preserving_ids(&projections.finish()?.table)?;
    let projections = StructuralProjectionTable::for_schemas(&schemas)?;
    Ok((schemas, projections))
}

fn source_schema_index(
    schemas: &SchemaTable,
) -> Result<Arc<[(SchemaKey, SchemaId)]>, SemanticModelError> {
    let mut entries = schemas
        .entries()
        .enumerate()
        .map(|(index, entry)| {
            Ok((
                entry.key(),
                SchemaId::new(
                    u32::try_from(index).map_err(|_| SemanticModelError::SchemaIdExhausted)?,
                ),
            ))
        })
        .collect::<Result<Vec<_>, SemanticModelError>>()?;
    entries.sort_unstable_by_key(|(key, _)| *key);
    Ok(entries.into())
}

fn indexed_schema_id(index: &[(SchemaKey, SchemaId)], key: SchemaKey) -> Option<SchemaId> {
    index
        .binary_search_by_key(&key, |(candidate, _)| *candidate)
        .ok()
        .map(|position| index[position].1)
}

fn source_schema_contexts(
    value: &mech_core::snapshot::Value,
    plan_schemas: &SchemaTable,
) -> Option<(Arc<SourceSchemaContext>, Arc<[Arc<SourceSchemaContext>]>)> {
    let root_owner = value.schemas()?;
    let mut discovery_meter = ResidentBudgetMeter::default();
    // The same admitted collector is used by the preflight and by actual
    // context construction. It stores every occurrence once, sorts by Arc
    // identity, and deduplicates before any owner closure is built.
    let owners = distinct_source_owners(value, &mut discovery_meter).ok()?;
    let projected = owners
        .owners
        .into_iter()
        .map(|owner| {
            let (schemas, projections) = structural_projection_schema_context(&owner).ok()?;
            Some((owner, schemas, projections))
        })
        .collect::<Option<Vec<_>>>()?;
    let root_index = projected
        .iter()
        .position(|(owner, _, _)| Arc::ptr_eq(owner, &root_owner))?;
    // Preserve the root owner's ordinals, then append every independently
    // owned nested schema by canonical key. Drafts rebound into this arena can
    // finalize a whole composite without interpreting a nested owner's local
    // schema ID against the root table.
    let binding_schemas = projected
        .get(root_index)?
        .1
        .extend_many_preserving_ids(
            projected
                .iter()
                .enumerate()
                .filter_map(|(index, (_, schemas, _))| (index != root_index).then_some(schemas))
                .chain(core::iter::once(plan_schemas)),
        )
        .ok()?;
    let binding_schema_index = source_schema_index(&binding_schemas).ok()?;
    let binding_schemas = Arc::new(binding_schemas);
    // `distinct_source_owners` returns pointer-sorted owners. Preserve that
    // order so every Dynamic owner switch uses the binary identity lookup.
    let contexts = projected
        .into_iter()
        .map(|(owner, schemas, projections)| {
            Arc::new(SourceSchemaContext {
                owner,
                schemas: Arc::new(schemas),
                projections,
                binding_schemas: Arc::clone(&binding_schemas),
                binding_schema_index: Arc::clone(&binding_schema_index),
            })
        })
        .collect::<Vec<_>>();
    let root = contexts.get(root_index)?.clone();
    Some((root, contexts.into()))
}

fn source_context_for_owner(
    contexts: &[Arc<SourceSchemaContext>],
    owner: &Arc<SchemaTable>,
) -> Option<Arc<SourceSchemaContext>> {
    let identity = Arc::as_ptr(owner) as usize;
    contexts
        .binary_search_by_key(&identity, |context| Arc::as_ptr(&context.owner) as usize)
        .ok()
        .and_then(|position| contexts.get(position))
        .filter(|context| Arc::ptr_eq(&context.owner, owner))
        .cloned()
}

#[derive(Clone, Copy, Default)]
struct SourceContextMaterializationBound {
    retained_bytes: u64,
    temporary_bytes: u64,
    retained_nodes: u64,
    work: u64,
    merged_schema_nodes: u64,
    max_shape_parameters: usize,
}

struct DistinctSourceOwners {
    owners: Vec<Arc<SchemaTable>>,
    allocation_bytes: u64,
}

fn distinct_source_owners(
    value: &Value,
    meter: &mut ResidentBudgetMeter,
) -> Result<DistinctSourceOwners, ResidentKernelError> {
    let mut occurrences = 0_u64;
    visit_value_schema_owners(value, meter, &mut |_, _| {
        occurrences = occurrences
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    let capacity = usize::try_from(occurrences).map_err(|_| ResidentKernelError::InvalidShape)?;
    let minimum_bytes = occurrences
        .checked_mul(core::mem::size_of::<Arc<SchemaTable>>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let sort_work = occurrences
        .checked_mul(occurrences.max(1).ilog2() as u64 + 1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    meter.charge_comparison_work(sort_work)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes: minimum_bytes,
            ..meter.estimate()
        },
    )
    .admit_control()?
    .into_plan();

    let mut owners = Vec::new();
    owners
        .try_reserve_exact(capacity)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let allocation_bytes = budget::checked_u64(owners.capacity())?
        .checked_mul(core::mem::size_of::<Arc<SchemaTable>>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes: allocation_bytes,
            ..meter.estimate()
        },
    )
    .admit_control()?
    .into_plan();
    visit_value_schema_owners(value, meter, &mut |owner, _| {
        owners.push(Arc::clone(owner));
        Ok(())
    })?;
    owners.sort_unstable_by_key(|owner| Arc::as_ptr(owner) as usize);
    owners.dedup_by(|left, right| Arc::ptr_eq(left, right));
    Ok(DistinctSourceOwners {
        owners,
        allocation_bytes,
    })
}

impl SourceContextMaterializationBound {
    fn add_owner(
        &mut self,
        owner: &SchemaTable,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<(), ResidentKernelError> {
        self.max_shape_parameters = self.max_shape_parameters.max(
            owner
                .entries()
                .map(|entry| entry.schema().dimension_parameters().len())
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        meter.charge_comparison_work(budget::checked_u64(owner.len())?)?;
        let remaining = meter.estimate().remaining_incremental_work()?;
        let closure_budget = SnapshotCanonicalizationBudget::new(remaining);
        let (retained, construction, schema_nodes) = owner
            .component_closure_bounds_with_budget(&closure_budget)
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.charge_comparison_work(closure_budget.consumed())?;
        let index_bytes = schema_nodes
            .checked_mul(core::mem::size_of::<(SchemaKey, SchemaId)>() as u64)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let index_work = schema_nodes
            .checked_mul(schema_nodes.max(1).ilog2() as u64 + 1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let construction = construction
            .checked_mul(4)
            .and_then(|bytes| bytes.checked_add(index_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let retained = retained
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(index_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let nodes = schema_nodes
            .checked_mul(5)
            .and_then(|nodes| nodes.checked_add(schema_nodes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.retained_bytes = self
            .retained_bytes
            .checked_add(retained)
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.temporary_bytes = self
            .temporary_bytes
            .checked_add(construction)
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.retained_nodes = self
            .retained_nodes
            .checked_add(nodes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.work = self
            .work
            .checked_add(construction)
            .and_then(|work| work.checked_add(nodes))
            .and_then(|work| work.checked_add(index_work))
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.merged_schema_nodes = self
            .merged_schema_nodes
            .checked_add(schema_nodes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    }

    fn add_binding_merge_work(&mut self) -> Result<(), ResidentKernelError> {
        let merge_work = self
            .merged_schema_nodes
            .checked_mul(self.merged_schema_nodes.max(1).ilog2() as u64 + 1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.work = self
            .work
            .checked_add(merge_work)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    }
}

fn source_context_materialization_bound(
    value: &mech_core::snapshot::Value,
    plan_schemas: &SchemaTable,
    meter: &mut ResidentBudgetMeter,
) -> Result<SourceContextMaterializationBound, ResidentKernelError> {
    let mut bound = SourceContextMaterializationBound::default();
    let owners = distinct_source_owners(value, meter)?;
    bound.temporary_bytes = owners.allocation_bytes;
    for owner in &owners.owners {
        bound.add_owner(owner, meter)?;
    }
    if !owners
        .owners
        .iter()
        .any(|owner| std::ptr::eq(owner.as_ref(), plan_schemas))
    {
        bound.add_owner(plan_schemas, meter)?;
    }
    bound.add_binding_merge_work()?;
    Ok(bound)
}

fn resolve_pattern_item(
    item: PatternItem,
    schemas: &SchemaTable,
) -> Result<Option<ResolvedPatternItem>, ResidentKernelError> {
    match item {
        PatternItem::Plain(_) => Err(ResidentKernelError::InvalidInput),
        PatternItem::Component {
            schema,
            body,
            shape_values,
            data,
        } => Ok(Some(ResolvedPatternItem {
            projection_schema: schema,
            value_schema: schema,
            shape_values,
            body,
            data,
            schemas: None,
            source_data: None,
            source_context: None,
            source_contexts: None,
        })),
        PatternItem::SourceComponent {
            mut projection_schema,
            mut value_schema,
            mut body,
            mut shape_values,
            mut data,
            mut source_data,
            mut context,
            contexts,
        } => loop {
            if !matches!(body, SchemaBody::Dynamic) {
                let value_schema = value_schema
                    .map(|schema| {
                        let key = context
                            .schemas
                            .entry(schema)
                            .ok_or(ResidentKernelError::InvalidInput)?
                            .key();
                        indexed_schema_id(&context.binding_schema_index, key)
                            .ok_or(ResidentKernelError::InvalidInput)
                    })
                    .transpose()?;
                return Ok(Some(ResolvedPatternItem {
                    projection_schema,
                    value_schema,
                    shape_values,
                    body,
                    data,
                    schemas: Some(Arc::clone(&context.binding_schemas)),
                    source_data,
                    source_context: Some(context),
                    source_contexts: Some(contexts),
                }));
            }
            let ValueDataDraft::Dynamic(Some(draft)) = data else {
                return Ok(None);
            };
            let Some(ValueData::Dynamic(canonical)) = source_data else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let value = canonical.value().ok_or(ResidentKernelError::InvalidInput)?;
            let owner = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
            context = source_context_for_owner(&contexts, &owner)
                .ok_or(ResidentKernelError::InvalidInput)?;
            let ValueDraft {
                schema: draft_schema,
                shape_values: _,
                data: nested,
            } = *draft;
            let schema = value.schema();
            if draft_schema != schema
                || value.schema_key()
                    != context
                        .schemas
                        .entry(schema)
                        .ok_or(ResidentKernelError::InvalidInput)?
                        .key()
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            let definition = context
                .schemas
                .get(schema)
                .ok_or(ResidentKernelError::InvalidInput)?;
            body = definition
                .closed_body(value.shape())
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            shape_values = value.shape().parameter_values().to_vec().into_boxed_slice();
            data = nested;
            source_data = Some(value.data().clone());
            projection_schema = Some(schema);
            value_schema = Some(schema);
        },
        PatternItem::Dynamic(None) => Ok(None),
        PatternItem::Dynamic(Some(mut value)) => loop {
            let ValueDraft {
                schema,
                shape_values,
                data,
            } = *value;
            let shape = schemas
                .get(schema)
                .ok_or(ResidentKernelError::InvalidInput)?
                .instantiate_shape(shape_values)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            let body = schemas
                .get(schema)
                .ok_or(ResidentKernelError::InvalidInput)?
                .closed_body(&shape)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            match (body, data) {
                (SchemaBody::Dynamic, ValueDataDraft::Dynamic(Some(next))) => value = next,
                (SchemaBody::Dynamic, ValueDataDraft::Dynamic(None)) => return Ok(None),
                (body, data) => {
                    return Ok(Some(ResolvedPatternItem {
                        projection_schema: Some(schema),
                        value_schema: Some(schema),
                        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                        body,
                        data,
                        schemas: None,
                        source_data: None,
                        source_context: None,
                        source_contexts: None,
                    }));
                }
            }
        },
    }
}

fn adapt_resolved_pattern_item(
    item: ResolvedPatternItem,
    target: &SchemaBody,
    schemas: &SchemaTable,
    projections: &StructuralProjectionTable,
) -> Result<Option<ValueDataDraft>, ResidentKernelError> {
    fn binding_projection_schema(
        schema: Option<SchemaId>,
        projection_schemas: &SchemaTable,
        source_context: Option<&SourceSchemaContext>,
    ) -> Result<Option<SchemaId>, ResidentKernelError> {
        let (Some(schema), Some(context)) = (schema, source_context) else {
            return Ok(schema);
        };
        let key = projection_schemas
            .entry(schema)
            .ok_or(ResidentKernelError::InvalidInput)?
            .key();
        indexed_schema_id(&context.binding_schema_index, key)
            .map(Some)
            .ok_or(ResidentKernelError::InvalidInput)
    }

    if item.body == *target {
        if let (Some(source_data), Some(context)) =
            (item.source_data.as_ref(), item.source_context.as_ref())
        {
            let validation =
                SnapshotValidationContext::with_shared_schemas(&context.binding_schemas)
                    .with_schema_index(&context.binding_schema_index);
            return canonical_snapshot_data_draft_with_context(
                &item.body,
                source_data,
                &validation,
            )
            .map(Some)
            .map_err(|_| ResidentKernelError::InvalidInput);
        }
        return Ok(Some(item.data));
    }
    if matches!(target, SchemaBody::Dynamic) {
        let schema = item.value_schema.ok_or(ResidentKernelError::InvalidInput)?;
        let data = if let (Some(source_data), Some(context)) =
            (item.source_data.as_ref(), item.source_context.as_ref())
        {
            let validation =
                SnapshotValidationContext::with_shared_schemas(&context.binding_schemas)
                    .with_schema_index(&context.binding_schema_index);
            canonical_snapshot_data_draft_with_context(&item.body, source_data, &validation)
                .map_err(|_| ResidentKernelError::InvalidInput)?
        } else {
            item.data
        };
        return Ok(Some(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema,
            shape_values: item.shape_values,
            data,
        })))));
    }
    let source_schemas = item.schemas.clone();
    let source_data = item.source_data;
    let source_context = item.source_context.clone();
    let source_contexts = item.source_contexts.clone();
    let (projection_schemas, projection_table) = source_context
        .as_ref()
        .map_or((schemas, projections), |context| {
            (context.schemas.as_ref(), &context.projections)
        });
    match (item.body, target, item.data) {
        (
            SchemaBody::Matrix {
                element: source,
                dimensions: source_dimensions,
            },
            SchemaBody::Matrix {
                element: target,
                dimensions: target_dimensions,
            },
            ValueDataDraft::Matrix(values),
        ) if source_dimensions == *target_dimensions => {
            let projection = item.projection_schema.and_then(|schema| {
                projected_schema_shape(
                    projection_table.matrix_element(schema)?,
                    source.as_ref(),
                    &item.shape_values,
                    projection_schemas,
                )
            });
            let values = values
                .into_vec()
                .into_iter()
                .enumerate()
                .map(|(index, data)| {
                    let child_source_data = source_data
                        .as_ref()
                        .map(|source| {
                            source_data_child(source, index)
                                .ok_or(ResidentKernelError::InvalidInput)
                        })
                        .transpose()?;
                    let projection_schema = projection.as_ref().map(|(schema, _)| *schema);
                    let value_schema = binding_projection_schema(
                        projection_schema,
                        projection_schemas,
                        source_context.as_deref(),
                    )?;
                    adapt_resolved_pattern_item(
                        ResolvedPatternItem {
                            projection_schema,
                            value_schema,
                            shape_values: projection
                                .as_ref()
                                .map_or_else(Box::default, |(_, shape)| shape.clone()),
                            body: source.as_ref().clone(),
                            data,
                            schemas: source_schemas.clone(),
                            source_data: child_source_data,
                            source_context: source_context.clone(),
                            source_contexts: source_contexts.clone(),
                        },
                        target,
                        schemas,
                        projections,
                    )
                })
                .collect::<Result<Option<Vec<_>>, _>>()?;
            Ok(values.map(|values| ValueDataDraft::Matrix(values.into_boxed_slice())))
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target), ValueDataDraft::Tuple(values))
            if source.len() == target.len() && source.len() == values.len() =>
        {
            let parent = item.projection_schema;
            let values = source
                .into_vec()
                .into_iter()
                .enumerate()
                .zip(target)
                .zip(values.into_vec())
                .map(|(((index, source), target), data)| {
                    let child_source_data = source_data
                        .as_ref()
                        .map(|source| {
                            source_data_child(source, index)
                                .ok_or(ResidentKernelError::InvalidInput)
                        })
                        .transpose()?;
                    let projection = parent.and_then(|parent| {
                        projected_schema_shape(
                            projection_table.tuple_child(parent, index)?,
                            &source,
                            &item.shape_values,
                            projection_schemas,
                        )
                    });
                    let projection_schema = projection.as_ref().map(|(schema, _)| *schema);
                    let value_schema = binding_projection_schema(
                        projection_schema,
                        projection_schemas,
                        source_context.as_deref(),
                    )?;
                    adapt_resolved_pattern_item(
                        ResolvedPatternItem {
                            projection_schema,
                            value_schema,
                            shape_values: projection.map_or_else(Box::default, |(_, shape)| shape),
                            body: source,
                            data,
                            schemas: source_schemas.clone(),
                            source_data: child_source_data,
                            source_context: source_context.clone(),
                            source_contexts: source_contexts.clone(),
                        },
                        target,
                        schemas,
                        projections,
                    )
                })
                .collect::<Result<Option<Vec<_>>, _>>()?;
            Ok(values.map(|values| ValueDataDraft::Tuple(values.into_boxed_slice())))
        }
        _ => Ok(None),
    }
}

fn binding_shape_observation(target: &SchemaBody, actual: &SchemaBody) -> Option<SchemaBody> {
    if matches!(target, SchemaBody::Dynamic) {
        return Some(SchemaBody::Dynamic);
    }
    if target == actual {
        return Some(actual.clone());
    }
    match (target, actual) {
        (
            SchemaBody::Matrix {
                element: target, ..
            },
            SchemaBody::Matrix {
                element: actual,
                dimensions,
            },
        ) => Some(SchemaBody::Matrix {
            element: Box::new(binding_shape_observation(target, actual)?),
            dimensions: dimensions.clone(),
        }),
        (SchemaBody::Tuple(target), SchemaBody::Tuple(actual)) if target.len() == actual.len() => {
            Some(SchemaBody::Tuple(
                target
                    .iter()
                    .zip(actual)
                    .map(|(target, actual)| binding_shape_observation(target, actual))
                    .collect::<Option<Vec<_>>>()?
                    .into_boxed_slice(),
            ))
        }
        _ => None,
    }
}

pub(super) fn binding_schema_owner(
    schema: SchemaId,
    plan_schemas: &std::sync::Arc<SchemaTable>,
    source_schemas: Option<std::sync::Arc<SchemaTable>>,
    source_schema_index: Option<&[(SchemaKey, SchemaId)]>,
) -> Result<(SchemaId, std::sync::Arc<SchemaTable>), ResidentKernelError> {
    let Some(source_schemas) = source_schemas else {
        return Ok((schema, std::sync::Arc::clone(plan_schemas)));
    };
    let source_schema_index = source_schema_index.ok_or(ResidentKernelError::InvalidInput)?;
    let key = plan_schemas
        .entry(schema)
        .ok_or(ResidentKernelError::InvalidInput)?
        .key();
    let schema =
        indexed_schema_id(source_schema_index, key).ok_or(ResidentKernelError::InvalidInput)?;
    Ok((schema, source_schemas))
}

pub(super) fn finalize_pattern_binding(
    schema: SchemaId,
    shape_values: &[u64],
    data: ValueDataDraft,
    source_schemas: Option<std::sync::Arc<SchemaTable>>,
    source_schema_index: Option<Arc<[(SchemaKey, SchemaId)]>>,
    plan_schemas: &std::sync::Arc<SchemaTable>,
    canonical_budget: &SnapshotCanonicalizationBudget,
) -> Result<mech_core::snapshot::Value, ResidentKernelError> {
    if source_schemas.is_some()
        && matches!(
            plan_schemas
                .get(schema)
                .ok_or(ResidentKernelError::InvalidInput)?
                .body(),
            SchemaBody::Dynamic
        )
    {
        let ValueDataDraft::Dynamic(value) = data else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let value = value
            .map(|value| {
                value
                    .finalize(
                        &SnapshotValidationContext::with_shared_schemas(
                            source_schemas
                                .as_ref()
                                .expect("foreign Dynamic binding retains its schema owner"),
                        )
                        .with_canonicalization_budget(canonical_budget),
                    )
                    .map_err(|_| ResidentKernelError::InvalidInput)
            })
            .transpose()?;
        return mech_core::snapshot::wrap_resident_dynamic_value(
            schema,
            shape_values.to_vec().into_boxed_slice(),
            std::sync::Arc::clone(plan_schemas),
            value,
        )
        .map_err(|_| ResidentKernelError::InvalidInput);
    }
    let (schema, schemas) = binding_schema_owner(
        schema,
        plan_schemas,
        source_schemas,
        source_schema_index.as_deref(),
    )?;
    pattern_binding_draft(schema, shape_values, data)
        .finalize(
            &SnapshotValidationContext::with_shared_schemas(&schemas)
                .with_canonicalization_budget(canonical_budget),
        )
        .map_err(|_| ResidentKernelError::InvalidInput)
}

fn source_sequence_item(
    sequence: mech_core::snapshot::SequenceView<'_>,
    index: usize,
) -> Option<ValueData> {
    use mech_core::snapshot::SequenceView;
    Some(match sequence {
        SequenceView::U8(values) => ValueData::U8(*values.get(index)?),
        SequenceView::U16(values) => ValueData::U16(*values.get(index)?),
        SequenceView::U32(values) => ValueData::U32(*values.get(index)?),
        SequenceView::U64(values) => ValueData::U64(*values.get(index)?),
        SequenceView::U128(values) => ValueData::U128(*values.get(index)?),
        SequenceView::I8(values) => ValueData::I8(*values.get(index)?),
        SequenceView::I16(values) => ValueData::I16(*values.get(index)?),
        SequenceView::I32(values) => ValueData::I32(*values.get(index)?),
        SequenceView::I64(values) => ValueData::I64(*values.get(index)?),
        SequenceView::I128(values) => ValueData::I128(*values.get(index)?),
        SequenceView::F32(values) => ValueData::F32(*values.get(index)?),
        SequenceView::F64(values) => ValueData::F64(*values.get(index)?),
        SequenceView::Complex32(values) => ValueData::Complex32(*values.get(index)?),
        SequenceView::Complex64(values) => ValueData::Complex64(*values.get(index)?),
        SequenceView::Rational64(values) => ValueData::Rational64(values.get(index)?.clone()),
        SequenceView::Bool(values) => ValueData::Bool(*values.get(index)?),
        SequenceView::String(values) => ValueData::String(values.get(index)?.clone()),
        SequenceView::Id(values) => ValueData::Id(*values.get(index)?),
        SequenceView::Index(values) => ValueData::Index(*values.get(index)?),
        SequenceView::Unit(count) if u64::try_from(index).ok()? < count => ValueData::Atom,
        SequenceView::Values(values) => values.get(index)?.clone(),
        SequenceView::Unit(_) => return None,
    })
}

fn source_data_child(data: &ValueData, index: usize) -> Option<ValueData> {
    match data {
        ValueData::Tuple(values) => values.get(index).cloned(),
        ValueData::Matrix(matrix) => source_sequence_item(matrix.elements(), index),
        _ => None,
    }
}

fn source_data_middle(data: &ValueData, prefix: usize, suffix: usize) -> Option<ValueData> {
    use mech_core::snapshot::{MatrixValue, SequenceView};
    let ValueData::Matrix(matrix) = data else {
        return None;
    };
    let end = matrix.elements().len().checked_sub(suffix)?;
    if prefix > end {
        return None;
    }
    let matrix = match matrix.elements() {
        SequenceView::Bool(values) => {
            MatrixValue::from_bool_elements(values[prefix..end].to_vec().into_boxed_slice())
        }
        SequenceView::Index(values) => {
            MatrixValue::from_index_elements(values[prefix..end].to_vec().into_boxed_slice())
        }
        SequenceView::F64(values) => {
            MatrixValue::from_f64_elements(values[prefix..end].to_vec().into_boxed_slice())
        }
        SequenceView::String(values) => {
            MatrixValue::from_string_elements(values[prefix..end].to_vec().into_boxed_slice())
        }
        SequenceView::Values(values) => {
            MatrixValue::from_value_elements(values[prefix..end].to_vec().into_boxed_slice())
        }
        _ => return None,
    };
    Some(ValueData::Matrix(matrix))
}

impl PatternItem {
    fn metadata_bytes(&self) -> Result<u64, ResidentKernelError> {
        let (body, shape_values) = match self {
            Self::Component {
                body, shape_values, ..
            }
            | Self::SourceComponent {
                body, shape_values, ..
            } => (body, shape_values),
            Self::Plain(_) | Self::Dynamic(_) => return Ok(0),
        };
        body.clone_allocation_bound_bytes()
            .and_then(|bytes| bytes.checked_add(core::mem::size_of::<SchemaBody>() as u64))
            .and_then(|bytes| {
                u64::try_from(shape_values.len())
                    .ok()?
                    .checked_mul(core::mem::size_of::<u64>() as u64)?
                    .checked_add(bytes)
            })
            .ok_or(ResidentKernelError::InvalidShape)
    }

    pub(super) fn new(data: ValueDataDraft) -> Self {
        match data {
            ValueDataDraft::Dynamic(value) => Self::Dynamic(value),
            data => Self::Plain(data),
        }
    }

    pub(super) fn enum_variant(
        &self,
        schemas: &SchemaTable,
    ) -> Result<Option<(u32, Option<Self>)>, ResidentKernelError> {
        let Some(mut resolved) = resolve_pattern_item(self.clone(), schemas)? else {
            return Ok(None);
        };
        let option_payload = match &resolved.body {
            SchemaBody::Option(payload) => Some(payload.as_ref().clone()),
            _ => None,
        };
        if let Some(body) = option_payload {
            let ValueDataDraft::Option(value) = resolved.data else {
                return Err(ResidentKernelError::InvalidInput);
            };
            if !value.present {
                return Ok(None);
            }
            let Some(data) = value.value else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let source_payload = resolved.source_data.take().and_then(|data| match data {
                ValueData::Option(Some(value)) => Some(*value),
                _ => None,
            });
            resolved.projection_schema = None;
            resolved.value_schema = None;
            resolved.body = body;
            resolved.data = *data;
            resolved.source_data = source_payload;
        }
        let SchemaBody::Enum { variants, .. } = &resolved.body else {
            return Ok(None);
        };
        let ValueDataDraft::Enum(value) = resolved.data else {
            return Ok(None);
        };
        let Some(variant) = variants.get(value.ordinal as usize) else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let source_payload = resolved.source_data.as_ref().and_then(|data| match data {
            ValueData::Enum(value) => value.payload().cloned(),
            _ => None,
        });
        let payload = match (variant.payload.as_ref(), value.payload) {
            (None, None) => None,
            (Some(body), Some(data)) => Some(
                if let (Some(context), Some(contexts)) =
                    (resolved.source_context, resolved.source_contexts)
                {
                    Self::source_component(
                        None,
                        None,
                        body.clone(),
                        resolved.shape_values,
                        *data,
                        source_payload,
                        context,
                        contexts,
                    )
                } else {
                    Self::component(None, body.clone(), resolved.shape_values, *data)
                },
            ),
            _ => return Err(ResidentKernelError::InvalidInput),
        };
        Ok(Some((value.ordinal, payload)))
    }

    fn component(
        schema: Option<SchemaId>,
        body: SchemaBody,
        shape_values: Box<[u64]>,
        data: ValueDataDraft,
    ) -> Self {
        if matches!(body, SchemaBody::Dynamic)
            && let ValueDataDraft::Dynamic(value) = data
        {
            return Self::Dynamic(value);
        }
        Self::Component {
            schema,
            body,
            shape_values,
            data,
        }
    }

    fn source_component(
        projection_schema: Option<SchemaId>,
        value_schema: Option<SchemaId>,
        body: SchemaBody,
        shape_values: Box<[u64]>,
        data: ValueDataDraft,
        source_data: Option<ValueData>,
        context: Arc<SourceSchemaContext>,
        contexts: Arc<[Arc<SourceSchemaContext>]>,
    ) -> Self {
        Self::SourceComponent {
            projection_schema,
            value_schema,
            body,
            shape_values,
            data,
            source_data,
            context,
            contexts,
        }
    }

    fn dynamic_data(value: &ValueDraft) -> Option<&ValueDataDraft> {
        match &value.data {
            ValueDataDraft::Dynamic(Some(value)) => Self::dynamic_data(value),
            ValueDataDraft::Dynamic(None) => None,
            data => Some(data),
        }
    }

    fn resolved_source_data(&self) -> Result<Option<&ValueDataDraft>, ResidentKernelError> {
        let Self::SourceComponent {
            body,
            data,
            source_data,
            context: _,
            contexts,
            ..
        } = self
        else {
            return Ok(self.data());
        };
        let mut body = body.clone();
        let mut data = data;
        let mut source_data = source_data.as_ref();
        while matches!(body, SchemaBody::Dynamic) {
            let ValueDataDraft::Dynamic(Some(draft)) = data else {
                return Ok(None);
            };
            let Some(ValueData::Dynamic(canonical)) = source_data else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let value = canonical.value().ok_or(ResidentKernelError::InvalidInput)?;
            let owner = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
            let context = source_context_for_owner(contexts, &owner)
                .ok_or(ResidentKernelError::InvalidInput)?;
            if draft.schema != value.schema()
                || value.schema_key()
                    != context
                        .schemas
                        .entry(value.schema())
                        .ok_or(ResidentKernelError::InvalidInput)?
                        .key()
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            body = context
                .schemas
                .get(value.schema())
                .ok_or(ResidentKernelError::InvalidInput)?
                .closed_body(value.shape())
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            data = &draft.data;
            source_data = Some(value.data());
        }
        Ok(Some(data))
    }

    pub(super) fn data(&self) -> Option<&ValueDataDraft> {
        match self {
            Self::Plain(data) => Some(data),
            Self::Dynamic(Some(value)) => Self::dynamic_data(value),
            Self::Dynamic(None) => None,
            Self::Component { data, .. } => Some(data),
            Self::SourceComponent { .. } => self.resolved_source_data().ok().flatten(),
        }
    }

    pub(super) fn scalar(&self) -> Option<Item> {
        self.data().and_then(Item::from_draft)
    }

    pub(super) fn structural_len(&self, tuple: bool) -> Option<usize> {
        match (tuple, self.data()?) {
            (true, ValueDataDraft::Tuple(items)) | (false, ValueDataDraft::Matrix(items)) => {
                Some(items.len())
            }
            _ => None,
        }
    }

    pub(super) fn child(
        &self,
        index: usize,
        schemas: &mech_core::SchemaTable,
        projections: &StructuralProjectionTable,
    ) -> Option<Self> {
        fn selected(
            projection_schema: Option<SchemaId>,
            closed_body: &SchemaBody,
            fallback_shape_values: &[u64],
            data: &ValueDataDraft,
            index: usize,
            schemas: &SchemaTable,
            projections: &StructuralProjectionTable,
            source_data: Option<&ValueData>,
            source_context: Option<Arc<SourceSchemaContext>>,
            source_contexts: Option<Arc<[Arc<SourceSchemaContext>]>>,
        ) -> Option<PatternItem> {
            let (body, data) = match (closed_body, data) {
                (SchemaBody::Tuple(bodies), ValueDataDraft::Tuple(items)) => {
                    let body = bodies.get(index)?.clone();
                    let data = items.get(index)?.clone();
                    (body, data)
                }
                (SchemaBody::Matrix { element, .. }, ValueDataDraft::Matrix(items)) => {
                    let data = items.get(index)?.clone();
                    (element.as_ref().clone(), data)
                }
                _ => return None,
            };
            let selected_source_data = source_data.and_then(|data| source_data_child(data, index));
            let (projection_schemas, projection_table) = source_context
                .as_ref()
                .map_or((schemas, projections), |context| {
                    (context.schemas.as_ref(), &context.projections)
                });
            let projection = projection_schema.and_then(|schema| {
                let parent = projection_schemas.get(schema)?;
                let projected = match (closed_body, parent.body()) {
                    (SchemaBody::Tuple(_), SchemaBody::Tuple(open)) => {
                        let _ = open.get(index)?;
                        projection_table.tuple_child(schema, index)?
                    }
                    (SchemaBody::Matrix { .. }, SchemaBody::Matrix { .. }) => {
                        projection_table.matrix_element(schema)?
                    }
                    _ => return None,
                };
                projected_schema_shape(projected, &body, fallback_shape_values, projection_schemas)
            });
            let projection_schema = projection.as_ref().map(|(schema, _)| *schema);
            let shape_values = projection.map_or_else(
                || fallback_shape_values.to_vec().into_boxed_slice(),
                |(_, shape)| shape,
            );
            Some(
                if let (Some(context), Some(contexts)) = (source_context, source_contexts) {
                    PatternItem::source_component(
                        projection_schema,
                        projection_schema,
                        body,
                        shape_values,
                        data,
                        selected_source_data,
                        context,
                        contexts,
                    )
                } else {
                    PatternItem::component(projection_schema, body, shape_values, data)
                },
            )
        }
        match self {
            Self::Plain(data) => match data {
                ValueDataDraft::Tuple(items) | ValueDataDraft::Matrix(items) => {
                    items.get(index).cloned().map(Self::new)
                }
                _ => None,
            },
            Self::Dynamic(None) => None,
            Self::Dynamic(Some(_)) | Self::Component { .. } | Self::SourceComponent { .. } => {
                let resolved = resolve_pattern_item(self.clone(), schemas).ok()??;
                selected(
                    resolved.projection_schema,
                    &resolved.body,
                    &resolved.shape_values,
                    &resolved.data,
                    index,
                    schemas,
                    projections,
                    resolved.source_data.as_ref(),
                    resolved.source_context,
                    resolved.source_contexts,
                )
            }
        }
    }

    pub(super) fn middle(
        &self,
        prefix: usize,
        suffix: usize,
        schemas: &mech_core::SchemaTable,
        projections: &StructuralProjectionTable,
    ) -> Option<Self> {
        fn selected(
            projection_schema: Option<SchemaId>,
            closed_body: &SchemaBody,
            fallback_shape_values: &[u64],
            data: &ValueDataDraft,
            prefix: usize,
            suffix: usize,
            schemas: &SchemaTable,
            projections: &StructuralProjectionTable,
            source_data: Option<&ValueData>,
            source_context: Option<Arc<SourceSchemaContext>>,
            source_contexts: Option<Arc<[Arc<SourceSchemaContext>]>>,
        ) -> Option<PatternItem> {
            let (element, items) = match (closed_body, data) {
                (SchemaBody::Matrix { element, .. }, ValueDataDraft::Matrix(items)) => {
                    (element.as_ref().clone(), items)
                }
                _ => return None,
            };
            let end = items.len().checked_sub(suffix)?;
            if prefix > end {
                return None;
            }
            let data = ValueDataDraft::Matrix(items[prefix..end].to_vec().into_boxed_slice());
            // Array-rest semantics produce a row matrix containing the middle
            // slice, including the canonical empty 1-by-0 case.
            let body = SchemaBody::Matrix {
                element: Box::new(element),
                dimensions: vec![
                    mech_core::DimensionExpr::Constant(1),
                    mech_core::DimensionExpr::Constant(u64::try_from(end - prefix).ok()?),
                ]
                .into_boxed_slice(),
            };
            let selected_source_data =
                source_data.and_then(|data| source_data_middle(data, prefix, suffix));
            let (projection_schemas, projection_table) = source_context
                .as_ref()
                .map_or((schemas, projections), |context| {
                    (context.schemas.as_ref(), &context.projections)
                });
            let projection = projection_schema.and_then(|schema| {
                projected_schema_shape(
                    projection_table.matrix_rest(schema)?,
                    &body,
                    fallback_shape_values,
                    projection_schemas,
                )
            });
            let projection_schema = projection.as_ref().map(|(schema, _)| *schema);
            let shape_values = projection.map_or_else(
                || fallback_shape_values.to_vec().into_boxed_slice(),
                |(_, shape)| shape,
            );
            Some(
                if let (Some(context), Some(contexts)) = (source_context, source_contexts) {
                    PatternItem::source_component(
                        projection_schema,
                        projection_schema,
                        body,
                        shape_values,
                        data,
                        selected_source_data,
                        context,
                        contexts,
                    )
                } else {
                    PatternItem::component(projection_schema, body, shape_values, data)
                },
            )
        }

        match self {
            Self::Plain(ValueDataDraft::Matrix(items)) => {
                let end = items.len().checked_sub(suffix)?;
                (prefix <= end).then(|| {
                    Self::new(ValueDataDraft::Matrix(
                        items[prefix..end].to_vec().into_boxed_slice(),
                    ))
                })
            }
            Self::Plain(_) | Self::Dynamic(None) => None,
            Self::Dynamic(Some(_)) | Self::Component { .. } | Self::SourceComponent { .. } => {
                let resolved = resolve_pattern_item(self.clone(), schemas).ok()??;
                selected(
                    resolved.projection_schema,
                    &resolved.body,
                    &resolved.shape_values,
                    &resolved.data,
                    prefix,
                    suffix,
                    schemas,
                    projections,
                    resolved.source_data.as_ref(),
                    resolved.source_context,
                    resolved.source_contexts,
                )
            }
        }
    }

    fn binding_resolution_workspace(
        &self,
        binding_schema: SchemaId,
        source_shape_values: &[u64],
        schemas: &SchemaTable,
    ) -> Result<u64, ResidentKernelError> {
        let binding = schemas
            .get(binding_schema)
            .ok_or(ResidentKernelError::InvalidInput)?;
        let shape_bytes = budget::checked_u64(
            binding
                .dimension_parameters()
                .len()
                .max(source_shape_values.len()),
        )?
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
        // Binding resolution borrows both the plan and source schema arenas.
        // Charge only bodies, witnesses, and parameter buffers constructed by
        // `into_binding`; the retained arenas are not cloned per iteration.
        let mut workspace = binding
            .body()
            .clone_allocation_bound_bytes()
            .and_then(|bytes| bytes.checked_mul(3))
            .and_then(|bytes| bytes.checked_add(shape_bytes.checked_mul(6)?))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let (body, shape_values) = match self {
            Self::Component {
                body, shape_values, ..
            } => (Some(body), shape_values.len()),
            Self::SourceComponent {
                body, shape_values, ..
            } => (Some(body), shape_values.len()),
            Self::Plain(_) | Self::Dynamic(_) => (None, 0),
        };
        if let Some(body) = body {
            workspace = workspace
                .checked_add(
                    body.clone_allocation_bound_bytes()
                        .and_then(|bytes| bytes.checked_mul(2))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )
                .and_then(|bytes| {
                    bytes.checked_add(
                        (shape_values as u64)
                            .checked_mul(core::mem::size_of::<u64>() as u64)?
                            .checked_mul(2)?,
                    )
                })
                .ok_or(ResidentKernelError::InvalidShape)?;
        }
        Ok(workspace)
    }

    pub(super) fn into_binding(
        self,
        binding_schema: SchemaId,
        source_shape_values: &[u64],
        schemas: &mech_core::SchemaTable,
        projections: &StructuralProjectionTable,
    ) -> Result<Option<PatternBindingItem>, ResidentKernelError> {
        let binding = schemas
            .get(binding_schema)
            .ok_or(ResidentKernelError::InvalidInput)?;
        if matches!(binding.body(), SchemaBody::Dynamic) {
            match &self {
                Self::Dynamic(value) => {
                    let binding_shape = dynamic_binding_shape(binding, None, source_shape_values)?;
                    return Ok(Some(PatternBindingItem {
                        shape_values: binding_shape.parameter_values().to_vec().into_boxed_slice(),
                        data: ValueDataDraft::Dynamic(value.clone()),
                        schemas: None,
                        schema_index: None,
                        footprint: BindingFootprint::Selected,
                    }));
                }
                Self::SourceComponent {
                    body: SchemaBody::Dynamic,
                    data,
                    source_data,
                    context,
                    shape_values,
                    ..
                } => {
                    let binding_shape =
                        dynamic_binding_shape(binding, Some(shape_values), source_shape_values)?;
                    let data = if let Some(source_data) = source_data {
                        let validation = SnapshotValidationContext::with_shared_schemas(
                            &context.binding_schemas,
                        )
                        .with_schema_index(&context.binding_schema_index);
                        canonical_snapshot_data_draft_with_context(
                            &SchemaBody::Dynamic,
                            source_data,
                            &validation,
                        )
                        .map_err(|_| ResidentKernelError::InvalidInput)?
                    } else {
                        data.clone()
                    };
                    return Ok(Some(PatternBindingItem {
                        shape_values: binding_shape.parameter_values().to_vec().into_boxed_slice(),
                        data,
                        schemas: Some(Arc::clone(&context.binding_schemas)),
                        schema_index: Some(Arc::clone(&context.binding_schema_index)),
                        footprint: BindingFootprint::Selected,
                    }));
                }
                _ => {}
            }
        }
        let selected_dynamic = matches!(&self, Self::Dynamic(_));
        let item = match self {
            Self::Plain(data) => {
                if matches!(binding.body(), SchemaBody::Dynamic) {
                    return Err(ResidentKernelError::InvalidInput);
                }
                let shape_values = if let (
                    SchemaBody::Matrix { dimensions, .. },
                    ValueDataDraft::Matrix(values),
                ) = (binding.body(), &data)
                    && dimensions.len() == 2
                    && matches!(&dimensions[0], DimensionExpr::Constant(1))
                    && matches!(&dimensions[1], DimensionExpr::Parameter(parameter)
                        if parameter.get() as usize == source_shape_values.len())
                    && binding.dimension_parameters().len() == source_shape_values.len() + 1
                {
                    // Plain native array-rest slices have no projected schema
                    // on the item. The rest declaration appends one inferred
                    // extent after the scrutinee's shape parameters.
                    let mut values_for_shape = source_shape_values.to_vec();
                    values_for_shape.push(budget::checked_u64(values.len())?);
                    binding
                        .instantiate_shape(values_for_shape.into_boxed_slice())
                        .map_err(|_| ResidentKernelError::InvalidInput)?
                        .parameter_values()
                        .to_vec()
                        .into_boxed_slice()
                } else {
                    source_shape_values.to_vec().into_boxed_slice()
                };
                return Ok(Some(PatternBindingItem {
                    shape_values,
                    data,
                    schemas: None,
                    schema_index: None,
                    footprint: BindingFootprint::Selected,
                }));
            }
            item => resolve_pattern_item(item, schemas)?,
        };
        let Some(item) = item else {
            return Ok(None);
        };
        let binding_shape = if matches!(binding.body(), SchemaBody::Dynamic) {
            dynamic_binding_shape(binding, Some(&item.shape_values), source_shape_values)?
        } else {
            let Some(observation) = binding_shape_observation(binding.body(), &item.body) else {
                return Ok(None);
            };
            // A projected child carries the selected parent's dimension
            // witnesses even when its own body does not mention every one.
            let selected_projection_shape = item.projection_schema.is_some()
                && binding.dimension_parameters().len() == item.shape_values.len();
            if let Some(shape) = selected_projection_shape
                .then(|| binding.instantiate_shape(item.shape_values.clone()).ok())
                .flatten()
                .filter(|shape| binding.closed_body(shape).ok().as_ref() == Some(&observation))
            {
                shape
            } else {
                let Ok(shape) = mech_core::shape_for_schema_components(
                    binding,
                    &[(binding.body(), observation)],
                    None,
                ) else {
                    return Ok(None);
                };
                shape
            }
        };
        let target = binding
            .closed_body(&binding_shape)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        let footprint = if matches!(target, SchemaBody::Dynamic) {
            if selected_dynamic {
                BindingFootprint::Selected
            } else {
                BindingFootprint::DynamicWrap {
                    nested_shape_parameters: item.shape_values.len(),
                }
            }
        } else {
            BindingFootprint::Concrete
        };
        let source_schemas = item.schemas.clone();
        let source_schema_index = item
            .source_context
            .as_ref()
            .map(|context| Arc::clone(&context.binding_schema_index));
        let Some(data) = adapt_resolved_pattern_item(item, &target, schemas, projections)? else {
            return Ok(None);
        };
        Ok(Some(PatternBindingItem {
            shape_values: binding_shape.parameter_values().to_vec().into_boxed_slice(),
            data,
            schemas: source_schemas,
            schema_index: source_schema_index,
            footprint,
        }))
    }

    #[cfg(test)]
    pub(super) fn is_atom(&self) -> bool {
        matches!(self.data(), Some(ValueDataDraft::Atom))
    }

    #[cfg(test)]
    pub(super) fn atom_matches(
        &self,
        peer: &mech_core::Value,
        schemas: &mech_core::SchemaTable,
    ) -> bool {
        if !matches!(peer.data(), ValueData::Atom) || !self.is_atom() {
            return false;
        }
        let Some(peer_owner) = peer.schemas() else {
            return false;
        };
        let Some(peer_schema) = peer.validate_against(&peer_owner).ok() else {
            return false;
        };
        match self {
            Self::Plain(_) => true,
            Self::Component { body, .. } => peer_schema
                .closed_body(peer.shape())
                .is_ok_and(|peer| peer == *body),
            Self::SourceComponent { body, .. } => peer_schema
                .closed_body(peer.shape())
                .is_ok_and(|peer| peer == *body),
            Self::Dynamic(Some(value)) => {
                let mut value = value.as_ref();
                loop {
                    match &value.data {
                        ValueDataDraft::Dynamic(Some(next)) => value = next,
                        ValueDataDraft::Dynamic(None) => return false,
                        ValueDataDraft::Atom => {
                            return schemas.entry(value.schema).is_some_and(|entry| {
                                entry.key() == peer.schema_key() && entry.schema() == peer_schema
                            });
                        }
                        _ => return false,
                    }
                }
            }
            Self::Dynamic(None) => false,
        }
    }

    pub(super) fn language_equals(
        &self,
        peer: ResidentValueRef<'_>,

        peer_region: ResidentRegion,
        peer_schema: SchemaId,
        source_shape_values: &[u64],
        canonicalization_work: u64,
        schemas: &std::sync::Arc<mech_core::SchemaTable>,
        projections: &StructuralProjectionTable,
    ) -> Result<Option<bool>, ResidentKernelError> {
        let canonical_budget = SnapshotCanonicalizationBudget::new(canonicalization_work);
        self.language_equals_with_budget(
            peer,
            peer_region,
            peer_schema,
            source_shape_values,
            &canonical_budget,
            schemas,
            projections,
        )
    }

    fn language_equals_with_budget(
        &self,
        peer: ResidentValueRef<'_>,
        peer_region: ResidentRegion,
        peer_schema: SchemaId,
        source_shape_values: &[u64],
        canonical_budget: &SnapshotCanonicalizationBudget,
        schemas: &std::sync::Arc<mech_core::SchemaTable>,
        projections: &StructuralProjectionTable,
    ) -> Result<Option<bool>, ResidentKernelError> {
        if matches!(
            schemas
                .get(peer_schema)
                .ok_or(ResidentKernelError::InvalidInput)?
                .body(),
            SchemaBody::Matrix { .. }
        ) && !matches!(peer, ResidentValueRef::Snapshot(_))
        {
            let Some(PatternBindingItem {
                shape_values, data, ..
            }) = self.clone().into_binding(
                peer_schema,
                source_shape_values,
                schemas,
                projections,
            )?
            else {
                return Ok(Some(false));
            };
            let peer_shape = mech_core::shape_for_resolved_extents(
                schemas
                    .get(peer_schema)
                    .ok_or(ResidentKernelError::InvalidInput)?,
                &[
                    u64::from(peer_region.shape.rows),
                    u64::from(peer_region.shape.columns),
                ],
            )
            .map_err(|_| ResidentKernelError::InvalidInput)?;
            let ValueDataDraft::Matrix(items) = data else {
                return Ok(Some(false));
            };
            if shape_values.as_ref() != peer_shape.parameter_values()
                || items.len() != peer_region.len
            {
                return Ok(Some(false));
            }
            let equal = match peer {
                ResidentValueRef::Bool(values) => {
                    if !values.iter().all(|value| matches!(value, 0 | 1)) {
                        return Err(ResidentKernelError::InvalidInput);
                    }
                    items.iter().enumerate().all(|(ordinal, item)| {
                        let Some(offset) = dense_collection_offset(peer_region, ordinal) else {
                            return false;
                        };
                        matches!((item, values.get(offset)), (ValueDataDraft::Bool(left), Some(right)) if *left == (*right != 0))
                    })
                }
                ResidentValueRef::Index(values) => {
                    items.iter().enumerate().all(|(ordinal, item)| {
                        let Some(offset) = dense_collection_offset(peer_region, ordinal) else {
                            return false;
                        };
                        matches!((item, values.get(offset)), (ValueDataDraft::Index(left), Some(right)) if *left == *right)
                    })
                }
                ResidentValueRef::F64(values) => {
                    items.iter().enumerate().all(|(ordinal, item)| {
                        let Some(offset) = dense_collection_offset(peer_region, ordinal) else {
                            return false;
                        };
                        matches!((item, values.get(offset)), (ValueDataDraft::F64(left), Some(right)) if left.to_f64() == *right)
                    })
                }
                ResidentValueRef::String(values) => {
                    items.iter().enumerate().all(|(ordinal, item)| {
                        let Some(offset) = dense_collection_offset(peer_region, ordinal) else {
                            return false;
                        };
                        matches!((item, values.get(offset)), (ValueDataDraft::String(left), Some(right)) if left == right)
                    })
                }
                ResidentValueRef::Snapshot(_) => unreachable!("snapshot peers use canonical equality"),
            };
            return Ok(Some(equal));
        }

        match peer {
            ResidentValueRef::Bool([value @ (0 | 1)]) => {
                return Ok(Some(
                    matches!(self.scalar(), Some(Item::Bool(left)) if left == (*value != 0)),
                ));
            }
            ResidentValueRef::Index([value]) => {
                return Ok(Some(
                    matches!(self.scalar(), Some(Item::Index(left)) if left == *value),
                ));
            }
            ResidentValueRef::F64([value]) => {
                return Ok(Some(
                    matches!(self.scalar(), Some(Item::F64(left)) if left == *value),
                ));
            }
            ResidentValueRef::String([value]) => {
                return Ok(Some(
                    matches!(self.data(), Some(ValueDataDraft::String(left)) if left == value),
                ));
            }
            ResidentValueRef::Snapshot([Some(peer)]) => {
                let owner = peer.schemas().ok_or(ResidentKernelError::InvalidInput)?;
                let Some(PatternBindingItem {
                    shape_values,
                    data,
                    schemas: source_schemas,
                    schema_index: source_schema_index,
                    ..
                }) = self.clone().into_binding(
                    peer_schema,
                    source_shape_values,
                    schemas,
                    projections,
                )?
                else {
                    return Ok(Some(false));
                };
                let candidate = finalize_pattern_binding(
                    peer_schema,
                    &shape_values,
                    data,
                    source_schemas,
                    source_schema_index,
                    schemas,
                    canonical_budget,
                )?;
                let candidate_schemas = candidate
                    .schemas()
                    .ok_or(ResidentKernelError::InvalidInput)?;
                candidate
                    .language_eq(&candidate_schemas, peer, &owner)
                    .map(Some)
                    .map_err(|_| ResidentKernelError::InvalidInput)
            }
            _ => Ok(None),
        }
    }
}

fn sequence_item(
    sequence: mech_core::snapshot::SequenceView<'_>,
    element_schema: SchemaId,
    element: &SchemaBody,
    shape_values: &[u64],
    index: usize,
    context: &SnapshotValidationContext<'_>,
) -> Option<PatternItem> {
    use mech_core::snapshot::SequenceView;
    macro_rules! draft {
        ($values:expr, $variant:ident) => {
            ValueDataDraft::$variant($values.get(index)?.clone())
        };
    }
    let data = match sequence {
        SequenceView::U8(values) => draft!(values, U8),
        SequenceView::U16(values) => draft!(values, U16),
        SequenceView::U32(values) => draft!(values, U32),
        SequenceView::U64(values) => draft!(values, U64),
        SequenceView::U128(values) => draft!(values, U128),
        SequenceView::I8(values) => draft!(values, I8),
        SequenceView::I16(values) => draft!(values, I16),
        SequenceView::I32(values) => draft!(values, I32),
        SequenceView::I64(values) => draft!(values, I64),
        SequenceView::I128(values) => draft!(values, I128),
        SequenceView::F32(values) => draft!(values, F32),
        SequenceView::F64(values) => draft!(values, F64),
        SequenceView::Complex32(values) => draft!(values, Complex32),
        SequenceView::Complex64(values) => draft!(values, Complex64),
        SequenceView::Rational64(values) => {
            let value = values.get(index)?;
            ValueDataDraft::Rational64 {
                numerator: value.numerator(),
                denominator: value.denominator(),
            }
        }
        SequenceView::Bool(values) => ValueDataDraft::Bool(*values.get(index)?),
        SequenceView::String(values) => {
            ValueDataDraft::String(values.get(index)?.as_ref().to_owned())
        }
        SequenceView::Id(values) => draft!(values, Id),
        SequenceView::Index(values) => draft!(values, Index),
        SequenceView::Values(values) => {
            canonical_snapshot_data_draft_with_context(element, values.get(index)?, context).ok()?
        }
        SequenceView::Unit(count) if (index as u64) < count => ValueDataDraft::Atom,
        _ => return None,
    };
    Some(PatternItem::component(
        Some(element_schema),
        element.clone(),
        shape_values.to_vec().into_boxed_slice(),
        data,
    ))
}

fn collection_item(
    value: ResidentValueRef<'_>,
    region: ResidentRegion,
    element_schema: SchemaId,
    element: &SchemaBody,
    element_shape_values: &[u64],
    ordinal: usize,
    context: &SnapshotValidationContext<'_>,
) -> Option<PatternItem> {
    if let ResidentValueRef::Snapshot([Some(value)]) = value {
        return match value.data() {
            ValueData::Set(set) => set
                .elements()
                .get(ordinal)
                .and_then(|item| {
                    canonical_snapshot_data_draft_with_context(element, item.data(), context).ok()
                })
                .map(|data| {
                    PatternItem::component(
                        Some(element_schema),
                        element.clone(),
                        element_shape_values.to_vec().into_boxed_slice(),
                        data,
                    )
                }),
            ValueData::Matrix(matrix) => sequence_item(
                matrix.elements(),
                element_schema,
                element,
                element_shape_values,
                ordinal,
                context,
            ),
            _ => None,
        };
    }
    // Native matrices are column-major; the canonical collection order is
    // row-major, shared with snapshot-backed matrices.
    let offset = dense_collection_offset(region, ordinal)?;
    let data = match value {
        ResidentValueRef::Bool(value) => match *value.get(offset)? {
            0 => ValueDataDraft::Bool(false),
            1 => ValueDataDraft::Bool(true),
            _ => return None,
        },
        ResidentValueRef::Index(value) => ValueDataDraft::Index(*value.get(offset)?),
        ResidentValueRef::F64(value) => ValueDataDraft::F64(F64Bits::from_f64(*value.get(offset)?)),
        ResidentValueRef::String(value) => ValueDataDraft::String(value.get(offset)?.clone()),
        _ => return None,
    };
    Some(PatternItem::component(
        Some(element_schema),
        element.clone(),
        element_shape_values.to_vec().into_boxed_slice(),
        data,
    ))
}

pub(super) fn dense_collection_offset(region: ResidentRegion, ordinal: usize) -> Option<usize> {
    let columns = region.shape.columns as usize;
    let rows = region.shape.rows as usize;
    if columns == 0 {
        return None;
    }
    (ordinal % columns)
        .checked_mul(rows)?
        .checked_add(ordinal / columns)
}

fn collection_item_footprint(
    value: ResidentValueRef<'_>,
    region: ResidentRegion,
    element: &SchemaBody,
    ordinal: usize,
    meter: &mut ResidentBudgetMeter,
) -> Result<ValueFootprint, ResidentKernelError> {
    if let ResidentValueRef::Snapshot([Some(value)]) = value {
        let mut item_meter = ResidentBudgetMeter::default();
        let footprint = match value.data() {
            ValueData::Set(set) => {
                let item = set
                    .elements()
                    .get(ordinal)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                budget::measure_canonical_data_footprint(&mut item_meter, element, item.data())?
            }
            ValueData::Matrix(matrix) => {
                let mut footprint = ValueFootprint::zero();
                crate::resident::numeric::selected_sequence_footprint(
                    &mut footprint,
                    &mut item_meter,
                    element,
                    matrix.elements(),
                    ordinal,
                )?;
                footprint
            }
            _ => return Err(ResidentKernelError::InvalidInput),
        };
        meter.charge_comparison_work(item_meter.estimate().comparison_work())?;
        return Ok(footprint);
    }
    let footprint = match value {
        ResidentValueRef::Bool(_) => scalar_footprint(1)?,
        ResidentValueRef::Index(_) | ResidentValueRef::F64(_) => scalar_footprint(8)?,
        ResidentValueRef::String(values) => {
            let offset = dense_collection_offset(region, ordinal)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let value = values
                .get(offset)
                .ok_or(ResidentKernelError::InvalidShape)?;
            meter.charge_compute_work(budget::checked_u64(value.len())?)?;
            scalar_footprint(value.len())?
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
    };
    // Fixed-width native lanes need no recursive traversal. String clone
    // traffic and recursive snapshot traversal are charged above.
    Ok(footprint)
}

fn descended_collection_item_footprint(
    value: ResidentValueRef<'_>,
    element: &SchemaBody,
    ordinal: usize,
    path: &[usize],
    meter: &mut ResidentBudgetMeter,
) -> Result<ValueFootprint, ResidentKernelError> {
    use mech_core::snapshot::SequenceView;

    let ResidentValueRef::Snapshot([Some(value)]) = value else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let (mut body, mut data) = match value.data() {
        ValueData::Set(set) => (
            element.clone(),
            set.elements()
                .get(ordinal)
                .ok_or(ResidentKernelError::InvalidShape)?
                .data(),
        ),
        ValueData::Matrix(matrix) => {
            let SequenceView::Values(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            (
                element.clone(),
                values
                    .get(ordinal)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )
        }
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    while matches!(body, SchemaBody::Dynamic) {
        let ValueData::Dynamic(dynamic) = data else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let Some(nested) = dynamic.value() else {
            // The selected value is the absent Dynamic wrapper itself. It is
            // valid input for a Dynamic binding and an ordinary nonmatch for
            // a concrete binding, so measure the wrapper instead of trying to
            // inspect nonexistent concrete data.
            break;
        };
        let owner = nested.schemas().ok_or(ResidentKernelError::InvalidInput)?;
        body = nested
            .validate_against(&owner)
            .map_err(|_| ResidentKernelError::InvalidInput)?
            .closed_body(nested.shape())
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        data = nested.data();
    }
    for (depth, index) in path.iter().enumerate() {
        while matches!(body, SchemaBody::Dynamic) {
            let ValueData::Dynamic(dynamic) = data else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let nested = dynamic.value().ok_or(ResidentKernelError::InvalidInput)?;
            let owner = nested.schemas().ok_or(ResidentKernelError::InvalidInput)?;
            body = nested
                .validate_against(&owner)
                .map_err(|_| ResidentKernelError::InvalidInput)?
                .closed_body(nested.shape())
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            data = nested.data();
        }
        match (&body, data) {
            (SchemaBody::Tuple(fields), ValueData::Tuple(items)) => {
                body = fields
                    .get(*index)
                    .ok_or(ResidentKernelError::InvalidShape)?
                    .clone();
                data = items.get(*index).ok_or(ResidentKernelError::InvalidShape)?;
            }
            (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) => {
                if depth + 1 == path.len() {
                    let mut selected_meter = ResidentBudgetMeter::default();
                    let mut footprint = ValueFootprint::zero();
                    crate::resident::numeric::selected_sequence_footprint(
                        &mut footprint,
                        &mut selected_meter,
                        element,
                        matrix.elements(),
                        *index,
                    )?;
                    meter.charge_comparison_work(selected_meter.estimate().comparison_work())?;
                    return Ok(footprint);
                }
                let SequenceView::Values(items) = matrix.elements() else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                body = element.as_ref().clone();
                data = items.get(*index).ok_or(ResidentKernelError::InvalidShape)?;
            }
            _ => return Err(ResidentKernelError::InvalidInput),
        }
    }
    while matches!(body, SchemaBody::Dynamic) {
        let ValueData::Dynamic(dynamic) = data else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let Some(nested) = dynamic.value() else {
            break;
        };
        let owner = nested.schemas().ok_or(ResidentKernelError::InvalidInput)?;
        body = nested
            .validate_against(&owner)
            .map_err(|_| ResidentKernelError::InvalidInput)?
            .closed_body(nested.shape())
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        data = nested.data();
    }
    let mut selected_meter = ResidentBudgetMeter::default();
    let footprint = budget::measure_canonical_data_footprint(&mut selected_meter, &body, data)?;
    meter.charge_comparison_work(selected_meter.estimate().comparison_work())?;
    Ok(footprint)
}

fn admit_generator_schema_workspace(
    workspace: u64,
    live_bytes: u64,
    live_nodes: u64,
    meter: &mut ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    let temporary_bytes = live_bytes
        .checked_add(workspace)
        .ok_or(ResidentKernelError::InvalidShape)?;
    meter.charge_compute_work(workspace)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes,
            retained_nodes: live_nodes,
            ..meter.estimate()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn schema_shape_resolution_workspace(
    schema: &mech_core::Schema,
    observed_shape_values: usize,
) -> Result<u64, ResidentKernelError> {
    let body_bytes = schema
        .body()
        .clone_allocation_bound_bytes()
        .ok_or(ResidentKernelError::InvalidShape)?;
    let shape_bytes = budget::checked_u64(
        schema
            .dimension_parameters()
            .len()
            .max(observed_shape_values),
    )?
    .checked_mul(core::mem::size_of::<u64>() as u64)
    .ok_or(ResidentKernelError::InvalidShape)?;
    // The selected schema bound includes parameter declarations and their
    // lower/upper expression trees. The remaining terms cover the open
    // and closed body copies plus lower bounds, witnesses, ShapeInstance
    // storage, and the retained parameter-value box.
    schema
        .clone_allocation_bound_bytes()
        .and_then(|bytes| bytes.checked_add(body_bytes.checked_mul(3)?))
        .and_then(|bytes| bytes.checked_add(shape_bytes.checked_mul(6)?))
        .ok_or(ResidentKernelError::InvalidShape)
}

fn generator_shape_values(
    value: ResidentValueRef<'_>,
    schema: SchemaId,
    activation_values: &[u64],
    schemas: &mech_core::SchemaTable,
) -> Result<Box<[u64]>, ResidentKernelError> {
    let ResidentValueRef::Snapshot([Some(value)]) = value else {
        return Ok(activation_values.to_vec().into_boxed_slice());
    };
    let source_schemas = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
    let source_schema = value
        .validate_against(&source_schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let source_body = source_schema
        .closed_body(value.shape())
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let target_schema = schemas
        .get(schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    if schemas
        .entry(schema)
        .is_some_and(|entry| entry.key() == value.schema_key())
    {
        let values = value.shape().parameter_values().to_vec().into_boxed_slice();
        let shape = target_schema
            .instantiate_shape(values)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        if target_schema
            .closed_body(&shape)
            .map_err(|_| ResidentKernelError::InvalidInput)?
            == source_body
        {
            return Ok(shape.parameter_values().to_vec().into_boxed_slice());
        }
    }
    let shape = mech_core::shape_for_schema_components(
        target_schema,
        &[(target_schema.body(), source_body.clone())],
        None,
    )
    .map_err(|_| ResidentKernelError::InvalidInput)?;
    if target_schema
        .closed_body(&shape)
        .map_err(|_| ResidentKernelError::InvalidInput)?
        != source_body
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    Ok(shape.parameter_values().to_vec().into_boxed_slice())
}

fn generator_element(
    source_schema: SchemaId,
    element_schema: SchemaId,
    source_shape_values: &[u64],
    schemas: &mech_core::SchemaTable,
) -> Result<(SchemaBody, Box<[u64]>), ResidentKernelError> {
    let source = schemas
        .get(source_schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let source_shape = source
        .instantiate_shape(source_shape_values.to_vec().into_boxed_slice())
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let closed = source
        .closed_body(&source_shape)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let actual = match closed {
        SchemaBody::Matrix { element, .. } | SchemaBody::Set { element, .. } => *element,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let (_, shape_values) =
        projected_schema_shape(element_schema, &actual, source_shape_values, schemas)
            .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((actual, shape_values))
}

fn retained_item_in(
    value: ResidentValueRef<'_>,
    expected_schema: SchemaId,
    context: &SnapshotValidationContext<'_>,
) -> Option<ValueDataDraft> {
    match value {
        ResidentValueRef::Bool([value @ (0 | 1)]) => Some(ValueDataDraft::Bool(*value != 0)),
        ResidentValueRef::Index([value]) => Some(ValueDataDraft::Index(*value)),
        ResidentValueRef::F64([value]) => Some(ValueDataDraft::F64(F64Bits::from_f64(*value))),
        ResidentValueRef::String([value]) => Some(ValueDataDraft::String(value.clone())),
        ResidentValueRef::Snapshot([Some(value)]) => {
            let schemas = context.schemas();
            let expected = schemas.entry(expected_schema)?;
            if value.schema_key() != expected.key() {
                return None;
            }
            canonical_snapshot_data_draft_with_context(
                expected.schema().body(),
                value.data(),
                context,
            )
            .ok()
        }
        _ => None,
    }
}

pub(super) fn retained_item(value: ResidentValueRef<'_>) -> Option<ValueDataDraft> {
    match value {
        ResidentValueRef::Bool([value @ (0 | 1)]) => Some(ValueDataDraft::Bool(*value != 0)),
        ResidentValueRef::Index([value]) => Some(ValueDataDraft::Index(*value)),
        ResidentValueRef::F64([value]) => Some(ValueDataDraft::F64(F64Bits::from_f64(*value))),
        ResidentValueRef::String([value]) => Some(ValueDataDraft::String(value.clone())),
        ResidentValueRef::Snapshot([Some(value)]) => value.canonical_data_draft().ok(),
        _ => None,
    }
}

pub(super) fn resident_pattern_item(
    value: ResidentValueRef<'_>,
    region: ResidentRegion,
    schema: SchemaId,
    shape_values: &[u64],
    schemas: &mech_core::SchemaTable,
    array: bool,
) -> Option<PatternItem> {
    if let ResidentValueRef::Snapshot([Some(value)]) = value {
        let definition = schemas.entry(schema)?;
        let owner = value.schemas()?;
        let source = value.validate_against(&owner).ok()?;
        if value.schema_key() != definition.key() || source != definition.schema() {
            return None;
        }
        let (context, contexts) = source_schema_contexts(value, schemas)?;
        let source_schema = value.schema();
        let body = context
            .schemas
            .get(source_schema)?
            .closed_body(value.shape())
            .ok()?;
        return Some(PatternItem::source_component(
            Some(source_schema),
            Some(source_schema),
            body,
            value.shape().parameter_values().to_vec().into_boxed_slice(),
            value.canonical_data_draft().ok()?,
            Some(value.data().clone()),
            context,
            contexts,
        ));
    }
    let data = if array {
        let PatternItem::Plain(data) = resident_array_pattern_item(value, region)? else {
            return None;
        };
        data
    } else {
        retained_item(value)?
    };
    let definition = schemas.get(schema)?;
    let shape = definition
        .instantiate_shape(shape_values.to_vec().into_boxed_slice())
        .ok()?;
    let body = definition.closed_body(&shape).ok()?;
    Some(PatternItem::component(
        Some(schema),
        body,
        shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    ))
}

pub(super) fn pattern_dynamic_target_depth(
    pattern: &crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
    schemas: &SchemaTable,
) -> Result<u64, ResidentKernelError> {
    fn body_depth(body: &SchemaBody) -> u64 {
        match body {
            SchemaBody::Dynamic => 1,
            SchemaBody::Option(child)
            | SchemaBody::Matrix { element: child, .. }
            | SchemaBody::Set { element: child, .. } => body_depth(child),
            SchemaBody::Tuple(children) => children.iter().map(body_depth).max().unwrap_or(0),
            SchemaBody::Record(fields)
            | SchemaBody::Table {
                columns: fields, ..
            } => fields
                .iter()
                .map(|field| body_depth(&field.schema))
                .max()
                .unwrap_or(0),
            SchemaBody::Map { key, value, .. } => body_depth(key).max(body_depth(value)),
            SchemaBody::Enum { variants, .. } => variants
                .iter()
                .filter_map(|variant| variant.payload.as_ref())
                .map(body_depth)
                .max()
                .unwrap_or(0),
            _ => 0,
        }
    }
    let target = |schema: SchemaId| {
        schemas
            .get(schema)
            .map(|schema| body_depth(schema.body()))
            .ok_or(ResidentKernelError::InvalidInput)
    };
    match pattern {
        crate::CollectionPattern::Wildcard => Ok(0),
        crate::CollectionPattern::Bind { schema, .. } => target(schema.schema),
        crate::CollectionPattern::Equal(peer) => target(peer.schema),
        crate::CollectionPattern::Enum { payload, .. } => payload
            .as_deref()
            .map_or(Ok(0), |item| pattern_dynamic_target_depth(item, schemas)),
        crate::CollectionPattern::Tuple(items) => items.iter().try_fold(0, |depth, item| {
            Ok(depth.max(pattern_dynamic_target_depth(item, schemas)?))
        }),
        crate::CollectionPattern::Array {
            prefix,
            rest,
            suffix,
        } => prefix
            .iter()
            .chain(rest.iter().map(Box::as_ref))
            .chain(suffix.iter())
            .try_fold(0, |depth, item| {
                Ok(depth.max(pattern_dynamic_target_depth(item, schemas)?))
            }),
    }
}

fn pattern_item_metadata_bound(
    value: &Value,
    meter: &mut ResidentBudgetMeter,
) -> Result<u64, ResidentKernelError> {
    let mut body_bytes = 0;
    let mut shape_parameters = 0;
    visit_value_schema_roots(value, false, meter, &mut |owner, schema_id, _, _| {
        let schema = owner
            .get(schema_id)
            .ok_or(ResidentKernelError::InvalidInput)?;
        body_bytes = body_bytes.max(
            schema
                .body()
                .clone_allocation_bound_bytes()
                .ok_or(ResidentKernelError::InvalidShape)?,
        );
        shape_parameters = shape_parameters.max(schema.dimension_parameters().len());
        Ok(())
    })?;
    // Component bodies are subtrees of a visited source schema. An array-rest
    // projection can introduce one cardinality parameter beyond its parent.
    let shape_bytes = budget::checked_u64(
        shape_parameters
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?
    .checked_mul(core::mem::size_of::<u64>() as u64)
    .ok_or(ResidentKernelError::InvalidShape)?;
    body_bytes
        .checked_add(core::mem::size_of::<SchemaBody>() as u64)
        .and_then(|bytes| bytes.checked_add(shape_bytes))
        .ok_or(ResidentKernelError::InvalidShape)
}

pub(super) fn admit_pattern_item_materialization(
    value: ResidentValueRef<'_>,
    _region: ResidentRegion,
    schema: SchemaId,
    array: bool,
    pattern_work: u64,
    _binding_count: u64,
    equality_count: u64,
    snapshot_finalization_count: u64,
    clone_multiplicity: u64,
    dynamic_target_depth: u64,
    schemas: &mech_core::SchemaTable,
) -> Result<u64, ResidentKernelError> {
    let mut meter = ResidentBudgetMeter::default();
    meter.charge_compute_work(pattern_work)?;
    meter.charge_comparison_work(pattern_work)?;
    let mut canonical_finalization_work = 0;
    let mut canonical_finalization_bytes = 0;
    let mut pattern_metadata_bytes = 0;
    let mut source_context_bound = SourceContextMaterializationBound::default();
    source_context_bound.max_shape_parameters = schemas
        .entries()
        .map(|entry| entry.schema().dimension_parameters().len())
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    meter.charge_comparison_work(budget::checked_u64(schemas.len())?)?;
    let dense_array = array
        && matches!(
            value,
            ResidentValueRef::Bool(_)
                | ResidentValueRef::Index(_)
                | ResidentValueRef::F64(_)
                | ResidentValueRef::String(_)
        );
    let finalization_count = snapshot_finalization_count;
    let footprint = match value {
        ResidentValueRef::Snapshot([Some(value)]) => {
            let expected = schemas
                .entry(schema)
                .ok_or(ResidentKernelError::InvalidInput)?;
            let owner = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
            let source = value
                .validate_against(&owner)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            if value.schema_key() != expected.key() || source != expected.schema() {
                return Err(ResidentKernelError::InvalidInput);
            }
            source_context_bound =
                source_context_materialization_bound(value, schemas, &mut meter)?;
            pattern_metadata_bytes = pattern_item_metadata_bound(value, &mut meter)?;
            let footprint = budget::measure_canonical_value_footprint(&mut meter, value, &owner)?;
            if finalization_count > 0 {
                let mut finalization_meter = ResidentBudgetMeter::default();
                canonical_finalization_work = budget::preflight_canonical_data_finalization(
                    &mut finalization_meter,
                    source.body(),
                    value.data(),
                )?;
                meter.charge_comparison_work(finalization_meter.estimate().comparison_work())?;
                meter.charge_comparison_work(
                    canonical_finalization_work
                        .checked_mul(finalization_count.saturating_sub(1))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )?;
                canonical_finalization_bytes =
                    mech_core::canonical_snapshot_finalization_bytes(CurrentMemoryFootprint {
                        // A binding can only select a subtree of this value.
                        // The complete source footprint therefore bounds the
                        // largest canonical candidate without another walk.
                        logical_elements: footprint.node_count,
                        payload_bytes: footprint.retained_bytes,
                        encoded_bytes: footprint.encoded_bytes,
                        retained_nodes: footprint.node_count,
                        schema_bytes: budget::checked_u64(expected.canonical_bytes().len())?,
                        shape_parameter_count: budget::checked_u64(
                            value.shape().parameter_values().len(),
                        )?,
                        ..CurrentMemoryFootprint::default()
                    })
                    .map_err(|_| ResidentKernelError::InvalidShape)?;
            }
            footprint
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
        ResidentValueRef::Bool(values) if array => ValueFootprint {
            encoded_bytes: budget::checked_u64(values.len())?
                .checked_mul(17)
                .and_then(|bytes| bytes.checked_add(16))
                .ok_or(ResidentKernelError::InvalidShape)?,
            retained_bytes: budget::checked_u64(values.len())?,
            node_count: budget::checked_u64(values.len())?
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?,
        },
        ResidentValueRef::Index(values) if array => {
            let count = budget::checked_u64(values.len())?;
            let retained = count
                .checked_mul(8)
                .ok_or(ResidentKernelError::InvalidShape)?;
            ValueFootprint {
                encoded_bytes: retained
                    .checked_add(
                        count
                            .checked_add(1)
                            .and_then(|nodes| nodes.checked_mul(16))
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                    .ok_or(ResidentKernelError::InvalidShape)?,
                retained_bytes: retained,
                node_count: count
                    .checked_add(1)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            }
        }
        ResidentValueRef::F64(values) if array => {
            let count = budget::checked_u64(values.len())?;
            let retained = count
                .checked_mul(8)
                .ok_or(ResidentKernelError::InvalidShape)?;
            ValueFootprint {
                encoded_bytes: retained
                    .checked_add(
                        count
                            .checked_add(1)
                            .and_then(|nodes| nodes.checked_mul(16))
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                    .ok_or(ResidentKernelError::InvalidShape)?,
                retained_bytes: retained,
                node_count: count
                    .checked_add(1)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            }
        }
        ResidentValueRef::String(values) if array => {
            let retained = values.iter().try_fold(0u64, |bytes, value| {
                bytes
                    .checked_add(budget::checked_u64(value.len())?)
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
            let count = budget::checked_u64(values.len())?;
            ValueFootprint {
                encoded_bytes: retained
                    .checked_add(
                        count
                            .checked_add(1)
                            .and_then(|nodes| nodes.checked_mul(16))
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                    .ok_or(ResidentKernelError::InvalidShape)?,
                retained_bytes: retained,
                node_count: count
                    .checked_add(1)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            }
        }
        ResidentValueRef::Bool([_]) => scalar_footprint(1)?,
        ResidentValueRef::Index([_]) | ResidentValueRef::F64([_]) => scalar_footprint(8)?,
        ResidentValueRef::String([value]) => scalar_footprint(value.len())?,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    if dense_array && finalization_count > 0 {
        // A rest binding or snapshot equality candidate turns the native dense
        // lane into an owned matrix draft and then a packed canonical value.
        // Preflight both the per-element packing loop and the immutable output
        // that overlaps the draft before materializing the first element.
        canonical_finalization_work = footprint.node_count.max(1);
        meter.charge_compute_work(
            canonical_finalization_work
                .checked_mul(finalization_count)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
        let expected = schemas
            .entry(schema)
            .ok_or(ResidentKernelError::InvalidInput)?;
        canonical_finalization_bytes =
            mech_core::canonical_snapshot_finalization_bytes(CurrentMemoryFootprint {
                logical_elements: budget::checked_u64(
                    collection_len(value).ok_or(ResidentKernelError::InvalidInput)?,
                )?,
                payload_bytes: footprint.retained_bytes,
                encoded_bytes: footprint.encoded_bytes,
                retained_nodes: footprint.node_count,
                schema_bytes: budget::checked_u64(expected.canonical_bytes().len())?,
                shape_parameter_count: budget::checked_u64(
                    expected.schema().dimension_parameters().len(),
                )?,
                ..CurrentMemoryFootprint::default()
            })
            .map_err(|_| ResidentKernelError::InvalidShape)?;
    }
    let mut adapted_candidate_nodes = 0;
    if dynamic_target_depth > 0 && finalization_count > 0 {
        // An annotated composite can replace each source component with a
        // Dynamic wrapper. Each source byte can appear in one canonical
        // envelope per Dynamic level, while each node adds an envelope, a
        // nested value, and a draft wrapper. Bound that expansion before
        // materializing the shared scrutinee or any adapted candidate.
        adapted_candidate_nodes = footprint
            .node_count
            .checked_mul(dynamic_target_depth)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let envelope =
            dynamic_canonical_allocation_bound_bytes(0, source_context_bound.max_shape_parameters)
                .ok_or(ResidentKernelError::InvalidShape)?;
        let per_wrapper = envelope
            .checked_add(core::mem::size_of::<ValueDraft>() as u64)
            .and_then(|bytes| bytes.checked_add(core::mem::size_of::<ValueDataDraft>() as u64))
            .and_then(|bytes| bytes.checked_add(core::mem::size_of::<ValueData>() as u64))
            .and_then(|bytes| bytes.checked_add(Value::canonical_owner_allocation_bytes()))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let expansion = footprint
            .encoded_bytes
            .checked_mul(dynamic_target_depth)
            .and_then(|bytes| {
                adapted_candidate_nodes
                    .checked_mul(per_wrapper)
                    .and_then(|wrappers| bytes.checked_add(wrappers))
            })
            .ok_or(ResidentKernelError::InvalidShape)?;
        canonical_finalization_bytes = canonical_finalization_bytes
            .checked_add(expansion)
            .ok_or(ResidentKernelError::InvalidShape)?;
        canonical_finalization_work = canonical_finalization_work
            .checked_add(expansion)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let attempted_expansion = expansion
            .checked_mul(finalization_count)
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.charge_compute_work(attempted_expansion)?;
        meter.charge_comparison_work(attempted_expansion)?;
        meter.charge_cloned_bytes(attempted_expansion)?;
    }
    let equality_work = footprint
        .encoded_bytes
        .max(footprint.node_count)
        .checked_mul(equality_count)
        .ok_or(ResidentKernelError::InvalidShape)?;
    meter.charge_comparison_work(equality_work)?;
    meter.charge_compute_work(source_context_bound.work)?;
    // One owned draft is materialized for the shared scrutinee. Source-backed
    // descent owns both the draft child and its canonical source child at every
    // level while their ancestors remain live. Other lanes own one child per
    // level. `clone_multiplicity` is the maximum structural depth of one arm;
    // separate arms are attempted sequentially.
    let copies = pattern_item_copy_multiplicity(
        matches!(value, ResidentValueRef::Snapshot([Some(_)])),
        clone_multiplicity,
    )?;
    meter.charge_temporary_bytes(
        item_clone_bytes(footprint)?
            .checked_add(pattern_metadata_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?
            .checked_mul(copies)
            .and_then(|bytes| bytes.checked_add(canonical_finalization_bytes))
            .and_then(|bytes| bytes.checked_add(source_context_bound.temporary_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    // A child or rest visit resolves an owned parent. Sibling visits can
    // therefore clone the same wide parent repeatedly even though only the
    // depth-bound copies overlap in memory. Charge copy traffic for every
    // visited pattern node across all attempted arms.
    let clone_visits = pattern_work
        .checked_mul(if matches!(value, ResidentValueRef::Snapshot([Some(_)])) {
            2
        } else {
            1
        })
        .and_then(|visits| visits.checked_add(1))
        .ok_or(ResidentKernelError::InvalidShape)?;
    meter.charge_cloned_bytes(
        footprint
            .retained_bytes
            .checked_add(pattern_metadata_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?
            .checked_mul(clone_visits)
            .and_then(|bytes| bytes.checked_add(source_context_bound.retained_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    meter.charge_retained_nodes(
        footprint
            .node_count
            .checked_mul(copies)
            .and_then(|nodes| nodes.checked_add(source_context_bound.retained_nodes))
            .and_then(|nodes| adapted_candidate_nodes.checked_mul(3)?.checked_add(nodes))
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    PreparedKernel::new((), meter.estimate())
        .admit_control()?
        .into_plan();
    Ok(canonical_finalization_work)
}

fn pattern_item_copy_multiplicity(
    source_backed: bool,
    clone_multiplicity: u64,
) -> Result<u64, ResidentKernelError> {
    clone_multiplicity
        .checked_mul(if source_backed { 2 } else { 1 })
        .and_then(|copies| copies.checked_add(1))
        .ok_or(ResidentKernelError::InvalidShape)
}

pub(super) fn resident_array_pattern_item(
    value: ResidentValueRef<'_>,
    region: ResidentRegion,
) -> Option<PatternItem> {
    if let ResidentValueRef::Snapshot([Some(value)]) = value {
        return value.canonical_data_draft().ok().map(PatternItem::new);
    }
    let count = collection_len(value)?;
    let mut items = Vec::with_capacity(count);
    for ordinal in 0..count {
        let offset = dense_collection_offset(region, ordinal)?;
        items.push(match value {
            ResidentValueRef::Bool(values) => match *values.get(offset)? {
                0 => ValueDataDraft::Bool(false),
                1 => ValueDataDraft::Bool(true),
                _ => return None,
            },
            ResidentValueRef::Index(values) => ValueDataDraft::Index(*values.get(offset)?),
            ResidentValueRef::F64(values) => {
                ValueDataDraft::F64(F64Bits::from_f64(*values.get(offset)?))
            }
            ResidentValueRef::String(values) => ValueDataDraft::String(values.get(offset)?.clone()),
            ResidentValueRef::Snapshot(_) => return None,
        });
    }
    Some(PatternItem::new(ValueDataDraft::Matrix(
        items.into_boxed_slice(),
    )))
}

pub(super) fn pattern_binding_draft(
    schema: SchemaId,
    shape_values: &[u64],
    data: ValueDataDraft,
) -> ValueDraft {
    ValueDraft {
        schema,
        shape_values: shape_values.to_vec().into_boxed_slice(),
        data,
    }
}

fn region(location: ResidentReadLocation) -> ResidentRegion {
    match location {
        ResidentReadLocation::Constant(region)
        | ResidentReadLocation::Input(region)
        | ResidentReadLocation::LexicalInput(region)
        | ResidentReadLocation::Scratch(region)
        | ResidentReadLocation::State { region, .. } => region,
    }
}

fn snapshot_workspace(
    count: usize,
    footprint: ValueFootprint,
    shape_parameter_count: usize,
) -> Result<(u64, u64), ResidentKernelError> {
    let footprint = CurrentMemoryFootprint {
        logical_elements: budget::checked_u64(count)?,
        payload_bytes: footprint.retained_bytes,
        encoded_bytes: footprint.encoded_bytes,
        retained_nodes: footprint.node_count,
        shape_parameter_count: budget::checked_u64(shape_parameter_count)?,
        ..CurrentMemoryFootprint::default()
    };
    let draft = mech_core::canonical_snapshot_draft_bytes(footprint)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let finalization = mech_core::canonical_snapshot_finalization_bytes(footprint)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    Ok((
        draft
            .checked_add(finalization)
            .ok_or(ResidentKernelError::InvalidShape)?,
        finalization,
    ))
}

fn snapshot_draft_bytes(
    count: usize,
    footprint: ValueFootprint,
    shape_parameter_count: usize,
) -> Result<u64, ResidentKernelError> {
    if count == 0 {
        return Ok(0);
    }
    mech_core::canonical_snapshot_draft_bytes(CurrentMemoryFootprint {
        logical_elements: budget::checked_u64(count)?,
        payload_bytes: footprint.retained_bytes,
        encoded_bytes: footprint.encoded_bytes,
        retained_nodes: footprint.node_count,
        shape_parameter_count: budget::checked_u64(shape_parameter_count)?,
        ..CurrentMemoryFootprint::default()
    })
    .map_err(|_| ResidentKernelError::InvalidShape)
}

fn draft_capacity_overlap_bytes(
    count: usize,
    current_capacity: usize,
) -> Result<u64, ResidentKernelError> {
    let slots = if count > current_capacity {
        // `try_reserve_exact` allocates the requested replacement while the
        // old backing array is still live.
        current_capacity
    } else {
        // No growth is needed, but the existing spare capacity remains live.
        current_capacity - count
    };
    budget::checked_u64(slots)?
        .checked_mul(core::mem::size_of::<ValueDataDraft>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)
}

fn comprehension_nested_live_demand(
    count: usize,
    current_capacity: usize,
    footprint: ValueFootprint,
    shape_parameter_count: usize,
    live_context_bytes: u64,
    live_locals: ValueFootprint,
    meter: ResidentBudgetMeter,
) -> Result<(u64, u64), ResidentKernelError> {
    let draft = snapshot_draft_bytes(count, footprint, shape_parameter_count)?
        .checked_add(draft_capacity_overlap_bytes(count, current_capacity)?)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let bytes = meter
        .estimate()
        .temporary_bytes()
        .checked_add(live_context_bytes)
        .and_then(|bytes| bytes.checked_add(draft))
        .and_then(|bytes| bytes.checked_add(live_locals.retained_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(footprint.node_count)
        .and_then(|nodes| nodes.checked_add(live_locals.node_count))
        .and_then(|nodes| nodes.checked_add(u64::from(count > 0)))
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((bytes, nodes))
}

fn unowned_comprehension_capture_footprint(
    captures: &HashMap<ResidentReadLocation, (usize, ValueFootprint)>,
    total: ValueFootprint,
    owned_locations: impl IntoIterator<Item = ResidentReadLocation>,
    seen: &mut [u64],
    generation: &mut u64,
    meter: &mut ResidentBudgetMeter,
) -> Result<ValueFootprint, ResidentKernelError> {
    *generation = generation
        .checked_add(1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let mut owned = ValueFootprint::zero();
    for location in owned_locations {
        meter.charge_compute_work(1)?;
        if let Some(&(index, footprint)) = captures.get(&location)
            && seen[index] != *generation
        {
            seen[index] = *generation;
            owned = owned
                .checked_add(footprint)
                .map_err(|_| ResidentKernelError::InvalidShape)?;
        }
    }
    Ok(ValueFootprint {
        encoded_bytes: total
            .encoded_bytes
            .checked_sub(owned.encoded_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?,
        retained_bytes: total
            .retained_bytes
            .checked_sub(owned.retained_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?,
        node_count: total
            .node_count
            .checked_sub(owned.node_count)
            .ok_or(ResidentKernelError::InvalidShape)?,
    })
}

fn admit_output(
    count: usize,
    current_capacity: usize,
    footprint: ValueFootprint,
    shape_parameter_count: usize,
    schema_arena_bytes: u64,
    live_locals: ValueFootprint,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    let (temporary, output) = snapshot_workspace(count, footprint, shape_parameter_count)?;
    let temporary = temporary
        .checked_add(draft_capacity_overlap_bytes(count, current_capacity)?)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(live_locals.retained_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(schema_arena_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output = output
        .checked_add(schema_arena_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let overlapping_nodes = footprint
        .node_count
        .checked_mul(2)
        .and_then(|nodes| nodes.checked_add(2))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(overlapping_nodes)
        .and_then(|nodes| nodes.checked_add(live_locals.node_count))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            output_elements: count,
            output_bytes: output,
            temporary_bytes: temporary,
            retained_nodes,
            ..meter.estimate()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn admit_draft_shrink(
    count: usize,
    current_capacity: usize,
    footprint: ValueFootprint,
    shape_parameter_count: usize,
    schema_arena_bytes: u64,
    live_locals: ValueFootprint,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    if count == current_capacity {
        return Ok(());
    }
    // `Vec::into_boxed_slice` may allocate the exact-length slice before it
    // releases the oversized Vec backing store. Admit that overlap while the
    // original draft and every lexical payload are still live.
    let old_backing = budget::checked_u64(current_capacity)?
        .checked_mul(core::mem::size_of::<ValueDataDraft>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let temporary = snapshot_draft_bytes(count, footprint, shape_parameter_count)?
        .checked_add(old_backing)
        .and_then(|bytes| bytes.checked_add(live_locals.retained_bytes))
        .and_then(|bytes| bytes.checked_add(schema_arena_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(footprint.node_count)
        .and_then(|nodes| nodes.checked_add(live_locals.node_count))
        .and_then(|nodes| nodes.checked_add(u64::from(count > 0)))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes: temporary,
            retained_nodes,
            ..meter.estimate()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn admit_set_draft(
    count: usize,
    current_capacity: usize,
    footprint: ValueFootprint,
    shape_parameter_count: usize,
    schema_arena_bytes: u64,
    live_locals: ValueFootprint,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    let temporary = snapshot_draft_bytes(count, footprint, shape_parameter_count)?
        .checked_add(draft_capacity_overlap_bytes(count, current_capacity)?)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(schema_arena_bytes)
        .and_then(|bytes| bytes.checked_add(live_locals.retained_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(footprint.node_count)
        .and_then(|nodes| nodes.checked_add(live_locals.node_count))
        .and_then(|nodes| nodes.checked_add(u64::from(count > 0)))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes: temporary,
            retained_nodes,
            ..meter.estimate()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn scalar_footprint(bytes: usize) -> Result<ValueFootprint, ResidentKernelError> {
    Ok(ValueFootprint {
        encoded_bytes: budget::checked_u64(bytes)?.saturating_add(16),
        retained_bytes: budget::checked_u64(bytes)?,
        node_count: 1,
    })
}

fn retained_value_footprint(
    value: ResidentValueRef<'_>,
    expected_schema: SchemaId,
    schemas: &mech_core::SchemaTable,
    meter: &mut ResidentBudgetMeter,
) -> Result<(ValueFootprint, u64), ResidentKernelError> {
    let footprint = match value {
        ResidentValueRef::Snapshot([Some(value)]) => {
            let expected = schemas
                .entry(expected_schema)
                .ok_or(ResidentKernelError::InvalidInput)?;
            let owner = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
            let source = value
                .validate_against(&owner)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            if value.schema_key() != expected.key() || source != expected.schema() {
                return Err(ResidentKernelError::InvalidInput);
            }
            let mut item_meter = ResidentBudgetMeter::default();
            // Only the canonical data is retained as one element of the
            // comprehension result. The source Value wrapper, root, shape,
            // and schema owner remain borrowed from the yielding port.
            let footprint = budget::measure_canonical_data_footprint(
                &mut item_meter,
                source.body(),
                value.data(),
            )?;
            // Rebuilding the retained draft recursively canonicalizes nested
            // Set/Map values. Reserve that exact work while the source is
            // still borrowed, before cloning any part of its data tree.
            let finalization_work = budget::preflight_canonical_data_finalization(
                &mut item_meter,
                source.body(),
                value.data(),
            )?;
            meter.charge_comparison_work(item_meter.estimate().comparison_work())?;
            return Ok((footprint, finalization_work));
        }
        ResidentValueRef::Bool([_]) => scalar_footprint(1)?,
        ResidentValueRef::Index([_]) | ResidentValueRef::F64([_]) => scalar_footprint(8)?,
        ResidentValueRef::String([value]) => {
            meter.charge_compute_work(budget::checked_u64(value.len())?)?;
            scalar_footprint(value.len())?
        }
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    // Native scalar yields need no recursive traversal beyond the control
    // work charged by the caller.
    Ok((footprint, 0))
}

fn admit_item_clone(
    item: ValueFootprint,
    metadata_bytes: u64,
    retained_count: usize,
    retained: ValueFootprint,
    retained_shape_parameter_count: usize,
    schema_arena_bytes: u64,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    admit_item_clones(
        item,
        metadata_bytes,
        1,
        retained_count,
        retained,
        retained_shape_parameter_count,
        schema_arena_bytes,
        meter,
    )
}

fn admit_item_clones(
    item: ValueFootprint,
    metadata_bytes: u64,
    copies: u64,
    retained_count: usize,
    retained: ValueFootprint,
    retained_shape_parameter_count: usize,
    schema_arena_bytes: u64,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    let item_temporary = item_clone_bytes(item)?
        .checked_add(metadata_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_mul(copies)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let temporary = snapshot_draft_bytes(retained_count, retained, retained_shape_parameter_count)?
        .checked_add(item_temporary)
        .and_then(|bytes| bytes.checked_add(schema_arena_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let cloned_nodes = item
        .node_count
        .checked_mul(copies)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(retained.node_count)
        .and_then(|nodes| nodes.checked_add(cloned_nodes))
        .and_then(|nodes| nodes.checked_add(u64::from(retained_count > 0)))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes: temporary,
            cloned_bytes: item.retained_bytes
                .checked_add(metadata_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?
                .checked_mul(copies)
                .ok_or(ResidentKernelError::InvalidShape)?,
            retained_nodes,
            ..meter.estimate()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn item_clone_bytes(item: ValueFootprint) -> Result<u64, ResidentKernelError> {
    item.node_count
        .saturating_sub(1)
        .checked_mul(core::mem::size_of::<ValueDataDraft>() as u64)
        .and_then(|bytes| bytes.checked_add(item.retained_bytes))
        .ok_or(ResidentKernelError::InvalidShape)
}

fn dynamic_wrapped_footprint(
    item: ValueFootprint,
    nested_shape_parameters: usize,
) -> Result<ValueFootprint, ResidentKernelError> {
    let nested = budget::projected_canonical_value_footprint(item, nested_shape_parameters)?;
    let canonical =
        dynamic_canonical_allocation_bound_bytes(item.encoded_bytes, nested_shape_parameters)
            .ok_or(ResidentKernelError::InvalidShape)?;
    ValueFootprint {
        encoded_bytes: canonical,
        retained_bytes: core::mem::size_of::<ValueData>() as u64,
        node_count: 1,
    }
    .checked_add(ValueFootprint {
        encoded_bytes: 0,
        retained_bytes: canonical,
        node_count: 0,
    })
    .and_then(|footprint| footprint.checked_add(nested))
    .map_err(|_| ResidentKernelError::InvalidShape)
}

fn admit_pattern_binding_finalization(
    schema: SchemaId,
    shape_values: &[u64],
    item: ValueFootprint,
    live_item_copies: u64,
    previous_binding: ValueFootprint,
    other_live_locals: ValueFootprint,
    retained_count: usize,
    retained: ValueFootprint,
    retained_shape_parameter_count: usize,
    schema_arena_bytes: u64,
    schemas: &mech_core::SchemaTable,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    let entry = schemas
        .entry(schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let candidate = budget::projected_canonical_value_footprint(item, shape_values.len())?;
    let finalization = mech_core::canonical_snapshot_finalization_bytes(CurrentMemoryFootprint {
        logical_elements: item.node_count,
        payload_bytes: candidate.retained_bytes,
        encoded_bytes: candidate.encoded_bytes,
        retained_nodes: candidate.node_count,
        schema_bytes: budget::checked_u64(entry.canonical_bytes().len())?,
        shape_parameter_count: budget::checked_u64(shape_values.len())?,
        ..CurrentMemoryFootprint::default()
    })
    .map_err(|_| ResidentKernelError::InvalidShape)?;
    let temporary = snapshot_draft_bytes(retained_count, retained, retained_shape_parameter_count)?
        .checked_add(
            item_clone_bytes(item)?
                .checked_mul(live_item_copies)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )
        .and_then(|bytes| bytes.checked_add(finalization))
        .and_then(|bytes| bytes.checked_add(previous_binding.retained_bytes))
        .and_then(|bytes| bytes.checked_add(other_live_locals.retained_bytes))
        .and_then(|bytes| bytes.checked_add(schema_arena_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let live_item_nodes = item
        .node_count
        .checked_mul(live_item_copies)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(retained.node_count)
        .and_then(|nodes| nodes.checked_add(live_item_nodes))
        .and_then(|nodes| nodes.checked_add(previous_binding.node_count))
        .and_then(|nodes| nodes.checked_add(other_live_locals.node_count))
        .and_then(|nodes| nodes.checked_add(candidate.node_count))
        .and_then(|nodes| nodes.checked_add(u64::from(retained_count > 0)))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes: temporary,
            retained_nodes,
            ..meter.estimate()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn visit_value_schema_roots(
    value: &Value,
    nested_dynamic: bool,
    meter: &mut ResidentBudgetMeter,
    visit: &mut impl FnMut(
        &Arc<mech_core::SchemaTable>,
        SchemaId,
        bool,
        &mut ResidentBudgetMeter,
    ) -> Result<(), ResidentKernelError>,
) -> Result<(), ResidentKernelError> {
    fn visit_sequence(
        sequence: mech_core::snapshot::SequenceView<'_>,
        meter: &mut ResidentBudgetMeter,
        visit: &mut impl FnMut(
            &Arc<mech_core::SchemaTable>,
            SchemaId,
            bool,
            &mut ResidentBudgetMeter,
        ) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        if let mech_core::snapshot::SequenceView::Values(values) = sequence {
            for value in values {
                visit_data_schema_owners(value, meter, visit)?;
            }
        }
        Ok(())
    }

    fn visit_data_schema_owners(
        data: &ValueData,
        meter: &mut ResidentBudgetMeter,
        visit: &mut impl FnMut(
            &Arc<mech_core::SchemaTable>,
            SchemaId,
            bool,
            &mut ResidentBudgetMeter,
        ) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        // Charge each recursive node before inspecting or descending into it.
        meter.charge_compute_work(1)?;
        match data {
            ValueData::Dynamic(dynamic) => {
                if let Some(value) = dynamic.value() {
                    visit_value_schema_roots(value, true, meter, visit)?;
                }
            }
            ValueData::Enum(value) => {
                if let Some(payload) = value.payload() {
                    visit_data_schema_owners(payload, meter, visit)?;
                }
            }
            ValueData::Option(Some(value)) => visit_data_schema_owners(value, meter, visit)?,
            ValueData::Tuple(values) => {
                for value in values {
                    visit_data_schema_owners(value, meter, visit)?;
                }
            }
            ValueData::Record(value) => {
                for field in value.fields() {
                    visit_data_schema_owners(field, meter, visit)?;
                }
            }
            ValueData::Matrix(value) => visit_sequence(value.elements(), meter, visit)?,
            ValueData::Table(value) => {
                for index in 0..value.len() {
                    let column = value
                        .column(index)
                        .ok_or(ResidentKernelError::InvalidInput)?;
                    visit_sequence(column, meter, visit)?;
                }
            }
            ValueData::Set(value) => {
                for element in value.elements() {
                    visit_data_schema_owners(element.data(), meter, visit)?;
                }
            }
            ValueData::Map(value) => {
                for entry in value.entries() {
                    visit_data_schema_owners(entry.key().data(), meter, visit)?;
                    visit_data_schema_owners(entry.value(), meter, visit)?;
                }
            }
            ValueData::Option(None)
            | ValueData::U8(_)
            | ValueData::U16(_)
            | ValueData::U32(_)
            | ValueData::U64(_)
            | ValueData::U128(_)
            | ValueData::I8(_)
            | ValueData::I16(_)
            | ValueData::I32(_)
            | ValueData::I64(_)
            | ValueData::I128(_)
            | ValueData::F32(_)
            | ValueData::F64(_)
            | ValueData::Complex32(_)
            | ValueData::Complex64(_)
            | ValueData::Rational64(_)
            | ValueData::Bool(_)
            | ValueData::String(_)
            | ValueData::Id(_)
            | ValueData::Index(_)
            | ValueData::Atom
            | ValueData::Type(_) => {}
        }
        Ok(())
    }

    meter.charge_compute_work(1)?;
    let owner = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
    visit(&owner, value.schema(), nested_dynamic, meter)?;
    visit_data_schema_owners(value.data(), meter, visit)
}

fn visit_value_schema_owners(
    value: &Value,
    meter: &mut ResidentBudgetMeter,
    visit: &mut impl FnMut(
        &Arc<mech_core::SchemaTable>,
        &mut ResidentBudgetMeter,
    ) -> Result<(), ResidentKernelError>,
) -> Result<(), ResidentKernelError> {
    visit_value_schema_roots(value, false, meter, &mut |owner, _, _, meter| {
        visit(owner, meter)
    })
}

struct SchemaOwnerRoots {
    owner: Arc<mech_core::SchemaTable>,
    root_start: usize,
    root_len: usize,
    root_capacity: usize,
    closure_capacity: usize,
}

impl SchemaOwnerRoots {
    fn roots<'a>(&self, storage: &'a [SchemaId]) -> &'a [SchemaId] {
        &storage[self.root_start..self.root_start + self.root_len]
    }

    fn insert_root(
        &mut self,
        storage: &mut [SchemaId],
        root: SchemaId,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<(), ResidentKernelError> {
        let end = self
            .root_start
            .checked_add(self.root_capacity)
            .ok_or(ResidentKernelError::InvalidShape)?;
        if end > storage.len() {
            return Err(ResidentKernelError::InvalidShape);
        }
        for candidate in &storage[self.root_start..self.root_start + self.root_len] {
            meter.charge_comparison_work(1)?;
            if *candidate == root {
                return Ok(());
            }
        }
        if self.root_len == self.root_capacity {
            return Err(ResidentKernelError::InvalidShape);
        }
        storage[self.root_start + self.root_len] = root;
        self.root_len += 1;
        Ok(())
    }
}

fn distinct_schema_owner_footprint(
    owner: &Arc<mech_core::SchemaTable>,
    plan: &Arc<mech_core::SchemaTable>,
    shared_owner_bytes: u64,
) -> Result<(u64, u64), ResidentKernelError> {
    if Arc::ptr_eq(owner, plan) {
        return Ok((0, 0));
    }
    Ok((
        owner
            .clone_allocation_bound_bytes()
            .and_then(|bytes| bytes.checked_add(shared_owner_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?,
        budget::checked_u64(owner.len())?,
    ))
}

fn same_schema_arena_contents(
    owner: &Arc<mech_core::SchemaTable>,
    plan: &Arc<mech_core::SchemaTable>,
    meter: &mut ResidentBudgetMeter,
) -> Result<bool, ResidentKernelError> {
    if Arc::ptr_eq(owner, plan) {
        return Ok(true);
    }
    if owner.len() != plan.len() {
        return Ok(false);
    }
    for (left, right) in owner.entries().zip(plan.entries()) {
        meter.charge_comparison_work(1)?;
        if left.key() != right.key() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn component_closure_is_addressable(
    owner: &mech_core::SchemaTable,
    root: SchemaId,
    plan: &mech_core::SchemaTable,
    meter: &mut ResidentBudgetMeter,
) -> Result<bool, ResidentKernelError> {
    fn contains_schema(
        plan: &mech_core::SchemaTable,
        key: mech_core::SchemaKey,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<bool, ResidentKernelError> {
        for entry in plan.entries() {
            meter.charge_comparison_work(1)?;
            if entry.key() == key {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn visit_body(
        parent: &mech_core::Schema,
        body: &SchemaBody,
        plan: &mech_core::SchemaTable,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<bool, ResidentKernelError> {
        meter.charge_compute_work(1)?;
        let mut visit_child = |child: &SchemaBody| {
            let schema = parent
                .canonical_component_schema(child)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            if !contains_schema(plan, schema.key(), meter)? {
                return Ok(false);
            }
            visit_body(parent, child, plan, meter)
        };
        match body {
            SchemaBody::Enum { variants, .. } => {
                for child in variants
                    .iter()
                    .filter_map(|variant| variant.payload.as_ref())
                {
                    if !visit_child(child)? {
                        return Ok(false);
                    }
                }
            }
            SchemaBody::Option(child)
            | SchemaBody::Matrix { element: child, .. }
            | SchemaBody::Set { element: child, .. } => {
                if !visit_child(child)? {
                    return Ok(false);
                }
            }
            SchemaBody::Tuple(children) => {
                for child in children {
                    if !visit_child(child)? {
                        return Ok(false);
                    }
                }
            }
            SchemaBody::Record(fields)
            | SchemaBody::Table {
                columns: fields, ..
            } => {
                for field in fields {
                    if !visit_child(&field.schema)? {
                        return Ok(false);
                    }
                }
            }
            SchemaBody::Map { key, value, .. } => {
                if !visit_child(key)? || !visit_child(value)? {
                    return Ok(false);
                }
            }
            SchemaBody::Dynamic
            | SchemaBody::Bool
            | SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::IntegerInterval(_)
            | SchemaBody::FloatingPoint(_)
            | SchemaBody::Complex(_)
            | SchemaBody::Rational64
            | SchemaBody::String
            | SchemaBody::Id
            | SchemaBody::Index
            | SchemaBody::Atom(_)
            | SchemaBody::ReifiedType => {}
        }
        Ok(true)
    }

    let root = owner.get(root).ok_or(ResidentKernelError::InvalidInput)?;
    if !contains_schema(plan, root.key(), meter)? {
        return Ok(false);
    }
    visit_body(root, root.body(), plan, meter)
}

fn schema_root_requires_import(
    owner: &Arc<mech_core::SchemaTable>,
    root: SchemaId,
    nested_dynamic: bool,
    plan: &Arc<mech_core::SchemaTable>,
    meter: &mut ResidentBudgetMeter,
    live_index_bytes: u64,
) -> Result<bool, ResidentKernelError> {
    if same_schema_arena_contents(owner, plan, meter)? && !nested_dynamic {
        return Ok(false);
    }
    let remaining = meter.estimate().remaining_incremental_work()?;
    let closure_budget = SnapshotCanonicalizationBudget::new(remaining);
    let (_, construction_bytes, closure_nodes) = owner
        .component_closure_bounds_for_roots_with_budget(&[root], &closure_budget)
        .ok_or(ResidentKernelError::InvalidShape)?;
    meter.charge_comparison_work(closure_budget.consumed())?;
    let retained_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(closure_nodes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes: construction_bytes
                .checked_add(live_index_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            retained_nodes,
            ..meter.estimate()
        },
    )
    .admit()?
    .into_plan();
    Ok(!component_closure_is_addressable(owner, root, plan, meter)?)
}

fn collection_canonicalization_work(
    kind: crate::ComprehensionKind,
    count: usize,
) -> Result<u64, ResidentKernelError> {
    if kind != crate::ComprehensionKind::Set {
        return Ok(0);
    }
    (count as u64)
        .checked_mul((count.max(1).ilog2() as u64 + 1) * 64)
        .and_then(|work| work.checked_add(64))
        .ok_or(ResidentKernelError::InvalidShape)
}

fn measure_owned_canonical_value_footprint(
    meter: &mut ResidentBudgetMeter,
    value: &mech_core::snapshot::Value,
    fallback_schemas: &mech_core::SchemaTable,
) -> Result<ValueFootprint, ResidentKernelError> {
    // Schema ids are arena-local. Snapshot-backed pattern bindings may retain
    // the arena of an imported value, so measure them against their owner
    // rather than the comprehension plan's arena.
    let owner = value.schemas();
    budget::measure_canonical_value_footprint(
        meter,
        value,
        owner.as_deref().unwrap_or(fallback_schemas),
    )
}

impl ReactiveInstance {
    fn resident_value_footprint(
        value: ResidentValueRef<'_>,
        schemas: &mech_core::SchemaTable,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<ValueFootprint, ResidentKernelError> {
        let mut footprint = ValueFootprint::zero();
        match value {
            ResidentValueRef::String(values) => {
                for value in values {
                    meter.charge_compute_work(1)?;
                    footprint = footprint
                        .checked_add(ValueFootprint {
                            encoded_bytes: 0,
                            retained_bytes: budget::checked_u64(value.capacity())?,
                            node_count: u64::from(!value.is_empty()),
                        })
                        .map_err(|_| ResidentKernelError::InvalidShape)?;
                }
            }
            ResidentValueRef::Snapshot(values) => {
                for value in values.iter().flatten() {
                    let mut local_meter = ResidentBudgetMeter::default();
                    let local =
                        measure_owned_canonical_value_footprint(&mut local_meter, value, schemas)?;
                    meter.charge_comparison_work(local_meter.estimate().comparison_work())?;
                    footprint = footprint
                        .checked_add(local)
                        .map_err(|_| ResidentKernelError::InvalidShape)?;
                }
            }
            ResidentValueRef::Bool(_) | ResidentValueRef::Index(_) | ResidentValueRef::F64(_) => {}
        }
        Ok(footprint)
    }

    pub(super) fn resident_local_footprint(
        &self,
        locals: impl IntoIterator<Item = ResidentRegion>,
        schemas: &mech_core::SchemaTable,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<ValueFootprint, ResidentKernelError> {
        let mut footprint = ValueFootprint::zero();
        for local in locals {
            footprint = footprint
                .checked_add(Self::resident_value_footprint(
                    self.workspace.scratch.read(local),
                    schemas,
                    meter,
                )?)
                .map_err(|_| ResidentKernelError::InvalidShape)?;
        }
        Ok(footprint)
    }

    fn comprehension_live_local_footprint(
        &self,
        locals: &[ResidentRegion],
        excluded: Option<ResidentRegion>,
        schemas: &mech_core::SchemaTable,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<ValueFootprint, ResidentKernelError> {
        self.resident_local_footprint(
            locals
                .iter()
                .copied()
                .filter(|local| Some(*local) != excluded),
            schemas,
            meter,
        )
    }

    fn incremental_comprehension_live_local_footprint(
        &self,
        locals: &[ResidentRegion],
        retained_local_count: u32,
        excluded_locals: &[u32],
        nested_match: bool,
        live: &mut ComprehensionLiveLocalFootprint,
        schemas: &mech_core::SchemaTable,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<ValueFootprint, ResidentKernelError> {
        let retained =
            usize::try_from(retained_local_count).map_err(|_| ResidentKernelError::InvalidShape)?;
        // A match may replace its prior output. Measure that slot for this
        // call, but keep it out of the cached prefix until the next step.
        let stable_end = retained
            .checked_sub(usize::from(nested_match))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let added = locals
            .get(live.retained_prefix..stable_end)
            .ok_or(ResidentKernelError::InvalidShape)?;
        live.footprint = live
            .footprint
            .checked_add(self.resident_local_footprint(added.iter().copied(), schemas, meter)?)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        live.retained_prefix = stable_end;
        let mut footprint = live.footprint;
        if nested_match {
            let prior_output = *locals
                .get(stable_end)
                .ok_or(ResidentKernelError::InvalidShape)?;
            footprint = footprint
                .checked_add(self.resident_local_footprint([prior_output], schemas, meter)?)
                .map_err(|_| ResidentKernelError::InvalidShape)?;
        }
        let mut excluded = ValueFootprint::zero();
        for index in excluded_locals.iter().copied() {
            let index = usize::try_from(index).map_err(|_| ResidentKernelError::InvalidShape)?;
            if index >= retained {
                return Err(ResidentKernelError::InvalidShape);
            }
            let region = *locals.get(index).ok_or(ResidentKernelError::InvalidShape)?;
            excluded = excluded
                .checked_add(self.resident_local_footprint([region], schemas, meter)?)
                .map_err(|_| ResidentKernelError::InvalidShape)?;
        }
        Ok(ValueFootprint {
            encoded_bytes: footprint
                .encoded_bytes
                .checked_sub(excluded.encoded_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            retained_bytes: footprint
                .retained_bytes
                .checked_sub(excluded.retained_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            node_count: footprint
                .node_count
                .checked_sub(excluded.node_count)
                .ok_or(ResidentKernelError::InvalidShape)?,
        })
    }

    fn comprehension_schema_arena(
        &self,
        control: &ActivatedComprehensionNode,
        working: InstanceEpoch,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<(Arc<mech_core::SchemaTable>, StructuralProjectionTable, u64), ResidentKernelError>
    {
        let plan_entries = budget::checked_u64(self.plan.schemas.len())?;
        let key_bytes = core::mem::size_of::<mech_core::SchemaKey>() as u64;
        let shared_owner_bytes = (core::mem::size_of::<usize>() as u64)
            .checked_mul(2)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let previous_output = if control.write.storage == ResidentStorageClass::Constant {
            self.activation.read(control.write.region)
        } else {
            self.workspace.scratch.read(control.write.region)
        };
        let previous_owner = match previous_output {
            ResidentValueRef::Snapshot([Some(previous)]) => Some(
                previous
                    .schemas()
                    .ok_or(ResidentKernelError::InvalidInput)?,
            ),
            _ => None,
        };
        let mut root_occurrences = 0_u64;
        let captured_reads = || {
            self.plan.reads[control.reads.start as usize..control.reads.end as usize]
                .iter()
                .copied()
                .chain(control.schema_reads.iter().copied())
        };
        for location in captured_reads() {
            let Some(ResidentValueRef::Snapshot(values)) = self.read_location(location, working)
            else {
                continue;
            };
            for value in values.iter().flatten() {
                visit_value_schema_roots(
                    value,
                    false,
                    meter,
                    &mut |owner, root, nested_dynamic, meter| {
                        if schema_root_requires_import(
                            owner,
                            root,
                            nested_dynamic,
                            &self.plan.schemas,
                            meter,
                            0,
                        )? {
                            root_occurrences = root_occurrences
                                .checked_add(1)
                                .ok_or(ResidentKernelError::InvalidShape)?;
                        }
                        Ok(())
                    },
                )?;
            }
        }
        if root_occurrences == 0 {
            if let Some(owner) = &previous_owner {
                let (bytes, nodes) =
                    distinct_schema_owner_footprint(owner, &self.plan.schemas, shared_owner_bytes)?;
                meter.charge_temporary_bytes(bytes)?;
                meter.charge_retained_nodes(nodes)?;
            }
            return Ok((
                Arc::clone(&self.plan.schemas),
                self.plan.structural_projections.clone(),
                0,
            ));
        }

        let minimum_owner_index_bytes = root_occurrences
            .checked_mul(
                (core::mem::size_of::<SchemaOwnerRoots>() + core::mem::size_of::<SchemaId>())
                    as u64,
            )
            .ok_or(ResidentKernelError::InvalidShape)?;
        // This is the only growable scan state. Admit its full occurrence
        // bound before reserving it; the completed index then retains only
        // distinct Arc identities.
        PreparedKernel::new(
            (),
            budget::resident_cost! {
                temporary_bytes: minimum_owner_index_bytes,
                ..meter.estimate()
            },
        )
        .admit()?
        .into_plan();
        let owner_capacity =
            usize::try_from(root_occurrences).map_err(|_| ResidentKernelError::InvalidShape)?;
        let mut owners = Vec::<SchemaOwnerRoots>::new();
        owners
            .try_reserve_exact(owner_capacity)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        let owner_buffer_bytes = allocation_capacity_bytes::<SchemaOwnerRoots>(owners.capacity())?;
        let minimum_root_buffer_bytes = root_occurrences
            .checked_mul(core::mem::size_of::<SchemaId>() as u64)
            .ok_or(ResidentKernelError::InvalidShape)?;
        PreparedKernel::new(
            (),
            budget::resident_cost! {
                temporary_bytes: owner_buffer_bytes
                    .checked_add(minimum_root_buffer_bytes)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                ..meter.estimate()
            },
        )
        .admit()?
        .into_plan();
        // Count every relevant root per owner before allocating root storage.
        // The subsequent fixed segments never grow, so no replacement buffer
        // can overlap the prior allocation during discovery.
        for location in captured_reads() {
            let Some(ResidentValueRef::Snapshot(values)) = self.read_location(location, working)
            else {
                continue;
            };
            for value in values.iter().flatten() {
                visit_value_schema_roots(
                    value,
                    false,
                    meter,
                    &mut |owner, root, nested_dynamic, meter| {
                        if !schema_root_requires_import(
                            owner,
                            root,
                            nested_dynamic,
                            &self.plan.schemas,
                            meter,
                            owner_buffer_bytes,
                        )? {
                            return Ok(());
                        }
                        let mut owner_index = None;
                        for (index, existing) in owners.iter().enumerate() {
                            meter.charge_comparison_work(1)?;
                            if Arc::ptr_eq(&existing.owner, owner) {
                                owner_index = Some(index);
                                break;
                            }
                        }
                        let owner_index = match owner_index {
                            Some(index) => index,
                            None => {
                                owners.push(SchemaOwnerRoots {
                                    owner: Arc::clone(owner),
                                    root_start: 0,
                                    root_len: 0,
                                    root_capacity: 0,
                                    closure_capacity: 0,
                                });
                                owners.len() - 1
                            }
                        };
                        owners[owner_index].root_capacity = owners[owner_index]
                            .root_capacity
                            .checked_add(1)
                            .ok_or(ResidentKernelError::InvalidShape)?;
                        Ok(())
                    },
                )?;
            }
        }

        let mut root_storage_len = 0_usize;
        for owner in &mut owners {
            owner.root_start = root_storage_len;
            root_storage_len = root_storage_len
                .checked_add(owner.root_capacity)
                .ok_or(ResidentKernelError::InvalidShape)?;
        }
        if root_storage_len != owner_capacity {
            return Err(ResidentKernelError::InvalidShape);
        }
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(root_storage_len)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        let root_buffer_bytes = allocation_capacity_bytes::<SchemaId>(roots.capacity())?;
        let owner_index_bytes = owner_buffer_bytes
            .checked_add(root_buffer_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        PreparedKernel::new(
            (),
            budget::resident_cost! {
                temporary_bytes: owner_index_bytes,
                ..meter.estimate()
            },
        )
        .admit()?
        .into_plan();
        roots.resize(root_storage_len, SchemaId::new(0));
        for location in captured_reads() {
            let Some(ResidentValueRef::Snapshot(values)) = self.read_location(location, working)
            else {
                continue;
            };
            for value in values.iter().flatten() {
                visit_value_schema_roots(
                    value,
                    false,
                    meter,
                    &mut |owner, root, nested_dynamic, meter| {
                        if !schema_root_requires_import(
                            owner,
                            root,
                            nested_dynamic,
                            &self.plan.schemas,
                            meter,
                            owner_index_bytes,
                        )? {
                            return Ok(());
                        }
                        let mut owner_index = None;
                        for (index, existing) in owners.iter().enumerate() {
                            meter.charge_comparison_work(1)?;
                            if Arc::ptr_eq(&existing.owner, owner) {
                                owner_index = Some(index);
                                break;
                            }
                        }
                        let owner_index = owner_index.ok_or(ResidentKernelError::InvalidShape)?;
                        owners[owner_index].insert_root(&mut roots, root, meter)
                    },
                )?;
            }
        }

        let plan_allocation = self
            .plan
            .schemas
            .clone_allocation_bound_bytes()
            .ok_or(ResidentKernelError::InvalidShape)?;
        let mut allocation_bound = plan_allocation
            .checked_add(shared_owner_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let mut closure_peak = plan_allocation
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(shared_owner_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let mut retained_nodes = plan_entries;
        let mut scan_work = 0_u64;
        for owner in &mut owners {
            let remaining = meter.estimate().remaining_incremental_work()?;
            let closure_budget = SnapshotCanonicalizationBudget::new(remaining);
            let (owner_allocation, owner_construction, owner_nodes) = owner
                .owner
                .component_closure_bounds_for_roots_with_budget(
                    owner.roots(&roots),
                    &closure_budget,
                )
                .ok_or(ResidentKernelError::InvalidShape)?;
            meter.charge_comparison_work(
                closure_budget
                    .consumed()
                    .checked_mul(2)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            owner.closure_capacity =
                usize::try_from(owner_nodes).map_err(|_| ResidentKernelError::InvalidShape)?;
            scan_work = owner_nodes
                .checked_mul(plan_entries)
                .and_then(|work| work.checked_mul(key_bytes))
                .and_then(|work| scan_work.checked_add(work))
                .ok_or(ResidentKernelError::InvalidShape)?;
            allocation_bound = allocation_bound
                .checked_add(owner_allocation)
                .ok_or(ResidentKernelError::InvalidShape)?;
            closure_peak = closure_peak
                .checked_add(owner_construction)
                .ok_or(ResidentKernelError::InvalidShape)?;
            retained_nodes = retained_nodes
                .checked_add(owner_nodes)
                .ok_or(ResidentKernelError::InvalidShape)?;
        }
        meter.charge_comparison_work(scan_work)?;
        let (previous_arena_bytes, previous_arena_nodes) = previous_owner
            .as_ref()
            .map(|owner| {
                distinct_schema_owner_footprint(owner, &self.plan.schemas, shared_owner_bytes)
            })
            .transpose()?
            .unwrap_or((0, 0));
        let closure_construction_peak = allocation_bound
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(closure_peak))
            .and_then(|bytes| bytes.checked_add(previous_arena_bytes))
            .and_then(|bytes| bytes.checked_add(owner_index_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let merge_peak = allocation_bound
            .checked_mul(4)
            .and_then(|bytes| bytes.checked_add(previous_arena_bytes))
            .and_then(|bytes| bytes.checked_add(owner_index_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let peak = closure_construction_peak.max(merge_peak);
        // Each extend_preserving_ids call clones the arena accumulated so
        // far. Bound every rebuild by the final arena, not just the first.
        meter.charge_compute_work(
            allocation_bound
                .checked_mul(budget::checked_u64(owners.len())?)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
        meter.charge_comparison_work(
            retained_nodes
                .checked_mul(retained_nodes)
                .and_then(|work| work.checked_mul(key_bytes))
                .and_then(|work| work.checked_add(allocation_bound))
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
        // Closure and merge can overlap the source entries, rooted closure,
        // prior merged table, replacement vector, and boxed-slice shrink.
        // The final arena retains only one population, charged below.
        let construction_nodes = retained_nodes
            .checked_mul(5)
            .and_then(|nodes| nodes.checked_add(previous_arena_nodes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        PreparedKernel::new(
            (),
            budget::resident_cost! {
                output_bytes: allocation_bound,
                temporary_bytes: peak,
                retained_nodes: construction_nodes,
                ..meter.estimate()
            },
        )
        .admit()?
        .into_plan();
        meter.charge_temporary_bytes(previous_arena_bytes)?;
        meter.charge_retained_nodes(previous_arena_nodes)?;
        // The merged arena remains live through every later binding and the
        // final published value, so keep its node population in the shared
        // meter after the one-time construction admission succeeds.
        meter.charge_retained_nodes(retained_nodes)?;
        let mut merged = self.plan.schemas.as_ref().clone();
        for owner in owners {
            let closed = owner
                .owner
                .component_closure_for_roots_with_capacity(
                    owner.roots(&roots),
                    owner.closure_capacity,
                )
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            merged = merged
                .extend_preserving_ids_preallocated(&closed)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
        }
        // Runtime source owners can append schemas that activation could not
        // see. Admit and construct their projection closure once for this
        // arena; every item then uses the indexed table below rather than
        // scanning schema definitions while descending a pattern.
        let remaining = meter.estimate().remaining_incremental_work()?;
        let projection_budget = SnapshotCanonicalizationBudget::new(remaining);
        let (_, projection_construction, projection_entries) = merged
            .component_closure_bounds_with_budget(&projection_budget)
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.charge_comparison_work(projection_budget.consumed())?;
        let projection_bytes = projection_construction
            .checked_mul(4)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let projection_nodes = projection_entries
            .checked_mul(5)
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.charge_compute_work(projection_bytes)?;
        meter.charge_comparison_work(
            projection_nodes
                .checked_mul(key_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
        PreparedKernel::new(
            (),
            budget::resident_cost! {
                output_bytes: projection_bytes,
                temporary_bytes: projection_bytes,
                retained_nodes: projection_nodes,
                ..meter.estimate()
            },
        )
        .admit()?
        .into_plan();
        let entries_before_projection = merged.len();
        let (merged, projections) = structural_projection_schema_context(&merged)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        let appended_schema_nodes = budget::checked_u64(
            merged
                .len()
                .checked_sub(entries_before_projection)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
        let (projection_retained_bytes, projection_retained_nodes) = projections
            .retained_footprint()
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.charge_retained_nodes(
            appended_schema_nodes
                .checked_add(projection_retained_nodes)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
        let retained = merged
            .clone_allocation_bound_bytes()
            .and_then(|bytes| bytes.checked_add(shared_owner_bytes))
            .and_then(|bytes| bytes.checked_add(projection_retained_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok((Arc::new(merged), projections, retained))
    }

    pub(super) fn execute_comprehension(
        &mut self,
        index: ActivatedNodeIndex,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<bool, ResidentExecutionError> {
        self.execute_comprehension_with_live_demand(index, before, working, probe, 0, 0)
    }

    pub(super) fn execute_comprehension_with_live_demand(
        &mut self,
        index: ActivatedNodeIndex,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        live_bytes: u64,
        live_nodes: u64,
    ) -> Result<bool, ResidentExecutionError> {
        let ActivatedTurnStep::Comprehension(control) = &self.plan.steps[index.get() as usize]
        else {
            unreachable!()
        };
        let control = control.clone();
        let result = budget::with_control_work_budget(|| {
            self.with_kernel_turn_plan_and_live_demand(
                index,
                before,
                working,
                live_bytes,
                live_nodes,
                |this| {
                    this.execute_collection_planned(
                        index, &control, before, working, probe, live_bytes, live_nodes,
                    )
                },
            )
        });
        // Lexical payloads have no consumers after this control invocation.
        // This also releases every completed inner allocation on a failed turn.
        for region in &control.locals {
            self.workspace.scratch.discard_payload_write(*region);
        }
        result
    }

    fn execute_collection_planned(
        &mut self,
        index: ActivatedNodeIndex,
        control: &ActivatedComprehensionNode,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        inherited_live_bytes: u64,
        inherited_live_nodes: u64,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel {
            node: control.artifact_node,
            error,
        };
        let mut meter = ResidentBudgetMeter::default();
        let (schemas, projections, schema_arena_bytes) = self
            .comprehension_schema_arena(control, working, &mut meter)
            .map_err(fail)?;
        let published_output_footprint = {
            let target = if control.write.storage == ResidentStorageClass::Constant {
                &self.activation
            } else {
                &self.workspace.scratch
            };
            match target.read(control.write.region) {
                ResidentValueRef::Snapshot([Some(current)]) => {
                    budget::published_canonical_footprint(&mut meter, current, &schemas)
                        .map_err(fail)?
                }
                ResidentValueRef::Snapshot(_) => ValueFootprint::zero(),
                _ => return Err(fail(ResidentKernelError::InvalidOutput)),
            }
        };
        // The previous result remains live while the replacement is built.
        // Thread its footprint through every nested admission with the arena.
        let schema_arena_bytes = schema_arena_bytes
            .checked_add(published_output_footprint.retained_bytes)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        // The wrapper call owns its captured inputs. Child calls only account
        // for captures that they consume themselves, so measure each distinct
        // location once before iterating the comprehension body.
        let mut captured_inputs = HashMap::new();
        let mut captured_total = ValueFootprint::zero();
        let mut seen_captures = Vec::new();
        let mut capture_generation = 0_u64;
        for location in self.plan.reads[control.reads.start as usize..control.reads.end as usize]
            .iter()
            .copied()
        {
            meter.charge_compute_work(1).map_err(fail)?;
            if captured_inputs.contains_key(&location) {
                continue;
            }
            // Charge a conservative bound for the hash bucket and generation
            // slot before either can allocate. Each unique capture is stored
            // once, regardless of how often the wrapper reads it.
            let entry_bytes =
                core::mem::size_of::<(ResidentReadLocation, (usize, ValueFootprint))>()
                    .checked_mul(4)
                    .and_then(|bytes| bytes.checked_add(2 * core::mem::size_of::<u64>()))
                    .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
            meter
                .charge_temporary_bytes(budget::checked_u64(entry_bytes).map_err(fail)?)
                .map_err(fail)?;
            captured_inputs
                .try_reserve(1)
                .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
            seen_captures
                .try_reserve(1)
                .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
            let value = self
                .read_location(location, working)
                .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
            let input =
                Self::resident_value_footprint(value, &schemas, &mut meter).map_err(fail)?;
            captured_total = captured_total
                .checked_add(input)
                .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
            captured_inputs.insert(location, (seen_captures.len(), input));
            seen_captures.push(0);
        }
        let inherited_live_nodes = inherited_live_nodes
            .checked_add(published_output_footprint.node_count)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let mut values = Vec::new();
        let mut footprint = ValueFootprint::zero();
        let mut nested_finalization_work = 0_u64;
        let mut element_body = None;
        let mut touched_local_end = 0;
        self.collection_from(
            control,
            0,
            ComprehensionLiveLocalFootprint::default(),
            &mut touched_local_end,
            &mut values,
            &mut footprint,
            &mut nested_finalization_work,
            &mut element_body,
            &mut meter,
            &schemas,
            &projections,
            schema_arena_bytes,
            &captured_inputs,
            captured_total,
            &mut seen_captures,
            &mut capture_generation,
            before,
            working,
            probe,
            inherited_live_bytes,
            inherited_live_nodes,
        )?;
        let live_locals = self
            .comprehension_live_local_footprint(&control.locals, None, &schemas, &mut meter)
            .map_err(fail)?;
        let draft_count = values.len();
        let draft_capacity = values.capacity();
        let canonical_work =
            collection_canonicalization_work(control.kind, draft_count).map_err(fail)?;
        if control.kind == crate::ComprehensionKind::Set {
            meter.charge_compute_work(canonical_work).map_err(fail)?;
            meter.charge_comparison_work(canonical_work).map_err(fail)?;
        }
        let schema = schemas
            .get(control.output_schema)
            .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?;
        let shape_workspace =
            schema_shape_resolution_workspace(schema, schema.dimension_parameters().len())
                .map_err(fail)?;
        let (live_bytes, live_nodes) = comprehension_nested_live_demand(
            draft_count,
            draft_capacity,
            footprint,
            schema.dimension_parameters().len(),
            schema_arena_bytes,
            live_locals,
            meter,
        )
        .map_err(fail)?;
        admit_generator_schema_workspace(shape_workspace, live_bytes, live_nodes, &mut meter)
            .map_err(fail)?;
        let (count, footprint, shape_values, data) = match control.kind {
            crate::ComprehensionKind::Matrix | crate::ComprehensionKind::MatrixPreserveShape => {
                let mech_core::SchemaBody::Matrix { .. } = schema.body() else {
                    return Err(fail(ResidentKernelError::InvalidOutput));
                };
                let element = element_body
                    .map(Ok)
                    .unwrap_or_else(|| lower_bound_yield_body(control.yield_schema, &schemas))
                    .map_err(fail)?;
                let dimensions = if control.kind == crate::ComprehensionKind::MatrixPreserveShape {
                    let source_shape = control.steps.iter().find_map(|step| match step {
                        crate::resident::general::comprehension::ActivatedCollectionStep::Generator {
                            source,
                            source_schema,
                            shape_values,
                            ..
                        } => Some((*source, *source_schema, shape_values.as_ref())),
                        _ => None,
                    });
                    let (source_location, source_schema, activation_shape_values) =
                        source_shape.ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                    let source_value = self
                        .read_location(source_location, working)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                    let shape_values = generator_shape_values(
                        source_value,
                        source_schema,
                        activation_shape_values,
                        &schemas,
                    )
                    .map_err(fail)?;
                    let source = schemas
                        .get(source_schema)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                    let source_shape = source
                        .instantiate_shape(shape_values)
                        .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
                    let SchemaBody::Matrix { dimensions, .. } =
                        source
                            .closed_body(&source_shape)
                            .map_err(|_| fail(ResidentKernelError::InvalidShape))?
                    else {
                        return Err(fail(ResidentKernelError::InvalidInput));
                    };
                    let source_count = dimensions.iter().try_fold(1u64, |count, dimension| {
                        let DimensionExpr::Constant(dimension) = dimension else {
                            return Err(fail(ResidentKernelError::InvalidShape));
                        };
                        count
                            .checked_mul(*dimension)
                            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))
                    })?;
                    if usize::try_from(source_count).ok() != Some(draft_count) {
                        return Err(fail(ResidentKernelError::InvalidShape));
                    }
                    dimensions
                } else {
                    vec![
                        DimensionExpr::Constant(1),
                        DimensionExpr::Constant(draft_count as u64),
                    ]
                    .into_boxed_slice()
                };
                let actual = SchemaBody::Matrix {
                    element: Box::new(element),
                    dimensions,
                };
                let shape = mech_core::shape_for_schema_components(
                    schema,
                    &[(schema.body(), actual)],
                    None,
                )
                .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
                (
                    draft_count,
                    footprint,
                    shape.parameter_values().to_vec().into_boxed_slice(),
                    {
                        admit_draft_shrink(
                            draft_count,
                            draft_capacity,
                            footprint,
                            schema.dimension_parameters().len(),
                            schema_arena_bytes,
                            live_locals,
                            meter,
                        )
                        .map_err(fail)?;
                        ValueDataDraft::Matrix(values.into_boxed_slice())
                    },
                )
            }
            crate::ComprehensionKind::Set => {
                let mech_core::SchemaBody::Set { element, .. } = schema.body() else {
                    return Err(fail(ResidentKernelError::InvalidOutput));
                };
                // The core key relation owns float normalization and set identity.
                // Admission above covers sorting and finalization before either runs.
                let compare = |left: &ValueDataDraft, right: &ValueDataDraft| {
                    let left = Item::from_draft(left)
                        .expect("binding limits set comprehensions to primitive elements")
                        .data();
                    let right = Item::from_draft(right)
                        .expect("binding limits set comprehensions to primitive elements")
                        .data();
                    mech_core::snapshot::compare_key_data(element, &left, &right)
                        .expect("validated primitive collection element")
                };
                values.sort_unstable_by(compare);
                values.dedup_by(|left, right| compare(left, right).is_eq());
                let footprint = values
                    .iter()
                    .try_fold(ValueFootprint::zero(), |total, value| {
                        let item = match value {
                            ValueDataDraft::Bool(_) => scalar_footprint(1),
                            ValueDataDraft::Index(_) | ValueDataDraft::F64(_) => {
                                scalar_footprint(8)
                            }
                            _ => Err(ResidentKernelError::InvalidInput),
                        }?;
                        total
                            .checked_add(item)
                            .map_err(|_| ResidentKernelError::InvalidShape)
                    })
                    .map_err(fail)?;
                let count = values.len();
                admit_draft_shrink(
                    count,
                    draft_capacity,
                    footprint,
                    schema.dimension_parameters().len(),
                    schema_arena_bytes,
                    live_locals,
                    meter,
                )
                .map_err(fail)?;
                let data = ValueDataDraft::Set(values.into_boxed_slice());
                let shape_values = completed_set_shape_values(schema, &data).map_err(fail)?;
                (count, footprint, shape_values, data)
            }
        };
        let candidate_footprint =
            budget::projected_canonical_value_footprint(footprint, shape_values.len())
                .map_err(fail)?;
        let target = if control.write.storage == ResidentStorageClass::Constant {
            &self.activation
        } else {
            &self.workspace.scratch
        };
        if let ResidentValueRef::Snapshot([Some(current)]) = target.read(control.write.region) {
            let equality_work = budget::projected_language_equality_work(
                &schemas,
                current,
                published_output_footprint,
                control.output_schema,
                shape_values.len(),
                candidate_footprint,
            )
            .map_err(fail)?;
            meter.charge_comparison_work(equality_work).map_err(fail)?;
        }
        admit_output(
            count,
            // Both collection branches have already converted the draft to
            // an exact-length boxed slice. The shrink-overlap admission above
            // owns the retired Vec capacity; only the exact draft remains
            // live during finalization.
            count,
            footprint,
            shape_values.len(),
            schema_arena_bytes,
            live_locals,
            meter,
        )
        .map_err(fail)?;
        let finalization_work = nested_finalization_work
            .checked_add(if control.kind == crate::ComprehensionKind::Set {
                canonical_work
            } else {
                0
            })
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let canonical_budget = SnapshotCanonicalizationBudget::new(finalization_work);
        let next = ValueDraft {
            schema: control.output_schema,
            shape_values,
            data,
        }
        .finalize(
            &SnapshotValidationContext::with_shared_schemas(&schemas)
                .with_canonicalization_budget(&canonical_budget),
        )
        .map_err(|_| fail(ResidentKernelError::InvalidOutput))?;
        let target = if control.write.storage == ResidentStorageClass::Constant {
            &mut self.activation
        } else {
            &mut self.workspace.scratch
        };
        let ResidentValueMut::Snapshot([target]) = target.write(control.write.region) else {
            return Err(fail(ResidentKernelError::InvalidOutput));
        };
        let changed = target
            .as_ref()
            .map_or(Ok(true), |old| {
                old.snapshot_eq(&schemas, &next, &schemas)
                    .map(|equal| !equal)
            })
            .map_err(|_| fail(ResidentKernelError::InvalidOutput))?;
        *target = Some(next);
        set_bit(
            &mut self.workspace.initialized_output_bits,
            index.get() as usize,
        );
        Ok(changed)
    }

    fn collection_pattern_item(
        &self,
        source: ResidentReadLocation,
        element_schema: SchemaId,
        element: &SchemaBody,
        element_shape_values: &[u64],
        ordinal: usize,
        path: &[usize],
        retained_count: usize,
        retained_footprint: ValueFootprint,
        retained_shape_parameter_count: usize,
        schemas: &Arc<mech_core::SchemaTable>,
        projections: &StructuralProjectionTable,
        schema_arena_bytes: u64,
        working: InstanceEpoch,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<(PatternItem, ValueFootprint, ValueFootprint), ResidentKernelError> {
        let value = self
            .read_location(source, working)
            .ok_or(ResidentKernelError::InvalidInput)?;
        let mut footprint =
            collection_item_footprint(value, region(source), element, ordinal, meter)?;
        let metadata_bytes = element
            .clone_allocation_bound_bytes()
            .and_then(|bytes| bytes.checked_add(core::mem::size_of::<SchemaBody>() as u64))
            .and_then(|bytes| {
                u64::try_from(element_shape_values.len())
                    .ok()?
                    .checked_mul(core::mem::size_of::<u64>() as u64)?
                    .checked_add(bytes)
            })
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.charge_compute_work(metadata_bytes)?;
        admit_item_clone(
            footprint,
            metadata_bytes,
            retained_count,
            retained_footprint,
            retained_shape_parameter_count,
            schema_arena_bytes,
            *meter,
        )?;
        let remaining = meter.estimate().remaining_incremental_work()?;
        let canonical_budget = SnapshotCanonicalizationBudget::new(remaining);
        let context = SnapshotValidationContext::with_shared_schemas(schemas)
            .with_canonicalization_budget(&canonical_budget);
        let mut item = collection_item(
            value,
            region(source),
            element_schema,
            element,
            element_shape_values,
            ordinal,
            &context,
        )
        .ok_or(ResidentKernelError::InvalidInput)?;
        meter.charge_comparison_work(canonical_budget.consumed())?;
        if let (
            ResidentValueRef::String(_),
            PatternItem::Component {
                data: ValueDataDraft::String(cloned),
                ..
            },
        ) = (value, &item)
        {
            footprint.retained_bytes = footprint
                .retained_bytes
                .checked_add(budget::checked_u64(
                    cloned.capacity().saturating_sub(cloned.len()),
                )?)
                .ok_or(ResidentKernelError::InvalidShape)?;
            admit_item_clone(
                footprint,
                metadata_bytes,
                retained_count,
                retained_footprint,
                retained_shape_parameter_count,
                schema_arena_bytes,
                *meter,
            )?;
        }
        for index in path {
            item = item
                .child(*index, schemas, projections)
                .ok_or(ResidentKernelError::InvalidInput)?;
        }
        let selected_footprint = if path.is_empty() {
            footprint
        } else {
            descended_collection_item_footprint(value, element, ordinal, path, meter)?
        };
        let concrete_footprint = if matches!(&item, PatternItem::Dynamic(None)) {
            selected_footprint
        } else if path.is_empty()
            && matches!(element, SchemaBody::Dynamic)
            && matches!(value, ResidentValueRef::Snapshot(_))
        {
            descended_collection_item_footprint(value, element, ordinal, path, meter)?
        } else {
            selected_footprint
        };
        Ok((item, selected_footprint, concrete_footprint))
    }

    pub(super) fn bind_collection_pattern_item(
        &mut self,
        node: NodeId,
        binding: ActivatedPatternBinding,
        locals: &[ResidentRegion],
        source_shape_values: &[u64],
        item: PatternItem,
        selected_footprint: ValueFootprint,
        concrete_footprint: ValueFootprint,
        live_item_copies: u64,
        retained_count: usize,
        retained_footprint: ValueFootprint,
        retained_shape_parameter_count: usize,
        schemas: &Arc<mech_core::SchemaTable>,
        projections: &StructuralProjectionTable,
        schema_arena_bytes: u64,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let binding_workspace = item
            .binding_resolution_workspace(binding.schema, source_shape_values, schemas)
            .map_err(fail)?;
        let live_locals = self
            .comprehension_live_local_footprint(locals, None, schemas, meter)
            .map_err(fail)?;
        let live_item = ValueFootprint {
            encoded_bytes: selected_footprint
                .encoded_bytes
                .max(concrete_footprint.encoded_bytes),
            retained_bytes: selected_footprint
                .retained_bytes
                .max(concrete_footprint.retained_bytes),
            node_count: selected_footprint
                .node_count
                .max(concrete_footprint.node_count),
        };
        let live_bytes = snapshot_draft_bytes(
            retained_count,
            retained_footprint,
            retained_shape_parameter_count,
        )
        .and_then(|bytes| {
            bytes
                .checked_add(item_clone_bytes(live_item)?)
                .ok_or(ResidentKernelError::InvalidShape)
        })
        .and_then(|bytes| {
            bytes
                .checked_add(schema_arena_bytes)
                .ok_or(ResidentKernelError::InvalidShape)
        })
        .and_then(|bytes| {
            bytes
                .checked_add(live_locals.retained_bytes)
                .ok_or(ResidentKernelError::InvalidShape)
        })
        .map_err(fail)?;
        let live_nodes = meter
            .estimate()
            .retained_nodes()
            .checked_add(retained_footprint.node_count)
            .and_then(|nodes| nodes.checked_add(live_item.node_count))
            .and_then(|nodes| nodes.checked_add(live_locals.node_count))
            .and_then(|nodes| nodes.checked_add(u64::from(retained_count > 0)))
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        admit_generator_schema_workspace(binding_workspace, live_bytes, live_nodes, meter)
            .map_err(fail)?;
        let Some(PatternBindingItem {
            shape_values,
            data,
            schemas: source_schemas,
            schema_index: source_schema_index,
            footprint,
        }) = item
            .into_binding(binding.schema, source_shape_values, schemas, projections)
            .map_err(fail)?
        else {
            return Ok(false);
        };
        let item_footprint = match footprint {
            BindingFootprint::Selected => selected_footprint,
            BindingFootprint::Concrete => concrete_footprint,
            BindingFootprint::DynamicWrap {
                nested_shape_parameters,
            } => dynamic_wrapped_footprint(selected_footprint, nested_shape_parameters)
                .map_err(fail)?,
        };
        match (data, binding.region.kind) {
            (ValueDataDraft::Bool(value), ResidentValueKind::Bool) => {
                let ResidentValueMut::Bool([target]) = self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was matched")
                };
                *target = u8::from(value);
            }
            (ValueDataDraft::Index(value), ResidentValueKind::Index) => {
                let ResidentValueMut::Index([target]) =
                    self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was matched")
                };
                *target = value;
            }
            (ValueDataDraft::F64(value), ResidentValueKind::F64) => {
                let ResidentValueMut::F64([target]) = self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was matched")
                };
                *target = value.to_f64();
            }
            (ValueDataDraft::String(value), ResidentValueKind::String) => {
                let scope = self
                    .workspace
                    .scratch
                    .prepare_payload_write(binding.region)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                if let Some(scope) = &scope {
                    scope
                        .admit_copy(ResidentValueRef::String(core::slice::from_ref(&value)), 0)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    scope.start();
                }
                let ResidentValueMut::String([target]) =
                    self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was matched")
                };
                *target = value;
                self.workspace
                    .scratch
                    .finish_payload_write(binding.region, scope)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            }
            (data, ResidentValueKind::Snapshot) => {
                let previous_binding = match self.workspace.scratch.read(binding.region) {
                    ResidentValueRef::Snapshot([Some(previous)]) => {
                        let mut previous_meter = ResidentBudgetMeter::default();
                        let footprint = measure_owned_canonical_value_footprint(
                            &mut previous_meter,
                            previous,
                            schemas,
                        )
                        .map_err(fail)?;
                        meter
                            .charge_comparison_work(previous_meter.estimate().comparison_work())
                            .map_err(fail)?;
                        footprint
                    }
                    ResidentValueRef::Snapshot(_) => ValueFootprint::zero(),
                    _ => return Err(fail(ResidentKernelError::InvalidOutput)),
                };
                let other_live_locals = self
                    .comprehension_live_local_footprint(
                        locals,
                        Some(binding.region),
                        schemas,
                        meter,
                    )
                    .map_err(fail)?;
                admit_pattern_binding_finalization(
                    binding.schema,
                    &shape_values,
                    item_footprint,
                    live_item_copies,
                    previous_binding,
                    other_live_locals,
                    retained_count,
                    retained_footprint,
                    retained_shape_parameter_count,
                    schema_arena_bytes,
                    schemas,
                    *meter,
                )
                .map_err(fail)?;
                let remaining = meter
                    .estimate()
                    .remaining_incremental_work()
                    .map_err(fail)?;
                let canonical_budget = SnapshotCanonicalizationBudget::new(remaining);
                let next = finalize_pattern_binding(
                    binding.schema,
                    &shape_values,
                    data,
                    source_schemas,
                    source_schema_index,
                    schemas,
                    &canonical_budget,
                )
                .map_err(|_| fail(ResidentKernelError::InvalidOutput))?;
                meter
                    .charge_comparison_work(canonical_budget.consumed())
                    .map_err(fail)?;
                let scope = self
                    .workspace
                    .scratch
                    .prepare_payload_write(binding.region)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                if let Some(scope) = &scope {
                    scope
                        .admit_value(&next)
                        .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
                    scope.start();
                }
                let ResidentValueMut::Snapshot([target]) =
                    self.workspace.scratch.write(binding.region)
                else {
                    unreachable!("binding kind was matched")
                };
                *target = Some(next);
                self.workspace
                    .scratch
                    .finish_payload_write(binding.region, scope)
                    .map_err(|error| ResidentExecutionError::MemoryRuntime { error })?;
            }
            _ => return Err(fail(ResidentKernelError::InvalidOutput)),
        }
        Ok(true)
    }

    fn match_collection_pattern_item(
        &mut self,
        node: NodeId,
        locals: &[ResidentRegion],
        pattern: &crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
        item: &PatternItem,
        source_shape_values: &[u64],
        item_footprint: ValueFootprint,
        depth: usize,
        retained_count: usize,
        retained_footprint: ValueFootprint,
        retained_shape_parameter_count: usize,
        schemas: &Arc<mech_core::SchemaTable>,
        projections: &StructuralProjectionTable,
        schema_arena_bytes: u64,
        working: InstanceEpoch,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        meter.charge_compute_work(1 + depth as u64).map_err(fail)?;
        match pattern {
            crate::CollectionPattern::Wildcard => Ok(true),
            crate::CollectionPattern::Bind {
                schema: binding, ..
            } => self.bind_collection_pattern_item(
                node,
                *binding,
                locals,
                source_shape_values,
                item.clone(),
                item_footprint,
                item_footprint,
                u64::try_from(depth)
                    .ok()
                    .and_then(|depth| depth.checked_add(2))
                    .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?,
                retained_count,
                retained_footprint,
                retained_shape_parameter_count,
                schemas,
                projections,
                schema_arena_bytes,
                meter,
            ),
            crate::CollectionPattern::Equal(peer) => {
                meter.charge_comparison_work(1).map_err(fail)?;
                let peer_value = self
                    .read_location(peer.location, working)
                    .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                let parameter_count = self
                    .plan
                    .schemas
                    .get(peer.schema)
                    .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?
                    .dimension_parameters()
                    .len();
                let shape_values = vec![0; parameter_count].into_boxed_slice();
                let other_live_locals = self
                    .comprehension_live_local_footprint(locals, None, schemas, meter)
                    .map_err(fail)?;
                admit_pattern_binding_finalization(
                    peer.schema,
                    &shape_values,
                    item_footprint,
                    u64::try_from(depth)
                        .ok()
                        .and_then(|depth| depth.checked_add(2))
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?,
                    ValueFootprint::zero(),
                    other_live_locals,
                    retained_count,
                    retained_footprint,
                    retained_shape_parameter_count,
                    schema_arena_bytes,
                    schemas,
                    *meter,
                )
                .map_err(fail)?;
                let remaining = meter
                    .estimate()
                    .remaining_incremental_work()
                    .map_err(fail)?;
                let canonical_budget = SnapshotCanonicalizationBudget::new(remaining);
                let matched = item
                    .language_equals_with_budget(
                        peer_value,
                        peer.location.region(),
                        peer.schema,
                        source_shape_values,
                        &canonical_budget,
                        schemas,
                        projections,
                    )
                    .map_err(fail)?
                    .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                meter
                    .charge_comparison_work(canonical_budget.consumed())
                    .map_err(fail)?;
                Ok(matched)
            }
            crate::CollectionPattern::Enum { ordinal, payload } => {
                let Some((actual, child)) = item.enum_variant(schemas).map_err(fail)? else {
                    return Ok(false);
                };
                if actual != *ordinal {
                    return Ok(false);
                }
                match (payload.as_deref(), child) {
                    (None, None) => Ok(true),
                    (Some(pattern), Some(child)) => self.match_collection_pattern_item(
                        node,
                        locals,
                        pattern,
                        &child,
                        source_shape_values,
                        item_footprint,
                        depth + 1,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    ),
                    _ => Ok(false),
                }
            }
            crate::CollectionPattern::Tuple(items) => {
                if item.structural_len(true) != Some(items.len()) {
                    return Ok(false);
                }
                for (index, pattern) in items.iter().enumerate() {
                    let Some(child) = item.child(index, schemas, projections) else {
                        return Ok(false);
                    };
                    if !self.match_collection_pattern_item(
                        node,
                        locals,
                        pattern,
                        &child,
                        source_shape_values,
                        item_footprint,
                        depth + 1,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                let Some(count) = item.structural_len(false) else {
                    return Ok(false);
                };
                let required = prefix
                    .len()
                    .checked_add(suffix.len())
                    .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                if count < required || (rest.is_none() && count != required) {
                    return Ok(false);
                }
                for (index, pattern) in prefix.iter().enumerate() {
                    let Some(child) = item.child(index, schemas, projections) else {
                        return Ok(false);
                    };
                    if !self.match_collection_pattern_item(
                        node,
                        locals,
                        pattern,
                        &child,
                        source_shape_values,
                        item_footprint,
                        depth + 1,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                if let Some(rest) = rest {
                    let Some(middle) =
                        item.middle(prefix.len(), suffix.len(), schemas, projections)
                    else {
                        return Ok(false);
                    };
                    if !self.match_collection_pattern_item(
                        node,
                        locals,
                        rest,
                        &middle,
                        source_shape_values,
                        item_footprint,
                        depth + 1,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                for (index, pattern) in suffix.iter().enumerate() {
                    let Some(child) =
                        item.child(count - suffix.len() + index, schemas, projections)
                    else {
                        return Ok(false);
                    };
                    if !self.match_collection_pattern_item(
                        node,
                        locals,
                        pattern,
                        &child,
                        source_shape_values,
                        item_footprint,
                        depth + 1,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    fn match_collection_pattern(
        &mut self,
        node: NodeId,
        locals: &[ResidentRegion],
        source: ResidentReadLocation,
        element_schema: SchemaId,
        element: &SchemaBody,
        element_shape_values: &[u64],
        source_shape_values: &[u64],
        ordinal: usize,
        pattern: &crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
        path: &mut [usize; crate::MAX_COLLECTION_PATTERN_DEPTH],
        depth: usize,
        retained_count: usize,
        retained_footprint: ValueFootprint,
        retained_shape_parameter_count: usize,
        schemas: &Arc<mech_core::SchemaTable>,
        projections: &StructuralProjectionTable,
        schema_arena_bytes: u64,
        working: InstanceEpoch,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        meter.charge_compute_work(1 + depth as u64).map_err(fail)?;
        match pattern {
            crate::CollectionPattern::Wildcard => Ok(true),
            crate::CollectionPattern::Bind {
                schema: binding, ..
            } => {
                let (item, selected_footprint, concrete_footprint) = self
                    .collection_pattern_item(
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        ordinal,
                        &path[..depth],
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )
                    .map_err(fail)?;
                // Partial writes on a failed pattern are private lexical locals.
                // Dominance requires a fresh binding before any later equality read.
                self.bind_collection_pattern_item(
                    node,
                    *binding,
                    locals,
                    source_shape_values,
                    item,
                    selected_footprint,
                    concrete_footprint,
                    1,
                    retained_count,
                    retained_footprint,
                    retained_shape_parameter_count,
                    schemas,
                    projections,
                    schema_arena_bytes,
                    meter,
                )
            }
            crate::CollectionPattern::Equal(_) | crate::CollectionPattern::Enum { .. } => {
                let (item, item_footprint, _) = self
                    .collection_pattern_item(
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        ordinal,
                        &path[..depth],
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )
                    .map_err(fail)?;
                admit_item_clones(
                    item_footprint,
                    item.metadata_bytes().map_err(fail)?,
                    2,
                    retained_count,
                    retained_footprint,
                    retained_shape_parameter_count,
                    schema_arena_bytes,
                    *meter,
                )
                .map_err(fail)?;
                self.match_collection_pattern_item(
                    node,
                    locals,
                    pattern,
                    &item,
                    source_shape_values,
                    item_footprint,
                    0,
                    retained_count,
                    retained_footprint,
                    retained_shape_parameter_count,
                    schemas,
                    projections,
                    schema_arena_bytes,
                    working,
                    meter,
                )
            }
            crate::CollectionPattern::Tuple(items) => {
                let Some(count) = self
                    .collection_pattern_item(
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        ordinal,
                        &path[..depth],
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )
                    .map_err(fail)?
                    .0
                    .structural_len(true)
                else {
                    return Ok(false);
                };
                if count != items.len() {
                    return Ok(false);
                }
                for (index, item) in items.iter().enumerate() {
                    path[depth] = index;
                    if !self.match_collection_pattern(
                        node,
                        locals,
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        source_shape_values,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                let Some(count) = self
                    .collection_pattern_item(
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        ordinal,
                        &path[..depth],
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )
                    .map_err(fail)?
                    .0
                    .structural_len(false)
                else {
                    return Ok(false);
                };
                let required = prefix
                    .len()
                    .checked_add(suffix.len())
                    .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                if count < required || (rest.is_none() && count != required) {
                    return Ok(false);
                }
                for (index, item) in prefix.iter().enumerate() {
                    path[depth] = index;
                    if !self.match_collection_pattern(
                        node,
                        locals,
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        source_shape_values,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                if let Some(rest) = rest {
                    let metrics = crate::pattern_metrics(rest)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                    let copies = u64::try_from(metrics.depth)
                        .ok()
                        .and_then(|depth| depth.checked_add(1))
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                    let (item, item_footprint, _) = self
                        .collection_pattern_item(
                            source,
                            element_schema,
                            element,
                            element_shape_values,
                            ordinal,
                            &path[..depth],
                            retained_count,
                            retained_footprint,
                            retained_shape_parameter_count,
                            schemas,
                            projections,
                            schema_arena_bytes,
                            working,
                            meter,
                        )
                        .map_err(fail)?;
                    admit_item_clones(
                        item_footprint,
                        item.metadata_bytes().map_err(fail)?,
                        copies,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schema_arena_bytes,
                        *meter,
                    )
                    .map_err(fail)?;
                    let Some(middle) =
                        item.middle(prefix.len(), suffix.len(), schemas, projections)
                    else {
                        return Ok(false);
                    };
                    drop(item);
                    if !self.match_collection_pattern_item(
                        node,
                        locals,
                        rest,
                        &middle,
                        source_shape_values,
                        item_footprint,
                        0,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                for (index, item) in suffix.iter().enumerate() {
                    path[depth] = count - suffix.len() + index;
                    if !self.match_collection_pattern(
                        node,
                        locals,
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        source_shape_values,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        retained_count,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        projections,
                        schema_arena_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    fn collection_from(
        &mut self,
        control: &ActivatedComprehensionNode,
        start: usize,
        mut live_local: ComprehensionLiveLocalFootprint,
        touched_local_end: &mut usize,
        values: &mut Vec<ValueDataDraft>,
        footprint: &mut ValueFootprint,
        nested_finalization_work: &mut u64,
        element_body: &mut Option<SchemaBody>,
        meter: &mut ResidentBudgetMeter,
        schemas: &Arc<mech_core::SchemaTable>,
        projections: &StructuralProjectionTable,
        schema_arena_bytes: u64,
        captured_inputs: &HashMap<ResidentReadLocation, (usize, ValueFootprint)>,
        captured_total: ValueFootprint,
        seen_captures: &mut [u64],
        capture_generation: &mut u64,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
        inherited_live_bytes: u64,
        inherited_live_nodes: u64,
    ) -> Result<(), ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel {
            node: control.artifact_node,
            error,
        };
        for position in start..control.steps.len() {
            meter.charge_compute_work(1).map_err(fail)?;
            match &control.steps[position] {
                ActivatedCollectionStep::Operation {
                    node,
                    work,
                    retained_local_count,
                    excluded_locals,
                } => {
                    meter.charge_compute_work(*work).map_err(fail)?;
                    let nested_match = matches!(
                        self.plan.steps[node.get() as usize],
                        ActivatedTurnStep::Match(_)
                    );
                    let output_end = usize::try_from(*retained_local_count)
                        .ok()
                        .and_then(|retained| retained.checked_add(usize::from(!nested_match)))
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                    *touched_local_end = (*touched_local_end).max(output_end);
                    let live_locals = self
                        .incremental_comprehension_live_local_footprint(
                            &control.locals,
                            *retained_local_count,
                            excluded_locals,
                            nested_match,
                            &mut live_local,
                            schemas,
                            meter,
                        )
                        .map_err(fail)?;
                    let shape_parameter_count = schemas
                        .get(control.output_schema)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?
                        .dimension_parameters()
                        .len();
                    let (live_bytes, live_nodes) = comprehension_nested_live_demand(
                        values.len(),
                        values.capacity(),
                        *footprint,
                        shape_parameter_count,
                        schema_arena_bytes,
                        live_locals,
                        *meter,
                    )
                    .map_err(fail)?;
                    let child = self.plan.steps[node.get() as usize].memory_site();
                    let child_reads = child.as_ref().map(|site| {
                        &self.plan.reads[site.reads.start as usize..site.reads.end as usize]
                    });
                    let child_output = child.as_ref().map(|site| match site.write.storage {
                        ResidentStorageClass::Constant => {
                            ResidentReadLocation::Constant(site.write.region)
                        }
                        ResidentStorageClass::Input => {
                            ResidentReadLocation::Input(site.write.region)
                        }
                        ResidentStorageClass::State => ResidentReadLocation::State {
                            slot: site.write.slot,
                            region: site.write.region,
                        },
                        ResidentStorageClass::Scratch => {
                            ResidentReadLocation::Scratch(site.write.region)
                        }
                    });
                    let owned_locations = child_reads
                        .into_iter()
                        .flatten()
                        .copied()
                        .chain(child.as_ref().and_then(|site| site.rmw_base))
                        .chain(child_output);
                    let unowned_captures = unowned_comprehension_capture_footprint(
                        captured_inputs,
                        captured_total,
                        owned_locations,
                        seen_captures,
                        capture_generation,
                        meter,
                    )
                    .map_err(fail)?;
                    // A child installs its own turn plan. Carry the demand
                    // received from enclosing controls, this comprehension's
                    // draft and locals, and captures absent from the child call.
                    let live_bytes = live_bytes
                        .checked_add(inherited_live_bytes)
                        .and_then(|bytes| bytes.checked_add(unowned_captures.retained_bytes))
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                    let live_nodes = live_nodes
                        .checked_add(inherited_live_nodes)
                        .and_then(|nodes| nodes.checked_add(unowned_captures.node_count))
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                    self.execute_step_with_live_demand(
                        *node, before, working, probe, live_bytes, live_nodes,
                    )?;
                }
                ActivatedCollectionStep::Filter(source) => {
                    match self.read_location(*source, working) {
                        Some(ResidentValueRef::Bool([1])) => {}
                        Some(ResidentValueRef::Bool([0])) => return Ok(()),
                        _ => return Err(fail(ResidentKernelError::InvalidInput)),
                    }
                }
                ActivatedCollectionStep::Generator {
                    source,
                    source_schema,
                    discard_from,
                    binding_end,
                    element_schema,
                    shape_values: activation_shape_values,
                    pattern,
                } => {
                    let source_value = self
                        .read_location(*source, working)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                    let count = collection_len(source_value)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                    let live_shape_values = generator_shape_values(
                        source_value,
                        *source_schema,
                        activation_shape_values,
                        schemas,
                    )
                    .map_err(fail)?;
                    let element = if let Some(element_schema) = element_schema {
                        let (body, shape_values) = generator_element(
                            *source_schema,
                            *element_schema,
                            &live_shape_values,
                            schemas,
                        )
                        .map_err(fail)?;
                        Some((*element_schema, body, shape_values))
                    } else {
                        debug_assert!(matches!(pattern, crate::CollectionPattern::Wildcard));
                        None
                    };
                    let retained_shape_parameter_count = schemas
                        .get(control.output_schema)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?
                        .dimension_parameters()
                        .len();
                    for ordinal in 0..count {
                        meter.charge_compute_work(1).map_err(fail)?;
                        let mut iteration_end = usize::try_from(*binding_end)
                            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
                        let matched = if let Some((element_schema, element, element_shape_values)) =
                            &element
                        {
                            self.match_collection_pattern(
                                control.artifact_node,
                                &control.locals,
                                *source,
                                *element_schema,
                                element,
                                element_shape_values,
                                &live_shape_values,
                                ordinal,
                                pattern,
                                &mut [0; crate::MAX_COLLECTION_PATTERN_DEPTH],
                                0,
                                values.len(),
                                *footprint,
                                retained_shape_parameter_count,
                                schemas,
                                projections,
                                schema_arena_bytes,
                                working,
                                meter,
                            )?
                        } else {
                            true
                        };
                        if matched {
                            self.collection_from(
                                control,
                                position + 1,
                                live_local,
                                &mut iteration_end,
                                values,
                                footprint,
                                nested_finalization_work,
                                element_body,
                                meter,
                                schemas,
                                projections,
                                schema_arena_bytes,
                                captured_inputs,
                                captured_total,
                                seen_captures,
                                capture_generation,
                                before,
                                working,
                                probe,
                                inherited_live_bytes,
                                inherited_live_nodes,
                            )?;
                        }
                        // A later operation or generator can still own its
                        // previous iteration's payload. Release the whole
                        // lexical suffix before the next element; only the
                        // preceding locals may remain live across iterations.
                        let discard_from = usize::try_from(*discard_from)
                            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
                        let suffix = control
                            .locals
                            .get(discard_from..iteration_end)
                            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                        for region in suffix {
                            self.workspace.scratch.discard_payload_write(*region);
                        }
                    }
                    return Ok(());
                }
            }
        }
        let value = self
            .read_location(control.yield_value, working)
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
        let closed_body = closed_yield_body(
            value,
            control.yield_value.region(),
            control.yield_schema,
            schemas,
        )
        .map_err(fail)?;
        if element_body
            .as_ref()
            .is_some_and(|expected| expected != &closed_body)
        {
            return Err(fail(ResidentKernelError::InvalidShape));
        }
        let (item_footprint, item_finalization_work) =
            retained_value_footprint(value, control.yield_schema, schemas, meter).map_err(fail)?;
        let next_footprint = footprint
            .checked_add(item_footprint)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        let next = values
            .len()
            .checked_add(1)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let next_finalization_work = nested_finalization_work
            .checked_add(item_finalization_work)
            .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
        let retained_shape_parameter_count = schemas
            .get(control.output_schema)
            .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?
            .dimension_parameters()
            .len();
        let live_locals = self
            .comprehension_live_local_footprint(&control.locals, None, schemas, meter)
            .map_err(fail)?;
        let current_capacity = values.capacity();
        if next > current_capacity {
            meter
                .charge_compute_work(budget::checked_u64(values.len()).map_err(fail)?)
                .map_err(fail)?;
        }
        match control.kind {
            crate::ComprehensionKind::Matrix | crate::ComprehensionKind::MatrixPreserveShape => {
                admit_output(
                    next,
                    current_capacity,
                    next_footprint,
                    retained_shape_parameter_count,
                    schema_arena_bytes,
                    live_locals,
                    *meter,
                )
            }
            crate::ComprehensionKind::Set => admit_set_draft(
                next,
                current_capacity,
                next_footprint,
                retained_shape_parameter_count,
                schema_arena_bytes,
                live_locals,
                *meter,
            ),
        }
        .map_err(fail)?;
        let remaining = meter
            .estimate()
            .remaining_incremental_work()
            .map_err(fail)?;
        let canonical_budget = SnapshotCanonicalizationBudget::new(remaining);
        let context = SnapshotValidationContext::with_shared_schemas(schemas)
            .with_canonicalization_budget(&canonical_budget);
        let item = retained_item_in(value, control.yield_schema, &context)
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
        meter
            .charge_comparison_work(canonical_budget.consumed())
            .map_err(fail)?;
        values
            .try_reserve_exact(1)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        // The allocator may grant more than the requested slot. Re-admit its
        // actual live capacity before retaining the new result item.
        match control.kind {
            crate::ComprehensionKind::Matrix | crate::ComprehensionKind::MatrixPreserveShape => {
                admit_output(
                    next,
                    values.capacity(),
                    next_footprint,
                    retained_shape_parameter_count,
                    schema_arena_bytes,
                    live_locals,
                    *meter,
                )
            }
            crate::ComprehensionKind::Set => admit_set_draft(
                next,
                values.capacity(),
                next_footprint,
                retained_shape_parameter_count,
                schema_arena_bytes,
                live_locals,
                *meter,
            ),
        }
        .map_err(fail)?;
        values.push(item);
        element_body.get_or_insert(closed_body);
        *footprint = next_footprint;
        *nested_finalization_work = next_finalization_work;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        CanonicalNominalPath, CardinalitySpec, DimensionExpr, DimensionLifetime,
        DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin, FloatWidth,
        IntegerWidth, NominalKey, NominalKind, ResidentShape, SchemaDraft, SchemaTableBuilder,
    };

    fn atom(name: &str) -> SchemaBody {
        SchemaBody::Atom(NominalKey::from_path(
            NominalKind::Atom,
            &CanonicalNominalPath::new(vec![name.to_owned()]).unwrap(),
        ))
    }

    #[test]
    fn capture_accounting_deduplicates_child_inputs_without_scanning_all_captures() {
        const COUNT: usize = 16_384;
        let location = |offset| {
            ResidentReadLocation::Scratch(ResidentRegion {
                kind: ResidentValueKind::Snapshot,
                offset,
                len: 1,
                shape: ResidentShape {
                    rows: 1,
                    columns: 1,
                },
            })
        };
        let one = ValueFootprint {
            encoded_bytes: 0,
            retained_bytes: 1,
            node_count: 1,
        };
        let captures = (0..COUNT)
            .map(|index| (location(index), (index, one)))
            .collect::<HashMap<_, _>>();
        let total = ValueFootprint {
            encoded_bytes: 0,
            retained_bytes: COUNT as u64,
            node_count: COUNT as u64,
        };
        let mut seen = vec![0; COUNT];
        let mut generation = 0;
        let mut meter = ResidentBudgetMeter::default();
        for index in 0..COUNT {
            let unowned = unowned_comprehension_capture_footprint(
                &captures,
                total,
                [location(index), location(index)],
                &mut seen,
                &mut generation,
                &mut meter,
            )
            .unwrap();
            assert_eq!(unowned.retained_bytes, COUNT as u64 - 1);
            assert_eq!(unowned.node_count, COUNT as u64 - 1);
        }
        assert_eq!(meter.estimate().compute_work(), 2 * COUNT as u64);
    }

    #[test]
    fn enum_pattern_payload_preserves_resolved_shape_values() {
        let parameter = DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(8)),
        };
        let enum_body = SchemaBody::Enum {
            key: NominalKey::from_bytes([0x13; 32]),
            variants: vec![mech_core::EnumVariantSchema {
                name: "values".to_owned(),
                payload: Some(SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                    dimensions: vec![
                        DimensionExpr::Constant(1),
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    ]
                    .into_boxed_slice(),
                }),
            }]
            .into_boxed_slice(),
        };
        let mut builder = SchemaTableBuilder::new();
        let enumeration = builder
            .insert(
                SchemaDraft {
                    body: enum_body,
                    dimension_parameters: vec![parameter].into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let enumeration = built.resolve(enumeration).unwrap();
        let schemas = built.into_parts().0;
        let body = schemas.get(enumeration).unwrap().body().clone();
        let item = PatternItem::component(
            Some(enumeration),
            body,
            vec![3].into_boxed_slice(),
            ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                ordinal: 0,
                payload: Some(Box::new(ValueDataDraft::Matrix(
                    [1.0, 2.0, 3.0]
                        .into_iter()
                        .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                        .collect(),
                ))),
            }),
        );

        let (ordinal, Some(PatternItem::Component { shape_values, .. })) =
            item.enum_variant(&schemas).unwrap().unwrap()
        else {
            panic!("enum payload must remain a shaped component")
        };
        assert_eq!(ordinal, 0);
        assert_eq!(shape_values.as_ref(), [3]);
    }

    #[test]
    fn enum_pattern_payload_retains_foreign_dynamic_schema_owner() {
        let enum_body = SchemaBody::Enum {
            key: NominalKey::from_bytes([0x31; 32]),
            variants: vec![mech_core::EnumVariantSchema {
                name: "wrapped".to_owned(),
                payload: Some(SchemaBody::Dynamic),
            }]
            .into_boxed_slice(),
        };
        let schema = |body| {
            SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap()
        };
        let mut foreign = SchemaTableBuilder::new();
        let foreign_tuple = foreign
            .insert(schema(SchemaBody::Tuple(
                vec![SchemaBody::FloatingPoint(FloatWidth::W64), SchemaBody::Bool]
                    .into_boxed_slice(),
            )))
            .unwrap();
        let foreign_enum = foreign.insert(schema(enum_body.clone())).unwrap();
        let foreign = foreign.finish().unwrap();
        let foreign_tuple = foreign.resolve(foreign_tuple).unwrap();
        let foreign_enum = foreign.resolve(foreign_enum).unwrap();
        let foreign = Arc::new(foreign.table);

        let mut plan = SchemaTableBuilder::new();
        plan.insert(schema(SchemaBody::Bool)).unwrap();
        let plan_enum = plan.insert(schema(enum_body)).unwrap();
        let plan = plan.finish().unwrap();
        let plan_enum = plan.resolve(plan_enum).unwrap();
        let plan = Arc::new(plan.table);
        assert!(
            plan.find_by_key(foreign.entry(foreign_tuple).unwrap().key())
                .is_none()
        );

        let source = ValueDraft {
            schema: foreign_enum,
            shape_values: Box::new([]),
            data: ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                ordinal: 0,
                payload: Some(Box::new(ValueDataDraft::Dynamic(Some(Box::new(
                    ValueDraft {
                        schema: foreign_tuple,
                        shape_values: Box::new([]),
                        data: ValueDataDraft::Tuple(
                            vec![
                                ValueDataDraft::F64(F64Bits::from_f64(7.0)),
                                ValueDataDraft::Bool(true),
                            ]
                            .into_boxed_slice(),
                        ),
                    },
                ))))),
            }),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&foreign))
        .unwrap();
        let lane = [Some(source)];
        let item = resident_pattern_item(
            ResidentValueRef::Snapshot(&lane),
            ResidentRegion {
                kind: ResidentValueKind::Snapshot,
                offset: 0,
                len: 1,
                shape: mech_core::ResidentShape::SCALAR,
            },
            plan_enum,
            &[],
            &plan,
            false,
        )
        .expect("matching nominal enum from a foreign schema arena");
        let (ordinal, Some(payload)) = item.enum_variant(&plan).unwrap().unwrap() else {
            panic!("enum payload remains available");
        };
        assert_eq!(ordinal, 0);
        assert!(matches!(
            payload.resolved_source_data().unwrap(),
            Some(ValueDataDraft::Tuple(values)) if values.len() == 2
        ));
    }

    #[test]
    fn buffer_accounting_uses_the_allocator_returned_capacity() {
        let mut values = Vec::<SchemaId>::new();
        values.try_reserve_exact(3).unwrap();
        assert_eq!(
            allocation_capacity_bytes::<SchemaId>(values.capacity()).unwrap(),
            (values.capacity() * core::mem::size_of::<SchemaId>()) as u64,
        );
    }

    #[test]
    fn runtime_projection_footprint_retains_entries_and_tuple_indices() {
        let mut builder = SchemaTableBuilder::new();
        builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::Bool, SchemaBody::Index].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let (schemas, projections) = structural_projection_schema_context(&build.table).unwrap();
        let (bytes, nodes) = projections.retained_footprint().unwrap();
        let tuple_children = projections
            .entries
            .iter()
            .map(|entry| entry.tuple_children.len())
            .sum::<usize>();

        assert_eq!(
            bytes,
            (schemas.len() * core::mem::size_of::<StructuralProjectionEntry>()
                + tuple_children * core::mem::size_of::<Option<SchemaId>>()) as u64
        );
        assert_eq!(
            nodes,
            1 + projections
                .entries
                .iter()
                .filter(|entry| !entry.tuple_children.is_empty())
                .count() as u64
        );
    }

    #[test]
    fn nested_control_live_demand_includes_the_retained_outer_draft() {
        let mut values = Vec::<ValueDataDraft>::new();
        values.try_reserve_exact(3).unwrap();
        values.push(ValueDataDraft::String("retained".to_owned()));
        let footprint = ValueFootprint {
            encoded_bytes: 8,
            retained_bytes: 4_096,
            node_count: 5,
        };

        let (bytes, nodes) = comprehension_nested_live_demand(
            values.len(),
            values.capacity(),
            footprint,
            0,
            0,
            ValueFootprint::zero(),
            ResidentBudgetMeter::default(),
        )
        .unwrap();
        assert!(
            bytes >= (values.capacity() * core::mem::size_of::<ValueDataDraft>()) as u64 + 4_096
        );
        assert!(nodes >= 6);
    }

    #[test]
    fn binding_resolution_charges_constructed_bodies_without_schema_arenas() {
        let mut builder = SchemaTableBuilder::new();
        let matrix = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let (schemas, _) = build.into_parts();
        let item = PatternItem::component(
            Some(matrix),
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                    .into_boxed_slice(),
            },
            vec![3].into_boxed_slice(),
            ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect(),
            ),
        );
        let workspace = item
            .binding_resolution_workspace(matrix, &[3], &schemas)
            .unwrap();
        let body_bytes = schemas
            .get(matrix)
            .unwrap()
            .body()
            .clone_allocation_bound_bytes()
            .unwrap();

        assert_eq!(
            workspace,
            body_bytes * 5 + 8 * core::mem::size_of::<u64>() as u64
        );
    }

    #[test]
    fn ordinary_scalar_binding_does_not_charge_the_schema_arena() {
        let mut builder = SchemaTableBuilder::new();
        let scalar = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        for width in [
            IntegerWidth::W8,
            IntegerWidth::W16,
            IntegerWidth::W32,
            IntegerWidth::W64,
        ] {
            builder
                .insert(
                    SchemaDraft {
                        body: SchemaBody::UnsignedInteger(width),
                        dimension_parameters: Box::new([]),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
        }
        let build = builder.finish().unwrap();
        let scalar = build.resolve(scalar).unwrap();
        let (schemas, _) = build.into_parts();
        let item = PatternItem::Plain(ValueDataDraft::F64(F64Bits::from_f64(1.0)));

        assert_eq!(
            item.binding_resolution_workspace(scalar, &[], &schemas)
                .unwrap(),
            0
        );
        assert!(schemas.clone_allocation_bound_bytes().unwrap() > 0);
    }

    #[test]
    fn nested_dynamic_items_resolve_concrete_bindings() {
        let mut builder = SchemaTableBuilder::new();
        let dynamic = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let f64_schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let bool_schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let dynamic = build.resolve(dynamic).unwrap();
        let f64_schema = build.resolve(f64_schema).unwrap();
        let bool_schema = build.resolve(bool_schema).unwrap();
        let (schemas, _) = build.into_parts();
        let item = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: dynamic,
            shape_values: Box::new([]),
            data: ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                schema: f64_schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::F64(F64Bits::from_f64(3.5)),
            }))),
        }))));

        assert!(matches!(item.data(), Some(ValueDataDraft::F64(value)) if value.to_f64() == 3.5));
        let binding = item
            .clone()
            .into_binding(
                f64_schema,
                &[],
                &schemas,
                &StructuralProjectionTable::default(),
            )
            .expect("binding inspection succeeds")
            .expect("the concrete annotation matches the nested dynamic payload");
        assert!(binding.shape_values.is_empty());
        assert!(matches!(binding.data, ValueDataDraft::F64(value) if value.to_f64() == 3.5));
        assert!(
            item.into_binding(
                bool_schema,
                &[],
                &schemas,
                &StructuralProjectionTable::default()
            )
            .unwrap()
            .is_none(),
            "a valid concrete type mismatch remains an ordinary nonmatch",
        );
    }

    #[test]
    fn concrete_binding_resolves_shape_values_in_its_own_schema() {
        let mut builder = SchemaTableBuilder::new();
        let parameterized = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let fixed = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let parameterized = build.resolve(parameterized).unwrap();
        let fixed = build.resolve(fixed).unwrap();
        let (schemas, _) = build.into_parts();
        let item = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: parameterized,
            shape_values: vec![3].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect(),
            ),
        }))));

        let binding = item
            .into_binding(fixed, &[], &schemas, &StructuralProjectionTable::default())
            .expect("binding inspection succeeds")
            .expect("equivalent closed schemas match across parameter arenas");
        assert!(
            binding.shape_values.is_empty(),
            "the fixed annotation owns no shape parameters"
        );
        assert!(matches!(binding.data, ValueDataDraft::Matrix(values) if values.len() == 3));
    }

    #[test]
    fn snapshot_binding_admission_counts_draft_and_finalized_payloads_together() {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::String,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let item = ValueFootprint {
            encoded_bytes: 9 * 1024 * 1024,
            retained_bytes: 9 * 1024 * 1024,
            node_count: 1,
        };
        assert!(
            admit_item_clone(
                item,
                0,
                0,
                ValueFootprint::zero(),
                0,
                0,
                ResidentBudgetMeter::default()
            )
            .is_ok(),
            "the mutable item draft fits before finalization"
        );
        assert!(
            admit_pattern_binding_finalization(
                schema,
                &[],
                item,
                1,
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                ValueFootprint::zero(),
                0,
                0,
                &schemas,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "the overlapping draft and immutable candidate exceed the temporary ceiling"
        );
    }

    #[test]
    fn dynamic_child_descent_carries_the_canonical_component_schema() {
        let mut builder = SchemaTableBuilder::new();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::FloatingPoint(FloatWidth::W64)].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let f64_schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let _f64_schema = build.resolve(f64_schema).unwrap();
        let (schemas, _) = build.into_parts();
        let item = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::F64(F64Bits::from_f64(7.0))].into_boxed_slice(),
            ),
        }))));

        let child = item
            .child(0, &schemas, &StructuralProjectionTable::default())
            .expect("component descent");
        assert!(matches!(child.data(), Some(ValueDataDraft::F64(value)) if value.to_f64() == 7.0));
        assert!(matches!(
            child,
            PatternItem::Component {
                body: SchemaBody::FloatingPoint(FloatWidth::W64),
                ..
            }
        ));
    }

    #[test]
    fn structural_projection_schemas_preserve_nested_dynamic_bindings() {
        let mut builder = SchemaTableBuilder::new();
        let dynamic = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::FloatingPoint(FloatWidth::W64)].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let dynamic = build.resolve(dynamic).unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let (schemas, _) = build.into_parts();
        let (schemas, projections) = structural_projection_schema_context(&schemas).unwrap();
        let item = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::F64(F64Bits::from_f64(7.0))].into_boxed_slice(),
            ),
        }))));
        let child = item.child(0, &schemas, &projections).unwrap();
        let binding = child
            .into_binding(dynamic, &[], &schemas, &projections)
            .unwrap()
            .expect("projected tuple child binds through Dynamic");
        assert!(
            matches!(binding.data, ValueDataDraft::Dynamic(Some(value)) if matches!(value.data, ValueDataDraft::F64(number) if number.to_f64() == 7.0))
        );
    }

    #[test]
    fn parameterized_binding_preflights_schema_and_shape_resolution() {
        let mut builder = SchemaTableBuilder::new();
        let matrix = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let (schemas, _) = build.into_parts();
        let item = PatternItem::component(
            Some(matrix),
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                    .into_boxed_slice(),
            },
            vec![3].into_boxed_slice(),
            ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect(),
            ),
        );
        let workspace = item
            .binding_resolution_workspace(matrix, &[3], &schemas)
            .unwrap();
        let body_bytes = schemas
            .get(matrix)
            .unwrap()
            .body()
            .clone_allocation_bound_bytes()
            .unwrap();
        assert_eq!(
            workspace,
            body_bytes * 5 + 8 * core::mem::size_of::<u64>() as u64
        );
    }

    #[test]
    fn output_shape_preflight_includes_retained_parameter_values() {
        let mut builder = SchemaTableBuilder::new();
        let schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![SchemaBody::String; 256].into_boxed_slice()),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(schema).unwrap();
        let (schemas, _) = build.into_parts();
        let workspace = schema_shape_resolution_workspace(schemas.get(schema).unwrap(), 1).unwrap();
        assert!(
            workspace
                > schemas
                    .get(schema)
                    .unwrap()
                    .clone_allocation_bound_bytes()
                    .unwrap()
        );
        assert!(workspace >= 6 * core::mem::size_of::<u64>() as u64);
        assert!(workspace < schemas.clone_allocation_bound_bytes().unwrap());
    }

    #[test]
    fn dynamic_component_binding_rejects_an_unaddressable_child_schema() {
        let mut builder = SchemaTableBuilder::new();
        let dynamic = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::FloatingPoint(FloatWidth::W64)].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let dynamic = build.resolve(dynamic).unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let (schemas, _) = build.into_parts();
        let original_entries = schemas
            .entries()
            .map(|entry| entry.canonical_bytes().to_vec())
            .collect::<Vec<_>>();
        let (schemas, projections) = structural_projection_schema_context(&schemas).unwrap();
        assert_eq!(
            schemas
                .entries()
                .take(original_entries.len())
                .map(|entry| entry.canonical_bytes())
                .collect::<Vec<_>>(),
            original_entries
                .iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>(),
            "projection schemas append without changing artifact schema IDs",
        );
        let child = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::F64(F64Bits::from_f64(7.0))].into_boxed_slice(),
            ),
        }))))
        .child(0, &schemas, &projections)
        .unwrap();

        let binding = child
            .into_binding(dynamic, &[], &schemas, &projections)
            .unwrap()
            .expect("the projected component binds through Dynamic");
        let ValueDataDraft::Dynamic(Some(value)) = binding.data else {
            panic!("Dynamic binding retains its concrete value")
        };
        assert!(matches!(
            schemas.get(value.schema).unwrap().body(),
            SchemaBody::FloatingPoint(FloatWidth::W64)
        ));
        assert!(matches!(value.data, ValueDataDraft::F64(value) if value.to_f64() == 7.0));
    }

    #[test]
    fn plain_native_array_rest_infers_its_binding_extent() {
        let parent = SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                    .into_boxed_slice(),
            },
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        builder.insert(parent.clone()).unwrap();
        let rest = builder
            .insert(
                projected_rest_schema(&parent, SchemaBody::FloatingPoint(FloatWidth::W64)).unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let rest = build.resolve(rest).unwrap();
        let (schemas, projections) = structural_projection_schema_context(&build.table).unwrap();
        let native = PatternItem::new(ValueDataDraft::Matrix(
            [1.0, 2.0, 3.0]
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                .into(),
        ));
        let middle = native.middle(1, 0, &schemas, &projections).unwrap();
        let binding = middle
            .into_binding(rest, &[], &schemas, &projections)
            .unwrap()
            .unwrap();
        assert_eq!(binding.shape_values.as_ref(), [2]);
        assert!(matches!(binding.data, ValueDataDraft::Matrix(values) if values.len() == 2));
    }

    #[test]
    fn native_bool_array_pattern_rejects_noncanonical_bytes() {
        let region = ResidentRegion {
            kind: ResidentValueKind::Bool,
            offset: 0,
            len: 1,
            shape: mech_core::ResidentShape::SCALAR,
        };
        assert!(resident_array_pattern_item(ResidentValueRef::Bool(&[2]), region).is_none());
        assert!(matches!(
            resident_array_pattern_item(ResidentValueRef::Bool(&[1]), region),
            Some(PatternItem::Plain(ValueDataDraft::Matrix(values)))
                if matches!(values.first(), Some(ValueDataDraft::Bool(true)))
        ));
    }

    #[test]
    fn projected_tuple_binding_keeps_referenced_rest_extent() {
        let body = SchemaBody::Tuple(
            vec![SchemaBody::Matrix {
                element: Box::new(SchemaBody::Index),
                dimensions: vec![
                    DimensionExpr::Constant(1),
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                ]
                .into_boxed_slice(),
            }]
            .into_boxed_slice(),
        );
        let mut builder = SchemaTableBuilder::new();
        let binding = builder
            .insert(
                SchemaDraft {
                    body: body.clone(),
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Inferred,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: None,
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let binding = build.resolve(binding).unwrap();
        let (schemas, projections) = structural_projection_schema_context(&build.table).unwrap();
        for extent in [2, 3] {
            let item = PatternItem::component(
                Some(binding),
                SchemaBody::Tuple(
                    vec![SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Index),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Constant(extent),
                        ]
                        .into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
                vec![extent].into_boxed_slice(),
                ValueDataDraft::Tuple(
                    vec![ValueDataDraft::Matrix(
                        (0..extent)
                            .map(|value| ValueDataDraft::Index(value + 1))
                            .collect(),
                    )]
                    .into_boxed_slice(),
                ),
            );
            let bound = item
                .into_binding(binding, &[], &schemas, &projections)
                .unwrap()
                .unwrap();
            assert_eq!(bound.shape_values.as_ref(), [extent]);
            let value = pattern_binding_draft(binding, &bound.shape_values, bound.data)
                .finalize(&SnapshotValidationContext::new(&schemas))
                .unwrap();
            assert_eq!(value.shape().parameter_values(), [extent]);
        }
    }

    #[test]
    fn projected_tuple_binding_prunes_an_unused_rest_extent() {
        let body = SchemaBody::Tuple(vec![SchemaBody::Index].into_boxed_slice());
        let mut builder = SchemaTableBuilder::new();
        let binding = builder
            .insert(
                SchemaDraft {
                    body: body.clone(),
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Inferred,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: None,
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let binding = build.resolve(binding).unwrap();
        let (schemas, projections) = structural_projection_schema_context(&build.table).unwrap();
        let item = PatternItem::component(
            Some(binding),
            body,
            vec![2].into_boxed_slice(),
            ValueDataDraft::Tuple(vec![ValueDataDraft::Index(7)].into_boxed_slice()),
        );
        let bound = item
            .into_binding(binding, &[], &schemas, &projections)
            .unwrap()
            .unwrap();
        assert!(bound.shape_values.is_empty());
        let value = pattern_binding_draft(binding, &bound.shape_values, bound.data)
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap();
        assert!(value.shape().parameter_values().is_empty());
    }

    #[test]
    fn snapshot_array_rest_keeps_unreferenced_source_parameter() {
        let parent = SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                    dimensions: vec![
                        DimensionExpr::Constant(1),
                        DimensionExpr::Parameter(DimensionParameterId::new(1)),
                    ]
                    .into_boxed_slice(),
                }),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                    .into_boxed_slice(),
            },
            dimension_parameters: vec![
                DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: Some(DimensionExpr::Constant(8)),
                },
                DimensionParameterDeclaration {
                    id: DimensionParameterId::new(1),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: Some(DimensionExpr::Parameter(DimensionParameterId::new(0))),
                },
            ]
            .into_boxed_slice(),
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let parent_id = builder.insert(parent.clone()).unwrap();
        let SchemaBody::Matrix { element, .. } = parent.body() else {
            unreachable!()
        };
        let rest_id = builder
            .insert(projected_rest_schema(&parent, element.as_ref().clone()).unwrap())
            .unwrap();
        let build = builder.finish().unwrap();
        let parent_id = build.resolve(parent_id).unwrap();
        let rest_id = build.resolve(rest_id).unwrap();
        let source_schemas = Arc::new(build.table);
        let value = ValueDraft {
            schema: parent_id,
            shape_values: vec![5, 2].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0]
                    .map(|value| {
                        ValueDataDraft::Matrix(
                            [value, value + 1.0]
                                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                                .into(),
                        )
                    })
                    .into(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &source_schemas,
        ))
        .unwrap();
        let (schemas, projections) = structural_projection_schema_context(&source_schemas).unwrap();
        let values = [Some(value)];
        assert_eq!(
            generator_shape_values(
                ResidentValueRef::Snapshot(&values),
                parent_id,
                &[0],
                &schemas,
            )
            .unwrap()
            .as_ref(),
            [5, 2],
        );
        let region = ResidentRegion {
            kind: ResidentValueKind::Snapshot,
            offset: 0,
            len: 1,
            shape: mech_core::ResidentShape::SCALAR,
        };
        let item = resident_pattern_item(
            ResidentValueRef::Snapshot(&values),
            region,
            parent_id,
            &[5, 2],
            &schemas,
            false,
        )
        .unwrap();
        let rest = item.middle(1, 0, &schemas, &projections).unwrap();
        let binding = rest
            .into_binding(rest_id, &[5, 2], &schemas, &projections)
            .unwrap()
            .unwrap();
        assert_eq!(binding.shape_values.as_ref(), [5, 2, 2]);
        let bound = pattern_binding_draft(rest_id, &binding.shape_values, binding.data)
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap();
        assert_eq!(bound.shape().parameter_values(), [5, 2, 2]);
    }

    #[test]
    fn dynamic_array_rest_wraps_concrete_elements_for_its_binding_schema() {
        let mut builder = SchemaTableBuilder::new();
        let concrete = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let rest = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Dynamic),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Inferred,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: None,
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let concrete = build.resolve(concrete).unwrap();
        let rest = build.resolve(rest).unwrap();
        let (schemas, _) = build.into_parts();
        let (schemas, projections) = structural_projection_schema_context(&schemas).unwrap();
        let item = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: concrete,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0]
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .into(),
            ),
        }))));
        let middle = item
            .middle(1, 0, &schemas, &projections)
            .expect("middle projection");
        let binding = middle
            .into_binding(rest, &[], &schemas, &projections)
            .unwrap()
            .expect("concrete rest conforms to Matrix<Dynamic>");
        assert_eq!(binding.shape_values.as_ref(), [2]);
        let ValueDataDraft::Matrix(values) = binding.data else {
            panic!("rest binding is a matrix")
        };
        assert_eq!(values.len(), 2);
        assert!(values.iter().zip([2.0, 3.0]).all(|(value, expected)| {
            let ValueDataDraft::Dynamic(Some(value)) = value else {
                return false;
            };
            matches!(value.data, ValueDataDraft::F64(value) if value.to_f64() == expected)
                && matches!(
                    schemas.get(value.schema).unwrap().body(),
                    SchemaBody::FloatingPoint(FloatWidth::W64)
                )
        }));
    }

    #[test]
    fn descended_binding_finalization_uses_only_the_selected_child_footprint() {
        let tuple = SchemaBody::Tuple(
            vec![
                SchemaBody::String,
                SchemaBody::FloatingPoint(FloatWidth::W64),
            ]
            .into_boxed_slice(),
        );
        let mut builder = SchemaTableBuilder::new();
        let matrix = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(tuple.clone()),
                        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let f64_schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let f64_schema = build.resolve(f64_schema).unwrap();
        let schemas = std::sync::Arc::new(build.table);
        let value = ValueDraft {
            schema: matrix,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::Tuple(
                    vec![
                        ValueDataDraft::String("discarded sibling".to_owned()),
                        ValueDataDraft::F64(F64Bits::from_f64(7.0)),
                    ]
                    .into_boxed_slice(),
                )]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        let slot = [Some(value)];
        let value_ref = ResidentValueRef::Snapshot(&slot);
        // This is the stale root footprint that the former implementation
        // forwarded after descent. Its discarded sibling is deliberately
        // represented at the finalization boundary rather than allocated in
        // the test, so the witness isolates that admission decision.
        let root = ValueFootprint {
            encoded_bytes: 9 * 1024 * 1024,
            retained_bytes: 9 * 1024 * 1024,
            node_count: 3,
        };
        let selected = descended_collection_item_footprint(
            value_ref,
            &tuple,
            0,
            &[1],
            &mut ResidentBudgetMeter::default(),
        )
        .unwrap();

        assert!(
            admit_pattern_binding_finalization(
                f64_schema,
                &[],
                root,
                1,
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                ValueFootprint::zero(),
                0,
                0,
                &schemas,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "the discarded large sibling would exceed the finalization ceiling",
        );
        assert!(
            admit_pattern_binding_finalization(
                f64_schema,
                &[],
                selected,
                1,
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                ValueFootprint::zero(),
                0,
                0,
                &schemas,
                ResidentBudgetMeter::default(),
            )
            .is_ok(),
            "the selected scalar child itself fits",
        );
    }

    fn turn_matrix_schema() -> mech_core::Schema {
        SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![
                    DimensionExpr::Constant(1),
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                ]
                .into_boxed_slice(),
            },
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Constant(8)),
            }]
            .into_boxed_slice(),
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn generator_shape_parameters_follow_the_live_snapshot_value() {
        let schema = turn_matrix_schema();
        let mut target_builder = SchemaTableBuilder::new();
        let target = target_builder.insert(schema.clone()).unwrap();
        let target_build = target_builder.finish().unwrap();
        let target = target_build.resolve(target).unwrap();
        let target_schemas = std::sync::Arc::new(target_build.table);

        let mut source_builder = SchemaTableBuilder::new();
        source_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let source = source_builder.insert(schema).unwrap();
        let source_build = source_builder.finish().unwrap();
        let source = source_build.resolve(source).unwrap();
        let source_schemas = std::sync::Arc::new(source_build.table);
        assert_ne!(
            source, target,
            "the witness uses independent schema ordinals"
        );
        let value = ValueDraft {
            schema: source,
            shape_values: vec![3].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &source_schemas,
        ))
        .unwrap();
        let slot = [Some(value)];

        assert_eq!(
            generator_shape_values(
                ResidentValueRef::Snapshot(&slot),
                target,
                &[1],
                &target_schemas,
            )
            .unwrap()
            .as_ref(),
            [3],
            "execution must replace the stale activation-time extent",
        );
    }

    #[test]
    fn whole_component_binding_uses_its_own_shape_environment() {
        let mut builder = SchemaTableBuilder::new();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let schemas = build.table;
        let item = PatternItem::component(
            Some(tuple),
            SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()),
            Box::new([]),
            ValueDataDraft::Tuple(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
        );

        let binding = item
            .into_binding(
                tuple,
                &[37],
                &schemas,
                &StructuralProjectionTable::default(),
            )
            .unwrap()
            .expect("the component matches its binding schema");
        assert!(
            binding.shape_values.is_empty(),
            "the enclosing collection cardinality does not leak into the tuple binding",
        );
    }

    #[test]
    fn parameterized_set_output_derives_its_shape_from_deduplicated_data() {
        let mut builder = SchemaTableBuilder::new();
        let schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Set {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        cardinality: CardinalitySpec::Exact(DimensionExpr::Parameter(
                            DimensionParameterId::new(0),
                        )),
                    },
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(schema).unwrap();
        let (schemas, _) = build.into_parts();
        let data = ValueDataDraft::Set(
            [1.0, 2.0]
                .into_iter()
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                .collect(),
        );
        let shape_values = completed_set_shape_values(schemas.get(schema).unwrap(), &data).unwrap();
        assert_eq!(shape_values.as_ref(), [2]);
        let value = ValueDraft {
            schema,
            shape_values,
            data,
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        assert_eq!(value.shape().parameter_values(), [2]);
    }

    #[test]
    fn dynamic_child_descent_closes_parameterized_component_shapes() {
        let mut builder = SchemaTableBuilder::new();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::Matrix {
                            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                            dimensions: vec![
                                DimensionExpr::Constant(1),
                                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                            ]
                            .into_boxed_slice(),
                        }]
                        .into_boxed_slice(),
                    ),
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let _fixed_collision = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let component = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let component = build.resolve(component).unwrap();
        let (schemas, _) = build.into_parts();
        let (schemas, projections) = structural_projection_schema_context(&schemas).unwrap();
        let item = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: tuple,
            shape_values: vec![3].into_boxed_slice(),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::Matrix(
                    [1.0, 2.0, 3.0]
                        .into_iter()
                        .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                        .collect(),
                )]
                .into_boxed_slice(),
            ),
        }))));

        let child = item
            .child(0, &schemas, &projections)
            .expect("parameterized component");
        assert_eq!(child.structural_len(false), Some(3));
        assert!(matches!(
            child,
            PatternItem::Component {
                schema,
                body: SchemaBody::Matrix { dimensions, .. },
                shape_values,
                ..
            } if schema == Some(component)
                && dimensions.as_ref() == [DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                && shape_values.as_ref() == [3]
        ));
    }

    #[test]
    fn dynamic_atom_equality_requires_the_embedded_nominal_schema() {
        let mut builder = SchemaTableBuilder::new();
        let left = builder
            .insert(
                SchemaDraft {
                    body: atom("left"),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let right = builder
            .insert(
                SchemaDraft {
                    body: atom("right"),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let left = build.resolve(left).unwrap();
        let right = build.resolve(right).unwrap();
        let schemas = std::sync::Arc::new(build.table);

        let mut peer_builder = SchemaTableBuilder::new();
        let peer_left = peer_builder
            .insert(
                SchemaDraft {
                    body: atom("left"),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let peer_build = peer_builder.finish().unwrap();
        let peer_left = peer_build.resolve(peer_left).unwrap();
        let peer_schemas = std::sync::Arc::new(peer_build.table);
        assert_ne!(left, peer_left, "the peer uses an independent ordinal");
        let peer = ValueDraft {
            schema: peer_left,
            shape_values: Box::new([]),
            data: ValueDataDraft::Atom,
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &peer_schemas,
        ))
        .unwrap();
        let matching = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: left,
            shape_values: Box::new([]),
            data: ValueDataDraft::Atom,
        }))));
        let distinct = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: right,
            shape_values: Box::new([]),
            data: ValueDataDraft::Atom,
        }))));

        assert!(matching.atom_matches(&peer, &schemas));
        assert!(!distinct.atom_matches(&peer, &schemas));

        let mut foreign = SchemaTableBuilder::new();
        let foreign_left = foreign
            .insert(
                SchemaDraft {
                    body: atom("left"),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let foreign = foreign.finish().unwrap();
        let foreign_left = foreign.resolve(foreign_left).unwrap();
        let foreign = std::sync::Arc::new(foreign.table);
        assert_ne!(left, foreign_left, "the witness needs reordered arenas");
        let foreign_peer = ValueDraft {
            schema: foreign_left,
            shape_values: Box::new([]),
            data: ValueDataDraft::Atom,
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&foreign))
        .unwrap();
        assert!(matching.atom_matches(&foreign_peer, &schemas));
    }

    #[test]
    fn repeated_composite_bindings_use_language_equality_and_shape() {
        let mut builder = SchemaTableBuilder::new();
        let dynamic = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let matrix = |rows, columns| SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![
                    DimensionExpr::Constant(rows),
                    DimensionExpr::Constant(columns),
                ]
                .into_boxed_slice(),
            },
            dimension_parameters: Box::new([]),
        };
        let one = builder.insert(matrix(1, 1).finalize().unwrap()).unwrap();
        let row = builder.insert(matrix(1, 2).finalize().unwrap()).unwrap();
        let column = builder.insert(matrix(2, 1).finalize().unwrap()).unwrap();
        let build = builder.finish().unwrap();
        let dynamic = build.resolve(dynamic).unwrap();
        let one = build.resolve(one).unwrap();
        let row = build.resolve(row).unwrap();
        let column = build.resolve(column).unwrap();
        let schemas = std::sync::Arc::new(build.table);
        let snapshot_region = ResidentRegion {
            kind: ResidentValueKind::Snapshot,
            offset: 0,
            len: 1,
            shape: mech_core::ResidentShape::SCALAR,
        };
        let nested = |schema, values: &[f64]| ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                values
                    .iter()
                    .copied()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect(),
            ),
        };
        let peer = |schema, values: &[f64]| {
            ValueDraft {
                schema: dynamic,
                shape_values: Box::new([]),
                data: ValueDataDraft::Dynamic(Some(Box::new(nested(schema, values)))),
            }
            .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
            .unwrap()
        };
        let item = |schema, values: &[f64]| {
            PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(nested(
                schema, values,
            )))))
        };

        let signed_zero = [Some(peer(one, &[-0.0]))];
        assert_eq!(
            item(one, &[0.0])
                .language_equals(
                    ResidentValueRef::Snapshot(&signed_zero),
                    snapshot_region,
                    dynamic,
                    &[],
                    0,
                    &schemas,
                    &StructuralProjectionTable::default(),
                )
                .unwrap(),
            Some(true)
        );
        let nan = [Some(peer(one, &[f64::NAN]))];
        assert_eq!(
            item(one, &[f64::NAN])
                .language_equals(
                    ResidentValueRef::Snapshot(&nan),
                    snapshot_region,
                    dynamic,
                    &[],
                    0,
                    &schemas,
                    &StructuralProjectionTable::default(),
                )
                .unwrap(),
            Some(false)
        );
        let different_shape = [Some(peer(row, &[1.0, 2.0]))];
        assert_eq!(
            item(column, &[1.0, 2.0])
                .language_equals(
                    ResidentValueRef::Snapshot(&different_shape),
                    snapshot_region,
                    dynamic,
                    &[],
                    0,
                    &schemas,
                    &StructuralProjectionTable::default(),
                )
                .unwrap(),
            Some(false)
        );
    }

    #[test]
    fn absent_dynamic_matches_only_dynamic_peers_and_bindings() {
        let mut builder = SchemaTableBuilder::new();
        let index = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Index,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let dynamic = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let index = build.resolve(index).unwrap();
        let dynamic = build.resolve(dynamic).unwrap();
        let (schemas, projections) = structural_projection_schema_context(&build.table).unwrap();
        let schemas = Arc::new(schemas);
        let item = PatternItem::Dynamic(None);
        let peer = [7_u64];
        assert_eq!(
            item.language_equals(
                ResidentValueRef::Index(&peer),
                ResidentRegion {
                    kind: ResidentValueKind::Index,
                    offset: 0,
                    len: 1,
                    shape: mech_core::ResidentShape::SCALAR,
                },
                index,
                &[],
                0,
                &schemas,
                &projections,
            )
            .unwrap(),
            Some(false),
        );

        let binding = item
            .clone()
            .into_binding(dynamic, &[], &schemas, &projections)
            .unwrap()
            .expect("an absent Dynamic remains a valid Dynamic binding");
        assert!(matches!(binding.data, ValueDataDraft::Dynamic(None)));
        let absent = ValueDraft {
            schema: dynamic,
            shape_values: Box::new([]),
            data: ValueDataDraft::Dynamic(None),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        assert_eq!(
            item.language_equals(
                ResidentValueRef::Snapshot(&[Some(absent)]),
                ResidentRegion {
                    kind: ResidentValueKind::Snapshot,
                    offset: 0,
                    len: 1,
                    shape: mech_core::ResidentShape::SCALAR,
                },
                dynamic,
                &[],
                1_000,
                &schemas,
                &projections,
            )
            .unwrap(),
            Some(true),
        );

        let nested_absence = PatternItem::Dynamic(Some(Box::new(ValueDraft {
            schema: dynamic,
            shape_values: Box::new([]),
            data: ValueDataDraft::Dynamic(None),
        })));
        let binding = nested_absence
            .into_binding(dynamic, &[], &schemas, &projections)
            .unwrap()
            .expect("a nested absent Dynamic retains its outer wrapper");
        assert!(matches!(
            binding.data,
            ValueDataDraft::Dynamic(Some(value))
                if matches!(value.data, ValueDataDraft::Dynamic(None))
        ));
    }

    #[test]
    fn source_backed_dynamic_wrappers_resolve_before_scalar_and_shape_inspection() {
        let mut builder = SchemaTableBuilder::new();
        let dynamic = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let number = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let dynamic = build.resolve(dynamic).unwrap();
        let number = build.resolve(number).unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let schemas = Arc::new(build.table);
        let wrapped = |schema, data| {
            ValueDraft {
                schema: dynamic,
                shape_values: Box::new([]),
                data: ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                    schema,
                    shape_values: Box::new([]),
                    data,
                }))),
            }
            .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
            .unwrap()
        };
        let scalar = wrapped(number, ValueDataDraft::F64(F64Bits::from_f64(7.0)));
        let tuple = wrapped(
            tuple,
            ValueDataDraft::Tuple(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
        );
        let region = ResidentRegion {
            kind: ResidentValueKind::Snapshot,
            offset: 0,
            len: 1,
            shape: mech_core::ResidentShape::SCALAR,
        };
        let scalar_lane = [Some(scalar)];
        let scalar = resident_pattern_item(
            ResidentValueRef::Snapshot(&scalar_lane),
            region,
            dynamic,
            &[],
            &schemas,
            false,
        )
        .unwrap();
        assert!(matches!(scalar.scalar(), Some(Item::F64(value)) if value == 7.0));

        let tuple_lane = [Some(tuple)];
        let tuple = resident_pattern_item(
            ResidentValueRef::Snapshot(&tuple_lane),
            region,
            dynamic,
            &[],
            &schemas,
            false,
        )
        .unwrap();
        assert_eq!(tuple.structural_len(true), Some(1));
    }

    #[test]
    fn foreign_binding_schema_lookup_uses_the_prebuilt_key_index() {
        let schema = |body| {
            SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap()
        };
        let mut plan = SchemaTableBuilder::new();
        let plan_bool = plan.insert(schema(SchemaBody::Bool)).unwrap();
        plan.insert(schema(SchemaBody::String)).unwrap();
        let plan = plan.finish().unwrap();
        let plan_bool = plan.resolve(plan_bool).unwrap();
        let plan = Arc::new(plan.table);

        let mut source = SchemaTableBuilder::new();
        source.insert(schema(SchemaBody::String)).unwrap();
        let source_bool = source.insert(schema(SchemaBody::Bool)).unwrap();
        let source = source.finish().unwrap();
        let source_bool = source.resolve(source_bool).unwrap();
        let source = Arc::new(source.table);
        let index = source_schema_index(&source).unwrap();

        let (resolved, owner) =
            binding_schema_owner(plan_bool, &plan, Some(Arc::clone(&source)), Some(&index))
                .unwrap();
        assert_eq!(resolved, source_bool);
        assert!(Arc::ptr_eq(&owner, &source));
    }

    #[test]
    fn native_string_yield_clones_charge_every_copied_byte() {
        let payload = "yield-byte".repeat(128);
        let values = [payload.clone()];
        let schemas = SchemaTableBuilder::new().finish().unwrap().table;
        let mut meter = ResidentBudgetMeter::default();
        let (footprint, _) = retained_value_footprint(
            ResidentValueRef::String(&values),
            SchemaId::new(0),
            &schemas,
            &mut meter,
        )
        .unwrap();
        assert_eq!(footprint.retained_bytes, payload.len() as u64);
        assert_eq!(meter.estimate().compute_work(), payload.len() as u64);
    }

    #[test]
    fn matrix_results_do_not_pay_set_sorting_work() {
        assert_eq!(
            collection_canonicalization_work(crate::ComprehensionKind::Matrix, 20_000).unwrap(),
            0
        );
        assert!(
            collection_canonicalization_work(crate::ComprehensionKind::Set, 20_000).unwrap() > 0
        );
    }

    #[test]
    fn descended_absent_dynamic_footprint_measures_the_wrapper() {
        let tuple_body = SchemaBody::Tuple(vec![SchemaBody::Dynamic].into_boxed_slice());
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(tuple_body.clone()),
            dimensions: vec![DimensionExpr::Constant(1)].into_boxed_slice(),
        };
        let mut builder = SchemaTableBuilder::new();
        builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        builder
            .insert(
                SchemaDraft {
                    body: tuple_body.clone(),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let matrix = builder
            .insert(
                SchemaDraft {
                    body: matrix_body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let schemas = Arc::new(build.table);
        let value = ValueDraft {
            schema: matrix,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::Tuple(
                    vec![ValueDataDraft::Dynamic(None)].into_boxed_slice(),
                )]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        let lane = [Some(value)];

        let footprint = descended_collection_item_footprint(
            ResidentValueRef::Snapshot(&lane),
            &tuple_body,
            0,
            &[0],
            &mut ResidentBudgetMeter::default(),
        )
        .expect("an absent selected Dynamic remains measurable");
        assert!(footprint.encoded_bytes > 0);
    }

    #[test]
    fn publication_equality_work_counts_zero_width_candidate_nodes() {
        let mut builder = SchemaTableBuilder::new();
        let schema = builder
            .insert(
                SchemaDraft {
                    body: atom("node"),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(schema).unwrap();
        let (schemas, _) = build.into_parts();
        let current = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Atom,
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let candidate_nodes = 32_768;
        let work = budget::projected_language_equality_work(
            &schemas,
            &current,
            ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: 0,
                node_count: 1,
            },
            schema,
            0,
            ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: 0,
                node_count: candidate_nodes,
            },
        )
        .unwrap();
        assert!(work >= candidate_nodes);
    }

    #[test]
    fn borrowed_pattern_descent_clones_only_the_selected_child() {
        let payload = "payload".repeat(1 << 12);
        let item = PatternItem::new(ValueDataDraft::Tuple(
            vec![
                ValueDataDraft::String(payload.clone()),
                ValueDataDraft::String("unselected".repeat(1 << 12)),
            ]
            .into_boxed_slice(),
        ));
        let schemas = SchemaTableBuilder::new().finish().unwrap().table;

        let child = item
            .child(0, &schemas, &StructuralProjectionTable::default())
            .expect("tuple child");
        let PatternItem::Plain(ValueDataDraft::String(selected)) = child else {
            panic!("expected String child")
        };
        assert_eq!(selected, payload);
        assert_eq!(item.structural_len(true), Some(2));
    }

    #[test]
    fn structural_scrutinee_clone_is_admitted_before_large_materialization() {
        let payload = "x".repeat(20 * 1024 * 1024);
        let schemas = SchemaTableBuilder::new().finish().unwrap().table;
        assert!(
            admit_pattern_item_materialization(
                ResidentValueRef::String(core::slice::from_ref(&payload)),
                ResidentRegion {
                    kind: ResidentValueKind::String,
                    offset: 0,
                    len: 1,
                    shape: mech_core::ResidentShape::SCALAR,
                },
                SchemaId::new(0),
                false,
                1,
                0,
                0,
                0,
                1,
                0,
                &schemas,
            )
            .is_err(),
            "the retained source and selected-arm clone must fit before either is allocated"
        );
    }

    #[test]
    fn nested_pattern_clone_admission_counts_every_live_descent_copy() {
        let payload = "x".repeat(1024 * 1024);
        let schemas = SchemaTableBuilder::new().finish().unwrap().table;
        let value = ResidentValueRef::String(core::slice::from_ref(&payload));
        let region = ResidentRegion {
            kind: ResidentValueKind::String,
            offset: 0,
            len: 1,
            shape: mech_core::ResidentShape::SCALAR,
        };
        assert!(
            admit_pattern_item_materialization(
                value,
                region,
                SchemaId::new(0),
                false,
                1,
                0,
                0,
                0,
                1,
                0,
                &schemas,
            )
            .is_ok()
        );
        assert!(
            admit_pattern_item_materialization(
                value,
                region,
                SchemaId::new(0),
                false,
                1,
                0,
                0,
                0,
                crate::MAX_COLLECTION_PATTERN_DEPTH as u64,
                0,
                &schemas,
            )
            .is_err(),
            "every ancestor clone must fit before nested descent starts"
        );
        assert!(
            admit_pattern_item_materialization(
                value,
                region,
                SchemaId::new(0),
                false,
                32,
                0,
                0,
                0,
                1,
                0,
                &schemas,
            )
            .is_err(),
            "a shallow pattern with many sibling visits must admit every parent clone",
        );
    }

    #[test]
    fn source_backed_descent_counts_draft_and_canonical_children_per_level() {
        assert_eq!(pattern_item_copy_multiplicity(false, 3).unwrap(), 4);
        assert_eq!(pattern_item_copy_multiplicity(true, 3).unwrap(), 7);
        assert!(pattern_item_copy_multiplicity(true, u64::MAX).is_err());
    }

    #[test]
    fn source_pattern_clone_bound_includes_large_body_with_small_payload() {
        let mut body = SchemaBody::Bool;
        for _ in 0..32 {
            body = SchemaBody::Option(Box::new(body));
        }
        let tuple = SchemaBody::Tuple(vec![body].into_boxed_slice());
        let mut builder = SchemaTableBuilder::new();
        let schema = builder
            .insert(
                SchemaDraft {
                    body: tuple,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(schema).unwrap();
        let owner = Arc::new(build.table);
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::Option(mech_core::snapshot::OptionDraft {
                    present: false,
                    value: None,
                })]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&owner))
        .unwrap();
        let metadata =
            pattern_item_metadata_bound(&value, &mut ResidentBudgetMeter::default()).unwrap();
        let payload = budget::measure_canonical_value_footprint(
            &mut ResidentBudgetMeter::default(),
            &value,
            &owner,
        )
        .unwrap();
        assert!(metadata > payload.retained_bytes);
        assert!(
            metadata
                >= owner
                    .get(schema)
                    .unwrap()
                    .body()
                    .clone_allocation_bound_bytes()
                    .unwrap()
        );
    }

    #[test]
    fn snapshot_pattern_binding_and_equality_bound_canonical_finalization() {
        let mut builder = SchemaTableBuilder::new();
        let schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Set {
                        element: Box::new(SchemaBody::String),
                        cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(4)),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(schema).unwrap();
        let (schemas, _) = build.into_parts();
        let schemas = std::sync::Arc::new(schemas);
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Set(
                ["alpha", "bravo", "charlie", "delta"]
                    .map(|value| ValueDataDraft::String(value.to_owned()))
                    .into(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let lane = [Some(value.clone())];
        let region = ResidentRegion {
            kind: ResidentValueKind::Snapshot,
            offset: 0,
            len: 1,
            shape: mech_core::ResidentShape::SCALAR,
        };

        let work = admit_pattern_item_materialization(
            ResidentValueRef::Snapshot(&lane),
            region,
            schema,
            false,
            1,
            1,
            0,
            1,
            1,
            0,
            &schemas,
        )
        .expect("binding finalization is admitted before the draft is cloned");
        assert!(work > 0);
        let adapted_work = admit_pattern_item_materialization(
            ResidentValueRef::Snapshot(&lane),
            region,
            schema,
            false,
            1,
            1,
            0,
            1,
            1,
            1,
            &schemas,
        )
        .expect("a Dynamic target's expanded candidate is admitted before adaptation");
        assert!(adapted_work > work);

        let data = value.canonical_data_draft().unwrap();
        let exact = SnapshotCanonicalizationBudget::new(work);
        pattern_binding_draft(schema, &[], data.clone())
            .finalize(
                &SnapshotValidationContext::new(&schemas).with_canonicalization_budget(&exact),
            )
            .expect("the admitted allowance covers the later finalizer exactly");
        assert_eq!(exact.consumed(), work);

        let insufficient = SnapshotCanonicalizationBudget::new(work - 1);
        assert!(
            pattern_binding_draft(schema, &[], data.clone())
                .finalize(
                    &SnapshotValidationContext::new(&schemas)
                        .with_canonicalization_budget(&insufficient),
                )
                .is_err(),
            "finalization must fail closed when its admitted allowance is reduced"
        );
        let equality_work = admit_pattern_item_materialization(
            ResidentValueRef::Snapshot(&lane),
            region,
            schema,
            false,
            1,
            0,
            1,
            1,
            1,
            0,
            &schemas,
        )
        .expect("equality finalization is admitted before the draft is cloned");
        assert_eq!(equality_work, work);
        let candidate = PatternItem::new(data);
        assert_eq!(
            candidate
                .language_equals(
                    ResidentValueRef::Snapshot(&lane),
                    region,
                    schema,
                    &[],
                    equality_work,
                    &schemas,
                    &StructuralProjectionTable::default(),
                )
                .unwrap(),
            Some(true)
        );
        assert!(
            candidate
                .language_equals(
                    ResidentValueRef::Snapshot(&lane),
                    region,
                    schema,
                    &[],
                    equality_work - 1,
                    &schemas,
                    &StructuralProjectionTable::default(),
                )
                .is_err(),
            "equality reconstruction must fail closed below the admitted allowance"
        );
        assert!(
            admit_pattern_item_materialization(
                ResidentValueRef::Snapshot(&lane),
                region,
                schema,
                false,
                1,
                0,
                0,
                u64::MAX,
                1,
                0,
                &schemas,
            )
            .is_err(),
            "aggregate snapshot finalization work must be checked before finalization"
        );
    }

    #[test]
    fn dense_structural_candidates_preflight_canonical_finalization() {
        let mut builder = SchemaTableBuilder::new();
        let matrix = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(4)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let (schemas, _) = build.into_parts();
        let values = [1.0, 2.0, 3.0, 4.0];
        let region = ResidentRegion {
            kind: ResidentValueKind::F64,
            offset: 0,
            len: values.len(),
            shape: mech_core::ResidentShape::SCALAR,
        };
        let binding_work = admit_pattern_item_materialization(
            ResidentValueRef::F64(&values),
            region,
            matrix,
            true,
            1,
            1,
            0,
            1,
            1,
            0,
            &schemas,
        )
        .unwrap();
        let equality_work = admit_pattern_item_materialization(
            ResidentValueRef::F64(&values),
            region,
            matrix,
            true,
            1,
            0,
            1,
            1,
            1,
            0,
            &schemas,
        )
        .unwrap();

        assert_eq!(binding_work, values.len() as u64 + 1);
        assert_eq!(equality_work, binding_work);

        // Long strings make the immutable packed candidate, rather than the
        // retained-node ceiling, the deciding resource. The draft and its
        // cloned payload fit independently; retaining both at once does not.
        let large_count = 10_000usize;
        let large_payload = 830usize;
        let mut builder = SchemaTableBuilder::new();
        let large_matrix = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::String),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Constant(large_count as u64),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let large_matrix = build.resolve(large_matrix).unwrap();
        let (large_schemas, _) = build.into_parts();
        let large_values = vec!["x".repeat(large_payload); large_count];
        let large_region = ResidentRegion {
            kind: ResidentValueKind::String,
            offset: 0,
            len: large_values.len(),
            shape: mech_core::ResidentShape::SCALAR,
        };
        assert!(
            admit_pattern_item_materialization(
                ResidentValueRef::String(&large_values),
                large_region,
                large_matrix,
                true,
                1,
                0,
                0,
                0,
                0,
                0,
                &large_schemas,
            )
            .is_ok(),
            "the dense draft alone remains within the control budget",
        );
        assert!(
            admit_pattern_item_materialization(
                ResidentValueRef::String(&large_values),
                large_region,
                large_matrix,
                true,
                1,
                1,
                0,
                1,
                0,
                0,
                &large_schemas,
            )
            .is_err(),
            "the overlapping packed canonical candidate must also fit",
        );
    }

    #[test]
    fn snapshot_pattern_binding_drafts_keep_component_shape_parameters() {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::SignedInteger(IntegerWidth::W32)),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();

        let value = pattern_binding_draft(
            schema,
            &[3],
            ValueDataDraft::Matrix(
                vec![
                    ValueDataDraft::I32(7),
                    ValueDataDraft::I32(8),
                    ValueDataDraft::I32(9),
                ]
                .into_boxed_slice(),
            ),
        )
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        assert_eq!(value.shape().parameter_values(), [3]);
        assert!(matches!(value.data(), ValueData::Matrix(_)));
    }

    #[test]
    fn retained_snapshot_elements_account_for_packed_data_only() {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::String,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::String("retained payload".repeat(32)),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let slot = [Some(value)];

        let mut meter = ResidentBudgetMeter::default();
        let (actual, finalization_work) = retained_value_footprint(
            ResidentValueRef::Snapshot(&slot),
            schema,
            &schemas,
            &mut meter,
        )
        .unwrap();
        let mut data_meter = ResidentBudgetMeter::default();
        let expected = budget::measure_canonical_data_footprint(
            &mut data_meter,
            schemas.get(schema).unwrap().body(),
            slot[0].as_ref().unwrap().data(),
        )
        .unwrap();
        let mut full_meter = ResidentBudgetMeter::default();
        let standalone = budget::measure_canonical_value_footprint(
            &mut full_meter,
            slot[0].as_ref().unwrap(),
            &schemas,
        )
        .unwrap();

        assert_eq!(actual, expected);
        assert_eq!(finalization_work, 0);
        assert!(
            standalone.retained_bytes > actual.retained_bytes
                || standalone.node_count > actual.node_count
        );
    }

    #[test]
    fn retained_snapshot_yields_compare_schema_keys_across_arenas() {
        let schema = SchemaDraft {
            body: SchemaBody::FloatingPoint(FloatWidth::W64),
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap();
        let mut target_builder = SchemaTableBuilder::new();
        let target = target_builder.insert(schema.clone()).unwrap();
        let target_build = target_builder.finish().unwrap();
        let target = target_build.resolve(target).unwrap();
        let target_schemas = target_build.table;

        let mut source_builder = SchemaTableBuilder::new();
        source_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let source = source_builder.insert(schema).unwrap();
        let source_build = source_builder.finish().unwrap();
        let source = source_build.resolve(source).unwrap();
        let source_schemas = std::sync::Arc::new(source_build.table);
        assert_ne!(
            source, target,
            "the witness uses independent schema ordinals"
        );
        let value = ValueDraft {
            schema: source,
            shape_values: Box::new([]),
            data: ValueDataDraft::F64(F64Bits::from_f64(4.5)),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &source_schemas,
        ))
        .unwrap();
        let slot = [Some(value)];
        let resident = ResidentValueRef::Snapshot(&slot);

        retained_value_footprint(
            resident,
            target,
            &target_schemas,
            &mut ResidentBudgetMeter::default(),
        )
        .expect("equivalent external schema key is accepted");
        assert!(matches!(
            retained_item_in(
                resident,
                target,
                &SnapshotValidationContext::new(&target_schemas),
            ),
            Some(ValueDataDraft::F64(value)) if value.to_f64() == 4.5
        ));
    }

    #[test]
    fn snapshot_generator_rebinds_nested_dynamic_schema_ids() {
        let f64_schema = SchemaDraft {
            body: SchemaBody::FloatingPoint(FloatWidth::W64),
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap();
        let dynamic_schema = SchemaDraft {
            body: SchemaBody::Dynamic,
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap();
        let matrix_schema = SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Dynamic),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                    .into_boxed_slice(),
            },
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap();

        let mut target_builder = SchemaTableBuilder::new();
        let target_f64 = target_builder.insert(f64_schema.clone()).unwrap();
        let target_dynamic = target_builder.insert(dynamic_schema.clone()).unwrap();
        target_builder.insert(matrix_schema.clone()).unwrap();
        let target_build = target_builder.finish().unwrap();
        let target_f64 = target_build.resolve(target_f64).unwrap();
        let target_dynamic = target_build.resolve(target_dynamic).unwrap();
        let target_schemas = std::sync::Arc::new(target_build.table);

        let mut source_builder = SchemaTableBuilder::new();
        source_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let source_f64 = source_builder.insert(f64_schema).unwrap();
        source_builder.insert(dynamic_schema).unwrap();
        let source_matrix = source_builder.insert(matrix_schema).unwrap();
        let source_build = source_builder.finish().unwrap();
        let source_f64 = source_build.resolve(source_f64).unwrap();
        let source_matrix = source_build.resolve(source_matrix).unwrap();
        let source_schemas = std::sync::Arc::new(source_build.table);
        assert_ne!(source_f64, target_f64, "the witness uses reordered arenas");
        let source_value = ValueDraft {
            schema: source_matrix,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                    schema: source_f64,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::F64(F64Bits::from_f64(6.25)),
                })))]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &source_schemas,
        ))
        .unwrap();
        let ValueData::Matrix(matrix) = source_value.data() else {
            panic!("matrix value")
        };
        let item = sequence_item(
            matrix.elements(),
            target_dynamic,
            &SchemaBody::Dynamic,
            &[],
            0,
            &SnapshotValidationContext::with_shared_schemas(&target_schemas),
        )
        .expect("generator item");
        let PatternItem::Dynamic(Some(value)) = &item else {
            panic!("dynamic item")
        };
        assert_eq!(value.schema, target_f64);
        let binding = item
            .into_binding(
                target_f64,
                &[],
                &target_schemas,
                &StructuralProjectionTable::default(),
            )
            .unwrap()
            .expect("rebound Dynamic value matches its concrete annotation");
        assert!(matches!(binding.data, ValueDataDraft::F64(value) if value.to_f64() == 6.25));
    }

    #[test]
    fn item_clone_admission_includes_already_retained_drafts() {
        let item = ValueFootprint {
            encoded_bytes: 6 * 1024 * 1024,
            retained_bytes: 6 * 1024 * 1024,
            node_count: 1,
        };
        assert!(
            admit_item_clone(
                item,
                0,
                0,
                ValueFootprint::zero(),
                0,
                0,
                ResidentBudgetMeter::default()
            )
            .is_ok(),
            "the current item fits by itself"
        );
        let retained = ValueFootprint {
            encoded_bytes: 11 * 1024 * 1024,
            retained_bytes: 11 * 1024 * 1024,
            node_count: 1,
        };
        assert!(
            admit_item_clone(item, 0, 1, retained, 0, 0, ResidentBudgetMeter::default()).is_err(),
            "the retained draft and current clone overlap above the temporary limit"
        );
    }

    #[test]
    fn output_admission_counts_draft_and_final_node_populations() {
        let one_population = mech_core::RESIDENT_MAX_RETAINED_NODES / 2;
        let footprint = ValueFootprint {
            encoded_bytes: 0,
            retained_bytes: 0,
            node_count: one_population,
        };
        assert!(
            admit_output(
                1,
                0,
                footprint,
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "draft and finalized nodes are simultaneously live"
        );
    }

    #[test]
    fn output_workspace_is_not_double_charged() {
        let footprint = ValueFootprint {
            encoded_bytes: 4 * 1024 * 1024,
            retained_bytes: 4 * 1024 * 1024,
            node_count: 1,
        };
        let (workspace, _) = snapshot_workspace(1, footprint, 0).unwrap();
        assert!(workspace > 8 * 1024 * 1024);
        assert!(
            admit_output(
                1,
                0,
                footprint,
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_ok(),
            "one admitted workspace below the resident ceiling must not be counted twice",
        );
    }

    #[test]
    fn output_workspace_tracks_every_shape_parameter() {
        let two = snapshot_workspace(1, ValueFootprint::zero(), 2).unwrap().0;
        let three = snapshot_workspace(1, ValueFootprint::zero(), 3).unwrap().0;
        assert!(three > two);
    }

    #[test]
    fn output_admission_counts_live_lexical_payloads() {
        let footprint = ValueFootprint {
            encoded_bytes: 4 * 1024 * 1024,
            retained_bytes: 4 * 1024 * 1024,
            node_count: 1,
        };
        assert!(
            admit_output(
                1,
                1,
                footprint,
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_ok(),
            "the output fits when no lexical payload remains live",
        );
        assert!(
            admit_output(
                1,
                1,
                footprint,
                0,
                0,
                ValueFootprint {
                    encoded_bytes: 0,
                    retained_bytes: 8 * 1024 * 1024,
                    node_count: 1,
                },
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "a live lexical payload overlaps final output construction",
        );
    }

    #[test]
    fn set_draft_admission_counts_live_lexical_payloads() {
        let footprint = ValueFootprint {
            encoded_bytes: 4 * 1024 * 1024,
            retained_bytes: 4 * 1024 * 1024,
            node_count: 1,
        };
        assert!(
            admit_set_draft(
                1,
                1,
                footprint,
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_ok(),
            "the Set draft fits when no lexical payload remains live",
        );
        assert!(
            admit_set_draft(
                1,
                1,
                footprint,
                0,
                0,
                ValueFootprint {
                    encoded_bytes: 0,
                    retained_bytes: 12 * 1024 * 1024,
                    node_count: 1,
                },
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "a live lexical payload overlaps the mutable Set draft",
        );
    }

    #[test]
    fn dynamic_wrapper_footprint_includes_the_fixed_canonical_envelope() {
        let atom = atom("wrapped");
        let mut builder = SchemaTableBuilder::new();
        let atom_schema = builder
            .insert(
                SchemaDraft {
                    body: atom.clone(),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let dynamic_schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let atom_schema = build.resolve(atom_schema).unwrap();
        let dynamic_schema = build.resolve(dynamic_schema).unwrap();
        let schemas = Arc::new(build.table);
        let wrapped = ValueDraft {
            schema: dynamic_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                schema: atom_schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Atom,
            }))),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        let ValueData::Dynamic(dynamic) = wrapped.data() else {
            unreachable!()
        };
        let selected = budget::measure_canonical_data_footprint(
            &mut ResidentBudgetMeter::default(),
            &atom,
            dynamic.value().unwrap().data(),
        )
        .unwrap();
        let actual = budget::measure_canonical_data_footprint(
            &mut ResidentBudgetMeter::default(),
            &SchemaBody::Dynamic,
            wrapped.data(),
        )
        .unwrap();

        assert_eq!(dynamic_wrapped_footprint(selected, 0).unwrap(), actual);
        assert_eq!(actual.encoded_bytes, 54);
    }

    #[test]
    fn set_prefix_admission_is_draft_only_until_deduplication() {
        let count = 40_000;
        let item = scalar_footprint(8).unwrap();
        let prefix = (0..count)
            .try_fold(ValueFootprint::zero(), |total, _| total.checked_add(item))
            .unwrap();
        assert!(
            admit_set_draft(
                count,
                0,
                prefix,
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_ok(),
            "the duplicate candidate prefix fits as a mutable draft",
        );
        assert!(
            admit_output(
                count,
                0,
                prefix,
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "projecting an undeduplicated finalized Set would exceed the node ceiling",
        );
        assert!(
            admit_output(
                1,
                0,
                item,
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_ok(),
            "the deduplicated final Set fits",
        );
    }

    #[test]
    fn set_draft_admission_counts_capacity_and_reallocation_overlap() {
        let unit = core::mem::size_of::<ValueDataDraft>();
        let oversized_capacity = (mech_core::RESIDENT_MAX_BYTES as usize / unit) + 1;
        assert!(
            admit_set_draft(
                1,
                oversized_capacity,
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "spare Vec capacity remains part of the live draft allocation",
        );
        assert_eq!(
            draft_capacity_overlap_bytes(9, 8).unwrap(),
            8 * unit as u64,
            "growth admits the old backing array while the exact replacement is allocated",
        );
    }

    #[test]
    fn boxed_slice_shrink_admission_counts_both_backing_allocations() {
        let unit = core::mem::size_of::<ValueDataDraft>();
        let oversized_capacity = (mech_core::RESIDENT_MAX_BYTES as usize / unit) + 1;
        assert!(
            admit_draft_shrink(
                1,
                oversized_capacity,
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "the old Vec backing store remains live while the exact boxed slice is allocated",
        );
        assert!(
            admit_draft_shrink(
                1,
                1,
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                ResidentBudgetMeter::default(),
            )
            .is_ok(),
            "an exact-capacity Vec needs no shrink allocation",
        );
    }

    #[test]
    fn binding_finalization_counts_the_previous_snapshot() {
        let mut builder = SchemaTableBuilder::new();
        let schema = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::String,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(schema).unwrap();
        let schemas = build.table;
        let selected = ValueFootprint {
            encoded_bytes: 5 * 1024 * 1024,
            retained_bytes: 5 * 1024 * 1024,
            node_count: 1,
        };
        assert!(
            admit_pattern_binding_finalization(
                schema,
                &[],
                selected,
                1,
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                ValueFootprint::zero(),
                0,
                0,
                &schemas,
                ResidentBudgetMeter::default(),
            )
            .is_ok()
        );
        assert!(
            admit_pattern_binding_finalization(
                schema,
                &[],
                selected,
                1,
                ValueFootprint {
                    encoded_bytes: 6 * 1024 * 1024,
                    retained_bytes: 6 * 1024 * 1024,
                    node_count: 1,
                },
                ValueFootprint::zero(),
                0,
                ValueFootprint::zero(),
                0,
                0,
                &schemas,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "the prior binding, mutable draft, and immutable replacement coexist",
        );
        assert!(
            admit_pattern_binding_finalization(
                schema,
                &[],
                selected,
                1,
                ValueFootprint::zero(),
                ValueFootprint {
                    encoded_bytes: 0,
                    retained_bytes: 6 * 1024 * 1024,
                    node_count: 1,
                },
                0,
                ValueFootprint::zero(),
                0,
                0,
                &schemas,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "a live sibling lexical binding overlaps this finalization",
        );
    }

    #[test]
    fn pattern_binding_finalization_reuses_the_plan_schema_owner() {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::String,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let schemas = std::sync::Arc::new(build.table);
        let value =
            pattern_binding_draft(schema, &[], ValueDataDraft::String("shared".to_string()))
                .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
                .unwrap();
        let owner = value.schemas().expect("finalized binding retains schemas");
        assert!(std::sync::Arc::ptr_eq(&owner, &schemas));
    }

    #[test]
    fn previous_pattern_binding_footprint_uses_the_snapshot_owner_arena() {
        let mut foreign_builder = SchemaTableBuilder::new();
        let foreign_tuple = foreign_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::String, SchemaBody::Bool].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let foreign_build = foreign_builder.finish().unwrap();
        let foreign_tuple = foreign_build.resolve(foreign_tuple).unwrap();
        let foreign = Arc::new(foreign_build.table);
        let previous = ValueDraft {
            schema: foreign_tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![
                    ValueDataDraft::String("foreign binding".repeat(64)),
                    ValueDataDraft::Bool(true),
                ]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&foreign))
        .unwrap();

        let mut plan_builder = SchemaTableBuilder::new();
        let plan_bool = plan_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let plan_build = plan_builder.finish().unwrap();
        let plan_bool = plan_build.resolve(plan_bool).unwrap();
        let plan = plan_build.table;
        assert_eq!(
            foreign_tuple, plan_bool,
            "the witness reuses an arena-local id"
        );
        assert_ne!(
            foreign.entry(foreign_tuple).unwrap().key(),
            plan.entry(plan_bool).unwrap().key(),
            "the reused id denotes different schemas in each arena",
        );

        let expected = budget::measure_canonical_value_footprint(
            &mut ResidentBudgetMeter::default(),
            &previous,
            &foreign,
        )
        .unwrap();
        let actual = measure_owned_canonical_value_footprint(
            &mut ResidentBudgetMeter::default(),
            &previous,
            &plan,
        )
        .expect("a previous foreign binding is measured in its owner arena");
        assert_eq!(actual, expected);
    }

    #[test]
    fn nested_dynamic_schema_owners_are_visited_independently() {
        let f64_schema = SchemaDraft {
            body: SchemaBody::FloatingPoint(FloatWidth::W64),
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap();
        let mut plan_builder = SchemaTableBuilder::new();
        let plan_f64 = plan_builder.insert(f64_schema.clone()).unwrap();
        let plan_tuple = plan_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![SchemaBody::Dynamic].into_boxed_slice()),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let plan_build = plan_builder.finish().unwrap();
        let plan_f64 = plan_build.resolve(plan_f64).unwrap();
        let plan_tuple = plan_build.resolve(plan_tuple).unwrap();
        let plan = Arc::new(plan_build.table);

        let mut foreign_builder = SchemaTableBuilder::new();
        foreign_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let foreign_f64 = foreign_builder.insert(f64_schema).unwrap();
        let foreign_build = foreign_builder.finish().unwrap();
        let foreign_f64 = foreign_build.resolve(foreign_f64).unwrap();
        let foreign = Arc::new(foreign_build.table);
        let child = ValueDraft {
            schema: foreign_f64,
            shape_values: Box::new([]),
            data: ValueDataDraft::F64(F64Bits::from_f64(4.5)),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&foreign))
        .unwrap();
        let child_shape = child.shape().clone();
        let tuple_shape = plan
            .get(plan_tuple)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let constructor = mech_core::snapshot::CompositeSnapshotConstructor::bind(
            plan_tuple,
            tuple_shape,
            &[(plan_f64, child_shape)],
            Arc::clone(&plan),
        )
        .unwrap();
        let outer = constructor
            .construct(vec![child.clone()].into_boxed_slice(), None)
            .unwrap();

        assert!(Arc::ptr_eq(&outer.schemas().unwrap(), &plan));
        let mut saw_plan = false;
        let mut saw_foreign = false;
        visit_value_schema_owners(
            &outer,
            &mut ResidentBudgetMeter::default(),
            &mut |owner, _| {
                saw_plan |= Arc::ptr_eq(owner, &plan);
                saw_foreign |= Arc::ptr_eq(owner, &foreign);
                Ok(())
            },
        )
        .unwrap();
        assert!(saw_plan && saw_foreign);
        let (_, contexts) = source_schema_contexts(&outer, &plan).expect("source contexts");
        assert!(contexts.windows(2).all(|pair| {
            (Arc::as_ptr(&pair[0].owner) as usize) < (Arc::as_ptr(&pair[1].owner) as usize)
        }));
        assert!(Arc::ptr_eq(
            &source_context_for_owner(&contexts, &plan)
                .expect("plan owner is indexed")
                .owner,
            &plan,
        ));
        assert!(Arc::ptr_eq(
            &source_context_for_owner(&contexts, &foreign)
                .expect("foreign owner is indexed")
                .owner,
            &foreign,
        ));
        assert_eq!(
            distinct_schema_owner_footprint(
                &outer.schemas().unwrap(),
                &plan,
                2 * core::mem::size_of::<usize>() as u64,
            )
            .unwrap(),
            (0, 0),
        );
        assert!(
            distinct_schema_owner_footprint(
                &child.schemas().unwrap(),
                &plan,
                2 * core::mem::size_of::<usize>() as u64,
            )
            .unwrap()
            .0 > 0,
            "a prior output's foreign arena contributes to construction overlap",
        );
    }

    #[test]
    fn multi_owner_context_bound_charges_the_combined_binding_index() {
        let owner = |body| {
            let mut builder = SchemaTableBuilder::new();
            builder
                .insert(
                    SchemaDraft {
                        body,
                        dimension_parameters: Box::new([]),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
            builder.finish().unwrap().table
        };
        let owners = [owner(SchemaBody::Bool), owner(SchemaBody::String)];
        let mut bound = SourceContextMaterializationBound::default();
        let mut meter = ResidentBudgetMeter::default();
        for owner in &owners {
            bound.add_owner(owner, &mut meter).unwrap();
        }
        let owner_work = bound.work;
        let merged_nodes = bound.merged_schema_nodes;
        bound.add_binding_merge_work().unwrap();
        assert_eq!(merged_nodes, 2);
        assert_eq!(
            bound.work - owner_work,
            merged_nodes * (merged_nodes.ilog2() as u64 + 1)
        );
    }

    #[test]
    fn foreign_dynamic_collection_items_rebind_to_the_plan_schema_arena() {
        fn schemas(include_prefix: bool) -> (mech_core::SchemaTable, SchemaId, SchemaId) {
            let mut builder = SchemaTableBuilder::new();
            if include_prefix {
                builder
                    .insert(
                        SchemaDraft {
                            body: SchemaBody::Bool,
                            dimension_parameters: Box::new([]),
                        }
                        .finalize()
                        .unwrap(),
                    )
                    .unwrap();
            }
            let tuple = builder
                .insert(
                    SchemaDraft {
                        body: SchemaBody::Tuple(
                            vec![
                                SchemaBody::FloatingPoint(FloatWidth::W64),
                                SchemaBody::String,
                            ]
                            .into_boxed_slice(),
                        ),
                        dimension_parameters: Box::new([]),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
            let matrix = builder
                .insert(
                    SchemaDraft {
                        body: SchemaBody::Matrix {
                            element: Box::new(SchemaBody::Dynamic),
                            dimensions: vec![
                                DimensionExpr::Constant(1),
                                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                            ]
                            .into_boxed_slice(),
                        },
                        dimension_parameters: vec![DimensionParameterDeclaration {
                            id: DimensionParameterId::new(0),
                            origin: DimensionParameterOrigin::Explicit,
                            lifetime: DimensionLifetime::Turn,
                            lower_bound: DimensionExpr::Constant(0),
                            upper_bound: Some(DimensionExpr::Constant(8)),
                        }]
                        .into_boxed_slice(),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
            let build = builder.finish().unwrap();
            let tuple = build.resolve(tuple).unwrap();
            let matrix = build.resolve(matrix).unwrap();
            let (schemas, _) = build.into_parts();
            (schemas, tuple, matrix)
        }

        let (foreign, foreign_tuple, foreign_matrix) = schemas(false);
        let (plan, plan_tuple, plan_matrix) = schemas(true);
        let foreign = std::sync::Arc::new(foreign);
        let plan = std::sync::Arc::new(plan);
        assert_ne!(
            foreign_tuple, plan_tuple,
            "the witness needs reordered arenas"
        );
        let source = ValueDraft {
            schema: foreign_matrix,
            shape_values: vec![1].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                    schema: foreign_tuple,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::Tuple(
                        vec![
                            ValueDataDraft::F64(F64Bits::from_f64(7.0)),
                            ValueDataDraft::String("unselected".repeat(128)),
                        ]
                        .into_boxed_slice(),
                    ),
                })))]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&foreign))
        .unwrap();
        let ValueData::Matrix(matrix) = source.data() else {
            unreachable!()
        };
        let item = sequence_item(
            matrix.elements(),
            plan_tuple,
            &SchemaBody::Dynamic,
            &[],
            0,
            &SnapshotValidationContext::with_shared_schemas(&plan),
        )
        .expect("foreign item rebinds");
        assert!(matches!(
            item,
            PatternItem::Dynamic(Some(value)) if value.schema == plan_tuple
        ));

        let lane = [Some(source.clone())];
        admit_pattern_item_materialization(
            ResidentValueRef::Snapshot(&lane),
            ResidentRegion {
                kind: ResidentValueKind::Snapshot,
                offset: 0,
                len: 1,
                shape: mech_core::ResidentShape::SCALAR,
            },
            plan_matrix,
            false,
            1,
            0,
            0,
            0,
            1,
            0,
            &plan,
        )
        .expect("foreign scrutinee admission follows its retained schema owner");
        let mut meter = ResidentBudgetMeter::default();
        retained_value_footprint(
            ResidentValueRef::Snapshot(&lane),
            plan_matrix,
            &plan,
            &mut meter,
        )
        .expect("equivalent schemas compare by definition rather than arena ordinal");
        let retained = retained_item_in(
            ResidentValueRef::Snapshot(&lane),
            plan_matrix,
            &SnapshotValidationContext::with_shared_schemas(&plan),
        )
        .expect("retained data rebinds nested dynamic identities");
        let ValueDataDraft::Matrix(values) = retained else {
            unreachable!()
        };
        assert!(matches!(
            values.as_ref(),
            [ValueDataDraft::Dynamic(Some(value))] if value.schema == plan_tuple
        ));

        let selected = descended_collection_item_footprint(
            ResidentValueRef::Snapshot(&lane),
            &SchemaBody::Dynamic,
            0,
            &[0],
            &mut ResidentBudgetMeter::default(),
        )
        .expect("selected child footprint");
        let mech_core::snapshot::SequenceView::Values(root_values) = matrix.elements() else {
            unreachable!()
        };
        let root = budget::measure_canonical_data_footprint(
            &mut ResidentBudgetMeter::default(),
            &SchemaBody::Dynamic,
            &root_values[0],
        )
        .unwrap();
        let concrete_root = descended_collection_item_footprint(
            ResidentValueRef::Snapshot(&lane),
            &SchemaBody::Dynamic,
            0,
            &[],
            &mut ResidentBudgetMeter::default(),
        )
        .expect("root Dynamic footprint is remeasured after unwrapping");
        assert!(concrete_root.retained_bytes < root.retained_bytes);
        assert!(concrete_root.encoded_bytes < root.encoded_bytes);
        assert!(selected.retained_bytes < root.retained_bytes);
        assert!(selected.encoded_bytes < root.encoded_bytes);
    }

    #[test]
    fn structural_materialization_retains_foreign_dynamic_schema_owners() {
        let matrix = |builder: &mut SchemaTableBuilder| {
            builder
                .insert(
                    SchemaDraft {
                        body: SchemaBody::Matrix {
                            element: Box::new(SchemaBody::Dynamic),
                            dimensions: vec![
                                DimensionExpr::Constant(1),
                                DimensionExpr::Constant(1),
                            ]
                            .into_boxed_slice(),
                        },
                        dimension_parameters: Box::new([]),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap()
        };
        let mut foreign = SchemaTableBuilder::new();
        let tuple = foreign
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::FloatingPoint(FloatWidth::W64), SchemaBody::Bool]
                            .into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let foreign_matrix = matrix(&mut foreign);
        let foreign = foreign.finish().unwrap();
        let tuple = foreign.resolve(tuple).unwrap();
        let foreign_matrix = foreign.resolve(foreign_matrix).unwrap();
        let (foreign, _) = foreign.into_parts();
        let tuple_key = foreign.entry(tuple).unwrap().key();
        let foreign = std::sync::Arc::new(foreign);

        let mut plan = SchemaTableBuilder::new();
        plan.insert(
            SchemaDraft {
                body: SchemaBody::Bool,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
        let plan_matrix = matrix(&mut plan);
        let plan_adaptation = plan
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::Dynamic, SchemaBody::Dynamic].into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let plan = plan.finish().unwrap();
        let plan_matrix = plan.resolve(plan_matrix).unwrap();
        let plan_adaptation = plan.resolve(plan_adaptation).unwrap();
        let (plan, _) = plan.into_parts();
        assert!(
            plan.find_by_key(foreign.entry(tuple).unwrap().key())
                .is_none()
        );
        let (plan, projections) = structural_projection_schema_context(&plan).unwrap();
        let plan = std::sync::Arc::new(plan);

        let source = ValueDraft {
            schema: foreign_matrix,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                    schema: tuple,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::Tuple(
                        vec![
                            ValueDataDraft::F64(F64Bits::from_f64(7.0)),
                            ValueDataDraft::Bool(true),
                        ]
                        .into_boxed_slice(),
                    ),
                })))]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&foreign))
        .unwrap();
        let lane = [Some(source)];
        let item = resident_pattern_item(
            ResidentValueRef::Snapshot(&lane),
            ResidentRegion {
                kind: ResidentValueKind::Snapshot,
                offset: 0,
                len: 1,
                shape: mech_core::ResidentShape::SCALAR,
            },
            plan_matrix,
            &[1],
            &plan,
            false,
        )
        .expect("foreign Dynamic payload does not block structural fallback");
        assert_eq!(item.structural_len(false), Some(1));
        let tuple_item = item.child(0, &plan, &projections).unwrap();
        let number = tuple_item.clone().child(0, &plan, &projections).unwrap();
        assert!(matches!(number.scalar(), Some(Item::F64(value)) if value == 7.0));

        let adapted = tuple_item
            .clone()
            .into_binding(plan_adaptation, &[], &plan, &projections)
            .unwrap()
            .expect("the foreign tuple adapts to a plan-only annotated tuple");
        let adapted = finalize_pattern_binding(
            plan_adaptation,
            &adapted.shape_values,
            adapted.data,
            adapted.schemas,
            adapted.schema_index,
            &plan,
            &SnapshotCanonicalizationBudget::new(1_000_000),
        )
        .expect("the merged binding arena owns the plan-only adaptation target");
        let ValueData::Tuple(adapted) = adapted.data() else {
            panic!("plan-only adaptation is a tuple")
        };
        assert!(matches!(
            adapted.as_ref(),
            [ValueData::Dynamic(left), ValueData::Dynamic(right)]
                if matches!(left.value().map(Value::data), Some(ValueData::F64(value)) if value.to_f64() == 7.0)
                    && matches!(right.value().map(Value::data), Some(ValueData::Bool(true)))
        ));

        let dynamic = projections
            .matrix_element(plan_matrix)
            .expect("the plan precomputes its Dynamic element projection");
        let binding = tuple_item
            .into_binding(dynamic, &[1], &plan, &projections)
            .unwrap()
            .expect("the foreign tuple binds through the plan Dynamic schema");
        let finalized = finalize_pattern_binding(
            dynamic,
            &binding.shape_values,
            binding.data,
            binding.schemas,
            binding.schema_index,
            &plan,
            &SnapshotCanonicalizationBudget::new(1_000_000),
        )
        .expect("binding retains the foreign tuple while using the plan outer schema");
        assert!(std::sync::Arc::ptr_eq(&finalized.schemas().unwrap(), &plan));
        let ValueData::Dynamic(dynamic_payload) = finalized.data() else {
            panic!("plan binding is Dynamic")
        };
        let payload = dynamic_payload
            .value()
            .expect("Dynamic binding retains its payload");
        assert_eq!(payload.schema_key(), tuple_key);
        assert_eq!(
            payload
                .schemas()
                .unwrap()
                .entry(payload.schema())
                .unwrap()
                .key(),
            tuple_key
        );

        let number_binding = number
            .into_binding(dynamic, &[1], &plan, &projections)
            .unwrap()
            .expect("a projected foreign child binds through Dynamic");
        let number = finalize_pattern_binding(
            dynamic,
            &number_binding.shape_values,
            number_binding.data,
            number_binding.schemas,
            number_binding.schema_index,
            &plan,
            &SnapshotCanonicalizationBudget::new(1_000_000),
        )
        .expect("the child projection retains a concrete schema identity");
        let ValueData::Dynamic(number) = number.data() else {
            panic!("projected binding is Dynamic")
        };
        assert!(matches!(
            number.value().map(|value| value.data()),
            Some(ValueData::F64(value)) if value.to_f64() == 7.0
        ));
    }

    #[test]
    fn recursive_tuple_adaptation_rebinds_nested_dynamic_owner() {
        let mut nested_builder = SchemaTableBuilder::new();
        let number = nested_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let nested_build = nested_builder.finish().unwrap();
        let number = nested_build.resolve(number).unwrap();
        let nested = Arc::new(nested_build.table);
        let number = ValueDraft {
            schema: number,
            shape_values: Box::new([]),
            data: ValueDataDraft::F64(F64Bits::from_f64(17.0)),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&nested))
        .unwrap();
        let number_key = number.schema_key();

        let source_body =
            SchemaBody::Tuple(vec![SchemaBody::Dynamic, SchemaBody::Bool].into_boxed_slice());
        let target_body =
            SchemaBody::Tuple(vec![SchemaBody::Dynamic, SchemaBody::Dynamic].into_boxed_slice());
        let mut outer_builder = SchemaTableBuilder::new();
        let dynamic = outer_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let boolean = outer_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let source_tuple = outer_builder
            .insert(
                SchemaDraft {
                    body: source_body.clone(),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let outer_build = outer_builder.finish().unwrap();
        let dynamic = outer_build.resolve(dynamic).unwrap();
        let boolean = outer_build.resolve(boolean).unwrap();
        let source_tuple = outer_build.resolve(source_tuple).unwrap();
        let outer = Arc::new(outer_build.table);
        let wrapped = mech_core::snapshot::wrap_resident_dynamic_value(
            dynamic,
            Box::new([]),
            Arc::clone(&outer),
            Some(number),
        )
        .unwrap();
        let true_value = ValueDraft {
            schema: boolean,
            shape_values: Box::new([]),
            data: ValueDataDraft::Bool(true),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&outer))
        .unwrap();
        let tuple_shape = outer
            .get(source_tuple)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let constructor = mech_core::snapshot::CompositeSnapshotConstructor::bind(
            source_tuple,
            tuple_shape,
            &[
                (dynamic, wrapped.shape().clone()),
                (boolean, true_value.shape().clone()),
            ],
            Arc::clone(&outer),
        )
        .unwrap();
        let source = constructor
            .construct(vec![wrapped, true_value].into_boxed_slice(), None)
            .unwrap();

        let mut plan_builder = SchemaTableBuilder::new();
        plan_builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let source_id = plan_builder
            .insert(
                SchemaDraft {
                    body: source_body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let target_id = plan_builder
            .insert(
                SchemaDraft {
                    body: target_body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let plan_build = plan_builder.finish().unwrap();
        let source_id = plan_build.resolve(source_id).unwrap();
        let target_id = plan_build.resolve(target_id).unwrap();
        let (plan, projections) = structural_projection_schema_context(&plan_build.table).unwrap();
        let plan = Arc::new(plan);
        let lane = [Some(source)];
        let item = resident_pattern_item(
            ResidentValueRef::Snapshot(&lane),
            ResidentRegion {
                kind: ResidentValueKind::Snapshot,
                offset: 0,
                len: 1,
                shape: mech_core::ResidentShape::SCALAR,
            },
            source_id,
            &[],
            &plan,
            false,
        )
        .unwrap();
        let binding = item
            .into_binding(target_id, &[], &plan, &projections)
            .unwrap()
            .expect("the tuple adapts through its canonical nested child");
        let adapted = finalize_pattern_binding(
            target_id,
            &binding.shape_values,
            binding.data,
            binding.schemas,
            binding.schema_index,
            &plan,
            &SnapshotCanonicalizationBudget::new(1_000_000),
        )
        .unwrap();
        let ValueData::Tuple(children) = adapted.data() else {
            panic!("tuple")
        };
        let ValueData::Dynamic(first) = &children[0] else {
            panic!("Dynamic child")
        };
        assert_eq!(first.value().unwrap().schema_key(), number_key);
    }

    #[test]
    fn structural_descent_switches_to_each_nested_dynamic_schema_owner() {
        let mut inner = SchemaTableBuilder::new();
        let tuple = inner
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![SchemaBody::FloatingPoint(FloatWidth::W64), SchemaBody::Bool]
                            .into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let inner = inner.finish().unwrap();
        let tuple = inner.resolve(tuple).unwrap();
        let (inner, _) = inner.into_parts();
        let inner = Arc::new(inner);
        let tuple_value = ValueDraft {
            schema: tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![
                    ValueDataDraft::F64(F64Bits::from_f64(9.0)),
                    ValueDataDraft::Bool(true),
                ]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&inner))
        .unwrap();
        let tuple_key = tuple_value.schema_key();

        let mut root = SchemaTableBuilder::new();
        let dynamic = root
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let matrix = root
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Dynamic),
                        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let root = root.finish().unwrap();
        let dynamic = root.resolve(dynamic).unwrap();
        let matrix = root.resolve(matrix).unwrap();
        let (root, _) = root.into_parts();
        let root = Arc::new(root);
        assert_eq!(
            tuple.get(),
            dynamic.get(),
            "the owners intentionally reuse a local ID"
        );
        let dynamic_value = mech_core::snapshot::wrap_resident_dynamic_value(
            dynamic,
            Box::new([]),
            Arc::clone(&root),
            Some(tuple_value),
        )
        .unwrap();
        let matrix_shape = root
            .get(matrix)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let constructor = mech_core::snapshot::CompositeSnapshotConstructor::bind(
            matrix,
            matrix_shape,
            &[(dynamic, dynamic_value.shape().clone())],
            Arc::clone(&root),
        )
        .unwrap();
        let source = constructor
            .construct(vec![dynamic_value].into_boxed_slice(), None)
            .unwrap();
        let lane = [Some(source)];
        let item = resident_pattern_item(
            ResidentValueRef::Snapshot(&lane),
            ResidentRegion {
                kind: ResidentValueKind::Snapshot,
                offset: 0,
                len: 1,
                shape: mech_core::ResidentShape::SCALAR,
            },
            matrix,
            &[1],
            &root,
            false,
        )
        .expect("the root source owner is valid");
        let (_, root_projections) = structural_projection_schema_context(&root).unwrap();
        let whole = item
            .clone()
            .into_binding(matrix, &[], &root, &root_projections)
            .unwrap()
            .expect("the complete matrix matches its declared schema");
        let whole = finalize_pattern_binding(
            matrix,
            &whole.shape_values,
            whole.data,
            whole.schemas,
            whole.schema_index,
            &root,
            &SnapshotCanonicalizationBudget::new(1_000_000),
        )
        .expect("whole-composite binding rebinds every nested Dynamic owner");
        let ValueData::Matrix(whole_matrix) = whole.data() else {
            panic!("whole binding is a matrix")
        };
        let mech_core::snapshot::SequenceView::Values(whole_values) = whole_matrix.elements()
        else {
            panic!("Dynamic matrix uses canonical ValueData elements")
        };
        let ValueData::Dynamic(whole_dynamic) = &whole_values[0] else {
            panic!("matrix element remains Dynamic")
        };
        let whole_payload = whole_dynamic
            .value()
            .expect("Dynamic payload remains present");
        assert_eq!(whole_payload.schema_key(), tuple_key);
        assert!(matches!(
            whole_payload.data(),
            ValueData::Tuple(values) if values.len() == 2
        ));
        let wrapped = item
            .clone()
            .into_binding(dynamic, &[], &root, &root_projections)
            .unwrap()
            .expect("the complete foreign composite binds through Dynamic");
        let wrapped = finalize_pattern_binding(
            dynamic,
            &wrapped.shape_values,
            wrapped.data,
            wrapped.schemas,
            wrapped.schema_index,
            &root,
            &SnapshotCanonicalizationBudget::new(1_000_000),
        )
        .expect("Dynamic wrapping rebinds nested source-owner schema IDs");
        let ValueData::Dynamic(wrapped) = wrapped.data() else {
            panic!("the binding schema is Dynamic")
        };
        let wrapped = wrapped.value().expect("the wrapped matrix remains present");
        let ValueData::Matrix(wrapped) = wrapped.data() else {
            panic!("the Dynamic payload is the complete matrix")
        };
        let mech_core::snapshot::SequenceView::Values(wrapped) = wrapped.elements() else {
            panic!("the Dynamic matrix retains canonical values")
        };
        let ValueData::Dynamic(wrapped) = &wrapped[0] else {
            panic!("the matrix element remains Dynamic")
        };
        assert_eq!(
            wrapped
                .value()
                .expect("the nested payload remains present")
                .schema_key(),
            tuple_key
        );
        let tuple_item = item
            .child(0, &root, &root_projections)
            .expect("Dynamic descent switches to the nested owner's tuple schema");
        let number = tuple_item
            .child(0, &root, &root_projections)
            .expect("the nested owner supplies its own tuple projection");
        assert!(matches!(number.scalar(), Some(Item::F64(value)) if value == 9.0));
        let number = number
            .into_binding(dynamic, &[], &root, &root_projections)
            .unwrap()
            .expect("the projected nested-owner child binds through Dynamic");
        let number = finalize_pattern_binding(
            dynamic,
            &number.shape_values,
            number.data,
            number.schemas,
            number.schema_index,
            &root,
            &SnapshotCanonicalizationBudget::new(1_000_000),
        )
        .expect("the projected child ID is translated into the merged arena");
        let ValueData::Dynamic(number) = number.data() else {
            panic!("projected number binding is Dynamic")
        };
        assert!(matches!(
            number.value().map(Value::data),
            Some(ValueData::F64(value)) if value.to_f64() == 9.0
        ));
    }

    #[test]
    fn live_collection_shape_uses_the_current_turn_value() {
        let schema = SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![
                    DimensionExpr::Constant(1),
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                ]
                .into_boxed_slice(),
            },
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Constant(8)),
            }]
            .into_boxed_slice(),
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema,
            shape_values: vec![3].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let lane = [Some(value)];
        assert_eq!(
            generator_shape_values(ResidentValueRef::Snapshot(&lane), schema, &[1], &schemas,)
                .unwrap()
                .as_ref(),
            &[3]
        );
    }

    #[test]
    fn set_output_shape_is_derived_from_completed_cardinality() {
        let schema = SchemaDraft {
            body: SchemaBody::Set {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                cardinality: mech_core::CardinalitySpec::Exact(DimensionExpr::Parameter(
                    DimensionParameterId::new(0),
                )),
            },
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Constant(8)),
            }]
            .into_boxed_slice(),
        }
        .finalize()
        .unwrap();
        let data = ValueDataDraft::Set(
            [1.0, 2.0]
                .into_iter()
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                .collect(),
        );
        assert_eq!(
            completed_set_shape_values(&schema, &data).unwrap().as_ref(),
            &[2]
        );
    }
}
