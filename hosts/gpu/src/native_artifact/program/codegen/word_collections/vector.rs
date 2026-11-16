//! Vector prefix inside the same admitted source/sink loan and scalar tail.
use super::{memory_type, saturating::Threshold};
use crate::native_artifact::codegen::common::ssa_words::Words;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    source: Value,
    sink: Value,
    count: Value,
    width: u8,
    threshold: Threshold,
) -> Value {
    let scalar = memory_type(width);
    let vector = scalar.by(16 / u32::from(width)).unwrap();
    let threshold = match threshold {
        Threshold::Constant(value) => b.ins().iconst(types::I64, value as i64),
        Threshold::Capture(slot) => words.read(b, slot as usize),
    };
    // A wider threshold makes every result zero. Clamping it to the element
    // maximum preserves that case instead of truncating a current argument.
    let threshold = if width < 8 {
        let maximum = b.ins().iconst(types::I64, (1i64 << (width * 8)) - 1);
        let value = b.ins().umin(threshold, maximum);
        b.ins().ireduce(scalar, value)
    } else {
        threshold
    };
    let threshold = b.ins().splat(vector, threshold);
    let quantum = 64 / i64::from(width);
    let end = b.ins().band_imm(count, -quantum);
    let header = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.append_block_param(header, types::I64);
    b.append_block_param(done, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[zero.into()]);
    b.switch_to_block(header);
    let index = b.block_params(header)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, end);
    b.ins().brif(more, body, &[], done, &[index.into()]);
    b.switch_to_block(body);
    let offset = b.ins().imul_imm(index, i64::from(width));
    let input = b.ins().iadd(source, offset);
    let output = b.ins().iadd(sink, offset);
    for offset in [0, 16, 32, 48] {
        let value = b.ins().load(vector, MemFlags::new(), input, offset);
        let result = if width <= 2 {
            b.ins().usub_sat(value, threshold)
        } else {
            // The portable x64 saturating instruction covers byte/halfword
            // lanes. Wider lanes use the same unsigned operation explicitly.
            let valid = b
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, value, threshold);
            let difference = b.ins().isub(value, threshold);
            let scalar_zero = b.ins().iconst(scalar, 0);
            let zero = b.ins().splat(vector, scalar_zero);
            b.ins().bitselect(valid, difference, zero)
        };
        b.ins().store(MemFlags::new(), result, output, offset);
    }
    let next = b.ins().iadd_imm(index, quantum);
    b.ins().jump(header, &[next.into()]);
    b.switch_to_block(done);
    b.block_params(done)[0]
}
