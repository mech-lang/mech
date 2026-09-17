use super::super::comprehension::canonical_component_schema_id;
use super::*;
use crate::resident::budget::{self, PreparedKernel, ResidentBudgetMeter};
use crate::resident::general::{
    ActivatedCollectionStep, ActivatedComprehensionNode, ActivatedPatternBinding,
};
use mech_core::snapshot::{
    F64Bits, SnapshotCanonicalizationBudget, SnapshotValidationContext, ValueFootprint,
    canonical_snapshot_data_draft_with_context, dynamic_canonical_allocation_bound_bytes,
};
use mech_core::{
    CurrentMemoryFootprint, SchemaBody, SchemaId, Value, ValueData, ValueDataDraft, ValueDraft,
};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Item {
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

#[derive(Clone, Debug)]
enum PatternItem {
    Plain(ValueDataDraft),
    Dynamic(Option<Box<ValueDraft>>),
    Component {
        schema: SchemaId,
        body: SchemaBody,
        shape_values: Box<[u64]>,
        data: ValueDataDraft,
    },
}

struct PatternBindingItem {
    shape_values: Box<[u64]>,
    data: ValueDataDraft,
    footprint: BindingFootprint,
}

#[derive(Clone, Copy, Debug)]
enum BindingFootprint {
    Selected,
    Concrete,
    DynamicWrap { nested_shape_parameters: usize },
}

impl PatternItem {
    fn new(data: ValueDataDraft) -> Self {
        match data {
            ValueDataDraft::Dynamic(value) => Self::Dynamic(value),
            data => Self::Plain(data),
        }
    }

    fn component(
        schema: SchemaId,
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

    fn dynamic_data(value: &ValueDraft) -> Option<&ValueDataDraft> {
        match &value.data {
            ValueDataDraft::Dynamic(Some(value)) => Self::dynamic_data(value),
            ValueDataDraft::Dynamic(None) => None,
            data => Some(data),
        }
    }

    fn data(&self) -> Option<&ValueDataDraft> {
        match self {
            Self::Plain(data) => Some(data),
            Self::Dynamic(Some(value)) => Self::dynamic_data(value),
            Self::Dynamic(None) => None,
            Self::Component { data, .. } => Some(data),
        }
    }

    fn concrete_schema(&self) -> Option<SchemaId> {
        match self {
            Self::Plain(_) | Self::Dynamic(None) => None,
            Self::Component { schema, .. } => Some(*schema),
            Self::Dynamic(Some(value)) => {
                let mut value = value.as_ref();
                loop {
                    match &value.data {
                        ValueDataDraft::Dynamic(Some(next)) => value = next,
                        ValueDataDraft::Dynamic(None) => return None,
                        _ => return Some(value.schema),
                    }
                }
            }
        }
    }

    fn structural_len(&self, tuple: bool) -> Option<usize> {
        match (tuple, self.data()?) {
            (true, ValueDataDraft::Tuple(items)) | (false, ValueDataDraft::Matrix(items)) => {
                Some(items.len())
            }
            _ => None,
        }
    }

    fn child(self, index: usize, schemas: &mech_core::SchemaTable) -> Option<Self> {
        fn selected(
            schema: SchemaId,
            body: SchemaBody,
            shape_values: Box<[u64]>,
            data: ValueDataDraft,
            index: usize,
            schemas: &mech_core::SchemaTable,
        ) -> Option<PatternItem> {
            let parent = schemas.get(schema)?;
            let (open_body, body, data) = match (parent.body(), body, data) {
                (
                    SchemaBody::Tuple(open_bodies),
                    SchemaBody::Tuple(bodies),
                    ValueDataDraft::Tuple(items),
                ) => {
                    let open_body = open_bodies.get(index)?.clone();
                    let body = bodies.into_vec().into_iter().nth(index)?;
                    let data = items.into_vec().into_iter().nth(index)?;
                    (open_body, body, data)
                }
                (
                    SchemaBody::Matrix {
                        element: open_element,
                        ..
                    },
                    SchemaBody::Matrix { element, .. },
                    ValueDataDraft::Matrix(items),
                ) => {
                    let data = items.into_vec().into_iter().nth(index)?;
                    (open_element.as_ref().clone(), *element, data)
                }
                _ => return None,
            };
            let schema = canonical_component_schema_id(parent, &open_body, schemas)?;
            let component = schemas.get(schema)?;
            let shape = mech_core::shape_for_schema_components(
                component,
                &[(component.body(), body.clone())],
                None,
            )
            .ok()?;
            let _ = shape_values;
            Some(PatternItem::component(
                schema,
                body,
                shape.parameter_values().to_vec().into_boxed_slice(),
                data,
            ))
        }

        match self {
            Self::Plain(data) => match data {
                ValueDataDraft::Tuple(items) | ValueDataDraft::Matrix(items) => {
                    items.into_vec().into_iter().nth(index).map(Self::new)
                }
                _ => None,
            },
            Self::Dynamic(Some(mut value)) => loop {
                let ValueDraft {
                    schema,
                    shape_values,
                    data,
                } = *value;
                let shape = schemas.get(schema)?.instantiate_shape(shape_values).ok()?;
                let body = schemas.get(schema)?.closed_body(&shape).ok()?;
                match (body, data) {
                    (SchemaBody::Dynamic, ValueDataDraft::Dynamic(Some(next))) => value = next,
                    (SchemaBody::Dynamic, ValueDataDraft::Dynamic(None)) => return None,
                    (body, data) => {
                        return selected(
                            schema,
                            body,
                            shape.parameter_values().to_vec().into_boxed_slice(),
                            data,
                            index,
                            schemas,
                        );
                    }
                }
            },
            Self::Dynamic(None) => None,
            Self::Component {
                schema,
                body,
                shape_values,
                data,
            } => selected(schema, body, shape_values, data, index, schemas),
        }
    }

    /// Bounds the schema closure and shape-copy storage used while a
    /// structural pattern descends through Dynamic values. The executor walks
    /// the same path once to materialize the PatternItem and again to measure
    /// the selected retained footprint, so both passes must be admitted before
    /// the first `closed_body` call.
    fn dynamic_descent_workspace(
        &self,
        path: &[usize],
        schemas: &mech_core::SchemaTable,
    ) -> Result<u64, ResidentKernelError> {
        enum Cursor<'a> {
            Plain(&'a ValueDataDraft),
            Typed {
                body: &'a SchemaBody,
                data: &'a ValueDataDraft,
            },
            Dynamic(Option<&'a ValueDraft>),
        }

        fn resolve_dynamic<'a>(
            mut cursor: Cursor<'a>,
            schemas: &'a mech_core::SchemaTable,
            workspace: &mut u64,
        ) -> Result<Option<Cursor<'a>>, ResidentKernelError> {
            loop {
                let Cursor::Dynamic(value) = cursor else {
                    return Ok(Some(cursor));
                };
                let Some(value) = value else {
                    return Ok(None);
                };
                let schema = schemas
                    .get(value.schema)
                    .ok_or(ResidentKernelError::InvalidInput)?;
                let shape_bytes = budget::checked_u64(value.shape_values.len())?
                    .checked_mul(core::mem::size_of::<u64>() as u64)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let body_bytes = schema
                    .body()
                    .clone_allocation_bound_bytes()
                    .ok_or(ResidentKernelError::InvalidShape)?;
                *workspace = workspace
                    .checked_add(shape_bytes)
                    .and_then(|bytes| bytes.checked_add(body_bytes))
                    .ok_or(ResidentKernelError::InvalidShape)?;
                cursor = match (schema.body(), &value.data) {
                    (SchemaBody::Dynamic, ValueDataDraft::Dynamic(next)) => {
                        Cursor::Dynamic(next.as_deref())
                    }
                    (body, data) => Cursor::Typed { body, data },
                };
            }
        }

        fn child<'a>(cursor: Cursor<'a>, index: usize) -> Option<Cursor<'a>> {
            match cursor {
                Cursor::Plain(data) => match data {
                    ValueDataDraft::Tuple(items) | ValueDataDraft::Matrix(items) => {
                        Some(Cursor::Plain(items.get(index)?))
                    }
                    _ => None,
                },
                Cursor::Typed { body, data } => match (body, data) {
                    (SchemaBody::Tuple(fields), ValueDataDraft::Tuple(items)) => {
                        let body = fields.get(index)?;
                        let data = items.get(index)?;
                        if matches!(body, SchemaBody::Dynamic) {
                            let ValueDataDraft::Dynamic(value) = data else {
                                return None;
                            };
                            Some(Cursor::Dynamic(value.as_deref()))
                        } else {
                            Some(Cursor::Typed { body, data })
                        }
                    }
                    (SchemaBody::Matrix { element, .. }, ValueDataDraft::Matrix(items)) => {
                        let body = element.as_ref();
                        let data = items.get(index)?;
                        if matches!(body, SchemaBody::Dynamic) {
                            let ValueDataDraft::Dynamic(value) = data else {
                                return None;
                            };
                            Some(Cursor::Dynamic(value.as_deref()))
                        } else {
                            Some(Cursor::Typed { body, data })
                        }
                    }
                    _ => None,
                },
                Cursor::Dynamic(_) => None,
            }
        }

        let mut cursor = match self {
            Self::Plain(data) => Cursor::Plain(data),
            Self::Dynamic(value) => Cursor::Dynamic(value.as_deref()),
            Self::Component { body, data, .. } => Cursor::Typed { body, data },
        };
        let mut workspace = 0_u64;
        for index in path {
            let Some(resolved) = resolve_dynamic(cursor, schemas, &mut workspace)? else {
                return Ok(workspace
                    .checked_mul(2)
                    .ok_or(ResidentKernelError::InvalidShape)?);
            };
            let Some(next) = child(resolved, *index) else {
                return Ok(workspace
                    .checked_mul(2)
                    .ok_or(ResidentKernelError::InvalidShape)?);
            };
            cursor = next;
        }
        // Component selection also clones the open and closed bodies, builds a
        // canonical component schema, solves its shape, and closes that schema
        // again. A clone of the complete schema context is a conservative
        // allocation bound for that one-component construction, including
        // dimension declarations and their expression trees. Only one such
        // construction is live at a time while the consumed parent item is
        // replaced by its selected child.
        if !path.is_empty() {
            workspace = workspace
                .checked_add(
                    schemas
                        .clone_allocation_bound_bytes()
                        .and_then(|bytes| bytes.checked_mul(2))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )
                .ok_or(ResidentKernelError::InvalidShape)?;
        }
        // The retained-footprint pass resolves a Dynamic at the selected leaf
        // even though `PatternItem::child` defers that final resolution.
        let _ = resolve_dynamic(cursor, schemas, &mut workspace)?;
        workspace
            .checked_mul(2)
            .ok_or(ResidentKernelError::InvalidShape)
    }

    fn binding_resolution_workspace(
        &self,
        binding_schema: SchemaId,
        source_shape_values: &[u64],
        schemas: &mech_core::SchemaTable,
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
        let body_bytes = binding
            .body()
            .clone_allocation_bound_bytes()
            .ok_or(ResidentKernelError::InvalidShape)?;
        // Schema entries are borrowed throughout binding resolution. Charge
        // only the bodies, witness buffers, and parameter-value boxes built by
        // `into_binding`; charging the complete arena here would multiply an
        // unrelated retained owner by every generator iteration.
        let mut workspace = body_bytes
            .checked_mul(3)
            .and_then(|bytes| bytes.checked_add(shape_bytes.checked_mul(6)?))
            .ok_or(ResidentKernelError::InvalidShape)?;
        match self {
            Self::Plain(_) | Self::Dynamic(None) => {}
            Self::Component {
                body, shape_values, ..
            } => {
                let component_shape_bytes = budget::checked_u64(shape_values.len())?
                    .checked_mul(core::mem::size_of::<u64>() as u64)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                workspace = workspace
                    .checked_add(
                        body.clone_allocation_bound_bytes()
                            .and_then(|bytes| bytes.checked_mul(2))
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                    .and_then(|bytes| bytes.checked_add(component_shape_bytes.checked_mul(2)?))
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
            Self::Dynamic(Some(value)) => {
                let mut current = Some(value.as_ref());
                while let Some(value) = current {
                    let schema = schemas
                        .get(value.schema)
                        .ok_or(ResidentKernelError::InvalidInput)?;
                    let dynamic_shape_bytes = budget::checked_u64(value.shape_values.len())?
                        .checked_mul(core::mem::size_of::<u64>() as u64)
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    workspace = workspace
                        .checked_add(
                            schema
                                .body()
                                .clone_allocation_bound_bytes()
                                .and_then(|bytes| bytes.checked_mul(2))
                                .ok_or(ResidentKernelError::InvalidShape)?,
                        )
                        .and_then(|bytes| bytes.checked_add(dynamic_shape_bytes.checked_mul(2)?))
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    if matches!(schema.body(), SchemaBody::Dynamic) {
                        let ValueDataDraft::Dynamic(next) = &value.data else {
                            return Err(ResidentKernelError::InvalidInput);
                        };
                        current = next.as_deref();
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(workspace)
    }

    fn into_binding(
        self,
        binding_schema: SchemaId,
        source_shape_values: &[u64],
        schemas: &mech_core::SchemaTable,
    ) -> Result<Option<PatternBindingItem>, ResidentKernelError> {
        let binding = schemas
            .get(binding_schema)
            .ok_or(ResidentKernelError::InvalidInput)?;
        if matches!(binding.body(), SchemaBody::Dynamic) {
            let binding_shape = binding
                .instantiate_shape(Box::new([]))
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            let (data, footprint) = match self {
                Self::Dynamic(value) => {
                    (ValueDataDraft::Dynamic(value), BindingFootprint::Selected)
                }
                Self::Component {
                    schema,
                    body,
                    shape_values,
                    data,
                } => {
                    let selected = schemas
                        .get(schema)
                        .ok_or(ResidentKernelError::InvalidInput)?;
                    let shape = selected
                        .instantiate_shape(shape_values.clone())
                        .map_err(|_| ResidentKernelError::InvalidInput)?;
                    if selected
                        .closed_body(&shape)
                        .map_err(|_| ResidentKernelError::InvalidInput)?
                        != body
                    {
                        return Err(ResidentKernelError::InvalidInput);
                    }
                    let nested_shape_parameters = shape_values.len();
                    (
                        ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                            schema,
                            shape_values,
                            data,
                        }))),
                        BindingFootprint::DynamicWrap {
                            nested_shape_parameters,
                        },
                    )
                }
                Self::Plain(_) => return Err(ResidentKernelError::InvalidInput),
            };
            return Ok(Some(PatternBindingItem {
                shape_values: binding_shape.parameter_values().to_vec().into_boxed_slice(),
                data,
                footprint,
            }));
        }

        let (actual_body, data) = match self {
            Self::Plain(data) => {
                return Ok(Some(PatternBindingItem {
                    shape_values: source_shape_values.to_vec().into_boxed_slice(),
                    data,
                    footprint: BindingFootprint::Selected,
                }));
            }
            Self::Component { body, data, .. } => (body, data),
            Self::Dynamic(Some(mut value)) => loop {
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
                        break (body, data);
                    }
                }
            },
            Self::Dynamic(None) => return Ok(None),
        };
        let Ok(binding_shape) = mech_core::shape_for_schema_components(
            binding,
            &[(binding.body(), actual_body.clone())],
            None,
        ) else {
            return Ok(None);
        };
        if binding
            .closed_body(&binding_shape)
            .map_err(|_| ResidentKernelError::InvalidInput)?
            != actual_body
        {
            return Ok(None);
        }
        Ok(Some(PatternBindingItem {
            shape_values: binding_shape.parameter_values().to_vec().into_boxed_slice(),
            data,
            footprint: BindingFootprint::Concrete,
        }))
    }

    fn equals_resident(
        &self,
        peer: ResidentValueRef<'_>,
        schemas: &mech_core::SchemaTable,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<Option<bool>, ResidentKernelError> {
        let Some(data) = self.data() else {
            // An absent Dynamic value is a valid value of the generator's
            // layout, but it cannot equal a present scalar/snapshot peer.
            // Reserve `None` for genuinely unsupported equality layouts.
            return Ok(matches!(self, Self::Dynamic(_)).then_some(false));
        };
        Ok(match peer {
            ResidentValueRef::Bool([peer]) => {
                Some(matches!(data, ValueDataDraft::Bool(value) if value == &(*peer != 0)))
            }
            ResidentValueRef::Index([peer]) => {
                Some(matches!(data, ValueDataDraft::Index(value) if value == peer))
            }
            ResidentValueRef::F64([peer]) => {
                Some(matches!(data, ValueDataDraft::F64(value) if value.to_f64() == *peer))
            }
            ResidentValueRef::String([peer]) => {
                if let ValueDataDraft::String(value) = data {
                    meter.charge_comparison_work(budget::checked_u64(
                        value.len().max(peer.len()),
                    )?)?;
                }
                Some(matches!(data, ValueDataDraft::String(value) if value == peer))
            }
            ResidentValueRef::Snapshot([Some(peer)]) => {
                let Some(schema) = self.concrete_schema() else {
                    return Ok(None);
                };
                let Some(entry) = schemas.entry(schema) else {
                    return Ok(None);
                };
                if entry.key() != peer.schema_key() {
                    return Ok(Some(false));
                }
                draft_leaf_language_eq(data, peer.data(), meter)?
            }
            _ => None,
        })
    }
}

fn draft_leaf_language_eq(
    draft: &ValueDataDraft,
    value: &ValueData,
    meter: &mut ResidentBudgetMeter,
) -> Result<Option<bool>, ResidentKernelError> {
    macro_rules! ordinary {
        ($variant:ident) => {
            if let (ValueDataDraft::$variant(left), ValueData::$variant(right)) = (draft, value) {
                return Ok(Some(left == right));
            }
        };
    }
    ordinary!(U8);
    ordinary!(U16);
    ordinary!(U32);
    ordinary!(U64);
    ordinary!(U128);
    ordinary!(I8);
    ordinary!(I16);
    ordinary!(I32);
    ordinary!(I64);
    ordinary!(I128);
    ordinary!(Bool);
    ordinary!(Id);
    ordinary!(Index);
    Ok(match (draft, value) {
        (ValueDataDraft::F32(left), ValueData::F32(right)) => Some(left.to_f32() == right.to_f32()),
        (ValueDataDraft::F64(left), ValueData::F64(right)) => Some(left.to_f64() == right.to_f64()),
        (ValueDataDraft::Complex32(left), ValueData::Complex32(right)) => Some(
            left.real().to_f32() == right.real().to_f32()
                && left.imaginary().to_f32() == right.imaginary().to_f32(),
        ),
        (ValueDataDraft::Complex64(left), ValueData::Complex64(right)) => Some(
            left.real().to_f64() == right.real().to_f64()
                && left.imaginary().to_f64() == right.imaginary().to_f64(),
        ),
        (
            ValueDataDraft::Rational64 {
                numerator,
                denominator,
            },
            ValueData::Rational64(right),
        ) => Some(*numerator == right.numerator() && *denominator == right.denominator()),
        (ValueDataDraft::String(left), ValueData::String(right)) => {
            meter.charge_comparison_work(budget::checked_u64(left.len().max(right.len()))?)?;
            Some(left.as_str() == right.as_ref())
        }
        (ValueDataDraft::Atom, ValueData::Atom) => Some(true),
        _ => None,
    })
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
        element_schema,
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
                        element_schema,
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
        element_schema,
        element.clone(),
        element_shape_values.to_vec().into_boxed_slice(),
        data,
    ))
}

fn dense_collection_offset(region: ResidentRegion, ordinal: usize) -> Option<usize> {
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
            // The native lane is cloned into a PatternItem immediately after
            // this preflight. Account for every copied byte even when a later
            // filter discards the path and no result is retained.
            meter.charge_compute_work(budget::checked_u64(value.len())?)?;
            scalar_footprint(value.len())?
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
    };
    // Fixed-width native scalar lanes need no recursive traversal beyond the
    // generator visit. Native String clone work and recursive snapshot
    // traversal are charged above.
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
    schemas: &mech_core::SchemaTable,
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
    // The schema-context clone bound includes parameter declarations and
    // their lower/upper expression trees. The remaining terms cover the open
    // and closed body copies plus lower bounds, witnesses, ShapeInstance
    // storage, and the retained parameter-value box.
    schemas
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
    live_bytes: u64,
    live_nodes: u64,
    meter: &mut ResidentBudgetMeter,
) -> Result<(Box<[u64]>, u64), ResidentKernelError> {
    let ResidentValueRef::Snapshot([Some(value)]) = value else {
        let retained_bytes = budget::checked_u64(activation_values.len())?
            .checked_mul(core::mem::size_of::<u64>() as u64)
            .ok_or(ResidentKernelError::InvalidShape)?;
        admit_generator_schema_workspace(retained_bytes, live_bytes, live_nodes, meter)?;
        return Ok((
            activation_values.to_vec().into_boxed_slice(),
            retained_bytes,
        ));
    };
    let source_schemas = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
    let source_schema = value
        .validate_against(&source_schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let target_schema = schemas
        .get(schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let source_body_bytes = source_schema
        .body()
        .clone_allocation_bound_bytes()
        .ok_or(ResidentKernelError::InvalidShape)?;
    let target_body_bytes = target_schema
        .body()
        .clone_allocation_bound_bytes()
        .ok_or(ResidentKernelError::InvalidShape)?;
    let source_shape_bytes = budget::checked_u64(source_schema.dimension_parameters().len())?
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let target_shape_bytes = budget::checked_u64(target_schema.dimension_parameters().len())?
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let workspace = source_body_bytes
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(target_body_bytes.checked_mul(2)?))
        .and_then(|bytes| bytes.checked_add(source_shape_bytes.checked_mul(2)?))
        .and_then(|bytes| bytes.checked_add(target_shape_bytes.checked_mul(3)?))
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_generator_schema_workspace(workspace, live_bytes, live_nodes, meter)?;
    let source_body = source_schema
        .closed_body(value.shape())
        .map_err(|_| ResidentKernelError::InvalidInput)?;
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
    let values = shape.parameter_values().to_vec().into_boxed_slice();
    let retained_bytes = budget::checked_u64(values.len())?
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((values, retained_bytes))
}

fn generator_element(
    source_schema: SchemaId,
    element_schema: SchemaId,
    source_shape_values: &[u64],
    schemas: &mech_core::SchemaTable,
    live_bytes: u64,
    live_nodes: u64,
    meter: &mut ResidentBudgetMeter,
) -> Result<(SchemaBody, Box<[u64]>, u64), ResidentKernelError> {
    let source = schemas
        .get(source_schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let component = schemas
        .get(element_schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let source_body_bytes = source
        .body()
        .clone_allocation_bound_bytes()
        .ok_or(ResidentKernelError::InvalidShape)?;
    let component_body_bytes = component
        .body()
        .clone_allocation_bound_bytes()
        .ok_or(ResidentKernelError::InvalidShape)?;
    let source_shape_bytes = budget::checked_u64(source_shape_values.len())?
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let component_shape_bytes = budget::checked_u64(component.dimension_parameters().len())?
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    // Closing the source, resolving the component shape, and closing the
    // component can transiently own the selected body, its resolver copy,
    // witness storage, and both old/new shape buffers at once. SchemaBody's
    // core-owned clone bound is also the closure allocation bound.
    let workspace = source_body_bytes
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(component_body_bytes.checked_mul(2)?))
        .and_then(|bytes| bytes.checked_add(source_shape_bytes))
        .and_then(|bytes| bytes.checked_add(component_shape_bytes.checked_mul(3)?))
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_generator_schema_workspace(workspace, live_bytes, live_nodes, meter)?;
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
    let shape = mech_core::shape_for_schema_components(
        component,
        &[(component.body(), actual.clone())],
        None,
    )
    .map_err(|_| ResidentKernelError::InvalidShape)?;
    if component
        .closed_body(&shape)
        .map_err(|_| ResidentKernelError::InvalidShape)?
        != actual
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let shape_values = shape.parameter_values().to_vec().into_boxed_slice();
    let shape_values_bytes = budget::checked_u64(shape_values.len())?
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_bytes = actual
        .clone_allocation_bound_bytes()
        .and_then(|bytes| bytes.checked_add(shape_values_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((actual, shape_values, retained_bytes))
}

fn retained_item(
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

fn pattern_binding_draft(
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
    schema_arena_bytes: u64,
    published_output_bytes: u64,
    live_locals: ValueFootprint,
    meter: ResidentBudgetMeter,
) -> Result<(u64, u64), ResidentKernelError> {
    let live_draft_bytes = snapshot_draft_bytes(count, footprint, shape_parameter_count)?
        .checked_add(draft_capacity_overlap_bytes(count, current_capacity)?)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let live_bytes = meter
        .estimate()
        .temporary_bytes()
        .checked_add(schema_arena_bytes)
        .and_then(|bytes| bytes.checked_add(published_output_bytes))
        .and_then(|bytes| bytes.checked_add(live_draft_bytes))
        .and_then(|bytes| bytes.checked_add(live_locals.retained_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let live_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(footprint.node_count)
        .and_then(|nodes| nodes.checked_add(live_locals.node_count))
        .and_then(|nodes| nodes.checked_add(u64::from(count > 0)))
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((live_bytes, live_nodes))
}

fn admit_output(
    count: usize,
    current_capacity: usize,
    footprint: ValueFootprint,
    shape_parameter_count: usize,
    schema_arena_bytes: u64,
    live_locals: ValueFootprint,
    published_output_bytes: u64,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    let (temporary, output) = snapshot_workspace(count, footprint, shape_parameter_count)?;
    let temporary = temporary
        .checked_add(draft_capacity_overlap_bytes(count, current_capacity)?)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(live_locals.retained_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(schema_arena_bytes)
        .and_then(|bytes| bytes.checked_add(published_output_bytes))
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
    published_output_bytes: u64,
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
        .and_then(|bytes| bytes.checked_add(published_output_bytes))
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
    published_output_bytes: u64,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    let temporary = snapshot_draft_bytes(count, footprint, shape_parameter_count)?
        .checked_add(draft_capacity_overlap_bytes(count, current_capacity)?)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(schema_arena_bytes)
        .and_then(|bytes| bytes.checked_add(live_locals.retained_bytes))
        .and_then(|bytes| bytes.checked_add(published_output_bytes))
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
        ResidentValueRef::String([value]) => scalar_footprint(value.len())?,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    // Native scalar yields need no recursive traversal beyond the control
    // work charged by the caller.
    Ok((footprint, 0))
}

fn admit_item_clone(
    item: ValueFootprint,
    retained_count: usize,
    retained_capacity: usize,
    retained: ValueFootprint,
    retained_shape_parameter_count: usize,
    schema_arena_bytes: u64,
    live_locals: ValueFootprint,
    published_output_bytes: u64,
    meter: ResidentBudgetMeter,
) -> Result<(), ResidentKernelError> {
    let (temporary, retained_nodes) = item_clone_live_demand(
        item,
        retained_count,
        retained_capacity,
        retained,
        retained_shape_parameter_count,
        schema_arena_bytes,
        live_locals,
        published_output_bytes,
        meter,
    )?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {
            temporary_bytes: temporary,
            cloned_bytes: item.retained_bytes,
            retained_nodes,
            ..meter.estimate()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn item_clone_live_demand(
    item: ValueFootprint,
    retained_count: usize,
    retained_capacity: usize,
    retained: ValueFootprint,
    retained_shape_parameter_count: usize,
    schema_arena_bytes: u64,
    live_locals: ValueFootprint,
    published_output_bytes: u64,
    meter: ResidentBudgetMeter,
) -> Result<(u64, u64), ResidentKernelError> {
    let item_temporary = item_clone_bytes(item)?;
    let temporary = snapshot_draft_bytes(retained_count, retained, retained_shape_parameter_count)?
        .checked_add(draft_capacity_overlap_bytes(
            retained_count,
            retained_capacity,
        )?)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(item_temporary)
        .and_then(|bytes| bytes.checked_add(schema_arena_bytes))
        .and_then(|bytes| bytes.checked_add(live_locals.retained_bytes))
        .and_then(|bytes| bytes.checked_add(published_output_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(retained.node_count)
        .and_then(|nodes| nodes.checked_add(item.node_count))
        .and_then(|nodes| nodes.checked_add(live_locals.node_count))
        .and_then(|nodes| nodes.checked_add(u64::from(retained_count > 0)))
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((temporary, retained_nodes))
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
    // The Dynamic canonical envelope retains its tag, 32-byte schema key,
    // two length prefixes, and the five-byte shape header even when the
    // selected payload itself has no encoded bytes.
    let canonical =
        dynamic_canonical_allocation_bound_bytes(item.encoded_bytes, nested_shape_parameters)
            .ok_or(ResidentKernelError::InvalidShape)?;
    Ok(ValueFootprint {
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
    .map_err(|_| ResidentKernelError::InvalidShape)?)
}

fn admit_pattern_binding_finalization(
    schema: SchemaId,
    shape_values: &[u64],
    item: ValueFootprint,
    previous_binding: ValueFootprint,
    other_live_locals: ValueFootprint,
    retained_count: usize,
    retained_capacity: usize,
    retained: ValueFootprint,
    retained_shape_parameter_count: usize,
    schema_arena_bytes: u64,
    published_output_bytes: u64,
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
        .checked_add(draft_capacity_overlap_bytes(
            retained_count,
            retained_capacity,
        )?)
        .ok_or(ResidentKernelError::InvalidShape)?
        .checked_add(item_clone_bytes(item)?)
        .and_then(|bytes| bytes.checked_add(finalization))
        .and_then(|bytes| bytes.checked_add(previous_binding.retained_bytes))
        .and_then(|bytes| bytes.checked_add(other_live_locals.retained_bytes))
        .and_then(|bytes| bytes.checked_add(schema_arena_bytes))
        .and_then(|bytes| bytes.checked_add(published_output_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = meter
        .estimate()
        .retained_nodes()
        .checked_add(retained.node_count)
        .and_then(|nodes| nodes.checked_add(item.node_count))
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

#[cfg(test)]
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
) -> Result<bool, ResidentKernelError> {
    // Equal arenas prove every ordinary root is already addressable without
    // constructing canonical component schemas. Foreign arenas and Dynamic
    // roots need the rooted check below: their complete closure may already
    // exist in the plan even when unrelated entries or ordering differ.
    if same_schema_arena_contents(owner, plan, meter)? && !nested_dynamic {
        return Ok(false);
    }
    // Computing canonical component keys transiently materializes each child
    // schema. Bound that inspection from the root before constructing any of
    // those values; a component-closed plan then avoids the much larger arena
    // clone and merge while an incomplete plan still falls through safely.
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
            temporary_bytes: construction_bytes,
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
    if kind == crate::ComprehensionKind::Matrix {
        return Ok(0);
    }
    (count as u64)
        .checked_mul((count.max(1).ilog2() as u64 + 1) * 64)
        .and_then(|work| work.checked_add(64))
        .ok_or(ResidentKernelError::InvalidShape)
}

impl ReactiveInstance {
    pub(super) fn comprehension_live_local_footprint(
        &self,
        locals: &[ResidentRegion],
        excluded: Option<ResidentRegion>,
        schemas: &mech_core::SchemaTable,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<ValueFootprint, ResidentKernelError> {
        let mut footprint = ValueFootprint::zero();
        for local in locals
            .iter()
            .copied()
            .filter(|local| Some(*local) != excluded)
        {
            match self.workspace.scratch.read(local) {
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
                        let local = budget::measure_canonical_value_footprint(
                            &mut local_meter,
                            value,
                            schemas,
                        )?;
                        meter.charge_comparison_work(local_meter.estimate().comparison_work())?;
                        footprint = footprint
                            .checked_add(local)
                            .map_err(|_| ResidentKernelError::InvalidShape)?;
                    }
                }
                ResidentValueRef::Bool(_)
                | ResidentValueRef::Index(_)
                | ResidentValueRef::F64(_) => {}
            }
        }
        Ok(footprint)
    }

    fn comprehension_schema_arena(
        &self,
        control: &ActivatedComprehensionNode,
        working: InstanceEpoch,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<(Arc<mech_core::SchemaTable>, u64), ResidentKernelError> {
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
            return Ok((Arc::clone(&self.plan.schemas), 0));
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
        for owner in &owners {
            let remaining = meter.estimate().remaining_incremental_work()?;
            let closure_budget = SnapshotCanonicalizationBudget::new(remaining);
            let (owner_allocation, owner_construction, owner_nodes) = owner
                .owner
                .component_closure_bounds_for_roots_with_budget(
                    owner.roots(&roots),
                    &closure_budget,
                )
                .ok_or(ResidentKernelError::InvalidShape)?;
            meter.charge_comparison_work(closure_budget.consumed())?;
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
        let (previous_arena_bytes, previous_arena_nodes) =
            if let Some(previous_owner) = &previous_owner {
                if owners
                    .iter()
                    .any(|owner| Arc::ptr_eq(&owner.owner, previous_owner))
                {
                    (0, 0)
                } else {
                    distinct_schema_owner_footprint(
                        previous_owner,
                        &self.plan.schemas,
                        shared_owner_bytes,
                    )?
                }
            } else {
                (0, 0)
            };
        let peak = allocation_bound
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(closure_peak))
            .and_then(|bytes| bytes.checked_add(previous_arena_bytes))
            .and_then(|bytes| bytes.checked_add(owner_index_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.charge_compute_work(allocation_bound)?;
        meter.charge_comparison_work(
            retained_nodes
                .checked_mul(retained_nodes)
                .and_then(|work| work.checked_mul(key_bytes))
                .and_then(|work| work.checked_add(allocation_bound))
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
        // Closure and merge can overlap the source entries, closed entries,
        // merge index, prior merged table, and replacement table. The final
        // arena retains only one population, charged into `meter` below.
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
        for owner in &owners {
            let closed = owner
                .owner
                .component_closure_for_roots(owner.roots(&roots))
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            merged = merged
                .extend_preserving_ids(&closed)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
        }
        let retained = merged
            .clone_allocation_bound_bytes()
            .and_then(|bytes| bytes.checked_add(shared_owner_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok((Arc::new(merged), retained))
    }

    pub(super) fn execute_comprehension(
        &mut self,
        index: ActivatedNodeIndex,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<bool, ResidentExecutionError> {
        let ActivatedTurnStep::Comprehension(control) = &self.plan.steps[index.get() as usize]
        else {
            unreachable!()
        };
        let control = control.clone();
        let result = budget::with_control_work_budget(|| {
            self.with_kernel_turn_plan(index, before, working, |this| {
                this.execute_collection_planned(index, &control, before, working, probe)
            })
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
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel {
            node: control.artifact_node,
            error,
        };
        let mut meter = ResidentBudgetMeter::default();
        let (schemas, schema_arena_bytes) = self
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
        let mut values = Vec::new();
        let mut footprint = ValueFootprint::zero();
        let mut nested_finalization_work = 0_u64;
        self.collection_from(
            control,
            0,
            &mut values,
            &mut footprint,
            &mut nested_finalization_work,
            &mut meter,
            &schemas,
            schema_arena_bytes,
            published_output_footprint.retained_bytes,
            before,
            working,
            probe,
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
        let shape_workspace = schema_shape_resolution_workspace(
            schema,
            &schemas,
            schema.dimension_parameters().len(),
        )
        .map_err(fail)?;
        let (live_bytes, live_nodes) = comprehension_nested_live_demand(
            draft_count,
            draft_capacity,
            footprint,
            schema.dimension_parameters().len(),
            schema_arena_bytes,
            published_output_footprint.retained_bytes,
            live_locals,
            meter,
        )
        .map_err(fail)?;
        admit_generator_schema_workspace(shape_workspace, live_bytes, live_nodes, &mut meter)
            .map_err(fail)?;
        let (count, footprint, shape_values, data) = match control.kind {
            crate::ComprehensionKind::Matrix => {
                let mech_core::SchemaBody::Matrix { dimensions, .. } = schema.body() else {
                    return Err(fail(ResidentKernelError::InvalidOutput));
                };
                let shape =
                    super::super::matrix_shape_for_extents(schema, &[1, draft_count as u64])
                        .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
                if dimensions.len() != 2 {
                    return Err(fail(ResidentKernelError::InvalidShape));
                }
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
                            published_output_footprint.retained_bytes,
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
                    published_output_footprint.retained_bytes,
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
            published_output_footprint.retained_bytes,
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
        locals: &[ResidentRegion],
        source: ResidentReadLocation,
        element_schema: SchemaId,
        element: &SchemaBody,
        element_shape_values: &[u64],
        ordinal: usize,
        path: &[usize],
        retained_count: usize,
        retained_capacity: usize,
        retained_footprint: ValueFootprint,
        retained_shape_parameter_count: usize,
        schemas: &Arc<mech_core::SchemaTable>,
        schema_arena_bytes: u64,
        published_output_bytes: u64,
        working: InstanceEpoch,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<(PatternItem, ValueFootprint, ValueFootprint), ResidentKernelError> {
        let value = self
            .read_location(source, working)
            .ok_or(ResidentKernelError::InvalidInput)?;
        let footprint = collection_item_footprint(value, region(source), element, ordinal, meter)?;
        let live_locals = self.comprehension_live_local_footprint(locals, None, schemas, meter)?;
        admit_item_clone(
            footprint,
            retained_count,
            retained_capacity,
            retained_footprint,
            retained_shape_parameter_count,
            schema_arena_bytes,
            live_locals,
            published_output_bytes,
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
        let descent_workspace = item.dynamic_descent_workspace(path, schemas)?;
        if descent_workspace > 0 {
            let (live_bytes, live_nodes) = item_clone_live_demand(
                footprint,
                retained_count,
                retained_capacity,
                retained_footprint,
                retained_shape_parameter_count,
                schema_arena_bytes,
                live_locals,
                published_output_bytes,
                *meter,
            )?;
            admit_generator_schema_workspace(descent_workspace, live_bytes, live_nodes, meter)?;
        }
        for index in path {
            item = item
                .child(*index, schemas)
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

    fn bind_collection_pattern_item(
        &mut self,
        node: NodeId,
        binding: ActivatedPatternBinding,
        locals: &[ResidentRegion],
        source_shape_values: &[u64],
        item: PatternItem,
        selected_footprint: ValueFootprint,
        concrete_footprint: ValueFootprint,
        retained_count: usize,
        retained_capacity: usize,
        retained_footprint: ValueFootprint,
        retained_shape_parameter_count: usize,
        schemas: &Arc<mech_core::SchemaTable>,
        schema_arena_bytes: u64,
        published_output_bytes: u64,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel { node, error };
        let binding_workspace = item
            .binding_resolution_workspace(binding.schema, source_shape_values, schemas)
            .map_err(fail)?;
        let live_locals = self
            .comprehension_live_local_footprint(locals, None, schemas, meter)
            .map_err(fail)?;
        let live_item = selected_footprint
            .checked_add(concrete_footprint)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        let (live_bytes, live_nodes) = item_clone_live_demand(
            live_item,
            retained_count,
            retained_capacity,
            retained_footprint,
            retained_shape_parameter_count,
            schema_arena_bytes,
            live_locals,
            published_output_bytes,
            *meter,
        )
        .map_err(fail)?;
        admit_generator_schema_workspace(binding_workspace, live_bytes, live_nodes, meter)
            .map_err(fail)?;
        let Some(PatternBindingItem {
            shape_values,
            data,
            footprint,
        }) = item
            .into_binding(binding.schema, source_shape_values, schemas)
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
                        let footprint = budget::measure_canonical_value_footprint(
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
                    previous_binding,
                    other_live_locals,
                    retained_count,
                    retained_capacity,
                    retained_footprint,
                    retained_shape_parameter_count,
                    schema_arena_bytes,
                    published_output_bytes,
                    schemas,
                    *meter,
                )
                .map_err(fail)?;
                let remaining = meter
                    .estimate()
                    .remaining_incremental_work()
                    .map_err(fail)?;
                let canonical_budget = SnapshotCanonicalizationBudget::new(remaining);
                let next = pattern_binding_draft(binding.schema, &shape_values, data)
                    .finalize(
                        &SnapshotValidationContext::with_shared_schemas(schemas)
                            .with_canonicalization_budget(&canonical_budget),
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

    fn match_collection_pattern(
        &mut self,
        node: NodeId,
        locals: &[ResidentRegion],
        source: ResidentReadLocation,
        element_schema: SchemaId,
        element: &SchemaBody,
        element_shape_values: &[u64],
        shape_values: &[u64],
        ordinal: usize,
        pattern: &crate::CollectionPattern<ActivatedPatternBinding, ResidentReadLocation>,
        path: &mut [usize; crate::MAX_COLLECTION_PATTERN_DEPTH],
        depth: usize,
        retained_count: usize,
        retained_capacity: usize,
        retained_footprint: ValueFootprint,
        retained_shape_parameter_count: usize,
        schemas: &Arc<mech_core::SchemaTable>,
        schema_arena_bytes: u64,
        published_output_bytes: u64,
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
                        locals,
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        ordinal,
                        &path[..depth],
                        retained_count,
                        retained_capacity,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        schema_arena_bytes,
                        published_output_bytes,
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
                    shape_values,
                    item,
                    selected_footprint,
                    concrete_footprint,
                    retained_count,
                    retained_capacity,
                    retained_footprint,
                    retained_shape_parameter_count,
                    schemas,
                    schema_arena_bytes,
                    published_output_bytes,
                    meter,
                )
            }
            crate::CollectionPattern::Equal(peer) => {
                meter.charge_comparison_work(1).map_err(fail)?;
                let (item, _, _) = self
                    .collection_pattern_item(
                        locals,
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        ordinal,
                        &path[..depth],
                        retained_count,
                        retained_capacity,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        schema_arena_bytes,
                        published_output_bytes,
                        working,
                        meter,
                    )
                    .map_err(fail)?;
                let peer = self
                    .read_location(*peer, working)
                    .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                item.equals_resident(peer, schemas, meter)
                    .map_err(fail)?
                    .ok_or_else(|| fail(ResidentKernelError::InvalidInput))
            }
            crate::CollectionPattern::Tuple(items) => {
                let Some(count) = self
                    .collection_pattern_item(
                        locals,
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        ordinal,
                        &path[..depth],
                        retained_count,
                        retained_capacity,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        schema_arena_bytes,
                        published_output_bytes,
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
                        shape_values,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        retained_count,
                        retained_capacity,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        schema_arena_bytes,
                        published_output_bytes,
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
                        locals,
                        source,
                        element_schema,
                        element,
                        element_shape_values,
                        ordinal,
                        &path[..depth],
                        retained_count,
                        retained_capacity,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        schema_arena_bytes,
                        published_output_bytes,
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
                        shape_values,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        retained_count,
                        retained_capacity,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        schema_arena_bytes,
                        published_output_bytes,
                        working,
                        meter,
                    )? {
                        return Ok(false);
                    }
                }
                // Binding a middle slice needs admitted composite storage; the
                // current target accepts only an ignored wildcard middle.
                if rest
                    .as_deref()
                    .is_some_and(|rest| !matches!(rest, crate::CollectionPattern::Wildcard))
                {
                    return Err(fail(ResidentKernelError::InvalidInput));
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
                        shape_values,
                        ordinal,
                        item,
                        path,
                        depth + 1,
                        retained_count,
                        retained_capacity,
                        retained_footprint,
                        retained_shape_parameter_count,
                        schemas,
                        schema_arena_bytes,
                        published_output_bytes,
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
        values: &mut Vec<ValueDataDraft>,
        footprint: &mut ValueFootprint,
        nested_finalization_work: &mut u64,
        meter: &mut ResidentBudgetMeter,
        schemas: &Arc<mech_core::SchemaTable>,
        schema_arena_bytes: u64,
        published_output_bytes: u64,
        before: InstanceEpoch,
        working: InstanceEpoch,
        probe: &mut ResidentStructuralProbe,
    ) -> Result<(), ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel {
            node: control.artifact_node,
            error,
        };
        for position in start..control.steps.len() {
            meter.charge_compute_work(1).map_err(fail)?;
            match &control.steps[position] {
                ActivatedCollectionStep::Operation { node, work } => {
                    meter.charge_compute_work(*work).map_err(fail)?;
                    let output = self.kernel_scratch_output_region(*node);
                    let live_locals = self
                        .comprehension_live_local_footprint(&control.locals, output, schemas, meter)
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
                        published_output_bytes,
                        live_locals,
                        *meter,
                    )
                    .map_err(fail)?;
                    self.execute_kernel_with_live_demand(
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
                    element_schema,
                    shape_values: activation_shape_values,
                    pattern,
                } => {
                    let source_value = self
                        .read_location(*source, working)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                    let count = collection_len(source_value)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
                    let retained_shape_parameter_count = schemas
                        .get(control.output_schema)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidOutput))?
                        .dimension_parameters()
                        .len();
                    let live_locals = self
                        .comprehension_live_local_footprint(&control.locals, None, schemas, meter)
                        .map_err(fail)?;
                    let (live_bytes, live_nodes) = comprehension_nested_live_demand(
                        values.len(),
                        values.capacity(),
                        *footprint,
                        retained_shape_parameter_count,
                        schema_arena_bytes,
                        published_output_bytes,
                        live_locals,
                        *meter,
                    )
                    .map_err(fail)?;
                    let (live_shape_values, live_shape_values_bytes) = generator_shape_values(
                        source_value,
                        *source_schema,
                        activation_shape_values,
                        schemas,
                        live_bytes,
                        live_nodes,
                        meter,
                    )
                    .map_err(fail)?;
                    let element_input_bytes = live_bytes
                        .checked_add(live_shape_values_bytes)
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                    let element = if let Some(element_schema) = element_schema {
                        let (body, shape_values, retained_bytes) = generator_element(
                            *source_schema,
                            *element_schema,
                            &live_shape_values,
                            schemas,
                            element_input_bytes,
                            live_nodes,
                            meter,
                        )
                        .map_err(fail)?;
                        Some((*element_schema, body, shape_values, retained_bytes))
                    } else {
                        debug_assert!(matches!(pattern, crate::CollectionPattern::Wildcard));
                        None
                    };
                    let nested_schema_bytes = schema_arena_bytes
                        .checked_add(live_shape_values_bytes)
                        .and_then(|bytes| {
                            bytes.checked_add(
                                element
                                    .as_ref()
                                    .map_or(0, |(_, _, _, retained_bytes)| *retained_bytes),
                            )
                        })
                        .ok_or_else(|| fail(ResidentKernelError::InvalidShape))?;
                    for ordinal in 0..count {
                        meter.charge_compute_work(1).map_err(fail)?;
                        let matched =
                            if let Some((element_schema, element, element_shape_values, _)) =
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
                                    values.capacity(),
                                    *footprint,
                                    retained_shape_parameter_count,
                                    schemas,
                                    nested_schema_bytes,
                                    published_output_bytes,
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
                                values,
                                footprint,
                                nested_finalization_work,
                                meter,
                                schemas,
                                nested_schema_bytes,
                                published_output_bytes,
                                before,
                                working,
                                probe,
                            )?;
                        }
                    }
                    return Ok(());
                }
            }
        }
        let value = self
            .read_location(control.yield_value, working)
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
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
            crate::ComprehensionKind::Matrix => admit_output(
                next,
                current_capacity,
                next_footprint,
                retained_shape_parameter_count,
                schema_arena_bytes,
                live_locals,
                published_output_bytes,
                *meter,
            ),
            crate::ComprehensionKind::Set => admit_set_draft(
                next,
                current_capacity,
                next_footprint,
                retained_shape_parameter_count,
                schema_arena_bytes,
                live_locals,
                published_output_bytes,
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
        let item = retained_item(value, control.yield_schema, &context)
            .ok_or_else(|| fail(ResidentKernelError::InvalidInput))?;
        meter
            .charge_comparison_work(canonical_budget.consumed())
            .map_err(fail)?;
        values
            .try_reserve_exact(1)
            .map_err(|_| fail(ResidentKernelError::InvalidShape))?;
        values.push(item);
        *footprint = next_footprint;
        *nested_finalization_work = next_finalization_work;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResidentShape;
    use mech_core::{
        CanonicalNominalPath, CardinalitySpec, DimensionExpr, DimensionLifetime,
        DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin, FloatWidth,
        IntegerWidth, NominalKey, NominalKind, SchemaDraft, SchemaTableBuilder,
    };

    fn atom(name: &str) -> SchemaBody {
        SchemaBody::Atom(NominalKey::from_path(
            NominalKind::Atom,
            &CanonicalNominalPath::new(vec![name.to_owned()]).unwrap(),
        ))
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
            .into_binding(f64_schema, &[], &schemas)
            .expect("binding inspection succeeds")
            .expect("the concrete annotation matches the nested dynamic payload");
        assert!(binding.shape_values.is_empty());
        assert!(matches!(binding.data, ValueDataDraft::F64(value) if value.to_f64() == 3.5));
        assert!(
            item.into_binding(bool_schema, &[], &schemas)
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
            .into_binding(fixed, &[], &schemas)
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
                ValueFootprint::zero(),
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
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                0,
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
        let f64_schema = build.resolve(f64_schema).unwrap();
        let (schemas, _) = build.into_parts();
        let item = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::F64(F64Bits::from_f64(7.0))].into_boxed_slice(),
            ),
        }))));

        let child = item.child(0, &schemas).expect("component descent");
        assert!(matches!(child.data(), Some(ValueDataDraft::F64(value)) if value.to_f64() == 7.0));
        assert!(matches!(
            child,
            PatternItem::Component {
                schema,
                body: SchemaBody::FloatingPoint(FloatWidth::W64),
                ..
            } if schema == f64_schema
        ));
    }

    #[test]
    fn dynamic_child_descent_accounts_for_both_schema_closure_passes() {
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
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let (schemas, _) = build.into_parts();
        let item = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::F64(F64Bits::from_f64(7.0))].into_boxed_slice(),
            ),
        }))));
        let one_closure = schemas
            .get(tuple)
            .unwrap()
            .body()
            .clone_allocation_bound_bytes()
            .unwrap();

        let component_resolution = schemas.clone_allocation_bound_bytes().unwrap() * 4;
        assert_eq!(
            item.dynamic_descent_workspace(&[0], &schemas).unwrap(),
            one_closure * 2 + component_resolution,
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
            matrix,
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
        let build = builder.finish().unwrap();
        let schema = build.resolve(schema).unwrap();
        let (schemas, _) = build.into_parts();
        let workspace =
            schema_shape_resolution_workspace(schemas.get(schema).unwrap(), &schemas, 1).unwrap();
        assert!(workspace > schemas.clone_allocation_bound_bytes().unwrap());
        assert!(workspace >= 6 * core::mem::size_of::<u64>() as u64);
    }

    #[test]
    fn dynamic_component_binding_rejects_an_unaddressable_child_schema() {
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
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let (schemas, _) = build.into_parts();
        let child = PatternItem::new(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema: tuple,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::F64(F64Bits::from_f64(7.0))].into_boxed_slice(),
            ),
        }))))
        .child(0, &schemas);

        assert!(
            child.is_none(),
            "unaddressable component identity is rejected"
        );
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
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                0,
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
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                0,
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
                0,
                0,
                &mut ResidentBudgetMeter::default(),
            )
            .unwrap()
            .0
            .as_ref(),
            [3],
            "execution must replace the stale activation-time extent",
        );
    }

    #[test]
    fn empty_generator_element_materialization_is_admitted_before_closure() {
        let element_body =
            SchemaBody::Tuple(vec![SchemaBody::String, SchemaBody::Bool].into_boxed_slice());
        let mut builder = SchemaTableBuilder::new();
        let element = builder
            .insert(
                SchemaDraft {
                    body: element_body.clone(),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let source = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(element_body),
                        dimensions: vec![DimensionExpr::Constant(0), DimensionExpr::Constant(1)]
                            .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let element = build.resolve(element).unwrap();
        let source = build.resolve(source).unwrap();
        assert!(
            generator_element(
                source,
                element,
                &[],
                &build.table,
                mech_core::RESIDENT_MAX_BYTES,
                0,
                &mut ResidentBudgetMeter::default(),
            )
            .is_err(),
            "schema closure must be rejected before an empty generator allocates above the live ceiling",
        );
        assert!(
            generator_element(
                source,
                element,
                &[],
                &build.table,
                0,
                0,
                &mut ResidentBudgetMeter::default(),
            )
            .is_ok(),
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
            tuple,
            SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()),
            Box::new([]),
            ValueDataDraft::Tuple(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
        );

        let binding = item
            .into_binding(tuple, &[37], &schemas)
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

        let child = item.child(0, &schemas).expect("parameterized component");
        assert_eq!(child.structural_len(false), Some(3));
        assert!(matches!(
            child,
            PatternItem::Component {
                schema,
                body: SchemaBody::Matrix { dimensions, .. },
                shape_values,
                ..
            } if schema == component
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

        let lane = [Some(peer)];
        assert_eq!(
            matching.equals_resident(
                ResidentValueRef::Snapshot(&lane),
                &schemas,
                &mut ResidentBudgetMeter::default(),
            ),
            Ok(Some(true))
        );
        assert_eq!(
            distinct.equals_resident(
                ResidentValueRef::Snapshot(&lane),
                &schemas,
                &mut ResidentBudgetMeter::default(),
            ),
            Ok(Some(false))
        );
    }

    #[test]
    fn absent_dynamic_equality_is_a_nonmatch() {
        let schemas = SchemaTableBuilder::new().finish().unwrap().table;
        let item = PatternItem::Dynamic(None);
        let peer = [7_u64];
        assert_eq!(
            item.equals_resident(
                ResidentValueRef::Index(&peer),
                &schemas,
                &mut ResidentBudgetMeter::default(),
            ),
            Ok(Some(false)),
        );
    }

    #[test]
    fn string_pattern_equality_charges_every_compared_byte() {
        let payload = "pattern-byte".repeat(128);
        let native_item = PatternItem::new(ValueDataDraft::String(payload.clone()));
        let native_peer = [payload.clone()];
        let mut native_meter = ResidentBudgetMeter::default();
        assert_eq!(
            native_item
                .equals_resident(
                    ResidentValueRef::String(&native_peer),
                    &SchemaTableBuilder::new().finish().unwrap().table,
                    &mut native_meter,
                )
                .unwrap(),
            Some(true),
        );
        assert_eq!(
            native_meter.estimate().comparison_work(),
            payload.len() as u64,
        );

        let mut builder = SchemaTableBuilder::new();
        let string = builder
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
        let string = build.resolve(string).unwrap();
        let schemas = Arc::new(build.table);
        let peer = ValueDraft {
            schema: string,
            shape_values: Box::new([]),
            data: ValueDataDraft::String(payload.clone()),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        let snapshot_item = PatternItem::component(
            string,
            SchemaBody::String,
            Box::new([]),
            ValueDataDraft::String(payload.clone()),
        );
        let lane = [Some(peer)];
        let mut snapshot_meter = ResidentBudgetMeter::default();
        assert_eq!(
            snapshot_item
                .equals_resident(
                    ResidentValueRef::Snapshot(&lane),
                    &schemas,
                    &mut snapshot_meter,
                )
                .unwrap(),
            Some(true),
        );
        assert_eq!(
            snapshot_meter.estimate().comparison_work(),
            payload.len() as u64,
        );
    }

    #[test]
    fn native_string_pattern_clones_charge_every_copied_byte() {
        let payload = "generator-byte".repeat(128);
        let values = [payload.clone()];
        let region = ResidentRegion {
            kind: ResidentValueKind::String,
            offset: 0,
            len: 1,
            shape: ResidentShape {
                rows: 1,
                columns: 1,
            },
        };
        let mut meter = ResidentBudgetMeter::default();
        let footprint = collection_item_footprint(
            ResidentValueRef::String(&values),
            region,
            &SchemaBody::String,
            0,
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
    fn pattern_descent_moves_owned_children_without_cloning_payloads() {
        let payload = "payload".repeat(1 << 12);
        let address = payload.as_ptr();
        let item = PatternItem::new(ValueDataDraft::Tuple(
            vec![ValueDataDraft::String(payload)].into_boxed_slice(),
        ));
        let schemas = SchemaTableBuilder::new().finish().unwrap().table;

        let child = item.child(0, &schemas).expect("tuple child");
        let PatternItem::Plain(ValueDataDraft::String(payload)) = child else {
            panic!("expected String child")
        };
        assert_eq!(
            payload.as_ptr(),
            address,
            "descent must move the allocation"
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
            retained_item(
                resident,
                target,
                &SnapshotValidationContext::with_shared_schemas(&target_schemas),
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
            .into_binding(target_f64, &[], &target_schemas)
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
                ValueFootprint::zero(),
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
            admit_item_clone(
                item,
                1,
                1,
                retained,
                0,
                0,
                ValueFootprint::zero(),
                0,
                ResidentBudgetMeter::default()
            )
            .is_err(),
            "the retained draft and current clone overlap above the temporary limit"
        );
    }

    #[test]
    fn item_clone_admission_counts_live_bindings_and_published_output() {
        let item = ValueFootprint {
            encoded_bytes: 5 * 1024 * 1024,
            retained_bytes: 5 * 1024 * 1024,
            node_count: 1,
        };
        let live_binding = ValueFootprint {
            encoded_bytes: 0,
            retained_bytes: 12 * 1024 * 1024,
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
                live_binding,
                0,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "a prior structural binding remains live while the selected item is cloned",
        );
        assert!(
            admit_item_clone(
                item,
                0,
                0,
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                12 * 1024 * 1024,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "the published comprehension result remains live while the selected item is cloned",
        );
    }

    #[test]
    fn pattern_admissions_count_spare_result_capacity() {
        let unit = core::mem::size_of::<ValueDataDraft>();
        let oversized_capacity = (mech_core::RESIDENT_MAX_BYTES as usize / unit) + 1;
        assert!(
            admit_item_clone(
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                0,
                ResidentBudgetMeter::default(),
            )
            .is_ok(),
        );
        assert!(
            admit_item_clone(
                ValueFootprint::zero(),
                0,
                oversized_capacity,
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                0,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "spare result slots remain live while the pattern item is cloned",
        );

        let mut builder = SchemaTableBuilder::new();
        let atom = builder
            .insert(
                SchemaDraft {
                    body: atom("capacity"),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let atom = build.resolve(atom).unwrap();
        assert!(
            admit_pattern_binding_finalization(
                atom,
                &[],
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                oversized_capacity,
                ValueFootprint::zero(),
                0,
                0,
                0,
                &build.table,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "spare result slots remain live while a snapshot binding is finalized",
        );
    }

    #[test]
    fn nested_operation_demand_includes_every_live_comprehension_allocation() {
        let retained = ValueFootprint {
            encoded_bytes: 2 * 1024 * 1024,
            retained_bytes: 2 * 1024 * 1024,
            node_count: 3,
        };
        let locals = ValueFootprint {
            encoded_bytes: 0,
            retained_bytes: 3 * 1024 * 1024,
            node_count: 5,
        };
        let mut meter = ResidentBudgetMeter::default();
        meter.charge_temporary_bytes(1024 * 1024).unwrap();
        meter.charge_retained_nodes(7).unwrap();
        let draft = snapshot_draft_bytes(1, retained, 0).unwrap()
            + draft_capacity_overlap_bytes(1, 1).unwrap();
        let (bytes, nodes) = comprehension_nested_live_demand(
            1,
            1,
            retained,
            0,
            2 * 1024 * 1024,
            4 * 1024 * 1024,
            locals,
            meter,
        )
        .unwrap();
        assert_eq!(bytes, draft + 10 * 1024 * 1024);
        assert_eq!(nodes, 7 + 3 + 5 + 1);
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
                0,
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
                0,
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
    fn output_admission_counts_the_published_value_during_replacement() {
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
                9 * 1024 * 1024,
                ResidentBudgetMeter::default(),
            )
            .is_err(),
            "the current publication overlaps the mutable draft and immutable replacement",
        );
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
                0,
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
                0,
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
                0,
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
                0,
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
                0,
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
                0,
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
                0,
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
                0,
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
                0,
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
                0,
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
                ValueFootprint::zero(),
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                0,
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
                ValueFootprint {
                    encoded_bytes: 6 * 1024 * 1024,
                    retained_bytes: 6 * 1024 * 1024,
                    node_count: 1,
                },
                ValueFootprint::zero(),
                0,
                0,
                ValueFootprint::zero(),
                0,
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
                ValueFootprint::zero(),
                ValueFootprint {
                    encoded_bytes: 0,
                    retained_bytes: 6 * 1024 * 1024,
                    node_count: 1,
                },
                0,
                0,
                ValueFootprint::zero(),
                0,
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
    fn local_dynamic_roots_reuse_a_component_closed_plan() {
        let mut builder = SchemaTableBuilder::new();
        let tuple = builder
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
        let build = builder.finish().unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let sparse = Arc::new(build.table);
        assert!(
            schema_root_requires_import(
                &sparse,
                tuple,
                true,
                &sparse,
                &mut ResidentBudgetMeter::default(),
            )
            .unwrap(),
            "a local Dynamic root with missing component entries still needs closure",
        );
        let closed = Arc::new(
            sparse
                .extend_with_component_closure_preserving_ids()
                .unwrap(),
        );
        assert!(
            !schema_root_requires_import(
                &closed,
                tuple,
                true,
                &closed,
                &mut ResidentBudgetMeter::default(),
            )
            .unwrap(),
            "a source-built component-closed plan needs no redundant clone or merge",
        );

        let mut extension = SchemaTableBuilder::new();
        extension
            .insert(
                SchemaDraft {
                    body: atom("unrelated"),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let foreign = Arc::new(
            closed
                .extend_preserving_ids(&extension.finish().unwrap().table)
                .unwrap(),
        );
        assert!(
            !schema_root_requires_import(
                &foreign,
                tuple,
                false,
                &closed,
                &mut ResidentBudgetMeter::default(),
            )
            .unwrap(),
            "an independently owned arena with unrelated entries reuses an addressable root closure",
        );
    }

    #[test]
    fn schema_root_segments_deduplicate_without_growing() {
        let owner = Arc::new(SchemaTableBuilder::new().finish().unwrap().table);
        let mut state = SchemaOwnerRoots {
            owner,
            root_start: 0,
            root_len: 0,
            root_capacity: 2,
        };
        let mut storage = vec![SchemaId::new(0); 2];
        let allocation = storage.as_ptr();
        let mut meter = ResidentBudgetMeter::default();
        state
            .insert_root(&mut storage, SchemaId::new(7), &mut meter)
            .unwrap();
        state
            .insert_root(&mut storage, SchemaId::new(7), &mut meter)
            .unwrap();
        state
            .insert_root(&mut storage, SchemaId::new(9), &mut meter)
            .unwrap();
        assert_eq!(state.roots(&storage), [SchemaId::new(7), SchemaId::new(9)]);
        assert_eq!(storage.as_ptr(), allocation);
        assert!(
            state
                .insert_root(&mut storage, SchemaId::new(11), &mut meter)
                .is_err(),
            "fixed root storage fails closed instead of reallocating",
        );
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
                                DimensionExpr::Constant(1),
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
            let tuple = build.resolve(tuple).unwrap();
            let matrix = build.resolve(matrix).unwrap();
            let (schemas, _) = build.into_parts();
            (schemas, tuple, matrix)
        }

        let (foreign, foreign_tuple, foreign_matrix) = schemas(false);
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
        let plan_matrix = plan_builder
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
        let plan_build = plan_builder.finish().unwrap();
        let plan_matrix = plan_build.resolve(plan_matrix).unwrap();
        let plan = plan_build.table;
        let foreign_tuple_key = foreign.entry(foreign_tuple).unwrap().key();
        assert!(
            plan.find_by_key(foreign_tuple_key).is_none(),
            "the concrete Dynamic payload is deliberately unknown to the plan",
        );
        let plan = std::sync::Arc::new(plan.extend_preserving_ids(&foreign).unwrap());
        let plan_tuple = plan.find_by_key(foreign_tuple_key).unwrap();
        let source = ValueDraft {
            schema: foreign_matrix,
            shape_values: Box::new([]),
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
        .finalize(&SnapshotValidationContext::new(&foreign))
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
        let mut meter = ResidentBudgetMeter::default();
        retained_value_footprint(
            ResidentValueRef::Snapshot(&lane),
            plan_matrix,
            &plan,
            &mut meter,
        )
        .expect("equivalent schemas compare by definition rather than arena ordinal");
        let retained = retained_item(
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
            generator_shape_values(
                ResidentValueRef::Snapshot(&lane),
                schema,
                &[1],
                &schemas,
                0,
                0,
                &mut ResidentBudgetMeter::default(),
            )
            .unwrap()
            .0
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
