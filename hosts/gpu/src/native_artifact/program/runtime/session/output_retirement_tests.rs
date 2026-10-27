//! Shared snapshot/capture output admission rejects before replacing any root.
use crate::native_artifact::JitProgramArtifact;
use crate::native_artifact::program::runtime::session::*;

#[test]
fn old_output_requires_its_exact_type_and_current_memory_account() {
    for artifact in crate::native_artifact::program::runtime::session::call_cache_tests::artifacts(
        crate::native_artifact::program::runtime::session::prefix_tests::source(),
    ) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let descriptor = jit.descriptor();
        assert_eq!(descriptor.metadata().plan.transport_output_count(), 1);
        let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
        let foreign = ManagedMemoryBudget::new(8 * 1024 * 1024);
        let library = NativeLibraryWorkBudget::new(1 << 30, 0, &memory).unwrap();
        let value = crate::native_artifact::program::runtime::session::prefix_tests::string_value(
            descriptor,
            descriptor.metadata().plan.transport_output_type(0).schema(),
            "prior output",
            &foreign,
        );
        let mut outputs = vec![Some(value)];
        let foreign_bytes = foreign.used_bytes();
        assert!(matches!(
            admit_output_retirement(descriptor, &outputs, &memory, &library),
            Err(NativeProgramExecutionError::Value(
                NativeValueError::UnadmittedPayload
            ))
        ));
        assert_eq!(library.used(), (0, 0));
        assert_eq!(foreign.used_bytes(), foreign_bytes);
        assert!(outputs[0].is_some());

        let ty = descriptor
            .metadata()
            .plan
            .values()
            .iter()
            .find(|ty| ty.scalar_kind() == Some(NativeScalarKind::U64))
            .unwrap();
        let wrong = ValueDraft {
            schema: ty.schema(),
            shape_values: Box::new([]),
            data: ValueDataDraft::U64(7),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &descriptor.metadata().schemas,
        ))
        .unwrap();
        let mut claim = memory
            .reserve_capacity(wrong.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        outputs[0] = Some(wrong.into_memory_budget(&mut claim).unwrap());
        drop(claim);
        assert_eq!(foreign.used_bytes(), 0);
        assert!(matches!(
            admit_output_retirement(descriptor, &outputs, &memory, &library),
            Err(NativeProgramExecutionError::Value(
                NativeValueError::TypeMismatch
            ))
        ));
        assert_eq!(library.used(), (0, 0));
        assert!(matches!(
            outputs[0].as_ref().unwrap().data(),
            ValueData::U64(7)
        ));
        drop((outputs, library));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn old_output_retirement_is_cumulative_exact_and_one_short_without_publication() {
    for artifact in crate::native_artifact::program::runtime::session::call_cache_tests::artifacts(
        crate::native_artifact::program::runtime::session::prefix_tests::source(),
    ) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let descriptor = jit.descriptor();
        assert_eq!(descriptor.metadata().plan.transport_output_count(), 1);
        let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
        let value = crate::native_artifact::program::runtime::session::prefix_tests::string_value(
            descriptor,
            descriptor.metadata().plan.transport_output_type(0).schema(),
            "retained Unicode output: 猫",
            &memory,
        );
        let debit = value.memory_budget_retained_bytes(&memory).unwrap() * 4 + 32;
        let prior = value
            .canonical_snapshot_bytes(&descriptor.metadata().schemas)
            .unwrap();
        let outputs = vec![Some(value)];
        let used = 17;
        for limit in [used + debit - 1, used + debit] {
            let library = NativeLibraryWorkBudget::new(limit, 0, &memory).unwrap();
            library.charge(used, 0).unwrap();
            let result = admit_output_retirement(descriptor, &outputs, &memory, &library);
            if limit == used + debit {
                result.unwrap();
                assert_eq!(library.used(), (used + debit, 0));
            } else {
                assert!(matches!(result, Err(NativeProgramExecutionError::Base(
                    NativeExecutionError::WorkLimit {
                        phase: NativeExecutionPhase::Setup, required, limit: actual,
                    }
                )) if required == used + debit && actual == limit));
                assert_eq!(library.used(), (used, 0));
            }
            assert_eq!(
                outputs[0]
                    .as_ref()
                    .unwrap()
                    .canonical_snapshot_bytes(&descriptor.metadata().schemas)
                    .unwrap(),
                prior
            );
            drop(library);
        }
        drop(outputs);
        assert_eq!(memory.used_bytes(), 0);
    }
}
