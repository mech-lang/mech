//! Private tagged map carriers retain current owners across unequal variants.
//! Variant payload tuples and terminal named records never escape.
use super::*;
use mech_core::snapshot::MapEntryDraft;
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame,
};

fn selected_sites(jit: &JitProgramArtifact) -> std::collections::BTreeSet<u64> {
    let plan = jit.descriptor().plan();
    let frames = plan.facts().buffer_frames();
    let mut packs = std::collections::BTreeSet::new();
    let mut observes = 0;
    let mut map_projections = 0;
    for (site, block) in plan.blocks().iter().enumerate() {
        if let Some(action) = frames.action(site) {
            match action.operation() {
                Frame::EnumPack { output, .. } => {
                    let [root] = frames.fields(*output).unwrap() else {
                        panic!("one tagged root")
                    };
                    assert!(matches!(root.lane(), Lane::Tagged { count: 2, .. }));
                    let spans = (0..2)
                        .map(|ordinal| {
                            frames
                                .children(frames.tagged_variant(*root, ordinal).unwrap())
                                .unwrap()
                                .len()
                        })
                        .collect::<std::collections::BTreeSet<_>>();
                    assert_eq!(
                        spans,
                        [2, 3].into_iter().collect(),
                        "variant payloads must have independent unequal spans"
                    );
                    packs.insert(site as u64);
                }
                Frame::Observe { .. } => observes += 1,
                Frame::Project { output, .. }
                    if matches!(
                        plan.values()[output.get() as usize].definition().body(),
                        mech_core::SchemaBody::Map { .. }
                    ) =>
                {
                    assert!(matches!(
                        block.action(),
                        mech_engine::native_artifact::NativeIterationAction::LibraryCall { .. }
                    ));
                    map_projections += 1;
                }
                _ => {}
            }
        }
    }
    assert!(
        !packs.is_empty(),
        "private mixed map FSM must select tagged frames"
    );
    assert!(
        observes > 0,
        "tag/payload observations must use the exact tagged layout"
    );
    assert!(
        map_projections > 0,
        "the current record's map field must remain a private projection"
    );
    packs
}

pub(super) const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/private-tagged-map.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    keys: &[&str],
    steps: u64,
    seed: u64,
    old: u64,
    marker: u64,
    query: &str,
    allowed: bool,
    divisor: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "keys",
                Data::Matrix(keys.iter().map(|key| Data::String((*key).into())).collect()),
            ),
            (
                "initial",
                Data::Map(
                    [MapEntryDraft {
                        items: [Data::String("old".into()), Data::U64(old)].into(),
                    }]
                    .into(),
                ),
            ),
            ("steps", Data::U64(steps)),
            ("seed", Data::U64(seed)),
            ("marker", Data::U64(marker)),
            ("query", Data::String(query.into())),
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

fn drafts(outputs: &[Option<Value>]) -> Vec<Data> {
    outputs
        .iter()
        .map(|value| value.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}

fn assert_prior(outputs: &[Option<Value>], prior: &[Option<Value>]) {
    assert_eq!(drafts(outputs), drafts(prior));
    assert!(outputs.iter().zip(prior).all(|(actual, old)| {
        actual
            .as_ref()
            .unwrap()
            .shares_frozen_storage(old.as_ref().unwrap())
    }));
}

fn expected(keys: &[&str], steps: u64, seed: u64, old: u64, marker: u64, query: &str) -> Data {
    let mut index = std::collections::BTreeMap::from([("old", old)]);
    let mut count = seed;
    for key in &keys[..steps as usize] {
        if !index.contains_key(key) {
            index.insert(key, count);
            count += 1;
        }
    }
    Data::U64(index[query] + count + marker)
}

#[test]
fn tagged_map_carrier_keeps_current_maps_scalars_and_record_tuple_captures() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        // Bool is deliberately outside the retained Map+U64 compound-result
        // witness. The original Map+U64 fixture is exercised independently in
        // retained_compound_map; this one must keep the generic tagged path.
        assert!(
            jit.descriptor()
                .plan()
                .facts()
                .map_owner_words()
                .unwrap()
                .regions()
                .is_empty()
        );
        let packs = selected_sites(&jit);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for (keys, steps, seed, old, marker, query) in [
                (vec!["old", "a", "a", "é"], 4, 8, 40, 100, "a"),
                (vec!["é", "é", "a"], 3, 3, 17, 7, "é"),
                (vec![], 0, 23, 91, 2, "old"),
                (vec!["old", "old", "old"], 3, 1, 9, 5, "old"),
                (vec!["🧑‍🚀", "a\0b", "🧑‍🚀"], 3, 7, 83, 13, "a\0b"),
            ] {
                let args = arguments(
                    &artifact, &memory, &keys, steps, seed, old, marker, query, true, 1,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut outputs, mode.execution(), &memory)
                };
                result.unwrap();
                assert!(
                    counts.helpers.keys().any(|(site, _)| packs.contains(site)),
                    "selected tagged packs must execute on current input: {counts:?}"
                );
                assert_eq!(
                    drafts(&outputs).last(),
                    Some(&expected(&keys, steps, seed, old, marker, query))
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(value, alias)| value.shares_frozen_storage(alias))
                );
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn tagged_map_carrier_preserves_ordered_errors_publication_and_same_owner_retry() {
    let mut errors = [None, None, None, None, None];
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        for jit in [&native] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for (steps, seed, allowed, divisor, query, rejection) in [
                    (1, 5, true, 1, "old", None),
                    (2, 5, false, 0, "missing", Some((0, "Integrity"))),
                    (2, 5, true, 0, "missing", Some((1, "Arithmetic"))),
                    (2, 5, true, 1, "old", Some((2, "Index"))),
                    (1, u64::MAX, true, 1, "old", Some((3, "Arithmetic"))),
                    (0, 13, true, 1, "missing", Some((4, "Library"))),
                    (0, 13, false, 0, "old", None),
                    (1, 17, true, 1, "a", None),
                ] {
                    let args = arguments(
                        &artifact,
                        &memory,
                        &["a"],
                        steps,
                        seed,
                        3,
                        2,
                        query,
                        allowed,
                        divisor,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = outputs.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _) = if restored {
                        observe::run_restored(jit, &args, &mut outputs, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut outputs, mode.execution(), &memory)
                    };
                    if let Some((slot, name)) = rejection {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains(name), "{error}");
                        assert_eq!(
                            error,
                            *errors[slot].get_or_insert(error.clone()),
                            "original/decoded/restored forwarding must preserve the exact checked error and origin"
                        );
                        assert_prior(&outputs, &prior);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        assert_eq!(
                            drafts(&outputs).last(),
                            Some(&expected(&["a"], steps, seed, 3, 2, query))
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn tagged_map_carrier_finite_funding_is_atomic_and_recoverable() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(
            artifact,
            memory,
            &["a", "é", "a"],
            3,
            5,
            7,
            11,
            "a",
            true,
            1,
        )
    });
}

#[test]
fn escaping_tagged_map_record_keeps_canonical_output_and_original_aliases() {
    let source = SOURCE.replace("consume(result, marker<u64>, query<string>)", "result");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let frames = jit.descriptor().plan().facts().buffer_frames();
        let output = jit
            .descriptor()
            .plan()
            .transport_output_value(jit.descriptor().plan().transport_output_count() - 1);
        assert!(
            frames.slot(output).is_none(),
            "the actual public record must keep canonical materialization"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(
                &artifact,
                &memory,
                &["a", "é", "a"],
                3,
                5,
                7,
                11,
                "a",
                true,
                1,
            );
            let aliases = args.clone();
            let before = contents(&args);
            let mut outputs = vec![None; artifact.outputs().len()];
            let result = if restored {
                observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory).0
            } else {
                observe::run(&jit, &args, &mut outputs, mode.execution(), &memory).0
            };
            result.unwrap();
            let Data::Record(fields) = drafts(&outputs).pop().unwrap() else {
                panic!("canonical record")
            };
            let fields = fields
                .iter()
                .map(|field| (field.name.as_str(), &field.value))
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(fields["count"], &Data::U64(7));
            let Data::Map(entries) = fields["index"] else {
                panic!("canonical map")
            };
            let entries = entries
                .iter()
                .map(|entry| match entry.items.as_ref() {
                    [Data::String(key), Data::U64(value)] => (key.as_str(), *value),
                    _ => panic!("string/scalar entries"),
                })
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(
                entries,
                [("a", 5), ("old", 7), ("é", 6)].into_iter().collect()
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            drop((outputs, aliases, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn tagged_map_carrier_library_output_and_temporary_boundaries_preserve_old_roots() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        selected_sites(&jit);
        let args = arguments(
            &artifact,
            &memory,
            &["a", "é", "a"],
            3,
            5,
            7,
            11,
            "a",
            true,
            1,
        );
        let before = contents(&args);
        let mut published = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut published, mode.execution(), &memory)
            .unwrap();
        let expected = drafts(&published);
        let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            for dimension in [
                "output-bytes",
                "output-elements",
                "temporary",
                "retained-nodes",
            ] {
                let trial = |limit| {
                    let mut policy = mode.execution();
                    match dimension {
                        "output-bytes" => policy.library.max_output_bytes = Some(limit),
                        "output-elements" => policy.library.max_output_elements = Some(limit),
                        "temporary" => policy.library.max_temporary_bytes = Some(limit),
                        "retained-nodes" => policy.library.max_retained_nodes = Some(limit),
                        _ => unreachable!(),
                    }
                    let mut outputs = published.clone();
                    let result = if restored {
                        observe::run_restored(&jit, &args, &mut outputs, policy, &memory).0
                    } else {
                        observe::run(&jit, &args, &mut outputs, policy, &memory).0
                    };
                    if result.is_ok() {
                        assert_eq!(drafts(&outputs), expected);
                    } else {
                        assert_prior(&outputs, &published);
                    }
                    drop(outputs);
                    assert_eq!(contents(&args), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                    result.is_ok()
                };
                let (mut low, mut high) = (0, 1_000_000_000u64);
                assert!(trial(high), "{dimension}");
                while low < high {
                    let middle = low + (high - low) / 2;
                    if trial(middle) {
                        high = middle;
                    } else {
                        low = middle + 1;
                    }
                }
                assert!(low > 0 && trial(low) && !trial(low - 1), "{dimension}");
                assert!(
                    trial(1_000_000_000),
                    "same admitted roots must recover after a tighter call"
                );
            }
        }
        drop((published, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn tagged_map_objects_require_the_current_typed_layout_identity() {
    use crate::native_artifact::{NATIVE_PROGRAM_TYPED_IR_VERSION, NativeProgramObjectIdentity};
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        selected_sites(&jit);
        let identity = NativeProgramObjectIdentity::from_descriptor(jit.descriptor()).unwrap();
        assert!(NATIVE_PROGRAM_TYPED_IR_VERSION > 148);
        let mut old = *identity.bytes();
        for version in [147u64, 148] {
            old[16..24].copy_from_slice(&version.to_le_bytes());
            assert!(
                crate::native_artifact::program::linked::prepare_descriptor(
                    &artifact, &catalog, &old
                )
                .is_err(),
                "old typed layout IR{version} must not restore against current ownership arbitration"
            );
        }
        old[32..40].copy_from_slice(&101u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
        let mut old_work = *identity.bytes();
        old_work[32..40].copy_from_slice(&101u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(
                &artifact, &catalog, &old_work
            )
            .is_err()
        );
    }
}

#[test]
fn direct_map_and_scalar_variant_payloads_use_their_own_current_lanes() {
    let scalar = r#"+> map
#Carry(initial<{string:u64}>, steps<u64>, seed<u64>) => <u64>
  | :Visit(left<u64>, index<{string:u64}>, total<u64>)
  | :Done(value<u64>).
#Carry(initial, steps, seed) -> :Visit(steps, initial, seed)
  :Visit(left, index, total)
    | left > 0u64 -> :Visit(left - 1u64, index, total + 1u64)
    | * -> :Done(index{"old"} + total)
  :Done(value) => value.
#Carry(initial<{string:u64}>, steps<u64>, seed<u64>)
"#;
    let managed = r#"+> map
#Carry(initial<{string:u64}>, steps<u64>) => <u64>
  | :Visit(left<u64>, index<{string:u64}>)
  | :Done(index<{string:u64}>).
#Carry(initial, steps) -> :Visit(steps, initial)
  :Visit(left, index)
    | left > 0u64 -> :Visit(left - 1u64, index)
    | * -> :Done(index)
  :Done(index) => index{"old"}.
#Carry(initial<{string:u64}>, steps<u64>)
"#;
    for (source, direct_scalar) in [(scalar, true), (managed, false)] {
        for artifact in artifacts(source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let plan = jit.descriptor().plan();
            let layout = plan.facts().buffer_frames();
            assert!(
                plan.blocks().iter().enumerate().any(|(site, _)| {
                    let Some(action) = layout.action(site) else {
                        return false;
                    };
                    let Frame::EnumPack { output, .. } = action.operation() else {
                        return false;
                    };
                    let Some([root]) = layout.fields(*output) else {
                        return false;
                    };
                    layout.children(*root).unwrap().iter().any(|field| {
                        if direct_scalar {
                            matches!(field.lane(), Lane::Scalar { .. })
                        } else {
                            matches!(field.lane(), Lane::Managed { .. })
                        }
                    })
                }),
                "the single-field variant must not be hidden in a tuple-only implementation"
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for (steps, old, seed) in [(0, 7, 11), (1, 31, 13), (5, 19, 23)] {
                    let args =
                        arguments(&artifact, &memory, &[], steps, seed, old, 0, "old", true, 1);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let result = if restored {
                        observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory)
                            .0
                    } else {
                        observe::run(&jit, &args, &mut outputs, mode.execution(), &memory).0
                    };
                    result.unwrap();
                    assert_eq!(
                        drafts(&outputs).last(),
                        Some(&Data::U64(if direct_scalar {
                            old + seed + steps
                        } else {
                            old
                        }))
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn retained_map_owner_and_private_tagged_frame_components_coexist_without_aliasing() {
    let owner = r#"
update-owned(index<{string:u64}>) => (out<{string:u64}>) :=
  one := map/insert(index, "one", 11u64)
  out := map/insert(one, "two", 22u64).
#Owned(initial<{string:u64}>) => <{string:u64}>
  | :Run(index<{string:u64}>)
  | :Done(index<{string:u64}>).
#Owned(initial) -> :Run(initial)
  :Run(index)
    | * -> :Done(update-owned(index))
  :Done(index) => index.
"#;
    let source = SOURCE
        .replace("result := #Collect", &format!("{owner}\nresult := #Collect"))
        .replace(
            "consume(result, marker<u64>, query<string>)",
            "owned := #Owned(initial)\nconsume(result, marker<u64>, query<string>) + owned{\"two\"}",
        );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let plan = jit.descriptor().plan();
        let facts = plan.facts();
        let owners = facts.map_owner_words().unwrap();
        assert!(
            !owners.regions().is_empty(),
            "the established retained-map specialization must remain selected"
        );
        let frames = facts.buffer_frames();
        let ordinary = facts.scalar_aggregates();
        // Map-owner words also include every ordinary scalar aggregate. Those
        // preexisting facts are not a retained-map storage claim. Only the
        // region-private extension must arbitrate with the new tagged frames.
        let retained_values = owners
            .regions()
            .iter()
            .flat_map(|region| region.values())
            .filter(|value| {
                owners.aggregates().slot(**value).is_some() && ordinary.slot(**value).is_none()
            })
            .map(|value| value.get() as usize)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(!retained_values.is_empty());
        for value in retained_values {
            assert!(
                frames.slot_at(value).is_none(),
                "one compound value cannot have two private storage owners: value={value}"
            );
        }
        selected_sites(&jit);
        let catalog = mech_stdlib::source_catalog();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        for tamper in ["lane", "region"] {
            let mut forged: serde_json::Value = serde_json::from_slice(&image).unwrap();
            let proof = &mut forged["descriptor"]["map_owner_words"];
            if tamper == "lane" {
                let slot = proof["aggregates"]["slots"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|slot| slot.is_array())
                    .unwrap();
                slot[0] = (slot[0].as_u64().unwrap() + 1).into();
            } else {
                // This is a real canonical Map input in the same plan, not an
                // invalid/out-of-range ID. It must not become private merely
                // because an image claims it belongs to the retained region.
                let input = plan
                    .inputs()
                    .iter()
                    .find(|input| {
                        matches!(
                            input.ty().definition().body(),
                            mech_core::SchemaBody::Map { .. }
                        )
                    })
                    .unwrap()
                    .value()
                    .get();
                assert_ne!(
                    proof["regions"][0]["values"][0].as_u64().unwrap(),
                    u64::from(input)
                );
                proof["regions"][0]["values"][0] = u64::from(input).into();
            }
            let bytes = serde_json::to_vec(&forged).unwrap();
            // SAFETY: malformed image diagnostics only. No decoded descriptor
            // is attached to generated code or executed, even if accepted.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                    .is_err(),
                "forged retained-map {tamper} proof must be independently rederived and rejected"
            );
        }
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for (keys, steps, seed, old, marker, query) in [
                (vec!["a", "a", "é"], 3, 5, 7, 11, "a"),
                (vec![], 0, 13, 19, 2, "old"),
                (vec!["old", "b"], 2, 1, 41, 7, "b"),
            ] {
                let args = arguments(
                    &artifact, &memory, &keys, steps, seed, old, marker, query, true, 1,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut outputs, mode.execution(), &memory).0
                };
                result.unwrap();
                let Data::U64(value) = expected(&keys, steps, seed, old, marker, query) else {
                    unreachable!()
                };
                assert_eq!(drafts(&outputs).last(), Some(&Data::U64(value + 22)));
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(value, alias)| value.shares_frozen_storage(alias))
                );
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

mod current_banks {
    use super::*;
    use crate::native_artifact::program::protocol::abi::{Frame as AbiFrame, Helper};
    use mech_engine::native_artifact::NativeBufferFrameField;
    use std::{cell::Cell, ffi::c_void};

    thread_local! {
        static HELPER: Cell<Option<Helper>> = const { Cell::new(None) };
        static OBSERVED: Cell<(usize, bool)> = const { Cell::new((0, true)) };
        static GUARDED: Cell<(usize, bool)> = const { Cell::new((0, true)) };
    }

    fn empty(
        layout: &mech_engine::native_artifact::NativeBufferFrames,
        frames: &super::super::super::buffer_frames::BufferFrames,
        field: NativeBufferFrameField,
    ) -> bool {
        match field.lane() {
            Lane::Composite { .. } => layout
                .children(field)
                .unwrap()
                .iter()
                .all(|child| empty(layout, frames, *child)),
            Lane::Tagged { .. } => {
                frames.current_tag(field).is_err()
                    && layout
                        .children(field)
                        .unwrap()
                        .iter()
                        .all(|child| empty(layout, frames, *child))
            }
            _ => frames.current_leaf(field).is_none(),
        }
    }

    fn leaf_values(
        layout: &mech_engine::native_artifact::NativeBufferFrames,
        frames: &super::super::super::buffer_frames::BufferFrames,
        fields: &[NativeBufferFrameField],
    ) -> Vec<Option<(Option<Data>, u64)>> {
        let mut values = Vec::new();
        for field in fields {
            if let Some(children) = layout.children(*field) {
                values.extend(leaf_values(layout, frames, children));
            } else {
                values.push(frames.current_leaf(*field).map(|(owner, bits)| {
                    (
                        owner.map(|value| value.canonical_data_draft().unwrap()),
                        bits,
                    )
                }));
            }
        }
        values
    }

    unsafe extern "C" fn helper(
        context: *mut c_void,
        site: u64,
        op: u64,
        a: u64,
        b: u64,
        c: u64,
        output: *mut u64,
    ) -> u64 {
        // SAFETY: retain this same synchronous invocation, JIT and callback.
        // Delegate the actual entry unchanged; the post-call diagnostic makes
        // one paid, rejected wrong-variant observation on the same work meter.
        let status = unsafe { HELPER.with(Cell::get).unwrap()(context, site, op, a, b, c, output) };
        if status == 0 && op == GlobalHelper::Library.raw() {
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            let layout = &invocation.descriptor.metadata().buffer_frames;
            if let Some(action) = layout.action(site as usize)
                && let Frame::EnumPack {
                    output,
                    payload,
                    ordinal,
                } = action.operation()
            {
                let mut valid = invocation
                    .state
                    .arena
                    .test_current_value(output.get())
                    .is_none();
                if let Some([root]) = layout.fields(*output)
                    && let Lane::Tagged { count, .. } = root.lane()
                {
                    valid &= invocation.state.buffer_frames.current_tag(*root).ok()
                        == Some(*ordinal as usize);
                    for inactive in 0..count {
                        if inactive != *ordinal as usize {
                            valid &= layout.tagged_variant(*root, inactive as u32).is_some_and(
                                |field| empty(layout, &invocation.state.buffer_frames, field),
                            );
                        }
                    }
                } else {
                    valid = false;
                }
                let fields = layout.fields(*payload).unwrap();
                let before = leaf_values(layout, &invocation.state.buffer_frames, fields);
                let spent = invocation.state.control.work.used();
                let denied = invocation
                    .state
                    .buffer_frames
                    .observe(
                        layout,
                        *output,
                        &[
                            mech_engine::native_artifact::NativeStructuralStep::EnumPayload {
                                ordinal: *ordinal ^ 1,
                            },
                        ],
                        mech_engine::native_artifact::NativeStructuralObservation::CanonicalValue,
                        *payload,
                        &mut invocation.state.control.work,
                    )
                    .is_err();
                let intact = leaf_values(layout, &invocation.state.buffer_frames, fields) == before;
                let paid = invocation.state.control.work.used() > spent;
                GUARDED.with(|state| {
                    let (count, prior) = state.get();
                    state.set((count + 1, prior && denied && intact && paid));
                });
                OBSERVED.with(|state| {
                    let (count, prior) = state.get();
                    state.set((count + 1, prior && valid));
                });
            }
        }
        status
    }

    #[test]
    fn each_current_tagged_pack_has_no_canonical_enum_or_live_inactive_bank() {
        for artifact in artifacts(SOURCE) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            selected_sites(&jit);
            let restored = super::super::super::tests::restore_admitted_helper_descriptor(&jit);
            for descriptor in [jit.descriptor(), &restored] {
                for steps in [0, 1, 4] {
                    let memory = mode.memory_budget();
                    let keys = ["a", "é", "a", "old"];
                    let args =
                        arguments(&artifact, &memory, &keys, steps, 7, 19, 3, "old", true, 1);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let mut outputs = vec![None; artifact.outputs().len()];
                    OBSERVED.with(|state| state.set((0, true)));
                    GUARDED.with(|state| state.set((0, true)));
                    let entry = jit.test_checked_entry();
                    execute(
                        descriptor,
                        |raw: *mut AbiFrame| {
                            // SAFETY: change only synchronous callback wiring and
                            // restore it before this exact checked entry returns.
                            unsafe {
                                let previous = (*raw).helper;
                                HELPER.with(|slot| slot.set(Some(previous)));
                                (*raw).helper = helper;
                                let status = entry(raw);
                                (*raw).helper = previous;
                                HELPER.with(|slot| slot.set(None));
                                status
                            }
                        },
                        &args,
                        &mut outputs,
                        mode.execution(),
                        &memory,
                    )
                    .unwrap();
                    let (packs, valid) = OBSERVED.with(Cell::get);
                    let (guards, intact) = GUARDED.with(Cell::get);
                    assert!(
                        packs >= steps as usize + 2 && valid,
                        "packs={packs}, current/disjoint banks={valid}"
                    );
                    assert!(
                        guards > 0 && intact,
                        "wrong-variant guard calls={guards}, paid atomic rejection={intact}"
                    );
                    assert_eq!(
                        drafts(&outputs).last(),
                        Some(&expected(&keys, steps, 7, 19, 3, "old"))
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    drop((outputs, args, aliases));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}
