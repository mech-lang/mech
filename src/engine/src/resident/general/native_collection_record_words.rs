//! Selected scalar record fields borrow immutable data into existing native lanes.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::{
    native_artifact::{NativeCollectionPrimitive as Primitive, NativeScalarKind as Kind},
    resident::budget::{
        NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
    },
};
use mech_core::{
    FloatWidth, IntegerWidth, MemoryBudgetLimits, ResourceDemand, SchemaBody, Value, ValueData,
};

impl NativePreparedLibraryCall {
    pub fn record_scalar_field_kind(&self) -> Option<Kind> {
        let [source, selector] = self.template.inputs() else {
            return None;
        };
        if self.template.primitive() != Primitive::RecordFieldAccess
            || selector.body() != &SchemaBody::Id
            || !selector.dimension_parameters().is_empty()
            || !self.template.output().dimension_parameters().is_empty()
            || !matches!(source.body(), SchemaBody::Record(fields) if fields.iter().any(|field| &field.schema == self.template.output().body()))
        {
            return None;
        }
        Some(match self.template.output().body() {
            SchemaBody::Bool => Kind::Bool,
            SchemaBody::UnsignedInteger(IntegerWidth::W8) => Kind::U8,
            SchemaBody::UnsignedInteger(IntegerWidth::W16) => Kind::U16,
            SchemaBody::UnsignedInteger(IntegerWidth::W32) => Kind::U32,
            SchemaBody::UnsignedInteger(IntegerWidth::W64) => Kind::U64,
            SchemaBody::SignedInteger(IntegerWidth::W64) => Kind::I64,
            SchemaBody::FloatingPoint(FloatWidth::W64) => Kind::F64,
            _ => return None,
        })
    }

    /// Only the genuine selected call supplies this path. The generated caller
    /// retains current-visit, coexistence and publication authority.
    pub fn record_scalar_field(
        &self,
        source: &Value,
        selector: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let kind = self
            .record_scalar_field_kind()
            .ok_or(InvokeError::InvalidInput)?;
        let bytes = match kind {
            Kind::Bool | Kind::U8 => 1,
            Kind::U16 => 2,
            Kind::U32 => 4,
            _ => 8,
        };
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                coexistence.output_elements = coexistence
                    .output_elements
                    .checked_add(1)
                    .ok_or(InvokeError::InvalidInput)?;
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: 1,
                        ..ResourceDemand::default()
                    },
                    bytes,
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
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(0, 0)?;
                context.charge(self.lookup_work, 0)?;
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                let memory = work.memory_budget();
                let mut retained = bytes;
                let mut nodes = 1u64;
                for (index, value) in [source, selector].into_iter().enumerate() {
                    if value.schema() != contract.inputs[index].schema
                        || value.schema_key() != self.input_keys[index]
                        || value.shape().parameter_values().len()
                            != self.template.inputs()[index].dimension_parameters().len()
                    {
                        return Err(InvokeError::InvalidInput);
                    }
                    value
                        .validate_against(&self.schemas)
                        .map_err(|_| InvokeError::InvalidInput)?;
                    retained = retained
                        .checked_add(
                            value
                                .memory_budget_retained_bytes(&memory)
                                .ok_or(InvokeError::UnadmittedValue)?,
                        )
                        .ok_or(InvokeError::InvalidInput)?;
                    nodes = nodes
                        .checked_add(
                            value
                                .memory_budget_retained_nodes(&memory)
                                .ok_or(InvokeError::UnadmittedValue)?,
                        )
                        .ok_or(InvokeError::InvalidInput)?;
                }
                let context = context.with_added_capacity(retained, nodes)?;
                context.check_capacity(0, 0)?;
                context.charge(128, 0)?;
                let (
                    SchemaBody::Record(fields),
                    ValueData::Record(record),
                    ValueData::Id(selected),
                ) = (
                    self.template.inputs()[0].body(),
                    source.data(),
                    selector.data(),
                )
                else {
                    return Err(InvokeError::InvalidInput);
                };
                let ordinal = crate::resident::numeric::named_schema_ordinal(
                    fields.iter().map(|field| field.name.as_str()),
                    *selected,
                    |characters| {
                        let compute = characters
                            .checked_mul(16)
                            .and_then(|n| n.checked_add(64))
                            .ok_or(mech_core::ResidentKernelError::InvalidShape)?;
                        context.charge(compute, characters)
                    },
                )?;
                if &fields[ordinal].schema != self.template.output().body() {
                    return Err(InvokeError::InvalidInput);
                }
                match (kind, record.fields().get(ordinal)) {
                    (Kind::Bool, Some(ValueData::Bool(value))) => Ok(u64::from(*value)),
                    (Kind::U8, Some(ValueData::U8(value))) => Ok(u64::from(*value)),
                    (Kind::U16, Some(ValueData::U16(value))) => Ok(u64::from(*value)),
                    (Kind::U32, Some(ValueData::U32(value))) => Ok(u64::from(*value)),
                    (Kind::U64, Some(ValueData::U64(value))) => Ok(*value),
                    (Kind::I64, Some(ValueData::I64(value))) => Ok(*value as u64),
                    (Kind::F64, Some(ValueData::F64(value))) => Ok(value.bits()),
                    _ => Err(InvokeError::InvalidInput),
                }
            },
        )?
    }
}
