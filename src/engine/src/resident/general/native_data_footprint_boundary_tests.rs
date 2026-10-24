//! Finite/account/geometry boundaries on a genuinely selected source memo.
use super::*;

#[test]
fn borrowed_data_probe_has_exact_finite_boundary_and_no_failed_debit_or_permit() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let (call, inputs) = fixture(&artifact, &memory);
        let mut state = NativePlanCacheState::Cold;
        let mut output = None;
        call.invoke_cached(&mut state, &inputs, &mut output, &work)
            .unwrap();
        let cache = cached(&state);
        let plan = cache
            .with_facts(|facts| cache.matching_base(facts))
            .unwrap();
        let source = inputs[0].as_ref().unwrap();
        let mut exact = 0;
        work.with_scope(|| {
            with_cached_turn_plan(plan.clone(), cache.clone(), || {
                let data = budget::native_turn_plan_cache::borrowed_data_footprint(
                    source,
                    source.schema_key(),
                    |n| {
                        exact = n;
                        work.charge(n, 0)
                    },
                )
                .unwrap()
                .unwrap();
                assert!(data.node_count > 64);
            })
        })
        .unwrap();
        assert!(exact > 0);
        for (allowance, pass) in [(exact, true), (exact - 1, false)] {
            let before = work.used();
            let result = work
                .with_call_limits(allowance, 1_000_000, || {
                    with_cached_turn_plan(plan.clone(), cache.clone(), || {
                        budget::native_turn_plan_cache::borrowed_data_footprint(
                            source,
                            source.schema_key(),
                            |n| work.charge(n, 0),
                        )
                    })
                })
                .unwrap();
            if pass {
                assert!(result.unwrap().is_some());
                assert_eq!(work.used().0, before.0 + exact);
            } else {
                assert!(matches!(result, Err(ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::ComputeWork, required, limit, ..
                }) if required == before.0 + exact && limit == before.0 + allowance));
                assert_eq!(work.used(), before);
            }
        }
        // The same base alone cannot clear an absent/wrong active account.
        assert!(
            with_cached_turn_plan(plan.clone(), cache.clone(), || {
                budget::native_turn_plan_cache::borrowed_data_footprint(
                    source,
                    source.schema_key(),
                    |_| Ok(()),
                )
            })
            .is_err()
        );
        let foreign = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        assert!(
            foreign
                .with_scope(|| with_cached_turn_plan(plan, cache.clone(), || {
                    budget::native_turn_plan_cache::borrowed_data_footprint(
                        source,
                        source.schema_key(),
                        |_| Ok(()),
                    )
                }))
                .unwrap()
                .is_err()
        );
        drop((output, state, cache, inputs, call, work, foreign));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn prepared_memo_keeps_current_geometry_policy_account_and_owner_authority() {
    use crate::resident::general::native_collection_calls::{
        NativePreparedLibraryCallCache, prepare_native_program_library_call,
    };
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let (call, mut inputs) = fixture(&artifact, &memory);
        let prepared =
            prepare_native_program_library_call(&artifact, call.template(), &catalog()).unwrap();
        let limits = call.memory_plan().target.limits;
        let baseline_nodes = memory.retained_nodes();
        let mut state = NativePreparedLibraryCallCache::default();
        let mut output = None;
        for _ in 0..3 {
            prepared
                .invoke_current_cached(&mut state, &inputs, &mut output, limits, &work)
                .unwrap();
            assert!(
                matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "a\0b")
            );
            drop(output.take());
            assert_eq!(memory.retained_nodes() - baseline_nodes,
                state.retained_metadata_demand(&work).unwrap().retained_nodes,
                "all lazy binding, plan and index owners share the node account");
        }
        let independent = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let mut changed = limits;
        changed.max_retained_nodes = Some(1);
        for (policy, account) in [(changed, &work), (limits, &independent)] {
            assert_eq!(
                prepared.invoke_current_cached(&mut state, &inputs, &mut output, policy, account),
                Err(NativeCollectionInvocationError::InvalidInput)
            );
        }
        let foreign = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let source = inputs[0].as_ref().unwrap();
        let identity = source.frozen_storage_identity();
        let mut reservation = foreign
            .reserve_capacity(
                source.memory_budget_required_bytes().unwrap()
                    + Value::memory_budget_claim_metadata_bytes(),
            )
            .unwrap();
        let imported = source
            .clone()
            .into_independent_memory_budget(&mut reservation)
            .unwrap();
        drop(reservation);
        assert!(identity.matches(&imported));
        assert_eq!(
            prepared.invoke_current_cached(
                &mut state,
                &[Some(imported.clone()), inputs[1].clone()],
                &mut output,
                limits,
                &work
            ),
            Err(NativeCollectionInvocationError::UnadmittedValue)
        );
        // Exact-key/shape rebind to a different retained arena shares DATA,
        // but its old wrapper claim does NOT admit that new schema owner.
        let arena = artifact.schemas().clone();
        let rebound = source
            .rebind_with_context(
                source.schema(),
                source.shape(),
                &SnapshotValidationContext::new(&arena),
            )
            .unwrap();
        assert!(identity.matches(&rebound));
        assert!(rebound.memory_budget_retained_bytes(&memory).is_none());
        assert_eq!(
            prepared.invoke_current_cached(
                &mut state,
                &[Some(rebound.clone()), inputs[1].clone()],
                &mut output,
                limits,
                &work
            ),
            Err(NativeCollectionInvocationError::UnadmittedValue)
        );
        let schema = artifact.schemas().get(source.schema()).unwrap();
        let shape = mech_core::shape_for_resolved_extents(schema, &[2, 32]).unwrap();
        let reshaped = source
            .rebind_with_context(
                source.schema(),
                &shape,
                &SnapshotValidationContext::new(artifact.schemas()),
            )
            .unwrap();
        assert!(
            !identity.matches(&reshaped),
            "changed shape gets actual new immutable data identity"
        );
        inputs[0] = Some(admitted(reshaped, &memory));
        prepared
            .invoke_current_cached(&mut state, &inputs, &mut output, limits, &work)
            .unwrap();
        // LinearScalar visits column-major coordinates: selector2 now maps
        // row1,column0 -> row-major flat32, whose fixture element is emoji.
        assert!(
            matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "🧑‍🚀")
        );
        // New policy on a NEW cache must enforce the genuine current node cap.
        let mut low = NativePreparedLibraryCallCache::default();
        assert!(
            prepared
                .invoke_current_cached(&mut low, &inputs, &mut output, changed, &work)
                .is_err()
        );
        drop((
            output,
            state,
            low,
            inputs,
            call,
            prepared,
            imported,
            rebound,
            identity,
            arena,
            work,
            independent,
        ));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(foreign.used_bytes(), 0);
        assert_eq!(memory.retained_nodes(), 0);
    }
}

#[test]
fn prepared_cache_node_denial_precedes_binding_allocation_and_publication() {
    use crate::resident::general::native_collection_calls::{
        NativePreparedLibraryCallCache, prepare_native_program_library_call,
    };
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let (call, inputs) = fixture(&artifact, &memory);
        let prepared = prepare_native_program_library_call(&artifact, call.template(), &catalog()).unwrap();
        let baseline_nodes = memory.retained_nodes();
        let capped = memory.with_limits(memory.limit_bytes(), baseline_nodes);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &capped).unwrap();
        let bytes = memory.used_bytes();
        let mut state = NativePreparedLibraryCallCache::default();
        let mut output = None;
        assert!(matches!(prepared.invoke_current_cached(&mut state, &inputs, &mut output,
            call.memory_plan().target.limits, &work),
            Err(NativeCollectionInvocationError::Memory(mech_core::MemoryRuntimeError::BudgetExceeded { .. }))));
        assert!(output.is_none());
        assert_eq!(state.retained_metadata_demand(&work).unwrap().retained_nodes, 0);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (bytes, baseline_nodes));
        drop((state, output, prepared, call, inputs, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn optional_plan_cache_declines_shared_node_capacity_without_rejecting_the_call() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let (call, inputs) = fixture(&artifact, &memory);
        let baseline = memory.retained_nodes();
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let mut output = None;
        call.invoke(&inputs, &mut output, &work).unwrap();
        let output_nodes = memory.retained_nodes() - baseline;
        let cache_nodes = 1 + NativeTurnPlanCache::retained_data_nodes(call.memory_plan()).unwrap();
        assert!(output_nodes < cache_nodes);
        drop(output.take());
        let capped = memory.with_limits(memory.limit_bytes(), baseline + output_nodes);
        let limited = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &capped).unwrap();
        let mut state = NativePlanCacheState::Cold;
        call.invoke_cached(&mut state, &inputs, &mut output, &limited).unwrap();
        assert!(matches!(state, NativePlanCacheState::Uncached));
        assert!(matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "a\0b"));
        assert_eq!(memory.retained_nodes(), baseline + output_nodes);
        drop((output, state, call, inputs, work, limited));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
