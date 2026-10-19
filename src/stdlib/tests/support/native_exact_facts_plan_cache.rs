//! Exact numeric planning facts do not cache arguments or results.
use super::*;
use mech_core::{ValueData, ValueDataDraft};
use mech_engine::native_artifact::{
    NativeIterationAction, NativeLibraryWorkBudget, NativePreparedLibraryCallCache,
    lower_native_program_artifact,
};

#[test]
fn same_facts_reuse_metadata_but_execute_current_unicode_and_changed_values() {
    for artifact in artifacts("+> sequence\nsequence/length(input<string>)\n") {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. } => Some(template),
                _ => None,
            })
            .unwrap();
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let schema = input_schema(&artifact, "sequence/length", 0);
        let args = ["é", "ab", "XY", "longer", "", "é"].map(|text| {
            [Some(admit_value(
                scalar_value(&artifact, schema, ValueDataDraft::String(text.into())),
                &memory,
            ))]
        });
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let baseline = memory.used_bytes();
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output = None;
        let mut costs = Vec::new();
        let mut retained = None;
        for (input, expected) in args.iter().zip([1, 2, 2, 6, 0, 1]) {
            let start = work.used().0;
            prepared
                .invoke_current_cached(&mut cache, input, &mut output, limits, &work)
                .unwrap();
            costs.push(work.used().0 - start);
            assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(n) if *n == expected));
            // All invocations have absent prior output, including the warm
            // equal-byte/different-grapheme input and both size misses.
            drop(output.take());
            if let Some(before) = retained {
                assert_eq!(memory.used_bytes(), before);
            }
            retained = Some(memory.used_bytes());
        }
        assert!(costs[0] > costs[1] + prepared.binding_work() / 2);
        assert!(
            costs[3] > costs[2] * 2,
            "real facts miss must reconstruct plans: {costs:?}"
        );
        let blocker = memory
            .reserve_capacity(memory.limit_bytes() - memory.used_bytes())
            .unwrap();
        assert!(
            prepared
                .invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work)
                .is_err()
        );
        assert!(output.is_none());
        drop(blocker);
        prepared
            .invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(2)));
        drop(output.take());
        drop(cache);
        assert_eq!(memory.used_bytes(), baseline);
        drop((args, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn same_base_facts_vary_candidate_utf8_bytes_reuse_current_permits_and_deny_atomically() {
    for artifact in artifacts("text := input<string>\ntext[position<u64>]\n") {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. }
                    if template.primitive() == NativeCollectionPrimitive::StringScalarAccess =>
                {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!("selected declared String access")
        };
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let text = admit_value(
            scalar_value(
                &artifact,
                contract.inputs[0].schema,
                ValueDataDraft::String("éA".into()),
            ),
            &memory,
        );
        let args = [1, 2].map(|index| {
            [
                Some(text.clone()),
                Some(admit_value(
                    scalar_value(
                        &artifact,
                        contract.inputs[1].schema,
                        ValueDataDraft::U64(index),
                    ),
                    &memory,
                )),
            ]
        });
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let baseline = memory.used_bytes();
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output = None;
        let mut costs = Vec::new();
        for (ordinal, expected) in [(0, "é"), (1, "A"), (1, "A"), (1, "A")] {
            let start = work.used().0;
            prepared
                .invoke_current_cached(&mut cache, &args[ordinal], &mut output, limits, &work)
                .unwrap();
            costs.push(work.used().0 - start);
            assert!(
                matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == expected)
            );
            drop(output.take());
        }
        assert!(
            costs[1] > costs[2] * 2,
            "current exact demand phases must really hit: {costs:?}"
        );
        assert_eq!(costs[2], costs[3]);
        drop(cache);
        assert_eq!(memory.used_bytes(), baseline);
        // Same finite cap and ledger throughout: a fresh one-byte result fits,
        // the two-byte grapheme does not. Keep the old publication absent:
        // an existing Snapshot also has a separately planned physical slot
        // and retained payload, which is not the candidate-only boundary.
        let mut bounded = limits;
        bounded.max_output_bytes = Some(
            mech_core::snapshot::prospective_string_value_footprint(0, false, 1, 1)
                .unwrap()
                .retained_bytes,
        );
        let mut cache = NativePreparedLibraryCallCache::default();
        for _ in 0..3 {
            prepared
                .invoke_current_cached(&mut cache, &args[1], &mut output, bounded, &work)
                .unwrap();
            assert!(
                matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "A")
            );
            drop(output.take());
        }
        let retained = memory.used_bytes();
        assert!(matches!(
            prepared.invoke_current_cached(&mut cache, &args[0], &mut output, bounded, &work),
            Err(mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                mech_core::ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::OutputBytes,
                    required, limit, ..
                }
            )) if required == limit + 1 && limit == bounded.max_output_bytes.unwrap()
        ));
        assert!(output.is_none());
        assert_eq!(memory.used_bytes(), retained);
        prepared
            .invoke_current_cached(&mut cache, &args[1], &mut output, bounded, &work)
            .unwrap();
        assert!(
            matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "A")
        );
        drop((output, cache));
        assert_eq!(memory.used_bytes(), baseline);

        // Independently preserve a real prior publication under a failed
        // allocation. Use unchanged full limits, including prior coexistence.
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output = None;
        prepared
            .invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work)
            .unwrap();
        let prior = output.as_ref().unwrap().clone();
        let retained = memory.used_bytes();
        let blocker = memory
            .reserve_capacity(memory.limit_bytes() - retained)
            .unwrap();
        assert!(
            prepared
                .invoke_current_cached(&mut cache, &args[0], &mut output, limits, &work)
                .is_err()
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        drop(blocker);
        assert_eq!(memory.used_bytes(), retained);
        drop(prior);
        prepared
            .invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work)
            .unwrap();
        assert!(
            matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "A")
        );
        drop((output, cache));
        assert_eq!(memory.used_bytes(), baseline);
        drop((args, text, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}
