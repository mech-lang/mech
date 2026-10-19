//! Borrow a real selected Map<String,String> entry; never build a key/output
//! Value, manufacture a turn plan, or grant permission to visit its source.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::resident::{
    budget::{NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext},
    numeric::string_map_access::{self, Budget},
};
use mech_core::{
    MemoryBudgetLimits, ResidentKernelError, ResourceDemand, SchemaBody, Value, ValueData,
};

struct BorrowedBudget<'a, 'b> {
    context: &'a NativeProjectionIndexContext<'b>,
    bytes: u64,
    nodes: u64,
}
impl Budget for BorrowedBudget<'_, '_> {
    fn compute(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.context.charge(amount, 0)
    }
    fn comparison(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        // ResidentBudgetMeter charges comparison work to BOTH counters.
        self.context.charge(amount, amount)
    }
    fn nodes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        let next = self
            .nodes
            .checked_add(amount)
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.context.check_capacity(self.bytes, next)?;
        self.nodes = next;
        Ok(())
    }
    fn selector_bytes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        let next = self
            .bytes
            .checked_add(amount)
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.context.check_capacity(next, self.nodes)?;
        self.bytes = next;
        Ok(())
    }
    fn admit(&mut self) -> Result<(), ResidentKernelError> {
        self.context.check_capacity(self.bytes, self.nodes)
    }
}

impl NativePreparedLibraryCall {
    /// The caller owns both genuine same-account roots and the generated
    /// current-candidate proof. `coexistence` ALREADY includes their FULL owner
    /// bounds and canonical nodes, the selected sink, and its cumulative work.
    /// Only the checked ordinal escapes; no borrowed pointer survives this call.
    pub fn selected_string_map_entry(
        &self,
        source: &Value,
        key_owner: &Value,
        key_bytes: core::ops::Range<usize>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<usize, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::MapKeyAccess
            || self.template.inputs().len() != 2
            || !matches!(self.template.inputs()[0].body(), SchemaBody::Map { key, value, .. }
                if key.as_ref() == &SchemaBody::String && value.as_ref() == &SchemaBody::String)
            || !self.template.inputs()[0].dimension_parameters().is_empty()
            || self.template.inputs()[1].body() != &SchemaBody::String
            || !self.template.inputs()[1].dimension_parameters().is_empty()
            || self.template.output().body() != &SchemaBody::String
            || !self.template.output().dimension_parameters().is_empty()
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(0, 0)?;
                // Setup already checked the immutable selected factory/template.
                // This route allocates no binding, key, result or cache metadata.
                // Prepay the original fixed header/shape/refcount probe only.
                context.charge(self.lookup_work, 0)?;
                let memory = work.memory_budget();
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if source.schema() != contract.inputs[0].schema
                    || source.schema_key() != self.input_keys[0]
                    || source.shape().parameter_values().len()
                        != self.template.inputs()[0].dimension_parameters().len()
                    || !key_owner.shape().parameter_values().is_empty()
                {
                    return Err(InvokeError::InvalidInput);
                }
                if source.memory_budget_retained_bytes(&memory).is_none()
                    || key_owner.memory_budget_retained_bytes(&memory).is_none()
                {
                    return Err(InvokeError::UnadmittedValue);
                }
                let (ValueData::Map(map), ValueData::String(key)) =
                    (source.data(), key_owner.data())
                else {
                    return Err(InvokeError::InvalidInput);
                };
                let key = key.get(key_bytes).ok_or(InvokeError::InvalidInput)?;
                let mut meter = BorrowedBudget {
                    context: &context,
                    bytes: 0,
                    nodes: 0,
                };
                let candidate = string_map_access::candidate_work(key, &mut meter)?;
                let ordinal = string_map_access::find(map, key, candidate, &mut meter)?;
                if !matches!(
                    map.entries().get(ordinal).map(|entry| entry.value()),
                    Some(ValueData::String(_))
                ) {
                    return Err(InvokeError::InvalidInput);
                }
                Ok(ordinal)
            },
        )?
    }
}
