//! Prepared physical scalar bindings retain current values at every child boundary.
use super::*;
use mech_engine::native_artifact::NativeBufferFrameLane as Lane;

const SOURCE: &str = r#"+> sequence
<rank> := <((u64,u64),u64)>
<state> := <([u8],rank)>
binding-child(data<[u8]>, left<u64>) => (out<[u8]>) :=
  ~next := data
  next[1u64] = data[1u64] + 1u8
  out := ((left > 0u64)? | true => binding-child(next, left - 1u64) | false => next).
binding-parent(data<[u8]>, left<u64>, rank<rank>) => (out<state>) :=
  more := left > 0u64
  advanced := (more? | true => ((rank.1.1 + 1u64, rank.1.2 + 2u64), rank.2 + 3u64) | false => rank)
  state := (data, advanced)
  window := state.1
  next-rank := state.2
  size := sequence/length(window)
  child := (more? | true => binding-child(window, size) | false => data)
  out := (more? | true => binding-parent(child, left - 1u64, next-rank) | false => (data, rank)).
start := seed<u64>
result := binding-parent(bytes<[u8]>, rounds<u64>, ((start, start), start))
(result.1, result.2.1.1, result.2.1.2, result.2.2, bytes)
"#;

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    bytes: &[u8],
    rounds: u64,
    seed: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "bytes",
                Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
            ),
            ("rounds", Data::U64(rounds)),
            ("seed", Data::U64(seed)),
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

fn assert_prior(output: &[Option<Value>], prior: &[Option<Value>]) {
    assert_eq!(drafts(output), drafts(prior));
    assert!(output.iter().zip(prior).all(|(a, b)| {
        a.as_ref()
            .unwrap()
            .shares_frozen_storage(b.as_ref().unwrap())
    }));
}

#[test]
fn numeric_scalar_bindings_are_independently_derived_and_emitted_directly() {
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::program::compiler::emit_program_object_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            crate::native_artifact::codegen::options::OptimizationPass::NumericTrees,
        )
        .unwrap();
        let metadata = object.descriptor().metadata();
        let mut nested = 0;
        let mut inputs = 0;
        let mut outputs = 0;
        let mut fused = 0;
        for row in object.metadata()["compiled_numeric_segments"]
            .as_array()
            .unwrap()
        {
            assert_eq!(row["runtime_scalar_path_walks"], 0);
            let callbacks = row["entry_callbacks"].as_u64().unwrap();
            assert!(matches!(callbacks, 1 | 2));
            fused += usize::from(callbacks == 1);
            inputs += row["scalar_input_loads"].as_u64().unwrap();
            outputs += row["scalar_output_stores"].as_u64().unwrap();
            let id = metadata
                .compositions
                .entry(row["entry"].as_u64().unwrap() as usize)
                .unwrap();
            let (_, piece) = metadata.compositions.region(id).unwrap();
            let frames = piece.frame_plan.as_ref().unwrap();
            for binding in &frames.scalar_bindings {
                let recipe = frames
                    .frames
                    .iter()
                    .find(|frame| frame.value == binding.value)
                    .unwrap();
                let (path, _) = recipe
                    .scalar_paths
                    .iter()
                    .find(|(_, word)| *word == binding.word)
                    .unwrap();
                nested += usize::from(path.len() > 1);
                let mut fields = metadata
                    .buffer_frames
                    .fields_at(binding.value as usize)
                    .unwrap();
                let mut selected = None;
                for (depth, ordinal) in path.iter().enumerate() {
                    let field = fields[*ordinal];
                    selected = Some(field);
                    if depth + 1 < path.len() {
                        fields = metadata.buffer_frames.children(field).unwrap();
                    }
                }
                let Lane::Scalar { at, kind } = selected.unwrap().lane() else {
                    panic!("binding must name a scalar leaf")
                };
                assert_eq!((binding.physical, binding.kind), (at, kind));
            }
        }
        assert!(
            nested > 0,
            "exercise recursive scalar paths, not just a flat carrier"
        );
        assert!(
            inputs > 0 && outputs > 0,
            "both sides of native boundaries must use physical scalar words"
        );
        assert!(
            fused > 0,
            "at least one real entry removes the adjacent admission callback"
        );
    }
}

#[test]
fn numeric_scalar_bindings_read_fresh_nested_words_and_ignore_omitted_argument_homes() {
    let mut printed = false;
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = compile_parent_pieces(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (bytes, rounds, seed) in [
                (vec![3, 5, 7], 2, 11),
                (vec![2], 0, 81),
                (vec![1, 2], 3, 4),
                (vec![9, 8, 7, 6], 1, 101),
            ] {
                let args = arguments(&artifact, &memory, &bytes, rounds, seed);
                let aliases = args.clone();
                let before = contents(&args);
                let (result, counts) = observe::run_with_unused_exit_poison(
                    &jit,
                    &args,
                    &mut output,
                    mode.execution(),
                    &memory,
                    restored,
                );
                result.unwrap();
                assert!(
                    counts.selected > 0 && counts.fused_numeric_entries > 0,
                    "{counts:?}"
                );
                assert!(
                    counts
                        .selected_sites
                        .iter()
                        .any(|(entry, selected)| *selected > 0
                            && counts.admission_callbacks.get(entry).copied().unwrap_or(0) == 0),
                    "a selected fused entry must avoid a separate physical admission callback: {counts:?}"
                );
                assert!(
                    counts.poisoned_exit_words > 0 && counts.poisoned_private_homes > 0,
                    "{counts:?}"
                );
                if !printed {
                    println!(
                        "{}",
                        serde_json::json!({
                            "witness":"physical-scalar-bindings-fused-entry",
                            "fused_numeric_entries":counts.fused_numeric_entries,
                            "selected_entries":counts.selected_sites.iter().map(|(entry, selected)|serde_json::json!({
                                "entry":entry,"selected":selected,
                                "logical_admissions":counts.admits.get(entry).copied().unwrap_or(0),
                                "standalone_admission_callbacks":counts.admission_callbacks.get(entry).copied().unwrap_or(0),
                            })).collect::<Vec<_>>(),
                        })
                    );
                    printed = true;
                }

                let mut expected = bytes.clone();
                if rounds > 0 {
                    expected[0] += (bytes.len() as u8 + 1) * rounds as u8;
                }
                assert_eq!(
                    drafts(&output).last(),
                    Some(&Data::Tuple(
                        [
                            Data::Matrix(expected.into_iter().map(Data::U8).collect()),
                            Data::U64(seed + rounds),
                            Data::U64(seed + 2 * rounds),
                            Data::U64(seed + 3 * rounds),
                            Data::Matrix(bytes.into_iter().map(Data::U8).collect()),
                        ]
                        .into()
                    ))
                );
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

#[test]
fn numeric_scalar_bindings_keep_finite_boundaries_and_atomic_failure_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, &[3, 5, 7], 2, 11)
    });
    let mut errors = [None, None];
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
            let mut output = vec![None; artifact.outputs().len()];
            for (bytes, rounds, seed, failure) in [
                (vec![1, 2], 1, 5, None),
                (vec![], 1, 5, Some((0, "Index"))),
                (vec![1], 1, u64::MAX, Some((1, "Arithmetic"))),
                (vec![3], 2, 7, None),
            ] {
                let args = arguments(&artifact, &memory, &bytes, rounds, seed);
                let before = contents(&args);
                let prior = output.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, _) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if let Some((index, name)) = failure {
                    let text = format!("{:?}", result.unwrap_err());
                    assert!(text.contains(name), "{text}");
                    assert_eq!(text, *errors[index].get_or_insert(text.clone()));
                    assert_prior(&output, &prior);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                } else {
                    result.unwrap();
                }
                assert_eq!(contents(&args), before);
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn child_numeric_bindings_import_current_leaves_and_narrow_private_boundaries() {
    for artifact in artifacts(nested_scalar_array::SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.descriptor().metadata();
        let mut narrowed = false;
        let mut nested = false;
        let mut emitted = false;
        for row in object.metadata()["compiled_numeric_write_loops"]
            .as_array()
            .unwrap()
        {
            let header = row["header"].as_u64().unwrap() as usize;
            let region =
                &metadata.closed_tails.regions[metadata.closed_tails.sites[header].unwrap()];
            let proof = region.numeric.as_ref().unwrap();
            assert_eq!(row["entry_shadow_argument_copies"], 0);
            assert_eq!(row["runtime_scalar_path_walks"], 0);
            assert_eq!(
                row["scalar_input_loads"].as_u64().unwrap() as usize,
                proof.scalar_inputs.len()
            );
            assert!(row["scalar_output_stores"].as_u64().unwrap() > 0);
            assert!(
                row["source_home_words"].as_u64().unwrap() > 0,
                "cold source operands must survive the narrower private spill"
            );
            narrowed |= row["private_boundary_words"].as_u64().unwrap() < proof.words as u64;
            for binding in proof.scalar_inputs.iter().chain(&proof.scalar_outputs) {
                let frame = proof
                    .frames
                    .iter()
                    .find(|frame| frame.value == binding.value)
                    .unwrap();
                let (path, _) = frame
                    .scalar_paths
                    .iter()
                    .find(|(_, word)| *word == binding.word)
                    .unwrap();
                nested |= path.len() > 1;
                assert_eq!(
                    binding.steps,
                    path.len(),
                    "prepaid path work must retain the exact original depth"
                );
                let mut fields = metadata
                    .buffer_frames
                    .fields_at(binding.value as usize)
                    .unwrap();
                let mut selected = None;
                for (depth, ordinal) in path.iter().enumerate() {
                    let field = fields[*ordinal];
                    selected = Some(field);
                    if depth + 1 < path.len() {
                        fields = metadata.buffer_frames.children(field).unwrap();
                    }
                }
                let Lane::Scalar { at, kind } = selected.unwrap().lane() else {
                    panic!("physical scalar leaf")
                };
                assert_eq!((binding.physical, binding.kind), (at, kind));
            }
            assert!(!proof.scalar_inputs.is_empty() && !proof.scalar_outputs.is_empty());
            emitted = true;
        }
        assert!(
            emitted && narrowed && nested,
            "the witness must exercise direct nested imports, result exports and a smaller real cold boundary"
        );
    }
}

mod child_argument_poison {
    use super::*;
    use crate::native_artifact::program::protocol::abi::{Frame, Helper};
    use std::{cell::Cell, ffi::c_void};
    thread_local! {
        static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};
        static POISONED:Cell<(usize,usize)>=const{Cell::new((0,0))};
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
        // SAFETY: one synchronous checked callback, with all operands unchanged.
        let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
        if status == 0 && op == GlobalHelper::ClosedTailBegin.raw() && unsafe { *out } == 1 {
            // SAFETY: only omitted scalar argument homes are overwritten, after
            // the real checked entry has selected and retained its native loan.
            // Fuel0/1, bank pointer2, array views/capacities/flags and all owners
            // are untouched. These words must not initialize native scalars.
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            let descriptor = invocation.descriptor;
            let metadata = descriptor.metadata();
            if let Some(id) = metadata
                .closed_tails
                .initializers
                .get(site as usize)
                .copied()
                .flatten()
                && let Some(proof) = metadata.closed_tails.regions[id].numeric.as_ref()
                && !proof.scalar_inputs.is_empty()
            {
                for word in 1..proof.scalar_words {
                    invocation
                        .write_borrowed_word(2 + word, u64::MAX - word as u64)
                        .unwrap();
                }
                POISONED.with(|v| {
                    let (calls, words) = v.get();
                    v.set((calls + 1, words + proof.scalar_words.saturating_sub(1)))
                });
            }
        }
        status
    }
    pub(super) fn run(
        jit: &JitProgramArtifact,
        args: &[Value],
        output: &mut [Option<Value>],
        memory: &ManagedMemoryBudget,
        restored: bool,
    ) -> (
        Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
        (usize, usize),
    ) {
        let decoded =
            restored.then(|| super::super::super::tests::restore_admitted_helper_descriptor(jit));
        let descriptor = decoded.as_ref().unwrap_or_else(|| jit.descriptor());
        let entry = jit.test_checked_entry();
        POISONED.with(|v| v.set((0, 0)));
        let result = execute(
            descriptor,
            |raw: *mut Frame| {
                // SAFETY: execute retains this exact descriptor/entry/frame. Wiring
                // is restored before leaving the synchronous diagnostic invocation.
                unsafe {
                    let previous = (*raw).helper;
                    ORIGINAL.with(|v| v.set(Some(previous)));
                    (*raw).helper = helper;
                    let status = entry(raw);
                    (*raw).helper = previous;
                    ORIGINAL.with(|v| v.set(None));
                    status
                }
            },
            args,
            output,
            NativeProgramLimitMode::NoLimits.execution(),
            memory,
        );
        (result, POISONED.with(Cell::get))
    }
}

#[test]
fn child_numeric_bindings_ignore_stale_scalar_arguments_and_preserve_current_aliases() {
    for artifact in artifacts(nested_scalar_array::SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (bytes, steps, minimum, winner, marker) in [
                (vec![7, 12, 12, 2], 4, 7, 42, 9),
                (vec![31, 1, 6], 0, 99, 701, 111),
                (vec![2, 9], 1, 8, 96, 4),
                (vec![2, 9], 2, 8, 96, 4),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "bytes",
                            Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
                        ),
                        ("steps", Data::U64(steps)),
                        ("minimum", Data::U8(minimum)),
                        ("winner", Data::U64(winner)),
                        ("marker", Data::U64(marker)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let (result, (calls, words)) =
                    child_argument_poison::run(&jit, &args, &mut output, &memory, restored);
                result.unwrap();
                assert!(
                    calls > 0 && words > 0,
                    "real selected child imports must encounter poisoned obsolete scalar arguments"
                );
                let (mut best, mut index) = (minimum, winner);
                let mut changed = bytes.clone();
                for at in 0..steps as usize {
                    if bytes[at] > best {
                        best = bytes[at];
                        index = at as u64;
                    }
                    changed[at] = 0;
                }
                assert_eq!(
                    drafts(&output).last(),
                    Some(&Data::Tuple(
                        [
                            Data::Matrix(changed.into_iter().map(Data::U8).collect()),
                            Data::U8(best),
                            Data::U64(index),
                            Data::U64(marker + steps),
                            Data::Matrix(bytes.into_iter().map(Data::U8).collect())
                        ]
                        .into()
                    ))
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

#[test]
fn child_numeric_binding_images_reject_old_prototypes_and_forged_physical_recipes() {
    use crate::native_artifact::{
        NATIVE_PROGRAM_TYPED_IR_VERSION, NativeProgramDescriptor, NativeProgramObjectIdentity,
    };
    for artifact in artifacts(nested_scalar_array::SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let identity = NativeProgramObjectIdentity::from_descriptor(jit.descriptor()).unwrap();
        assert!(NATIVE_PROGRAM_TYPED_IR_VERSION > 150);
        for version in [149u64, 150] {
            let mut old = *identity.bytes();
            old[16..24].copy_from_slice(&version.to_le_bytes());
            assert!(
                crate::native_artifact::program::linked::prepare_descriptor(
                    &artifact, &catalog, &old
                )
                .is_err()
            );
        }
        let original: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        for field in ["physical", "kind", "word", "steps"] {
            let mut forged = original.clone();
            let numeric = forged["descriptor"]["closed_tails"]["regions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find_map(|region| {
                    let numeric = &mut region["numeric"];
                    (!numeric["scalar_inputs"].as_array()?.is_empty()).then_some(numeric)
                })
                .unwrap();
            let binding = &mut numeric["scalar_inputs"][0];
            if field == "kind" {
                binding[field] = if binding[field] == "Bool" {
                    "U64".into()
                } else {
                    "Bool".into()
                };
            } else {
                binding[field] = (binding[field].as_u64().unwrap() + 1).into();
            }
            let bytes = serde_json::to_vec(&forged).unwrap();
            // SAFETY: malformed descriptor diagnostics only; even an accepted
            // result is never associated with an entry or executed.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                    .is_err(),
                "forged independently derived scalar binding {field}"
            );
        }
    }
}

#[test]
fn wide_retirement_stores_preserve_cold_scalar_homes_aliases_errors_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::WideRetirementStores,
        )
        .unwrap();
        assert!(native.test_emissions().wide_retirement_pairs > 0);
        assert_eq!(ordinary.test_emissions().wide_retirement_pairs, 0);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            for (bytes, rounds, seed) in [
                (vec![3, 5, 7], 2, 11),
                (vec![], 1, 0),
                (vec![1, 2], 1, u64::MAX),
                (vec![255], 1, 0),
                (vec![2, 3], 1, 4),
            ] {
                let args = arguments(&artifact, &memory, &bytes, rounds, seed);
                let aliases = args.clone();
                let before = contents(&args);
                let mut results = Vec::new();
                for (jit, slot) in [&native, &ordinary].into_iter().zip(&mut output) {
                    let result = if restored {
                        observe::run_restored(jit, &args, slot, mode.execution(), &memory).0
                    } else {
                        jit.evaluate_into(&args, slot, mode.execution(), &memory)
                    };
                    results.push(format!("{result:?}"));
                }
                assert_eq!(results[0], results[1]);
                assert_eq!(drafts(&output[0]), drafts(&output[1]));
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
