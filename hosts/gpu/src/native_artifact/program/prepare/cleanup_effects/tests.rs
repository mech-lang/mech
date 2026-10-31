use super::*;
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn plans(source: &str) -> [NativeProgramArtifactPlan; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x434c4541), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let catalog = mech_stdlib::source_catalog();
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
        .map(|a| mech_engine::native_artifact::lower_native_program_artifact(&a, &catalog).unwrap())
}

#[test]
fn completed_children_keep_original_prices_returns_and_current_scope_effects() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    let mut omitted_count = 0;
    let mut return_count = 0;
    for plan in plans(source) {
        plan.validate_current_visits().unwrap();
        let frames = plan.facts().buffer_frames();
        let recipes = FrameRetirements::derive(&frames, plan.values().len()).unwrap();
        let tables = RetirementTables::derive(&plan, &plan.scalar_aggregates()).unwrap();
        let layout = Layout::derive(&plan, &tables, &frames, &recipes).unwrap();
        for iteration in 0..plan.iterations().len() {
            for kind in [RetirementKind::TailLatch, RetirementKind::TailExit] {
                let Some(summary) = layout.get(iteration as u32, kind) else {
                    continue;
                };
                let original = tables
                    .slots(tables.words(), iteration as u32, kind)
                    .unwrap();
                let price = |slot: u64| 32 + recipes.get(slot as usize).map_or(0, |r| r.0);
                assert_eq!(
                    summary.total,
                    original.iter().copied().map(price).sum::<u64>()
                );
                let omitted = original
                    .iter()
                    .copied()
                    .filter(|s| !summary.effects.contains(s))
                    .collect::<Vec<_>>();
                assert_eq!(
                    summary.omitted_work,
                    omitted.iter().copied().map(price).sum::<u64>()
                );
                assert!(summary.effects.windows(2).all(|p| p[0] < p[1]));
                for &slot in &omitted {
                    // Independent exit-inventory oracle: every omitted slot is
                    // already swept by a successful strict child's exit.
                    assert!(
                        plan.iterations()
                            .iter()
                            .enumerate()
                            .any(|(child, control)| {
                                child != iteration
                                    && matches!(control, NativeControlLoop::Tail(_))
                                    && tables
                                        .slots(
                                            tables.words(),
                                            child as u32,
                                            RetirementKind::TailExit,
                                        )
                                        .unwrap()
                                        .contains(&slot)
                            })
                    );
                }
                omitted_count += omitted.len();
                for control in plan.iterations() {
                    if let NativeControlLoop::Tail(tail) = control {
                        let returned = u64::from(tail.returned().get());
                        if original.contains(&returned) {
                            return_count += 1;
                            assert!(summary.effects.contains(&returned));
                        }
                    }
                }
                for &slot in original {
                    if plan.value_scopes()[slot as usize]
                        == (NativeIterationValueScope::Immutable {
                            iteration: Some(iteration as u32),
                        })
                        || plan.value_scopes()[slot as usize]
                            == (NativeIterationValueScope::Carrier {
                                iteration: iteration as u32,
                            })
                    {
                        assert!(summary.effects.contains(&slot));
                    }
                }
            }
        }
    }
    assert!(omitted_count > 0 && return_count > 0);
}

#[test]
fn collection_boundaries_and_scalar_only_programs_keep_original_inventories() {
    let crossing = "+> text\ninner(n<u64>, token<string>) => <string> | (n, token) => ((n > 0u64)? | true => inner(n - 1u64, token) | false => token).\nouter(n<u64>, token<string>) => (out<string>) :=\n  joined := text/join([inner(k, token) | k <- [1u64, 2u64]], \"\")\n  out := ((n > 0u64)? | true => outer(n - 1u64, joined) | false => joined).\nouter(n<u64>, token<string>)\n";
    for source in [
        "seed<u64> + 1u64\n",
        "+> text\n[text/join([inner | inner <- values], outer) | outer <- values<[string]>]\n",
    ] {
        for plan in plans(source) {
            let frames = plan.facts().buffer_frames();
            let recipes = FrameRetirements::derive(&frames, plan.values().len()).unwrap();
            let tables = RetirementTables::derive(&plan, &plan.scalar_aggregates()).unwrap();
            let layout = Layout::derive(&plan, &tables, &frames, &recipes).unwrap();
            assert!(layout.boundaries.is_empty());
        }
    }
    // The source pipeline currently rejects recursive calls inside a
    // comprehension, before a native child/collection crossing can exist.
    // Preserve that rejection rather than fabricating an unreachable plan.
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x434c4541), Revision(1), crossing).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let Err(error) = CanonicalSourceFrontend.compile_document_with_catalog(
        &DocumentSyntax::cast(parsed.syntax()).unwrap(),
        mech_stdlib::source_catalog(),
    ) else {
        panic!("recursive comprehension must retain its source rejection")
    };
    assert_eq!(error.code, "source-semantics/recursive-comprehension");
}
