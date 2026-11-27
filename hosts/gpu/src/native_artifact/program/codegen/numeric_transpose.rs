//! Exact-bit native transpose over runtime-admitted disjoint typed banks.
use super::numeric_binary::address;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    count: Value,
    output_columns: Value,
    input_columns: Value,
    done: Block,
) {
    let header = b.create_block();
    let body = b.create_block();
    b.append_block_param(header, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[zero.into()]);
    b.switch_to_block(header);
    let index = b.block_params(header)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, count);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    // A nonempty admitted product guarantees nonzero axes. Empty matrices
    // take the header's done edge before any division, load or store.
    let row = b.ins().udiv(index, output_columns);
    let column = b.ins().urem(index, output_columns);
    let source = b.ins().imul(column, input_columns);
    let source = b.ins().iadd(source, row);
    let from = address(b, input, source);
    let to = address(b, output, index);
    let word = b.ins().load(types::I64, MemFlags::new(), from, 0);
    b.ins().store(MemFlags::new(), word, to, 0);
    let next = b.ins().iadd_imm(index, 1);
    b.ins().jump(header, &[next.into()]);
}
