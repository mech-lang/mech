//! Immutable index metadata belongs to one exact admitted native binding.
use super::*;
use mech_core::{ValueData, ValueDataDraft};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError as Error, NativeIterationAction, NativeLibraryWorkBudget,
    NativePreparedLibraryCallCache, lower_native_program_artifact,
};

#[test]
fn snapshot_string_index_warm_work_and_retained_size_do_not_scale_with_unit_source_bytes() {
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
            panic!("declared String selector")
        };
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let position = admit_value(
            scalar_value(&artifact, contract.inputs[1].schema, ValueDataDraft::U64(2)),
            &memory,
        );
        let args = ["ACGT".to_owned(), "ACGT".repeat(16_384)].map(|text| {
            [
                Some(admit_value(
                    scalar_value(
                        &artifact,
                        contract.inputs[0].schema,
                        ValueDataDraft::String(text.into()),
                    ),
                    &memory,
                )),
                Some(position.clone()),
            ]
        });
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let baseline = memory.used_bytes();
        let mut cold = Vec::new();
        let mut warm = Vec::new();
        let mut retained = Vec::new();
        for input in &args {
            let mut cache = NativePreparedLibraryCallCache::default();
            let mut output = None;
            let mut costs = Vec::new();
            for _ in 0..4 {
                let before = work.used().0;
                prepared
                    .invoke_current_cached(&mut cache, input, &mut output, limits, &work)
                    .unwrap();
                costs.push(work.used().0 - before);
                assert!(matches!(output.as_ref().unwrap().data(),
                    ValueData::String(text) if text.as_ref() == "C"));
                drop(output.take());
            }
            cold.push(costs[0]);
            assert_eq!(
                costs[2], costs[3],
                "all current facts and index metadata are warm"
            );
            warm.push(costs[3]);
            retained.push(memory.used_bytes() - baseline);
            drop(cache);
            assert_eq!(memory.used_bytes(), baseline);
        }
        assert!(
            cold[1] >= cold[0] + 65_532,
            "initial canonical classification must really be charged"
        );
        assert_eq!(warm[0], warm[1], "a hit must not scan source bytes again");
        assert_eq!(
            retained[0], retained[1],
            "proven unit boundaries must not allocate a byte-sized offsets table"
        );
        drop((args, position, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn snapshot_string_index_preserves_raw_graphemes_and_exact_binding_authority() {
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
            panic!("declared String selector")
        };
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let other_site =
            prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let independent = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let foreign = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let foreign_work =
            NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &foreign).unwrap();
        let source = admit_value(
            scalar_value(
                &artifact,
                contract.inputs[0].schema,
                ValueDataDraft::String("e\u{301}🧑‍🚀\r\n🇺🇸\0é".into()),
            ),
            &memory,
        );
        let args = [1, 2, 3, 4, 5, 6].map(|position| {
            [
                Some(source.clone()),
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
        let wrong = [args[0][1].clone(), args[0][1].clone()];
        let foreign_args = [
            Some(admit_value(
                scalar_value(
                    &artifact,
                    contract.inputs[0].schema,
                    ValueDataDraft::String("e\u{301}🧑‍🚀\r\n🇺🇸\0é".into()),
                ),
                &foreign,
            )),
            args[0][1].clone(),
        ];
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let baseline = memory.used_bytes();
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output = None;
        for (input, expected) in args.iter().zip(["e\u{301}", "🧑‍🚀", "\r\n", "🇺🇸", "\0", "é"])
        {
            prepared
                .invoke_current_cached(&mut cache, input, &mut output, limits, &work)
                .unwrap();
            assert!(matches!(output.as_ref().unwrap().data(),
                ValueData::String(text) if text.as_ref() == expected));
            drop(output.take());
        }
        prepared
            .invoke_current_cached(&mut cache, &args[0], &mut output, limits, &work)
            .unwrap();
        let prior = output.as_ref().unwrap().clone();
        let held = memory.used_bytes();
        let mut changed = limits;
        changed.max_output_elements = Some(limits.max_output_elements.unwrap() + 1);
        for (owner, policy, ledger) in [
            (&other_site, limits, &work),
            (&prepared, changed, &work),
            (&prepared, limits, &independent),
            (&prepared, limits, &foreign_work),
        ] {
            assert_eq!(
                owner.invoke_current_cached(&mut cache, &args[0], &mut output, policy, ledger),
                Err(Error::InvalidInput)
            );
            assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
            assert_eq!(memory.used_bytes(), held);
        }
        assert_eq!(
            prepared.invoke_current_cached(&mut cache, &wrong, &mut output, limits, &work),
            Err(Error::InvalidInput)
        );
        assert_eq!(
            prepared.invoke_current_cached(&mut cache, &foreign_args, &mut output, limits, &work),
            Err(Error::UnadmittedValue)
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert_eq!(memory.used_bytes(), held);
        drop(prior);
        prepared
            .invoke_current_cached(&mut cache, &args[2], &mut output, limits, &work)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(),
            ValueData::String(text) if text.as_ref() == "\r\n"));
        drop((output, cache));
        assert_eq!(memory.used_bytes(), baseline);
        drop((
            args,
            wrong,
            source,
            work,
            independent,
            foreign_args,
            foreign_work,
        ));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(foreign.used_bytes(), 0);
    }
}

#[test]
fn snapshot_string_index_offsets_obey_same_finite_temporary_cap_and_recover() {
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
            panic!("declared String selector")
        };
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let position = admit_value(
            scalar_value(&artifact, contract.inputs[1].schema, ValueDataDraft::U64(1)),
            &memory,
        );
        // Same UTF-8 byte count and scalar shape, but one source requires a
        // real 500,001-entry offsets table, not proven unit boundaries.
        // Uncached dense offsets exceed the fixed native-cache envelope, so
        // the first successful cold cap can genuinely select the weak index.
        let args = ["A".repeat(1_000_000), "é".repeat(500_000)].map(|text| {
            [
                Some(admit_value(
                    scalar_value(
                        &artifact,
                        contract.inputs[0].schema,
                        ValueDataDraft::String(text.into()),
                    ),
                    &memory,
                )),
                Some(position.clone()),
            ]
        });
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
            .unwrap()
            .limits;
        limits.max_compute_work = Some(128 * 1024 * 1024);
        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
        let baseline = memory.used_bytes();
        let mut output = None;
        let mut initial = NativePreparedLibraryCallCache::default();
        prepared
            .invoke_current_cached(&mut initial, &args[0], &mut output, limits, &work)
            .unwrap();
        drop(initial);
        let publication = memory.used_bytes();
        let mut finite = limits;
        // Search only within the existing finite policy. Jumping to a dense
        // uncached denial's required size could skip the smaller cached
        // frontier, so bracket by success rather than inflate that size.
        let mut denied_cap = 0;
        let mut admitted_cap = limits.max_temporary_bytes.unwrap();
        for _ in 0..32 {
            if admitted_cap - denied_cap == 1 {
                break;
            }
            let trial = denied_cap + (admitted_cap - denied_cap) / 2;
            finite.max_temporary_bytes = Some(trial);
            let mut candidate = NativePreparedLibraryCallCache::default();
            let prior = output.as_ref().unwrap().clone();
            match prepared.invoke_current_cached(
                &mut candidate,
                &args[0],
                &mut output,
                finite,
                &work,
            ) {
                Ok(_) => {
                    assert!(matches!(output.as_ref().unwrap().data(),
                        ValueData::String(text) if text.as_ref() == "A"));
                    admitted_cap = trial;
                }
                Err(Error::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation }))
                | Err(Error::Binding(NativeArtifactLoweringError::ResidentBinding {
                    error:
                        mech_engine::resident::ResidentActivationError::ResidentMemoryPlanRejected {
                            error: mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                        },
                })) if violation.dimension == mech_core::MemoryBudgetDimension::TemporaryBytes => {
                    assert!(violation.required > violation.limit);
                    assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
                    denied_cap = trial;
                }
                Err(Error::Kernel(mech_core::ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::TemporaryBytes,
                    required,
                    limit,
                    ..
                })) => {
                    assert!(required > limit);
                    assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
                    denied_cap = trial;
                }
                other => panic!("unexpected temporary-boundary result: {other:?}"),
            }
            drop((candidate, prior));
            assert_eq!(memory.used_bytes(), publication);
        }
        assert_eq!(admitted_cap, denied_cap + 1, "finite search converges");
        finite.max_temporary_bytes = Some(admitted_cap);
        let mut cache = NativePreparedLibraryCallCache::default();
        prepared
            .invoke_current_cached(&mut cache, &args[0], &mut output, finite, &work)
            .unwrap();
        assert!(
            memory.used_bytes() - publication > prepared.binding_metadata_bytes() + 4096,
            "the actual native planner/index cache must be retained, not its uncached alternative"
        );
        let prior = output.as_ref().unwrap().clone();
        let held = memory.used_bytes();
        let unicode =
            prepared.invoke_current_cached(&mut cache, &args[1], &mut output, finite, &work);
        assert!(
            matches!(&unicode,
            Err(Error::Kernel(mech_core::ResidentKernelError::ResourceLimit {
                dimension: mech_core::MemoryBudgetDimension::TemporaryBytes, required, limit, ..
            })) if required.saturating_sub(*limit) > 1 && *limit <= finite.max_temporary_bytes.unwrap()),
            "offset growth must exceed the selected Unicode result's single extra byte: {unicode:?}"
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert!(
            memory.used_bytes() <= held,
            "failed replacement cannot retain both index backings"
        );
        drop(prior);
        prepared
            .invoke_current_cached(&mut cache, &args[0], &mut output, finite, &work)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(),
            ValueData::String(text) if text.as_ref() == "A"));
        drop((output, cache));
        assert_eq!(memory.used_bytes(), baseline);
        drop((args, position, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn snapshot_string_index_global_reservation_denial_is_typed_atomic_and_recovers() {
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
            panic!("declared String selector")
        };
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let position = admit_value(
            scalar_value(&artifact, contract.inputs[1].schema, ValueDataDraft::U64(1)),
            &memory,
        );
        let args = ["A".repeat(1_000_000), "é".repeat(500_000)].map(|text| {
            [
                Some(admit_value(
                    scalar_value(
                        &artifact,
                        contract.inputs[0].schema,
                        ValueDataDraft::String(text.into()),
                    ),
                    &memory,
                )),
                Some(position.clone()),
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
        for _ in 0..3 {
            prepared
                .invoke_current_cached(&mut cache, &args[0], &mut output, limits, &work)
                .unwrap();
        }
        // Leave enough real capacity for the same warm call's current plan,
        // but not for the Unicode source's separately owned four-MiB offsets.
        // The immediately repeated successful call proves normal scopes fit.
        let allowance = prepared.binding_metadata_bytes() + 2_000_000 + 65_536;
        let blocker = memory
            .reserve_capacity(
                memory
                    .limit_bytes()
                    .checked_sub(memory.used_bytes())
                    .unwrap()
                    .checked_sub(allowance)
                    .unwrap(),
            )
            .unwrap();
        prepared
            .invoke_current_cached(&mut cache, &args[0], &mut output, limits, &work)
            .unwrap();
        let prior = output.as_ref().unwrap().clone();
        let held = memory.used_bytes();
        assert!(
            matches!(
                prepared.invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work),
                Err(Error::Memory(_))
            ),
            "global index growth refusal must use the real payload admission error channel"
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert!(memory.used_bytes() <= held);
        drop((prior, blocker));
        prepared
            .invoke_current_cached(&mut cache, &args[1], &mut output, limits, &work)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(),
            ValueData::String(text) if text.as_ref() == "é"));
        prepared
            .invoke_current_cached(&mut cache, &args[0], &mut output, limits, &work)
            .unwrap();
        assert!(matches!(output.as_ref().unwrap().data(),
            ValueData::String(text) if text.as_ref() == "A"));
        drop((output, cache));
        assert_eq!(memory.used_bytes(), baseline);
        drop((args, position, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}
