//! Disjoint private storage in the synchronous, owner-constructed native ABI.
//!
//! Vmctx covers the frame's error-node word and read-only numeric loan cells.
//! Heap covers appended borrowed String view lanes and mutable numeric cells;
//! Table covers appended non-map Tail loan lanes and the private scalar bank.
//! Other memory, including ordinary values, map frames and hook arguments,
//! retains the default region. Layout derivation assigns nonoverlapping word
//! ranges, and the owning session allocates transport separately from Frame.
//! Only the program emitter uses these flags; scalar/match/iteration images
//! retain their existing memory contract and identities. Calls remain full
//! memory barriers, and each continuation reads fresh owner state.
//!
//! Private transport and origin homes are nontrapping at their existing points.
//! Prepared owns the initialized Frame and a checked, default-initialized u64
//! transport projection for the entire synchronous entry call. Borrowed/Tail
//! layouts reserve checked disjoint subranges within that projection. Clearing
//! a loan invalidates the values it describes, never the storage of its homes.
//! Only Program layouts emit these accesses. This does not promise readonly,
//! alignment or movement across control flow. These nontrapping flags never
//! cover source cells, row offsets, maps or callback argument/result buffers.
//!
//! Numeric input owners survive the current closed-tail loan. If two physical
//! roots share backing, a changed output store first obtains an exclusive COW
//! owner through the existing first-change callback. Equal stores do not write.
//! A proof-local array with any write/append is classified mutable for ALL of
//! its accesses. Thus no numeric store overlaps a read-only numeric cell in
//! the same callback-free epoch. All mutable arrays share one conservative
//! class, and all read-only arrays may alias each other. Calls remain barriers.
//! Cell flags never assert readonly, nontrapping, alignment or free movement.
//!
//! BufferFrames owns the separate initialized scalar bank for the invocation;
//! scalar_bank_pointer checks its exact derived extent and it never resizes.
//! Every bank access uses a checked physical scalar binding from that layout.
//! Bank clearing ends value validity, not the lifetime of the admitted storage.
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

#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) fn numeric_cells(
    options: OptimizationOptions,
    writable: bool,
) -> MemFlags {
    if !options.enabled(OptimizationPass::NumericArrayAliases)
        || !options.enabled(OptimizationPass::DisjointState)
    {
        return MemFlags::new();
    }
    MemFlags::new().with_alias_region(Some(if writable {
        AliasRegion::Heap
    } else {
        AliasRegion::Vmctx
    }))
}

#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) fn numeric_bank(options: OptimizationOptions) -> MemFlags {
    if !options.enabled(OptimizationPass::NumericArrayAliases) {
        return MemFlags::new();
    }
    flags(options, AliasRegion::Table)
}

#[test]
#[cfg(feature = "canonical-native-program")]
fn numeric_array_alias_flags_preserve_source_traps_and_private_bank_lifetimes() {
    for writable in [false, true] {
        let cells = numeric_cells(OptimizationOptions::default(), writable);
        assert_eq!(
            cells.alias_region(),
            Some(if writable {
                AliasRegion::Heap
            } else {
                AliasRegion::Vmctx
            })
        );
        assert!(!cells.notrap());
        assert!(!cells.readonly());
        assert!(!cells.aligned());
        assert!(!cells.can_move());
        for pass in [
            OptimizationPass::NumericArrayAliases,
            OptimizationPass::DisjointState,
        ] {
            assert_eq!(
                numeric_cells(OptimizationOptions::without(pass), writable),
                MemFlags::new()
            );
        }
        assert_eq!(
            numeric_cells(
                OptimizationOptions::without(OptimizationPass::NontrappingHomes),
                writable
            ),
            cells
        );
    }
    let bank = numeric_bank(OptimizationOptions::default());
    assert_eq!(bank.alias_region(), Some(AliasRegion::Table));
    assert!(bank.notrap());
    assert!(!bank.readonly());
    assert!(!bank.aligned());
    assert!(!bank.can_move());
    let trapping = numeric_bank(OptimizationOptions::without(
        OptimizationPass::NontrappingHomes,
    ));
    assert_eq!(trapping.with_notrap(), bank);
    for pass in [
        OptimizationPass::NumericArrayAliases,
        OptimizationPass::DisjointState,
    ] {
        assert_eq!(
            numeric_bank(OptimizationOptions::without(pass)),
            MemFlags::new()
        );
    }
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
