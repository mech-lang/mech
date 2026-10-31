//! Independent integer oracles for real resident and native word execution.
#![cfg(feature = "canonical-native")]
use mech_core::snapshot::{SnapshotValidationContext, ValueDraft};
use mech_core::{ReactiveInstanceId, ValueData, ValueDataDraft};
use mech_engine::native_artifact::{NativeScalarOp, NativeScalarValue, lower_pure_native_artifact};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{JitArtifact, NativeExecutionLimits};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

struct Case {
    item: &'static str,
    op: NativeScalarOp,
    unary: bool,
    expected: fn(u64, u64) -> u64,
}
fn cases() -> [Case; 11] {
    use NativeScalarOp::*;
    [
        Case {
            item: "and",
            op: U64BitAnd,
            unary: false,
            expected: |a, b| a & b,
        },
        Case {
            item: "or",
            op: U64BitOr,
            unary: false,
            expected: |a, b| a | b,
        },
        Case {
            item: "xor",
            op: U64BitXor,
            unary: false,
            expected: |a, b| a ^ b,
        },
        Case {
            item: "not",
            op: U64BitNot,
            unary: true,
            expected: |a, _| !a,
        },
        Case {
            item: "shift-left-wrap",
            op: U64ShlWrap,
            unary: false,
            expected: |a, b| a.wrapping_shl((b % 64) as u32),
        },
        Case {
            item: "shift-right-wrap",
            op: U64ShrWrap,
            unary: false,
            expected: |a, b| a.wrapping_shr((b % 64) as u32),
        },
        Case {
            item: "leading-zeros",
            op: U64Clz,
            unary: true,
            expected: |a, _| u64::from(a.leading_zeros()),
        },
        Case {
            item: "trailing-zeros",
            op: U64Ctz,
            unary: true,
            expected: |a, _| u64::from(a.trailing_zeros()),
        },
        Case {
            item: "wrapping-add",
            op: U64WrappingAdd,
            unary: false,
            expected: u64::wrapping_add,
        },
        Case {
            item: "wrapping-sub",
            op: U64WrappingSub,
            unary: false,
            expected: u64::wrapping_sub,
        },
        Case {
            item: "wrapping-mul",
            op: U64WrappingMul,
            unary: false,
            expected: u64::wrapping_mul,
        },
    ]
}
fn compile(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x42495453), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    [original, decoded]
}
fn samples() -> Vec<(u64, u64)> {
    let mut samples = Vec::new();
    for lhs in [0, 1, 0x8000_0000_0000_0000, u64::MAX, 0x1234_5678_9abc_def0] {
        for rhs in [0, 1, 31, 32, 63, 64, 65, 0x1_0000_0001, u64::MAX] {
            samples.push((lhs, rhs));
        }
    }
    let mut seed = 0x1280_9abc_ef01_3456u64;
    for _ in 0..64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let lhs = seed;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        samples.push((lhs, seed));
    }
    samples
}

#[test]
fn word_primitives_match_integer_oracles_resident_and_jit_original_and_decoded() {
    for case in cases() {
        let arguments = if case.unary {
            "lhs<u64>"
        } else {
            "lhs<u64>, rhs<u64>"
        };
        let source = format!("+> bits\nbits/{}({arguments})\n", case.item);
        for artifact in compile(&source) {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.lets().len(), 1);
            assert_eq!(plan.lets()[0].op(), case.op);
            let jit = JitArtifact::compile(&plan).unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x42495453, 0),
                &artifact,
                &mech_stdlib::source_catalog(),
                &ActivationFacts::default(),
            )
            .unwrap();
            for (lhs, rhs) in samples() {
                let values = artifact
                    .inputs()
                    .iter()
                    .map(|input| {
                        let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                        let word = match name.as_str() {
                            "lhs" => lhs,
                            "rhs" => rhs,
                            _ => panic!("{name}"),
                        };
                        ValueDraft {
                            schema: input.schema,
                            shape_values: Box::new([]),
                            data: ValueDataDraft::U64(word),
                        }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                        .unwrap()
                    })
                    .collect::<Vec<_>>();
                let captured = instance
                    .plan
                    .inputs
                    .iter()
                    .map(|input| {
                        let index = artifact
                            .inputs()
                            .iter()
                            .position(|decl| decl.slot == input.artifact_slot)
                            .unwrap();
                        CapturedValueInput {
                            slot: input.slot,
                            value: &values[index],
                        }
                    })
                    .collect::<Vec<_>>();
                instance
                    .prepare_turn_values(&captured)
                    .unwrap()
                    .publish()
                    .unwrap();
                let expected = (case.expected)(lhs, rhs);
                let input = values
                    .iter()
                    .map(|value| {
                        let ValueData::U64(value) = value.data() else {
                            panic!()
                        };
                        NativeScalarValue::U64(*value)
                    })
                    .collect::<Vec<_>>();
                let mut outputs = vec![NativeScalarValue::U64(0xfeed); plan.outputs().len()];
                jit.evaluate_into(&input, &mut outputs, NativeExecutionLimits::default(), None)
                    .unwrap();
                for (index, output) in outputs.iter().enumerate() {
                    assert_eq!(
                        *output,
                        NativeScalarValue::U64(expected),
                        "{} ({lhs}, {rhs})",
                        case.item
                    );
                    assert_eq!(
                        instance
                            .copied_output(index)
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        ValueDataDraft::U64(expected),
                        "{} ({lhs}, {rhs})",
                        case.item
                    );
                }
            }
        }
    }
}

#[test]
fn libdeflate_hash_prefix_executes_as_native_word_operations() {
    let source = "+> bits\nproduct := bits/wrapping-mul(prefix<u64>, 506832829u64)\nword := bits/and(product, 4294967295u64)\nbits/shift-right-wrap(word, 32u64 - hash-bits<u64>)\n";
    for artifact in compile(source) {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitArtifact::compile(&plan).unwrap();
        for prefix in [0, 1, 0x616263, 0xffff_ffff] {
            for bits in [1, 8, 15, 16, 32] {
                let args = artifact
                    .inputs()
                    .iter()
                    .map(|input| {
                        match mech_engine::decode_source_input_name(&input.name).as_deref() {
                            Some("prefix") => NativeScalarValue::U64(prefix),
                            Some("hash-bits") => NativeScalarValue::U64(bits),
                            other => panic!("{other:?}"),
                        }
                    })
                    .collect::<Vec<_>>();
                let mut output = vec![NativeScalarValue::U64(0); plan.outputs().len()];
                jit.evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None)
                    .unwrap();
                let expected = u64::from((prefix as u32).wrapping_mul(0x1e35_a7bd)) >> (32 - bits);
                assert_eq!(output.last(), Some(&NativeScalarValue::U64(expected)));
            }
        }
    }
}

#[test]
fn a_same_named_replacement_cannot_obtain_native_bitwise_semantics() {
    fn fake(
        _: &mech_core::ResidentKernelBindRequest<'_>,
    ) -> Result<mech_core::BoundResidentKernel, mech_core::ResidentKernelBindError> {
        Ok(mech_core::BoundResidentKernel::new(
            |_, _, _| panic!("metadata validation must never execute the replacement"),
            Box::new([]),
        )
        .with_work_accounting(mech_core::ResidentKernelWorkAccounting::RuntimeMetered))
    }
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["bits"],
            "xor",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            fake,
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    for artifact in compile("+> bits\nbits/xor(lhs<u64>, rhs<u64>)\n") {
        assert!(matches!(
            lower_pure_native_artifact(&artifact, &catalog),
            Err(
                mech_engine::native_artifact::NativeArtifactLoweringError::MissingSemanticWitness { .. }
            )
        ));
    }
}
