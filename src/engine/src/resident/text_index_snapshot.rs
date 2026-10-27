//! One binding's admitted immutable Snapshot String index. No Values, source
//! bytes, schema owners or execution permission survive a call in this cache.
use super::{Backing, CachedIndex, IndexUse, PreparedGraphemeIndex, budget};
use crate::resident::budget::NativeLibraryWorkBudget;
use mech_core::{
    ManagedMemoryReservation, ManagedNodeReservation, ResidentKernelError, Value, ValueData,
    snapshot::FrozenSnapshotIdentity,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Debug)]
struct Entry {
    // Drop the weak shell before releasing the reservation that covers it.
    identity: FrozenSnapshotIdentity,
    index: Rc<CachedIndex>,
}
#[derive(Debug, Default)]
pub(in crate::resident) struct Slot {
    entry: RefCell<Option<Entry>>,
    bytes: Cell<u64>,
    nodes: Cell<u64>,
    building: Cell<bool>,
}
struct Pending<'a> {
    slot: &'a Slot,
    reservation: Option<ManagedMemoryReservation>,
    nodes: Option<ManagedNodeReservation>,
    complete: bool,
}
impl Drop for Pending<'_> {
    fn drop(&mut self) {
        if !self.complete {
            drop(self.reservation.take());
            drop(self.nodes.take());
            self.slot.bytes.set(0);
            self.slot.nodes.set(0);
        }
        self.slot.building.set(false);
    }
}
impl Pending<'_> {
    fn grow(
        &mut self,
        bytes: u64,
        nodes: u64,
        check: &mut impl FnMut(u64, u64) -> Result<(), ResidentKernelError>,
        memory_error: &impl Fn(mech_core::MemoryRuntimeError) -> ResidentKernelError,
    ) -> Result<(), ResidentKernelError> {
        check(bytes, nodes)?;
        self.nodes
            .as_mut()
            .unwrap()
            .resize(nodes)
            .map_err(memory_error)?;
        self.reservation
            .as_mut()
            .unwrap()
            .resize_capacity(bytes)
            .map_err(memory_error)?;
        // Only actual independently reserved capacity is visible to admission.
        // Cells avoid re-entering an Entry RefCell during kernel budget checks.
        self.slot.bytes.set(bytes);
        self.slot.nodes.set(nodes);
        Ok(())
    }
}

fn admitted_offsets(
    data: Vec<usize>,
    admitted_capacity: usize,
) -> Result<Vec<usize>, ResidentKernelError> {
    // try_reserve_exact may report more capacity than requested. Never retain
    // an allocation larger than the prepaid index/projection capacity: reject
    // it before offsets are filled or the cache entry can be published.
    if data.capacity() != admitted_capacity {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(data)
}

impl Slot {
    pub(in crate::resident) fn bytes(&self) -> u64 {
        self.bytes.get()
    }
    pub(in crate::resident) fn nodes(&self) -> u64 {
        self.nodes.get()
    }

    pub(in crate::resident) fn prepare(
        &self,
        value: &Value,
        work: &NativeLibraryWorkBudget,
        check: impl FnMut(u64, u64) -> Result<(), ResidentKernelError>,
    ) -> Result<IndexUse, ResidentKernelError> {
        self.prepare_with_charge(
            value,
            work,
            |compute, comparison| work.charge(compute, comparison),
            check,
            budget::record_native_index_memory_error,
        )
    }

    // A constructor-private non-Value projection supplies cumulative selected
    // sink accounting around the SAME work account and index mechanics. The
    // ordinary materializing path above delegates its unchanged debit here.
    pub(in crate::resident) fn prepare_with_charge(
        &self,
        value: &Value,
        work: &NativeLibraryWorkBudget,
        charge: impl Fn(u64, u64) -> Result<(), ResidentKernelError>,
        mut check: impl FnMut(u64, u64) -> Result<(), ResidentKernelError>,
        memory_error: impl Fn(mech_core::MemoryRuntimeError) -> ResidentKernelError,
    ) -> Result<IndexUse, ResidentKernelError> {
        // Fixed metadata/refcount/schema/account checks are prepaid, including
        // rejection paths. No payload hash, equality or source scan is a key.
        let lookup = (core::mem::size_of::<Slot>() as u64)
            .checked_mul(8)
            .and_then(|n| n.checked_add(256))
            .ok_or(ResidentKernelError::InvalidShape)?;
        charge(lookup, 0)?;
        if self.building.get()
            || value
                .memory_budget_retained_bytes(&work.memory_budget())
                .is_none()
        {
            return Err(ResidentKernelError::InvalidInput);
        }
        let ValueData::String(source) = value.data() else {
            return Err(ResidentKernelError::InvalidInput);
        };
        {
            let entry = self.entry.borrow();
            if let Some(entry) = entry.as_ref() {
                if entry.identity.matches(value) {
                    return Ok(IndexUse {
                        index: entry.index.clone(),
                        compute_work: 0,
                        scratch_bytes: 0,
                    });
                }
                // A nested borrower may still be using the old offsets.
                // Refuse replacement rather than under-admit coexistence.
                if Rc::strong_count(&entry.index) != 1 {
                    return Err(ResidentKernelError::InvalidInput);
                }
            }
        }
        // The prior metadata's eventual destruction was prepaid at creation.
        // Retire it before new capacity; no source payload is retained by it.
        drop(self.entry.borrow_mut().take());
        self.bytes.set(0);
        self.nodes.set(0);
        self.building.set(true);
        let mut pending = Pending {
            slot: self,
            reservation: None,
            nodes: None,
            complete: false,
        };
        pending.reservation = Some(
            work.memory_budget()
                .reserve_capacity(0)
                .map_err(&memory_error)?,
        );
        pending.nodes = Some(
            work.memory_budget()
                .reserve_nodes(0)
                .map_err(&memory_error)?,
        );
        let header = Value::frozen_storage_identity_allocation_bytes()
            .checked_add(
                (core::mem::size_of::<CachedIndex>() + 2 * core::mem::size_of::<usize>()) as u64,
            )
            .ok_or(ResidentKernelError::InvalidShape)?;
        // Covers the weak retained shell, Rc owner, capacity checks and
        // eventual metadata destruction before any allocation/identity retain.
        charge(
            header
                .checked_mul(8)
                .and_then(|n| n.checked_add(4096))
                .ok_or(ResidentKernelError::InvalidShape)?,
            0,
        )?;
        pending.grow(header, 2, &mut check, &memory_error)?;
        let index = PreparedGraphemeIndex::build_compact_with(
            source,
            |scan| {
                charge(
                    u64::try_from(scan).map_err(|_| ResidentKernelError::InvalidShape)?,
                    0,
                )
            },
            |capacity| {
                let offsets = (capacity as u64)
                    .checked_mul(core::mem::size_of::<usize>() as u64)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                charge(
                    offsets
                        .checked_mul(8)
                        .and_then(|n| n.checked_add(256))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    0,
                )?;
                pending.grow(
                    header
                        .checked_add(offsets)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                    3,
                    &mut check,
                    &memory_error,
                )?;
                let mut data = Vec::new();
                data.try_reserve_exact(capacity)
                    .map_err(|_| ResidentKernelError::InvalidShape)?;
                admitted_offsets(data, capacity)
            },
            || ResidentKernelError::InvalidShape,
        )?;
        #[cfg(test)]
        super::BUILDS.with(|builds| builds.set(builds.get() + 1));
        let index = Rc::new(CachedIndex {
            index,
            _backing: Backing {
                bytes: 0,
                nodes: 0,
                _reservation: pending.reservation.take(),
                _native_nodes: pending.nodes.take(),
            },
        });
        *self.entry.borrow_mut() = Some(Entry {
            identity: value.frozen_storage_identity(),
            index: index.clone(),
        });
        pending.complete = true;
        // Classification/offset construction was charged exactly above.
        // The two subsequent selector/publication permits do not scan source.
        Ok(IndexUse {
            index,
            compute_work: 0,
            scratch_bytes: 0,
        })
    }
}

#[cfg(test)]
#[path = "text_index_snapshot_tests.rs"]
mod tests;
