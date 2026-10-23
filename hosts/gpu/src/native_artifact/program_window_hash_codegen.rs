//! Generated window iteration; every iteration performs the library table update.
use super::*;
use cranelift_codegen::ir::Signature;

pub(super) fn emit(b: &mut FunctionBuilder<'_>) {
    let entry = b.create_block();
    let head = b.create_block();
    let body = b.create_block();
    let advance = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(head, types::I64);
    b.append_block_param(done, types::I64);
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(head, &[zero.into()]);
    b.switch_to_block(head);
    let index = b.block_params(head)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, args[1]);
    b.ins().brif(more, body, &[], done, &[index.into()]);
    b.switch_to_block(body);
    let mut signature = Signature::new(b.func.signature.call_conv);
    signature.params = vec![AbiParam::new(types::I64); 2];
    signature.returns.push(AbiParam::new(types::I64));
    let signature = b.import_signature(signature);
    // The owner and function are bound together by WINDOW_PREPARE. They are
    // never program Values or an unchecked caller-selected foreign function.
    let call = b.ins().call_indirect(signature, args[3], &[args[2], index]);
    let updated = b.inst_results(call)[0];
    b.ins().brif(updated, advance, &[], done, &[index.into()]);
    b.switch_to_block(advance);
    let next = b.ins().iadd_imm(index, 1);
    b.ins().jump(head, &[next.into()]);
    b.switch_to_block(done);
    let completed = b.block_params(done)[0];
    b.ins().return_(&[completed]);
}
