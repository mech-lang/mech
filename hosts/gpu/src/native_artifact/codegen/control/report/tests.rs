use crate::native_artifact::codegen::control::codegen_report::*;
#[test]
fn emission_receipts_are_bounded_and_report_incomplete_coverage() {
    let mut report = CodegenReport::default();
    for header in 0..MAX_REGIONS {
        assert!(report.packed_ssa_region(header).is_some());
    }
    assert!(report.source_edge_facts.complete);
    assert!(report.packed_ssa_region(MAX_REGIONS).is_none());
    assert!(!report.source_edge_facts.complete);
    assert_eq!(report.source_edge_facts.regions.len(), MAX_REGIONS);
    // The admission lane is shared by both substitution kinds; overflow
    // cannot quietly become a receipt claiming complete site coverage.
    let mut report = CodegenReport::default();
    let region = report.packed_ssa_region(19);
    for _ in 0..MAX_SUBSTITUTIONS {
        assert!(report.admit(region).is_some());
    }
    assert!(report.source_edge_facts.complete);
    assert!(report.admit(region).is_none());
    assert!(!report.source_edge_facts.complete);
    assert_eq!(report.substitutions, MAX_SUBSTITUTIONS);
    let mut report = CodegenReport::default();
    report.tag_threading(
        1,
        2,
        vec![0, 1],
        32,
        64,
        3,
        vec![9; MAX_SUBSTITUTIONS - 4],
        vec![10; 2],
    );
    assert!(report.tag_threading.complete);
    assert_eq!(report.tag_threading.regions.len(), 1);
    report.tag_threading(1, 2, vec![0, 1], 32, 64, 3, vec![9], vec![10]);
    assert!(!report.tag_threading.complete);
    assert_eq!(
        report.tag_threading.regions.len(),
        1,
        "bounded receipt must flag incomplete coverage"
    );
    let mut report = CodegenReport::default();
    for header in 0..MAX_REGIONS {
        report.frame_effects(header, vec![(0, 3, 8, 16)]);
    }
    assert!(report.frame_effects.complete);
    report.frame_effects(MAX_REGIONS, vec![]);
    assert!(!report.frame_effects.complete);
    assert_eq!(report.frame_effects.regions.len(), MAX_REGIONS);
    let mut report = CodegenReport::default();
    report.frame_effects(0, vec![(0, 1, 0, 0); 15]);
    assert!(!report.frame_effects.complete);
    assert!(report.frame_effects.regions.is_empty());
    let mut report = CodegenReport::default();
    for header in 0..MAX_REGIONS {
        report.guarded_loop(
            header,
            1,
            2,
            3,
            0,
            [4, 5, 6],
            7,
            vec![8],
            64,
            vec![39],
            1024,
        );
    }
    assert!(report.guarded_loops.complete);
    report.guarded_loop(0, 1, 2, 3, 0, [4, 5, 6], 7, vec![8], 64, vec![39], 1024);
    assert!(!report.guarded_loops.complete);
    assert_eq!(report.guarded_loops.regions.len(), MAX_REGIONS);
    let mut report = CodegenReport::default();
    for header in 0..MAX_REGIONS {
        report.lookup_facts(header, 32, 64, vec![(1, 2, 3, "Fresh")]);
    }
    assert!(report.lookup_facts.complete);
    report.lookup_facts(0, 32, 64, vec![]);
    assert!(!report.lookup_facts.complete);
    assert_eq!(report.lookup_facts.regions.len(), MAX_REGIONS);
    let mut report = CodegenReport::default();
    report.lookup_facts(0, 32, 64, vec![(1, 2, 3, "Cached"); MAX_SUBSTITUTIONS + 1]);
    assert!(!report.lookup_facts.complete);
    assert!(report.lookup_facts.regions.is_empty());
}
