//! Current variable-length owners, capacity fallback and ordered failures.
use super::super::{artifacts, equal, inputs, reference_result};
use mech_core::{Value, ValueDataDraft as Data, snapshot::UnsignedMatrixBuffer};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};
use std::sync::Arc;

pub(crate) fn source(width: u16) -> String {
    format!(
        r#"+> sequence
quotient(word<u{width}>, divisor<u64>) => (out<u{width}>) :=
  wide<u64> := word
  out<u{width}> := wide / divisor.
walk(xs<[u{width}]>, i<u64>, left<u64>, reject<u64>, word<u{width}>, divisor<u64>) => (out<([u{width}],u64)>) :=
  allowed! := i < reject
  more := left > 0u64
  before := sequence/length(xs)
  next := (more? | true => sequence/append(xs, quotient(word, divisor)) | false => xs)
  expected := before + (more? | true => 1u64 | false => 0u64)
  current! := sequence/length(next) == expected
  out := (more? | true => walk(next, i + 1u64, left - 1u64, reject, word, divisor) | false => (xs, i + 17u64)).
seed := sequence/reserve(data<[u{width}]>, 1u64)
(result, position) := walk(seed, 0u64, steps<u64>, reject<u64>, word<u{width}>, divisor<u64>)
(result, position + 0u64)
"#
    )
}
fn word(width: u16, n: u64) -> Data {
    match width {
        8 => Data::U8(n as u8),
        16 => Data::U16(n as u16),
        32 => Data::U32(n as u32),
        64 => Data::U64(n),
        _ => unreachable!(),
    }
}
fn selected(artifact: &mech_engine::ProgramArtifact) {
    let metadata = emit_program_object(artifact, &mech_stdlib::source_catalog())
        .unwrap()
        .metadata();
    assert!(
        metadata["numeric_write_regions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["proof"]["arrays"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["appends"] == true)),
        "{}",
        metadata["readonly_numeric_candidates"]
    );
    for r in metadata["compiled_numeric_write_loops"].as_array().unwrap() {
        assert_eq!(r["successful_backedge_helpers"], 0);
        assert!(!r["append_sites"].as_array().unwrap().is_empty());
        assert!(!r["capacity_guards"].as_array().unwrap().is_empty());
    }
}
fn buffered_inputs(
    artifact: &mech_engine::ProgramArtifact,
    memory: &mech_core::ManagedMemoryBudget,
    width: u16,
    capacity: usize,
    steps: u64,
    reject: u64,
    incoming: u64,
    divisor: u64,
) -> Vec<Value> {
    let mut args = inputs(
        artifact,
        &[
            ("data", Data::Matrix([word(width, 7)].into())),
            ("steps", Data::U64(steps)),
            ("reject", Data::U64(reject)),
            ("word", word(width, incoming)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    );
    let index = artifact
        .inputs()
        .iter()
        .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == "data")
        .unwrap();
    macro_rules! buffer {
        ($ty:ty, $kind:ident) => {{
            let mut values = Vec::<$ty>::with_capacity(capacity);
            values.push(7);
            UnsignedMatrixBuffer::$kind(values)
        }};
    }
    let cells = match width {
        8 => buffer!(u8, U8),
        16 => buffer!(u16, U16),
        32 => buffer!(u32, U32),
        64 => buffer!(u64, U64),
        _ => unreachable!(),
    };
    let value = Value::from_unsigned_matrix_buffer(
        artifact.inputs()[index].schema,
        args[index].shape().clone(),
        cells,
        Arc::new(artifact.schemas().clone()),
    )
    .unwrap();
    let mut reserve = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    args[index] = value.into_memory_budget(&mut reserve).unwrap();
    args
}
fn unchanged(out: &[Option<Value>], before: &[Option<Value>]) {
    assert!(out.iter().zip(before).all(|(a, b)| match (a, b) {
        (Some(a), Some(b)) => a.shares_frozen_storage(b),
        (None, None) => true,
        _ => false,
    }));
}

#[test]
fn append_lengths_capacity_exhaustion_current_arguments_aliases_and_recovery() {
    for width in [8, 16, 32, 64] {
        for artifact in artifacts(&source(width)).0 {
            selected(&artifact);
            let mode = NativeProgramLimitMode::NoLimits;
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            super::super::record_buffer_qualification::assert_build_owned_image_round_trip(&jit);
            let memory = mode.memory_budget();
            let mut out = vec![None; artifact.outputs().len()];
            for (capacity, steps, reject, incoming, divisor) in [
                (32, 0, u64::MAX, 9, 0),
                (32, 3, u64::MAX, 11, 1),
                (32, 4, 2, 13, 1),
                (32, 2, 0, 17, 0),
                (32, 2, u64::MAX, 17, 0),
                (32, 3, u64::MAX, 0, 1),
                (2, 7, u64::MAX, 19, 1),
                (32, 1, u64::MAX, 23, 1),
            ] {
                let args = buffered_inputs(
                    &artifact, &memory, width, capacity, steps, reject, incoming, divisor,
                );
                let aliases = args.clone();
                let prior = out.clone();
                let used = memory.used_bytes();
                let evaluated = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
                let reference = reference_result(&artifact, &args);
                if reject <= steps || (divisor == 0 && steps > 0) {
                    let error = evaluated.unwrap_err();
                    if reject <= steps {
                        assert!(format!("{error:?}").contains("Integrity"));
                    } else {
                        assert!(format!("{error:?}").contains("Arithmetic"));
                    }
                    assert!(reference.is_err());
                    unchanged(&out, &prior);
                    assert_eq!(memory.used_bytes(), used);
                } else {
                    evaluated.unwrap();
                    equal(&artifact, &out, &reference.unwrap());
                    let words = Data::Matrix(
                        std::iter::once(word(width, 7))
                            .chain((0..steps).map(|_| word(width, incoming)))
                            .collect(),
                    );
                    assert_eq!(
                        out.last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Tuple([words, Data::U64(steps + 17)].into())
                    );
                }
                for (a, b) in args.iter().zip(&aliases) {
                    assert!(a.shares_frozen_storage(b));
                }
                let index = artifact
                    .inputs()
                    .iter()
                    .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == "data")
                    .unwrap();
                assert_eq!(
                    aliases[index].canonical_data_draft().unwrap(),
                    Data::Matrix([word(width, 7)].into())
                );
            }
            drop(out);
            assert_eq!(
                (
                    memory.used_bytes(),
                    memory.retained_snapshot_nodes(),
                    memory.retained_nodes()
                ),
                (0, 0, 0)
            );
        }
    }
}

#[test]
fn append_finite_denials_preserve_prior_publication_and_retry() {
    for artifact in artifacts(&source(32)).0 {
        selected(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = mode.memory_budget();
        let args = buffered_inputs(&artifact, &memory, 32, 32, 7, u64::MAX, 17, 1);
        let aliases = args.clone();
        let mut out = vec![None; artifact.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        for dimension in 0..6 {
            let mut policy = mode.execution();
            match dimension {
                0 => policy.execution.max_work = report.work / 2,
                1 => policy.library.max_compute_work = Some(1),
                2 => policy.library.max_output_elements = Some(6),
                3 => policy.library.max_output_bytes = Some(1),
                4 => policy.library.max_temporary_bytes = Some(1),
                5 => policy.library.max_retained_nodes = Some(1),
                _ => unreachable!(),
            }
            let prior = out.clone();
            let used = memory.used_bytes();
            assert!(
                jit.evaluate_into(&args, &mut out, policy, &memory).is_err(),
                "dimension={dimension}"
            );
            unchanged(&out, &prior);
            assert_eq!(memory.used_bytes(), used);
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            for (a, b) in args.iter().zip(&aliases) {
                assert!(a.shares_frozen_storage(b));
            }
        }
        let mut finite = super::super::limits();
        finite.execution.max_work = report.work * 4;
        finite.library.max_compute_work = Some(report.work * 4);
        let finite_report = jit.evaluate_into(&args, &mut out, finite, &memory).unwrap();
        assert_eq!(
            finite_report.visits, report.visits,
            "finite funding declined native execution"
        );
        // Append has no old-cell equality comparison. Zero comparison funding
        // is a supported success when this route reports zero such work.
        if report.comparison_work == 0 {
            let mut policy = mode.execution();
            policy.library.max_comparison_work = Some(0);
            jit.evaluate_into(&args, &mut out, policy, &memory).unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
        }
        drop((args, aliases, out));
        assert_eq!(
            (
                memory.used_bytes(),
                memory.retained_snapshot_nodes(),
                memory.retained_nodes()
            ),
            (0, 0, 0)
        );
    }
}

#[test]
fn old_array_length_after_append_keeps_the_ordinary_version() {
    let source = source(64).replace(
        "current! := sequence/length(next) == expected",
        "old-length! := sequence/length(xs) == before",
    );
    for artifact in artifacts(&source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        assert!(
            metadata["numeric_write_regions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            metadata["readonly_numeric_candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["rejection"] == "content use reads an older array version")
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = buffered_inputs(&artifact, &memory, 64, 32, 2, u64::MAX, 9, 1);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        assert_eq!(
            out.last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Tuple(
                [
                    Data::Matrix([Data::U64(7), Data::U64(9), Data::U64(9)].into()),
                    Data::U64(19)
                ]
                .into()
            )
        );
        drop((args, out));
        assert_eq!(memory.used_bytes(), 0);
    }
}
