from pathlib import Path
p=Path('hosts/gpu/src/control/mod.rs');s=p.read_text();s+='''
// Private same-object ABI. Storage remains owned by a core planned projection.
#[repr(C)]
struct RefArenaContext {
    base: *mut u64,
    capacity: u64,
    cursor: u64,
    arity: u64,
}
''';p.write_text(s)
p=Path('hosts/gpu/src/control/runtime.rs');s=p.read_text();a=s.index('// The registry validates');b=s.index('/// Format unsigned scalar fields',a)
s=s[:a]+'''/// Run an admitted scalar callback in an exclusively projected core memory arena.
/// The realization is the sole physical storage authority. No per-record host
/// allocation, ownership registry, or benchmark-specific allocator is involved.
/// # Safety
/// The same-object emitter supplies a matching synchronous callback, initialized
/// scalar arguments and exact exclusive output storage. Its scoped-reference
/// type rules prohibit references in the callback arguments or return values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_control_ref_arena(
    capacity: u64,
    arity: u64,
    function: u64,
    arguments: *const u64,
    argument_count: u64,
    output: *mut u64,
) -> u64 {
    use mech_core::{AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement,
        ArenaPlan, IntegerWidth, MemoryArenaId, MemoryBudgetLimits, MemoryDomain,
        MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemorySpace, PlannedSlotKind,
        PortDirection, ResourceDemand, RuntimePlanView, ScalarMemoryKind};
    if capacity == 0 || !(1..=32).contains(&arity) || function == 0 || argument_count > 32 {
        return 9;
    }
    let Some(bytes) = capacity.checked_mul(arity).and_then(|n| n.checked_mul(8))
        .filter(|&n| n <= isize::MAX as u64) else { return 9; };
    let Ok(domain) = MemoryDomain::new() else { return 9; };
    let Ok(revision) = domain.issue_plan_revision() else { return 9; };
    let arena = MemoryArenaId::new(0);
    let object = MemoryObjectId::new(0);
    let allocations = [AllocationPlan {
        id: object,
        owner: MemoryObjectOwner::DirectCallPort { call: 0, direction: PortDirection::Output, port: 0 },
        role: AllocationRole::FixedStorage,
        slot: Some(PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64))),
        space: MemorySpace::Host,
        current_bytes: bytes,
        capacity_bytes: bytes,
        payload_block_capacity: 0,
        alignment: 8,
        lifetime: MemoryLifetime::Activation,
        placement: ArenaPlacement { arena, offset: 0 },
        reuse_group: None,
    }];
    let arenas = [ArenaPlan { id: arena, space: MemorySpace::Host,
        backing: ArenaBackingKind::ContiguousBytes, alignment: 8, capacity_bytes: bytes,
        members: vec![object].into_boxed_slice() }];
    let Ok(reservation) = domain.prepare_realization(RuntimePlanView::for_call(
        revision, &allocations, &arenas,
        ResourceDemand { activation_bytes: bytes, ..ResourceDemand::default() },
        0, MemoryBudgetLimits::default(), &[], 1, &[])) else { return 9; };
    let Ok(realized) = domain.materialize(reservation) else { return 9; };
    let Ok(mut storage) = domain.project_host_arena::<u64>(&realized, arena, (bytes / 8) as usize)
        else { return 9; };
    let mut context = super::RefArenaContext { base: storage.as_mut_ptr(), capacity, cursor: 0, arity };
    type Callback = unsafe extern "C" fn(*mut super::RefArenaContext, *const u64, *mut u64) -> u64;
    // SAFETY: the private callback is emitted in the same object with this exact
    // ABI. Context and its exclusive typed projection stay live synchronously;
    // callback admission prevents references escaping this realization.
    unsafe { std::mem::transmute::<usize, Callback>(function as usize)(&mut context, arguments, output) }
}

'''+s[b:]
a=s.index('    // Any unsuccessful native call can leave');b=s.index('    if streaming {',a);s=s[:a]+s[b:]
a=s.index('            let (mark, live) = HEAP.with');b=s.index('            if status == 0 { Ok(result) }',a)
s=s[:a]+'''            let status = unsafe { callback(begin, last, args.as_ptr(), result.as_mut_ptr()) };
'''+s[b:];p.write_text(s)
