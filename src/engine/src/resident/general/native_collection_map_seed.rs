//! Original-factory ownership of a private flat String/U64 Map seed aggregate.
//! No entry permission, map mutation pointer, or caller cost certificate.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::{NativeCollectionPrimitive as Primitive, NativeLibraryWorkBudget};
use mech_core::{
    IntegerWidth, ManagedMemoryReservation, ManagedNodeReservation, MemoryBudgetLimits,
    ResourceDemand, SchemaBody, SchemaId, SchemaKey, Value, ValueData,
};
use std::{mem::size_of, sync::Arc};

pub struct NativeMapSeedFrame {
    // Real children retire before the metadata claims covering their pin array.
    children: Box<[Option<Value>]>,
    map_ordinal: usize,
    schema: SchemaId,
    key: SchemaKey,
    logical_bytes: u64,
    logical_nodes: u64,
    source_factory: Arc<()>,
    work: NativeLibraryWorkBudget,
    limits: MemoryBudgetLimits,
    spent: (u64, u64),
    metadata: ManagedMemoryReservation,
    nodes: ManagedNodeReservation,
}
pub enum NativeMapSeedChoice {
    Frame(NativeMapSeedFrame),
    Canonical(Value),
}
fn add(a: u64, b: u64) -> Result<u64, InvokeError> {
    a.checked_add(b).ok_or(InvokeError::InvalidInput)
}
fn map(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Map { key, value, .. }
        if key.as_ref()==&SchemaBody::String
            && value.as_ref()==&SchemaBody::UnsignedInteger(IntegerWidth::W64))
}
impl NativePreparedLibraryCall {
    /// One source call scope covers optional frame preparation and immediate
    /// ordinary refusal. Genuine inputs come from the same selected factory.
    pub fn map_seed_or_canonical(
        &self,
        inputs: &[Option<Value>],
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<NativeMapSeedChoice, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::CompositePack(inputs.len())
            || inputs.len() > 32
            || inputs.is_empty()
            || inputs.iter().any(Option::is_none)
        {
            return Err(InvokeError::InvalidInput);
        }
        let enclosing = work.has_active_call_limit()?;
        let started = work.used();
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                if !enclosing
                    && let Some(frame) =
                        self.try_map_seed_in_call(inputs, limits, work, coexistence, started)?
                {
                    return Ok(NativeMapSeedChoice::Frame(frame));
                }
                let mut output = None;
                self.invoke_current(inputs, &mut output, limits, work)?;
                Ok(NativeMapSeedChoice::Canonical(
                    output.ok_or(InvokeError::InvalidInput)?,
                ))
            },
        )?
    }
    fn try_map_seed_in_call(
        &self,
        inputs: &[Option<Value>],
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
        started: (u64, u64),
    ) -> Result<Option<NativeMapSeedFrame>, InvokeError> {
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        let mut fields = [None; 32];
        let field_count = match self.template.output().body() {
            SchemaBody::Tuple(items) if items.len() <= 32 => {
                for (slot, item) in fields.iter_mut().zip(items.iter()) {
                    *slot = Some(item);
                }
                items.len()
            }
            SchemaBody::Record(items) if items.len() <= 32 => {
                for (slot, item) in fields.iter_mut().zip(items.iter()) {
                    *slot = Some(&item.schema);
                }
                items.len()
            }
            _ => return Ok(None),
        };
        // Bound geometry BEFORE payload inspection or optional owned allocation.
        let metadata_bytes = add(
            size_of::<NativeMapSeedFrame>() as u64,
            (inputs.len() as u64)
                .checked_mul((2 * size_of::<Option<Value>>()) as u64)
                .ok_or(InvokeError::InvalidInput)?,
        )?;
        work.charge(
            add(
                metadata_bytes
                    .checked_mul(8)
                    .ok_or(InvokeError::InvalidInput)?,
                512,
            )?,
            0,
        )?;
        if field_count != inputs.len()
            || self.template.inputs().len() != inputs.len()
            || contract.inputs.len() != inputs.len()
            || contract.outputs.len() != 1
            || !self.template.output().dimension_parameters().is_empty()
        {
            return Ok(None);
        }
        let mut map_ordinal = None;
        for (i, (field, input)) in fields[..field_count]
            .iter()
            .zip(self.template.inputs())
            .enumerate()
        {
            let field = field.ok_or(InvokeError::InvalidInput)?;
            if !input.dimension_parameters().is_empty() || input.body() != field {
                return Ok(None);
            }
            match field {
                SchemaBody::UnsignedInteger(IntegerWidth::W64) => {}
                body if map(body) && map_ordinal.is_none() => map_ordinal = Some(i),
                _ => return Ok(None),
            }
        }
        let Some(map_ordinal) = map_ordinal else {
            return Ok(None);
        };
        // The original frame validator checks exact current types, keys, real
        // account admission, shape, and its maintained output envelope.
        self.admit_buffer_frame(inputs, limits, work, coexistence)?;
        let memory = work.memory_budget();
        let mut scan = 0;
        for value in inputs.iter().flatten() {
            scan = add(
                scan,
                value
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?,
            )?;
        }
        work.charge(
            add(scan.checked_mul(64).ok_or(InvokeError::InvalidInput)?, 512)?,
            0,
        )?;
        let ValueData::Map(table) = inputs[map_ordinal].as_ref().unwrap().data() else {
            return Err(InvokeError::InvalidInput);
        };
        let mut key_bytes = 0;
        for entry in table.entries() {
            let (ValueData::String(key), ValueData::U64(_)) = (entry.key().data(), entry.value())
            else {
                return Err(InvokeError::InvalidInput);
            };
            key_bytes = add(key_bytes, key.len() as u64)?;
        }
        // Keep the SAME canonical constructor's logical clone/output floor,
        // even though its immutable child payloads are represented by pins.
        self.admit_map_owner_constructor(
            table.entries().len() as u64,
            key_bytes,
            limits,
            work,
            coexistence,
        )?;
        let mut logical_bytes = add(size_of::<Value>() as u64, size_of::<ValueData>() as u64)?;
        let mut logical_nodes = 2;
        for value in inputs.iter().flatten() {
            let footprint = value
                .retained_footprint(&self.schemas)
                .map_err(|_| InvokeError::InvalidInput)?;
            logical_bytes = add(
                logical_bytes,
                footprint
                    .retained_bytes
                    .checked_sub(size_of::<Value>() as u64)
                    .ok_or(InvokeError::InvalidInput)?,
            )?;
            logical_nodes = add(
                logical_nodes,
                footprint
                    .node_count
                    .checked_sub(1)
                    .ok_or(InvokeError::InvalidInput)?,
            )?;
        }
        let context =
            crate::resident::budget::native_turn_plan_cache::NativeProjectionIndexContext::new(
                work,
                limits,
                coexistence,
            );
        if context.check_capacity(metadata_bytes, 1).is_err() {
            return Ok(None);
        }
        let memory = memory.with_limits(
            limits.max_temporary_bytes.unwrap(),
            limits.max_retained_nodes.unwrap(),
        );
        let Ok(metadata) = memory.reserve_capacity(metadata_bytes) else {
            return Ok(None);
        };
        let Ok(nodes) = memory.reserve_nodes(1) else {
            return Ok(None);
        };
        let mut children = Vec::new();
        if children.try_reserve_exact(inputs.len()).is_err() || children.capacity() != inputs.len()
        {
            return Ok(None);
        }
        children.extend(inputs.iter().cloned());
        let used = work.used();
        Ok(Some(NativeMapSeedFrame {
            children: children.into_boxed_slice(),
            map_ordinal,
            schema: contract.outputs[0].schema,
            key: self.output_key,
            logical_bytes,
            logical_nodes,
            source_factory: self.cache_identity.clone(),
            work: work.clone(),
            limits,
            spent: (
                used.0
                    .checked_sub(started.0)
                    .ok_or(InvokeError::InvalidInput)?,
                used.1
                    .checked_sub(started.1)
                    .ok_or(InvokeError::InvalidInput)?,
            ),
            metadata,
            nodes,
        }))
    }
    /// Consume once, retiring optional metadata before the ordinary factory.
    /// The already-spent source prefix is neither refunded nor granted again.
    pub fn materialize_map_seed(
        &self,
        frame: NativeMapSeedFrame,
        work: &NativeLibraryWorkBudget,
    ) -> Result<Value, InvokeError> {
        frame.check_account(work)?;
        if !Arc::ptr_eq(&self.cache_identity, &frame.source_factory) || self.output_key != frame.key
        {
            return Err(InvokeError::InvalidInput);
        }
        let NativeMapSeedFrame {
            children,
            mut limits,
            spent,
            metadata,
            nodes,
            ..
        } = frame;
        limits.max_compute_work = Some(
            limits
                .max_compute_work
                .unwrap()
                .checked_sub(spent.0)
                .ok_or(InvokeError::InvalidInput)?,
        );
        limits.max_comparison_work = Some(
            limits
                .max_comparison_work
                .unwrap()
                .checked_sub(spent.1)
                .ok_or(InvokeError::InvalidInput)?,
        );
        // Move headers into bounded stack storage before retiring the pin
        // array and its real allocation claims. No unclaimed backing survives.
        let mut children = children;
        let arity = children.len();
        let mut current: [Option<Value>; 32] = std::array::from_fn(|_| None);
        for (to, from) in current.iter_mut().zip(children.iter_mut()) {
            *to = from.take();
        }
        drop((children, metadata, nodes));
        let mut output = None;
        self.invoke_current(&current[..arity], &mut output, limits, work)?;
        output.ok_or(InvokeError::InvalidInput)
    }
}
impl NativeMapSeedFrame {
    pub fn schema(&self) -> SchemaId {
        self.schema
    }
    pub fn schema_key(&self) -> SchemaKey {
        self.key
    }
    pub fn logical_retained_bytes(&self) -> u64 {
        self.logical_bytes
    }
    pub fn logical_nodes(&self) -> u64 {
        self.logical_nodes
    }
    pub fn metadata_demand(&self) -> ResourceDemand {
        ResourceDemand {
            turn_peak_bytes: self.metadata.capacity_bytes(),
            retained_nodes: self.nodes.nodes(),
            ..Default::default()
        }
    }
    pub fn check_account(&self, work: &NativeLibraryWorkBudget) -> Result<(), InvokeError> {
        if !self.work.shares_account(work) || self.metadata.budget() != work.memory_budget() {
            return Err(InvokeError::InvalidInput);
        }
        // Also reject a foreign currently installed relative account.
        work.has_active_call_limit()?;
        Ok(())
    }
    pub fn children(&self) -> &[Option<Value>] {
        &self.children
    }
    pub fn map_ordinal(&self) -> usize {
        self.map_ordinal
    }
}

#[cfg(all(test, any(feature = "full_source", feature = "full_compiler")))]
#[path = "native_collection_map_seed_tests.rs"]
mod tests;
