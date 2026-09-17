//! Exact source/decoded acceptance for recovery finding G02's canonical
//! numeric target families.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use mech_core::ReactiveInstanceId;
use mech_engine::ProgramArtifact;
use mech_engine::resident::{ActivationFacts, activate};
use mech_runtime::{RuntimeBuilder, RuntimeValueSnapshot, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision};

fn compile(source: &str) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(
        "s8-recovery-numeric-targets.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(document.is_strictly_clean());
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap()
        .compile_document(&document)
        .unwrap()
        .artifact()
        .clone()
}

fn exact_outputs(artifact: &ProgramArtifact, expected: &[&str]) {
    let catalog = mech_stdlib::source_catalog();
    let mut instance = activate(
        ReactiveInstanceId::new(0x58c, 2),
        artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    for expected in expected {
        instance.turn(&[]).unwrap();
        let output = RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap())
            .unwrap()
            .format_canonical_inline();
        assert_eq!(&output, expected);
    }
}

fn source_and_decoded(source: &str, expected: &[&str]) {
    let artifact = compile(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    exact_outputs(&artifact, expected);
    exact_outputs(&decoded, expected);
}

#[test]
fn canonical_power_family_executes_source_and_decoded() {
    for kind in [
        "u64", "u128", "i8", "i16", "i32", "i64", "i128", "c32", "c64", "r64",
    ] {
        source_and_decoded(
            &format!("answer := 2<{kind}> ^ 2<{kind}>\nanswer == 4<{kind}>\n"),
            &["true", "true"],
        );
    }
}

#[test]
fn canonical_c32_arithmetic_and_updates_execute_source_and_decoded() {
    for (source, expected) in [
        (
            "~a := 1<c32>\na += 1<c32>\na == 2<c32>\n",
            &["true", "false"][..],
        ),
        (
            "answer := 6<c32> + 2<c32>\nanswer == 8<c32>\n",
            &["true", "true"],
        ),
        (
            "answer := 6<c32> - 2<c32>\nanswer == 4<c32>\n",
            &["true", "true"],
        ),
        (
            "answer := 6<c32> * 2<c32>\nanswer == 12<c32>\n",
            &["true", "true"],
        ),
        (
            "answer := 6<c32> / 2<c32>\nanswer == 3<c32>\n",
            &["true", "true"],
        ),
        (
            "~a := [10<c32> 20<c32>; 30<c32> 40<c32>]\na[[1 1],:] += 2<c32>\na[1,1] == 14<c32>\n",
            &["true", "false"],
        ),
    ] {
        source_and_decoded(source, expected);
    }

    for (operation, expected) in [("+", 6), ("-", 2), ("*", 8), ("/", 2)] {
        source_and_decoded(
            &format!(
                "a := [1<c32> 2<c32>; 3<c32> 4<c32>]\nanswer := a {operation} 2<c32>\nanswer[2,2] == {expected}<c32>\n"
            ),
            &["true", "true"],
        );
    }
}

#[test]
fn canonical_complex_and_rational_matrix_ops_execute_source_and_decoded() {
    source_and_decoded(
        "+> stats\na := [1<c32> 2<c32>; 3<c32> 4<c32>]\nanswer := stats/sum/row(a)\nanswer[2] == 6<c32>\n",
        &["true", "true"],
    );

    for kind in ["c32", "c64", "r64"] {
        source_and_decoded(
            &format!(
                "a := [1<{kind}> 2<{kind}>; 3<{kind}> 4<{kind}>]\nanswer := matrix/matmul(a,a)\nanswer[2,2] == 22<{kind}>\n"
            ),
            &["true", "true"],
        );
    }
}

#[test]
fn canonical_f32_special_binary_broadcast_remains_executable() {
    source_and_decoded(
        "+> math\na := [1f32 2f32;3f32 4f32]\nanswer := math/copysign(a,-1f32)\nanswer\n",
        &["[-1 -2; -3 -4]", "[-1 -2; -3 -4]"],
    );
}
