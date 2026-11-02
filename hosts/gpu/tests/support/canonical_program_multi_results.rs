//! Current owners for every field of a private numeric tuple/record return.
use super::{allocation_probe, artifacts, equal, inputs, reference_result};
use mech_core::{Value, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

pub(super) const SOURCE: &str = r#"put16(xs<[u16]>, at<u64>, word<u16>) => (out<[u16]>) :=
  ~next := xs
  next[at] = word
  out := next.
put32(xs<[u32]>, at<u64>, word<u32>) => (out<[u32]>) :=
  ~next := xs
  next[at] = word
  out := next.
put8(xs<[u8]>, at<u64>, word<u8>) => (out<[u8]>) :=
  ~next := xs
  next[at] = word
  out := next.
quotient(xs<[u32]>, at<u64>, divisor<u64>) => (out<u32>) :=
  wide<u64> := xs[at]
  out<u32> := (wide + 1u64) / divisor.
walk(a<[u16]>, b<[u32]>, c<[u8]>, i<u64>, left<u64>, permit<bool>, divisor<u64>) => (out<([u16],u64,[u32],[u8])>) :=
  more := left > 0u64
  next-a := (more? | true => put16(a, i + 1u64, 8u16) | false => a)
  allowed! := permit == true
  next-b := (more? | true => put32(b, i + 1u64, quotient(b, i + 1u64, divisor)) | false => b)
  next-c := (more? | true => put8(c, i + 1u64, 18u8) | false => c)
  out := (more? | true => walk(next-a, next-b, next-c, i + 1u64, left - 1u64, permit, divisor) | false => (a, i + 17u64, b, c)).
(a,i,b,c) := walk(first<[u16]>, second<[u32]>, third<[u8]>, 0u64, steps<u64>, permit<bool>, divisor<u64>)
(a, i + 0u64, b, c)
"#;

pub(super) const RECORD_SOURCE: &str = include_str!("../fixtures/native/private-record-return.mec");

fn matrix(words: impl IntoIterator<Item = Data>) -> Data {
    Data::Matrix(words.into_iter().collect())
}
fn result(out: &[Option<Value>]) -> Data {
    out.last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn unchanged(out: &[Option<Value>], before: &[Option<Value>]) {
    assert!(out.iter().zip(before).all(|(a, b)| match (a, b) {
        (Some(a), Some(b)) => a.shares_frozen_storage(b),
        (None, None) => true,
        _ => false,
    }));
}
fn selected(artifact: &mech_engine::ProgramArtifact, count: usize) {
    let metadata = emit_program_object(artifact, &mech_stdlib::source_catalog())
        .unwrap()
        .metadata();
    let candidates = metadata["numeric_write_regions"].as_array().unwrap();
    assert!(
        candidates.iter().any(|r| {
            r["proof"]["result"]["Frame"]["arrays"]
                .as_array()
                .is_some_and(|a| a.len() == count)
        }),
        "missing {count}-array return proof: {metadata}"
    );
    for region in metadata["compiled_numeric_write_loops"].as_array().unwrap() {
        assert_eq!(region["successful_backedge_helpers"], 0);
    }
}

fn check_current_owners(source: &str, variant: &str) {
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        super::record_buffer_qualification::assert_build_owned_image_round_trip(&jit);
        let mut out = vec![None; artifact.outputs().len()];
        let mut rows = Vec::new();
        for (steps, permit, divisor, short) in [
            (0, true, 1, false),
            (1, true, 1, false),
            (64, true, 1, false),
            (1, false, 0, false),
            (1, true, 0, false),
            (1, true, 1, true),
            (2, true, 1, false),
            (0, true, 0, false),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("first", matrix(vec![Data::U16(7); 288])),
                    (
                        "second",
                        matrix(vec![Data::U32(11); if short { 0 } else { 288 }]),
                    ),
                    ("third", matrix(vec![Data::U8(17); 288])),
                    ("steps", Data::U64(steps)),
                    ("permit", Data::Bool(permit)),
                    ("divisor", Data::U64(divisor)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let before = args
                .iter()
                .map(|a| a.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let prior = out.clone();
            let used = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            });
            if permit && (steps == 0 || divisor != 0 && !short) {
                let report = run.unwrap();
                let updated =
                    |old: u64, new: u64| (0..288).map(move |i| if i < steps { new } else { old });
                assert_eq!(
                    result(&out),
                    Data::Tuple(
                        vec![
                            matrix(updated(7, 8).map(|v| Data::U16(v as u16))),
                            Data::U64(steps + 17),
                            matrix(updated(11, 12).map(|v| Data::U32(v as u32))),
                            matrix(updated(17, 18).map(|v| Data::U8(v as u8)))
                        ]
                        .into()
                    )
                );
                // The resident oracle has a finite comparison budget; keep its
                // semantic witnesses short, and measure the funded native slope separately.
                if steps < 64 {
                    equal(
                        &artifact,
                        &out,
                        &reference_result(&artifact, &args).unwrap(),
                    );
                }
                rows.push((steps, requests, bytes, report.visits));
                println!(
                    "{}",
                    serde_json::json!({"witness":"multi-array-current-return","variant":variant,"form":form,"steps":steps,"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work})
                );
            } else {
                let error = format!("{:?}", run.unwrap_err());
                assert!(
                    if !permit {
                        error.contains("Integrity")
                    } else if divisor == 0 {
                        error.contains("Arithmetic")
                    } else {
                        error.contains("Kernel")
                    },
                    "{error}"
                );
                assert!(reference_result(&artifact, &args).is_err());
                unchanged(&out, &prior);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    used
                );
            }
            assert_eq!(
                args.iter()
                    .map(|a| a.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                before
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        // Selection is checked after correctness so the parent run retains
        // useful ordinary-route observations before the expected red assertion.
        selected(&artifact, 3);
        let one = rows.iter().find(|r| r.0 == 1).unwrap();
        let many = rows.iter().find(|r| r.0 == 64).unwrap();
        assert_eq!(
            (one.1, one.2, one.3),
            (many.1, many.2, many.3),
            "per-step result transport remains: {rows:?}"
        );
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn multi_array_results_select_pinned_huffman_placement_and_keep_branch_versions() {
    let source = format!(
        "{}\n(counted,lengths,packed) := huff-place-symbols(freqs<[u32]>, 4u64, counts<[u64]>, lens<[u8]>, words<[u32]>, 0u64, 3u64)\n(counted,lengths,packed)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec")
    );
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = inputs(
            &artifact,
            &[
                ("freqs", matrix([0, 2, 0, 1].map(Data::U32))),
                ("counts", matrix([0, 0, 1, 2].map(Data::U64))),
                ("lens", matrix([7; 4].map(Data::U8))),
                ("words", matrix([99; 4].map(Data::U32))),
            ],
            &memory,
        );
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        assert_eq!(
            result(&out),
            Data::Tuple(
                vec![
                    matrix([0, 1, 2, 2].map(Data::U64)),
                    matrix([0, 7, 0, 7].map(Data::U8)),
                    matrix([1027, 2049, 99, 99].map(Data::U32))
                ]
                .into()
            )
        );
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        selected(&artifact, 3);
        drop((args, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn check_stale_return(source: &str) {
    // Only the second returned root is stale. Returning a root of the right
    // kind is insufficient after an earlier write invalidated that version.
    for artifact in artifacts(source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        assert!(
            metadata["numeric_write_regions"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{metadata}"
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("first", matrix([Data::U32(3)])),
                ("second", matrix([Data::U32(5)])),
            ],
            &memory,
        );
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        assert_eq!(
            result(&out),
            Data::Tuple(vec![matrix([Data::U32(9)]), matrix([Data::U32(5)])].into())
        );
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        drop((args, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn check_shared_input_backing(source: &str) {
    let source = source.replace("u16", "u32").replace("u8", "u32");
    for artifact in artifacts(&source).0 {
        selected(&artifact, 3);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for steps in [0, 1, 3, 0] {
            let mut args = inputs(
                &artifact,
                &[
                    ("first", matrix(vec![Data::U32(7); 3])),
                    ("second", matrix(vec![Data::U32(7); 3])),
                    ("third", matrix(vec![Data::U32(7); 3])),
                    ("steps", Data::U64(steps)),
                    ("permit", Data::Bool(true)),
                    ("divisor", Data::U64(1)),
                ],
                &memory,
            );
            let indices = ["first", "second", "third"].map(|name| {
                artifact
                    .inputs()
                    .iter()
                    .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == name)
                    .unwrap()
            });
            assert!(
                indices
                    .iter()
                    .all(|&i| args[i].schema() == args[indices[0]].schema())
            );
            for &i in &indices[1..] {
                args[i] = args[indices[0]].clone();
            }
            let alias = args[indices[0]].clone();
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            let words =
                |changed| matrix((0..3).map(|i| Data::U32(if i < steps { changed } else { 7 })));
            assert_eq!(
                result(&out),
                Data::Tuple(vec![words(8), Data::U64(steps + 17), words(8), words(18)].into())
            );
            assert!(
                indices
                    .iter()
                    .all(|&i| args[i].shares_frozen_storage(&alias))
            );
            assert_eq!(
                alias.canonical_data_draft().unwrap(),
                matrix(vec![Data::U32(7); 3])
            );
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn check_finite_rejections(source: &str) {
    for artifact in artifacts(source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("first", matrix(vec![Data::U16(7); 3])),
                ("second", matrix(vec![Data::U32(11); 3])),
                ("third", matrix(vec![Data::U8(17); 3])),
                ("steps", Data::U64(2)),
                ("permit", Data::Bool(true)),
                ("divisor", Data::U64(1)),
            ],
            &memory,
        );
        let aliases = args.clone();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        for dimension in 0..7 {
            let mut limits = mode.execution();
            match dimension {
                0 => limits.execution.max_work = 0,
                1 => limits.library.max_output_elements = Some(0),
                2 => limits.library.max_output_bytes = Some(0),
                3 => limits.library.max_temporary_bytes = Some(0),
                4 => limits.library.max_retained_nodes = Some(0),
                5 => limits.library.max_compute_work = Some(0),
                _ => limits.library.max_comparison_work = Some(0),
            }
            let prior = out.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            assert!(
                jit.evaluate_into(&args, &mut out, limits, &memory).is_err(),
                "dimension {dimension}"
            );
            unchanged(&out, &prior);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
        }
        drop((args, aliases, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn check_real_payloads(body: &str, binding: &str) {
    use mech_core::ValueData;
    let published_source = format!(
        "{body}\nrun(n16<u64>,n32<u64>,steps<u64>) => (out<([u16],u64,[u32],[u8])>) :=\n  first := [0u16 | i <- 0u64..=n16, i < n16]\n  second := [0u32 | i <- 0u64..=n32, i < n32]\n  third := [0u8 | i <- 0u64..=8u64, i < 8u64]\n  {binding}\n  out := (a,i + 0u64,b,c).\nrun(n16<u64>,n32<u64>,steps<u64>)\n"
    );
    // Public canonical tuple materialization has its own payload copy. Compare
    // it with a caller that consumes every returned buffer inside Mech, as the
    // compressor does, to isolate transport from public construction.
    let consumed_source = format!(
        "+> sequence\n{body}\nrun(n16<u64>,n32<u64>,steps<u64>) => (out<u64>) :=\n  first := [0u16 | i <- 0u64..=n16, i < n16]\n  second := [0u32 | i <- 0u64..=n32, i < n32]\n  third := [0u8 | i <- 0u64..=8u64, i < 8u64]\n  {binding}\n  x<u64> := a[1u64]\n  y<u64> := b[1u64]\n  z<u64> := c[1u64]\n  changed := steps > 0u64\n  ex := (changed? | true => 8u64 | false => 0u64)\n  ey := (changed? | true => 1u64 | false => 0u64)\n  ez := (changed? | true => 18u64 | false => 0u64)\n  current! := x == ex && y == ey && z == ez\n  last-a<u64> := a[sequence/length(a)]\n  last-b<u64> := b[sequence/length(b)]\n  untouched! := last-a == 0u64 && last-b == 0u64\n  out := i + sequence/length(a) + sequence/length(b) + sequence/length(c) + x + y + z.\nrun(n16<u64>,n32<u64>,steps<u64>)\n"
    );
    for (published, source, payloads) in [(true, published_source, 2), (false, consumed_source, 1)]
    {
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            selected(&artifact, 3);
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let mut rows = Vec::new();
            for (n16, n32) in [(288usize, 288usize), (262144, 1500513)] {
                for steps in [0, 1, 8] {
                    let memory = mode.memory_budget();
                    let args = inputs(
                        &artifact,
                        &[
                            ("n16", Data::U64(n16 as u64)),
                            ("n32", Data::U64(n32 as u64)),
                            ("steps", Data::U64(steps)),
                        ],
                        &memory,
                    );
                    let mut out = vec![None; artifact.outputs().len()];
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                        .unwrap();
                    out.fill(None);
                    let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    });
                    let report = run.unwrap();
                    if published {
                        let ValueData::Tuple(fields) = out.last().unwrap().as_ref().unwrap().data()
                        else {
                            panic!("tuple result")
                        };
                        macro_rules! check {
                            ($at:expr,$kind:ident,$count:expr,$changed:expr) => {{
                                let ValueData::Matrix(matrix) = &fields[$at] else {
                                    panic!("matrix")
                                };
                                let mech_core::snapshot::SequenceView::$kind(words) =
                                    matrix.elements()
                                else {
                                    panic!("kind")
                                };
                                assert_eq!(words.len(), $count);
                                assert!(words.iter().enumerate().all(|(i, &word)| word
                                    == if i < steps as usize { $changed } else { 0 }));
                            }};
                        }
                        check!(0, U16, n16, 8);
                        check!(2, U32, n32, 1);
                        check!(3, U8, 8, 18);
                        assert!(matches!(fields[1],ValueData::U64(n) if n==steps+17));
                    } else {
                        assert_eq!(
                            result(&out),
                            Data::U64(
                                n16 as u64
                                    + n32 as u64
                                    + 8
                                    + steps
                                    + 17
                                    + if steps > 0 { 27 } else { 0 }
                            )
                        );
                    }

                    rows.push((n16, n32, steps, requests, bytes, report.visits));
                    println!(
                        "{}",
                        serde_json::json!({"witness":"multi-array-real-payloads","form":form,"published_tuple":published,"payloads":payloads,"u16_count":n16,"u32_count":n32,"steps":steps,"requests":requests,"requested_bytes":bytes,"visits":report.visits})
                    );
                    drop((args, out));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
            for steps in [0, 1, 8] {
                let small = rows.iter().find(|r| r.0 == 288 && r.2 == steps).unwrap();
                let real = rows.iter().find(|r| r.0 == 262144 && r.2 == steps).unwrap();
                assert_eq!(
                    real.4 - small.4,
                    payloads * ((262144 - 288) * 2 + (1500513 - 288) * 4),
                    "no hidden payload copies: {rows:?}"
                );
            }
            for n16 in [288, 262144] {
                let one = rows.iter().find(|r| r.0 == n16 && r.2 == 1).unwrap();
                let many = rows.iter().find(|r| r.0 == n16 && r.2 == 8).unwrap();
                assert_eq!(
                    (one.3, one.4, one.5),
                    (many.3, many.4, many.5),
                    "no per-step transactions: {rows:?}"
                );
            }
        }
    }
}

#[test]
fn multi_array_results_restore_all_current_owners_and_recover_after_late_failure() {
    check_current_owners(SOURCE, "tuple");
}

#[test]
fn multi_record_results_restore_all_current_owners_and_recover_after_late_failure() {
    check_current_owners(RECORD_SOURCE, "record");
}

#[test]
fn multi_array_results_shared_input_backing_stays_immutable_for_each_root() {
    check_shared_input_backing(SOURCE);
}

#[test]
fn multi_record_results_shared_input_backing_stays_immutable_for_each_root() {
    check_shared_input_backing(RECORD_SOURCE);
}

#[test]
fn multi_record_results_repeated_root_fields_keep_current_values_and_recover() {
    let source = r#"<repeated-record> := <{words<[u32]> index<u64> twin<[u32]>}>
put(xs<[u32]>) => (out<[u32]>) :=
  ~next := xs
  next[1u64] = xs[1u64] + 1u32
  out := next.
walk(xs<[u32]>, left<u64>, permit<bool>) => (out<repeated-record>) :=
  allowed! := permit
  next := put(xs)
  out := ((left > 0u64)? | true => walk(next, left - 1u64, permit) | false => {words: next, index: left + 17u64, twin: next}).
r := walk(data<[u32]>, steps<u64>, permit<bool>)
words := r.words
twin := r.twin
index := r.index
(words,twin,index + 0u64)
"#;
    for artifact in artifacts(source).0 {
        selected(&artifact, 2);
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        let regions = metadata["numeric_write_regions"].as_array().unwrap();
        assert_eq!(regions.len(), 1);
        let fields = regions[0]["proof"]["result"]["Frame"]["arrays"]
            .as_array()
            .unwrap();
        assert_eq!(fields.len(), 2);
        assert_ne!(fields[0][0], fields[1][0]);
        assert_eq!(fields[0][1], fields[1][1]);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        super::record_buffer_qualification::assert_build_owned_image_round_trip(&jit);
        let mut out = vec![None; artifact.outputs().len()];
        for (initial, steps, permit) in [(7, 0, true), (11, 2, true), (17, 1, false), (23, 1, true)]
        {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix([Data::U32(initial)])),
                    ("steps", Data::U64(steps)),
                    ("permit", Data::Bool(permit)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = out.clone();
            let evaluated = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
            if permit {
                evaluated.unwrap();
                let words = matrix([Data::U32(initial + steps as u32 + 1)]);
                assert_eq!(
                    result(&out),
                    Data::Tuple(vec![words.clone(), words, Data::U64(17)].into())
                );
                equal(
                    &artifact,
                    &out,
                    &reference_result(&artifact, &args).unwrap(),
                );
            } else {
                assert!(matches!(
                    evaluated,
                    Err(
                        mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                            mech_gpu::native_artifact::NativeIterationExecutionError::Integrity { .. }
                        )
                    )
                ));
                unchanged(&out, &prior);
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            let data = artifact
                .inputs()
                .iter()
                .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == "data")
                .unwrap();
            assert_eq!(
                aliases[data].canonical_data_draft().unwrap(),
                matrix([Data::U32(initial)])
            );
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn multi_array_results_finite_rejections_keep_prior_publication_and_retry() {
    check_finite_rejections(SOURCE);
}

#[test]
fn multi_record_results_finite_rejections_keep_prior_publication_and_retry() {
    check_finite_rejections(RECORD_SOURCE);
}

#[test]
fn multi_array_results_check_freshness_of_each_returned_field() {
    check_stale_return(
        r#"put(xs<[u32]>) => (out<[u32]>) :=
  ~next := xs
  next[1u64] = 9u32
  out := next.
walk(a<[u32]>, b<[u32]>, left<u64>) => (out<([u32],[u32])>) :=
  next-a := put(a)
  next-b := put(b)
  out := ((left > 0u64)? | true => walk(next-a,next-b,left - 1u64) | false => (next-a,b)).
(a,b) := walk(first<[u32]>,second<[u32]>,0u64)
(a,b)
"#,
    );
}

#[test]
fn multi_record_results_check_freshness_of_each_returned_field() {
    check_stale_return(
        r#"<stale-record> := <{changed<[u32]> stale<[u32]>}>
put(xs<[u32]>) => (out<[u32]>) :=
  ~next := xs
  next[1u64] = 9u32
  out := next.
walk(a<[u32]>, b<[u32]>, left<u64>) => (out<stale-record>) :=
  next-a := put(a)
  next-b := put(b)
  out := ((left > 0u64)? | true => walk(next-a,next-b,left - 1u64) | false => {changed: next-a, stale: b}).
r := walk(first<[u32]>,second<[u32]>,0u64)
changed := r.changed
stale := r.stale
(changed,stale)
"#,
    );
}

#[test]
fn multi_array_results_real_tree_and_cache_keep_one_initial_payload_each() {
    check_real_payloads(
        SOURCE.split("(a,i,b,c) :=").next().unwrap(),
        "(a,i,b,c) := walk(first,second,third,0u64,steps,true,1u64)",
    );
}

#[test]
fn multi_record_results_real_tree_and_cache_keep_one_initial_payload_each() {
    check_real_payloads(
        RECORD_SOURCE.split("r := walk").next().unwrap(),
        "r := walk(first,second,third,0u64,steps,true,1u64)\n  a := r.first\n  i := r.position\n  b := r.second\n  c := r.third",
    );
}

#[test]
fn multi_record_results_select_the_unchanged_full_length_codeword_builder() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-writer-tables-reference.json"
    ))
    .unwrap();
    let source = format!(
        "{}\n{}\nr := block-compute-full-length-codewords(lens<[u8]>, words<[u32]>, slots<[u8]>, bases<[u32]>, extra<[u8]>)\nwords-result := r.codewords\nlens-result := r.lens\n(words-result,lens-result)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        include_str!("../fixtures/native/deflate-writer-tables.mec")
    );
    let field = |values: &serde_json::Value, width| {
        matrix(values.as_array().unwrap().iter().map(|v| match width {
            8 => Data::U8(v.as_u64().unwrap() as u8),
            32 => Data::U32(v.as_u64().unwrap() as u32),
            _ => unreachable!(),
        }))
    };
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        super::record_buffer_qualification::assert_build_owned_image_round_trip(&jit);
        for case in fixture["full_cases"].as_array().unwrap() {
            let memory = mode.memory_budget();
            let args = inputs(
                &artifact,
                &[
                    ("lens", field(&case["lens"], 8)),
                    ("words", field(&case["codewords"], 32)),
                    ("slots", field(&fixture["length_slot_tab"], 8)),
                    ("bases", field(&fixture["length_slot_base"], 32)),
                    ("extra", field(&fixture["extra_length_bits"], 8)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut out = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            assert_eq!(
                result(&out),
                Data::Tuple(
                    vec![
                        field(&case["expected"]["codewords"], 32),
                        field(&case["expected"]["lens"], 8)
                    ]
                    .into()
                )
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            drop((args, aliases, out));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
        selected(&artifact, 2);
    }
}

#[test]
fn multi_record_results_public_escape_keeps_the_canonical_constructor() {
    let body = RECORD_SOURCE.split("r := walk").next().unwrap();
    let source =
        format!("{body}walk(first<[u16]>,second<[u32]>,third<[u8]>,0u64,1u64,true,1u64)\n");
    for artifact in artifacts(&source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        assert!(
            metadata["numeric_write_regions"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{metadata}"
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("first", matrix([Data::U16(7)])),
                ("second", matrix([Data::U32(11)])),
                ("third", matrix([Data::U8(17)])),
            ],
            &memory,
        );
        let aliases = args.clone();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        let Data::Record(fields) = result(&out) else {
            panic!()
        };
        for (name, expected) in [
            ("first", matrix([Data::U16(8)])),
            ("second", matrix([Data::U32(12)])),
            ("third", matrix([Data::U8(18)])),
            ("position", Data::U64(18)),
        ] {
            assert_eq!(
                fields.iter().find(|f| f.name == name).unwrap().value,
                expected
            );
        }
        assert!(
            args.iter()
                .zip(&aliases)
                .all(|(a, b)| a.shares_frozen_storage(b))
        );
        drop((args, aliases, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
