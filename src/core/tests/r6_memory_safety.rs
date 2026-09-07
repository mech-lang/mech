use mech_core::{
    AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement, ArenaPlan, CallAccessRequest,
    FunctionInvocation, ManagedCallAccessRequest, ManagedString, MemoryAccessMode,
    MemoryAccessRegion, MemoryArenaId, MemoryBudgetLimits, MemoryBudgetViolation, MemoryDomain,
    MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemoryPlanPoint, MemoryPlanRevision,
    MemoryRuntimeError, MemorySpace, ResourceDemand, ReuseGroupId, RuntimePlanView, ValueCell,
};

#[test]
fn owned_cell_session_close_revokes_cell_access_but_not_detached_snapshot() {
    let domain = MemoryDomain::new().unwrap();
    let cell = mech_core::ValueCell::from_exact_in(&domain, 41_u64).unwrap();
    let snapshot = cell.snapshot().unwrap();

    domain.close().unwrap();

    assert!(cell.snapshot().is_err());
    assert!(matches!(snapshot.data(), mech_core::ValueData::U64(41)));
}

fn runtime_plan_view<'a>(
    revision: MemoryPlanRevision,
    allocations: &'a [AllocationPlan],
    arenas: &'a [ArenaPlan],
    demand: ResourceDemand,
    limits: MemoryBudgetLimits,
    violations: &'a [MemoryBudgetViolation],
) -> RuntimePlanView<'a> {
    RuntimePlanView::new(
        revision,
        allocations,
        arenas,
        demand,
        0,
        limits,
        &[],
        64,
        violations,
    )
}

fn allocation(id: u32, offset: u64, lifetime: MemoryLifetime) -> AllocationPlan {
    AllocationPlan {
        id: MemoryObjectId::new(id),
        owner: MemoryObjectOwner::NodeScratch {
            node: mech_core::NodeId::new(id),
            ordinal: 0,
        },
        role: AllocationRole::Scratch,
        slot: Some(mech_core::PlannedSlotKind::FixedScalar(
            mech_core::ScalarMemoryKind::Unsigned(mech_core::IntegerWidth::W64),
        )),
        space: MemorySpace::Host,
        current_bytes: 8,
        capacity_bytes: 8,
        payload_block_capacity: 0,
        alignment: 8,
        lifetime,
        placement: ArenaPlacement {
            arena: MemoryArenaId::new(0),
            offset,
        },
        reuse_group: None,
    }
}

#[test]
fn payload_owner_outlives_domain_and_close_revokes_growth() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let mut payload = allocation(0, 0, MemoryLifetime::Activation);
    payload.role = AllocationRole::VariablePayload;
    payload.slot = None;
    payload.current_bytes = 0;
    payload.capacity_bytes = 8;
    payload.payload_block_capacity = 1;
    payload.alignment = 1;
    let payload_arena = ArenaPlan {
        id: MemoryArenaId::new(0),
        space: MemorySpace::Host,
        backing: ArenaBackingKind::IndirectOwnedPayloads,
        capacity_bytes: 8,
        alignment: 1,
        members: vec![MemoryObjectId::new(0)].into_boxed_slice(),
    };
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &[payload],
                    &[payload_arena],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let allocator = domain.planned_allocator(&realized, object).unwrap();
    let text = ManagedString::try_new(allocator.clone(), "retained").unwrap();
    domain.close().unwrap();
    assert!(matches!(
        ManagedString::try_new(allocator, "x"),
        Err(MemoryRuntimeError::DomainClosed)
    ));
    drop(realized);
    drop(domain);
    assert_eq!(text.as_str(), "retained");
}

#[test]
fn equal_size_scalar_types_cannot_open_each_others_planned_slots() {
    let domain = MemoryDomain::new().unwrap();
    let cell = ValueCell::from_exact_in(&domain, 0.0_f64).unwrap();
    let port = FunctionInvocation::nullary(cell)
        .output()
        .try_managed::<f64>()
        .unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, MemoryLifetime::Activation)];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &[arena(&[0], 8)],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let prepared = domain
        .prepare_managed_call(
            &realized,
            &[ManagedCallAccessRequest::new(
                port,
                object,
                MemoryAccessMode::Write,
                MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 8,
                },
            )],
        )
        .unwrap();
    let mut frame = domain.acquire_call(&realized, &prepared).unwrap();
    assert!(matches!(
        frame.with_port_init_writer(port, |writer| writer.write_next(1.0)),
        Err(MemoryRuntimeError::InvalidLayout { .. })
    ));
}

#[test]
fn initializer_marks_nothing_when_its_callback_returns_an_error() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, MemoryLifetime::Activation)];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &[arena(&[0], 8)],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    let mut frame = domain.acquire_call(&realized, &write).unwrap();
    frame
        .with_object_init_writer::<u8>(object, |_writer| Ok(()))
        .unwrap();
    drop(frame);
    assert_eq!(realized.binding(object).unwrap().initialized_bytes(), 0);

    let mut frame = domain.acquire_call(&realized, &write).unwrap();
    let injected = frame.with_object_init_writer::<u8>(object, |writer| {
        writer.write_next(7)?;
        Err::<(), _>(MemoryRuntimeError::DomainClosed)
    });
    assert!(matches!(injected, Err(MemoryRuntimeError::DomainClosed)));
    drop(frame);
    assert_eq!(realized.binding(object).unwrap().initialized_bytes(), 0);
    assert!(matches!(
        domain.prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        ),
        Err(MemoryRuntimeError::UninitializedAccess { .. })
    ));
}

#[test]
fn initialization_tracking_does_not_authorize_stride_gaps() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, MemoryLifetime::Activation)];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &[arena(&[0], 8)],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();

    for offset in [0, 2] {
        let write = domain
            .prepare_call(
                &realized,
                &[CallAccessRequest {
                    object,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: offset,
                        length_bytes: 1,
                    },
                }],
            )
            .unwrap();
        domain
            .acquire_call(&realized, &write)
            .unwrap()
            .with_object_init_writer::<u8>(object, |writer| writer.write_next(offset as u8 + 1))
            .unwrap();
    }

    let strided = MemoryAccessRegion::Strided {
        offset_bytes: 0,
        count: 2,
        stride_bytes: 2,
        element_bytes: 1,
    };
    let strided_read = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Read,
                region: strided,
            }],
        )
        .unwrap();
    let frame = domain.acquire_call(&realized, &strided_read).unwrap();
    assert!(matches!(
        frame.with_bytes(object, |_| ()),
        Err(MemoryRuntimeError::InvalidLayout { .. })
    ));
    drop(frame);

    let gap_read = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Read,
                region: MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 3,
                },
            }],
        )
        .unwrap();
    assert!(matches!(
        domain.acquire_call(&realized, &gap_read),
        Err(MemoryRuntimeError::UninitializedAccess { .. })
    ));
}

fn arena(objects: &[u32], bytes: u64) -> ArenaPlan {
    ArenaPlan {
        id: MemoryArenaId::new(0),
        space: MemorySpace::Host,
        backing: ArenaBackingKind::ContiguousBytes,
        capacity_bytes: bytes,
        alignment: 8,
        members: objects
            .iter()
            .copied()
            .map(MemoryObjectId::new)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }
}

#[test]
fn invalid_alias_construction_is_rejected_before_materialization() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(0, 0, MemoryLifetime::Activation),
        allocation(1, 0, MemoryLifetime::Activation),
    ];
    assert!(matches!(
        domain.prepare_realization(runtime_plan_view(
            revision,
            &allocations,
            &[arena(&[0, 1], 8)],
            ResourceDemand::default(),
            MemoryBudgetLimits::default(),
            &[],
        )),
        Err(MemoryRuntimeError::InvalidReuse { .. })
    ));
}

#[test]
fn a_lease_cannot_observe_a_reused_region_outside_its_lifetime() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let lifetime = MemoryLifetime::Turn {
        first: MemoryPlanPoint::new(1),
        last: MemoryPlanPoint::new(1),
    };
    let allocations = [allocation(0, 0, lifetime)];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &[arena(&[0], 8)],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let prepared = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::ExclusiveInPlace,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    assert!(matches!(
        domain.acquire_call(&realized, &prepared),
        Err(MemoryRuntimeError::InvalidLifetimeTransition { .. })
    ));
    let _scope = domain.enter_plan_point(MemoryPlanPoint::new(1)).unwrap();
    let mut frame = domain.acquire_call(&realized, &prepared).unwrap();
    frame
        .with_object_init_writer::<u8>(object, |writer| {
            writer.copy_from_slice(&7_u64.to_ne_bytes())
        })
        .unwrap();
    frame
        .with_bytes(object, |bytes| assert_eq!(bytes, 7_u64.to_ne_bytes()))
        .unwrap();
}

#[test]
fn unwind_releases_every_scoped_lease() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [allocation(0, 0, MemoryLifetime::Activation)];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &[arena(&[0], 8)],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let object = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let prepared = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _frame = domain.acquire_call(&realized, &prepared).unwrap();
        panic!("injected kernel unwind");
    }));
    assert_eq!(domain.allocation_observations()[0].active_leases, 0);
    drop(domain.acquire_call(&realized, &prepared).unwrap());
}

#[test]
fn region_reuse_revokes_the_previous_incarnation_and_waits_for_leases() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let mut first_allocation = allocation(
        0,
        0,
        MemoryLifetime::Turn {
            first: MemoryPlanPoint::new(1),
            last: MemoryPlanPoint::new(1),
        },
    );
    let mut second_allocation = allocation(
        1,
        0,
        MemoryLifetime::Turn {
            first: MemoryPlanPoint::new(2),
            last: MemoryPlanPoint::new(2),
        },
    );
    first_allocation.reuse_group = Some(ReuseGroupId::new(4));
    second_allocation.reuse_group = Some(ReuseGroupId::new(4));
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &[first_allocation, second_allocation],
                    &[arena(&[0, 1], 8)],
                    ResourceDemand::default(),
                    MemoryBudgetLimits::default(),
                    &[],
                ))
                .unwrap(),
        )
        .unwrap();
    let first = domain
        .plan_object_key(revision, MemoryObjectId::new(0))
        .unwrap();
    let second = domain
        .plan_object_key(revision, MemoryObjectId::new(1))
        .unwrap();
    let first_write = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object: first,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::WholeInitialized,
            }],
        )
        .unwrap();

    let initial = realized.binding(first).unwrap().incarnation();
    let first_scope = domain.enter_plan_point(MemoryPlanPoint::new(1)).unwrap();
    let first_live = realized.binding(first).unwrap().incarnation();
    assert!(first_live > initial);
    let held = domain.acquire_call(&realized, &first_write).unwrap();
    drop(first_scope);
    assert!(matches!(
        domain.enter_plan_point(MemoryPlanPoint::new(2)),
        Err(MemoryRuntimeError::OutstandingLease { .. })
    ));
    drop(held);

    let second_scope = domain.enter_plan_point(MemoryPlanPoint::new(2)).unwrap();
    let second_live = realized.binding(second).unwrap().incarnation();
    drop(second_scope);
    let first_before_reentry = realized.binding(first).unwrap().incarnation();
    let _first_scope = domain.enter_plan_point(MemoryPlanPoint::new(1)).unwrap();
    assert!(realized.binding(first).unwrap().incarnation() > first_before_reentry);
    assert_ne!(second_live, initial);
}
