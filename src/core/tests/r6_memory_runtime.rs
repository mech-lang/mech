#![cfg(feature = "full")]

use mech_core::{
    AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement, ArenaPlan, CallAccessRequest,
    MemoryAccessMode, MemoryAccessRegion, MemoryArenaId, MemoryBudgetLimits, MemoryDomain,
    MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemoryPlanPoint, MemoryRuntimeError,
    MemorySpace, PublicationCandidate, ResourceDemand, ReuseGroupId, RuntimeBinding,
    RuntimePlanView,
};

fn allocation(
    id: u32,
    arena: u32,
    offset: u64,
    current: u64,
    capacity: u64,
    lifetime: MemoryLifetime,
    reuse_group: Option<u32>,
) -> AllocationPlan {
    AllocationPlan {
        id: MemoryObjectId::new(id),
        owner: MemoryObjectOwner::DirectCallPort {
            call: 0,
            direction: mech_core::PortDirection::Output,
            port: id as u16,
        },
        role: AllocationRole::FixedStorage,
        space: MemorySpace::Host,
        current_bytes: current,
        capacity_bytes: capacity,
        alignment: 8,
        lifetime,
        placement: ArenaPlacement {
            arena: MemoryArenaId::new(arena),
            offset,
        },
        reuse_group: reuse_group.map(ReuseGroupId::new),
    }
}

fn arena(id: u32, backing: ArenaBackingKind, capacity: u64, members: &[u32]) -> ArenaPlan {
    ArenaPlan {
        id: MemoryArenaId::new(id),
        space: MemorySpace::Host,
        backing,
        alignment: 8,
        capacity_bytes: capacity,
        members: members
            .iter()
            .copied()
            .map(MemoryObjectId::new)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }
}

#[test]
fn reservations_are_finite_and_release_unused_authority() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 8, 64, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 64, &[0])];
    {
        let reservation = domain
            .prepare_realization(RuntimePlanView::new(
                revision,
                &allocations,
                &arenas,
                ResourceDemand::default(),
                MemoryBudgetLimits::default(),
                &[],
            ))
            .unwrap();
        assert_eq!(reservation.reserved_bytes(), 64);
        assert_eq!(domain.ledger().reserved_bytes, 64);
        assert_eq!(domain.ledger().active_reservations, 1);
    }
    assert_eq!(domain.ledger().reserved_bytes, 0);
    assert_eq!(domain.ledger().active_reservations, 0);
}

#[test]
fn realization_tracks_initialization_and_scoped_access() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 8, 64, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 64, &[0])];
    let reservation = domain
        .prepare_realization(RuntimePlanView::new(
            revision,
            &allocations,
            &arenas,
            ResourceDemand::default(),
            MemoryBudgetLimits::default(),
            &[],
        ))
        .unwrap();
    let realized = domain.materialize(reservation).unwrap();
    let key = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let binding = realized.binding(key).unwrap();
    assert_eq!(binding.required_initialization_bytes(), 8);
    assert_eq!(binding.initialized_bytes(), 0);
    let unreadable = domain.prepare_call(
        &realized,
        &[CallAccessRequest {
            object: key,
            mode: MemoryAccessMode::Read,
            region: MemoryAccessRegion::Contiguous {
                offset_bytes: 0,
                length_bytes: 8,
            },
        }],
    );
    assert!(matches!(
        unreadable,
        Err(MemoryRuntimeError::UninitializedAccess { .. })
    ));

    let write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: key,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 8,
                },
            }],
        )
        .unwrap();
    domain
        .acquire_call(&realized, &write)
        .unwrap()
        .with_bytes_mut(key, |bytes| bytes.copy_from_slice(&42_u64.to_ne_bytes()))
        .unwrap();
    assert_eq!(realized.binding(key).unwrap().initialized_bytes(), 8);

    let read = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: key,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    let frame = domain.acquire_call(&realized, &read).unwrap();
    let value = frame
        .with_bytes(key, |bytes| u64::from_ne_bytes(bytes.try_into().unwrap()))
        .unwrap();
    assert_eq!(value, 42);

    let observations = domain.allocation_observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].capacity_bytes, 64);
    assert_eq!(observations[0].actual_block_bytes, 64);
    assert!(observations[0].actual_block_alignment >= 8);
}

#[test]
fn lease_acquisition_is_atomic_and_region_aware() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None),
        allocation(1, 0, 8, 0, 8, MemoryLifetime::Activation, None),
    ];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 16, &[0, 1])];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let left = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let right = domain
        .plan_object_key(revision, MemoryObjectId::new(1))
        .unwrap();
    for key in [left, right] {
        let write = domain
            .prepare_call(
                &realized,
                &[CallAccessRequest {
                    object: key,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: 8,
                    },
                }],
            )
            .unwrap();
        domain
            .acquire_call(&realized, &write)
            .unwrap()
            .with_bytes_mut(key, |bytes| bytes.fill(key.object().get() as u8))
            .unwrap();
    }

    let left_read = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: left,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    let reader_one = domain.acquire_call(&realized, &left_read).unwrap();
    let reader_two = domain.acquire_call(&realized, &left_read).unwrap();
    let left_write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: left,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    assert!(matches!(
        domain.acquire_call(&realized, &left_write),
        Err(MemoryRuntimeError::BorrowConflict { .. })
    ));
    let right_write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: right,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    assert!(domain.acquire_call(&realized, &right_write).is_ok());
    drop(reader_two);
    drop(reader_one);
    assert!(domain.acquire_call(&realized, &left_write).is_ok());

    let gap = MemoryAccessRegion::Contiguous {
        offset_bytes: 4,
        length_bytes: 4,
    };
    let empty_domain = MemoryDomain::new().unwrap();
    let empty_revision = empty_domain.issue_plan_revision().unwrap();
    let empty_realized = empty_domain
        .materialize(
            empty_domain
                .prepare_realization(RuntimePlanView::new(
                    empty_revision,
                    &[allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)],
                    &[arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let empty_key = empty_domain
        .plan_object_key(empty_revision, MemoryObjectId::new(0))
        .unwrap();
    assert!(matches!(
        empty_domain.prepare_call(
            &empty_realized,
            &[CallAccessRequest {
                object: empty_key,
                mode: MemoryAccessMode::Write,
                region: gap,
            }],
        ),
        Err(MemoryRuntimeError::UninitializedAccess { .. })
    ));
}

#[test]
fn overlapping_regions_require_a_disjoint_lifetime_reuse_group() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(
            0,
            0,
            0,
            0,
            32,
            MemoryLifetime::Turn {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(1),
            },
            Some(3),
        ),
        allocation(
            1,
            0,
            0,
            0,
            32,
            MemoryLifetime::Turn {
                first: MemoryPlanPoint::new(2),
                last: MemoryPlanPoint::new(3),
            },
            Some(3),
        ),
    ];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 32, &[0, 1])];
    assert!(
        domain
            .prepare_realization(RuntimePlanView::new(
                revision,
                &allocations,
                &arenas,
                ResourceDemand::default(),
                MemoryBudgetLimits::default(),
                &[],
            ))
            .is_ok()
    );

    let invalid_domain = MemoryDomain::new().unwrap();
    let invalid_revision = invalid_domain.issue_plan_revision().unwrap();
    let mut invalid = allocations;
    invalid[1].lifetime = MemoryLifetime::Turn {
        first: MemoryPlanPoint::new(1),
        last: MemoryPlanPoint::new(2),
    };
    assert!(matches!(
        invalid_domain.prepare_realization(RuntimePlanView::new(
            invalid_revision,
            &invalid,
            &arenas,
            ResourceDemand::default(),
            MemoryBudgetLimits::default(),
            &[],
        )),
        Err(MemoryRuntimeError::InvalidReuse { .. })
    ));
}

#[test]
fn handles_are_domain_scoped_and_stale_generations_are_rejected() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])];
    let first = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let key = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let old = first.binding(key).unwrap().handle().unwrap();
    let other = MemoryDomain::new().unwrap();
    assert!(matches!(
        other.retire(old),
        Err(MemoryRuntimeError::WrongMemoryDomain { .. })
    ));
    domain.retire(old).unwrap();
    assert_eq!(domain.collect_retired().unwrap(), 1);

    let next_revision = domain.issue_plan_revision().unwrap();
    let second = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
                    next_revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let next_key = domain
        .plan_object_key(next_revision, MemoryObjectId::new(0))
        .unwrap();
    let current = second.binding(next_key).unwrap().handle().unwrap();
    assert_eq!(old.slot(), current.slot());
    assert_ne!(old.generation(), current.generation());
    assert!(matches!(
        domain.retire(old),
        Err(MemoryRuntimeError::StaleAllocationGeneration { .. })
    ));
}

#[test]
fn indirect_payload_envelopes_are_owned_separately_and_charged_once() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let mut left = allocation(0, 0, 0, 0, 11, MemoryLifetime::Activation, None);
    left.role = AllocationRole::VariablePayload;
    left.alignment = 1;
    let mut right = allocation(1, 0, 0, 0, 13, MemoryLifetime::Activation, None);
    right.role = AllocationRole::VariablePayload;
    right.alignment = 1;
    let allocations = [left, right];
    let mut payload_arena = arena(0, ArenaBackingKind::IndirectOwnedPayloads, 24, &[0, 1]);
    payload_arena.alignment = 1;
    let realized = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
                    revision,
                    &allocations,
                    &[payload_arena],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    assert_eq!(realized.bindings().len(), 2);
    assert_eq!(domain.ledger().committed_bytes, 24);
    assert_eq!(domain.allocation_observations().len(), 2);
}

#[test]
fn publication_versions_change_only_for_changed_candidates() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, 0, 0, 8, MemoryLifetime::Activation, None)];
    let arenas = [arena(0, ArenaBackingKind::ContiguousBytes, 8, &[0])];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(RuntimePlanView::new(
                    revision,
                    &allocations,
                    &arenas,
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let key = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let binding = realized.binding(key).unwrap();
    assert!(matches!(binding, RuntimeBinding::ManagedHostRegion { .. }));
    let mut unchanged = domain
        .prepare_publication(
            &realized,
            vec![PublicationCandidate {
                object: key,
                binding: binding.clone(),
                shape: vec![1].into_boxed_slice(),
                changed: false,
            }],
        )
        .unwrap();
    let first = domain.commit_publication(&mut unchanged).unwrap();
    assert_eq!(first[0].version.get(), 1);
    assert!(matches!(
        domain.commit_publication(&mut unchanged),
        Err(MemoryRuntimeError::PublicationAlreadyCompleted)
    ));

    let mut changed = domain
        .prepare_publication(
            &realized,
            vec![PublicationCandidate {
                object: key,
                binding,
                shape: vec![1].into_boxed_slice(),
                changed: true,
            }],
        )
        .unwrap();
    let second = domain.commit_publication(&mut changed).unwrap();
    assert_eq!(second[0].version.get(), 1);
    let mut third = domain
        .prepare_publication(
            &realized,
            vec![PublicationCandidate {
                object: key,
                binding: second[0].binding.clone(),
                shape: vec![1].into_boxed_slice(),
                changed: true,
            }],
        )
        .unwrap();
    assert_eq!(
        domain.commit_publication(&mut third).unwrap()[0]
            .version
            .get(),
        2
    );
}
