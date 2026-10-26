//! Paid immutable DATA measurements, never wrapper facts or admission permits.
use super::*;
use mech_core::{
    SchemaKey,
    snapshot::{FrozenSnapshotIdentity, ValueFootprint},
};
use std::cell::Cell;

#[path = "native_snapshot_slots.rs"]
mod slots;

#[derive(Debug)]
struct Entry {
    identity: FrozenSnapshotIdentity,
    data: ValueFootprint,
}

#[derive(Debug)]
struct Slot {
    key: SchemaKey,
    schema_bytes: u64,
    ready: Option<Entry>,
    pending: Option<Entry>,
}

#[derive(Debug, Default)]
pub(super) struct DataFootprints {
    slots: RefCell<Vec<Slot>>,
    building: Cell<bool>,
    #[cfg(test)]
    observations: Cell<(u64, u64, u64)>, // live hits, cold chunks, kernel hits
}

fn overflow() -> mech_core::MemoryPlanError {
    mech_core::MemoryPlanError::TargetAddressOverflow
}

impl DataFootprints {
    pub(super) fn bytes(inputs: usize) -> Option<u64> {
        let count = u64::try_from(inputs).ok()?;
        count
            .checked_mul(core::mem::size_of::<Slot>() as u64)?
            // Ready and pending may refer to different expired data shells.
            .checked_add(
                count
                    .checked_mul(2)?
                    .checked_mul(Value::frozen_storage_identity_allocation_bytes())?,
            )
    }

    pub(super) fn nodes(inputs: usize) -> Option<u64> {
        u64::try_from(inputs)
            .ok()?
            .checked_mul(2)?
            .checked_add(u64::from(inputs != 0))
    }

    pub(super) fn new(call: &CallMemoryPlan) -> Result<Self, ResidentKernelError> {
        // The enclosing cache reservation already covers EXACTLY this many
        // Slot carriers and both weak-shell populations. Check the allocator's
        // actual capacity before any descriptor/Slot initialization.
        let slots = slots::allocate(call.inputs.len(), |ordinal| {
            let port = &call.inputs[ordinal];
            Slot {
                key: port.descriptor.schema().key(),
                schema_bytes: port.descriptor.schema().canonical_bytes().len() as u64,
                ready: None,
                pending: None,
            }
        })?;
        Ok(Self {
            slots: RefCell::new(slots),
            building: Cell::new(false),
            #[cfg(test)]
            observations: Cell::new((0, 0, 0)),
        })
    }

    fn probe_work() -> Result<u64, ResidentKernelError> {
        (core::mem::size_of::<Slot>() as u64)
            .checked_add(core::mem::size_of::<ValueFootprint>() as u64)
            .and_then(|n| n.checked_mul(8))
            .and_then(|n| n.checked_add(256))
            .ok_or(ResidentKernelError::InvalidShape)
    }

    pub(super) fn lookup(
        &self,
        value: &Value,
        key: SchemaKey,
        charge: impl FnOnce(u64) -> Result<(), ResidentKernelError>,
    ) -> Result<Option<ValueFootprint>, ResidentKernelError> {
        if self.building.get() {
            return Err(ResidentKernelError::InvalidInput);
        }
        // Numeric source access has no semantic port ordinal. Bound the full
        // fixed slot search, not the size of the immutable source payload.
        let work = Self::probe_work()?
            .checked_mul(self.slots.borrow().len() as u64)
            .ok_or(ResidentKernelError::InvalidShape)?;
        charge(work)?;
        let found = self.slots.borrow().iter().find_map(|slot| {
            (slot.key == key).then_some(())?;
            slot.ready
                .as_ref()
                .filter(|entry| entry.identity.matches(value))
                .map(|entry| entry.data)
        });
        #[cfg(test)]
        if found.is_some() {
            let (a, b, c) = self.observations.get();
            self.observations.set((a, b, c.saturating_add(1)));
        }
        Ok(found)
    }
}

pub(in crate::resident) struct DataFootprintTransaction<'a> {
    cache: &'a NativeTurnPlanCache,
    committed: bool,
}

impl NativeTurnPlanCache {
    #[cfg(test)]
    pub(crate) fn data_observations(&self) -> ((u64, u64, u64), usize, usize) {
        let slots = self.data_footprints.slots.borrow();
        (
            self.data_footprints.observations.get(),
            slots.iter().filter(|s| s.ready.is_some()).count(),
            slots.iter().filter(|s| s.pending.is_some()).count(),
        )
    }

    pub(in crate::resident) fn begin_data_measurements(
        &self,
    ) -> Result<DataFootprintTransaction<'_>, ResidentKernelError> {
        if !self.work.has_matching_active_scope() || self.data_footprints.building.get() {
            return Err(ResidentKernelError::InvalidInput);
        }
        // Both transaction sweeps and weak-token retirement are prepaid before
        // touching slot state. Weak shell capacity is already fully resident.
        let work = (core::mem::size_of::<Slot>() as u64)
            .checked_mul(self.data_footprints.slots.borrow().len() as u64)
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| n.checked_mul(8))
            .and_then(|n| n.checked_add(256))
            .ok_or(ResidentKernelError::InvalidShape)?;
        self.work.charge(work, 0)?;
        self.data_footprints.building.set(true);
        Ok(DataFootprintTransaction {
            cache: self,
            committed: false,
        })
    }
}

impl DataFootprintTransaction<'_> {
    pub(in crate::resident) fn header(
        &self,
        ordinal: usize,
        key: SchemaKey,
    ) -> Result<u64, mech_core::MemoryPlanError> {
        self.cache
            .data_footprints
            .slots
            .borrow()
            .get(ordinal)
            .filter(|slot| slot.key == key)
            .map(|slot| slot.schema_bytes)
            .ok_or(mech_core::MemoryPlanError::DescriptorMismatch)
    }

    pub(in crate::resident) fn measure(
        &self,
        ordinal: usize,
        value: &Value,
        key: SchemaKey,
        mut consume: impl FnMut(ValueFootprint, u64) -> Result<(), mech_core::MemoryPlanError>,
    ) -> Result<(), mech_core::MemoryPlanError> {
        let fail = || mech_core::MemoryPlanError::DescriptorMismatch;
        if !self.cache.work.has_matching_active_scope()
            || value.schema_key() != key
            || value
                .memory_budget_retained_bytes(&self.cache.work.memory_budget())
                .is_none()
        {
            return Err(fail());
        }
        let before = self.cache.work.used().0;
        let mut requested = DataFootprints::probe_work().map_err(|_| overflow())?;
        consume(ValueFootprint::default(), requested)?;
        let hit = {
            let slots = self.cache.data_footprints.slots.borrow();
            let slot = slots
                .get(ordinal)
                .filter(|slot| slot.key == key)
                .ok_or_else(fail)?;
            slot.ready
                .as_ref()
                .filter(|entry| entry.identity.matches(value))
                .map(|entry| entry.data)
        };
        if let Some(data) = hit {
            #[cfg(test)]
            {
                let (a, b, c) = self.cache.data_footprints.observations.get();
                self.cache
                    .data_footprints
                    .observations
                    .set((a.saturating_add(1), b, c));
            }
            // Full logical nodes/bytes remain fresh current-call demand.
            consume(data, 0)?;
            if !self.cache.work.has_matching_active_scope()
                || self
                    .cache
                    .work
                    .used()
                    .0
                    .checked_sub(before)
                    .is_none_or(|paid| paid < requested)
            {
                return Err(fail());
            }
            return Ok(());
        }
        let owner = value.schemas().ok_or_else(fail)?;
        let schema = value.validate_against(&owner).map_err(|_| fail())?;
        let mut data = ValueFootprint::default();
        mech_core::snapshot::visit_canonical_data_work(schema.body(), value.data(), |chunk| {
            let work = chunk.node_count.max(1);
            requested = requested.checked_add(work).ok_or_else(overflow)?;
            consume(
                ValueFootprint {
                    encoded_bytes: chunk.encoded_bytes,
                    retained_bytes: chunk.retained_bytes,
                    node_count: chunk.node_count,
                },
                work,
            )?;
            #[cfg(test)]
            {
                let (a, b, c) = self.cache.data_footprints.observations.get();
                self.cache
                    .data_footprints
                    .observations
                    .set((a, b.saturating_add(1), c));
            }
            data.encoded_bytes = data
                .encoded_bytes
                .checked_add(chunk.encoded_bytes)
                .ok_or_else(overflow)?;
            data.retained_bytes = data
                .retained_bytes
                .checked_add(chunk.retained_bytes)
                .ok_or_else(overflow)?;
            data.node_count = data
                .node_count
                .checked_add(chunk.node_count)
                .ok_or_else(overflow)?;
            Ok::<(), mech_core::MemoryPlanError>(())
        })
        .map_err(|error| match error {
            mech_core::snapshot::CanonicalDataWorkError::Visitor(error) => error,
            _ => fail(),
        })?;
        // A callback returning Ok outside a native scope is not payment. The
        // dual identity check and actual monotonic debit witness are required.
        if !self.cache.work.has_matching_active_scope()
            || self
                .cache
                .work
                .used()
                .0
                .checked_sub(before)
                .is_none_or(|paid| paid < requested)
        {
            return Err(fail());
        }
        let mut slots = self.cache.data_footprints.slots.borrow_mut();
        let slot = slots
            .get_mut(ordinal)
            .filter(|slot| slot.key == key)
            .ok_or_else(fail)?;
        slot.pending = Some(Entry {
            identity: value.frozen_storage_identity(),
            data,
        });
        Ok(())
    }

    pub(in crate::resident) fn commit(mut self) {
        for slot in self.cache.data_footprints.slots.borrow_mut().iter_mut() {
            if let Some(entry) = slot.pending.take() {
                slot.ready = Some(entry);
            }
        }
        self.committed = true;
    }
}

impl Drop for DataFootprintTransaction<'_> {
    fn drop(&mut self) {
        if !self.committed {
            for slot in self.cache.data_footprints.slots.borrow_mut().iter_mut() {
                slot.pending.take();
            }
        }
        self.cache.data_footprints.building.set(false);
    }
}

/// The only borrowed-kernel reuse entry. It returns measurement, not clearance.
pub(in crate::resident) fn borrowed_data_footprint(
    value: &Value,
    key: SchemaKey,
    charge: impl FnOnce(u64) -> Result<(), ResidentKernelError>,
) -> Result<Option<ValueFootprint>, ResidentKernelError> {
    let Some(cache) = ACTIVE.with(|active| active.borrow().clone()) else {
        return Ok(None);
    };
    let Some(plan) = with_active_turn_plan(|active| active.clone()) else {
        return Ok(None);
    };
    if !cache
        .base
        .borrow()
        .as_ref()
        .is_some_and(|base| Arc::ptr_eq(base, &plan))
    {
        return Ok(None);
    }
    if !cache.work.has_matching_active_scope()
        || value.schema_key() != key
        || value
            .memory_budget_retained_bytes(&cache.work.memory_budget())
            .is_none()
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    cache.data_footprints.lookup(value, key, charge)
}
