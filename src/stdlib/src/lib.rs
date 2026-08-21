#![cfg_attr(all(feature = "no_std", not(feature = "std")), no_std)]

#[cfg(feature = "no_std")]
extern crate alloc;

mod catalog;

pub use catalog::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstalledLibraryVersion {
    pub name: &'static str,
    pub version: &'static str,
}

pub const INSTALLED_LIBRARIES: &[InstalledLibraryVersion] = &[
    InstalledLibraryVersion {
        name: "standard library",
        version: VERSION,
    },
    #[cfg(feature = "mech-math")]
    InstalledLibraryVersion {
        name: "math",
        version: mech_math::VERSION,
    },
    #[cfg(feature = "mech-compare")]
    InstalledLibraryVersion {
        name: "compare",
        version: mech_compare::VERSION,
    },
    #[cfg(feature = "mech-logic")]
    InstalledLibraryVersion {
        name: "logic",
        version: mech_logic::VERSION,
    },
    #[cfg(feature = "mech-range")]
    InstalledLibraryVersion {
        name: "range",
        version: mech_range::VERSION,
    },
    #[cfg(feature = "mech-matrix")]
    InstalledLibraryVersion {
        name: "matrix",
        version: mech_matrix::VERSION,
    },
    #[cfg(feature = "mech-set")]
    InstalledLibraryVersion {
        name: "set",
        version: mech_set::VERSION,
    },
    #[cfg(feature = "mech-string")]
    InstalledLibraryVersion {
        name: "string",
        version: mech_string::VERSION,
    },
    #[cfg(feature = "mech-stats")]
    InstalledLibraryVersion {
        name: "stats",
        version: mech_stats::VERSION,
    },
    #[cfg(feature = "mech-combinatorics")]
    InstalledLibraryVersion {
        name: "combinatorics",
        version: mech_combinatorics::VERSION,
    },
];
