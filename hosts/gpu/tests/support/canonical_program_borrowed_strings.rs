//! Selection proof only; native view execution is a separate qualification.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeBorrowedStringAction as View, lower_native_program_artifact,
};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

fn scan(body: &str) -> String {
    format!(
        "+> sequence\n#Scan(lines<[string]>, n<u64>) => <u64>\n  | :Step(i<u64>, found<u64>)\n  | :Done(found<u64>).\n#Scan(lines, n) -> :Step(1u64, 0u64)\n  :Step(i, found)\n    | i > n -> :Done(found)\n    | {body} -> :Step(i + 1u64, found + 1u64)\n    | * -> :Step(i + 1u64, found)\n  :Done(found) => found.\nxs := input<[string]>\n#Scan(xs, sequence/length(xs))\n"
    )
}
#[test]
fn borrowed_string_proof_uses_selected_operations_and_rejects_escaping_strings() {
    for (body, expected) in [
        ("sequence/length(lines[i]) > 3u64", true),
        ("lines[i][1u64] == \"x\"", true),
        ("lines[i][1u64..=3u64] == \"abc\"", true),
        ("lines[i] + \"!\" == \"x!\"", false),
        ("lines[i] == \"🧑‍🚀\"", false),
    ] {
        let source = scan(body);
        for artifact in artifacts(&source).0 {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let regions = plan.borrowed_string_regions();
            assert_eq!(!regions.is_empty(), expected, "{source}");
            for region in regions {
                assert_eq!(region.matrices().len(), 1);
                assert!(
                    region
                        .actions()
                        .iter()
                        .any(|(_, a)| matches!(a, View::MatrixElement { .. }))
                );
                assert!(!region.values().is_empty());
                for output in plan.outputs() {
                    assert!(!region.values().contains(&output.value()));
                }
            }
        }
    }
}

#[test]
fn complete_idiomatic_record_scan_has_a_closed_borrowed_string_component() {
    let fixture = include_str!("../fixtures/native/knucleotide-idiomatic.mec");
    let mut source = fixture
        .lines()
        .filter(|line| !line.starts_with("+> @"))
        .map(|line| {
            line.replace("@in/text", "input<string>")
                .replace("@out/text <- ", "delivery := ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    source.push_str("\ndelivery\n");
    for artifact in artifacts(&source).0 {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let regions = plan.borrowed_string_regions();
        assert_eq!(
            regions.len(),
            1,
            "actual canonical record FSM must be recognized"
        );
        assert_eq!(
            plan.selected_text_joins().len(),
            1,
            "the actual selected record must have one nonescaping join consumer"
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.metadata()["closed_tail_regions"],
            1,
            "the complete record FSM must eliminate per-operation control callbacks"
        );
        let region = &regions[0];
        assert_eq!(region.matrices().len(), 1);
        assert_eq!(
            region
                .actions()
                .iter()
                .filter(|(_, action)| matches!(action, View::ConstantRange { .. }))
                .count(),
            1,
            "the repeated private prefix selector must not allocate a Matrix"
        );
        for wanted in ["matrix", "length", "scalar", "range", "equal"] {
            assert!(
                region.actions().iter().any(|(_, a)| match (wanted, a) {
                    ("matrix", View::MatrixElement { .. })
                    | ("length", View::Length { .. })
                    | ("scalar", View::ScalarSelect { .. })
                    | ("range", View::FixedRange { .. })
                    | ("equal", View::Equal { .. }) => true,
                    _ => false,
                }),
                "missing {wanted}: {region:?}"
            );
        }
    }
}

#[test]
fn constant_selector_elision_requires_its_complete_semantic_use_set() {
    for (body, expected) in [
        ("found := line[r] == \"abc\".", 1),
        (
            "n := sequence/length(r)\n  found := (n? | 3u64 => line[r] == \"abc\" | * => false).",
            0,
        ),
    ] {
        let source = scan("inspect(lines[i])").replacen("+> sequence\n", &format!(
            "+> sequence\ninspect(line<string>) => (found<bool>) :=\n  r := 1u64..=3u64\n  {body}\n"), 1);
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let regions = jit.descriptor().plan().borrowed_string_regions();
            assert_eq!(regions.len(), 1, "String view proof must still apply");
            assert_eq!(
                regions[0]
                    .actions()
                    .iter()
                    .filter(|(_, action)| matches!(action, View::ConstantRange { .. }))
                    .count(),
                expected,
                "{source}"
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            for text in ["abc", "xyz", "🧑‍🚀ab", "\r\nab"] {
                let args = inputs(
                    &artifact,
                    &[(
                        "input",
                        Data::Matrix(vec![Data::String(text.into())].into()),
                    )],
                    &memory,
                );
                let ordinary = reference(&artifact, &args);
                let mut output = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
            }
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn native_string_views_match_canonical_ascii_unicode_and_crlf_results() {
    for body in [
        "sequence/length(lines[i]) > 3u64",
        "lines[i][1u64] == \"x\"",
        "lines[i][1u64..=3u64] == \"abc\"",
        "lines[i] != \"abc\"",
    ] {
        let source = scan(body);
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(object.metadata()["borrowed_string_regions"], 1);
            assert_eq!(
                object.descriptor().typed_ir_sha256(),
                jit.descriptor().typed_ir_sha256()
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            for strings in [
                vec![],
                vec!["abc", "xyz", "abcd", "xxx"],
                vec!["🧑‍🚀ab", "e\u{301}ab", "abc"],
                vec!["\r\nab", "abc"],
            ] {
                let args = inputs(
                    &artifact,
                    &[(
                        "input",
                        Data::Matrix(strings.iter().map(|s| Data::String((*s).into())).collect()),
                    )],
                    &memory,
                );
                let ordinary = reference(&artifact, &args);
                let mut output = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
            }
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn native_string_bounds_fail_atomically_and_keep_the_selected_leaf_origin() {
    for body in [
        "lines[i][0u64] == \"x\"",
        "lines[i][4u64] == \"x\"",
        "lines[i][1u64..=4u64] == \"xxxx\"",
        "lines[i][18446744073709551614u64..=18446744073709551615u64] == \"xx\"",
        "lines[i + 1u64] == \"x\"",
    ] {
        for artifact in artifacts(&scan(body)).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(jit.descriptor().plan().borrowed_string_regions().len(), 1);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[(
                    "input",
                    Data::Matrix(vec![Data::String("abc".into())].into()),
                )],
                &memory,
            );
            assert!(reference_result(&artifact, &args).is_err());
            let mut output = vec![None; artifact.outputs().len()];
            let before = memory.used_bytes();
            let error = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap_err();
            let mech_gpu::native_artifact::NativeProgramExecutionError::Library { origin, .. } =
                error
            else {
                panic!("{error:?}")
            };
            assert!(
                jit.descriptor().plan().borrowed_string_regions()[0]
                    .actions()
                    .iter()
                    .any(|(site, action)| matches!(
                        action,
                        View::MatrixElement { .. }
                            | View::ScalarSelect { .. }
                            | View::FixedRange { .. }
                    ) && jit.descriptor().plan().blocks()[*site as usize]
                        .origin()
                        == origin)
            );
            assert!(output.iter().all(Option::is_none));
            assert_eq!(memory.used_bytes(), before);
            let empty = inputs(&artifact, &[("input", Data::Matrix(Box::new([])))], &memory);
            jit.evaluate_into(&empty, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &empty));
        }
    }
}

#[test]
fn borrowed_string_comparison_denial_preserves_outputs_and_recovers() {
    for artifact in artifacts(&scan("lines[i] == \"abc\"")).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[(
                "input",
                Data::Matrix(vec![Data::String("abc".into()), Data::String("xyz".into())].into()),
            )],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut denied = limits();
        // Let the preceding canonical Matrix length bind, then deny the first
        // generated <=32-byte equality at its own selected origin.
        denied.library.max_comparison_work = Some(31);
        let error = jit
            .evaluate_into(&args, &mut output, denied, &memory)
            .unwrap_err();
        assert!(
            matches!(
                &error,
                mech_gpu::native_artifact::NativeProgramExecutionError::Collection {
                    origin,
                    error: mech_core::ResidentKernelError::ResourceLimit {
                        dimension: mech_core::MemoryBudgetDimension::ComparisonWork,
                        ..
                    },
                    ..
                } if jit.descriptor().plan().borrowed_string_regions()[0].actions().iter().any(|(site, action)|
                    matches!(action, View::Equal { .. }) && jit.descriptor().plan().blocks()[*site as usize].origin() == *origin)
            ),
            "{error:?}"
        );
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
    }
}
