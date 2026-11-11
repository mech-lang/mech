//! Retained Map state publishes a complete canonical flat record/tuple result.
use super::*;
use crate::native_artifact::program::protocol::abi::{Frame, Helper};
use mech_core::snapshot::MapEntryDraft;
use std::{cell::Cell, collections::BTreeMap, ffi::c_void};

const ORIGINAL: &str =
    include_str!("../../../../../tests/fixtures/native/retained-compound-map.mec");

#[derive(Clone, Copy, Debug)]
enum Shape {
    Record,
    MapFirst,
    MapLast,
}

fn source(shape: Shape, publish: bool) -> String {
    let mut source = ORIGINAL.to_owned();
    let (definition, terminal, consume) = match shape {
        Shape::Record => (
            "<{index<{string:u64}> count<u64>}>",
            "{index: index, count: count}",
            "words.index{key} + words.count + marker",
        ),
        Shape::MapFirst => (
            "<({string:u64},u64)>",
            "(index, count)",
            "words.1{key} + words.2 + marker",
        ),
        Shape::MapLast => (
            "<(u64,{string:u64})>",
            "(count, index)",
            "words.2{key} + words.1 + marker",
        ),
    };
    source = source
        .replace("<{index<{string:u64}> count<u64>}>", definition)
        .replace("{index: index, count: count}", terminal)
        .replace("words.index{key} + words.count + marker", consume);
    if publish {
        source = source.replace(
            "consume(result, marker<u64>, query<string>)",
            "(result, initial)",
        );
    }
    source
}

pub(super) fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    keys: &[&str],
    steps: u64,
    seed: u64,
    old: u64,
    allowed: bool,
    divisor: u64,
    query: &str,
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
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
            ("marker", Data::U64(2)),
            ("query", Data::String(query.into())),
        ],
        memory,
    )
}

fn drafts(output: &[Option<Value>]) -> Vec<Data> {
    output
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn prior(output: &[Option<Value>], old: &[Option<Value>]) {
    assert_eq!(drafts(output), drafts(old));
    assert!(output.iter().zip(old).all(|(a, b)| {
        a.as_ref()
            .unwrap()
            .shares_frozen_storage(b.as_ref().unwrap())
    }));
}

fn selected(jit: &JitProgramArtifact) {
    let plan = jit.descriptor().plan();
    let facts = plan.facts();
    let owners = facts.map_owner_words().unwrap();
    assert!(
        !owners.regions().is_empty(),
        "the original flat Map+U64 result must migrate to retained ownership"
    );
    let ordinary = facts.scalar_aggregates();
    let frames = facts.buffer_frames();
    for region in owners.regions() {
        assert!(
            !region.values().contains(&region.tail().result()),
            "canonical result must be outside the private owner region"
        );
        for value in region.values() {
            if owners.aggregates().slot(*value).is_some() && ordinary.slot(*value).is_none() {
                assert!(
                    frames.slot(*value).is_none(),
                    "retained owner and generic frame overlap: value={value:?}"
                );
            }
        }
    }
}

thread_local! {
    static ORIGINAL_HELPER:Cell<Option<Helper>>=const{Cell::new(None)};
    static OWNER_COUNTS:Cell<(u64,u64)>=const{Cell::new((0,0))};
}
unsafe extern "C" fn helper(
    context: *mut c_void,
    site: u64,
    op: u64,
    a: u64,
    b: u64,
    c: u64,
    out: *mut u64,
) -> u64 {
    // SAFETY: the checked synchronous entry retains the original helper and its
    // context. All arguments are forwarded unchanged, with no intervening loan.
    let status =
        unsafe { ORIGINAL_HELPER.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
    if status == 0 {
        if matches!(
            GlobalHelper::decode(op),
            GlobalHelper::MapOwnerBegin | GlobalHelper::MapOwnerBeginBorrowedValues
        ) && unsafe { *out } != 0
        {
            OWNER_COUNTS.with(|v| {
                let (a, b) = v.get();
                v.set((a + 1, b))
            });
        }
        if op == GlobalHelper::MapOwnerEnd.raw() {
            OWNER_COUNTS.with(|v| {
                let (a, b) = v.get();
                v.set((a, b + 1))
            });
        }
    }
    status
}

fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    output: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    (u64, u64),
) {
    let decoded = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = decoded.as_ref().unwrap_or_else(|| jit.descriptor());
    OWNER_COUNTS.with(|v| v.set((0, 0)));
    let entry = jit.test_checked_entry();
    let result = execute(
        descriptor,
        |raw: *mut Frame| {
            // SAFETY: execute owns the initialized frame and the same JIT entry for
            // this call; hook wiring is restored before returning its status.
            unsafe {
                let old = (*raw).helper;
                ORIGINAL_HELPER.with(|v| v.set(Some(old)));
                (*raw).helper = helper;
                let status = entry(raw);
                (*raw).helper = old;
                ORIGINAL_HELPER.with(|v| v.set(None));
                status
            }
        },
        args,
        output,
        policy,
        memory,
    );
    (result, OWNER_COUNTS.with(Cell::get))
}

fn check_published(data: &Data, shape: Shape, keys: &[&str], steps: u64, seed: u64, old: u64) {
    let Data::Tuple(items) = data else {
        panic!("complete result and seed must be canonical")
    };
    let (map, count) = match (&items[0], shape) {
        (Data::Record(fields), Shape::Record) => {
            assert_eq!(fields.len(), 2);
            let fields = fields
                .iter()
                .map(|f| (f.name.as_str(), &f.value))
                .collect::<BTreeMap<_, _>>();
            (fields["index"], fields["count"])
        }
        (Data::Tuple(fields), Shape::MapFirst) => (&fields[0], &fields[1]),
        (Data::Tuple(fields), Shape::MapLast) => (&fields[1], &fields[0]),
        _ => panic!("wrong complete canonical compound result"),
    };
    let mut expected = BTreeMap::from([("old", old)]);
    let mut expected_count = seed;
    for key in &keys[..steps as usize] {
        if !expected.contains_key(key) {
            expected.insert(key, expected_count);
            expected_count += 1;
        }
    }
    assert_eq!(count, &Data::U64(expected_count));
    let Data::Map(entries) = map else {
        panic!("compound Map field")
    };
    let actual = entries
        .iter()
        .map(|entry| match entry.items.as_ref() {
            [Data::String(k), Data::U64(v)] => (k.as_str(), *v),
            _ => panic!("native String/U64 Map"),
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected);
    assert_eq!(
        items[1],
        Data::Map(
            [MapEntryDraft {
                items: [Data::String("old".into()), Data::U64(old)].into()
            }]
            .into()
        )
    );
}

#[test]
fn retained_compound_map_records_and_both_tuple_ordinals_publish_current_canonical_values() {
    for shape in [Shape::Record, Shape::MapFirst, Shape::MapLast] {
        for artifact in artifacts(&source(shape, true)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            selected(&jit);
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (keys, steps, seed, old) in [
                    (vec!["a", "a", "é"], 3, 5, 41),
                    (vec![], 0, 19, 7),
                    (vec!["old", "old"], 2, 1, 83),
                    (vec!["🧑‍🚀", "a\0b", "🧑‍🚀"], 3, 11, 2),
                ] {
                    let args =
                        arguments(&artifact, &memory, &keys, steps, seed, old, true, 1, "old");
                    let aliases = args.clone();
                    let before = contents(&args);
                    let (result, (begins, ends)) = run(
                        &jit,
                        &args,
                        &mut output,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    result.unwrap();
                    assert!(
                        begins > 0 && ends > 0,
                        "{shape:?}: actual retained entry/publication must execute"
                    );
                    check_published(
                        drafts(&output).last().unwrap(),
                        shape,
                        &keys,
                        steps,
                        seed,
                        old,
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                }
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn retained_compound_map_errors_preserve_origins_prior_roots_and_retry() {
    for shape in [Shape::Record, Shape::MapFirst, Shape::MapLast] {
        let mut errors = [None, None, None, None, None];
        for artifact in artifacts(&source(shape, false)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            selected(&jit);
            // Error origins are compared between original/decoded/restored forms
            // of the same source shape; record and tuple plans can differ.
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (steps, seed, allowed, divisor, query, failure) in [
                    (1, 5, true, 1, "old", None),
                    (2, 5, false, 0, "old", Some((0, "Integrity"))),
                    (2, 5, true, 0, "old", Some((1, "Arithmetic"))),
                    (2, 5, true, 1, "old", Some((2, "Index"))),
                    (1, u64::MAX, true, 1, "old", Some((4, "Arithmetic"))),
                    (0, 5, true, 1, "missing", Some((3, "Library"))),
                    (0, 7, true, 0, "old", None),
                    (1, 9, true, 1, "a", None),
                ] {
                    let args = arguments(
                        &artifact,
                        &memory,
                        &["a"],
                        steps,
                        seed,
                        3,
                        allowed,
                        divisor,
                        query,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let old = output.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _) = run(
                        &jit,
                        &args,
                        &mut output,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    if let Some((index, name)) = failure {
                        let text = format!("{:?}", result.unwrap_err());
                        assert!(text.contains(name), "{text}");
                        assert_eq!(text, *errors[index].get_or_insert(text.clone()));
                        prior(&output, &old);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                    } else {
                        result.unwrap();
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn retained_compound_map_complete_output_and_temporary_caps_are_exact_and_retryable() {
    for shape in [Shape::Record, Shape::MapFirst, Shape::MapLast] {
        for artifact in artifacts(&source(shape, true)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            selected(&jit);
            for restored in [false, true] {
                // Fresh retained hash tables have randomized probe work. A
                // single repeated key gives deterministic exact work bounds;
                // distinct Unicode keys still exercise growth, strict byte/node
                // boundaries, and atomic low/ample work denial and recovery.
                let witnesses: [(bool, &[&str], &[&str]); 2] = [
                    (
                        true,
                        &["old", "old", "old"],
                        &["control", "library", "comparison"],
                    ),
                    (
                        false,
                        &["a", "é", "a"],
                        &[
                            "control",
                            "library",
                            "comparison",
                            "output-bytes",
                            "output-elements",
                            "temporary",
                            "cloned-bytes",
                            "retained-nodes",
                        ],
                    ),
                ];
                for (fixed_probe_work, keys, dimensions) in witnesses {
                    let memory = mode.memory_budget();
                    let args = arguments(&artifact, &memory, keys, 3, 5, 7, true, 1, "old");
                    let before = contents(&args);
                    let mut published = vec![None; artifact.outputs().len()];
                    run(
                        &jit,
                        &args,
                        &mut published,
                        mode.execution(),
                        &memory,
                        restored,
                    )
                    .0
                    .unwrap();
                    check_published(drafts(&published).last().unwrap(), shape, keys, 3, 5, 7);
                    let expected = drafts(&published);
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    for &dimension in dimensions {
                        let last_error = std::cell::RefCell::new(None::<String>);
                        let trial = |limit| {
                            let mut policy = mode.execution();
                            match dimension {
                                "control" => policy.execution.max_work = limit,
                                "library" => policy.library.max_compute_work = Some(limit),
                                "comparison" => policy.library.max_comparison_work = Some(limit),
                                "output-bytes" => policy.library.max_output_bytes = Some(limit),
                                "output-elements" => {
                                    policy.library.max_output_elements = Some(limit)
                                }
                                "temporary" => policy.library.max_temporary_bytes = Some(limit),
                                "cloned-bytes" => policy.library.max_cloned_bytes = Some(limit),
                                "retained-nodes" => policy.library.max_retained_nodes = Some(limit),
                                _ => unreachable!(),
                            };
                            let mut output = published.clone();
                            let result = run(&jit, &args, &mut output, policy, &memory, restored).0;
                            if result.is_ok() {
                                *last_error.borrow_mut() = None;
                                assert_eq!(drafts(&output), expected);
                            } else {
                                *last_error.borrow_mut() =
                                    Some(format!("{:?}", result.as_ref().unwrap_err()));
                                prior(&output, &published);
                            }
                            drop(output);
                            assert_eq!(contents(&args), before);
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                held
                            );
                            result.is_ok()
                        };
                        if !fixed_probe_work
                            && matches!(dimension, "control" | "library" | "comparison")
                        {
                            assert!(
                                !trial(0),
                                "{shape:?} multi-key {dimension} must deny a zero allowance atomically"
                            );
                            assert!(
                                trial(1_000_000_000_000),
                                "{shape:?} multi-key {dimension} must recover with ample work"
                            );
                            continue;
                        }
                        let (mut low, mut high) = (0, 1_000_000_000_000u64);
                        assert!(trial(high), "{dimension}");
                        while low < high {
                            let mid = low + (high - low) / 2;
                            if trial(mid) {
                                high = mid
                            } else {
                                low = mid + 1
                            }
                        }
                        let exact = trial(low);
                        println!(
                            "{}",
                            serde_json::json!({"witness":"retained-compound-map-finite-boundary","shape":format!("{shape:?}"),"restored":restored,"dimension":dimension,"fixed_probe_work":fixed_probe_work,"candidate_minimum":low,"exact":exact,"error":*last_error.borrow()})
                        );
                        assert!(
                            exact,
                            "{shape:?} restored={restored} {dimension} allowance={low}: {:?}",
                            last_error.borrow()
                        );
                        if low > 0 {
                            assert!(!trial(low - 1), "{shape:?} {dimension}");
                        }
                        assert!(trial(1_000_000_000_000));
                    }
                    drop((args, published));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}

#[test]
fn retained_compound_map_new_images_reject_the_previous_identity() {
    use crate::native_artifact::{
        NATIVE_PROGRAM_TYPED_IR_VERSION, NATIVE_PROGRAM_WORK_POLICY_VERSION,
        NativeProgramObjectIdentity,
    };
    for artifact in artifacts(&source(Shape::Record, true)) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        selected(&jit);
        let identity = NativeProgramObjectIdentity::from_descriptor(jit.descriptor()).unwrap();
        assert!(NATIVE_PROGRAM_TYPED_IR_VERSION > 149);
        assert!(NATIVE_PROGRAM_WORK_POLICY_VERSION > 102);
        let mut old = *identity.bytes();
        old[16..24].copy_from_slice(&149u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
        let mut old = *identity.bytes();
        old[32..40].copy_from_slice(&102u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
    }
}

#[test]
fn retained_compound_map_old_record_alias_survives_a_later_map_write() {
    const SOURCE: &str = r#"+> map
<dictionary> := <{index<{string:u64}> count<u64>}>
#Alias(initial<{string:u64}>, limit<u64>) => <dictionary>
  | :Next(index<{string:u64}>, count<u64>)
  | :Done(index<{string:u64}>, count<u64>).
#Alias(initial, limit) -> :Next(initial, 0u64)
  :Next(index, count)
    | count < limit -> {
        old := {index: index, count: count}
        next := map/insert(index, "needle", count)
        observed := old.index{"needle"}
      } -> :Done(next, observed)
    | * -> :Done(index, count)
  :Done(index, count) => {index: index, count: count}.
#Alias(initial<{string:u64}>, limit<u64>)
"#;
    for variant in ["late-projection", "early-projection", "record-copy"] {
        let source = match variant {
            "late-projection" => SOURCE.to_owned(),
            "early-projection" => SOURCE
                .replace(
                    "        next :=",
                    "        alias := old.index\n        next :=",
                )
                .replace(
                    "observed := old.index{\"needle\"}",
                    "observed := alias{\"needle\"}",
                ),
            "record-copy" => SOURCE
                .replace("        next :=", "        saved := old\n        next :=")
                .replace(
                    "observed := old.index{\"needle\"}",
                    "observed := saved.index{\"needle\"}",
                ),
            _ => unreachable!(),
        };
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let has_private_owner = !jit
                .descriptor()
                .plan()
                .facts()
                .map_owner_words()
                .unwrap()
                .regions()
                .is_empty();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (old, limit) in [(40, 1), (7, 0), (123, 4)] {
                    let original = Data::Map(
                        [MapEntryDraft {
                            items: [Data::String("needle".into()), Data::U64(old)].into(),
                        }]
                        .into(),
                    );
                    let args = inputs(
                        &artifact,
                        &[("initial", original), ("limit", Data::U64(limit))],
                        &memory,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let (result, (begins, _)) = run(
                        &jit,
                        &args,
                        &mut output,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    result.unwrap();
                    let Data::Record(fields) = drafts(&output).pop().unwrap() else {
                        panic!("canonical record")
                    };
                    let fields = fields
                        .iter()
                        .map(|field| (field.name.as_str(), &field.value))
                        .collect::<BTreeMap<_, _>>();
                    assert_eq!(
                        fields["count"],
                        &Data::U64(if limit > 0 { old } else { 0 }),
                        "{variant}: reading a pre-write record must see its old Map; private_owner={has_private_owner}"
                    );
                    assert_eq!(
                        fields["index"],
                        &Data::Map(
                            [MapEntryDraft {
                                items: [
                                    Data::String("needle".into()),
                                    Data::U64(if limit > 0 { 0 } else { old })
                                ]
                                .into()
                            }]
                            .into()
                        )
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    assert_eq!(
                        begins, 0,
                        "unsupported old aliases must not acquire a mutable retained owner"
                    );
                }
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
            assert!(
                !has_private_owner,
                "complete Record/RecordFieldAccess alias inventory must decline this candidate"
            );
        }
    }
}
