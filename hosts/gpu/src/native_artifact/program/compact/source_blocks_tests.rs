use super::*;

#[test]
fn source_boundaries_follow_unit_allocation_instead_of_old_native_numbers() {
    let mut global = SourceBlocks::new(vec![Some(1), Some(2), Some(3), Some(4)]);
    global.record(1, 20, 24);
    let mut local = SourceBlocks::new(vec![None, Some(1), Some(2), None]);
    local.record(1, 4, 8);
    assert_eq!(
        local.summary()["auxiliary_ranges"][1],
        serde_json::json!([4, 8])
    );
    let specification = "s1:a1.2:1";
    let global = global.windows(specification).unwrap();
    let local = local.windows(specification).unwrap();
    assert_eq!((global[0][0].entry, global[0][0].exit), (2, 22));
    assert_eq!((local[0][0].entry, local[0][0].exit), (1, 6));
    assert_eq!(global[0][0].origin, local[0][0].origin);
}

#[test]
fn partial_windows_are_omitted_and_original_family_positions_survive() {
    let mut unit = SourceBlocks::new(vec![None, Some(1), Some(2), None]);
    unit.record(0, 1, 4);
    unit.record(1, 4, 7);
    assert!(unit.summary()["auxiliary_ranges"][0].is_null());
    let families = unit.windows("s0:s1:0,s1:s3:1;s1:a1.1:1").unwrap();
    assert!(families[0].is_empty());
    assert_eq!(families[1].len(), 1);
    assert_eq!(families[1][0].ordinal, 2);
    assert_eq!((families[1][0].entry, families[1][0].exit), (1, 5));
}

#[test]
fn changed_local_lowering_refuses_an_old_auxiliary_offset() {
    let mut unit = SourceBlocks::new(vec![Some(1), Some(2)]);
    unit.record(0, 3, 5);
    assert!(unit.windows("s0:a0.2:0").is_err());
    assert!(unit.windows("a0.18446744073709551615:s1:0").is_err());
    assert!(unit.windows("s0:s0:0").is_err());
}

#[test]
fn malformed_or_out_of_plan_hints_do_not_select_native_blocks() {
    let unit = SourceBlocks::new(vec![Some(1), Some(2)]);
    for specification in [
        "1:2:0",
        "s0:s1",
        "s0:s1:2",
        "s2:s1:0",
        "a2.0:s1:0",
        "s-1:s1:0",
        "s0:a1.bad:0",
        "s0:s1:bad",
    ] {
        assert!(unit.windows(specification).is_err(), "{specification}");
    }
}
