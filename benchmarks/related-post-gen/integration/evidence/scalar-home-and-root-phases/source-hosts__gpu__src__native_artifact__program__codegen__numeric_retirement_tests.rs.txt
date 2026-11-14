//! Execute the real emitted masks across loops, dirty entry, bit63, a second
//! mask word and a simulated resumable helper that dirties physical homes.
use super::ScalarHomes;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn traversals(
    b: &mut FunctionBuilder<'_>,
    homes: &ScalarHomes,
    bank: Value,
    rounds: Value,
    clears: Variable,
) {
    let at = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(at, zero);
    let header = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let index = b.use_var(at);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, rounds);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    for node in [1usize, 64, 65] {
        homes
            .retire(b, node, |b| {
                let zero = b.ins().iconst(types::I64, 0);
                let offset = ((node - 1) * 16) as i32;
                b.ins().store(MemFlags::new(), zero, bank, offset);
                b.ins().store(MemFlags::new(), zero, bank, offset + 8);
                let count = b.use_var(clears);
                let count = b.ins().iadd_imm(count, 1);
                b.def_var(clears, count);
                Ok(())
            })
            .unwrap();
    }
    let index = b.use_var(at);
    let next = b.ins().iadd_imm(index, 1);
    b.def_var(at, next);
    b.ins().jump(header, &[]);
    b.switch_to_block(done);
}

#[test]
fn scalar_home_masks_preserve_dirty_entry_repeated_exits_and_helper_reentry() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature = module.make_signature();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut frontend = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let bank = b.block_params(entry)[0];
        let rounds = b.block_params(entry)[1];
        let homes = ScalarHomes::new(&mut b, 1..=65);
        assert_eq!(homes.word_count(), 2);
        let clears = b.declare_var(types::I64);
        let zero = b.ins().iconst(types::I64, 0);
        b.def_var(clears, zero);
        traversals(&mut b, &homes, bank, rounds, clears);
        // Model any resumable callback's physical-bank writes, including a
        // nonzero payload with a cleared validity word. All masks reset.
        for node in [1usize, 64, 65] {
            let offset = ((node - 1) * 16) as i32;
            let dirty = b.ins().iconst(types::I64, -17);
            let zero = b.ins().iconst(types::I64, 0);
            b.ins().store(MemFlags::new(), dirty, bank, offset);
            b.ins().store(MemFlags::new(), zero, bank, offset + 8);
        }
        homes.materialized(&mut b);
        traversals(&mut b, &homes, bank, rounds, clears);
        let count = b.use_var(clears);
        b.ins().return_(&[count]);
        b.seal_all_blocks();
        b.finalize();
    }
    let id = module
        .declare_function("scalar_home_masks", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: this retained generated function takes a pointer to130 writable
    // u64 cells and a bounded loop count, and returns its observed clear count.
    let run: unsafe extern "C" fn(u64, u64) -> u64 =
        unsafe { core::mem::transmute(module.get_finalized_function(id)) };
    for rounds in [0, 1, 2, 17] {
        let mut bank = (0u64..130).map(|i| i.wrapping_mul(17)).collect::<Vec<_>>();
        let mut expected = bank.clone();
        for node in [1usize, 64, 65] {
            let at = (node - 1) * 2;
            expected[at] = if rounds == 0 { (-17i64) as u64 } else { 0 };
            expected[at + 1] = 0;
        }
        // SAFETY: every emitted access lies in the live130-cell bank; no
        // pointer escapes, and both loops execute at most17 traversals.
        let count = unsafe { run(bank.as_mut_ptr() as u64, rounds) };
        assert_eq!(count, if rounds == 0 { 0 } else { 6 });
        assert_eq!(bank, expected, "rounds={rounds}");
    }
}
