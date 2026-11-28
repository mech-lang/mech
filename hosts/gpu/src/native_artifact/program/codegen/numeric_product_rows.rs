//! A two-row register tile shares right loads and k control. Each scalar or
//! column-SIMD accumulator keeps its own original +0 and ordered dot product.
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
    paired_rows: bool,
    vector: bool,
    epilogue: Option<&Epilogue<'_>>,
) {
    let ty = if vector {
        types::F64.by(2).unwrap()
    } else {
        types::F64
    };
    let left0 = b.ins().imul(row, inner);
    let index0 = b.ins().imul(row, columns);
    let index0 = b.ins().iadd(index0, column);
    let second = paired_rows.then(|| {
        let row1 = b.ins().iadd_imm(row, 1);
        let base1 = b.ins().imul(row1, inner);
        let index1 = b.ins().iadd(index0, columns);
        (row1, base1, index1)
    });
    let k = b.declare_var(types::I64);
    let sum0 = b.declare_var(ty);
    let sum1 = paired_rows.then(|| b.declare_var(ty));
    let zero = b.ins().iconst(types::I64, 0);
    let identity = b.ins().f64const(0.0);
    let identity = if vector {
        b.ins().splat(ty, identity)
    } else {
        identity
    };
    b.def_var(k, zero);
    b.def_var(sum0, identity);
    if let Some(sum1) = sum1 {
        b.def_var(sum1, identity);
    }
    let header = b.create_block();
    let body = b.create_block();
    let finish = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let at = b.use_var(k);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, inner);
    b.ins().brif(more, body, &[], finish, &[]);
    b.switch_to_block(body);
    let ri = b.ins().imul(at, columns);
    let ri = b.ins().iadd(ri, column);
    let rp = address(b, right, ri);
    // One reached right load serves both independent row accumulators.
    let rhs = b.ins().load(ty, MemFlags::new(), rp, 0);
    for (base, sum) in std::iter::once((left0, sum0))
        .chain(second.zip(sum1).map(|((_, base, _), sum)| (base, sum)))
    {
        let li = b.ins().iadd(base, at);
        let lp = address(b, left, li);
        let lhs = b.ins().load(types::F64, MemFlags::new(), lp, 0);
        let lhs = if vector { b.ins().splat(ty, lhs) } else { lhs };
        let term = f64_arithmetic(b, NativeScalarOp::F64Mul, lhs, rhs);
        let prior = b.use_var(sum);
        let next = f64_arithmetic(b, NativeScalarOp::F64Add, prior, term);
        b.def_var(sum, next);
    }
    let advanced = b.ins().iadd_imm(at, 1);
    b.def_var(k, advanced);
    b.ins().jump(header, &[]);
    b.switch_to_block(finish);
    for (r, index, sum) in std::iter::once((row, index0, sum0))
        .chain(second.zip(sum1).map(|((r, _, index), sum)| (r, index, sum)))
    {
        let result = b.use_var(sum);
        let result = if let Some(e) = epilogue {
            e.apply(b, result, r, column, index, vector)
        } else {
            result
        };
        let destination = address(b, output, index);
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
    done: Block,
) {
    let nonempty = b.create_block();
    let valid = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
    // Complete empty products before a division, either axis loop, or address.
    b.ins().brif(valid, nonempty, &[], done, &[]);
    b.switch_to_block(nonempty);
    // The exact prepared product loan certifies count==rows*columns; a
    // nonempty result has positive columns and admitted full input/output banks.
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
    let two = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
    let pair = b.create_block();
    let single = b.create_block();
    b.ins().brif(two, pair, &[], single, &[]);
    for (entry, paired_rows, step) in [(pair, true, 2), (single, false, 1)] {
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
                paired_rows,
                vector,
                epilogue,
            );
            let next = b.ins().iadd_imm(at_column, width);
            b.def_var(column, next);
            b.ins().jump(columns_header, &[]);
        }
        b.switch_to_block(next_rows);
        let next = b.ins().iadd_imm(at_row, step);
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
    fn function(paired: bool) -> Function {
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
            super::super::emit(&mut b, a[0], a[1], a[2], a[3], a[4], a[5], paired, done);
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
            let f = function(paired);
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
}
