//! Private append-only capture pages with same-account capacity ownership.
//! Published UTF-8 slices never move or mutate; Rc prevents cross-thread use.
use super::{Account, Claim, InvokeError};
use std::{cell::Cell, ptr::NonNull, rc::Rc};

const CAPACITY: usize = 2 * 1024 * 1024;
const MIN_FRAGMENT: usize = 16 * 1024;
const HEADER: usize = std::mem::size_of::<Slab>() + 2 * std::mem::size_of::<usize>() + 128;

// Each attempt consumes a distinct complete fragment of at least MIN_FRAGMENT
// input bytes, even if mapping fails or earlier slabs are later retired. This
// intentionally loose bound includes all preferred claims in addition to the
// ordinary heap bound; it does not depend on successful slab packing.
pub(in super::super) fn allocation_work_bound(input_limit: u64) -> Option<u64> {
    if input_limit < CAPACITY as u64 {
        return Some(0);
    }
    let debit = ((CAPACITY + HEADER) as u64)
        .checked_mul(8)?
        .checked_add(128)?;
    (input_limit / MIN_FRAGMENT as u64).checked_mul(debit)
}

struct Mapping {
    pointer: NonNull<u8>,
    bytes: usize,
}
impl Mapping {
    fn new() -> Option<Self> {
        #[cfg(test)]
        if FAIL_NEXT.with(|flag| flag.replace(false)) {
            return None;
        }
        // SAFETY: sysconf reads a process-wide numeric page-size property.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page <= 0 || CAPACITY % usize::try_from(page).ok()? != 0 {
            return None;
        }
        // SAFETY: anonymous private mapping, no file/foreign owner or borrowed
        // address. CAPACITY is nonzero; Mapping exclusively retires this extent.
        let pointer = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                CAPACITY,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if pointer == libc::MAP_FAILED {
            return None;
        }
        let Some(pointer) = NonNull::new(pointer.cast::<u8>()) else {
            // A valid mapping at address zero cannot back Rust references.
            // SAFETY: this is the just-created owned mapping, still unexposed.
            if unsafe { libc::munmap(pointer, CAPACITY) } != 0 {
                std::process::abort();
            }
            return None;
        };
        Some(Self {
            pointer,
            bytes: CAPACITY,
        })
    }
}

#[cfg(test)]
thread_local! { static FAIL_NEXT: Cell<bool> = const { Cell::new(false) }; }
#[cfg(test)]
#[path = "native_text_line_page_tests.rs"]
mod page_storage_tests;
impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: the mapping extent is immutable and owned by the final Rc.
        // No Text or borrowed str can survive its owning Slab.
        if unsafe { libc::munmap(self.pointer.as_ptr().cast(), self.bytes) } != 0 {
            // Never release the capacity claim while its backing remains live.
            std::process::abort();
        }
    }
}

struct Slab {
    // Physical pages retire before the same-account capacity claim.
    mapping: Mapping,
    used: Cell<usize>,
    claim: Claim,
}
impl Slab {
    fn append(self: &Rc<Self>, text: &str) -> Option<Text> {
        let start = self.used.get();
        let end = start.checked_add(text.len())?;
        if end > self.mapping.bytes {
            return None;
        }
        // SAFETY: start..end is inside this owned mapping and strictly after
        // every published slice. Rc and Cell keep this callback-free operation
        // on one thread. The source is valid UTF-8; no published bytes change.
        unsafe {
            std::ptr::copy_nonoverlapping(
                text.as_ptr(),
                self.mapping.pointer.as_ptr().add(start),
                text.len(),
            );
        }
        self.used.set(end);
        Some(Text {
            slab: self.clone(),
            start,
            bytes: text.len(),
        })
    }
}

pub(in super::super) struct Text {
    slab: Rc<Slab>,
    start: usize,
    bytes: usize,
}
impl Text {
    pub fn as_str(&self) -> &str {
        // SAFETY: only Slab::append constructs Text, from a complete str copied
        // into a disjoint initialized range. No mutable view is exposed. The
        // borrow is bounded by this Rc owner; pooling resets only with Rc==1.
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                self.slab.mapping.pointer.as_ptr().add(self.start),
                self.bytes,
            ))
        }
    }
}

#[derive(Default)]
pub(in super::super) struct Pool {
    active: Option<Rc<Slab>>,
    disabled: bool,
}
impl Pool {
    pub fn clear(&mut self) {
        self.active = None;
    }
    pub fn copy(
        &mut self,
        text: &str,
        input_limit: u64,
        account: &Rc<Account>,
    ) -> Result<Option<Text>, InvokeError> {
        if self.disabled
            || input_limit < CAPACITY as u64
            || text.len() < MIN_FRAGMENT
            || text.len() > CAPACITY
        {
            return Ok(None);
        }
        if let Some(active) = &self.active {
            if !Rc::ptr_eq(&active.claim.account, account) {
                return Err(InvokeError::InvalidInput);
            }
            // No Text owner (and thus no borrowed str) can exist in this case.
            if Rc::strong_count(active) == 1 {
                active.used.set(0);
            }
            if let Some(text) = active.append(text) {
                return Ok(Some(text));
            }
        }
        let Some(claim) = Account::reserve_preferred(account, (CAPACITY + HEADER) as u64)? else {
            self.disabled = true;
            self.clear();
            return Ok(None);
        };
        let Some(mapping) = Mapping::new() else {
            // One failed physical preference remains charged; never retry it
            // for each source fragment or refund cumulative work.
            self.disabled = true;
            self.clear();
            drop(claim);
            return Ok(None);
        };
        let slab = Rc::new(Slab {
            mapping,
            used: Cell::new(0),
            claim,
        });
        let text = slab.append(text).ok_or(InvokeError::InvalidInput)?;
        self.active = Some(slab);
        Ok(Some(text))
    }
}
