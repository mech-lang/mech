//! Copy complexity through the recursive packed-argument carrier, not just a leaf.
use super::{allocation_probe, artifacts, inputs};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

fn pointer(value: &mech_core::Value) -> usize {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("packed matrix")
    };
    match matrix.elements() {
        mech_core::snapshot::SequenceView::U8(v) => v.as_ptr() as usize,
        mech_core::snapshot::SequenceView::U16(v) => v.as_ptr() as usize,
        mech_core::snapshot::SequenceView::U32(v) => v.as_ptr() as usize,
        mech_core::snapshot::SequenceView::U64(v) => v.as_ptr() as usize,
        _ => panic!("packed kind"),
    }
}

#[test]
fn selected_unsigned_store_changed_bit_aliases_and_admission_are_exact() {
    use mech_engine::native_artifact::{
        NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
        NativeLibraryWorkBudget, lower_native_program_artifact,
        prepare_native_program_library_call,
    };
    for width in [8, 16, 32, 64] {
        let word = |n| match width {
            8 => Data::U8(n as u8),
            16 => Data::U16(n as u16),
            32 => Data::U32(n as u32),
            64 => Data::U64(n),
            _ => unreachable!(),
        };
        let source = format!(
            "<buffer> := <[u{width}]:2,3>\nreplace(xs<buffer>, at<u64>, word<u{width}>) => (out<buffer>) :=\n  ~next := xs\n  next[at] = word\n  out := next.\nreplace(input<buffer>, at<u64>, word<u{width}>)\n"
        );
        for artifact in artifacts(&source).0 {
            let catalog = mech_stdlib::source_catalog();
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let template = plan
                .blocks()
                .iter()
                .find_map(|b| match b.action() {
                    Action::LibraryCall { template, .. }
                        if template.primitive() == Primitive::MatrixIndexedReplace =>
                    {
                        Some(template)
                    }
                    _ => None,
                })
                .unwrap();
            let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            assert!(call.supports_unsigned_matrix_replace());
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::Matrix((1..=6).map(word).collect())),
                    ("at", Data::U64(2)),
                    ("word", word(127)),
                ],
                &memory,
            );
            let original = &args[artifact
                .inputs()
                .iter()
                .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == "input")
                .unwrap()];
            let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 32, &memory).unwrap();
            let policy = super::limits().library;
            let high = if width == 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            };
            let (first, changed) = call
                .unsigned_matrix_replace(
                    original.clone(),
                    2,
                    high,
                    true,
                    policy,
                    &work,
                    Default::default(),
                )
                .unwrap();
            assert!(changed);
            assert_ne!(
                pointer(&first),
                pointer(original),
                "external alias requires a copy"
            );
            let mut expected = (1..=6).map(word).collect::<Vec<_>>();
            expected[3] = word(high);
            assert_eq!(
                first.canonical_data_draft().unwrap(),
                Data::Matrix(expected.clone().into())
            );
            let backing = pointer(&first);
            let mut zero_copy = policy;
            zero_copy.max_cloned_bytes = Some(0);
            let (second, changed) = call
                .unsigned_matrix_replace(first, 6, 99, true, zero_copy, &work, Default::default())
                .unwrap();
            assert!(changed);
            assert_eq!(
                pointer(&second),
                backing,
                "private update transfers its backing"
            );
            expected[5] = word(99);
            assert_eq!(
                second.canonical_data_draft().unwrap(),
                Data::Matrix(expected.into())
            );
            let (same, changed) = call
                .unsigned_matrix_replace(
                    second.clone(),
                    6,
                    99,
                    true,
                    zero_copy,
                    &work,
                    Default::default(),
                )
                .unwrap();
            assert!(!changed);
            assert!(
                same.shares_frozen_storage(&second),
                "unchanged write keeps the exact immutable owner"
            );
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            assert!(
                call.unsigned_matrix_replace(
                    original.clone(),
                    1,
                    high,
                    true,
                    zero_copy,
                    &work,
                    Default::default()
                )
                .is_err()
            );
            assert!(
                call.unsigned_matrix_replace(
                    second.clone(),
                    7,
                    high,
                    true,
                    policy,
                    &work,
                    Default::default()
                )
                .is_err()
            );
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            drop(same);
            let (recovered, changed) = call
                .unsigned_matrix_replace(
                    second,
                    1,
                    high,
                    true,
                    zero_copy,
                    &work,
                    Default::default(),
                )
                .unwrap();
            assert!(changed);
            assert_eq!(pointer(&recovered), backing);
            assert_eq!(
                original.canonical_data_draft().unwrap(),
                Data::Matrix((1..=6).map(word).collect())
            );
            drop((args, recovered, work));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn recursive_unsigned_updates_allocate_only_the_necessary_payload() {
    let mut comparisons = Vec::new();
    for (width, real) in [(16, 262_144usize), (32, 1_500_513)] {
        for private in [false, true] {
            let entry = if private {
                format!(
                    "run(count<u64>, steps<u64>) => (out<[u{width}]>) :=\n  words := [0u{width} | i <- 0u64..=count, i < count]\n  out := walk(words, 1u64, steps).\nrun(count<u64>, steps<u64>)\n"
                )
            } else {
                format!("walk(input<[u{width}]>, 1u64, steps<u64>)\n")
            };
            let source = format!(
                "put(words<[u{width}]>, at<u64>) => (out<[u{width}]>) :=\n  ~next := words\n  next[at] = 7u{width}\n  out := next.\nwalk(words<[u{width}]>, at<u64>, left<u64>) => <[u{width}]>\n  | (words, at, left) => ((left > 0u64)? | true => walk(put(words, at), at + 1u64, left - 1u64) | false => words).\n{entry}"
            );
            for (form, artifact) in ["original", "decoded"]
                .into_iter()
                .zip(artifacts(&source).0)
            {
                let mode = NativeProgramLimitMode::NoLimits;
                let jit = JitProgramArtifact::compile_with_limits(
                    &artifact,
                    &mech_stdlib::source_catalog(),
                    mode.compilation(),
                )
                .unwrap();
                let mut measurements = Vec::new();
                for count in [288, real] {
                    for steps in [0, 1, 8] {
                        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                        let named = if private {
                            vec![
                                ("count", Data::U64(count as u64)),
                                ("steps", Data::U64(steps)),
                            ]
                        } else {
                            let zero = if width == 16 {
                                Data::U16(0)
                            } else {
                                Data::U32(0)
                            };
                            vec![
                                ("input", Data::Matrix(vec![zero; count].into())),
                                ("steps", Data::U64(steps)),
                            ]
                        };
                        let args = inputs(&artifact, &named, &memory);
                        let aliases = args.clone();
                        let mut output = vec![None; artifact.outputs().len()];
                        jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                            .unwrap();
                        output.fill(None);
                        let started = std::time::Instant::now();
                        let (result, requests, bytes) =
                            allocation_probe::measured_with_bytes(|| {
                                jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                            });
                        let elapsed = started.elapsed().as_secs_f64();
                        let report = result.unwrap();
                        let ValueData::Matrix(matrix) =
                            output.last().unwrap().as_ref().unwrap().data()
                        else {
                            panic!("packed output")
                        };
                        macro_rules! check {
                            ($words:expr) => {{
                                assert_eq!($words.len(), count);
                                assert!($words.iter().enumerate().all(
                                    |(i, &word)| word == if i < steps as usize { 7 } else { 0 }
                                ));
                            }};
                        }
                        match matrix.elements() {
                            mech_core::snapshot::SequenceView::U16(words) => check!(words),
                            mech_core::snapshot::SequenceView::U32(words) => check!(words),
                            _ => panic!("packed kind"),
                        }
                        assert!(
                            args.iter()
                                .zip(&aliases)
                                .all(|(a, b)| a.shares_frozen_storage(b))
                        );
                        for value in &aliases {
                            if let ValueData::Matrix(matrix) = value.data() {
                                match matrix.elements() {
                                    mech_core::snapshot::SequenceView::U16(words) => {
                                        assert!(words.iter().all(|&v| v == 0))
                                    }
                                    mech_core::snapshot::SequenceView::U32(words) => {
                                        assert!(words.iter().all(|&v| v == 0))
                                    }
                                    _ => panic!("input kind"),
                                }
                            }
                        }
                        println!(
                            "{}",
                            serde_json::json!({"witness":"recursive-scalar-store","form":form,"width":width,"private":private,"count":count,"steps":steps,"requests":requests,"requested_bytes":bytes,"elapsed_seconds":elapsed,"visits":report.visits,"work":report.work})
                        );
                        measurements.push((count, steps, bytes));
                        drop((output, args, aliases));
                        assert_eq!(memory.used_bytes(), 0);
                        assert_eq!(memory.retained_snapshot_nodes(), 0);
                    }
                }
                for steps in [0, 1, 8] {
                    let bytes = |count| {
                        measurements
                            .iter()
                            .find(|(n, s, _)| *n == count && *s == steps)
                            .unwrap()
                            .2
                    };
                    let expected = usize::from(private || steps > 0) * (real - 288) * (width / 8);
                    comparisons.push((
                        width,
                        private,
                        form,
                        steps,
                        bytes(real) - bytes(288),
                        expected,
                    ));
                }
            }
        }
    }
    for (width, private, form, steps, actual, expected) in comparisons {
        assert_eq!(
            actual, expected,
            "u{width}, private={private}, {form}, stores={steps}: payload allocation must not scale with store count"
        );
    }
}
