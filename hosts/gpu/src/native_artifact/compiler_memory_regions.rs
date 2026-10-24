//! Disjoint private storage in the synchronous, owner-constructed native ABI.
//!
//! Vmctx is ONLY the frame's error-node word. Heap is the appended borrowed
//! String view lanes; Table is the appended non-map Tail loan lanes. Other
//! memory, including ordinary source values, map frames and hook arguments,
//! retains the default region. Layout derivation assigns nonoverlapping word
//! ranges, and the owning session allocates transport separately from Frame.
//! Only the program emitter uses these flags; scalar/match/iteration images
//! retain their existing memory contract and identities. Calls remain full
//! memory barriers: no readonly, notrap or can_move promise
//! is added, and each continuation reads fresh owner state.
use cranelift_codegen::ir::{AliasRegion, MemFlags};

#[cfg(test)]
thread_local! {
    static ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

fn flags(region: AliasRegion) -> MemFlags {
    #[cfg(test)]
    if !ENABLED.with(std::cell::Cell::get) {
        return MemFlags::new();
    }
    MemFlags::new().with_alias_region(Some(region))
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
