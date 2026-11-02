//! Changing nested scalar words beside a private byte array keep value semantics.
//! These generic scans qualify composite transport; they contain no post/tag logic.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;

const SOURCE: &str = r#"+> sequence
<rank> := <((u8,u64),u64)>
<state> := <([u8],rank)>
put(xs<[u8]>, at<u64>, value<u8>) => (out<[u8]>) :=
  ~next := xs
  next[at] = value
  out := next.
lane(xs<[u8]>, at<u64>, best<rank>) => (out<state>) :=
  count := xs[at + 1u64]
  selected := ((count > best.1.1)? | true => (count, at) | false => best.1)
  next-best := (selected, best.2 + 1u64)
  zeroed := put(xs, at + 1u64, 0u8)
  out := (zeroed, next-best).
walk(xs<[u8]>, at<u64>, end<u64>, best<rank>) => (out<state>) :=
  more := at < end
  step := (more? | true => lane(xs, at, best) | false => (xs, best))
  (next, chosen) := step
  out := (more? | true => walk(next, at + 1u64, end, chosen) | false => (next, chosen)).
result := walk(bytes<[u8]>, 0u64, steps<u64>, ((minimum<u8>, winner<u64>), marker<u64>))
(result.1, result.2.1.1, result.2.1.2, result.2.2, bytes)
"#;

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    steps: u64,
    marker: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("bytes", Data::Matrix([7, 12, 12, 2].map(Data::U8).into())),
            ("steps", Data::U64(steps)),
            ("minimum", Data::U8(7)),
            ("winner", Data::U64(42)),
            ("marker", Data::U64(marker)),
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

fn last(outputs: &[Option<Value>]) -> Data {
    outputs
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}

fn expected(steps: u64, marker: u64, clear: bool) -> Data {
    let original = [7, 12, 12, 2];
    let mut bytes = original;
    let (mut best, mut winner) = (7, 42);
    for at in 0..steps as usize {
        if original[at] > best {
            best = original[at];
            winner = at as u64;
        }
        if clear {
            bytes[at] = 0;
        }
    }
    Data::Tuple(
        [
            Data::Matrix(bytes.map(Data::U8).into()),
            Data::U8(best),
            Data::U64(winner),
            Data::U64(marker + steps),
            Data::Matrix(original.map(Data::U8).into()),
        ]
        .into(),
    )
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

#[test]
fn nested_scalar_array_updates_select_typed_writes_and_preserve_current_words() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| {
                    region.numeric.as_ref().is_some_and(|proof| {
                        proof
                            .actions
                            .iter()
                            .any(|(_, action)| matches!(action, Operation::Write { .. }))
                    })
                }),
            "the changing nested scalar state must not exclude the byte-array write loop"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for (steps, marker) in [(0, 99), (1, 17), (4, 3), (2, 123)] {
                let args = arguments(&artifact, &memory, steps, marker);
                let aliases = args.clone();
                let before = contents(&args);
                let result = if restored {
                    observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &args, &mut outputs, mode.execution(), &memory).0
                };
                result.unwrap();
                assert_eq!(last(&outputs), expected(steps, marker, true));
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
fn nested_scalar_array_errors_keep_published_roots_and_recover_in_restored_code() {
    let mut errors = [None, None];
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        for jit in [&ordinary, &native] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for (steps, marker, rejection) in [
                    (1, 5, None),
                    (5, 5, Some((0, "Index"))),
                    (1, u64::MAX, Some((1, "Arithmetic"))),
                    (4, 31, None),
                ] {
                    let args = arguments(&artifact, &memory, steps, marker);
                    let before = contents(&args);
                    let aliases = args.clone();
                    let prior = outputs.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(jit, &args, &mut outputs, mode.execution(), &memory).0
                    } else {
                        observe::run(jit, &args, &mut outputs, mode.execution(), &memory).0
                    };
                    if let Some((index, name)) = rejection {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains(name), "{error}");
                        assert_eq!(
                            error,
                            *errors[index].get_or_insert(error.clone()),
                            "native retry must preserve the ordinary source error and origin"
                        );
                        assert_prior(&outputs, &prior);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        assert_eq!(last(&outputs), expected(steps, marker, true));
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
fn nested_scalar_array_one_short_finite_work_is_atomic_and_retryable() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, 3, 17);
            let before = contents(&args);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, mode.execution(), &memory)
                .unwrap();
            let wanted = drafts(&outputs);
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for dimension in ["control", "library", "output-bytes", "output-elements"] {
                let mut trial = |allowance| {
                    let mut policy = mode.execution();
                    match dimension {
                        "control" => policy.execution.max_work = allowance,
                        "library" => policy.library.max_compute_work = Some(allowance),
                        "output-bytes" => policy.library.max_output_bytes = Some(allowance),
                        "output-elements" => policy.library.max_output_elements = Some(allowance),
                        _ => unreachable!(),
                    }
                    let prior = outputs.clone();
                    let result = if restored {
                        observe::run_restored(&jit, &args, &mut outputs, policy, &memory).0
                    } else {
                        observe::run(&jit, &args, &mut outputs, policy, &memory).0
                    };
                    if result.is_err() {
                        assert_prior(&outputs, &prior);
                    }
                    assert_eq!(drafts(&outputs), wanted);
                    drop(prior);
                    assert_eq!(contents(&args), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    result.is_ok()
                };
                let (mut low, mut high) = (0, 1_000_000_000_000u64);
                assert!(trial(high));
                while low < high {
                    let middle = low + (high - low) / 2;
                    if trial(middle) {
                        high = middle;
                    } else {
                        low = middle + 1;
                    }
                }
                assert!(trial(low));
                assert!(low > 0 && !trial(low - 1));
                assert!(trial(1_000_000_000_000));
            }
            drop((args, outputs));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn discarded_byte_array_root_does_not_gain_an_in_place_write_proof() {
    let source = SOURCE.replace("out := (zeroed, next-best).", "out := (xs, next-best).");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .all(|region| region.numeric.as_ref().is_none_or(|proof| !proof.writes())),
            "a discarded update must not grant mutation of the forwarded original root"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, 4, 31);
            let before = contents(&args);
            let mut outputs = vec![None; artifact.outputs().len()];
            let result = if restored {
                observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory).0
            } else {
                observe::run(&jit, &args, &mut outputs, mode.execution(), &memory).0
            };
            result.unwrap();
            assert_eq!(last(&outputs), expected(4, 31, false));
            assert_eq!(contents(&args), before);
            drop((args, outputs));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn escaping_nested_scalar_array_is_materialized_with_the_original_alias_intact() {
    let source = SOURCE.replace(
        "(result.1, result.2.1.1, result.2.1.2, result.2.2, bytes)",
        "result",
    );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, 4, 31);
            let aliases = args.clone();
            let before = contents(&args);
            let mut output = vec![None; artifact.outputs().len()];
            let result = if restored {
                observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory).0
            } else {
                observe::run(&jit, &args, &mut output, mode.execution(), &memory).0
            };
            result.unwrap();
            assert_eq!(
                last(&output),
                Data::Tuple(
                    [
                        Data::Matrix([0, 0, 0, 0].map(Data::U8).into()),
                        Data::Tuple(
                            [
                                Data::Tuple([Data::U8(12), Data::U64(1)].into()),
                                Data::U64(35)
                            ]
                            .into()
                        ),
                    ]
                    .into()
                )
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            drop((args, aliases, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn nested_scalar_array_old_object_identity_and_missing_recursive_words_are_rejected() {
    use crate::native_artifact::{
        NATIVE_PROGRAM_TYPED_IR_VERSION, NativeProgramDescriptor, NativeProgramObjectIdentity,
    };
    fn remove_paths(value: &mut serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(fields) => {
                let removed = usize::from(fields.remove("scalar_paths").is_some());
                removed + fields.values_mut().map(remove_paths).sum::<usize>()
            }
            serde_json::Value::Array(values) => values.iter_mut().map(remove_paths).sum(),
            _ => 0,
        }
    }
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let identity = NativeProgramObjectIdentity::from_descriptor(jit.descriptor()).unwrap();
        assert_eq!(
            u64::from_le_bytes(identity.bytes()[16..24].try_into().unwrap()),
            NATIVE_PROGRAM_TYPED_IR_VERSION
        );
        assert!(NATIVE_PROGRAM_TYPED_IR_VERSION > 145);
        let mut old = *identity.bytes();
        old[16..24].copy_from_slice(&145u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        let mut image: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            remove_paths(&mut image) > 0,
            "fixture must contain recursive scalar frame words"
        );
        let old_layout = serde_json::to_vec(&image).unwrap();
        // SAFETY: malformed build-image diagnostic only; any result is confined
        // and is never associated with an entry or executed.
        assert!(
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&old_layout, &catalog) }
                .is_err()
        );
    }
}

mod stale_composite {
    use super::*;
    use crate::native_artifact::program::protocol::abi::{Frame, Helper};
    use mech_engine::native_artifact::{NativeBufferFrameField, NativeBufferFrameLane as Lane};
    use std::{
        cell::{Cell, RefCell},
        ffi::c_void,
    };

    #[derive(Clone, Copy)]
    struct Probe {
        entry: u64,
        visit: u64,
        value: u32,
        leaf: NativeBufferFrameField,
    }
    thread_local! {
        static HELPER: Cell<Option<Helper>> = const { Cell::new(None) };
        static VISITS: Cell<u64> = const { Cell::new(0) };
        static CANDIDATES: RefCell<Vec<Probe>> = const { RefCell::new(Vec::new()) };
        static FRESH: Cell<Option<Probe>> = const { Cell::new(None) };
        static INJECT: Cell<Option<Probe>> = const { Cell::new(None) };
        static OBSERVED: Cell<Option<(bool, u64)>> = const { Cell::new(None) };
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
        let numeric = op == GlobalHelper::NumericSegmentBegin.raw();
        let visit = if numeric {
            VISITS.with(|count| {
                let next = count.get() + 1;
                count.set(next);
                next
            })
        } else {
            0
        };
        let mut fresh = None;
        let mut injected = false;
        if numeric {
            // SAFETY: execute owns this unchanged synchronous invocation. The
            // diagnostic introduces one valid scalar word, never an owner,
            // argument, budget, array root, descriptor, or code mutation.
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            let layout = &invocation.descriptor.metadata().buffer_frames;
            if let Some(target) = INJECT.with(Cell::get)
                && target.entry == site
                && target.visit == visit
                && OBSERVED.with(Cell::get).is_none()
            {
                let direct_empty = layout
                    .fields_at(target.value as usize)
                    .is_some_and(|fields| {
                        fields.iter().all(|field| {
                            matches!(field.lane(), Lane::Composite { .. })
                                && invocation
                                    .state
                                    .buffer_frames
                                    .current_leaf(*field)
                                    .is_none()
                        })
                    });
                let leaf_empty = invocation
                    .state
                    .buffer_frames
                    .current_leaf(target.leaf)
                    .is_none();
                injected = direct_empty
                    && leaf_empty
                    && invocation
                        .state
                        .buffer_frames
                        .synchronize_scalar_leaf(target.leaf, 1)
                        .is_ok();
            } else if INJECT.with(Cell::get).is_none() && FRESH.with(Cell::get).is_none() {
                fresh = CANDIDATES.with(|candidates| {
                    candidates.borrow().iter().copied().find(|probe| {
                        probe.entry == site
                            && invocation
                                .state
                                .buffer_frames
                                .current_leaf(probe.leaf)
                                .is_none()
                    })
                });
            }
        }
        // SAFETY: every callback argument is delegated unchanged to the real
        // helper. Its live output word is read only after successful return.
        let status = unsafe { HELPER.with(Cell::get).unwrap()(context, site, op, a, b, c, output) };
        if numeric && status == 0 {
            let selected = unsafe { *output };
            if let Some(mut probe) = fresh
                && selected == 1
            {
                probe.visit = visit;
                FRESH.with(|slot| slot.set(Some(probe)));
            }
            if INJECT
                .with(Cell::get)
                .is_some_and(|probe| probe.entry == site && probe.visit == visit)
                && OBSERVED.with(Cell::get).is_none()
            {
                OBSERVED.with(|slot| slot.set(Some((injected, selected))));
            }
        }
        status
    }

    fn run(
        jit: &JitProgramArtifact,
        descriptor: &NativeProgramDescriptor,
        args: &[Value],
        outputs: &mut [Option<Value>],
        memory: &ManagedMemoryBudget,
    ) {
        let entry = jit.test_checked_entry();
        execute(
            descriptor,
            |raw: *mut Frame| {
                // SAFETY: retain this exact JIT owner and initialized frame. Only
                // synchronous helper wiring changes, and it is restored on return.
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
            args,
            outputs,
            NativeProgramLimitMode::NoLimits.execution(),
            memory,
        )
        .unwrap();
    }

    #[test]
    fn stale_scalar_grandchild_declines_a_previously_selected_parent_piece() {
        let definitions = SOURCE
            .split("result := walk")
            .next()
            .unwrap()
            .replace(
                "<rank> := <((u8,u64),u64)>",
                "<rank> := <((u8,u64),(u64,u64))>",
            )
            .replace(
                "next-best := (selected, best.2 + 1u64)",
                "next-best := (selected, (best.2.1 + 1u64, best.2.2))",
            );
        let source = format!(
            "{definitions}\nrepeat(xs<[u8]>, left<u64>, best<rank>) => (out<state>) :=\n  more := left > 0u64\n  step := (more? | true => walk(xs, 0u64, 2u64, best) | false => (xs, best))\n  (next, chosen) := step\n  out := (more? | true => repeat(next, left - 1u64, chosen) | false => (next, chosen)).\nresult := repeat(bytes<[u8]>, 2u64, ((minimum<u8>, winner<u64>), (marker<u64>, 91u64)))\n(result.1, result.2.1.1, result.2.1.2, result.2.2.1, bytes)\n"
        );
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let metadata = jit.descriptor().metadata();
            let candidates = metadata
                .compositions
                .parents
                .iter()
                .flat_map(|parent| &parent.segments)
                .filter(|segment| {
                    metadata
                        .compositions
                        .entry(segment.entry as usize)
                        .is_some()
                })
                .flat_map(|segment| {
                    let frames = segment.frame_plan.as_ref().unwrap();
                    frames
                        .frames
                        .iter()
                        .filter(|frame| !frames.inputs.contains(&frame.value))
                        .filter_map(|frame| {
                            let fields = metadata.buffer_frames.fields_at(frame.value as usize)?;
                            if fields.is_empty()
                                || !fields
                                    .iter()
                                    .all(|field| matches!(field.lane(), Lane::Composite { .. }))
                            {
                                return None;
                            }
                            let (path, _) =
                                frame.scalar_paths.iter().find(|(path, _)| path.len() > 1)?;
                            let mut current = fields;
                            let mut selected = None;
                            for (depth, ordinal) in path.iter().enumerate() {
                                let field = *current.get(*ordinal)?;
                                selected = Some(field);
                                if depth + 1 < path.len() {
                                    current = metadata.buffer_frames.children(field)?;
                                }
                            }
                            let leaf = selected?;
                            Some(Probe {
                                entry: u64::from(segment.entry),
                                visit: 0,
                                value: frame.value,
                                leaf,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            assert!(
                !candidates.is_empty(),
                "fixture needs a noninput scalar-only all-Composite frame"
            );
            for restored in [false, true] {
                CANDIDATES.with(|slot| *slot.borrow_mut() = candidates.clone());
                FRESH.with(|slot| slot.set(None));
                VISITS.with(|slot| slot.set(0));
                INJECT.with(|slot| slot.set(None));
                OBSERVED.with(|slot| slot.set(None));
                let descriptor = if restored {
                    Some(super::super::super::tests::restore_admitted_helper_descriptor(&jit))
                } else {
                    None
                };
                let descriptor = descriptor.as_ref().unwrap_or(jit.descriptor());
                let memory = mode.memory_budget();
                let args = arguments(&artifact, &memory, 0, 31);
                let before = contents(&args);
                let aliases = args.clone();
                let mut outputs = vec![None; artifact.outputs().len()];
                run(&jit, descriptor, &args, &mut outputs, &memory);
                let fresh = FRESH
                    .with(Cell::get)
                    .expect("fresh parent piece must actually select");
                assert_eq!(last(&outputs), expected(2, 33, true));
                INJECT.with(|slot| slot.set(Some(fresh)));
                VISITS.with(|slot| slot.set(0));
                run(&jit, descriptor, &args, &mut outputs, &memory);
                assert_eq!(
                    OBSERVED.with(Cell::get),
                    Some((true, 0)),
                    "one stale scalar grandchild must decline while all direct Composite fields look empty"
                );
                assert_eq!(last(&outputs), expected(2, 33, true));
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                INJECT.with(|slot| slot.set(None));
                CANDIDATES.with(|slot| slot.borrow_mut().clear());
                drop((args, aliases, outputs));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
