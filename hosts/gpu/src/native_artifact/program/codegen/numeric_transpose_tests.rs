use super::*;
use cranelift_codegen::ir::{AbiParam, Function, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};

type Kernel = unsafe extern "C" fn(*const u64, *mut u64, u64, u64, u64);
fn function(tiled: bool) -> Function {
    let mut f = Function::new();
    f.signature.params = vec![AbiParam::new(types::I64); 5];
    let mut context = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut f, &mut context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let done = b.create_block();
        let emit = if tiled { emit } else { emit_ordinal };
        emit(&mut b, args[0], args[1], args[2], args[3], args[4], done);
        b.switch_to_block(done);
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
            "exact_bit_transpose",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let assembly = context.compiled_code().unwrap().vcode.clone().unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: exact five-word signature; module retained through every call.
    let kernel =
        unsafe { core::mem::transmute::<*const u8, Kernel>(module.get_finalized_function(id)) };
    (module, kernel, assembly)
}
#[test]
fn numeric_transpose_tiles_dynamic_exact_words_odd_edges_empty_null_and_canaries() {
    let (_tiled_module, tiled, _) = compile(function(true));
    let (_ordinal_module, ordinal, _) = compile(function(false));
    let sentinel = 0xbad0_bad0_bad0_bad0;
    let patterns = [
        0,
        1 << 63,
        1,
        (1 << 63) | 1,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0xfff0_0000_0000_0023,
        0x7ff8_1111_2222_3333,
        0xfff8_4444_5555_6666,
    ];
    for rows in [0, 1, 2, 3, 4, 5, 7, 16, 33] {
        for columns in [0, 1, 2, 3, 4, 5, 8, 17, 34] {
            let count = rows * columns;
            let input = (0..count)
                .map(|i| {
                    if i % 2 == 0 {
                        patterns[i % patterns.len()]
                    } else {
                        (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                    }
                })
                .collect::<Vec<_>>();
            let before = input.clone();
            for kernel in [tiled, ordinal] {
                let mut output = vec![sentinel; count + 2];
                let source = if count == 0 {
                    core::ptr::null()
                } else {
                    input.as_ptr()
                };
                // SAFETY: both kernels receive exactly the admitted rectangular
                // disjoint spans, with canaries outside the output span.
                unsafe {
                    kernel(
                        source,
                        output.as_mut_ptr().add(1),
                        count as u64,
                        rows as u64,
                        columns as u64,
                    )
                };
                assert_eq!(output[0], sentinel);
                assert_eq!(output[count + 1], sentinel);
                for row in 0..rows {
                    for column in 0..columns {
                        assert_eq!(
                            output[1 + column * rows + row],
                            input[row * columns + column],
                            "{rows}x{columns} ({row},{column})"
                        );
                    }
                }
                assert_eq!(input, before);
            }
        }
    }
    for (rows, columns) in [(0, u64::MAX), (u64::MAX, 0), (0, 0)] {
        // SAFETY: an empty admitted rectangle never accesses either pointer or
        // computes a stride from its potentially huge nonempty axis.
        unsafe { tiled(core::ptr::null(), core::ptr::null_mut(), 0, rows, columns) };
    }
}
#[test]
fn numeric_transpose_tiles_ssa_has_integer_vectors_without_division_or_float_arithmetic() {
    let f = function(true);
    let ops = f
        .layout
        .blocks()
        .flat_map(|b| f.layout.block_insts(b))
        .map(|i| f.dfg.insts[i].opcode())
        .collect::<Vec<_>>();
    assert!(!ops.iter().any(|o| matches!(
        o,
        Opcode::Udiv | Opcode::Urem | Opcode::Fadd | Opcode::Fmul | Opcode::Fdiv | Opcode::Fsub
    )));
    assert!(
        f.layout
            .blocks()
            .flat_map(|b| f.layout.block_insts(b))
            .any(|i| f.dfg.insts[i].opcode() == Opcode::Load
                && f.dfg.value_type(f.dfg.inst_results(i)[0]) == types::I64X2)
    );
    let (_module, _, assembly) = compile(f);
    assert!(
        !assembly.contains("udiv") && !assembly.contains("sdiv"),
        "{assembly}"
    );
    #[cfg(target_arch = "aarch64")]
    assert!(
        (assembly.contains("zip1") || assembly.contains("uzp1"))
            && (assembly.contains("zip2") || assembly.contains("uzp2")),
        "{assembly}"
    );
}
