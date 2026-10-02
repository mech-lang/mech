//! Canonical retained-document syntax.
#![cfg_attr(all(feature = "no_std", not(feature = "std")), no_std)]

extern crate alloc;

// The existing audited allocator is confined to std-enabled library tests.
#[cfg(all(test, any(feature = "std", not(feature = "no_std"))))]
#[path = "../../core/tests/support/r6_allocation_probe.rs"]
mod allocation_probe;

#[cfg(all(test, any(feature = "std", not(feature = "no_std"))))]
#[global_allocator]
static TEST_ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

pub mod document;
