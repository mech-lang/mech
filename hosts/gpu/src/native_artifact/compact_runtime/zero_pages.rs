use std::{alloc::Layout, ptr::NonNull};

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::{ffi::c_void, sync::OnceLock};

    unsafe extern "C" {
        fn getpagesize() -> i32;
        fn mmap(
            address: *mut c_void,
            bytes: usize,
            protection: i32,
            flags: i32,
            descriptor: i32,
            offset: i64,
        ) -> *mut c_void;
        fn munmap(address: *mut c_void, bytes: usize) -> i32;
    }

    #[cfg(test)]
    thread_local! {
        pub(crate) static DENY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    pub fn reserve(layout: Layout) -> Result<Option<NonNull<u8>>, u64> {
        static PAGE_BYTES: OnceLock<usize> = OnceLock::new();
        let page_bytes = *PAGE_BYTES.get_or_init(|| {
            // SAFETY: the system query has no pointer arguments.
            usize::try_from(unsafe { getpagesize() }).unwrap()
        });
        if layout.size() < page_bytes * 4 || layout.align() > page_bytes {
            return Ok(None);
        }
        #[cfg(test)]
        if DENY.with(|deny| deny.get()) {
            return Err(super::super::ALLOCATION);
        }
        // The Darwin SDK defines PROT_READ=1, PROT_WRITE=2, MAP_PRIVATE=2,
        // MAP_ANON=0x1000. A fresh anonymous private mapping starts zero-filled.
        let pointer = unsafe { mmap(std::ptr::null_mut(), layout.size(), 3, 0x1002, -1, 0) };
        if pointer == usize::MAX as *mut c_void {
            return Err(super::super::ALLOCATION);
        }
        if let Some(pointer) = NonNull::new(pointer.cast()) {
            Ok(Some(pointer))
        } else {
            // A zero address cannot be represented as a Rust allocation.
            // Release that mapping before reporting allocation refusal.
            unsafe { munmap(pointer, layout.size()) };
            Err(super::super::ALLOCATION)
        }
    }

    pub unsafe fn release(pointer: *mut u8, bytes: usize) -> Result<(), u64> {
        // SAFETY: the caller owns this exact live mmap result and its byte size.
        if unsafe { munmap(pointer.cast(), bytes) } == 0 {
            Ok(())
        } else {
            Err(super::super::INVALID)
        }
    }
}

#[cfg(target_os = "macos")]
pub(super) use platform::{release, reserve};

#[cfg(all(test, target_os = "macos"))]
pub(super) use platform::DENY;

#[cfg(not(target_os = "macos"))]
pub fn reserve(_: Layout) -> Result<Option<NonNull<u8>>, u64> {
    Ok(None)
}

#[cfg(not(target_os = "macos"))]
pub unsafe fn release(_: *mut u8, _: usize) -> Result<(), u64> {
    Err(super::INVALID)
}
