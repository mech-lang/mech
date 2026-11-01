//! Actual selected original/decoded invocation failure and same-account recovery.
use super::*;

#[test]
fn oversized_memo_slots_fail_before_initialization_without_publication_or_refund() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let (call, inputs) = fixture(&artifact, &memory);
        // Obtain an actual old publication through the SAME maintained leaf.
        let mut state = NativePlanCacheState::Uncached;
        let mut output = None;
        call.invoke_cached(&mut state, &inputs, &mut output, &work)
            .unwrap();
        let prior = output.as_ref().unwrap().clone();
        assert!(matches!(prior.data(), ValueData::String(s) if s.as_ref() == "a\0b"));
        state = NativePlanCacheState::Cold;
        let baseline = memory.used_bytes();
        let retained =
            NativeTurnPlanCache::retained_bytes(call.memory_plan(), call.invocation_metadata_bytes)
                .unwrap();
        let guard_bytes = (core::mem::size_of::<NativePlanCacheState>()
            + core::mem::size_of::<ResourceDemand>()
            + core::mem::size_of::<mech_core::MemoryBudgetLimits>()
            + core::mem::size_of::<mech_core::ManagedMemoryReservation>()
            + 3 * core::mem::size_of::<u64>()) as u64;
        let owner = NativeTurnPlanCache::construction_owner_bytes(call.memory_plan()).unwrap();
        let construction_work =
            guard_bytes * 8 + 256 + call.invocation_metadata_bytes + owner * 8 + 256;
        let before = work.used();
        let (result, observed) =
            NativeTurnPlanCache::with_data_slot_allocation_extra(1, &memory, || {
                call.invoke_cached(&mut state, &inputs, &mut output, &work)
            });
        assert_eq!(
            result,
            Err(NativeCollectionInvocationError::Kernel(
                ResidentKernelError::InvalidShape,
            ))
        );
        assert_eq!(
            observed.0, 1,
            "one actual construction attempt, no quota retry"
        );
        assert_eq!(
            observed.1, 0,
            "no Slot descriptor or weak shell initialized"
        );
        assert_eq!(observed.2, 1, "oversized allocation was dropped");
        assert!(
            observed.3 >= baseline + retained,
            "caller reservation remains live during rejection"
        );
        assert!(
            matches!(state, NativePlanCacheState::Cold),
            "no partial cache or uncached fallback"
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert_eq!(memory.used_bytes(), baseline);
        let charged = work.used();
        assert_eq!(
            charged,
            (before.0 + construction_work, before.1),
            "both actual prepaid construction/retirement debits survive failure"
        );

        // A new external invocation can recover on the SAME unchanged account.
        let (recovery, observed) =
            NativeTurnPlanCache::with_data_slot_allocation_extra(0, &memory, || {
                call.invoke_cached(&mut state, &inputs, &mut output, &work)
            });
        recovery.unwrap();
        assert_eq!(observed.0, 1);
        assert_eq!(observed.1, call.memory_plan().inputs.len());
        assert_eq!(observed.2, 0);
        let cache = cached(&state);
        assert!(cache.validate(call.memory_plan().target.limits, &work));
        assert_eq!(cache.data_observations().1, 2);
        assert_eq!(cache.data_observations().2, 0);
        assert!(cache.data_observations().0.1 > 64);
        assert!(
            cache.data_observations().0.2 > 0,
            "same selected numeric helper consumes paid memo"
        );
        assert!(
            matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "a\0b")
        );
        let cold = cache.data_observations().0.1;
        let (warm, observed) =
            NativeTurnPlanCache::with_data_slot_allocation_extra(1, &memory, || {
                call.invoke_cached(&mut state, &inputs, &mut output, &work)
            });
        warm.unwrap();
        assert_eq!(
            observed,
            (0, 0, 0, 0),
            "warm hit never reallocates Slot backing"
        );
        assert_eq!(cache.data_observations().0.1, cold);
        let paid = work.used();
        drop((prior, output, state, cache, inputs, call));
        assert_eq!(
            work.used(),
            paid,
            "retirement never refunds the SAME account"
        );
        drop(work);
        assert_eq!(memory.used_bytes(), 0);
    }
}
