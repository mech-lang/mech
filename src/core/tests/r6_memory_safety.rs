#![cfg(all(
    feature = "functions",
    feature = "u8",
    feature = "u64",
    feature = "f64",
    feature = "string",
    feature = "matrixd"
))]

use mech_core::{
    AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement, ArenaPlan, CallAccessRequest,
    CellPublicationCandidate, CellPublicationEvidence, FunctionInvocation,
    ManagedCallAccessRequest, ManagedString, MemoryAccessMode, MemoryAccessRegion, MemoryArenaId,
    MemoryBudgetLimits, MemoryBudgetViolation, MemoryDomain, MemoryLifetime, MemoryObjectId,
    MemoryObjectOwner, MemoryPlanPoint, MemoryPlanRevision, MemoryRuntimeError, MemorySpace,
    ResourceDemand, ReuseGroupId, RuntimePlanView, ValueCell,
};

#[path = "support/r6_allocation_probe.rs"]
mod allocation_probe;

#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

#[test]
fn explicit_external_wrappers_share_one_publication_record() {
    let initial = ValueCell::from_exact(7.0_f64).unwrap().snapshot().unwrap();
    let schemas = std::rc::Rc::new(initial.schemas().as_ref().unwrap().as_ref().clone());
    let external = mech_core::Ref::new(7.0_f64);
    let first = ValueCell::from_ref(
        external.clone(),
        initial.schema(),
        initial.shape().clone(),
        schemas.clone(),
    )
    .unwrap();
    let second = ValueCell::from_ref(
        external.clone(),
        initial.schema(),
        initial.shape().clone(),
        schemas.clone(),
    )
    .unwrap();
    let before = second.published_version();
    first
        .replace(&ValueCell::from_exact(11.0_f64).unwrap().snapshot().unwrap())
        .unwrap();
    assert_eq!(*external.borrow(), 11.0);
    assert_ne!(second.published_version(), before);
    assert_eq!(first.published_version(), second.published_version());
    assert!(first.same_logical_cell(&second));
    let last = ValueCell::from_ref(
        external.clone(),
        initial.schema(),
        initial.shape().clone(),
        schemas,
    )
    .unwrap();
    assert_eq!(last.published_version(), second.published_version());
    assert!(
        matches!(last.snapshot().unwrap().data(), mech_core::ValueData::F64(value) if value.to_f64() == 11.0)
    );
}

#[test]
fn external_registration_rejects_incompatible_schema_without_replacing_the_record() {
    let initial = ValueCell::from_exact(7.0_f64).unwrap().snapshot().unwrap();
    let schemas = std::rc::Rc::new(initial.schemas().as_ref().unwrap().as_ref().clone());
    let external = mech_core::Ref::new(7.0_f64);
    let first = ValueCell::from_ref(
        external.clone(),
        initial.schema(),
        initial.shape().clone(),
        schemas.clone(),
    )
    .unwrap();
    let incompatible = ValueCell::from_exact(7_u64).unwrap().snapshot().unwrap();
    let error = ValueCell::from_ref(
        external.clone(),
        incompatible.schema(),
        incompatible.shape().clone(),
        std::rc::Rc::new(incompatible.schemas().as_ref().unwrap().as_ref().clone()),
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "ValueCellSchemaMismatch");
    let alias =
        ValueCell::from_ref(external, initial.schema(), initial.shape().clone(), schemas).unwrap();
    first
        .replace(&ValueCell::from_exact(8.0_f64).unwrap().snapshot().unwrap())
        .unwrap();
    assert_eq!(alias.published_version(), first.published_version());
}

#[test]
fn repeated_complete_call_acquisition_and_release_allocate_no_metadata() {
    let domain = MemoryDomain::new().unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocations = [
        allocation(0, 0, MemoryLifetime::Activation),
        allocation(1, 8, MemoryLifetime::Activation),
    ];
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &allocations,
                    &[arena(&[0, 1], 16)],
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
    let region = MemoryAccessRegion::Contiguous {
        offset_bytes: 0,
        length_bytes: 8,
    };
    let initialize = domain
        .prepare_call(
            &realized,
            &[
                CallAccessRequest {
                    object: first,
                    mode: MemoryAccessMode::Write,
                    region,
                },
                CallAccessRequest {
                    object: second,
                    mode: MemoryAccessMode::Write,
                    region,
                },
            ],
        )
        .unwrap();
    {
        let mut frame = domain.acquire_call(&realized, &initialize).unwrap();
        frame
            .with_object_init_writer::<u64>(first, |writer| writer.write_next(17))
            .unwrap();
        frame
            .with_object_init_writer::<u64>(second, |writer| writer.write_next(0))
            .unwrap();
    }
    let prepared = domain
        .prepare_call(
            &realized,
            &[
                CallAccessRequest {
                    object: first,
                    mode: MemoryAccessMode::Read,
                    region,
                },
                CallAccessRequest {
                    object: second,
                    mode: MemoryAccessMode::Write,
                    region,
                },
            ],
        )
        .unwrap();
    let before = domain.ledger();
    let (_, count) = allocation_probe::measured(|| {
        for _ in 0..128 {
            let mut frame = domain.acquire_call(&realized, &prepared).unwrap();
            let value = frame
                .with_bytes(first, |bytes| u64::from_ne_bytes(bytes.try_into().unwrap()))
                .unwrap();
            frame
                .with_bytes_mut(second, |bytes| bytes.copy_from_slice(&value.to_ne_bytes()))
                .unwrap();
        }
    });
    assert_eq!(count, 0, "fixed-width lease acquisition allocated metadata");
    assert_eq!(domain.ledger(), before);
    assert!(
        domain
            .allocation_observations()
            .iter()
            .all(|allocation| allocation.active_leases == 0)
    );
}

#[test]
fn managed_numeric_footprint_measurement_does_not_copy_matrix_payload() {
    let domain = MemoryDomain::new().unwrap();
    let cell = ValueCell::from_exact_in(
        &domain,
        nalgebra::DMatrix::<f64>::from_element(512, 512, 1.0),
    )
    .unwrap();
    let (footprint, _, allocated_bytes) =
        allocation_probe::measured_with_bytes(|| cell.current_memory_footprint().unwrap());

    assert_eq!(footprint.logical_elements, 512 * 512);
    assert_eq!(footprint.fixed_bytes, 512 * 512 * 8);
    assert_eq!(footprint.payload_bytes, 0);
    assert!(
        allocated_bytes < 16 * 1024,
        "footprint inspection allocated {allocated_bytes} bytes for a managed numeric matrix"
    );
}

#[test]
fn fixed_width_publication_retains_region_evidence_without_a_canonical_copy() {
    let rows = 256_u64;
    let columns = 256_u64;
    let elements = rows * columns;
    let bytes = elements * 8;
    let domain = MemoryDomain::new().unwrap();
    let cell = ValueCell::from_exact_in(
        &domain,
        nalgebra::DMatrix::<f64>::from_element(rows as usize, columns as usize, 1.0),
    )
    .unwrap();
    let revision = domain.issue_plan_revision().unwrap();
    let allocation = AllocationPlan {
        id: MemoryObjectId::new(0),
        owner: MemoryObjectOwner::DirectCallPort {
            call: 0,
            direction: mech_core::PortDirection::Output,
            port: 0,
        },
        role: AllocationRole::TransactionStage,
        slot: Some(mech_core::PlannedSlotKind::FixedScalar(
            mech_core::ScalarMemoryKind::Floating(mech_core::FloatWidth::W64),
        )),
        space: MemorySpace::Host,
        current_bytes: 0,
        capacity_bytes: bytes,
        payload_block_capacity: 0,
        alignment: 8,
        lifetime: MemoryLifetime::Activation,
        placement: ArenaPlacement {
            arena: MemoryArenaId::new(0),
            offset: 0,
        },
        reuse_group: None,
    };
    let arena = ArenaPlan {
        id: MemoryArenaId::new(0),
        space: MemorySpace::Host,
        backing: ArenaBackingKind::ContiguousBytes,
        capacity_bytes: bytes,
        alignment: 8,
        members: vec![MemoryObjectId::new(0)].into_boxed_slice(),
    };
    let realized = domain
        .materialize(
            domain
                .prepare_realization(runtime_plan_view(
                    revision,
                    &[allocation],
                    &[arena],
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
    domain.activate_realization(&realized).unwrap();
    let region = MemoryAccessRegion::Rectangle {
        offset_bytes: 0,
        rows,
        columns,
        row_stride_bytes: 8,
        column_stride_bytes: rows * 8,
        element_bytes: 8,
    };
    let initialization = domain
        .prepare_call(
            &realized,
            &[CallAccessRequest {
                object,
                mode: MemoryAccessMode::Write,
                region: MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: bytes,
                },
            }],
        )
        .unwrap();
    let values = vec![22.0_f64; elements as usize];
    domain
        .acquire_call(&realized, &initialization)
        .unwrap()
        .with_object_init_writer::<f64>(object, |writer| writer.copy_from_slice(&values))
        .unwrap();
    let shape = cell.shape().clone();
    let (_, _, allocated_bytes) = allocation_probe::measured_with_bytes(|| {
        let prepared = domain
            .prepare_cell_publication(
                &realized,
                vec![CellPublicationCandidate {
                    cell: cell.clone(),
                    object,
                    binding: realized.binding(object).unwrap(),
                    region,
                    evidence: CellPublicationEvidence::initialized_region(shape),
                    changed: true,
                }],
            )
            .unwrap();
        domain.ready_cell_publication(prepared).unwrap().commit()
    });
    assert!(
        allocated_bytes < 64 * 1024,
        "fixed-width publication allocated {allocated_bytes} bytes of evidence for a {bytes}-byte initialized region",
    );
    assert_eq!(cell.current_memory_footprint().unwrap().fixed_bytes, bytes);
}

#[test]
fn owned_cell_session_close_revokes_cell_access_but_not_detached_snapshot() {
    let domain = MemoryDomain::new().unwrap();
    let cell = mech_core::ValueCell::from_exact_in(&domain, 41_u64).unwrap();
    let snapshot = cell.snapshot().unwrap();

    domain.close().unwrap();

    assert!(cell.snapshot().is_err());
    assert!(matches!(snapshot.data(), mech_core::ValueData::U64(41)));
}

#[test]
fn canonical_snapshots_share_frozen_payload_and_retain_one_charge_until_last_drop() {
    let domain = MemoryDomain::new().unwrap();
    let cell = ValueCell::from_exact_in(&domain, "retained payload".to_owned()).unwrap();
    let charged = domain.ledger().exported_snapshot_bytes;
    assert!(charged > 0);

    let first = cell.snapshot().unwrap();
    let second = cell.snapshot().unwrap();
    assert!(first.shares_frozen_storage(&second));
    assert_eq!(domain.ledger().exported_snapshot_bytes, charged);

    domain.close().unwrap();
    drop(cell);
    assert_eq!(domain.ledger().exported_snapshot_bytes, charged);
    assert!(
        matches!(first.data(), mech_core::ValueData::String(value) if value.as_ref() == "retained payload")
    );
    drop(first);
    assert_eq!(domain.ledger().exported_snapshot_bytes, charged);
    drop(second);
    assert!(domain.ledger().exported_snapshot_bytes < charged);
    domain.collect_retired().unwrap();
    assert_eq!(domain.ledger().exported_snapshot_bytes, 0);
}

#[test]
fn owned_dynamic_cell_growth_moves_its_actual_backing_and_preserves_snapshots() {
    let domain = MemoryDomain::new().unwrap();
    let cell = ValueCell::from_exact_in(
        &domain,
        nalgebra::DMatrix::from_row_slice(2, 3, &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0]),
    )
    .unwrap();
    let alias = cell.clone();
    let before = cell.snapshot().unwrap();
    let original_version = cell.published_version();
    let replacement = ValueCell::from_exact(nalgebra::DMatrix::from_row_slice(
        3,
        2,
        &[7.0_f64, 8.0, 9.0, 10.0, 11.0, 12.0],
    ))
    .unwrap()
    .snapshot()
    .unwrap();
    cell.replace(&replacement).unwrap();
    let after = alias.snapshot().unwrap();
    assert!(cell.same_logical_cell(&alias));
    assert_ne!(cell.published_version(), original_version);
    assert_eq!(after.shape().parameter_values(), &[3, 2]);
    let mech_core::ValueData::Matrix(matrix) = after.data() else {
        panic!("expected matrix")
    };
    let mech_core::snapshot::SequenceView::F64(values) = matrix.elements() else {
        panic!("expected F64")
    };
    assert_eq!(
        values
            .iter()
            .map(|value| value.to_f64())
            .collect::<Vec<_>>(),
        vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]
    );
    assert_eq!(before.shape().parameter_values(), &[2, 3]);
    domain.close().unwrap();
    assert_eq!(before.shape().parameter_values(), &[2, 3]);
    assert_eq!(after.shape().parameter_values(), &[3, 2]);
    assert!(alias.snapshot().is_err());
}

#[test]
fn owned_dynamic_shrink_publishes_only_initialized_logical_slots() {
    let domain = MemoryDomain::new().unwrap();
    let cell =
        ValueCell::from_exact_in(&domain, nalgebra::DMatrix::from_element(3, 2, 7.0_f64)).unwrap();
    let alias = cell.clone();
    let retained = cell.snapshot().unwrap();
    let allocations = domain.ledger().live_allocations;
    for (rows, columns, values) in [
        (1, 2, vec![11.0_f64, 12.0]),
        (0, 2, vec![]),
        (3, 0, vec![]),
        (3, 2, vec![21.0, 22.0, 23.0, 24.0, 25.0, 26.0]),
    ] {
        let replacement =
            ValueCell::from_exact(nalgebra::DMatrix::from_row_slice(rows, columns, &values))
                .unwrap()
                .snapshot()
                .unwrap();
        cell.replace(&replacement).unwrap();
        let published = alias.snapshot().unwrap();
        assert!(
            published
                .snapshot_eq(
                    &published.schemas().unwrap(),
                    &replacement,
                    &replacement.schemas().unwrap()
                )
                .unwrap()
        );
        // The 1x2 output has a capacity-dependent column stride of three.
        // Its gaps are not initialized, exposed, or filled to the old extent.
        assert_eq!(domain.ledger().live_allocations, allocations);
    }
    domain.close().unwrap();
    drop(cell);
    drop(alias);
    domain.collect_retired().unwrap();
    assert_eq!(domain.ledger().committed_bytes, 0);
    assert_eq!(retained.shape().parameter_values(), &[3, 2]);
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
    payload.capacity_bytes = 16;
    payload.payload_block_capacity = 1;
    payload.alignment = 1;
    let payload_arena = ArenaPlan {
        id: MemoryArenaId::new(0),
        space: MemorySpace::Host,
        backing: ArenaBackingKind::IndirectOwnedPayloads,
        capacity_bytes: 16,
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
    assert!(matches!(
        ManagedString::try_new(allocator.clone(), "x"),
        Err(MemoryRuntimeError::UnplannedAllocation { .. })
    ));
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
                &port,
                object,
                MemoryAccessMode::Write,
                MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 8,
                },
            )],
        )
        .unwrap();
    domain.activate_realization(&realized).unwrap();
    let mut frame = domain.acquire_call(&realized, &prepared).unwrap();
    assert!(matches!(
        frame.with_port_init_writer(&port, |writer| writer.write_next(1.0)),
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
                region: MemoryAccessRegion::Contiguous {
                    offset_bytes: 0,
                    length_bytes: 8,
                },
            }],
        )
        .unwrap();
    let mut frame = domain.acquire_call(&realized, &write).unwrap();
    frame
        .with_object_init_writer::<u8>(object, |_writer| Ok(()))
        .unwrap();
    assert!(matches!(
        frame.with_object_value_view::<u64, _>(object, |_| ()),
        Err(MemoryRuntimeError::UninitializedAccess { .. })
    ));
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
