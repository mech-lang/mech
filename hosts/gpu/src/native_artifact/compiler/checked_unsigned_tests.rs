//! Checked unsigned arithmetic should branch on the word comparison directly.
use super::{JitArtifact, NativeExecutionError, NativeExecutionLimits, NativeScalarOp};
use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn assembly(op: NativeScalarOp) -> String {
    let mut module = super::codegen_backend::jit(super::codegen_backend::SCALAR_FLAGS).unwrap();
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    context.set_disasm(true);
    let mut frontend = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        let error = b.create_block();
        b.append_block_param(error, types::I64);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let value = super::primitive(&mut b, op, &args, error);
        b.ins().return_(&[value]);
        b.switch_to_block(error);
        let status = b.block_params(error)[0];
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        b.finalize();
    }
    let id = module
        .declare_function("checked_unsigned", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    context.compiled_code().unwrap().vcode.clone().unwrap()
}

#[test]
fn checked_unsigned_branches_do_not_materialize_carry_or_borrow_booleans() {
    for op in [NativeScalarOp::U64Add, NativeScalarOp::U64Sub] {
        let assembly = assembly(op);
        if cfg!(target_arch = "aarch64") {
            assert!(
                !assembly.lines().any(|line| line.contains("cset ")),
                "{op:?} still materializes a carry/borrow Boolean:\n{assembly}"
            );
        }
    }
}

fn source(op: &str) -> ProgramArtifact {
    let text = format!("result := left<u64> {op} right<u64>\nresult\n");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x55475541), Revision(0), text.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}

#[test]
fn checked_unsigned_words_match_rust_boundaries_and_preserve_failed_publication() {
    use mech_engine::native_artifact::NativeScalarValue as Word;
    let values = [0, 1, 2, u64::MAX / 2, 1u64 << 63, u64::MAX - 1, u64::MAX];
    for op in ["+", "-"] {
        let original = source(op);
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
        for artifact in [&original, &decoded] {
            let plan = mech_engine::native_artifact::lower_pure_native_artifact(
                artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let jit = JitArtifact::compile(&plan).unwrap();
            let arguments = |left, right| {
                artifact
                    .inputs()
                    .iter()
                    .map(|port| {
                        let name = mech_engine::decode_source_input_name(&port.name).unwrap();
                        Word::U64(match name.as_str() {
                            "left" => left,
                            "right" => right,
                            _ => panic!("unexpected input {name}"),
                        })
                    })
                    .collect::<Vec<_>>()
            };
            let mut outputs = vec![Word::U64(0xbeef); jit.descriptor().outputs().len()];
            for left in values {
                for right in values {
                    let expected = if op == "+" {
                        left.checked_add(right)
                    } else {
                        left.checked_sub(right)
                    };
                    let before = outputs.clone();
                    let result = jit.evaluate_into(
                        &arguments(left, right),
                        &mut outputs,
                        NativeExecutionLimits::default(),
                        None,
                    );
                    match expected {
                        Some(expected) => {
                            result.unwrap();
                            assert!(outputs.iter().all(|word| *word == Word::U64(expected)));
                        }
                        None => {
                            assert!(matches!(
                                result,
                                Err(NativeExecutionError::Arithmetic { .. })
                            ));
                            assert_eq!(outputs, before);
                        }
                    }
                }
            }
            let before = outputs.clone();
            let inputs = arguments(if op == "+" { u64::MAX } else { 0 }, 1);
            assert!(matches!(
                jit.evaluate_into(
                    &inputs,
                    &mut outputs,
                    NativeExecutionLimits {
                        max_work: 0,
                        ..NativeExecutionLimits::default()
                    },
                    None,
                ),
                Err(NativeExecutionError::WorkLimit { .. })
            ));
            assert_eq!(outputs, before);
            jit.evaluate_into(
                &arguments(7, 3),
                &mut outputs,
                NativeExecutionLimits::default(),
                None,
            )
            .unwrap();
            assert!(
                outputs
                    .iter()
                    .all(|word| *word == Word::U64(if op == "+" { 10 } else { 4 }))
            );
        }
    }
}
