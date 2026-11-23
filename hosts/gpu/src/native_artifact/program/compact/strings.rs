//! Native operations on immutable UTF8 spans. All byte accesses are inside the
//! current span; no zero terminator, alignment, padding or host String is used.
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

pub(super) fn equal(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    left_len: Value,
    right: Value,
    right_len: Value,
) -> Value {
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let same_len = b.create_block();
    let chunks = b.create_block();
    b.append_block_param(chunks, types::I64);
    let chunk = b.create_block();
    let advance = b.create_block();
    let tail = b.create_block();
    b.append_block_param(tail, types::I64);
    let byte = b.create_block();
    let next_byte = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I64);

    let lengths_equal = b.ins().icmp(IntCC::Equal, left_len, right_len);
    b.ins()
        .brif(lengths_equal, same_len, &[], done, &[zero.into()]);
    b.switch_to_block(same_len);
    let pointers_equal = b.ins().icmp(IntCC::Equal, left, right);
    b.ins()
        .brif(pointers_equal, done, &[one.into()], chunks, &[zero.into()]);

    b.switch_to_block(chunks);
    let at = b.block_params(chunks)[0];
    let remaining = b.ins().isub(left_len, at);
    let wide = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 8);
    b.ins().brif(wide, chunk, &[], tail, &[at.into()]);
    b.switch_to_block(chunk);
    let a = b.ins().iadd(left, at);
    let c = b.ins().iadd(right, at);
    let a = b.ins().load(types::I64, MemFlags::new(), a, 0);
    let c = b.ins().load(types::I64, MemFlags::new(), c, 0);
    let same = b.ins().icmp(IntCC::Equal, a, c);
    b.ins().brif(same, advance, &[], done, &[zero.into()]);
    b.switch_to_block(advance);
    let at = b.ins().iadd_imm(at, 8);
    b.ins().jump(chunks, &[at.into()]);

    b.switch_to_block(tail);
    let at = b.block_params(tail)[0];
    let finished = b.ins().icmp(IntCC::Equal, at, left_len);
    b.ins().brif(finished, done, &[one.into()], byte, &[]);
    b.switch_to_block(byte);
    let a = b.ins().iadd(left, at);
    let c = b.ins().iadd(right, at);
    let a = b.ins().load(types::I8, MemFlags::new(), a, 0);
    let c = b.ins().load(types::I8, MemFlags::new(), c, 0);
    let same = b.ins().icmp(IntCC::Equal, a, c);
    b.ins().brif(same, next_byte, &[], done, &[zero.into()]);
    b.switch_to_block(next_byte);
    let at = b.ins().iadd_imm(at, 1);
    b.ins().jump(tail, &[at.into()]);

    b.switch_to_block(done);
    b.block_params(done)[0]
}

/// Canonical raw UTF8 byte ordering, returning -1, 0 or 1. Wide words
/// are byte-swapped only after a mismatch so unsigned ordering follows the
/// first differing byte on little-endian targets.
pub(super) fn order(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    left_len: Value,
    right: Value,
    right_len: Value,
    endian: cranelift_codegen::ir::Endianness,
) -> Value {
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let minus = b.ins().iconst(types::I64, -1);
    let length = b.ins().umin(left_len, right_len);
    let chunks = b.create_block();
    b.append_block_param(chunks, types::I64);
    let wide = b.create_block();
    let mismatch = b.create_block();
    let advance = b.create_block();
    let tail = b.create_block();
    b.append_block_param(tail, types::I64);
    let byte = b.create_block();
    let next = b.create_block();
    let lengths = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I64);
    b.ins().jump(chunks, &[zero.into()]);
    b.switch_to_block(chunks);
    let at = b.block_params(chunks)[0];
    let remaining = b.ins().isub(length, at);
    let enough = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 8);
    b.ins().brif(enough, wide, &[], tail, &[at.into()]);
    b.switch_to_block(wide);
    let a = b.ins().iadd(left, at);
    let c = b.ins().iadd(right, at);
    let a = b.ins().load(types::I64, MemFlags::new(), a, 0);
    let c = b.ins().load(types::I64, MemFlags::new(), c, 0);
    let same = b.ins().icmp(IntCC::Equal, a, c);
    b.ins().brif(same, advance, &[], mismatch, &[]);
    b.switch_to_block(mismatch);
    let (a, c) = if endian == cranelift_codegen::ir::Endianness::Little {
        (b.ins().bswap(a), b.ins().bswap(c))
    } else {
        (a, c)
    };
    let less = b.ins().icmp(IntCC::UnsignedLessThan, a, c);
    let result = b.ins().select(less, minus, one);
    b.ins().jump(done, &[result.into()]);
    b.switch_to_block(advance);
    let at = b.ins().iadd_imm(at, 8);
    b.ins().jump(chunks, &[at.into()]);
    b.switch_to_block(tail);
    let at = b.block_params(tail)[0];
    let finished = b.ins().icmp(IntCC::Equal, at, length);
    b.ins().brif(finished, lengths, &[], byte, &[]);
    b.switch_to_block(byte);
    let a = b.ins().iadd(left, at);
    let c = b.ins().iadd(right, at);
    let a = b.ins().load(types::I8, MemFlags::new(), a, 0);
    let c = b.ins().load(types::I8, MemFlags::new(), c, 0);
    let same = b.ins().icmp(IntCC::Equal, a, c);
    let less = b.ins().icmp(IntCC::UnsignedLessThan, a, c);
    let result = b.ins().select(less, minus, one);
    b.ins().brif(same, next, &[], done, &[result.into()]);
    b.switch_to_block(next);
    let at = b.ins().iadd_imm(at, 1);
    b.ins().jump(tail, &[at.into()]);
    b.switch_to_block(lengths);
    let same = b.ins().icmp(IntCC::Equal, left_len, right_len);
    let less = b.ins().icmp(IntCC::UnsignedLessThan, left_len, right_len);
    let result = b.ins().select(less, minus, one);
    let result = b.ins().select(same, zero, result);
    b.ins().jump(done, &[result.into()]);
    b.switch_to_block(done);
    b.block_params(done)[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::AbiParam;
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    type Compare = unsafe extern "C" fn(*const u8, usize, *const u8, usize) -> u64;

    fn compile() -> (JITModule, Compare) {
        let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 4];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let mut fb = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let result = equal(&mut b, args[0], args[1], args[2], args[3]);
            b.ins().return_(&[result]);
            b.seal_all_blocks();
            b.finalize();
        }
        assert!(context.func.layout.blocks().all(|block| {
            context
                .func
                .layout
                .block_insts(block)
                .all(|inst| !context.func.dfg.insts[inst].opcode().is_call())
        }));
        let id = module
            .declare_function(
                "test_native_utf8_equal",
                Linkage::Export,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: the signature above is four pointer-sized integers returning
        // one integer; the returned module retains its executable allocation.
        let call =
            unsafe { std::mem::transmute::<*const u8, Compare>(module.get_finalized_function(id)) };
        (module, call)
    }

    #[test]
    fn utf8_equality_emits_native_chunks_and_exact_tails() {
        let (_module, compare) = compile();
        let texts = [
            "",
            "a",
            "a\0b",
            "naïve",
            "e\u{301}",
            "é",
            "🦀🦀",
            "\r\n",
            "abcdefghijklmnopq",
        ];
        for left in texts {
            for right in texts {
                // SAFETY: both immutable UTF8 buffers remain live during the call.
                let actual =
                    unsafe { compare(left.as_ptr(), left.len(), right.as_ptr(), right.len()) };
                assert_eq!(actual, u64::from(left == right), "{left:?}, {right:?}");
            }
        }
        for len in 0..65 {
            let left = vec![b'a'; len + 2];
            let mut right = left.clone();
            // Exercise unaligned spans and every possible mismatch position
            // around both the chunk and tail boundaries.
            let call = |right: &[u8]| unsafe {
                compare(left.as_ptr().add(1), len, right.as_ptr().add(1), len)
            };
            assert_eq!(call(&right), 1);
            for at in 0..len {
                right[at + 1] = b'b';
                assert_eq!(call(&right), 0, "len={len}, at={at}");
                right[at + 1] = b'a';
            }
        }
        // Empty native spans may have no backing allocation at all.
        assert_eq!(
            unsafe { compare(std::ptr::null(), 0, std::ptr::null(), 0) },
            1
        );
    }
}
