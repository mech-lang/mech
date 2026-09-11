#[cfg(feature = "no_std")]
use alloc::{
    alloc::{AllocError, Allocator, Layout, alloc, dealloc},
    boxed::Box,
    rc::Rc,
    string::String,
};
use core::{
    cell::Cell,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
#[cfg(not(feature = "no_std"))]
use std::{
    alloc::{AllocError, Allocator, Layout, alloc, dealloc},
    boxed::Box,
    rc::Rc,
    string::String,
};

use crate::{MemoryArenaId, MemoryObjectId, MemorySpace};

use super::{MemoryRuntimeError, MemoryRuntimeResult, RealizedMemoryPlan};

/// Exact alignment-correct owned arena block.
pub(crate) struct HostBlock {
    pointer: Option<NonNull<u8>>,
    layout: Layout,
}

impl HostBlock {
    pub(crate) fn allocate(
        object: Option<MemoryObjectId>,
        bytes: u64,
        alignment: u32,
        space: MemorySpace,
    ) -> MemoryRuntimeResult<Self> {
        let size = usize::try_from(bytes).map_err(|_| MemoryRuntimeError::InvalidLayout {
            object,
            size: bytes,
            alignment,
            reason: "allocation size exceeds the host address range",
        })?;
        let alignment =
            usize::try_from(alignment).map_err(|_| MemoryRuntimeError::InvalidLayout {
                object,
                size: bytes,
                alignment,
                reason: "alignment exceeds the host address range",
            })?;
        let layout = Layout::from_size_align(size, alignment).map_err(|_| {
            MemoryRuntimeError::InvalidLayout {
                object,
                size: bytes,
                alignment: u32::try_from(alignment).unwrap_or(u32::MAX),
                reason: "size and alignment do not form a valid host layout",
            }
        })?;
        if size == 0 {
            return Ok(Self {
                pointer: None,
                layout,
            });
        }
        // SAFETY: `layout` was validated above and is retained unchanged for
        // the matching deallocation in `Drop`.
        let pointer =
            NonNull::new(unsafe { alloc(layout) }).ok_or(MemoryRuntimeError::AllocationFailed {
                object,
                requested: bytes,
                alignment: u32::try_from(alignment).unwrap_or(u32::MAX),
                space,
            })?;
        Ok(Self {
            pointer: Some(pointer),
            layout,
        })
    }

    pub(crate) const fn bytes(&self) -> usize {
        self.layout.size()
    }

    pub(crate) const fn alignment(&self) -> usize {
        self.layout.align()
    }

    pub(crate) fn pointer(&self) -> Option<NonNull<u8>> {
        self.pointer
    }
}

impl Drop for HostBlock {
    fn drop(&mut self) {
        if let Some(pointer) = self.pointer {
            // SAFETY: this pointer was returned by `alloc(self.layout)`, has
            // not been transferred, and is deallocated exactly once here.
            unsafe { dealloc(pointer.as_ptr(), self.layout) };
        }
    }
}

struct PlannedHostArenaAuthority {
    // A projected container must retain the realization whose arena bytes it
    // inhabits. The registry itself intentionally holds only a weak owner.
    _realized: RealizedMemoryPlan,
    _projection_owner: Rc<()>,
    arena: MemoryArenaId,
    pointer: NonNull<u8>,
    bytes: usize,
    alignment: usize,
    claimed: Cell<bool>,
}

enum PlannedHostArenaAllocatorKind {
    Arena(Rc<PlannedHostArenaAuthority>),
    Empty { alignment: usize },
}

/// Sealed allocator that projects one typed resident container over an
/// already-realized contiguous R5 arena. It never acquires another block and
/// therefore cannot become a second physical memory authority.
struct PlannedHostArenaAllocator {
    kind: PlannedHostArenaAllocatorKind,
}

impl core::fmt::Debug for PlannedHostArenaAllocator {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.kind {
            PlannedHostArenaAllocatorKind::Arena(authority) => formatter
                .debug_struct("PlannedHostArenaAllocator")
                .field("arena", &authority.arena)
                .field("bytes", &authority.bytes)
                .field("alignment", &authority.alignment)
                .finish(),
            PlannedHostArenaAllocatorKind::Empty { alignment } => formatter
                .debug_struct("PlannedHostArenaAllocator")
                .field("bytes", &0)
                .field("alignment", alignment)
                .finish(),
        }
    }
}

impl PlannedHostArenaAllocator {
    pub(crate) fn from_realized_parts(
        realized: RealizedMemoryPlan,
        projection_owner: Rc<()>,
        arena: MemoryArenaId,
        pointer: NonNull<u8>,
        bytes: usize,
        alignment: usize,
    ) -> MemoryRuntimeResult<Self> {
        Layout::from_size_align(bytes, alignment).map_err(|_| {
            MemoryRuntimeError::InvalidLayout {
                object: None,
                size: u64::try_from(bytes).unwrap_or(u64::MAX),
                alignment: u32::try_from(alignment).unwrap_or(u32::MAX),
                reason: "realized host arena layout is invalid",
            }
        })?;
        Ok(Self {
            kind: PlannedHostArenaAllocatorKind::Arena(Rc::new(PlannedHostArenaAuthority {
                _realized: realized,
                _projection_owner: projection_owner,
                arena,
                pointer,
                bytes,
                alignment,
                claimed: Cell::new(false),
            })),
        })
    }

    /// Constructs zero-length typed projections for planner lanes that have
    /// no physical arena member.
    fn empty(alignment: usize) -> MemoryRuntimeResult<Self> {
        if alignment == 0 || !alignment.is_power_of_two() {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: 0,
                alignment: u32::try_from(alignment).unwrap_or(u32::MAX),
                reason: "empty arena projection alignment is invalid",
            });
        }
        Ok(Self {
            kind: PlannedHostArenaAllocatorKind::Empty { alignment },
        })
    }
}

unsafe impl Allocator for PlannedHostArenaAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        match &self.kind {
            PlannedHostArenaAllocatorKind::Arena(authority) => {
                if layout.size() != authority.bytes
                    || layout.align() > authority.alignment
                    || authority.claimed.replace(true)
                {
                    return Err(AllocError);
                }
                Ok(NonNull::slice_from_raw_parts(
                    authority.pointer,
                    authority.bytes,
                ))
            }
            PlannedHostArenaAllocatorKind::Empty { alignment } => {
                if layout.size() != 0 || layout.align() > *alignment {
                    return Err(AllocError);
                }
                let pointer = NonNull::new(layout.align() as *mut u8).ok_or(AllocError)?;
                Ok(NonNull::slice_from_raw_parts(pointer, 0))
            }
        }
    }

    unsafe fn deallocate(&self, pointer: NonNull<u8>, layout: Layout) {
        match &self.kind {
            PlannedHostArenaAllocatorKind::Arena(authority) => {
                debug_assert_eq!(pointer, authority.pointer);
                debug_assert_eq!(layout.size(), authority.bytes);
                debug_assert!(layout.align() <= authority.alignment);
                // The realization owns the enclosing HostBlock. Dropping a
                // projected Box destroys its initialized elements but must
                // not free the shared arena a second time.
            }
            PlannedHostArenaAllocatorKind::Empty { .. } => {
                debug_assert_eq!(layout.size(), 0);
            }
        }
    }
}

mod planned_arena_element_sealed {
    pub trait Sealed {}
}

/// Closed set of resident lane elements that may inhabit a typed projection
/// over a planned host arena. Pointer-containing values remain real Rust
/// objects; the API never exposes their representation as arbitrary bytes.
pub trait PlannedArenaElement: planned_arena_element_sealed::Sealed + Default + 'static {
    #[doc(hidden)]
    fn supports_planned_slot(slot: crate::PlannedSlotKind) -> bool;
}

impl planned_arena_element_sealed::Sealed for u8 {}
impl PlannedArenaElement for u8 {
    fn supports_planned_slot(slot: crate::PlannedSlotKind) -> bool {
        matches!(
            slot,
            crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Bool)
        )
    }
}

impl planned_arena_element_sealed::Sealed for u64 {}
impl PlannedArenaElement for u64 {
    fn supports_planned_slot(slot: crate::PlannedSlotKind) -> bool {
        matches!(
            slot,
            crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Index)
                | crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Unsigned(
                    crate::IntegerWidth::W64
                ))
        )
    }
}

impl planned_arena_element_sealed::Sealed for f64 {}
impl PlannedArenaElement for f64 {
    fn supports_planned_slot(slot: crate::PlannedSlotKind) -> bool {
        matches!(
            slot,
            crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Floating(
                crate::FloatWidth::W64
            ))
        )
    }
}

impl planned_arena_element_sealed::Sealed for String {}
impl PlannedArenaElement for String {
    fn supports_planned_slot(slot: crate::PlannedSlotKind) -> bool {
        slot == crate::PlannedSlotKind::StringHeader
    }
}

impl planned_arena_element_sealed::Sealed for Option<crate::Value> {}
impl PlannedArenaElement for Option<crate::Value> {
    fn supports_planned_slot(slot: crate::PlannedSlotKind) -> bool {
        slot == crate::PlannedSlotKind::CanonicalValueHandle
    }
}

/// Typed owner of one complete realized host arena. Dropping the projection
/// destroys every initialized Rust element and then releases its claim; the
/// enclosing realization remains the sole owner of the raw block.
pub struct PlannedArenaProjection<T: PlannedArenaElement> {
    values: Box<[T], PlannedHostArenaAllocator>,
}

impl<T: PlannedArenaElement + core::fmt::Debug> core::fmt::Debug for PlannedArenaProjection<T> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PlannedArenaProjection")
            .field("values", &&*self.values)
            .finish()
    }
}

impl<T: PlannedArenaElement> PlannedArenaProjection<T> {
    pub(crate) fn validate_realized_layout(
        bytes: usize,
        alignment: usize,
        len: usize,
    ) -> MemoryRuntimeResult<()> {
        let expected = Layout::array::<T>(len).map_err(|_| MemoryRuntimeError::InvalidLayout {
            object: None,
            size: u64::MAX,
            alignment: core::mem::align_of::<T>() as u32,
            reason: "resident arena element count overflows",
        })?;
        if expected.size() != bytes || expected.align() > alignment {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: u64::try_from(bytes).unwrap_or(u64::MAX),
                alignment: u32::try_from(alignment).unwrap_or(u32::MAX),
                reason: "resident lane type does not exactly cover its planned arena",
            });
        }
        Ok(())
    }

    pub(crate) fn from_realized_parts(
        realized: RealizedMemoryPlan,
        projection_owner: Rc<()>,
        arena: MemoryArenaId,
        pointer: NonNull<u8>,
        bytes: usize,
        alignment: usize,
        len: usize,
    ) -> MemoryRuntimeResult<Self> {
        Self::validate_realized_layout(bytes, alignment, len)?;
        let allocator = PlannedHostArenaAllocator::from_realized_parts(
            realized,
            projection_owner,
            arena,
            pointer,
            bytes,
            alignment,
        )?;
        Self::initialize(len, allocator, None)
    }

    pub fn empty() -> MemoryRuntimeResult<Self> {
        Self::initialize(
            0,
            PlannedHostArenaAllocator::empty(core::mem::align_of::<T>())?,
            None,
        )
    }

    fn initialize(
        len: usize,
        allocator: PlannedHostArenaAllocator,
        object: Option<MemoryObjectId>,
    ) -> MemoryRuntimeResult<Self> {
        let mut values = Box::try_new_uninit_slice_in(len, allocator).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object,
                requested: u64::try_from(len)
                    .unwrap_or(u64::MAX)
                    .saturating_mul(core::mem::size_of::<T>() as u64),
                alignment: core::mem::align_of::<T>() as u32,
                space: MemorySpace::ResidentCpu,
            }
        })?;
        for value in &mut values {
            value.write(T::default());
        }
        // SAFETY: every projected lane element was initialized exactly once.
        let values = unsafe { values.assume_init() };
        Ok(Self { values })
    }
}

impl<T: PlannedArenaElement> Deref for PlannedArenaProjection<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<T: PlannedArenaElement> DerefMut for PlannedArenaProjection<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}
