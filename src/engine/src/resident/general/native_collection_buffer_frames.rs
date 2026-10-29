//! Validate ONE selected tuple/record pack without materializing its child payloads.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeLibraryWorkBudget, NativeScalarKind,
    NativeScalarValue,
};
use crate::resident::budget::{
    ResidentBudgetMeter, measure_borrowed_canonical_value_footprint,
    measure_native_scalar_frame_footprint, native_turn_plan_cache::NativeProjectionIndexContext,
};
use mech_core::{
    MemoryBudgetLimits, ResourceDemand, Value, snapshot::CompositeSnapshotConstructor,
};

/// Only immutable schema work is retained. Current arguments and shapes are
/// never cached; unsupported shape relations use the ordinary constructor.
#[derive(Debug)]
pub(super) struct FrameGeometry {
    metadata: Option<u64>,
    normalization: Option<mech_core::snapshot::CompositeBindingCost>,
    shape: Option<super::frame_shape::FrameShapeRecipe>,
    admission: Option<super::frame_admission::FrameAdmissionRecipe>,
}
impl FrameGeometry {
    pub(super) fn derive(
        template: &super::NativeProgramLibraryTemplate,
        schemas: &mech_core::SchemaTable,
        binding_bytes: u64,
    ) -> Option<Box<Self>> {
        if !matches!(template.primitive(), Primitive::CompositePack(_))
            || !matches!(
                template.output().body(),
                mech_core::SchemaBody::Tuple(_) | mech_core::SchemaBody::Record(_)
            )
        {
            return None;
        }
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            return None;
        };
        let output = contract.outputs[0].schema;
        let inputs = contract
            .inputs
            .iter()
            .map(|port| port.schema)
            .collect::<Vec<_>>();
        Some(Box::new(Self {
            metadata: schemas
                .clone_allocation_bound_bytes()
                .and_then(|n| binding_bytes.checked_add(n))
                .and_then(|n| n.checked_add((inputs.len() as u64).checked_mul(1024)?))
                .and_then(|n| n.checked_add(4096)),
            normalization: CompositeSnapshotConstructor::binding_cost_for_inputs(
                output,
                inputs.len(),
                |i| inputs.get(i).copied(),
                schemas,
            ),
            shape: super::frame_shape::FrameShapeRecipe::derive(output, &inputs, schemas),
            admission: super::frame_admission::FrameAdmissionRecipe::derive(&inputs, schemas),
        }))
    }
}

#[derive(Clone, Copy)]
enum Child<'a> {
    Managed(&'a Value),
    Word(mech_core::SchemaId, NativeScalarValue),
}
impl NativePreparedLibraryCall {
    /// Validate the exact selected field operation for a compiler-proved private
    /// record. The pack already checked the complete record shape; its actual
    /// canonical child remains the output owner, without a payload clone.
    pub fn admit_buffer_record_field(
        &self,
        selector: &Value,
        ordinal: usize,
        child: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        self.admit_buffer_record_child(
            selector,
            ordinal,
            Child::Managed(child),
            limits,
            work,
            coexistence,
        )
    }

    /// A selected scalar field stays in its admitted native lane. This does
    /// not grant entry to a source site or authorize a canonical publication.
    pub fn admit_buffer_record_word(
        &self,
        selector: &Value,
        ordinal: usize,
        schema: mech_core::SchemaId,
        value: NativeScalarValue,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        self.admit_buffer_record_child(
            selector,
            ordinal,
            Child::Word(schema, value),
            limits,
            work,
            coexistence,
        )
    }

    fn admit_buffer_record_child(
        &self,
        selector: &Value,
        ordinal: usize,
        child: Child<'_>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::RecordFieldAccess
            || self.template.inputs().len() != 2
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let memory = work.memory_budget();
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if selector.schema() != contract.inputs[1].schema
                    || selector.schema_key() != self.input_keys[1]
                    || match child {
                        Child::Managed(child) => {
                            child.schema() != contract.outputs[0].schema
                                || child.schema_key() != self.output_key
                        }
                        Child::Word(schema, value) => {
                            schema != contract.outputs[0].schema
                                || !self.template.output().dimension_parameters().is_empty()
                                || NativeScalarKind::from_schema_body(self.template.output().body())
                                    != Some(value.kind())
                        }
                    }
                {
                    return Err(InvokeError::InvalidInput);
                }
                selector
                    .validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let (retained, nodes) = match child {
                    Child::Managed(child) => {
                        child
                            .validate_against(&self.schemas)
                            .map_err(|_| InvokeError::InvalidInput)?;
                        (
                            child
                                .memory_budget_retained_bytes(&memory)
                                .ok_or(InvokeError::UnadmittedValue)?,
                            child
                                .memory_budget_retained_nodes(&memory)
                                .ok_or(InvokeError::UnadmittedValue)?,
                        )
                    }
                    Child::Word(..) => (0, 0), // The owning frame's typed arena is already admitted.
                };
                let selector_bytes = selector
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let selector_nodes = selector
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(
                        retained
                            .checked_add(selector_bytes)
                            .ok_or(InvokeError::InvalidInput)?,
                        nodes
                            .checked_add(selector_nodes)
                            .ok_or(InvokeError::InvalidInput)?,
                    )?;
                context.check_capacity(0, 0)?;
                context.charge(
                    self.lookup_work
                        .checked_add(128)
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                let (mech_core::SchemaBody::Record(fields), mech_core::ValueData::Id(selected)) =
                    (self.template.inputs()[0].body(), selector.data())
                else {
                    return Err(InvokeError::InvalidInput);
                };
                let actual = crate::resident::numeric::named_schema_ordinal(
                    fields.iter().map(|f| f.name.as_str()),
                    *selected,
                    |characters| {
                        context.charge(
                            characters
                                .checked_mul(16)
                                .and_then(|n| n.checked_add(64))
                                .ok_or(mech_core::ResidentKernelError::InvalidShape)?,
                            characters,
                        )
                    },
                )?;
                if actual != ordinal {
                    return Err(InvokeError::InvalidInput);
                }
                let mut meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
                let footprint = match child {
                    Child::Managed(child) => measure_borrowed_canonical_value_footprint(
                        &mut meter,
                        child,
                        &self.schemas,
                    )?,
                    Child::Word(_, value) => measure_native_scalar_frame_footprint(
                        &mut meter,
                        self.template.output().body(),
                        value,
                    )?,
                };
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: 1,
                        ..ResourceDemand::default()
                    },
                    footprint.retained_bytes,
                    0,
                    limits,
                )
                .first()
                {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded {
                            violation: violation.clone(),
                        },
                    ));
                }
                Ok(())
            },
        )?
    }

    /// The generated caller proves private frame uses and retains the actual
    /// already-admitted child owners in its planned invocation-local lanes.
    /// This method supplies neither a visit grant nor a canonical tuple Value.
    pub fn admit_buffer_frame(
        &self,
        inputs: &[Option<Value>],
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        self.admit_buffer_frame_inputs(inputs, None, limits, work, coexistence)
    }

    pub fn admit_buffer_frame_words(
        &self,
        inputs: &[Option<Value>],
        words: &[u64],
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        if inputs.len() != words.len() {
            return Err(InvokeError::InvalidInput);
        }
        self.admit_buffer_frame_inputs(inputs, Some(words), limits, work, coexistence)
    }

    fn admit_buffer_frame_inputs(
        &self,
        inputs: &[Option<Value>],
        words: Option<&[u64]>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::CompositePack(inputs.len())
            || !matches!(
                self.template.output().body(),
                mech_core::SchemaBody::Tuple(_) | mech_core::SchemaBody::Record(_)
            )
            || self.template.inputs().len() != inputs.len()
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let memory = work.memory_budget();
                let geometry = self
                    .frame_geometry
                    .as_ref()
                    .ok_or(InvokeError::InvalidInput)?;
                let metadata = geometry.metadata.ok_or(InvokeError::InvalidInput)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                // Prepay the allocation-free schema preflight, then use the
                // core constructor's own shape/closure witness. Arity alone
                // does not bound dimension expressions or solver metadata.
                context.charge(metadata.checked_mul(8).ok_or(InvokeError::InvalidInput)?, 0)?;
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                let output = contract.outputs[0].schema;
                let normalization = geometry.normalization.ok_or(InvokeError::InvalidInput)?;
                let metadata = metadata
                    .checked_add(normalization.temporary_bytes)
                    .ok_or(InvokeError::InvalidInput)?;
                let nodes = normalization
                    .metadata_nodes
                    .checked_add(inputs.len() as u64)
                    .and_then(|n| n.checked_add(8))
                    .ok_or(InvokeError::InvalidInput)?;
                context.check_capacity(metadata, nodes)?;
                context.charge(
                    normalization
                        .temporary_bytes
                        .checked_mul(8)
                        .and_then(|n| n.checked_add(normalization.compute_work))
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                let _admission = memory.reserve_capacity(metadata)?;
                let _nodes = memory.reserve_nodes(nodes)?;
                let mut meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
                let admitted = match (&geometry.admission, &geometry.shape) {
                    (Some(recipe), Some(shape)) => {
                        recipe.try_admit(self, inputs, words, shape, &memory, &mut meter)?
                    }
                    _ => None,
                };
                let mut logical_bytes = admitted.unwrap_or(0);
                if admitted.is_none() {
                    for (index, value) in inputs.iter().enumerate() {
                        let footprint = match value {
                            Some(value) => {
                                if words.is_some_and(|bits| bits[index] != 0)
                                    || value.schema_key() != self.input_keys[index]
                                    || value.memory_budget_retained_bytes(&memory).is_none()
                                {
                                    return Err(InvokeError::InvalidInput);
                                }
                                value
                                    .validate_against(&self.schemas)
                                    .map_err(|_| InvokeError::InvalidInput)?;
                                measure_borrowed_canonical_value_footprint(
                                    &mut meter,
                                    value,
                                    &self.schemas,
                                )?
                            }
                            None => {
                                let schema = &self.template.inputs()[index];
                                if !schema.dimension_parameters().is_empty() {
                                    return Err(InvokeError::InvalidInput);
                                }
                                let kind = NativeScalarKind::from_schema_body(schema.body())
                                    .ok_or(InvokeError::InvalidInput)?;
                                let bits = words.ok_or(InvokeError::InvalidInput)?[index];
                                let value = NativeScalarValue::from_bits(kind, bits)
                                    .ok_or(InvokeError::InvalidInput)?;
                                measure_native_scalar_frame_footprint(
                                    &mut meter,
                                    schema.body(),
                                    value,
                                )?
                            }
                        };
                        logical_bytes = logical_bytes
                            .checked_add(footprint.retained_bytes)
                            .ok_or(InvokeError::InvalidInput)?;
                    }
                    if let Some(recipe) = &geometry.shape {
                        recipe.validate(&self.schemas, |i| inputs[i].as_ref().map(Value::shape))?;
                    } else {
                        let children = inputs
                            .iter()
                            .enumerate()
                            .map(|(index, value)| {
                                if let Some(value) = value {
                                    Ok((value.schema(), value.shape().clone()))
                                } else {
                                    let schema = contract.inputs[index].schema;
                                    let shape = self
                                        .schemas
                                        .get(schema)
                                        .ok_or(InvokeError::InvalidInput)?
                                        .instantiate_shape(Box::new([]))
                                        .map_err(|_| InvokeError::InvalidInput)?;
                                    Ok((schema, shape))
                                }
                            })
                            .collect::<Result<Vec<_>, InvokeError>>()?;
                        let shape = CompositeSnapshotConstructor::shape_for_children(
                            output,
                            &children,
                            &self.schemas,
                        )
                        .map_err(|_| InvokeError::InvalidInput)?;
                        let _constructor = CompositeSnapshotConstructor::bind(
                            output,
                            shape,
                            &children,
                            self.schemas.clone(),
                        )
                        .map_err(|_| InvokeError::InvalidInput)?;
                    }
                }
                // Retain the logical output bound even though no payload is copied.
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: inputs.len() as u64,
                        ..ResourceDemand::default()
                    },
                    logical_bytes,
                    0,
                    limits,
                )
                .first()
                {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded {
                            violation: violation.clone(),
                        },
                    ));
                }
                Ok(())
            },
        )?
    }
}

impl NativePreparedLibraryCall {
    /// Static elision eligibility only. Current owners and logical output
    /// limits remain the selecting region's invocation-bound responsibility.
    pub fn supports_invariant_buffer_frame(&self) -> bool {
        self.frame_geometry
            .as_ref()
            .and_then(|geometry| geometry.shape.as_ref())
            .is_some_and(|recipe| recipe.is_identity_forwarding())
    }
}
