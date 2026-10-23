//! Rolling ASCII keys in generated code; every window updates the library table.
use super::*;
use cranelift_codegen::ir::Signature;

pub(super) fn emit(b: &mut FunctionBuilder<'_>) {
    let entry = b.create_block();
    let select = b.create_block();
    let empty = b.create_block();
    let head = b.create_block();
    let body = b.create_block();
    let update = b.create_block();
    let advance = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    for _ in 0..3 {
        b.append_block_param(head, types::I64);
    }
    b.append_block_param(done, types::I64);
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let (source, length, owner, callback, width) = (args[0], args[1], args[2], args[3], args[4]);
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let positive = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, width, 0);
    let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, width, length);
    let packed = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThanOrEqual, width, 128 / 7);
    let valid = b.ins().band(positive, fits);
    let valid = b.ins().band(valid, packed);
    b.ins().brif(valid, select, &[], empty, &[]);
    b.switch_to_block(empty);
    b.ins().return_(&[zero]);
    b.switch_to_block(select);
    let last = b.ins().isub(length, width);
    let accepted = b.ins().iadd_imm(last, 1);
    let bits = b.ins().imul_imm(width, 7);
    let narrow = b.ins().icmp_imm(IntCC::UnsignedLessThan, bits, 64);
    let low = b.ins().ishl(one, bits);
    let low = b.ins().iadd_imm(low, -1);
    let ones = b.ins().iconst(types::I64, -1);
    let low_mask = b.ins().select(narrow, low, ones);
    let high_bits = b.ins().iadd_imm(bits, -64);
    let high = b.ins().ishl(one, high_bits);
    let high = b.ins().iadd_imm(high, -1);
    let high_mask = b.ins().select(narrow, zero, high);
    b.ins().jump(head, &[zero.into(), zero.into(), zero.into()]);
    b.switch_to_block(head);
    let params = b.block_params(head).to_vec();
    let (index, previous_low, previous_high) = (params[0], params[1], params[2]);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    b.ins().brif(more, body, &[], done, &[accepted.into()]);
    b.switch_to_block(body);
    // Exactly one byte read, guarded by index < genuine source length. The
    // immutable owner and ASCII/no-CRLF proof survive the synchronous entry.
    let address = b.ins().iadd(source, index);
    let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
    let byte = b.ins().uextend(types::I64, byte);
    let low = b.ins().ishl_imm(previous_low, 7);
    let low = b.ins().bor(low, byte);
    let low = b.ins().band(low, low_mask);
    let carry = b.ins().ushr_imm(previous_low, 57);
    let high = b.ins().ishl_imm(previous_high, 7);
    let high = b.ins().bor(high, carry);
    let high = b.ins().band(high, high_mask);
    let next = b.ins().iadd_imm(index, 1);
    let complete = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, next, width);
    b.ins().brif(complete, update, &[], advance, &[]);
    b.switch_to_block(update);
    let start = b.ins().isub(next, width);
    let mut signature = Signature::new(b.func.signature.call_conv);
    signature.params = vec![AbiParam::new(types::I64); 4];
    signature.returns.push(AbiParam::new(types::I64));
    let signature = b.import_signature(signature);
    // Private owner/function pair; no arbitrary program-supplied pointer.
    let call = b
        .ins()
        .call_indirect(signature, callback, &[owner, start, low, high]);
    let updated = b.inst_results(call)[0];
    b.ins().brif(updated, advance, &[], done, &[start.into()]);
    b.switch_to_block(advance);
    b.ins().jump(head, &[next.into(), low.into(), high.into()]);
    b.switch_to_block(done);
    let completed = b.block_params(done)[0];
    b.ins().return_(&[completed]);
}
