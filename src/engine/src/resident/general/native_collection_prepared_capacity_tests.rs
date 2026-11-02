//! Failed replacement leaves a real owner-only guard, not a full binding.
use super::*;
use crate::resident::general::native_collection_calls::invocation::data_footprint_tests::{
    maximum_tests::{maximum_limits, setup},
    variants,
};

#[test]
fn owner_only_binding_guard_declines_before_starving_a_viable_call() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let limits = maximum_limits(u64::MAX);
        let (call, prepared, inputs) = setup(&artifact, &memory, limits, 2);
        assert!(
            prepared.binding_bytes >= call.invocation_metadata_bytes,
            "the prepared envelope must bound its selected invocation envelope"
        );
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output = None;
        prepared
            .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
            .unwrap();
        // This is the exact state retained after the replacement branch retires
        // its old call, then a later binding reservation/constructor fails.
        let binding = cache.binding.as_mut().unwrap();
        binding.plans = NativePlanCacheState::Cold;
        drop(binding.call.take());
        binding._nodes.resize(1).unwrap();
        binding
            ._reservation
            .resize_capacity(core::mem::size_of::<CachedBinding>() as u64)
            .unwrap();
        assert_eq!(binding._nodes.nodes(), 1);
        let prior = output.clone();
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        let available = memory.limit_bytes() - memory.used_bytes();
        let trial = |allowance| {
            let padding = memory.reserve_capacity(available - allowance).unwrap();
            let mut candidate = prior.clone();
            let result = prepared.invoke_current(&inputs, &mut candidate, limits, &work);
            if result.is_err() {
                assert!(
                    candidate
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(prior.as_ref().unwrap())
                );
            }
            let pass = result.is_ok();
            drop((candidate, padding));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            pass
        };
        let (mut low, mut high) = (0, available);
        assert!(trial(high));
        while low < high {
            let middle = low + (high - low) / 2;
            if trial(middle) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        assert!(low > 0 && trial(low) && !trial(low - 1));
        let padding = memory.reserve_capacity(available - (low - 1)).unwrap();
        prepared
            .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
            .unwrap();
        assert!(
            matches!(output.as_ref().unwrap().data(), mech_core::ValueData::String(value) if value.as_ref() == "a\0b")
        );
        assert!(
            matches!(prior.as_ref().unwrap().data(), mech_core::ValueData::String(value) if value.as_ref() == "a\0b")
        );
        drop((padding, prior, output, cache, inputs, prepared, call, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
