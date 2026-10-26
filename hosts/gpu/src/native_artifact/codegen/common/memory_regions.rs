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

use crate::native_artifact::codegen::options::OptimizationOptions;
#[cfg(feature = "canonical-native-program")]
use crate::native_artifact::codegen::options::OptimizationPass;

fn flags(options: OptimizationOptions, region: AliasRegion) -> MemFlags {
    #[cfg(feature = "canonical-native-program")]
    if !options.enabled(OptimizationPass::DisjointState) {
        return MemFlags::new();
    }
    let flags = MemFlags::new().with_alias_region(Some(region));
    #[cfg(feature = "canonical-native-program")]
    if !options.enabled(OptimizationPass::NontrappingHomes) {
        return flags;
    }
    #[cfg(not(feature = "canonical-native-program"))]
    let _ = options;
    flags.with_notrap()
}
pub(in crate::native_artifact) fn origin(options: OptimizationOptions) -> MemFlags {
    flags(options, AliasRegion::Vmctx)
}
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) fn borrowed(options: OptimizationOptions) -> MemFlags {
    flags(options, AliasRegion::Heap)
}
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) fn scalar_tail(options: OptimizationOptions) -> MemFlags {
    flags(options, AliasRegion::Table)
}

#[test]
#[cfg(feature = "canonical-native-program")]
fn private_home_flags_keep_mutability_and_control_flow_constraints() {
    for get in [origin, borrowed, scalar_tail] {
        let optimized = get(OptimizationOptions::default());
        assert!(optimized.notrap());
        assert!(!optimized.readonly());
        assert!(!optimized.aligned());
        assert!(!optimized.can_move());
        let reference = get(OptimizationOptions::without(
            OptimizationPass::NontrappingHomes,
        ));
        assert_eq!(optimized, reference.with_notrap());
        assert!(!reference.notrap());
        assert_eq!(
            get(OptimizationOptions::without(
                OptimizationPass::DisjointState
            )),
            MemFlags::new()
        );
        assert_eq!(get(OptimizationOptions::default()), optimized);
    }
}
