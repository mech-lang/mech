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

/// Private all-path source-slot inventory. Constructed once from the checked
/// worker proof and borrowed by every fresh worker. It is never image authority.
#[derive(Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct NativeValueSlotMap {
    indices: Box<[u32]>,
    physical: usize,
}
impl NativeValueSlotMap {
    #[cfg(any(
        feature = "canonical-native",
        feature = "canonical-native-program-runtime"
    ))]
    pub(in crate::native_artifact) fn from_inventory(
        inventory: &[bool],
    ) -> Result<Self, &'static str> {
        let prefix = inventory
            .iter()
            .rposition(|&live| live)
            .map_or(0, |i| i + 1);
        let mut indices = Vec::with_capacity(prefix);
        let mut physical = 0usize;
        for &live in &inventory[..prefix] {
            indices.push(if live {
                let index = u32::try_from(physical)
                    .ok()
                    .filter(|&index| index != u32::MAX)
                    .ok_or("worker dense index exceeds u32 sentinel")?;
                physical += 1;
                index
            } else {
                u32::MAX
            });
        }
        Ok(Self {
            indices: indices.into_boxed_slice(),
            physical,
        })
    }
    pub(in crate::native_artifact) fn physical_slots(&self) -> usize {
        self.physical
    }
    pub(in crate::native_artifact) fn logical_prefix(&self) -> usize {
        self.indices.len()
    }
}

/// Both handle lanes and generation lanes are actual planned Host arenas on
/// the caller's existing finite account. Values retain their existing shared
/// root ownership; this arena never imports or traverses unadmitted payloads.
pub struct NativeValueArena<'a> {
    values: NativeTypedStorage<Option<Value>>,
    generations: NativeWordStorage,
    types: &'a [NativeValueSlot],
    slots: Option<&'a NativeValueSlotMap>,
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
        Self::new_with_prefix(types, types.len(), max_transport_bytes, budget, work)
    }
    /// A private compiler-derived physical prefix. The original full type
    /// inventory continues to set transport limits, work and both byte claims.
    pub(in crate::native_artifact) fn new_with_prefix(
        types: &'a [NativeValueSlot],
        physical: usize,
        max_transport_bytes: u64,
        budget: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Self, NativeValueError> {
        Self::new_storage(types, physical, None, max_transport_bytes, budget, work)
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn new_with_worker_slots(
        types: &'a [NativeValueSlot],
        slots: Option<&'a NativeValueSlotMap>,
        max_transport_bytes: u64,
        budget: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Self, NativeValueError> {
        match slots {
            Some(slots) => Self::new_storage(
                types,
                slots.physical_slots(),
                Some(slots),
                max_transport_bytes,
                budget,
                work,
            ),
            None => Self::new(types, max_transport_bytes, budget, work),
        }
    }

    fn new_storage(
        types: &'a [NativeValueSlot],
        physical: usize,
        slots: Option<&'a NativeValueSlotMap>,
        max_transport_bytes: u64,
        budget: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Self, NativeValueError> {
        let count = u32::try_from(types.len()).map_err(|_| NativeValueError::InvalidSlot)?;
        let logical_prefix = slots.map_or(physical, NativeValueSlotMap::logical_prefix);
        if physical > logical_prefix
            || logical_prefix > types.len()
            || slots.is_some_and(|slots| slots.physical_slots() != physical)
        {
            return Err(NativeValueError::InvalidSlot);
        }
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
        let values = NativeTypedStorage::new_values_compact(
            types.len(),
            physical,
            max_transport_bytes,
            budget,
        )?;
        let generations =
            NativeWordStorage::new_compact(types.len(), physical, max_transport_bytes, budget)?;
        work.bind_memory_budget(budget)?;
        Ok(Self {
            values,
            generations,
            // Types, handles and generated word coordinates remain global.
            // The checked map affects only these two private physical banks.
            types: &types[..logical_prefix],
            slots,
            budget: budget.clone(),
            work_identity: work.identity(),
        })
    }

    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(in crate::native_artifact) fn test_physical_slots(&self) -> (usize, usize, usize) {
        (
            self.values.values().len(),
            self.generations.values().len(),
            self.types.len(),
        )
    }

    fn logical_type(&self, slot: u32) -> Result<&NativeValueSlot, NativeValueError> {
        self.types
            .get(slot as usize)
            .ok_or(NativeValueError::InvalidSlot)
    }

    fn physical_index(&self, slot: u32) -> Option<usize> {
        if slot as usize >= self.types.len() {
            return None;
        }
        match self.slots {
            Some(slots) => slots
                .indices
                .get(slot as usize)
                .copied()
                .filter(|&index| index != u32::MAX)
                .and_then(|index| {
                    ((index as usize) < self.values.values().len()).then_some(index as usize)
                }),
            None => Some(slot as usize),
        }
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    fn current(&self, slot: u32) -> Result<Option<&Value>, NativeValueError> {
        self.logical_type(slot)?;
        Ok(self
            .physical_index(slot)
            .and_then(|index| self.values.values()[index].as_ref()))
    }

    fn generation(&self, slot: u32) -> Result<u64, NativeValueError> {
        self.logical_type(slot)?;
        Ok(self
            .physical_index(slot)
            .map_or(0, |index| self.generations.values()[index]))
    }

    // Only a proved worker slot can be written. Check this after the original
    // fallible validation/debits; holes retain their old empty/zero previews.
    fn writable_index(&self, slot: u32) -> Result<usize, NativeValueError> {
        self.physical_index(slot)
            .ok_or(NativeValueError::InvalidSlot)
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
        let ty = self.logical_type(slot)?;
        if example.schema() != ty.schema || example.schema_key() != ty.key {
            return Err(NativeValueError::TypeMismatch);
        }
        let retained = example
            .memory_budget_retained_bytes(&self.budget)
            .ok_or(NativeValueError::UnadmittedPayload)?;
        let generation = self
            .generation(slot)?
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
        let slot = ticket.handle.slot;
        let ty = self.logical_type(slot)?;
        if ticket.handle.domain != self.values.domain_id()
            || self.generation(slot)?.checked_add(1) != Some(ticket.handle.generation)
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
        let index = self.writable_index(slot)?;
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
        let ty = self.logical_type(slot)?;
        if value.schema() != ty.schema || value.schema_key() != ty.key {
            return Err(NativeValueError::TypeMismatch);
        }
        let retained = value
            .memory_budget_retained_bytes(&self.budget)
            .ok_or(NativeValueError::UnadmittedPayload)?;
        let generation = self
            .generation(slot)?
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
        let index = self.writable_index(slot)?;
        self.values.values_mut()[index] = Some(value.clone());
        self.generations.values_mut()[index] = generation;
        Ok(NativeValueHandle {
            domain: self.values.domain_id(),
            slot,
            generation,
        })
    }

    /// Consume a worker's already-owned immutable capture into its fresh
    /// constructor slot. Pure preflight proves every check between the two
    /// original put debits before a bounded shared-lock callback begins.
    /// Refused preflight keeps the original put's checks and paid prefix.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn put_owned_worker_capture(
        &mut self,
        slot: u32,
        value: Value,
        work: &mut NativeWorkMeter,
    ) -> Result<NativeValueHandle, NativeValueError> {
        self.check_work(work)?;
        let prepared = (|| {
            let index = self.physical_index(slot)?;
            let ty = self.logical_type(slot).ok()?;
            if value.schema() != ty.schema
                || value.schema_key() != ty.key
                || !self.values.values().get(index)?.is_none()
                || self.generations.values().get(index) != Some(&0)
            {
                return None;
            }
            let retained = value.memory_budget_retained_bytes(&self.budget)?;
            Some((index, retained.checked_mul(4)?.checked_add(32)?))
        })();
        // The callback only borrows this outer owner. On either denied debit
        // it remains here until after the batch/escrow guard has been dropped.
        let mut owner = Some(value);
        if let Some((index, retirement)) = prepared {
            let handle = NativeValueHandle {
                domain: self.values.domain_id(),
                slot,
                generation: 1,
            };
            let outcome = work.with_inactive_worker_debit_batch(|batch| {
                let error = |error| {
                    NativeValueError::from(NativeWorkMeter::library_error(
                        NativeExecutionPhase::Setup,
                        error,
                    ))
                };
                batch.charge(32, 0).map_err(error)?;
                batch.charge(retirement, 0).map_err(error)?;
                // Freshness was checked before entering this callback. These
                // fixed writes drop no old owner and access no memory account.
                self.values.values_mut()[index] = owner.take();
                self.generations.values_mut()[index] = 1;
                Ok::<_, NativeValueError>(handle)
            });
            #[cfg(test)]
            if !matches!(&outcome, Ok(None)) {
                // work.used borrows the same engine account. This observation
                // would panic if the batch still held its account borrow; on
                // denial the still-present outer owner has not been dropped.
                OWNED_CAPTURE_BATCH_END.with(|seen| {
                    seen.set(Some((
                        owner.is_some(),
                        mech_engine::native_artifact::NativeLibraryWorkBudget::has_active_scope(),
                        work.used(),
                        self.budget.used_bytes(),
                    )))
                });
            }
            if let Some(handle) = outcome? {
                return Ok(handle);
            }
        }
        self.put(slot, owner.as_ref().unwrap(), work)
    }

    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(in crate::native_artifact) fn test_current_value(&self, slot: u32) -> Option<&Value> {
        self.current(slot).ok().flatten()
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(in crate::native_artifact) fn test_handle_value(
        &self,
        handle: NativeValueHandle,
    ) -> Option<&Value> {
        self.values.values().get(self.index(handle).ok()?)?.as_ref()
    }

    fn index(&self, handle: NativeValueHandle) -> Result<usize, NativeValueError> {
        if handle.domain != self.values.domain_id() {
            return Err(NativeValueError::StaleHandle);
        }
        let index = self
            .physical_index(handle.slot)
            .ok_or(NativeValueError::StaleHandle)?;
        if self.generations.values()[index] != handle.generation
            || self.values.values()[index].is_none()
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
        self.current(slot)
            .ok()
            .flatten()
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
        let value = self.current(source)?.ok_or(NativeValueError::StaleHandle)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        let ty = self.logical_type(target)?;
        if value.schema() != ty.schema || value.schema_key() != ty.key {
            return Err(NativeValueError::TypeMismatch);
        }
        value
            .memory_budget_retained_bytes(&self.budget)
            .ok_or(NativeValueError::UnadmittedPayload)?;
        let generation = self
            .generation(target)?
            .checked_add(1)
            .ok_or(NativeValueError::GenerationExhausted)?;
        work.charge(NativeExecutionPhase::Setup, 32)?;
        let index = self.writable_index(target)?;
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
        let value = self.current(slot)?;
        Ok(value.map(|value| {
            (
                value.clone(),
                NativeValueHandle {
                    domain: self.values.domain_id(),
                    slot,
                    generation: self.generation(slot).unwrap(),
                },
            )
        }))
    }

    /// Define one typed compiler-private logical value without a canonical
    /// snapshot. The opaque generation grants no `get` access; its real owner
    /// must remain in the selecting loan until canonicalization or retirement.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn define_virtual(
        &mut self,
        slot: u32,
        schema: SchemaId,
        key: SchemaKey,
        work: &mut NativeWorkMeter,
    ) -> Result<NativeValueHandle, NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 96)?;
        let ty = self.logical_type(slot)?;
        if ty.schema != schema || ty.key != key {
            return Err(NativeValueError::TypeMismatch);
        }
        let generation = self
            .generation(slot)?
            .checked_add(1)
            .ok_or(NativeValueError::GenerationExhausted)?;
        let index = self.writable_index(slot)?;
        self.values.values_mut()[index] = None;
        self.generations.values_mut()[index] = generation;
        Ok(NativeValueHandle {
            domain: self.values.domain_id(),
            slot,
            generation,
        })
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn check_virtual(
        &self,
        handle: NativeValueHandle,
        work: &mut NativeWorkMeter,
    ) -> Result<(), NativeValueError> {
        self.check_work(work)?;
        work.charge(NativeExecutionPhase::Setup, 16)?;
        if handle.domain != self.values.domain_id()
            || self.generation(handle.slot).ok() != Some(handle.generation)
            || !matches!(self.current(handle.slot), Ok(None))
        {
            return Err(NativeValueError::StaleHandle);
        }
        Ok(())
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
        if self.current(slot).ok().flatten().is_none() {
            return Err(NativeValueError::StaleHandle);
        }
        Ok(NativeValueHandle {
            domain: self.values.domain_id(),
            slot,
            generation: self.generation(slot)?,
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
        self.current(slot)?
            .map(|value| {
                value
                    .memory_budget_retained_bytes(&self.budget)
                    .ok_or(NativeValueError::UnadmittedPayload)
            })
            .transpose()
    }

    /// Pure current-entry availability, geometry and retirement previews.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn native_entry_preview(
        &self,
        slot: u32,
        work: &NativeWorkMeter,
    ) -> Result<Option<&Value>, NativeValueError> {
        // Pure current availability/geometry preview, analogous to private
        // frame current_leaf. It grants neither mutation nor an entry loan.
        // The caller must fund and validate its selected view before execution.
        self.check_work(work)?;
        self.current(slot)
    }
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn native_retirement_available(
        &self,
        slot: u32,
        work: &NativeWorkMeter,
    ) -> Result<bool, NativeValueError> {
        self.check_work(work)?;
        let value = self.current(slot)?;
        Ok(value.is_none() || self.generation(slot)? != u64::MAX)
    }

    /// Pure preview for an escaping alias from one native piece. No ticket or
    /// permission survives this call: the ordinary put still performs all
    /// checks and charges at the piece's maintained exit. Current empty slots
    /// exclude stale canonical definitions while the alias is private.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn native_alias_put_work(
        &self,
        slot: u32,
        value: &Value,
        work: &NativeWorkMeter,
    ) -> Result<Option<u64>, NativeValueError> {
        self.check_work(work)?;
        let ty = self.logical_type(slot)?;
        if self.current(slot)?.is_some()
            || self.generation(slot)? == u64::MAX
            || value.schema() != ty.schema
            || value.schema_key() != ty.key
        {
            return Ok(None);
        }
        Ok(value
            .memory_budget_retained_bytes(&self.budget)
            .and_then(|bytes| bytes.checked_mul(4))
            .and_then(|bytes| bytes.checked_add(64)))
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
        self.retire_current_value(slot)
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    pub(in crate::native_artifact) fn retire_current_prepaid(
        &mut self,
        slot: u32,
        paid: &mut crate::native_artifact::work::PaidRetirements<'_>,
    ) -> Result<(), NativeValueError> {
        self.check_work(paid.meter())?;
        paid.pay(32)?;
        self.retire_current_value(slot)
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    fn retire_current_value(&mut self, slot: u32) -> Result<(), NativeValueError> {
        #[cfg(test)]
        PHYSICAL_RETIREMENTS.with(|count| count.set(count.get() + 1));
        self.logical_type(slot)?;
        let Some(index) = self.physical_index(slot) else {
            return Ok(());
        };
        let value = &mut self.values.values_mut()[index];
        if value.is_some() {
            let next = self.generations.values()[index]
                .checked_add(1)
                .ok_or(NativeValueError::GenerationExhausted)?;
            self.generations.values_mut()[index] = next;
            *value = None;
        }
        Ok(())
    }

    #[cfg(all(test, feature = "canonical-native-program-runtime"))]
    pub(in crate::native_artifact) fn take_retirement_effect_count() -> u64 {
        PHYSICAL_RETIREMENTS.with(|count| count.replace(0))
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
        if self
            .current(slot)?
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
        if end > self.types.len() || start > end {
            return Err(NativeValueError::InvalidSlot);
        }
        work.charge(
            NativeExecutionPhase::Completion,
            (outputs.len() as u64)
                .checked_mul(16)
                .and_then(|n| n.checked_add(32))
                .ok_or(NativeValueError::InvalidSlot)?,
        )?;
        if (start..end).any(|slot| self.current(slot as u32).ok().flatten().is_none()) {
            return Err(NativeValueError::StaleHandle);
        }
        // All fallible checks/debits precede allocation-free publication.
        // The caller has already prepaid retirement of the previous outputs.
        for (offset, target) in outputs.iter_mut().enumerate() {
            *target = Some(
                self.current((start + offset) as u32)
                    .unwrap()
                    .unwrap()
                    .clone(),
            );
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

#[cfg(all(test, feature = "canonical-native-program-runtime"))]
thread_local! {
    static PHYSICAL_RETIREMENTS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

#[cfg(all(test, feature = "canonical-native-program-runtime"))]
thread_local! {
    static OWNED_CAPTURE_BATCH_END: core::cell::Cell<Option<(bool, bool, u64, u64)>> = const { core::cell::Cell::new(None) };
}

#[cfg(test)]
mod tests;
