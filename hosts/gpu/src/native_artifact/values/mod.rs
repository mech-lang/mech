//! Typed managed transport for already-admitted immutable native library values.
//! This is storage, not a graph executor or permission to run a library call.
//! Current-visit and live geometry proof remain the compiled caller's duty.
use crate::native_artifact::memory::{NativeTypedStorage, NativeWordStorage};
use crate::native_artifact::{NativeExecutionError, NativeExecutionPhase, NativeWorkMeter};
use mech_core::{ManagedMemoryBudget, MemoryDomainId, SchemaId, SchemaKey, Value};

/// Descriptive slot type from the checked native plan. It grants no execution
/// authority; every inserted Value must already have canonical validation and
/// complete immutable ownership in the invocation's same memory account.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct NativeValueSlot {
    pub schema: SchemaId,
    pub key: SchemaKey,
}

/// Invocation-local opaque identity. No integer or raw-pointer constructor is
/// exposed; slot reuse and another arena cannot revive a stale handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeValueHandle {
    domain: MemoryDomainId,
    slot: u32,
    generation: u64,
}

#[cfg(feature = "canonical-native-program-runtime")]
pub(in crate::native_artifact) struct PreparedValuePut {
    handle: NativeValueHandle,
    retained: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeValueError {
    Base(NativeExecutionError),
    InvalidSlot,
    TypeMismatch,
    UnadmittedPayload,
    StaleHandle,
    GenerationExhausted,
    WrongWorkLedger,
}
impl From<NativeExecutionError> for NativeValueError {
    fn from(error: NativeExecutionError) -> Self {
        Self::Base(error)
    }
}
impl std::fmt::Display for NativeValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native value transport: {self:?}")
    }
}
impl std::error::Error for NativeValueError {}

/// Both handle lanes and generation lanes are actual planned Host arenas on
/// the caller's existing finite account. Values retain their existing shared
/// root ownership; this arena never imports or traverses unadmitted payloads.
pub struct NativeValueArena<'a> {
    values: NativeTypedStorage<Option<Value>>,
    generations: NativeWordStorage,
    types: &'a [NativeValueSlot],
    budget: ManagedMemoryBudget,
    work_identity: u64,
}
impl<'a> NativeValueArena<'a> {
    pub fn new(
        types: &'a [NativeValueSlot],
        max_transport_bytes: u64,
        budget: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Self, NativeValueError> {
        let count = u32::try_from(types.len()).map_err(|_| NativeValueError::InvalidSlot)?;
        let bytes = u64::from(count)
            .checked_mul(
                (core::mem::size_of::<Option<Value>>() + core::mem::size_of::<u64>()) as u64,
            )
            .ok_or(NativeExecutionError::Descriptor {
                reason: "native value transport size overflows",
            })?;
        if bytes > max_transport_bytes {
            return Err(NativeExecutionError::TransportLimit {
                required: bytes,
                limit: max_transport_bytes,
            }
            .into());
        }
        work.check_memory_budget(budget)?;
        // Covers both initialization passes and their bounded metadata. This
        // debits the caller's ledger, not a fresh per-arena work allowance.
        let setup = u64::from(count)
            .checked_mul(24)
            .and_then(|n| n.checked_add(256))
            .ok_or(NativeExecutionError::Descriptor {
                reason: "native value setup work overflows",
            })?;
        work.charge(NativeExecutionPhase::Setup, setup)?;
        let values = NativeTypedStorage::new_values(types.len(), max_transport_bytes, budget)?;
        let generations = NativeWordStorage::new(types.len(), max_transport_bytes, Some(budget))?;
        work.bind_memory_budget(budget)?;
        Ok(Self {
            values,
            generations,
            types,
            budget: budget.clone(),
            work_identity: work.identity(),
        })
    }

    /// Prepay one fixed-shape native producer's eventual slot installation.
    /// The ticket binds this arena generation, exact type, same account and
    /// retained bound. Installing it performs no new resource admission.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn prepare_put(
        &self,
        slot: u32,
        example: &Value,
        work: &mut NativeWorkMeter,
    ) -> Result<PreparedValuePut, NativeValueError> {
        self.check_work(work)?;
        let index = slot as usize;
        let ty = self.types.get(index).ok_or(NativeValueError::InvalidSlot)?;
        if example.schema() != ty.schema || example.schema_key() != ty.key {
            return Err(NativeValueError::TypeMismatch);
        }
        let retained = example
            .memory_budget_retained_bytes(&self.budget)
            .ok_or(NativeValueError::UnadmittedPayload)?;
        let generation = self.generations.values()[index]
            .checked_add(1)
            .ok_or(NativeValueError::GenerationExhausted)?;
        let retirement = retained
            .checked_mul(4)
            .and_then(|n| n.checked_add(64))
            .ok_or(NativeExecutionError::Descriptor {
                reason: "native value retirement work overflows",
            })?;
        work.charge(NativeExecutionPhase::Setup, retirement)?;
        Ok(PreparedValuePut {
            handle: NativeValueHandle {
                domain: self.values.domain_id(),
                slot,
                generation,
            },
            retained,
        })
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn put_prepared(
        &mut self,
        ticket: PreparedValuePut,
        value: Value,
    ) -> Result<(), NativeValueError> {
        let index = ticket.handle.slot as usize;
        let ty = self.types.get(index).ok_or(NativeValueError::InvalidSlot)?;
        if ticket.handle.domain != self.values.domain_id()
            || self.generations.values()[index].checked_add(1) != Some(ticket.handle.generation)
        {
            return Err(NativeValueError::StaleHandle);
        }
        if value.schema() != ty.schema || value.schema_key() != ty.key {
            return Err(NativeValueError::TypeMismatch);
        }
        if !value
            .memory_budget_retained_bytes(&self.budget)
            .is_some_and(|bytes| bytes <= ticket.retained)
        {
            return Err(NativeValueError::UnadmittedPayload);
        }
        self.values.values_mut()[index] = Some(value);
        self.generations.values_mut()[index] = ticket.handle.generation;
        Ok(())
    }

    /// All fallible checks precede replacing an existing slot. No payload
    /// clone, equality traversal or post-allocation accounting is performed.
    pub fn put(
        &mut self,
        slot: u32,
        value: &Value,
        work: &mut NativeWorkMeter,
    ) -> Result<NativeValueHandle, NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        let index = slot as usize;
        let ty = self.types.get(index).ok_or(NativeValueError::InvalidSlot)?;
        if value.schema() != ty.schema || value.schema_key() != ty.key {
            return Err(NativeValueError::TypeMismatch);
        }
        let retained = value
            .memory_budget_retained_bytes(&self.budget)
            .ok_or(NativeValueError::UnadmittedPayload)?;
        let generation = self.generations.values()[index]
            .checked_add(1)
            .ok_or(NativeValueError::GenerationExhausted)?;
        // Prepay retirement before retaining a root: eventual replacement or
        // arena teardown may release its last owner, including nested/schema
        // destructors. The complete cached ownership bound includes all those
        // retained nodes. Failed puts only borrow and never destroy the input.
        let retirement = retained
            .checked_mul(4)
            .and_then(|n| n.checked_add(32))
            .ok_or(NativeExecutionError::Descriptor {
                reason: "native value retirement work overflows",
            })?;
        work.charge(NativeExecutionPhase::Setup, retirement)?;
        self.values.values_mut()[index] = Some(value.clone());
        self.generations.values_mut()[index] = generation;
        Ok(NativeValueHandle {
            domain: self.values.domain_id(),
            slot,
            generation,
        })
    }

    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(in crate::native_artifact) fn test_current_value(&self, slot: u32) -> Option<&Value> {
        self.values.values().get(slot as usize)?.as_ref()
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(in crate::native_artifact) fn test_handle_value(
        &self,
        handle: NativeValueHandle,
    ) -> Option<&Value> {
        self.values.values().get(self.index(handle).ok()?)?.as_ref()
    }

    fn index(&self, handle: NativeValueHandle) -> Result<usize, NativeValueError> {
        let index = handle.slot as usize;
        if handle.domain != self.values.domain_id()
            || self.generations.values().get(index) != Some(&handle.generation)
            || self
                .values
                .values()
                .get(index)
                .and_then(Option::as_ref)
                .is_none()
        {
            return Err(NativeValueError::StaleHandle);
        }
        Ok(index)
    }

    fn check_work(&self, work: &NativeWorkMeter) -> Result<(), NativeValueError> {
        if work.identity() != self.work_identity {
            return Err(NativeValueError::WrongWorkLedger);
        }
        work.check_memory_budget(&self.budget)?;
        Ok(())
    }

    pub fn get(
        &self,
        handle: NativeValueHandle,
        work: &mut NativeWorkMeter,
    ) -> Result<&Value, NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 16)?;
        let index = self.index(handle)?;
        self.values.values()[index]
            .as_ref()
            .ok_or(NativeValueError::StaleHandle)
    }

    /// Private generated-caller access. The emitted CFG/current-visit proof
    /// determines the slot; no public integer-to-handle capability is exposed.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn copy_current(
        &self,
        slot: u32,
        work: &mut NativeWorkMeter,
    ) -> Result<Value, NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        self.values
            .values()
            .get(slot as usize)
            .and_then(Option::as_ref)
            .cloned()
            .ok_or(NativeValueError::StaleHandle)
    }

    /// Generated reference copy of an already admitted current owner. Every
    /// populated source was inserted by put (or this operation), so its one
    /// eventual payload/schema destruction is already prepaid. Adding an Arc
    /// alias needs only fixed read, publication and alias-retirement work; it
    /// creates no new payload or budget wrapper. Destination retirement was
    /// prepaid by its own insertion. All checks precede either replacement.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn copy_current_to(
        &mut self,
        source: u32,
        target: u32,
        work: &mut NativeWorkMeter,
    ) -> Result<NativeValueHandle, NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        let value = self
            .values
            .values()
            .get(source as usize)
            .ok_or(NativeValueError::InvalidSlot)?
            .as_ref()
            .ok_or(NativeValueError::StaleHandle)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        let index = target as usize;
        let ty = self.types.get(index).ok_or(NativeValueError::InvalidSlot)?;
        if value.schema() != ty.schema || value.schema_key() != ty.key {
            return Err(NativeValueError::TypeMismatch);
        }
        value
            .memory_budget_retained_bytes(&self.budget)
            .ok_or(NativeValueError::UnadmittedPayload)?;
        let generation = self.generations.values()[index]
            .checked_add(1)
            .ok_or(NativeValueError::GenerationExhausted)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        let value = value.clone();
        self.values.values_mut()[index] = Some(value);
        self.generations.values_mut()[index] = generation;
        Ok(NativeValueHandle {
            domain: self.values.domain_id(),
            slot: target,
            generation,
        })
    }

    /// Private read inside an already prepaid generated operation. The caller
    /// must include this fixed-size slot/generation/refcount work in that
    /// operation's complete bound before entering this leaf. It cannot publish
    /// a value, create fuel, or replace the arena's original invocation ledger.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn copy_current_prepaid(
        &self,
        slot: u32,
        work: &NativeWorkMeter,
    ) -> Result<Option<(Value, NativeValueHandle)>, NativeValueError> {
        self.check_work(work)?;
        let index = slot as usize;
        let value = self
            .values
            .values()
            .get(index)
            .ok_or(NativeValueError::InvalidSlot)?;
        Ok(value.as_ref().map(|value| {
            (
                value.clone(),
                NativeValueHandle {
                    domain: self.values.domain_id(),
                    slot,
                    generation: self.generations.values()[index],
                },
            )
        }))
    }

    /// Capture an existing current generation without publishing/replacing a
    /// slot. Only the certified generated caller can obtain this handle.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn current_handle(
        &self,
        slot: u32,
        work: &mut NativeWorkMeter,
    ) -> Result<NativeValueHandle, NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 16)?;
        let index = slot as usize;
        if self
            .values
            .values()
            .get(index)
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(NativeValueError::StaleHandle);
        }
        Ok(NativeValueHandle {
            domain: self.values.domain_id(),
            slot,
            generation: self.generations.values()[index],
        })
    }

    /// Constant-time ownership observation, never payload access or a value
    /// read capability. Includes an old scalar until replacement actually drops it.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn current_retained_bytes(
        &self,
        slot: u32,
        work: &mut NativeWorkMeter,
    ) -> Result<Option<u64>, NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 16)?;
        self.values
            .values()
            .get(slot as usize)
            .ok_or(NativeValueError::InvalidSlot)?
            .as_ref()
            .map(|value| {
                value
                    .memory_budget_retained_bytes(&self.budget)
                    .ok_or(NativeValueError::UnadmittedPayload)
            })
            .transpose()
    }

    /// Only the certified generated caller may retire a lexical local. Payload
    /// retirement was prepaid by put; invalidate its handle before releasing it.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn retire_current(
        &mut self,
        slot: u32,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        let index = slot as usize;
        let value = self
            .values
            .values_mut()
            .get_mut(index)
            .ok_or(NativeValueError::InvalidSlot)?;
        if value.is_some() {
            let next = self.generations.values()[index]
                .checked_add(1)
                .ok_or(NativeValueError::GenerationExhausted)?;
            self.generations.values_mut()[index] = next;
            *value = None;
        }
        Ok(())
    }

    /// Retirement only, not an ownership/last-use proof. The generated caller
    /// supplies its immutable complete-use inventory and retains `owner` while
    /// matching current frozen allocations. A later generation is never swept.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn retire_current_matching(
        &mut self,
        slot: u32,
        owner: &Value,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 16)?;
        let value = self
            .values
            .values()
            .get(slot as usize)
            .ok_or(NativeValueError::InvalidSlot)?;
        if value
            .as_ref()
            .is_some_and(|value| value.shares_frozen_storage(owner))
        {
            self.retire_current(slot, work)?;
        }
        Ok(())
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn publish_range(
        &self,
        start: usize,
        outputs: &mut [Option<Value>],
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeValueError> {
        self.check_work(work)?;
        let end = start
            .checked_add(outputs.len())
            .ok_or(NativeValueError::InvalidSlot)?;
        let values = self
            .values
            .values()
            .get(start..end)
            .ok_or(NativeValueError::InvalidSlot)?;
        work.charge(
            NativeExecutionPhase::Completion,
            (outputs.len() as u64)
                .checked_mul(16)
                .and_then(|n| n.checked_add(32))
                .ok_or(NativeValueError::InvalidSlot)?,
        )?;
        if values.iter().any(Option::is_none) {
            return Err(NativeValueError::StaleHandle);
        }
        // All fallible checks/debits precede allocation-free publication.
        // The caller has already prepaid retirement of the previous outputs.
        for (target, value) in outputs.iter_mut().zip(values) {
            *target = Some(value.as_ref().unwrap().clone());
        }
        Ok(())
    }

    /// Moving a value out retains its real immutable ownership. It invalidates
    /// the handle before any later slot reuse; generation wrap fails closed.
    pub fn take(
        &mut self,
        handle: NativeValueHandle,
        work: &mut NativeWorkMeter,
    ) -> Result<Value, NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        let index = self.index(handle)?;
        let next = handle
            .generation
            .checked_add(1)
            .ok_or(NativeValueError::GenerationExhausted)?;
        self.generations.values_mut()[index] = next;
        self.values.values_mut()[index]
            .take()
            .ok_or(NativeValueError::StaleHandle)
    }
}

#[cfg(test)]
mod tests;
