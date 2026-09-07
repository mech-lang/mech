#[cfg(feature = "no_std")]
use alloc::alloc::{Layout, alloc, dealloc};
use core::ptr::NonNull;
#[cfg(not(feature = "no_std"))]
use std::alloc::{Layout, alloc, dealloc};

use crate::{MemoryObjectId, MemorySpace};

use super::{MemoryRuntimeError, MemoryRuntimeResult};

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
