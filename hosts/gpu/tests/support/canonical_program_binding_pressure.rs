//! Optional retained bindings must leave capacity for the maintained call.
use super::artifacts;
use mech_core::{
    IntegerWidth, ManagedMemoryBudget, ResolvedOperationContract, SchemaBody, Value,
    ValueDataDraft as Data, ValueDraft, snapshot::SnapshotValidationContext,
};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError, NativeIterationAction, NativeLibraryWorkBudget,
    NativePreparedLibraryCallCache, lower_native_program_artifact,
    prepare_native_program_library_call,
};

#[test]
fn repeated_record_bindings_leave_room_for_execution_and_preserve_recovery() {
    let source = "{left: left<[u16]>, first: first<u64>, second: second<u64>, right: right<[u32]>, position: position<u64>}\n";
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let templates = plan
            .blocks()
            .iter()
            .filter_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. }
                    if template.primitive().name() == "core/composite-pack" =>
                {
                    Some(template)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(templates.len(), 1);
        let call = prepare_native_program_library_call(&artifact, templates[0], &catalog).unwrap();
        let ResolvedOperationContract::Declared(contract) = templates[0].contract() else {
            panic!()
        };
        let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(10_000_000_000, 10_000_000_000, &memory).unwrap();
        let mut policy = super::limits().library;
        policy.max_temporary_bytes = Some(4 * 1024 * 1024);
        policy.max_output_bytes = Some(1024 * 1024);
        let args = contract
            .inputs
            .iter()
            .map(|port| {
                let schema = artifact.schemas().get(port.schema).unwrap();
                let (shape_values, data) = match schema.body() {
                    SchemaBody::Matrix { element, .. } => {
                        let value = match element.as_ref() {
                            SchemaBody::UnsignedInteger(IntegerWidth::W16) => Data::U16(0xf123),
                            SchemaBody::UnsignedInteger(IntegerWidth::W32) => Data::U32(0xf0123456),
                            other => panic!("unexpected matrix {other:?}"),
                        };
                        (
                            mech_core::shape_for_resolved_extents(schema, &[1, 3])
                                .unwrap()
                                .parameter_values()
                                .to_vec()
                                .into_boxed_slice(),
                            Data::Matrix(vec![value; 3].into()),
                        )
                    }
                    SchemaBody::UnsignedInteger(IntegerWidth::W64) => {
                        (Box::<[u64]>::default(), Data::U64(17))
                    }
                    other => panic!("unexpected port {other:?}"),
                };
                let value = ValueDraft {
                    schema: port.schema,
                    shape_values,
                    data,
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                Some(value.into_memory_budget(&mut reservation).unwrap())
            })
            .collect::<Vec<Option<Value>>>();
        let aliases = args.clone();
        let mut output = None;
        call.invoke_current(&args, &mut output, policy, &work)
            .unwrap();
        let expected = output.as_ref().unwrap().canonical_data_draft().unwrap();
        let mut caches = (0..12)
            .map(|_| NativePreparedLibraryCallCache::default())
            .collect::<Vec<_>>();
        for (index, cache) in caches.iter_mut().enumerate() {
            let result = call.invoke_current_cached(cache, &args, &mut output, policy, &work);
            println!(
                "binding-pressure {form} slot={index} memory={} result={result:?}",
                memory.used_bytes()
            );
            result.unwrap();
            assert_eq!(
                output.as_ref().unwrap().canonical_data_draft().unwrap(),
                expected
            );
        }
        assert!(caches.iter().any(|cache| {
            cache
                .retained_metadata_demand(&work)
                .unwrap()
                .turn_peak_bytes
                == 0
        }));
        let fresh = args
            .iter()
            .map(|argument| {
                let value = argument.as_ref().unwrap();
                if !matches!(value.data(), mech_core::ValueData::U64(_)) {
                    return argument.clone();
                }
                let value = ValueDraft {
                    schema: value.schema(),
                    shape_values: Box::new([]),
                    data: Data::U64(29),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                Some(value.into_memory_budget(&mut reservation).unwrap())
            })
            .collect::<Vec<_>>();
        let mut fresh_expected = None;
        call.invoke_current(&fresh, &mut fresh_expected, policy, &work)
            .unwrap();
        assert_ne!(
            fresh_expected
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected
        );
        call.invoke_current_cached(
            caches.last_mut().unwrap(),
            &fresh,
            &mut output,
            policy,
            &work,
        )
        .unwrap();
        assert_eq!(
            output.as_ref().unwrap().canonical_data_draft().unwrap(),
            fresh_expected
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
        );
        let prior = output.clone();
        let before = memory.used_bytes();
        let spent = work.used().0;
        let rejected = work
            .with_call_limits(call.binding_metadata_bytes(), 10_000_000_000, || {
                call.invoke_current_cached(
                    caches.last_mut().unwrap(),
                    &args,
                    &mut output,
                    policy,
                    &work,
                )
            })
            .unwrap();
        assert!(matches!(
            rejected,
            Err(NativeCollectionInvocationError::Kernel(
                mech_core::ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                    ..
                }
            ))
        ));
        assert!(work.used().0 > spent, "a failed call preserves its debits");
        assert_eq!(memory.used_bytes(), before);
        assert!(
            output
                .as_ref()
                .unwrap()
                .shares_frozen_storage(prior.as_ref().unwrap())
        );
        let mut invalid = args.clone();
        invalid[0] = None;
        let rejected = call.invoke_current_cached(
            caches.last_mut().unwrap(),
            &invalid,
            &mut output,
            policy,
            &work,
        );
        assert!(
            matches!(rejected, Err(NativeCollectionInvocationError::InvalidInput)),
            "{rejected:?}"
        );
        assert_eq!(memory.used_bytes(), before);
        assert!(
            output
                .as_ref()
                .unwrap()
                .shares_frozen_storage(prior.as_ref().unwrap())
        );
        call.invoke_current_cached(
            caches.last_mut().unwrap(),
            &args,
            &mut output,
            policy,
            &work,
        )
        .unwrap();
        assert_eq!(
            output.as_ref().unwrap().canonical_data_draft().unwrap(),
            expected
        );
        assert!(args.iter().zip(&aliases).all(|(value, alias)| {
            value
                .as_ref()
                .unwrap()
                .shares_frozen_storage(alias.as_ref().unwrap())
        }));
        drop((
            output,
            prior,
            invalid,
            caches,
            args,
            aliases,
            fresh,
            fresh_expected,
            work,
        ));
        assert_eq!(memory.used_bytes(), 0);
    }
}
