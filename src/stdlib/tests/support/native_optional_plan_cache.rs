//! Optional planning metadata must coexist with the actual maintained leaf.
use super::*;
use mech_core::{ValueData, ValueDataDraft};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError as Error, NativeIterationAction, NativeLibraryWorkBudget,
    NativePreparedLibraryCallCache, lower_native_program_artifact,
};

#[test]
fn optional_plan_cache_refusal_keeps_fresh_execution_authority_and_real_denials() {
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
            panic!("declared selector");
        };
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let other_site =
            prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        for finite_cap_refusal in [true, false] {
            let memory = mech_core::ManagedMemoryBudget::new(if finite_cap_refusal {
                256 * 1024 * 1024
            } else {
                8 * 1024 * 1024
            });
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let independent =
                NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let foreign = mech_core::ManagedMemoryBudget::new(8 * 1024 * 1024);
            let foreign_work =
                NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &foreign).unwrap();
            let text = admit_value(
                scalar_value(
                    &artifact,
                    contract.inputs[0].schema,
                    ValueDataDraft::String("éA".into()),
                ),
                &memory,
            );
            let args = [1, 2].map(|position| {
                [
                    Some(text.clone()),
                    Some(admit_value(
                        scalar_value(
                            &artifact,
                            contract.inputs[1].schema,
                            ValueDataDraft::U64(position),
                        ),
                        &memory,
                    )),
                ]
            });
            let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
                .unwrap()
                .limits;
            limits.max_compute_work = Some(128 * 1024 * 1024);
            limits.max_temporary_bytes = Some(if finite_cap_refusal {
                // A complete unchanged binding and ordinary invocation fit;
                // five retained planner populations cannot coexist with them.
                prepared.binding_metadata_bytes() * 2 + 65_536
            } else {
                128 * 1024 * 1024
            });
            let baseline = memory.used_bytes();
            let mut cache = NativePreparedLibraryCallCache::default();
            let mut output = None;
            for (ordinal, expected) in [(1, "A"), (0, "é"), (1, "A"), (1, "A")] {
                prepared
                    .invoke_current_cached(&mut cache, &args[ordinal], &mut output, limits, &work)
                    .unwrap();
                assert!(matches!(output.as_ref().unwrap().data(),
                    ValueData::String(text) if text.as_ref() == expected));
                drop(output.take());
                assert!(
                    memory.used_bytes() - baseline <= prepared.binding_metadata_bytes() + 4096,
                    "capacity refusal must not retain full planner populations"
                );
            }
            prepared
                .invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work)
                .unwrap();
            let prior = output.as_ref().unwrap().clone();
            let held = memory.used_bytes();
            let mut changed = limits;
            changed.max_output_elements = Some(limits.max_output_elements.unwrap() + 1);
            for (site, policy, ledger) in [
                (&other_site, limits, &work),
                (&prepared, changed, &work),
                (&prepared, limits, &independent),
                (&prepared, limits, &foreign_work),
            ] {
                assert_eq!(
                    site.invoke_current_cached(&mut cache, &args[1], &mut output, policy, ledger),
                    Err(Error::InvalidInput)
                );
                assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
                assert_eq!(memory.used_bytes(), held);
            }
            // An actual mandatory scope allocation refusal still propagates.
            let blocker = memory
                .reserve_capacity(memory.limit_bytes() - memory.used_bytes())
                .unwrap();
            assert!(matches!(
                prepared.invoke_current_cached(&mut cache, &args[0], &mut output, limits, &work),
                Err(Error::Memory(
                    mech_core::MemoryRuntimeError::BudgetExceeded { .. }
                )),
            ));
            assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
            drop(blocker);
            assert_eq!(memory.used_bytes(), held);
            drop(prior);
            prepared
                .invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work)
                .unwrap();
            drop((output, cache));
            assert_eq!(memory.used_bytes(), baseline);

            // The kernel's genuine output-byte denial remains unchanged in
            // the same sticky no-planner-cache mode and on the same ledger.
            let mut bounded = limits;
            bounded.max_output_bytes = Some(
                mech_core::snapshot::prospective_string_value_footprint(0, false, 1, 1)
                    .unwrap()
                    .retained_bytes,
            );
            let mut cache = NativePreparedLibraryCallCache::default();
            let mut output = None;
            prepared
                .invoke_current_cached(&mut cache, &args[1], &mut output, bounded, &work)
                .unwrap();
            drop(output.take());
            let retained = memory.used_bytes();
            assert!(matches!(
                prepared.invoke_current_cached(&mut cache, &args[0], &mut output, bounded, &work),
                Err(Error::Kernel(mech_core::ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::OutputBytes, required, limit, ..
                })) if required == limit + 1 && limit == bounded.max_output_bytes.unwrap(),
            ));
            assert!(output.is_none());
            assert_eq!(memory.used_bytes(), retained);
            prepared
                .invoke_current_cached(&mut cache, &args[1], &mut output, bounded, &work)
                .unwrap();
            drop((output, cache));
            assert_eq!(memory.used_bytes(), baseline);
            drop((args, text, work, independent, foreign_work));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(foreign.used_bytes(), 0);
        }
    }
}
