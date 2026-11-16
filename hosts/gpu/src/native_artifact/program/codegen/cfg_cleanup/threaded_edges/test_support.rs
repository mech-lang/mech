use super::Report;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use crate::native_artifact::program::codegen::cfg_cleanup::thread_loop_edges;
use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{Linkage, Module};

const ASSERT: u64 = u64::MAX - 17;
const OVERFLOW: u64 = u64::MAX - 18;
const MULTIPLIER: u64 = 0x9e37_79b9;
pub(super) type Entry = unsafe extern "C" fn(u64, u64, u64, *mut u64) -> u64;

pub(super) fn fixture(module: &JITModule, variant: &str) -> Context {
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 4];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let header = b.create_block();
        let dispatch = b.create_block();
        let loop_test = b.create_block();
        let body = b.create_block();
        let guard = b.create_block();
        let stored = b.create_block();
        let done = b.create_block();
        let finish = b.create_block();
        let join = b.create_block();
        let bad_assert = b.create_block();
        let bad_overflow = b.create_block();
        b.append_block_params_for_function_params(entry);
        for ty in [types::I64, types::I64, types::I64, types::F64] {
            b.append_block_param(header, ty);
        }
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        let payload = b.ins().bitcast(types::F64, MemFlags::new(), args[0]);
        b.ins().jump(
            header,
            &[zero.into(), zero.into(), args[0].into(), payload.into()],
        );
        b.switch_to_block(header);
        let h = b.block_params(header).to_vec();
        if variant == "effect" {
            b.ins().store(MemFlags::new(), h[0], args[3], 8);
        }
        let loaded = if variant == "load" {
            Some(b.ins().load(types::I64, MemFlags::new(), args[3], 8))
        } else {
            None
        };
        let opaque = if variant == "opaque" {
            Some(b.ins().iadd(h[0], args[2]))
        } else {
            None
        };
        let step = b.ins().icmp_imm(IntCC::Equal, h[0], 0);
        b.ins().brif(step, dispatch, &[], finish, &[]);
        b.switch_to_block(dispatch);
        if variant == "merge" {
            b.ins().brif(args[2], loop_test, &[], join, &[]);
        } else {
            b.ins().jump(loop_test, &[]);
        }
        b.switch_to_block(loop_test);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, h[1], args[1]);
        b.ins().brif(more, body, &[], done, &[]);
        let sum;
        b.switch_to_block(body);
        if variant == "overflow-first" {
            let (s, overflow) = b.ins().uadd_overflow(h[2], one);
            sum = s;
            b.ins().brif(overflow, bad_overflow, &[], guard, &[]);
            b.switch_to_block(guard);
            b.ins().brif(args[2], stored, &[], bad_assert, &[]);
        } else {
            b.ins().brif(args[2], guard, &[], bad_assert, &[]);
            b.switch_to_block(guard);
            let (s, overflow) = b.ins().uadd_overflow(h[2], one);
            sum = s;
            b.ins().brif(overflow, bad_overflow, &[], stored, &[]);
        }
        b.switch_to_block(stored);
        let next = b.ins().iadd(h[1], one);
        b.ins().store(MemFlags::new(), next, args[3], 0);
        b.ins()
            .jump(header, &[zero.into(), next.into(), sum.into(), h[3].into()]);
        b.switch_to_block(done);
        let total = b.ins().iadd_imm(h[2], 17);
        let bits = b.ins().bitcast(types::I64, MemFlags::new(), h[3]);
        let bits = b.ins().bxor(bits, args[2]);
        let payload = b.ins().bitcast(types::F64, MemFlags::new(), bits);
        if variant == "duplicate" {
            let other_total = b.ins().iadd_imm(h[2], 33);
            let old_bits = b.ins().bitcast(types::I64, MemFlags::new(), h[3]);
            let bits = b.ins().bnot(args[2]);
            let bits = b.ins().bxor(old_bits, bits);
            let other = b.ins().bitcast(types::F64, MemFlags::new(), bits);
            b.ins().brif(
                args[2],
                header,
                &[one.into(), h[1].into(), total.into(), payload.into()],
                header,
                &[one.into(), h[1].into(), other_total.into(), other.into()],
            );
        } else {
            b.ins().jump(
                header,
                &[one.into(), h[1].into(), total.into(), payload.into()],
            );
        }
        b.switch_to_block(finish);
        if variant == "merge" {
            let marker = b.ins().iconst(types::I64, 0x55);
            b.ins().store(MemFlags::new(), marker, args[3], 8);
        }
        b.ins().jump(join, &[]);
        b.switch_to_block(join);
        let scaled = b.ins().imul_imm(h[2], MULTIPLIER as i64);
        let bits = b.ins().bitcast(types::I64, MemFlags::new(), h[3]);
        let mut result = b.ins().bxor(scaled, bits);
        if let Some(loaded) = loaded {
            result = b.ins().bxor(result, loaded);
        }
        if let Some(opaque) = opaque {
            result = b.ins().bxor(result, opaque);
        }
        b.ins().return_(&[result]);
        b.switch_to_block(bad_assert);
        let error = b.ins().iconst(types::I64, ASSERT as i64);
        b.ins().return_(&[error]);
        b.switch_to_block(bad_overflow);
        let error = b.ins().iconst(types::I64, OVERFLOW as i64);
        b.ins().return_(&[error]);
        b.seal_all_blocks();
        b.finalize();
    }
    c
}

pub(super) fn define(module: &mut JITModule, c: &mut Context, name: &str) -> Entry {
    let id = module
        .declare_function(name, Linkage::Local, &c.func.signature)
        .unwrap();
    module.define_function(id, c).unwrap();
    module.finalize_definitions().unwrap();
    unsafe { std::mem::transmute(module.get_finalized_function(id)) }
}

fn expected(seed: u64, limit: u64, flag: u64, variant: &str) -> (u64, [u64; 2]) {
    let mut memory = [0, 0xabcd];
    let mut total = seed;
    if variant == "merge" && flag == 0 {
        return (total.wrapping_mul(MULTIPLIER) ^ seed, memory);
    }
    for k in 0..limit {
        if variant == "effect" {
            memory[1] = 0;
        }
        if variant == "overflow-first" && total == u64::MAX {
            return (OVERFLOW, memory);
        }
        if flag == 0 {
            return (ASSERT, memory);
        }
        let Some(next) = total.checked_add(1) else {
            return (OVERFLOW, memory);
        };
        total = next;
        memory[0] = k + 1;
    }
    if variant == "effect" {
        memory[1] = 1;
    }
    if variant == "merge" {
        memory[1] = 0x55;
    }
    let (amount, payload) = if variant == "duplicate" && flag == 0 {
        (33, seed ^ !flag)
    } else {
        (17, seed ^ flag)
    };
    let mut result = total.wrapping_add(amount).wrapping_mul(MULTIPLIER) ^ payload;
    if variant == "load" {
        result ^= memory[1];
    }
    (result, memory)
}

pub(super) fn pair(module: &mut JITModule, variant: &str, index: usize) -> (Entry, Entry, Report) {
    let mut before = fixture(module, variant);
    before.verify(module.isa()).unwrap();
    let old = define(module, &mut before, &format!("before_{index}"));
    let mut after = fixture(module, variant);
    let controls = CompilationControls::new(OptimizationOptions::default());
    let report = thread_loop_edges(&controls, &mut after, module.isa()).unwrap();
    after.verify(module.isa()).unwrap();
    let new = define(module, &mut after, &format!("after_{index}"));
    (old, new, report)
}

pub(super) fn check(
    pair: &(Entry, Entry, Report),
    seed: u64,
    limit: u64,
    flag: u64,
    variant: &str,
) {
    let wanted = expected(seed, limit, flag, variant);
    for f in [pair.0, pair.1] {
        let mut memory = [0, 0xabcd];
        let value = unsafe { f(seed, limit, flag, memory.as_mut_ptr()) };
        assert_eq!((value, memory), wanted, "{variant} {seed} {limit} {flag}");
    }
}
