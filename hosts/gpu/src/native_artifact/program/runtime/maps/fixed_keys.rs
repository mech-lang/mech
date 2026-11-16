//! Current fixed-key lane for generated ordinal selection and U64 replacement.
//! These records do not execute a source loop or authorize an invocation.
use super::admission::{Error, add, mul};
use super::string::PublicationGeometry as Geometry;
use mech_core::{ManagedMemoryReservation, ManagedNodeReservation, MemoryBudgetLimits};
use mech_engine::native_artifact::{
    NativeBorrowedMapKeys, NativeLibraryWorkBudget, NativeStringMatrixKeyCell,
    NativeStringMatrixKeyView,
};
use std::mem::MaybeUninit;

#[cfg(test)]
thread_local! {
    static OBSERVATIONS: std::cell::Cell<[u64;4]> = const { std::cell::Cell::new([0;4]) };
}
#[cfg(test)]
pub(in crate::native_artifact::program::runtime) fn record(index: usize, count: u64) {
    OBSERVATIONS.with(|v| {
        let mut n = v.get();
        n[index] += count;
        v.set(n);
    });
}
#[cfg(test)]
pub(in crate::native_artifact::program::runtime) fn observations(reset: bool) -> [u64; 4] {
    OBSERVATIONS.with(|v| if reset { v.replace([0; 4]) } else { v.get() })
}

#[repr(C)]
pub(in crate::native_artifact) struct Frame {
    pub cells: u64,
    pub count: u64,
    pub values: u64,
    pub token: u64,
    pub updates: u64,
    pub epoch: u64,
    pub used_bytes: u64,
    pub snapshot_nodes: u64,
    pub fixed_nodes: u64,
    pub temporary_limit: u64,
    pub node_limit: u64,
    pub map_nodes: u64,
}
#[repr(C)]
pub(in crate::native_artifact::program::runtime) struct Lane {
    pub frame: Frame,
    pub view: NativeBorrowedMapKeys,
    pub lane: usize,
    pub compute: u64,
    pub comparison: u64,
    pub insert: usize,
    pub native: NativeStringMatrixKeyView,
    pub value_address: u64,
    pub token: u64,
    pub counters: mech_core::NativeMemoryBudgetCounters,
    pub fixed_nodes: u64,
    pub _metadata: ManagedMemoryReservation,
    pub _nodes: ManagedNodeReservation,
}
#[derive(Clone, Copy, Debug)]
pub(in crate::native_artifact::program::runtime) struct Quote {
    pub compute: u64,
    pub comparison: u64,
}

/// # Safety
/// `view.cells` must identify exactly `view.count` readable immutable records
/// throughout this call. This returns descriptive costs only. The owning
/// layer must separately bind a genuine current loan, source and account.
pub(in crate::native_artifact::program::runtime) unsafe fn quote(
    view: NativeStringMatrixKeyView,
    geometry: Geometry,
    limits: MemoryBudgetLimits,
    work: &NativeLibraryWorkBudget,
) -> Result<Option<Quote>, Error> {
    if view.epoch == 0
        || view.rows != 1
        || view.columns != view.count
        || view.count > isize::MAX as u64 / size_of::<NativeStringMatrixKeyCell>() as u64
        || view.cells == 0
        || view.cells % align_of::<NativeStringMatrixKeyCell>() as u64 != 0
    {
        return Ok(None);
    }
    let (
        Some(out_elements),
        Some(out_bytes),
        Some(cloned),
        Some(temporary),
        Some(nodes),
        Some(compute),
        Some(comparison),
    ) = (
        limits.max_output_elements,
        limits.max_output_bytes,
        limits.max_cloned_bytes,
        limits.max_temporary_bytes,
        limits.max_retained_nodes,
        limits.max_compute_work,
        limits.max_comparison_work,
    )
    else {
        return Ok(None);
    };
    work.charge(add(mul(view.count, 512)?, 1024)?, 0)?;
    if out_elements < 1 || geometry.live_bytes > temporary || geometry.live_nodes > nodes {
        return Ok(None);
    }
    // SAFETY: the caller pins this exact checked key-view allocation. No
    // callback or mutation occurs while these immutable records are read.
    let cells = unsafe {
        std::slice::from_raw_parts(
            view.cells as *const NativeStringMatrixKeyCell,
            view.count as usize,
        )
    };
    let mut max = Quote {
        compute: view.selector_compute_work,
        comparison: view.selector_comparison_work,
    };
    for cell in cells {
        let c = add(view.selector_compute_work, cell.selection_compute_work)?;
        let p = add(
            view.selector_comparison_work,
            cell.selection_comparison_work,
        )?;
        if c > compute
            || p > comparison
            || cell.logical_output_bytes > out_bytes
            || cell.logical_cloned_bytes > cloned
            || add(geometry.live_bytes, cell.logical_temporary_bytes)? > temporary
            || add(geometry.live_nodes, cell.logical_extra_nodes)? > nodes
        {
            return Ok(None);
        }
        max.compute = max.compute.max(c);
        max.comparison = max.comparison.max(p);
    }
    Ok(Some(max))
}

/// # Safety
/// The caller has prepaid and reserved the exact Layout<T> and one private
/// node. Initialize before assuming T; drop this Box on every refusal path.
pub(in crate::native_artifact::program::runtime) unsafe fn allocate_one<T>()
-> Result<Box<MaybeUninit<T>>, Error> {
    let layout = std::alloc::Layout::new::<MaybeUninit<T>>();
    if layout.size() == 0 {
        return Err(Error::Binding("zero-sized native fixed-key owner"));
    }
    // SAFETY: caller holds the admission claim for this nonzero exact layout.
    let pointer = unsafe { std::alloc::alloc(layout) }.cast::<MaybeUninit<T>>();
    let pointer = std::ptr::NonNull::new(pointer).ok_or(Error::Allocation)?;
    // SAFETY: fresh Global allocation of exactly one uninitialized T.
    Ok(unsafe { Box::from_raw(pointer.as_ptr()) })
}
