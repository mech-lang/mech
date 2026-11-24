use super::{lower, prove};
use cranelift_codegen::ir::{
    AbiParam, Function, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Change {
    None,
    SignedReadBound,
    ExtraStore,
    UnusedDivide,
    WrongStride,
    DeepEntry,
    DeepBody,
}

fn doubled(b: &mut FunctionBuilder<'_>, mut value: Value) {
    for _ in 0..7 {
        value = b.ins().iadd(value, value);
    }
}

fn scatter_loop(change: Change) -> Function {
    let mut f = Function::new();
    f.signature.params.extend([AbiParam::new(types::I64); 3]);
    f.signature.returns.push(AbiParam::new(types::I64));
    let mut context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut f, &mut context);
    let [
        entry,
        head,
        span,
        bounds,
        read,
        key,
        capacity,
        write,
        advance,
        done,
        fail,
    ] = core::array::from_fn(|_| b.create_block());
    b.append_block_params_for_function_params(entry);
    let index = b.append_block_param(head, types::I64);
    // These forwarded copies have lower Value IDs than the dominating packet loads.
    let [bound, base, src, src_len, dst, dst_len] =
        core::array::from_fn(|_| b.append_block_param(span, types::I64));
    let byte = b.append_block_param(key, types::I64);
    b.switch_to_block(entry);
    let packet = b.block_params(entry)[1];
    let mem = MemFlags::new();
    let [offset, limit, first, input, n, output, m] =
        [0, 8, 16, 24, 32, 40, 48].map(|off| b.ins().load(types::I64, mem, packet, off));
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let marker = b.ins().iconst(types::I64, 200);
    if change == Change::DeepEntry {
        doubled(&mut b, offset);
    }
    b.ins().jump(head, &[first.into()]);
    b.switch_to_block(head);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, limit);
    let args = [limit, offset, input, n, output, m].map(Into::into);
    b.ins().brif(more, span, &args, done, &[]);
    b.switch_to_block(span);
    let at = b.ins().iadd(base, index);
    let fits = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, at, base);
    b.ins().brif(fits, bounds, &[], fail, &[]);
    b.switch_to_block(bounds);
    let kind = if change == Change::SignedReadBound {
        IntCC::SignedLessThan
    } else {
        IntCC::UnsignedLessThan
    };
    let inside = b.ins().icmp(kind, at, src_len);
    b.ins().brif(inside, read, &[], key, &[zero.into()]);
    b.switch_to_block(read);
    let address = b.ins().iadd(src, at);
    let value = b.ins().uload8(types::I64, mem, address, 0);
    b.ins().jump(key, &[value.into()]);
    b.switch_to_block(key);
    let ordinal = b.ins().iadd_imm(byte, 1);
    let fits = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, ordinal, byte);
    b.ins().brif(fits, capacity, &[], fail, &[]);
    b.switch_to_block(capacity);
    let fits = b.ins().icmp(IntCC::UnsignedLessThan, byte, dst_len);
    b.ins().brif(fits, write, &[], fail, &[]);
    b.switch_to_block(write);
    let address = b.ins().iadd(dst, byte);
    let value = b.ins().ireduce(types::I8, marker);
    if change == Change::ExtraStore {
        b.ins().store(mem, value, dst, 0);
    }
    if change == Change::UnusedDivide {
        b.ins().udiv(index, zero);
    }
    if change == Change::DeepBody {
        doubled(&mut b, byte);
    }
    b.ins().store(mem, value, address, 0);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, bound);
    b.ins().brif(more, advance, &[], done, &[]);
    b.switch_to_block(advance);
    let next = b
        .ins()
        .iadd_imm(index, if change == Change::WrongStride { 2 } else { 1 });
    b.ins().jump(head, &[next.into()]);
    b.switch_to_block(done);
    b.ins().return_(&[zero]);
    b.switch_to_block(fail);
    b.ins().return_(&[one]);
    b.seal_all_blocks();
    b.finalize();
    f
}

#[test]
fn rejects_changed_checks_effects_and_excessive_symbolic_growth() {
    for change in [
        Change::SignedReadBound,
        Change::ExtraStore,
        Change::UnusedDivide,
        Change::WrongStride,
        Change::DeepEntry,
        Change::DeepBody,
    ] {
        assert!(
            prove(&scatter_loop(change), 7, 3, 5).is_none(),
            "{change:?}"
        );
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn scatter_reads_each_byte_after_the_preceding_store() {
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Module, default_libcall_names};

    let builder =
        JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names()).unwrap();
    let mut module = JITModule::new(builder);
    let mut cx = module.make_context();
    cx.func = scatter_loop(Change::None);
    cx.func.signature.call_conv = module.make_signature().call_conv;
    let witness = prove(&cx.func, 7, 3, 5).expect("constant scatter");
    lower(&mut cx.func, witness);
    // The admission guard may use only packet values defined before the loop.
    cx.verify(module.isa()).unwrap();
    let id = module
        .declare_anonymous_function(&cx.func.signature)
        .unwrap();
    module.define_function(id, &mut cx).unwrap();
    module.finalize_definitions().unwrap();
    type Run = unsafe extern "C" fn(u64, *const u64, u64) -> u64;
    let run: Run = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
    for (src, dst) in [(0, 0), (4, 3), (3, 4)] {
        for count in [7, 8, 9, 16, 17] {
            let mut bytes = [0u8; 264];
            for i in 0..count {
                bytes[src + i] = (src + i + 1 - dst) as u8;
            }
            // The first store changes the next source byte. Reading all eight
            // inputs before any store would select a different destination.
            let mut expected = bytes;
            for i in 0..count {
                let key = usize::from(expected[src + i]);
                expected[dst + key] = 200;
            }
            let base = bytes.as_mut_ptr();
            let packet = [
                0,
                count as u64,
                0,
                base.wrapping_add(src) as u64,
                count as u64,
                base.wrapping_add(dst) as u64,
                256,
            ];
            // Both views and the complete packet live across this call.
            assert_eq!(unsafe { run(0, packet.as_ptr(), 0) }, 0);
            assert_eq!(bytes, expected, "{src} {dst} {count}");
        }
    }
}
