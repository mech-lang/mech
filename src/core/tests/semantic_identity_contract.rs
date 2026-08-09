use mech_core::*;

#[test]
fn resident_ids_keep_their_exact_scalar_layout() {
    assert_eq!(
        core::mem::size_of::<CellSlotId>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::align_of::<CellSlotId>(),
        core::mem::align_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<SlotIndex>(),
        core::mem::size_of::<u32>()
    );
    assert_eq!(
        core::mem::align_of::<SlotIndex>(),
        core::mem::align_of::<u32>()
    );
    assert_eq!(
        core::mem::size_of::<InstanceEpoch>(),
        core::mem::size_of::<u64>()
    );
    assert_eq!(
        core::mem::align_of::<InstanceEpoch>(),
        core::mem::align_of::<u64>()
    );
}

#[test]
fn final_artifact_identities_are_dense_u32_domains() {
    macro_rules! assert_artifact_id {
        ($identity:ty) => {
            assert_eq!(core::mem::size_of::<$identity>(), 4);
            assert_eq!(core::mem::align_of::<$identity>(), 4);
            assert_eq!(<$identity>::new(17).get(), 17);
        };
    }

    assert_artifact_id!(NodeId);
    assert_artifact_id!(BindingId);
    assert_artifact_id!(InputId);
    assert_artifact_id!(OutputId);
    assert_artifact_id!(IntegrityConstraintId);

    fn accepts_node(_: NodeId) {}
    accepts_node(NodeId(3));
}

#[cfg(feature = "resident-execution")]
#[test]
fn resident_execution_paths_reexport_the_final_identity_types() {
    fn accepts_slot(_: mech_core::resident_execution::CellSlotId) {}
    fn accepts_index(_: mech_core::resident_execution::SlotIndex) {}
    fn accepts_epoch(_: mech_core::resident_execution::InstanceEpoch) {}

    accepts_slot(CellSlotId::new(1));
    accepts_index(SlotIndex::new(2));
    accepts_epoch(InstanceEpoch::new(3));
}

#[test]
fn every_generation_increment_is_checked() {
    assert!(InstanceEpoch::new(u64::MAX).checked_next().is_err());
    assert!(PlanGeneration::new(u64::MAX).checked_next().is_err());
    assert!(LayoutGeneration::new(u64::MAX).checked_next().is_err());
    assert!(
        ReactiveInstanceId::new(4, u32::MAX)
            .checked_next_generation()
            .is_err()
    );
}

#[test]
fn cell_identity_includes_instance_generation_but_not_plan_generation() {
    let before = ReactiveInstanceId::new(9, 1);
    let after = before.checked_next_generation().unwrap();
    assert_ne!(
        CellId::new(before, CellSlotId(3)),
        CellId::new(after, CellSlotId(3))
    );
    assert_eq!(before, ReactiveInstanceId::new(9, 1));
    assert_eq!(PlanGeneration::new(12).get(), 12);
}

#[test]
fn nominal_path_validation_and_hashing_are_canonical() {
    for segments in [vec![], vec![""], vec!["."], vec![".."], vec!["a\0b"]] {
        let owned = segments.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert!(CanonicalNominalPath::new(owned.into_boxed_slice()).is_err());
    }

    let path = CanonicalNominalPath::new(
        vec!["fixture".to_owned(), "Choice".to_owned()].into_boxed_slice(),
    )
    .unwrap();
    assert_eq!(
        NominalKey::from_path(NominalKind::Enum, &path).into_bytes(),
        [
            0x8f, 0x9a, 0x09, 0x64, 0x45, 0x0f, 0xe4, 0xb4, 0x28, 0x88, 0xa0, 0x7c, 0xb9, 0x0a,
            0xa3, 0x31, 0x29, 0x45, 0x38, 0x13, 0xcc, 0xc9, 0xb9, 0x80, 0x5b, 0x95, 0x8c, 0xaf,
            0x83, 0xfe, 0x7a, 0xd9,
        ]
    );
}
