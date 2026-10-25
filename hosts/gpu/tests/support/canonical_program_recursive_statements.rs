//! Statement bodies use fresh parameter bindings on every recursive visit.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn recursive_statement_locals_use_the_current_scalar_and_tuple_arguments() {
    for source in [
        "count(n<u64>) => (out<u64>) :=\n  next := ((n > 0u64)? | true => n - 1u64 | false => 0u64)\n  out := ((n > 0u64)? | true => count(next) | false => 17u64).\ncount(input<u64>)\n",
        "count(n<u64>, total<u64>) => (out<u64>) :=\n  next := ((n > 0u64)? | true => n - 1u64 | false => 0u64)\n  sum := total + n\n  out := ((n > 0u64)? | true => count(next, sum) | false => total).\ncount(input<u64>, 17u64)\n",
    ] {
        for artifact in artifacts(source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for n in [0, 1, 7, 2, 0] {
                let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
                let expected = if source.contains("total<u64>") {
                    17 + n * (n + 1) / 2
                } else {
                    17
                };
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(expected)
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn recursive_statement_arrays_keep_their_shapes_and_fresh_mutable_locals() {
    let source = "fill(xs<[u16]>, n<u64>) => (out<[u16]>) :=\n  wide<u64> := xs[1u64]\n  ~next := xs\n  next[1u64] = wide + n\n  left := ((n > 0u64)? | true => n - 1u64 | false => 0u64)\n  out := ((n > 0u64)? | true => fill(next, left) | false => xs).\nfill(input<[u16]>, count<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (size, n) in [(1, 0), (3, 4), (2, 1), (4, 2), (1, 0)] {
            let data = Data::Matrix(vec![Data::U16(31); size].into());
            let args = inputs(
                &artifact,
                &[("input", data.clone()), ("count", Data::U64(n))],
                &memory,
            );
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            let mut expected = vec![Data::U16(31); size];
            expected[0] = Data::U16((31 + n * (n + 1) / 2) as u16);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(expected.into())
            );
            let index = artifact
                .inputs()
                .iter()
                .position(|input| {
                    mech_engine::decode_source_input_name(&input.name).unwrap() == "input"
                })
                .unwrap();
            assert_eq!(args[index].canonical_data_draft().unwrap(), data);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn recursive_statement_late_failure_and_work_denial_preserve_output_and_recover() {
    let source = "count(n<u64>, total<u64>) => (out<u64>) :=\n  divisor := ((n == 0u64)? | true => 1u64 | false => n % 3u64)\n  unused := 12u64 / divisor\n  left := ((n > 0u64)? | true => n - 1u64 | false => 0u64)\n  out := ((n > 0u64)? | true => count(left, total + n) | false => total).\ncount(input<u64>, 11u64)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0, 2, 5, 1] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
            if n == 5 {
                assert!(reference_result(&artifact, &args).is_err());
                let prior = output
                    .iter()
                    .map(|value| value.clone().unwrap())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                let error = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap_err();
                assert!(
                    matches!(
                        error,
                        mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                            mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
                        )
                    ),
                    "{error:?}"
                );
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
            }
        }
        let args = inputs(&artifact, &[("input", Data::U64(2))], &memory);
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut policy = limits();
        policy.execution.max_work = 1;
        assert!(
            jit.evaluate_into(&args, &mut output, policy, &memory)
                .is_err()
        );
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &args));
        drop((args, prior, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn non_tail_statement_recursion_is_canonical_but_still_rejects_native_lowering() {
    let source = "sum(n<u64>) => (out<u64>) :=\n  out := ((n > 0u64)? | true => sum(n - 1u64) + n | false => 0u64).\nsum(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, &[("input", Data::U64(4))], &memory);
        assert_eq!(
            reference(&artifact, &args)
                .last()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::U64(10)
        );
        assert!(JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).is_err());
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn recursive_statement_named_outputs_share_the_canonical_result_tuple() {
    let source = "pair(n<u64>) => (left<u64>, right<u64>) :=\n  result := (n? | 0u64 => (17u64, 23u64) | * => pair(n - 1u64))\n  (left, right) := result.\npair(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for n in [0, 4, 1] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
            assert_eq!(
                reference(&artifact, &args)
                    .last()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(vec![Data::U64(17), Data::U64(23)].into())
            );
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn recursive_statement_invariants_cannot_escape_as_document_constraints() {
    use mech_engine::CanonicalSourceFrontend;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    let source = "count(n<u64>) => (out<u64>) :=\n  valid! := n > 0u64\n  out := ((n > 0u64)? | true => count(n - 1u64) | false => 7u64).\ncount(input<u64>)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0xB7), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let Err(error) = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
    else {
        panic!("a visit-local constraint must not become a root declaration")
    };
    assert_eq!(error.code, "source-semantics/recursive-function-invariant");
}
