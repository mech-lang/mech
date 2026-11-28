//! Exact-bit native transpose over runtime-admitted disjoint typed banks.
use super::numeric_binary::address;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

/// All extents/spans and disjointness have already been checked by the
/// ordinary selected transpose caller. Empty counts finish before either axis.
pub(super) fn emit_selected(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    count: Value,
    output_columns: Value,
    input_columns: Value,
    induction: bool,
    done: Block,
) {
    if !induction {
        emit(b, input, output, count, output_columns, input_columns, done);
        return;
    }
    let rows = b.create_block();
    let cells = b.create_block();
    let next_cell = b.create_block();
    let next_row = b.create_block();
    for _ in 0..2 {
        b.append_block_param(rows, types::I64);
    }
    for _ in 0..3 {
        b.append_block_param(cells, types::I64);
    }
    b.append_block_param(next_row, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    let nonempty = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
    b.ins()
        .brif(nonempty, rows, &[zero.into(), zero.into()], done, &[]);
    b.switch_to_block(rows);
    let row = b.block_params(rows)[0];
    let index = b.block_params(rows)[1];
    // count == input_columns * output_columns > 0 proves both axes
    // positive. Original output-row-major order begins at input[row].
    b.ins()
        .jump(cells, &[zero.into(), index.into(), row.into()]);
    b.switch_to_block(cells);
    let column = b.block_params(cells)[0];
    let index = b.block_params(cells)[1];
    let source = b.block_params(cells)[2];
    let from = address(b, input, source);
    let to = address(b, output, index);
    let word = b.ins().load(types::I64, MemFlags::new(), from, 0);
    b.ins().store(MemFlags::new(), word, to, 0);
    let column = b.ins().iadd_imm(column, 1);
    let index = b.ins().iadd_imm(index, 1);
    let more = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, column, output_columns);
    b.ins()
        .brif(more, next_cell, &[], next_row, &[index.into()]);
    b.switch_to_block(next_cell);
    // Advance only on a reached next cell; no unused out-of-span source
    // address (or source index) is formed after the last column.
    let source = b.ins().iadd(source, input_columns);
    b.ins()
        .jump(cells, &[column.into(), index.into(), source.into()]);
    b.switch_to_block(next_row);
    let index = b.block_params(next_row)[0];
    let row = b.ins().iadd_imm(row, 1);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, row, input_columns);
    b.ins()
        .brif(more, rows, &[row.into(), index.into()], done, &[]);
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, Function, Opcode};
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_module::{Linkage, Module, default_libcall_names};

    type Copy = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;

    // Only the admitted copy loop is isolated here. Production retains the
    // full source/owner, checked span/product and nonoverlap guards.
    fn function(induction: bool) -> Function {
        let mut f = Function::new();
        f.signature.params = vec![AbiParam::new(types::I64); 5];
        f.signature.returns = vec![AbiParam::new(types::I64)];
        let mut ctx = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut f, &mut ctx);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let a = b.block_params(entry).to_vec();
            let done = b.create_block();
            emit_selected(&mut b, a[0], a[1], a[2], a[3], a[4], induction, done);
            b.switch_to_block(done);
            let result = b.ins().iconst(types::I64, 1);
            b.ins().return_(&[result]);
            b.seal_all_blocks();
            b.finalize();
        }
        f
    }

    #[test]
    fn numeric_transpose_induction_runtime_axes_remove_divisions_and_keep_exact_copy() {
        for induction in [false, true] {
            let f = function(induction);
            let divisions = f
                .layout
                .blocks()
                .flat_map(|b| f.layout.block_insts(b))
                .filter(|&i| matches!(f.dfg.insts[i].opcode(), Opcode::Udiv | Opcode::Urem))
                .count();
            assert_eq!(divisions, if induction { 0 } else { 2 });
            let mut module = cranelift_jit::JITModule::new(
                cranelift_jit::JITBuilder::with_flags(
                    &[("opt_level", "speed")],
                    default_libcall_names(),
                )
                .unwrap(),
            );
            let mut ctx = module.make_context();
            ctx.func = f;
            ctx.func.signature.call_conv = module.make_signature().call_conv;
            ctx.set_disasm(true);
            let id = module
                .declare_function("runtime_transpose", Linkage::Local, &ctx.func.signature)
                .unwrap();
            module.define_function(id, &mut ctx).unwrap();
            let assembly = ctx.compiled_code().unwrap().vcode.as_ref().unwrap();
            if cfg!(target_arch = "aarch64") {
                let machine = assembly.split(";; REGALLOC_INPUT_BEGIN").next().unwrap();
                let divisions = machine
                    .lines()
                    .filter(|line| line.contains("udiv "))
                    .count();
                assert_eq!(divisions == 0, induction, "{machine}");
            }
            module.finalize_definitions().unwrap();
            // SAFETY: the retained function has exactly five word arguments
            // and one word result; all reached copies use disjoint valid banks.
            let copy = unsafe {
                core::mem::transmute::<*const u8, Copy>(module.get_finalized_function(id))
            };
            for rows in 0..=9u64 {
                for columns in 0..=9u64 {
                    let source = (0..rows * columns)
                        .map(|i| i.wrapping_mul(0x9e37_79b9_7f4a_7c15))
                        .collect::<Vec<_>>();
                    let saved = source.clone();
                    let mut output = vec![0xabcd_9876_1234_5678; source.len() + 2];
                    assert_eq!(
                        unsafe {
                            copy(
                                source.as_ptr() as u64,
                                output.as_mut_ptr().add(1) as u64,
                                rows * columns,
                                rows,
                                columns,
                            )
                        },
                        1
                    );
                    let mut expected = Vec::new();
                    for column in 0..columns {
                        for row in 0..rows {
                            expected.push(source[(row * columns + column) as usize]);
                        }
                    }
                    assert_eq!(&output[1..output.len() - 1], expected);
                    assert_eq!(output[0], 0xabcd_9876_1234_5678);
                    assert_eq!(output[output.len() - 1], output[0]);
                    assert_eq!(source, saved);
                }
            }
            for (rows, columns) in [(0, 0), (0, u64::MAX), (u64::MAX, 0)] {
                // SAFETY: empty views must finish before any pointer arithmetic
                // or load/store, even with null banks and enormous empty axes.
                assert_eq!(unsafe { copy(0, 0, 0, rows, columns) }, 1);
            }
        }
    }
}
