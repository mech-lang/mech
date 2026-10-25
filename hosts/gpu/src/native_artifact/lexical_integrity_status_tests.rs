//! A compiled arithmetic failure cannot be relabeled as a lexical assertion.
use super::*;
use mech_core::{ManagedMemoryBudget, SchemaBody};
#[cfg(feature = "canonical-native-program")]
use mech_core::{ValueDataDraft, ValueDraft};
use mech_engine::native_artifact::{
    NativeScalarValue as Word, lower_scalar_iteration_artifact, lower_scalar_match_artifact,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::cell::Cell;

fn artifact() -> mech_engine::ProgramArtifact {
    let source = "flag<bool> ? | true => 1u64 / denominator<u64> | false => 17u64\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0xA55E47), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}

fn words(artifact: &mech_engine::ProgramArtifact) -> Vec<Word> {
    artifact
        .inputs()
        .iter()
        .map(
            |input| match artifact.schemas().get(input.schema).unwrap().body() {
                SchemaBody::Bool => Word::Bool(true),
                SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64) => Word::U64(0),
                _ => panic!("unexpected fixture input"),
            },
        )
        .collect()
}

#[test]
fn match_failure_classes_require_their_own_admitted_terminal() {
    let artifact = artifact();
    let plan = lower_scalar_match_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let jit = JitMatchArtifact::compile(&plan).unwrap();
    let entry = jit.test_checked_entry();
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let mut output = vec![Word::U64(99); plan.outputs().len()];
    for forged in [abi::STATUS_INTEGRITY, control_abi::STATUS_REJECT] {
        let mapped = Cell::new(false);
        let error = control_runtime::execute(
            jit.descriptor(),
            |frame| {
                // The actual checked JIT sets the original admitted arithmetic site.
                let status = unsafe { entry(frame) };
                assert_eq!(status, abi::STATUS_ARITHMETIC);
                mapped.set(true);
                forged
            },
            &words(&artifact),
            &mut output,
            NativeExecutionLimits::default(),
            Some(&memory),
        )
        .unwrap_err();
        assert!(mapped.get());
        assert!(
            matches!(
                error,
                NativeMatchExecutionError::Base(NativeExecutionError::Descriptor { .. })
            ),
            "{error:?}"
        );
        assert_eq!(output, vec![Word::U64(99); output.len()]);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn iteration_integrity_status_cannot_replace_an_arithmetic_failure() {
    let artifact = artifact();
    let plan = lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let jit = JitIterationArtifact::compile(&plan).unwrap();
    let entry = jit.test_checked_entry();
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let mut output = vec![Word::U64(99); plan.outputs().len()];
    let error = iteration_runtime::execute(
        jit.descriptor(),
        |frame| {
            let status = unsafe { entry(frame) };
            assert_eq!(status, abi::STATUS_ARITHMETIC);
            abi::STATUS_INTEGRITY
        },
        &words(&artifact),
        &mut output,
        NativeExecutionLimits::default(),
        Some(&memory),
    )
    .unwrap_err();
    assert!(
        matches!(
            error,
            NativeIterationExecutionError::Base(NativeExecutionError::Descriptor { .. })
        ),
        "{error:?}"
    );
    assert_eq!(output, vec![Word::U64(99); output.len()]);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
#[cfg(feature = "canonical-native-program")]
fn program_integrity_status_cannot_replace_an_arithmetic_failure() {
    let artifact = artifact();
    let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let entry = jit.test_checked_entry();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let inputs = artifact
        .inputs()
        .iter()
        .zip(words(&artifact))
        .map(|(input, word)| {
            let data = match word {
                Word::Bool(v) => ValueDataDraft::Bool(v),
                Word::U64(v) => ValueDataDraft::U64(v),
                _ => unreachable!(),
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                artifact.schemas(),
            ))
            .unwrap();
            let mut capacity = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut capacity).unwrap()
        })
        .collect::<Vec<_>>();
    let before = memory.used_bytes();
    let mut output = vec![None; artifact.outputs().len()];
    let mapped = Cell::new(false);
    let error = program_runtime::execute(
        jit.descriptor(),
        |frame| {
            let status = unsafe { entry(frame) };
            if status == abi::STATUS_ARITHMETIC {
                mapped.set(true);
                abi::STATUS_INTEGRITY
            } else {
                status
            }
        },
        &inputs,
        &mut output,
        NativeProgramExecutionLimits {
            execution: NativeExecutionLimits::default(),
            library: mech_core::TargetMemoryProfile::current_resident_cpu()
                .unwrap()
                .limits,
        },
        &memory,
    )
    .unwrap_err();
    assert!(mapped.get());
    assert!(
        matches!(
            error,
            NativeProgramExecutionError::Base(NativeExecutionError::Descriptor { .. })
        ),
        "{error:?}"
    );
    assert!(output.iter().all(Option::is_none));
    assert_eq!(memory.used_bytes(), before);
    drop(inputs);
    assert_eq!(memory.used_bytes(), 0);
}
