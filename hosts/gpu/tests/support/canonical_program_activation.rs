//! Closed pure activation uses the same generated CFG, never a compiler evaluator.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramExecutionError,
    emit_program_object,
};

#[test]
fn closed_operations_matches_loops_and_comprehensions_precede_each_fresh_turn() {
    // Deliberately interleave an input-dependent root before closed roots.
    // Activation dependencies must be ordered by the canonical schedule, not
    // artifact ordinal, and their outputs must feed ordinary live operations.
    let source = format!(
        "{}live := windows(input<string>, width<u64>)\nfixed := windows(\"ABBA\", 2u64)\nstep(x<u64>) => (out<u64>) := out := x + 1u64.\nclosed-loop := 1u64 |3> step\nclosed-match := (true? | true => 6u64 / 2u64 | false => 1u64 / 0u64)\nclosed-range := 1u64..2u64..=5u64\nlive-sum := closed-loop + width\n",
        super::window_qualification::WINDOWS_FUNCTION
    );
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut work = Vec::new();
        for (text, width) in [("ABBA", 2), ("", 0), ("é🧑‍🚀é", 2), ("ABBA", 2)] {
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::String(text.into())),
                    ("width", Data::U64(width)),
                ],
                &memory,
            );
            let expected = reference(&artifact, &args);
            // Establish the same prior publication before comparing work:
            // replacing a larger prior root legitimately costs more than
            // replacing the just-produced root, independently of activation.
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            let prior = output
                .iter()
                .map(|v| v.clone().unwrap())
                .collect::<Vec<_>>();
            let before = memory.used_bytes();
            let mut denied = limits();
            denied.library.max_compute_work = Some(0);
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err()
            );
            equal(&artifact, &output, &prior);
            assert_eq!(memory.used_bytes(), before);
            // No partial activation cache survives a denied invocation.
            let recovered = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            assert_eq!(recovered.work, report.work);
            work.push(report.work);
        }
        // Every call is fresh, including its closed computation admission.
        assert_eq!(work[0], work[3]);
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn closed_failure_is_not_run_by_compilation_and_precedes_an_earlier_turn_failure() {
    let source = "live := 1u64 / divisor<u64>\nclosed := 1u64 / 0u64\n";
    for artifact in artifacts(source).0 {
        // If metadata selection executed values, neither construction could
        // succeed. Both must emit the failing operation into native code.
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let closed = artifact.nodes().last().unwrap().node;
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for divisor in [0, 1, 0] {
            let args = inputs(&artifact, &[("divisor", Data::U64(divisor))], &memory);
            let resident = reference_result(&artifact, &args).unwrap_err();
            assert!(resident.contains("ActivationKernel"), "{resident}");
            assert!(resident.contains(&format!("{closed:?}")), "{resident}");
            let mut output = vec![Some(args[0].clone()); artifact.outputs().len()];
            let prior = output
                .iter()
                .map(|v| v.clone().unwrap())
                .collect::<Vec<_>>();
            let before = memory.used_bytes();
            assert!(matches!(
                jit.evaluate_into(&args, &mut output, limits(), &memory),
                Err(NativeProgramExecutionError::Control(NativeIterationExecutionError::Arithmetic { origin })) if origin.node() == closed
            ));
            equal(&artifact, &output, &prior);
            assert_eq!(memory.used_bytes(), before);
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}
