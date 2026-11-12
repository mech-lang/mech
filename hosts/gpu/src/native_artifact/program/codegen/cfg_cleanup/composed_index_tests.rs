use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

#[derive(Clone, Copy)]
enum Case {
    Direct,
    IdentityMask,
    LossyMask,
    DifferentWord,
    DifferentBank,
    DifferentSubtraction,
}

fn admitted(case: Case) -> bool {
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 2];
    let mut context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut context);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [value, other] = b.block_params(entry).try_into().unwrap();
    let word = b.ins().band_imm(value, 0xffff);
    let original = if matches!(case, Case::DifferentWord) {
        b.ins().band_imm(other, 0xffff)
    } else {
        word
    };
    let masked = b.ins().band_imm(word, 0xffff_ffff);
    let reduced = b.ins().ireduce(types::I32, masked);
    let bias = b.ins().iconst(types::I32, 256);
    let diff = b.ins().isub(bias, reduced);
    let extended = b.ins().uextend(types::I64, diff);
    let selection = b.ins().ushr_imm(extended, 29);
    let subtract = if matches!(case, Case::DifferentSubtraction) {
        2
    } else {
        1
    };
    let subtract = b.ins().iconst(types::I64, subtract);
    let raw = b.ins().isub(original, subtract);
    let count = match case {
        Case::Direct => selection,
        Case::LossyMask => b.ins().band_imm(selection, 3),
        _ => b.ins().band_imm(selection, 63),
    };
    let index = b.ins().ushr(raw, count);
    b.ins().return_(&[]);
    b.seal_all_blocks();
    b.finalize();
    let mut facts = Facts::new(&function);
    let selector = super::selector(&function, &mut facts, selection).unwrap();
    assert_eq!(selector.threshold, 256);
    assert_eq!(selector.high, 7);
    let slots = if matches!(case, Case::DifferentBank) {
        128
    } else {
        256
    };
    split_index(&function, &mut facts, index, selection, selector, slots).is_some()
}

#[test]
fn composed_index_admits_two_exact_widths() {
    assert!(admitted(Case::Direct));
    assert!(admitted(Case::IdentityMask));
}

#[test]
fn composed_index_declines_lossy_mask() {
    assert!(!admitted(Case::LossyMask));
}

#[test]
fn composed_index_requires_current_word_and_geometry() {
    assert!(!admitted(Case::DifferentWord));
    assert!(!admitted(Case::DifferentBank));
    assert!(!admitted(Case::DifferentSubtraction));
}

#[test]
fn composed_index_declines_lossy_word_projection() {
    let mut function = Function::new();
    function.signature.params.push(AbiParam::new(types::I64));
    let mut context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut context);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let word = b.block_params(entry)[0];
    let narrow = b.ins().ireduce(types::I32, word);
    let masked = b.ins().band_imm(word, i64::MAX);
    b.ins().return_(&[]);
    b.seal_all_blocks();
    b.finalize();
    let mut facts = Facts::new(&function);
    assert!(same_unsigned_word(&function, &mut facts, narrow, word).is_none());
    assert!(same_unsigned_word(&function, &mut facts, masked, word).is_none());
}

#[test]
fn composed_read_bound_accepts_only_a_canonical_comparison_mask() {
    let mut function = Function::new();
    function.signature.params = vec![
        AbiParam::new(types::I64),
        AbiParam::new(types::I64),
        AbiParam::new(types::I8),
    ];
    let mut context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut context);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [index, length, byte] = b.block_params(entry).try_into().unwrap();
    let bound = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    let masked = b.ins().band_imm(bound, 1);
    let lost_bound = b.ins().band_imm(bound, 2);
    let raw_mask = b.ins().band_imm(byte, 1);
    b.ins().return_(&[]);
    b.seal_all_blocks();
    b.finalize();

    let comparison = definition(&function, bound).unwrap().0;
    assert_eq!(read_comparison(&function, bound), Some(comparison));
    assert_eq!(read_comparison(&function, masked), Some(comparison));
    assert_eq!(read_comparison(&function, lost_bound), None);
    assert_eq!(read_comparison(&function, raw_mask), None);
}
