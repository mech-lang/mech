//! Real generated cleanup keeps current elements/captures and atomic outputs.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionError, NativeIterationExecutionError,
    NativeProgramExecutionError as Error, NativeValueError, emit_program_object,
};

fn work_denial(error: &Error) -> bool {
    matches!(
        error,
        Error::Base(NativeExecutionError::WorkLimit { .. })
            | Error::Control(NativeIterationExecutionError::Work { .. })
            | Error::Control(NativeIterationExecutionError::Base(
                NativeExecutionError::WorkLimit { .. }
            ))
            | Error::Value(NativeValueError::Base(
                NativeExecutionError::WorkLimit { .. }
            ))
            | Error::Collection {
                error: mech_core::ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                    ..
                },
                ..
            }
            | Error::Library {
                error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                    mech_core::ResidentKernelError::ResourceLimit {
                        dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                        ..
                    }
                ),
                ..
            }
    )
}

#[test]
fn table_cleanup_preserves_nested_yields_later_imports_and_failure_recovery() {
    let source = "+> text\ncaptured := prefix<string>\ncollected := [\n  text/join([captured + item | item <- values], outer)\n  | outer <- values<[string]>, keep<bool>\n]\nsuffix<string> + text/join(collected, \"/\")\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let named = |words: &[&str], keep| {
            [
                ("prefix", Data::String("P".into())),
                ("suffix", Data::String("Z".into())),
                (
                    "values",
                    Data::Matrix(
                        words
                            .iter()
                            .map(|word| Data::String((*word).into()))
                            .collect(),
                    ),
                ),
                ("keep", Data::Bool(keep)),
            ]
        };
        let mut output = vec![None; artifact.outputs().len()];
        for (words, keep, expected) in [
            (&["A", "é"][..], true, "ZPAAPé/PAéPé"),
            (&["A", "é"][..], false, "Z"),
            (&[][..], true, "Z"),
            (&["B"][..], true, "ZPB"),
        ] {
            let args = inputs(&artifact, &named(words, keep), &memory);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::String(expected.into())
            );
        }
        let args = inputs(&artifact, &named(&["A", "é"], true), &memory);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let report = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut denied = limits();
        denied.execution.max_work = report.work - 1;
        let error = jit
            .evaluate_into(&args, &mut output, denied, &memory)
            .unwrap_err();
        assert!(work_denial(&error), "{error:?}");
        equal(&artifact, &output, &prior);
        assert_eq!(
            memory.used_bytes(),
            before,
            "cleanup denial leaked invocation storage"
        );
        let mut denied = limits();
        denied.execution.max_transport_bytes = report.transport_bytes - 1;
        assert!(matches!(
            jit.evaluate_into(&args, &mut output, denied, &memory),
            Err(Error::Base(NativeExecutionError::TransportLimit { required, limit }))
                if required > limit && required == report.transport_bytes
        ));
        equal(&artifact, &output, &prior);
        assert_eq!(
            memory.used_bytes(),
            before,
            "table admission denial leaked backing"
        );
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &args));
        drop((args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}
