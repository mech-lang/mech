//! Byte-preserving copies after a complete checked private map succeeds.
//! Callers prove the complete spans are disjoint and admitted before entry.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    destination: Value,
    bytes: u64,
) -> Result<(), NativeCompileError> {
    if bytes > isize::MAX as u64 {
        return Err(backend("fixed copy exceeds the admitted address domain"));
    }
    let bulk = bytes / 32 * 32;
    if bulk != 0 {
        let copy = b.create_block();
        let done = b.create_block();
        b.append_block_param(copy, types::I64);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(copy, &[zero.into()]);
        b.switch_to_block(copy);
        let offset = b.block_params(copy)[0];
        let from = b.ins().iadd(source, offset);
        let to = b.ins().iadd(destination, offset);
        // Loads/stores carry no floating-point arithmetic or alignment promise.
        let low = b.ins().load(types::I8X16, MemFlags::new(), from, 0);
        let high = b.ins().load(types::I8X16, MemFlags::new(), from, 16);
        b.ins().store(MemFlags::new(), low, to, 0);
        b.ins().store(MemFlags::new(), high, to, 16);
        let next = b.ins().iadd_imm(offset, 32);
        let more = b.ins().icmp_imm(IntCC::UnsignedLessThan, next, bulk as i64);
        b.ins().brif(more, copy, &[next.into()], done, &[]);
        b.switch_to_block(done);
    }
    let mut remaining = bytes - bulk;
    if remaining != 0 {
        let source = b.ins().iadd_imm(source, bulk as i64);
        let destination = b.ins().iadd_imm(destination, bulk as i64);
        let mut offset = 0;
        for (width, ty) in [
            (16, types::I8X16),
            (8, types::I64),
            (4, types::I32),
            (2, types::I16),
            (1, types::I8),
        ] {
            if remaining >= width {
                let value = b.ins().load(ty, MemFlags::new(), source, offset);
                b.ins().store(MemFlags::new(), value, destination, offset);
                offset += width as i32;
                remaining -= width;
            }
        }
        debug_assert_eq!(remaining, 0);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, Function, Opcode};
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_module::{Linkage, Module, default_libcall_names};
    type Copy = unsafe extern "C" fn(*const u8, *mut u8);
    fn compile(bytes: u64) -> (cranelift_jit::JITModule, Copy) {
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut context = module.make_context();
        context.func = Function::new();
        context.func.signature.call_conv = module.make_signature().call_conv;
        context.func.signature.params = vec![AbiParam::new(types::I64); 2];
        let mut frontend = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let p = b.block_params(entry).to_vec();
            emit(&mut b, p[0], p[1], bytes).unwrap();
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize();
        }
        let packed_loads = context
            .func
            .layout
            .blocks()
            .flat_map(|block| context.func.layout.block_insts(block))
            .filter(|&i| {
                context.func.dfg.insts[i].opcode() == Opcode::Load
                    && context
                        .func
                        .dfg
                        .value_type(context.func.dfg.inst_results(i)[0])
                        == types::I8X16
            })
            .count();
        assert_eq!(
            packed_loads,
            (if bytes >= 32 { 2 } else { 0 }) + usize::from(bytes % 32 >= 16)
        );
        let id = module
            .declare_function(
                "private_fixed_copy",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: the retained generated function accepts two pointers, returns
        // nothing, and touches only the caller-proven complete disjoint spans.
        let copy =
            unsafe { core::mem::transmute::<*const u8, Copy>(module.get_finalized_function(id)) };
        (module, copy)
    }
    #[test]
    fn numeric_map_packed_copy_keeps_bits_unaligned_tails_and_canaries() {
        let lengths = (0..=65).chain([
            127, 128, 129, 159, 160, 191, 192, 193, 239, 240, 255, 256, 257, 4096, 4097, 65536,
        ]);
        for bytes in lengths {
            let (_module, copy) = compile(bytes as u64);
            for skew in [0, 1, 3, 7, 15] {
                let begin = 32 + skew;
                let mut source = (0..bytes + 96)
                    .map(|i| ((i * 73 + 117) ^ (i >> 3)) as u8)
                    .collect::<Vec<_>>();
                for (i, bits) in [
                    0u64,
                    1 << 63,
                    0x7ff0_0000_0000_0000,
                    0xfff0_0000_0000_0000,
                    0x7ff8_0000_0000_1234,
                    0x7ff0_0000_0000_0001,
                ]
                .into_iter()
                .enumerate()
                {
                    if (i + 1) * 8 <= bytes {
                        source[begin + i * 8..begin + (i + 1) * 8]
                            .copy_from_slice(&bits.to_ne_bytes());
                    }
                }
                let before = source.clone();
                let mut destination = vec![0xa5; bytes + 96];
                let mut expected = destination.clone();
                expected[begin..begin + bytes].copy_from_slice(&source[begin..begin + bytes]);
                // SAFETY: distinct live allocations include both complete spans
                // and surrounding canaries; unaligned addresses are intentional.
                unsafe {
                    copy(
                        source.as_ptr().add(begin),
                        destination.as_mut_ptr().add(begin),
                    );
                }
                assert_eq!(destination, expected, "bytes={bytes} skew={skew}");
                assert_eq!(source, before);
            }
        }
    }
}
