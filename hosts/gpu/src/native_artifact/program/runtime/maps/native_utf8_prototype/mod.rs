//! Isolated storage/emission prototype. No canonical source region selects it.
//! The eventual caller must also prove original producer/call admission and
//! owner export. Here real canonical handles, ownership and finite work are
//! exercised without assuming hashbrown's representation or a source name.
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
use crate::native_artifact::{
    NativeExecutionPhase, NativeValueArena, NativeValueHandle, NativeWorkMeter,
};
use mech_core::snapshot::SequenceView;
use mech_core::{
    IntegerWidth, ManagedMemoryReservation, ManagedNodeReservation, SchemaBody, Value, ValueData,
};
use mech_engine::native_artifact::NativeLibraryWorkBudget;
use std::mem::{MaybeUninit, offset_of, size_of};
use std::sync::atomic::{AtomicU64, Ordering};

mod emit;
mod tests;

const OFFSET: u64 = 0xcbf29ce484222325;
const PRIME: u64 = 0x100000001b3;
const OK: u64 = 0;
const COLD: u64 = 1;
static EPOCHS: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Cell {
    address: u64,
    bytes: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Slot {
    address: u64,
    bytes: u64,
    value: u64,
    hash: u64,
    occupied: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct ResultCell {
    value: u64,
    found: u64,
}
#[repr(C)]
struct Frame {
    slots: u64,
    capacity: u64,
    cells: u64,
    count: u64,
    rows: u64,
    columns: u64,
    output: u64,
    stored_bytes: u64,
    fuel: u64,
    comparison: u64,
    epoch: u64,
    issued: u64,
    next: u64,
}

// An exact admitted layout, not Vec's potentially larger requested capacity.
fn fixed_slice<T: Copy>(len: usize, value: T) -> Option<Box<[T]>> {
    if len == 0 {
        return Some(Box::default());
    }
    let layout = std::alloc::Layout::array::<MaybeUninit<T>>(len).ok()?;
    // SAFETY: caller owns the exact admitted layout. MaybeUninit makes the
    // fresh allocation legal before all entries are initialized below.
    let pointer = unsafe { std::alloc::alloc(layout) }.cast::<MaybeUninit<T>>();
    let pointer = std::ptr::NonNull::new(pointer)?;
    let mut cells =
        unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(pointer.as_ptr(), len)) };
    for cell in cells.iter_mut() {
        cell.write(value);
    }
    // SAFETY: every element was written; the allocation and length are kept.
    Some(unsafe { cells.assume_init() })
}
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}
fn price(capacity: u64, stored: u64, key: u64) -> Option<(u64, u64)> {
    let bytes = capacity
        .checked_mul(4)?
        .checked_add(32)?
        .checked_add(stored)?
        .checked_add(key)?;
    Some((bytes.checked_mul(64)?, bytes))
}

struct Loan {
    // These pins outlive every pointer in both vectors. No element owns a
    // fabricated Rust reference or a pointer into hashbrown's private table.
    map: Value,
    keys: Value,
    map_handle: NativeValueHandle,
    keys_handle: NativeValueHandle,
    slots: Box<[Slot]>,
    cells: Box<[Cell]>,
    output: Box<[ResultCell]>,
    frame: Box<Frame>,
    work: NativeLibraryWorkBudget,
    _nodes: ManagedNodeReservation,
    _reservation: ManagedMemoryReservation,
}
impl Loan {
    fn new(
        arena: &NativeValueArena,
        map_slot: u32,
        key_slot: u32,
        work: &mut NativeWorkMeter,
        memory: &mech_core::ManagedMemoryBudget,
    ) -> Result<Option<Self>, Error> {
        let map = arena.copy_current(map_slot, work)?;
        let keys = arena.copy_current(key_slot, work)?;
        let map_handle = arena.current_handle(map_slot, work)?;
        let keys_handle = arena.current_handle(key_slot, work)?;
        let Some(schemas) = map.schemas() else {
            return Ok(None);
        };
        let Some(map_schema) = schemas.get(map.schema()) else {
            return Ok(None);
        };
        let SchemaBody::Map { key, value, .. } = map_schema.body() else {
            return Ok(None);
        };
        if **key != SchemaBody::String || **value != SchemaBody::UnsignedInteger(IntegerWidth::W64)
        {
            return Ok(None);
        }
        let (ValueData::Map(entries), ValueData::Matrix(matrix)) = (map.data(), keys.data()) else {
            return Ok(None);
        };
        let SequenceView::String(strings) = matrix.elements() else {
            return Ok(None);
        };
        let Some(key_schemas) = keys.schemas() else {
            return Ok(None);
        };
        let Some(schema) = key_schemas.get(keys.schema()) else {
            return Ok(None);
        };
        let SchemaBody::Matrix {
            dimensions,
            element,
        } = schema.body()
        else {
            return Ok(None);
        };
        if **element != SchemaBody::String {
            return Ok(None);
        }
        let [rows, columns] = dimensions.as_ref() else {
            return Ok(None);
        };
        let rows = keys
            .shape()
            .resolve_dimension(rows)
            .map_err(|_| invalid("prototype key rows"))?;
        let columns = keys
            .shape()
            .resolve_dimension(columns)
            .map_err(|_| invalid("prototype key columns"))?;
        if rows.checked_mul(columns) != Some(strings.len() as u64) {
            return Ok(None);
        }
        let Some(capacity) = entries
            .entries()
            .len()
            .checked_mul(2)
            .and_then(|n| n.max(4).checked_next_power_of_two())
        else {
            return Ok(None);
        };
        let bytes = (capacity as u64)
            .checked_mul(size_of::<Slot>() as u64)
            .and_then(|n| {
                n.checked_add(
                    (strings.len() as u64)
                        .checked_mul((size_of::<Cell>() + size_of::<ResultCell>()) as u64)?,
                )
            })
            .and_then(|n| n.checked_add((size_of::<Self>() + size_of::<Frame>()) as u64))
            .ok_or_else(|| invalid("prototype storage bound overflow"))?;
        let Some(map_retained) = map.memory_budget_retained_bytes(memory) else {
            return Ok(None);
        };
        let Some(key_retained) = keys.memory_budget_retained_bytes(memory) else {
            return Ok(None);
        };
        let Some(retirement) = map_retained
            .checked_add(key_retained)
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(128))
        else {
            return Ok(None);
        };
        let setup = (entries.entries().len() as u64)
            .checked_mul(capacity as u64)
            .and_then(|n| n.checked_mul(32))
            .and_then(|n| n.checked_add(map_retained.checked_mul(64)?))
            .and_then(|n| n.checked_add(bytes.checked_mul(8)?))
            .and_then(|n| n.checked_add(256))
            .and_then(|n| n.checked_add(retirement))
            .ok_or_else(|| invalid("prototype setup work overflow"))?;
        let library = work.shared_library_budget(NativeExecutionPhase::Setup, u64::MAX, memory)?;
        if library
            .unscoped_remaining()
            .is_none_or(|remaining| remaining.0 < setup)
        {
            return Ok(None);
        }
        work.charge(NativeExecutionPhase::Setup, setup)?;
        let Ok(reservation) = memory.reserve_capacity(bytes) else {
            return Ok(None);
        };
        let Ok(nodes) = memory.reserve_nodes(4) else {
            return Ok(None);
        };
        // The complete source-key/hash inventory is traversed only after
        // the admitted source footprint and worst-case probing were prepaid.
        let stored_bytes = entries
            .entries()
            .iter()
            .try_fold(0u64, |n, entry| {
                let (ValueData::String(key), ValueData::U64(_)) =
                    (entry.key().data(), entry.value())
                else {
                    return None;
                };
                n.checked_add(key.len() as u64)
            })
            .ok_or_else(|| invalid("prototype map key/value family"))?;
        let Some(mut slots) = fixed_slice(capacity, Slot::default()) else {
            return Ok(None);
        };
        let Some(mut cells) = fixed_slice(strings.len(), Cell::default()) else {
            return Ok(None);
        };
        let Some(mut output) = fixed_slice(strings.len(), ResultCell::default()) else {
            return Ok(None);
        };
        for entry in entries.entries() {
            let (ValueData::String(key), ValueData::U64(value)) =
                (entry.key().data(), entry.value())
            else {
                return Ok(None);
            };
            let code = hash(key.as_bytes());
            let mut at = code as usize & (capacity - 1);
            while slots[at].occupied != 0 {
                at = (at + 1) & (capacity - 1)
            }
            slots[at] = Slot {
                address: key.as_ptr() as u64,
                bytes: key.len() as u64,
                value: *value,
                hash: code,
                occupied: 1,
            };
        }
        for (cell, key) in cells.iter_mut().zip(strings) {
            *cell = Cell {
                address: key.as_ptr() as u64,
                bytes: key.len() as u64,
            };
        }
        let epoch = loop {
            // Every contended attempt is paid; exhaustion never wraps a loan
            // generation or turns it into permission for an older table.
            work.charge(NativeExecutionPhase::Setup, 128)?;
            let old = EPOCHS.load(Ordering::Relaxed);
            let next = old
                .checked_add(1)
                .ok_or_else(|| invalid("prototype epoch exhausted"))?;
            if EPOCHS
                .compare_exchange(old, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break next;
            }
        };
        let frame = Box::new(Frame {
            slots: slots.as_ptr() as u64,
            capacity: capacity as u64,
            cells: cells.as_ptr() as u64,
            count: cells.len() as u64,
            rows,
            columns,
            output: output.as_mut_ptr() as u64,
            stored_bytes,
            fuel: 0,
            comparison: 0,
            epoch,
            issued: 0,
            next: 0,
        });
        Ok(Some(Self {
            map,
            keys,
            map_handle,
            keys_handle,
            slots,
            cells,
            output,
            frame,
            work: library,
            _nodes: nodes,
            _reservation: reservation,
        }))
    }
    fn run(
        &mut self,
        arena: &NativeValueArena,
        work: &mut NativeWorkMeter,
        run: impl FnOnce(*mut Frame) -> u64,
    ) -> Result<u64, Error> {
        if !arena
            .get(self.map_handle, work)?
            .shares_frozen_storage(&self.map)
            || !arena
                .get(self.keys_handle, work)?
                .shares_frozen_storage(&self.keys)
        {
            return Err(invalid("prototype changed source"));
        }
        let library = work.shared_library_budget(
            NativeExecutionPhase::Setup,
            self.work.limits().1,
            &self.work.memory_budget(),
        )?;
        if !library.shares_account(&self.work) {
            return Err(invalid("prototype foreign work account"));
        }
        let used = self.work.used();
        let remaining = self
            .work
            .unscoped_remaining()
            .ok_or_else(|| invalid("prototype nested scope"))?;
        self.frame.fuel = remaining.0;
        self.frame.comparison = remaining.1;
        self.frame.issued = self.frame.epoch;
        let result = run(&mut *self.frame);
        if self.work.used() != used || self.work.unscoped_remaining() != Some(remaining) {
            return Err(invalid("prototype account changed during native loan"));
        }
        let compute = remaining
            .0
            .checked_sub(self.frame.fuel)
            .ok_or_else(|| invalid("prototype renewed fuel"))?;
        let comparison = remaining
            .1
            .checked_sub(self.frame.comparison)
            .ok_or_else(|| invalid("prototype renewed comparison fuel"))?;
        self.frame.issued = 0;
        self.work
            .charge(compute, comparison)
            .map_err(|_| invalid("prototype finite reconciliation failed"))?;
        Ok(result)
    }
}
