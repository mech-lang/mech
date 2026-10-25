//! Disjoint private storage in the synchronous, owner-constructed native ABI.
//!
//! Vmctx is ONLY the frame's error-node word. Heap is the appended borrowed
//! String view lanes; Table is the appended non-map Tail loan lanes. Other
//! memory, including ordinary source values, map frames and hook arguments,
//! retains the default region. Layout derivation assigns nonoverlapping word
//! ranges, and the owning session allocates transport separately from Frame.
//! Only the program emitter uses these flags; scalar/match/iteration images
//! retain their existing memory contract and identities. Calls remain full
//! memory barriers, and each continuation reads fresh owner state.
//!
//! These three private homes are nontrapping at their existing program points.
//! Prepared owns the initialized Frame and a checked, default-initialized u64
//! transport projection for the entire synchronous entry call. Borrowed/Tail
//! layouts reserve checked disjoint subranges within that projection. Clearing
//! a loan invalidates the values it describes, never the storage of its homes.
//! Only Program layouts emit these accesses. This does not promise readonly,
//! alignment or movement across control flow, and never covers pointed-to
//! source bytes, row offsets, map storage or callback argument/result buffers.
use cranelift_codegen::ir::{AliasRegion, MemFlags};

#[cfg(test)]
thread_local! {
    static ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    static NONTRAPPING: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

fn flags(region: AliasRegion) -> MemFlags {
    #[cfg(test)]
    if !ENABLED.with(std::cell::Cell::get) {
        return MemFlags::new();
    }
    let flags = MemFlags::new().with_alias_region(Some(region));
    #[cfg(test)]
    if !NONTRAPPING.with(std::cell::Cell::get) {
        return flags;
    }
    flags.with_notrap()
}

pub(in crate::native_artifact) fn origin() -> MemFlags {
    flags(AliasRegion::Vmctx)
}
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) fn borrowed() -> MemFlags {
    flags(AliasRegion::Heap)
}
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) fn scalar_tail() -> MemFlags {
    flags(AliasRegion::Table)
}

#[cfg(test)]
pub(in crate::native_artifact) fn without_disjoint_state<T>(run: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            ENABLED.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(ENABLED.with(|v| v.replace(false)));
    run()
}

/// Differential reference that retains the existing alias regions.
#[cfg(test)]
pub(in crate::native_artifact) fn without_nontrapping_homes<T>(run: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            NONTRAPPING.with(|v| v.set(self.0));
        }
    }
    let _restore = Restore(NONTRAPPING.with(|v| v.replace(false)));
    run()
}

#[test]
#[cfg(feature = "canonical-native-program")]
fn private_home_flags_keep_mutability_and_control_flow_constraints() {
    for get in [origin, borrowed, scalar_tail] {
        let optimized = get();
        assert!(optimized.notrap());
        assert!(!optimized.readonly());
        assert!(!optimized.aligned());
        assert!(!optimized.can_move());
        let reference = without_nontrapping_homes(get);
        assert_eq!(optimized, reference.with_notrap());
        assert!(!reference.notrap());
        assert_eq!(without_disjoint_state(get), MemFlags::new());
        assert_eq!(
            get(),
            optimized,
            "reference scopes must restore the contract"
        );
    }
}
