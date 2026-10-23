//! Exact numeric families, changing shapes and late range failures.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::snapshot::{F32Bits, F64Bits};
use mech_core::{ManagedMemoryBudget, ResidentKernelError, ValueDataDraft as Data};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionError, NativeProgramExecutionError, NativeValueError,
    emit_program_object,
};

fn exercise(source: &str, calls: &[[Data; 3]]) {
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        for values in calls {
            let named = [
                ("start", values[0].clone()),
                ("increment", values[1].clone()),
                ("end", values[2].clone()),
            ];
            let args = inputs(&artifact, &named, &memory);
            match reference_result(&artifact, &args) {
                Ok(expected) => {
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &expected);
                }
                Err(error) => {
                    assert!(error.contains("InvalidInput"), "{error}");
                    let prior = output
                        .iter()
                        .map(|v| v.as_ref().expect("a prior successful call").clone())
                        .collect::<Vec<_>>();
                    let before = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    assert!(matches!(error, NativeProgramExecutionError::Library {error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(ResidentKernelError::InvalidInput), ..}), "{error:?}");
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), before);
                }
            }
        }
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn all_numeric_range_families_keep_exact_kinds_live_shapes_and_late_failure_recovery() {
    let kinds: [(&str, fn(i64) -> Data); 13] = [
        ("ix", |n| Data::Index(n as u64)),
        ("u8", |n| Data::U8(n as u8)),
        ("u16", |n| Data::U16(n as u16)),
        ("u32", |n| Data::U32(n as u32)),
        ("u64", |n| Data::U64(n as u64)),
        ("u128", |n| Data::U128(n as u128)),
        ("i8", |n| Data::I8(n as i8)),
        ("i16", |n| Data::I16(n as i16)),
        ("i32", |n| Data::I32(n as i32)),
        ("i64", Data::I64),
        ("i128", |n| Data::I128(n as i128)),
        ("f32", |n| Data::F32(F32Bits::from_f32(n as f32))),
        ("f64", |n| Data::F64(F64Bits::from_f64(n as f64))),
    ];
    for (kind, scalar) in kinds {
        let source = format!(
            "from := start<{kind}>\nstride := increment<{kind}>\nuntil := end<{kind}>\na := from..until\nb := from..=until\nc := from..stride..until\nd := from..stride..=until\n"
        );
        // Canonical Index values are strictly positive; zero is not a valid
        // input value of that type. Exercise its valid but empty range error
        // instead, while every other numeric kind checks a zero stride.
        let invalid = if kind == "ix" { (1, 1, 1) } else { (1, 0, 5) };
        let calls = [(1, 2, 9), (2, 3, 8), invalid, (9, 1, 10)]
            .map(|(a, b, c)| [scalar(a), scalar(b), scalar(c)]);
        exercise(&source, &calls);
    }
}

#[test]
fn wide_endpoints_and_descending_float_ranges_do_not_narrow_or_change_bits() {
    for (kind, values) in [
        (
            "u128",
            vec![
                [
                    Data::U128(u128::MAX - 4),
                    Data::U128(2),
                    Data::U128(u128::MAX),
                ],
                [
                    Data::U128(1 << 100),
                    Data::U128(1),
                    Data::U128((1 << 100) + 3),
                ],
            ],
        ),
        (
            "i128",
            vec![
                [
                    Data::I128(i128::MIN),
                    Data::I128(2),
                    Data::I128(i128::MIN + 4),
                ],
                [
                    Data::I128(i128::MAX),
                    Data::I128(-2),
                    Data::I128(i128::MAX - 4),
                ],
            ],
        ),
        ("i64", vec![[Data::I64(9), Data::I64(-2), Data::I64(1)]]),
        (
            "f32",
            vec![[
                Data::F32(F32Bits::from_f32(0.5)),
                Data::F32(F32Bits::from_f32(-0.1)),
                Data::F32(F32Bits::from_f32(0.1)),
            ]],
        ),
        (
            "f64",
            vec![[
                Data::F64(F64Bits::from_f64(0.5)),
                Data::F64(F64Bits::from_f64(-0.1)),
                Data::F64(F64Bits::from_f64(0.1)),
            ]],
        ),
    ] {
        exercise(
            &format!(
                "from := start<{kind}>\nstride := increment<{kind}>\nuntil := end<{kind}>\na := from..stride..until\nb := from..stride..=until\n"
            ),
            &values,
        );
    }
    // Independent exact-bit oracle for the non-associative repeated addition.
    let source = "0.5..-0.1..=0.1\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, &[], &memory);
        let expected = reference(&artifact, &args);
        let data = expected.last().unwrap().canonical_data_draft().unwrap();
        let Data::Matrix(values) = data else {
            panic!("range is a matrix")
        };
        let bits = values
            .iter()
            .map(|value| match value {
                Data::F64(v) => v.bits(),
                _ => panic!("exact F64"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            bits,
            [
                0x3fe0000000000000,
                0x3fd999999999999a,
                0x3fd3333333333334,
                0x3fc999999999999b,
                0x3fb999999999999c
            ]
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &expected);
    }
}

#[test]
fn materialized_numeric_range_keeps_exact_work_finite_denials_and_atomic_recovery() {
    for source in [
        "start<u64>..end<u64>\n",
        "expand(x<u64>, y<u64>) => (out<[u64]>) := out := x..y.\nexpand(start<u64>, end<u64>)\n",
    ] {
        for artifact in artifacts(source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let args = inputs(
                &artifact,
                &[("start", Data::U64(7)), ("end", Data::U64(40))],
                &memory,
            );
            let expected = reference(&artifact, &args);
            let mut output = vec![None; artifact.outputs().len()];
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            // Empty output slots have no prior ownership to retire. Prove
            // their exact boundary independently of replacing a published root.
            let mut fresh = vec![None; artifact.outputs().len()];
            let mut exact = limits();
            exact.execution.max_work = report.work;
            assert_eq!(
                jit.evaluate_into(&args, &mut fresh, exact, &memory)
                    .unwrap(),
                report
            );
            equal(&artifact, &fresh, &expected);
            drop(fresh);
            let mut fresh = vec![None; artifact.outputs().len()];
            let before = memory.used_bytes();
            exact.execution.max_work -= 1;
            let error = jit
                .evaluate_into(&args, &mut fresh, exact, &memory)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    NativeProgramExecutionError::Value(NativeValueError::Base(
                        NativeExecutionError::WorkLimit { required, limit, .. }
                    )) if required > limit && limit == report.work - 1
                ),
                "{error:?}"
            );
            assert!(fresh.iter().all(Option::is_none));
            assert_eq!(memory.used_bytes(), before);

            // The existing publication protocol prepays retirement of each
            // caller-visible root: 4 * retained bytes + 32. It is not range
            // construction work, and must not disappear with packed storage.
            let retirement = output
                .iter()
                .flatten()
                .map(|value| value.memory_budget_retained_bytes(&memory).unwrap() * 4 + 32)
                .sum::<u64>();
            let mut replacement_report = report;
            replacement_report.work += retirement;
            exact.execution.max_work = replacement_report.work;
            assert_eq!(
                jit.evaluate_into(&args, &mut output, exact, &memory)
                    .unwrap(),
                replacement_report
            );
            let prior = output.clone();
            for dimension in 0..7 {
                let mut denied = limits();
                match dimension {
                    0 => denied.execution.max_work = replacement_report.work - 1,
                    1 => denied.library.max_compute_work = Some(0),
                    2 => denied.library.max_temporary_bytes = Some(0),
                    3 => denied.library.max_retained_nodes = Some(0),
                    4 => denied.library.max_output_elements = Some(32),
                    5 => denied.library.max_output_bytes = Some(0),
                    _ => denied.library.max_cloned_bytes = Some(0),
                }
                let before = memory.used_bytes();
                let error = jit
                    .evaluate_into(&args, &mut output, denied, &memory)
                    .expect_err(&format!("dimension {dimension}"));
                if dimension == 0 {
                    assert!(
                        matches!(
                            error,
                            NativeProgramExecutionError::Value(NativeValueError::Base(
                                NativeExecutionError::WorkLimit { required, limit, .. }
                            )) if required > limit && limit == replacement_report.work - 1
                        ),
                        "{error:?}"
                    );
                }
                assert_eq!(memory.used_bytes(), before);
                for (actual, previous) in output.iter().zip(&prior) {
                    assert!(
                        actual
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(previous.as_ref().unwrap())
                    );
                }
            }
            // A truly shared account with no remaining capacity cannot let a
            // smaller physical constructor bypass the original admission.
            let claim = memory
                .reserve_capacity(256 * 1024 * 1024 - memory.used_bytes())
                .unwrap();
            let before = memory.used_bytes();
            assert!(
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), before);
            equal(&artifact, &output, &expected);
            drop(claim);
            assert_eq!(
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap(),
                replacement_report
            );
            equal(&artifact, &output, &expected);
            drop((args, output, prior, expected));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
