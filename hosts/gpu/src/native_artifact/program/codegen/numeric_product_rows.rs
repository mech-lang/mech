//! Bounded row register tiles share RHS loads and k control. Each scalar or
//! column-SIMD accumulator keeps its original +0 and ordered dot product.
use super::*;
fn tile(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    right: Value,
    output: Value,
    row: Value,
    column: Value,
    columns: Value,
    inner: Value,
    row_width: usize,
    vector: bool,
    epilogue: Option<&Epilogue<'_>>,
    transposed_left: bool,
    rows: Value,
) {
    debug_assert!([1, 2, 4].contains(&row_width));
    let left0 = if transposed_left {
        row
    } else {
        b.ins().imul(row, inner)
    };
    let index0 = b.ins().imul(row, columns);
    let index0 = b.ins().iadd(index0, column);
    let mut states = vec![(row, left0, index0)];
    let mut prior_index = index0;
    for offset in 1..row_width {
        let r = b.ins().iadd_imm(row, offset as i64);
        let base = if transposed_left {
            r
        } else {
            b.ins().imul(r, inner)
        };
        let index = b.ins().iadd(prior_index, columns);
        states.push((r, base, index));
        prior_index = index;
    }
    let left_step = if transposed_left {
        b.ins().imul_imm(rows, 8)
    } else {
        b.ins().iconst(types::I64, 8)
    };
    let right_step = b.ins().imul_imm(columns, 8);
    let right_pointer = address(b, right, column);
    let streams = states
        .iter()
        .map(|(_, base, _)| Stream {
            pointer: address(b, left, *base),
            step: left_step,
            vector: false,
        })
        .collect::<Vec<_>>();
    let sums = numeric_dot_core::emit(
        b,
        &streams,
        Stream {
            pointer: right_pointer,
            step: right_step,
            vector,
        },
        inner,
        vector,
    );
    for ((r, _, index), result) in states.iter().zip(sums) {
        let result = if let Some(e) = epilogue {
            e.apply(b, result, *r, column, *index, vector)
        } else {
            result
        };
        let destination = address(b, output, *index);
        b.ins().store(MemFlags::new(), result, destination, 0);
    }
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    right: Value,
    output: Value,
    count: Value,
    columns: Value,
    inner: Value,
    epilogue: Option<&Epilogue<'_>>,
    quartets: bool,
    transposed_left: bool,
    done: Block,
) {
    let nonempty = b.create_block();
    let valid = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
    // No division, either axis traversal or address is reached for empty output.
    b.ins().brif(valid, nonempty, &[], done, &[]);
    b.switch_to_block(nonempty);
    let rows = b.ins().udiv(count, columns);
    let row = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(row, zero);
    let header = b.create_block();
    let body = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let at_row = b.use_var(row);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at_row, rows);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let remaining = b.ins().isub(rows, at_row);
    let mut entries = Vec::new();
    if quartets {
        let four = b.create_block();
        let tails = b.create_block();
        let enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 4);
        b.ins().brif(enough, four, &[], tails, &[]);
        b.switch_to_block(tails);
        entries.push((four, 4));
    }
    let two = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
    let pair = b.create_block();
    let single = b.create_block();
    b.ins().brif(two, pair, &[], single, &[]);
    entries.extend([(pair, 2), (single, 1)]);
    for (entry, step) in entries {
        b.switch_to_block(entry);
        let column = b.declare_var(types::I64);
        b.def_var(column, zero);
        let columns_header = b.create_block();
        let columns_body = b.create_block();
        let next_rows = b.create_block();
        b.ins().jump(columns_header, &[]);
        b.switch_to_block(columns_header);
        let at_column = b.use_var(column);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, at_column, columns);
        b.ins().brif(more, columns_body, &[], next_rows, &[]);
        b.switch_to_block(columns_body);
        let remaining = b.ins().isub(columns, at_column);
        let two = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
        let paired_columns = b.create_block();
        let scalar_column = b.create_block();
        b.ins().brif(two, paired_columns, &[], scalar_column, &[]);
        for (block, vector, width) in [(paired_columns, true, 2), (scalar_column, false, 1)] {
            b.switch_to_block(block);
            tile(
                b,
                left,
                right,
                output,
                at_row,
                at_column,
                columns,
                inner,
                step,
                vector,
                epilogue,
                transposed_left,
                rows,
            );
            let next = b.ins().iadd_imm(at_column, width);
            b.def_var(column, next);
            b.ins().jump(columns_header, &[]);
        }
        b.switch_to_block(next_rows);
        let next = b.ins().iadd_imm(at_row, step as i64);
        b.def_var(row, next);
        b.ins().jump(header, &[]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, Function, Opcode};
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_module::{Linkage, Module, default_libcall_names};
    type Product = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64;
    fn function(paired: bool, quartets: bool) -> Function {
        let mut f = Function::new();
        f.signature.params = vec![AbiParam::new(types::I64); 6];
        f.signature.returns = vec![AbiParam::new(types::I64)];
        let mut ctx = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut f, &mut ctx);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let a = b.block_params(entry).to_vec();
            let done = b.create_block();
            super::super::emit(
                &mut b, a[0], a[1], a[2], a[3], a[4], a[5], paired, quartets, done,
            );
            b.switch_to_block(done);
            let one = b.ins().iconst(types::I64, 1);
            b.ins().return_(&[one]);
            b.seal_all_blocks();
            b.finalize();
        }
        f
    }
    #[test]
    fn numeric_product_rows_runtime_kernel_shares_right_ssa_and_keeps_order_tails_and_banks() {
        for paired in [false, true] {
            let f = function(paired, false);
            let shared = f
                .layout
                .blocks()
                .filter(|&block| {
                    let args = f
                        .layout
                        .block_insts(block)
                        .filter(|&i| f.dfg.insts[i].opcode() == Opcode::Fmul)
                        .map(|i| f.dfg.inst_args(i)[1])
                        .collect::<Vec<_>>();
                    args.len() == 2 && args[0] == args[1]
                })
                .count();
            assert_eq!(shared, if paired { 2 } else { 0 }, "{}", f.display());
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
                .declare_function("ordered_product_rows", Linkage::Local, &ctx.func.signature)
                .unwrap();
            module.define_function(id, &mut ctx).unwrap();
            let assembly = ctx.compiled_code().unwrap().vcode.as_ref().unwrap();
            assert!(
                !assembly.contains("fmadd") && !assembly.contains("fmla"),
                "{assembly}"
            );
            module.finalize_definitions().unwrap();
            // SAFETY: retained Cranelift function has six word arguments and a
            // word result. Tiny banks are disjoint and match the admitted view.
            let product = unsafe {
                core::mem::transmute::<*const u8, Product>(module.get_finalized_function(id))
            };
            for rows in 0..=5u64 {
                for inner in 0..=5u64 {
                    for columns in 0..=5u64 {
                        let words = [1e16, 1.0, -1e16, -0.0, 2.5, -3.0];
                        let a = (0..rows * inner)
                            .map(|i| words[i as usize % words.len()])
                            .collect::<Vec<_>>();
                        let c = (0..inner * columns)
                            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
                            .collect::<Vec<_>>();
                        let mut output =
                            vec![0xabcd_9876_1234_5678u64; (rows * columns) as usize + 2];
                        assert_eq!(
                            unsafe {
                                product(
                                    a.as_ptr() as u64,
                                    c.as_ptr() as u64,
                                    output.as_mut_ptr().add(1) as u64,
                                    rows * columns,
                                    columns,
                                    inner,
                                )
                            },
                            1
                        );
                        for row in 0..rows {
                            for column in 0..columns {
                                let mut sum = 0.0f64;
                                for k in 0..inner {
                                    let term = std::hint::black_box(
                                        a[(row * inner + k) as usize]
                                            * c[(k * columns + column) as usize],
                                    );
                                    sum = std::hint::black_box(sum + term);
                                }
                                assert_eq!(
                                    output[(row * columns + column) as usize + 1],
                                    sum.to_bits(),
                                    "{rows}x{inner}x{columns}, paired{paired}"
                                );
                            }
                        }
                        assert_eq!(output[0], 0xabcd_9876_1234_5678);
                        assert_eq!(output[output.len() - 1], output[0]);
                    }
                }
            }
            for (columns, inner) in [(0, 0), (0, u64::MAX), (u64::MAX, 0)] {
                // SAFETY: empty count must finish without division, traversal or access.
                assert_eq!(unsafe { product(0, 0, 0, 0, columns, inner) }, 1);
            }
        }
    }
    #[test]
    fn numeric_product_quartets_runtime_kernel_shares_four_rhs_uses_order_tails_and_banks() {
        for paired in [true] {
            let f = function(paired, true);
            let shared = f
                .layout
                .blocks()
                .filter(|&block| {
                    let args = f
                        .layout
                        .block_insts(block)
                        .filter(|&i| f.dfg.insts[i].opcode() == Opcode::Fmul)
                        .map(|i| f.dfg.inst_args(i)[1])
                        .collect::<Vec<_>>();
                    args.len() == 4 && args.iter().all(|&v| v == args[0])
                })
                .count();
            assert_eq!(shared, if paired { 2 } else { 0 }, "{}", f.display());
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
                .declare_function(
                    "ordered_product_quartets",
                    Linkage::Local,
                    &ctx.func.signature,
                )
                .unwrap();
            module.define_function(id, &mut ctx).unwrap();
            let assembly = ctx.compiled_code().unwrap().vcode.as_ref().unwrap();
            assert!(
                !assembly.contains("fmadd") && !assembly.contains("fmla"),
                "{assembly}"
            );
            module.finalize_definitions().unwrap();
            // SAFETY: retained Cranelift function has six word arguments and a
            // word result. Tiny banks are disjoint and match the admitted view.
            let product = unsafe {
                core::mem::transmute::<*const u8, Product>(module.get_finalized_function(id))
            };
            for rows in 0..=9u64 {
                for inner in 0..=7u64 {
                    for columns in 0..=7u64 {
                        let words = [1e16, 1.0, -1e16, -0.0, 2.5, -3.0];
                        let a = (0..rows * inner)
                            .map(|i| words[i as usize % words.len()])
                            .collect::<Vec<_>>();
                        let c = (0..inner * columns)
                            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
                            .collect::<Vec<_>>();
                        let mut output =
                            vec![0xabcd_9876_1234_5678u64; (rows * columns) as usize + 2];
                        assert_eq!(
                            unsafe {
                                product(
                                    a.as_ptr() as u64,
                                    c.as_ptr() as u64,
                                    output.as_mut_ptr().add(1) as u64,
                                    rows * columns,
                                    columns,
                                    inner,
                                )
                            },
                            1
                        );
                        for row in 0..rows {
                            for column in 0..columns {
                                let mut sum = 0.0f64;
                                for k in 0..inner {
                                    let term = std::hint::black_box(
                                        a[(row * inner + k) as usize]
                                            * c[(k * columns + column) as usize],
                                    );
                                    sum = std::hint::black_box(sum + term);
                                }
                                assert_eq!(
                                    output[(row * columns + column) as usize + 1],
                                    sum.to_bits(),
                                    "{rows}x{inner}x{columns}, paired{paired}"
                                );
                            }
                        }
                        assert_eq!(output[0], 0xabcd_9876_1234_5678);
                        assert_eq!(output[output.len() - 1], output[0]);
                    }
                }
            }
            for (columns, inner) in [(0, 0), (0, u64::MAX), (u64::MAX, 0)] {
                // SAFETY: empty count must finish without division, traversal or access.
                assert_eq!(unsafe { product(0, 0, 0, 0, columns, inner) }, 1);
            }
        }
    }
}
