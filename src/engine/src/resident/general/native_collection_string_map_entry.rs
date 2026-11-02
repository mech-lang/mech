//! Borrow a real selected Map<String,String> entry; never build a key/output
//! Value, manufacture a turn plan, or grant permission to visit its source.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::resident::{
    budget::{NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext},
    numeric::string_map_access::{self, Budget},
};
use mech_core::{
    MemoryBudgetLimits, ResidentKernelError, ResourceDemand, SchemaBody, Value, ValueData,
    snapshot::OwnedSnapshotBytes,
};

enum KeyOwner<'a> {
    Frozen(&'a Value),
    Owned(&'a OwnedSnapshotBytes),
    #[cfg(feature = "pure_text")]
    Private(&'a super::NativePrivateStringBuffer),
}

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
        self.selected_string_map_entry_owner(
            source,
            KeyOwner::Frozen(key_owner),
            key_bytes,
            limits,
            work,
            coexistence,
        )
    }

    /// Same selected lookup and budget leaf, after a compiler-proved last-use
    /// transfer. The actual owned String carries its same-account charge; no
    /// canonical Value is fabricated for mutable native-loop backing.
    pub fn selected_owned_string_map_entry(
        &self,
        source: &Value,
        key_owner: &OwnedSnapshotBytes,
        key_bytes: core::ops::Range<usize>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<usize, InvokeError> {
        self.selected_string_map_entry_owner(
            source,
            KeyOwner::Owned(key_owner),
            key_bytes,
            limits,
            work,
            coexistence,
        )
    }

    /// Original selected lookup over an admitted private String buffer. The
    /// owner binds initialized bytes, the selected String schema and account;
    /// the caller still proves the exact source generation and operation visit.
    #[cfg(feature = "pure_text")]
    pub fn selected_private_string_map_entry(
        &self,
        source: &Value,
        key_owner: &super::NativePrivateStringBuffer,
        key_bytes: core::ops::Range<usize>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<usize, InvokeError> {
        self.selected_string_map_entry_owner(
            source,
            KeyOwner::Private(key_owner),
            key_bytes,
            limits,
            work,
            coexistence,
        )
    }

    fn selected_string_map_entry_owner(
        &self,
        source: &Value,
        key_owner: KeyOwner<'_>,
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
                let key_schema_valid = match &key_owner {
                    KeyOwner::Frozen(value) => value.shape().parameter_values().is_empty(),
                    KeyOwner::Owned(value) => value.schema_key() == self.input_keys[1],
                    #[cfg(feature = "pure_text")]
                    KeyOwner::Private(value) => value.schema_key() == self.input_keys[1],
                };
                if source.schema() != contract.inputs[0].schema
                    || source.schema_key() != self.input_keys[0]
                    || source.shape().parameter_values().len()
                        != self.template.inputs()[0].dimension_parameters().len()
                    || !key_schema_valid
                {
                    return Err(InvokeError::InvalidInput);
                }
                if source.memory_budget_retained_bytes(&memory).is_none()
                    || match &key_owner {
                        KeyOwner::Frozen(value) => {
                            value.memory_budget_retained_bytes(&memory).is_none()
                        }
                        KeyOwner::Owned(value) => {
                            value.memory_budget() != memory
                                || value.retained_capacity_bytes() < value.as_bytes().len() as u64
                        }
                        #[cfg(feature = "pure_text")]
                        KeyOwner::Private(value) => {
                            value.memory_budget() != memory
                                || value.retained_capacity_bytes() < value.as_bytes().len() as u64
                        }
                    }
                {
                    return Err(InvokeError::UnadmittedValue);
                }
                let ValueData::Map(map) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                let key: &[u8] = match &key_owner {
                    KeyOwner::Frozen(value) => match value.data() {
                        ValueData::String(key) => key.as_bytes(),
                        _ => return Err(InvokeError::InvalidInput),
                    },
                    KeyOwner::Owned(value) => value.as_bytes(),
                    #[cfg(feature = "pure_text")]
                    KeyOwner::Private(value) => value.as_bytes(),
                };
                let key = key.get(key_bytes).ok_or(InvokeError::InvalidInput)?;
                let key = core::str::from_utf8(key).map_err(|_| InvokeError::InvalidInput)?;
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
