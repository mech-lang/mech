use super::*;
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x52544952), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

#[test]
fn exact_retirement_sets_keep_captures_current_elements_and_tail_returns() {
    let tail = "#Scan(n<u64>, token<string>) => <(u64,string)>\n  | :Step(left<u64>, total<u64>)\n  | :Done(total<u64>).\n#Scan(n, token) -> :Step(n, 0u64)\n  :Step(left, total)\n    | left > 0u64 -> :Step(left - 1u64, total + 1u64)\n    | * -> :Done(total)\n  :Done(total) => (total, token).\n#Scan(input<u64>, payload<string>)\n";
    let nested =
        "+> text\n[text/join([inner | inner <- values], outer) | outer <- values<[string]>]\n";
    let folded = "identity(word<string>) => <string>\n  | value => value.\n[word |2> identity | word <- values<[string]>]\n";
    let mut saw_nested = false;
    let mut saw_tail = false;
    let mut saw_managed_slots = false;
    for source in [nested, folded, tail] {
        for artifact in artifacts(source) {
            let plan = mech_engine::native_artifact::lower_native_program_artifact(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            plan.validate_current_visits().unwrap();
            let tables = RetirementTables::derive(&plan).unwrap();
            for (iteration, control) in plan.iterations().iter().enumerate() {
                for kind in KINDS {
                    let slots = tables
                        .slots(tables.words(), iteration as u32, kind)
                        .unwrap();
                    assert!(slots.windows(2).all(|pair| pair[0] < pair[1]));
                    for &slot in slots {
                        saw_managed_slots = true;
                        let slot = slot as usize;
                        assert!(slot < plan.values().len());
                        assert!(plan.values()[slot].scalar_kind().is_none());
                        assert!(
                            !plan
                                .inputs()
                                .iter()
                                .any(|input| input.value().get() as usize == slot)
                        );
                        assert!(
                            !plan
                                .constants()
                                .iter()
                                .any(|value| value.value().get() as usize == slot)
                        );
                    }
                    let keep = match (control, kind) {
                        (NativeControlLoop::Collection(data), RetirementKind::Collection) => {
                            data.element()
                        }
                        (NativeControlLoop::Tail(data), RetirementKind::TailLatch) => {
                            Some(data.carrier())
                        }
                        (NativeControlLoop::Tail(data), RetirementKind::TailExit) => {
                            Some(data.returned())
                        }
                        _ => {
                            assert!(slots.is_empty());
                            continue;
                        }
                    };
                    // Independently enumerate the old complete scope predicate.
                    // This checks exact membership, not just exclusion examples.
                    let expected = plan
                        .value_scopes()
                        .iter()
                        .enumerate()
                        .filter_map(|(slot, scope)| {
                            if plan.values()[slot].scalar_kind().is_some()
                                || keep.is_some_and(|value| value.get() as usize == slot)
                            {
                                return None;
                            }
                            let mut owner = match scope {
                                NativeIterationValueScope::Immutable { iteration } => *iteration,
                                NativeIterationValueScope::Carrier { iteration } => {
                                    Some(*iteration)
                                }
                            };
                            while let Some(id) = owner {
                                if id as usize == iteration {
                                    return Some(slot as u64);
                                }
                                owner = plan.iterations()[id as usize].as_ref().parent();
                            }
                            None
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(slots, expected);
                    if let Some(keep) = keep {
                        assert!(!slots.contains(&u64::from(keep.get())));
                    }
                }
                match control {
                    NativeControlLoop::Collection(data) => {
                        if let Some(parent) = data.parent() {
                            saw_nested = true;
                            if let NativeControlLoop::Collection(outer) =
                                &plan.iterations()[parent as usize]
                            {
                                if let Some(element) = outer.element() {
                                    let slots = tables
                                        .slots(
                                            tables.words(),
                                            iteration as u32,
                                            RetirementKind::Collection,
                                        )
                                        .unwrap();
                                    assert!(!slots.contains(&u64::from(element.get())));
                                }
                            }
                        }
                    }
                    NativeControlLoop::Tail(data) => {
                        saw_tail = true;
                        let latch = tables
                            .slots(tables.words(), iteration as u32, RetirementKind::TailLatch)
                            .unwrap();
                        let exit = tables
                            .slots(tables.words(), iteration as u32, RetirementKind::TailExit)
                            .unwrap();
                        assert!(latch.contains(&u64::from(data.returned().get())));
                        assert!(exit.contains(&u64::from(data.carrier().get())));
                    }
                    NativeControlLoop::Iteration(_) => {}
                }
            }
        }
    }
    assert!(saw_nested && saw_tail && saw_managed_slots);
}

#[test]
fn retirement_lookup_refuses_wrong_sized_backing_and_out_of_range_prefixes() {
    let tables = RetirementTables {
        words: Box::new([4, 5, 5, 5, 2]),
        iterations: 1,
    };
    assert_eq!(
        tables
            .slots(tables.words(), 0, RetirementKind::Collection)
            .unwrap(),
        &[2]
    );
    assert!(tables.slots(&[], 0, RetirementKind::Collection).is_err());
    assert!(
        tables
            .slots(tables.words(), 1, RetirementKind::Collection)
            .is_err()
    );
    let changed = [0, 5, 5, 5, 2];
    assert!(
        tables
            .slots(&changed, 0, RetirementKind::Collection)
            .is_err()
    );
    let changed = [4, 6, 5, 5, 2];
    assert!(
        tables
            .slots(&changed, 0, RetirementKind::Collection)
            .is_err()
    );
}

#[test]
fn static_retirement_policy_rejects_preceding_work_policy_object_identities() {
    for artifact in artifacts("+> text\ntext/join([word | word <- values<[string]>], \"\")\n") {
        let catalog = mech_stdlib::source_catalog();
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let current = *object.identity().bytes();
        let policy = crate::native_artifact::NATIVE_PROGRAM_WORK_POLICY_VERSION;
        assert!(
            policy >= 12,
            "static retirement requires its policy boundary"
        );
        assert_eq!(
            u64::from_le_bytes(current[32..40].try_into().unwrap()),
            policy
        );
        assert!(
            crate::native_artifact::program_linked::prepare_descriptor(
                &artifact, &catalog, &current,
            )
            .is_ok()
        );
        for preceding in 0..policy {
            let mut old = current;
            old[32..40].copy_from_slice(&preceding.to_le_bytes());
            assert!(
                crate::native_artifact::program_linked::prepare_descriptor(
                    &artifact, &catalog, &old,
                )
                .is_err(),
                "old work policy {preceding} must not be relabeled as {policy}"
            );
        }
    }
}
