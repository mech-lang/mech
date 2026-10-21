//! Typed managed transport for already-admitted immutable native library values.
//! This is storage, not a graph executor or permission to run a library call.
//! Current-visit and live geometry proof remain the compiled caller's duty.
use super::memory::{NativeTypedStorage, NativeWordStorage};
use super::{NativeExecutionError, NativeExecutionPhase, NativeWorkMeter};
use mech_core::{ManagedMemoryBudget, MemoryDomainId, SchemaId, SchemaKey, Value};

/// Descriptive slot type from the checked native plan. It grants no execution
/// authority; every inserted Value must already have canonical validation and
/// complete immutable ownership in the invocation's same memory account.
#[derive(Clone, Copy, Debug)]
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
    pub(super) fn test_current_value(&self, slot: u32) -> Option<&Value> {
        self.values.values().get(slot as usize)?.as_ref()
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub(super) fn test_handle_value(&self, handle: NativeValueHandle) -> Option<&Value> {
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
    pub(super) fn copy_current(
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

    /// Capture an existing current generation without publishing/replacing a
    /// slot. Only the certified generated caller can obtain this handle.
    #[cfg(feature = "canonical-native-program-runtime")]
    pub(super) fn current_handle(
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
    pub(super) fn current_retained_bytes(
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
    pub(super) fn retire_current(
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
    pub(super) fn retire_current_matching(
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
    pub(super) fn publish_range(
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
mod tests {
    use super::*;
    use mech_core::snapshot::{MapEntryDraft, SnapshotValidationContext};
    use mech_core::{
        SchemaBody, SchemaDraft, SchemaTableBuilder, ValueData, ValueDataDraft, ValueDraft,
    };
    const TRANSPORT_LIMIT: u64 = 1024 * 1024;

    fn value(body: SchemaBody, data: ValueDataDraft) -> (NativeValueSlot, Value) {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let id = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema: id,
            shape_values: Box::new([]),
            data,
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        (
            NativeValueSlot {
                schema: id,
                key: value.schema_key(),
            },
            value,
        )
    }
    fn admit(value: &Value, budget: &ManagedMemoryBudget) -> Value {
        let mut reservation = budget
            .reserve_capacity(value.memory_budget_admission_bytes(budget).unwrap())
            .unwrap();
        value.clone().into_memory_budget(&mut reservation).unwrap()
    }
    fn text(text: &str) -> (NativeValueSlot, Value) {
        value(SchemaBody::String, ValueDataDraft::String(text.into()))
    }

    #[cfg(feature = "canonical-native-program-runtime")]
    #[test]
    fn private_local_retirement_invalidates_only_its_exact_slot_and_ledger() {
        let budget = ManagedMemoryBudget::new(1_000_000);
        let (ty, raw) = text("local");
        let owned = admit(&raw, &budget);
        let types = [ty, ty];
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
        let local = arena.put(0, &owned, &mut work).unwrap();
        let capture = arena.put(1, &owned, &mut work).unwrap();
        let before = budget.used_bytes();
        let mut foreign = NativeWorkMeter::new(1_000_000).unwrap();
        assert_eq!(
            arena.retire_current(0, &mut foreign).unwrap_err(),
            NativeValueError::WrongWorkLedger
        );
        assert!(arena.get(local, &mut work).is_ok());
        arena.retire_current(0, &mut work).unwrap();
        assert_eq!(
            arena.get(local, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert!(arena.get(capture, &mut work).is_ok());
        assert_eq!(
            budget.used_bytes(),
            before,
            "the live capture keeps its shared payload"
        );
        arena.retire_current(0, &mut work).unwrap();
        let replacement = arena.put(0, &owned, &mut work).unwrap();
        assert_ne!(replacement, local);
        assert_eq!(
            arena.get(local, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        drop((owned, arena));
        assert_eq!(budget.used_bytes(), 0);
    }

    #[test]
    fn strings_maps_and_full_u64_payloads_use_real_same_account_managed_slots() {
        let budget = ManagedMemoryBudget::new(1_000_000);
        let (text_ty, text_value) = text("unicode λ\0prefix");
        let (map_ty, map_value) = value(
            SchemaBody::Map {
                key: Box::new(SchemaBody::String),
                value: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
                cardinality: mech_core::DimensionExpr::Constant(1).into(),
            },
            ValueDataDraft::Map(
                vec![MapEntryDraft {
                    items: vec![
                        ValueDataDraft::String("λ".into()),
                        ValueDataDraft::U64(u64::MAX),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        );
        let text_value = admit(&text_value, &budget);
        let map_value = admit(&map_value, &budget);
        let retained = budget.used_bytes();
        let types = [text_ty, map_ty];
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        {
            let mut arena =
                NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
            assert!(budget.used_bytes() > retained);
            let before_put = budget.used_bytes();
            let a = arena.put(0, &text_value, &mut work).unwrap();
            let b = arena.put(1, &map_value, &mut work).unwrap();
            assert_eq!(
                budget.used_bytes(),
                before_put,
                "retaining admitted roots must not import or duplicate their payload"
            );
            assert!(
                matches!(arena.get(a, &mut work).unwrap().data(), ValueData::String(s) if s.as_ref() == "unicode λ\0prefix")
            );
            assert_eq!(
                arena.get(b, &mut work).unwrap().schema_key(),
                map_value.schema_key()
            );
            let ValueData::Map(counts) = arena.get(b, &mut work).unwrap().data() else {
                panic!("expected canonical map")
            };
            assert!(
                matches!(counts.entries()[0].value(), ValueData::U64(count) if *count == u64::MAX)
            );
            assert_eq!(
                arena.take(b, &mut work).unwrap().schema_key(),
                map_value.schema_key()
            );
            assert_eq!(
                arena.get(b, &mut work).unwrap_err(),
                NativeValueError::StaleHandle
            );
        }
        assert_eq!(
            budget.used_bytes(),
            retained,
            "both realized transport arenas release their account claims"
        );
        drop(text_value);
        drop(map_value);
        assert_eq!(budget.used_bytes(), 0);
    }

    #[test]
    fn late_foreign_imports_do_not_expand_arena_replacement_or_teardown_work() {
        for replace in [false, true] {
            for later in [0, 16] {
                let budget = ManagedMemoryBudget::new(1_000_000);
                let (ty, raw) = text("small");
                let owned = admit(&raw, &budget);
                let types = [ty];
                let mut work = NativeWorkMeter::new(1_000_000).unwrap();
                let mut arena =
                    NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
                arena.put(0, &owned, &mut work).unwrap();
                drop(owned); // The arena owns A's last imported wrapper.
                let foreign = (0..later)
                    .map(|_| {
                        let account = ManagedMemoryBudget::new(1_000_000);
                        let value = admit(&raw, &account);
                        (account, value)
                    })
                    .collect::<Vec<_>>();
                if replace {
                    let (_, replacement) = text("other");
                    let replacement = admit(&replacement, &budget);
                    let retained = replacement.memory_budget_retained_bytes(&budget).unwrap();
                    let before = work.used();
                    arena.put(0, &replacement, &mut work).unwrap();
                    assert_eq!(work.used() - before, retained * 4 + 64);
                    drop(replacement);
                }
                drop(arena);
                assert_eq!(budget.used_bytes(), 0);
                for (account, value) in foreign {
                    assert!(value.memory_budget_retained_bytes(&account).is_some());
                    drop(value);
                    assert_eq!(account.used_bytes(), 0);
                }
            }
        }
    }

    #[test]
    fn slot_reuse_and_other_arenas_never_revalidate_old_handles() {
        let budget = ManagedMemoryBudget::new(1_000_000);
        let (ty, first) = text("first");
        let (_, second) = text("second");
        let first = admit(&first, &budget);
        let second = admit(&second, &budget);
        let types = [ty];
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        let mut a = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
        let mut b = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
        let old = a.put(0, &first, &mut work).unwrap();
        let current = a.put(0, &second, &mut work).unwrap();
        let foreign = b.put(0, &first, &mut work).unwrap();
        assert_eq!(
            a.get(old, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(
            a.get(foreign, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(
            b.get(current, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        a.take(current, &mut work).unwrap();
        let next = a.put(0, &first, &mut work).unwrap();
        assert_ne!(next, current);
        assert_eq!(
            a.get(current, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
    }

    #[test]
    fn wrong_types_unadmitted_and_foreign_budget_values_preserve_existing_slots() {
        let budget = ManagedMemoryBudget::new(1_000_000);
        let other_budget = ManagedMemoryBudget::new(1_000_000);
        let (ty, raw) = text("stable");
        let good = admit(&raw, &budget);
        let foreign = admit(&raw, &other_budget);
        let (_, wrong) = value(
            SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
            ValueDataDraft::U64(42),
        );
        let wrong = admit(&wrong, &budget);
        let types = [ty];
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
        let stable = arena.put(0, &good, &mut work).unwrap();
        let retained = budget.used_bytes();
        assert_eq!(
            arena.put(0, &raw, &mut work).unwrap_err(),
            NativeValueError::UnadmittedPayload
        );
        assert_eq!(
            arena.put(0, &foreign, &mut work).unwrap_err(),
            NativeValueError::UnadmittedPayload
        );
        assert_eq!(
            arena.put(0, &wrong, &mut work).unwrap_err(),
            NativeValueError::TypeMismatch
        );
        assert_eq!(
            arena.put(1, &good, &mut work).unwrap_err(),
            NativeValueError::InvalidSlot
        );
        assert_eq!(budget.used_bytes(), retained);
        assert_eq!(
            arena.get(stable, &mut work).unwrap().schema_key(),
            good.schema_key()
        );
    }

    #[test]
    fn setup_transport_shared_memory_and_cumulative_work_denials_fail_closed() {
        let budget = ManagedMemoryBudget::new(1_000_000);
        let (ty, raw) = text("retained");
        let good = admit(&raw, &budget);
        let types = [ty];
        let mut denied = NativeWorkMeter::new(0).unwrap();
        let before = budget.used_bytes();
        assert!(matches!(
            NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut denied),
            Err(NativeValueError::Base(
                NativeExecutionError::WorkLimit { .. }
            ))
        ));
        assert_eq!(budget.used_bytes(), before);
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        assert!(matches!(
            NativeValueArena::new(&types, 0, &budget, &mut work),
            Err(NativeValueError::Base(
                NativeExecutionError::TransportLimit { .. }
            ))
        ));
        assert_eq!(work.used(), 0);
        let no_memory = ManagedMemoryBudget::new(0);
        assert!(NativeValueArena::new(&types, TRANSPORT_LIMIT, &no_memory, &mut work).is_err());
        assert_eq!(no_memory.used_bytes(), 0);
        let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
        let handle = arena.put(0, &good, &mut work).unwrap();
        work.charge(NativeExecutionPhase::Setup, work.limit() - work.used())
            .unwrap();
        assert!(matches!(
            arena.put(0, &good, &mut work),
            Err(NativeValueError::Base(
                NativeExecutionError::WorkLimit { .. }
            ))
        ));
        assert!(arena.values.values()[0].is_some());
        assert_eq!(arena.generations.values()[0], handle.generation);
        assert_eq!(
            work.used(),
            work.limit(),
            "no failed call resets cumulative authority"
        );
        let mut fresh = NativeWorkMeter::new(1_000_000).unwrap();
        assert_eq!(
            arena.put(0, &good, &mut fresh).unwrap_err(),
            NativeValueError::WrongWorkLedger
        );
        assert_eq!(
            arena.get(handle, &mut fresh).unwrap_err(),
            NativeValueError::WrongWorkLedger
        );
        assert_eq!(
            arena.take(handle, &mut fresh).unwrap_err(),
            NativeValueError::WrongWorkLedger
        );
        assert_eq!(fresh.used(), 0);
    }

    #[test]
    fn arena_before_promotion_pins_memory_and_retains_the_same_live_handles_and_counter() {
        let memory = ManagedMemoryBudget::new(1_000_000);
        let other = ManagedMemoryBudget::new(1_000_000);
        let (ty, raw) = text("same invocation");
        let good = admit(&raw, &memory);
        let types = [ty];
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
        let handle = arena.put(0, &good, &mut work).unwrap();
        let before = work.used();
        let bytes = memory.used_bytes();
        assert!(
            work.shared_library_budget(NativeExecutionPhase::Setup, 10, &other)
                .is_err()
        );
        assert_eq!(work.used(), before);
        assert_eq!(memory.used_bytes(), bytes);
        assert_eq!(other.used_bytes(), 0);
        let library = work
            .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
            .unwrap();
        assert_eq!(library.used(), (before + 64, 0));
        assert_eq!(
            arena.get(handle, &mut work).unwrap().schema_key(),
            good.schema_key()
        );
        assert_eq!(library.used(), (before + 80, 0));
        library.charge(3, 2).unwrap();
        assert_eq!(work.used(), before + 83);
        let moved = arena.take(handle, &mut work).unwrap();
        assert!(moved.shares_frozen_storage(&good));
        drop((arena, work, library, good, moved));
        assert_eq!(memory.used_bytes(), 0);
    }

    #[test]
    fn promotion_before_arena_rejects_foreign_backing_before_work_or_allocation() {
        let memory = ManagedMemoryBudget::new(1_000_000);
        let other = ManagedMemoryBudget::new(1_000_000);
        let (ty, raw) = text("already shared");
        let good = admit(&raw, &memory);
        let types = [ty];
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        let library = work
            .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
            .unwrap();
        let bytes = memory.used_bytes();
        assert!(matches!(
            NativeValueArena::new(&types, TRANSPORT_LIMIT, &other, &mut work),
            Err(NativeValueError::Base(
                NativeExecutionError::Descriptor { .. }
            ))
        ));
        assert_eq!(work.used(), 64);
        assert_eq!(other.used_bytes(), 0);
        assert_eq!(memory.used_bytes(), bytes);
        let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
        let handle = arena.put(0, &good, &mut work).unwrap();
        assert_eq!(library.used().0, work.used());
        assert!(
            arena
                .get(handle, &mut work)
                .unwrap()
                .shares_frozen_storage(&good)
        );
        drop((arena, work, library, good));
        assert_eq!(memory.used_bytes(), 0);
    }

    #[test]
    fn generation_exhaustion_cannot_replace_or_remove_the_live_value() {
        let budget = ManagedMemoryBudget::new(1_000_000);
        let (ty, raw) = text("last generation");
        let good = admit(&raw, &budget);
        let types = [ty];
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
        let mut handle = arena.put(0, &good, &mut work).unwrap();
        arena.generations.values_mut()[0] = u64::MAX;
        handle.generation = u64::MAX;
        assert_eq!(
            arena.put(0, &good, &mut work).unwrap_err(),
            NativeValueError::GenerationExhausted
        );
        assert_eq!(
            arena.take(handle, &mut work).unwrap_err(),
            NativeValueError::GenerationExhausted
        );
        assert!(arena.get(handle, &mut work).is_ok());
    }
}
