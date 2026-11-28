use super::*;
use cranelift_codegen::ir::{AbiParam, Function, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
type Kernel = unsafe extern "C" fn(*const f64, *const f64, *mut u64, u64, i64, i64, u64);
fn function(rows: usize, vector: bool, left_vector: bool, right_vector: bool) -> Function {
    let mut f = Function::new();
    f.signature.params = vec![AbiParam::new(types::I64); 7];
    let mut context = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut f, &mut context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let streams = (0..rows)
            .map(|row| {
                let offset = b.ins().imul_imm(args[6], row as i64);
                let pointer = b.ins().iadd(args[0], offset);
                Stream {
                    pointer,
                    step: args[4],
                    vector: left_vector,
                }
            })
            .collect::<Vec<_>>();
        let sums = emit(
            &mut b,
            &streams,
            Stream {
                pointer: args[1],
                step: args[5],
                vector: right_vector,
            },
            args[3],
            vector,
        );
        for (row, value) in sums.into_iter().enumerate() {
            b.ins().store(
                MemFlags::new(),
                value,
                args[2],
                (row * if vector { 16 } else { 8 }) as i32,
            );
        }
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    f
}
fn compile(f: Function) -> (cranelift_jit::JITModule, Kernel, String) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func = f;
    context.func.signature.call_conv = module.make_signature().call_conv;
    context.set_disasm(true);
    let id = module
        .declare_function(
            "shared_ordered_dot_core",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let assembly = context.compiled_code().unwrap().vcode.clone().unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: exact seven-word signature; module retained through every call.
    let kernel =
        unsafe { core::mem::transmute::<*const u8, Kernel>(module.get_finalized_function(id)) };
    (module, kernel, assembly)
}
#[test]
fn ordered_map_dot_core_runtime_strides_reverse_streams_independent_lanes_exact_order_and_canaries()
{
    let sentinel = 0xbad0_bad0_bad0_bad0u64;
    let pattern = [1e16, 1.0, -1e16, -0.0, 0.0, f64::from_bits(1), 2.5, -3.25];
    for rows in [1, 2, 4] {
        for (vector, left_vector, right_vector) in [
            (false, false, false),
            (true, false, true),
            (true, true, false),
            (true, true, true),
            (true, false, false),
        ] {
            let (_module, kernel, assembly) =
                compile(function(rows, vector, left_vector, right_vector));
            assert!(
                !assembly.contains("fmadd") && !assembly.contains("fmla"),
                "{assembly}"
            );
            let left = (0..rows * 64 + 16)
                .map(|i| pattern[i % pattern.len()])
                .collect::<Vec<_>>();
            let right = (0..128)
                .map(|i| if i % 3 == 0 { -1.0f64 } else { 1.0f64 })
                .collect::<Vec<_>>();
            let left_before = left.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
            let right_before = right.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
            let lanes = if vector { 2 } else { 1 };
            for count in [0u64, 1, 2, 3, 7, 9] {
                for left_step in [8i64, -8, 24, -24] {
                    for right_step in [8i64, -8, 24, -24] {
                        let left_start = if left_step < 0 {
                            count.saturating_sub(1) as i64 * (-left_step) / 8
                        } else {
                            0
                        };
                        let right_start = if right_step < 0 {
                            count.saturating_sub(1) as i64 * (-right_step) / 8
                        } else {
                            0
                        };
                        let mut output = vec![sentinel; rows * lanes + 2];
                        let lp = if count == 0 {
                            core::ptr::null()
                        } else {
                            unsafe { left.as_ptr().add(left_start as usize) }
                        };
                        let rp = if count == 0 {
                            core::ptr::null()
                        } else {
                            unsafe { right.as_ptr().add(right_start as usize) }
                        };
                        // SAFETY: every strided/adjacent input cell is inside its immutable
                        // bank; empty dots use null inputs and may never load them.
                        unsafe {
                            kernel(
                                lp,
                                rp,
                                output.as_mut_ptr().add(1),
                                count,
                                left_step,
                                right_step,
                                64 * 8,
                            )
                        };
                        assert_eq!(output[0], sentinel);
                        assert_eq!(output[rows * lanes + 1], sentinel);
                        for row in 0..rows {
                            for lane in 0..lanes {
                                let mut sum = 0.0f64;
                                for k in 0..count {
                                    let li = row * 64
                                        + (left_start + k as i64 * left_step / 8) as usize
                                        + if vector && left_vector { lane } else { 0 };
                                    let ri = (right_start + k as i64 * right_step / 8) as usize
                                        + if vector && right_vector { lane } else { 0 };
                                    let term = std::hint::black_box(left[li] * right[ri]);
                                    sum = std::hint::black_box(sum + term);
                                }
                                assert_eq!(
                                    output[1 + row * lanes + lane],
                                    sum.to_bits(),
                                    "rows{rows} lanes{lanes} k{count} strides{left_step},{right_step} lane{lane}"
                                );
                            }
                        }
                        assert_eq!(
                            left.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                            left_before
                        );
                        assert_eq!(
                            right.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                            right_before
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn ordered_map_dot_core_ssa_shares_right_load_and_preserves_four_independent_ordered_sums() {
    for vector in [false, true] {
        let f = function(4, vector, false, vector);
        let groups = f
            .layout
            .blocks()
            .filter(|&block| {
                let terms = f
                    .layout
                    .block_insts(block)
                    .filter(|&i| f.dfg.insts[i].opcode() == Opcode::Fmul)
                    .collect::<Vec<_>>();
                terms.len() == 4
                    && terms
                        .iter()
                        .all(|&i| f.dfg.inst_args(i)[1] == f.dfg.inst_args(terms[0])[1])
            })
            .count();
        assert_eq!(groups, 1, "{}", f.display());
        let adds = f
            .layout
            .blocks()
            .flat_map(|b| f.layout.block_insts(b))
            .filter(|&i| f.dfg.insts[i].opcode() == Opcode::Fadd)
            .collect::<Vec<_>>();
        assert_eq!(adds.len(), 4);
        assert_eq!(
            adds.iter()
                .map(|&i| f.dfg.inst_args(i)[0])
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            4
        );
        let (_module, _, assembly) = compile(f);
        assert!(
            !assembly.contains("fmadd") && !assembly.contains("fmla"),
            "{assembly}"
        );
    }
}
