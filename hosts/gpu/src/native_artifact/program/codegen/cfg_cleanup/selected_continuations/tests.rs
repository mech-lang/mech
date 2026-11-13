use super::optimize;
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Control {
    Normal,
    RawCondition,
    SharedArm,
    InterveningStore,
    EscapingWord,
}

fn fixture(module: &JITModule, control: Control) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let header = b.create_block();
    let yes = b.create_block();
    let no = b.create_block();
    let merge = b.create_block();
    let again = b.create_block();
    let exit = b.create_block();
    let other_exit = b.create_block();
    let dispatch = b.create_block();
    let loop_check = b.create_block();

    b.append_block_params_for_function_params(entry);
    b.append_block_param(header, types::I64);
    b.append_block_param(merge, types::I64);

    b.append_block_param(exit, types::I64);

    b.switch_to_block(entry);
    let [start, limit, fail_at, salt, output] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let enter = b.ins().icmp_imm(IntCC::Equal, start, 0);
    b.ins()
        .brif(enter, other_exit, &[], header, &[start.into()]);

    b.switch_to_block(header);
    let position = b.block_params(header)[0];
    let next = b.ins().iadd_imm(position, 1);
    let at_limit = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, position, limit);
    let at_failure = b.ins().icmp(IntCC::Equal, position, fail_at);
    let at_limit = b.ins().uextend(types::I64, at_limit);
    let at_failure = b.ins().uextend(types::I64, at_failure);
    let stop = b.ins().bor(at_limit, at_failure);
    let stopped = if matches!(control, Control::RawCondition) {
        b.ins().bor(stop, salt)
    } else {
        stop
    };
    let decision = b.ins().band_imm(stopped, 1);
    if matches!(control, Control::SharedArm) {
        let extra = b.ins().icmp_imm(IntCC::Equal, salt, 17);
        b.ins().brif(extra, yes, &[], dispatch, &[]);
        b.switch_to_block(dispatch);
    }
    b.ins().brif(decision, yes, &[], no, &[]);

    b.switch_to_block(yes);
    let final_value = b.ins().bxor(start, salt);
    b.ins().store(MemFlags::new(), final_value, output, 0);
    b.ins().jump(merge, &[start.into()]);

    b.switch_to_block(no);
    b.ins().store(MemFlags::new(), position, output, 8);
    b.ins().jump(merge, &[next.into()]);

    b.switch_to_block(merge);
    let word = b.block_params(merge)[0];
    if matches!(control, Control::InterveningStore) {
        b.ins().store(MemFlags::new(), word, output, 16);
    }
    let walking = b.ins().bxor_imm(stopped, 1);
    let walking = b.ins().band_imm(walking, 1);
    let more = if matches!(control, Control::EscapingWord) {
        loop_check
    } else {
        again
    };
    b.ins().brif(walking, more, &[], exit, &[word.into()]);
    if matches!(control, Control::EscapingWord) {
        // Both original exit edges pass through the same definition.
        // The selected route can skip it only if the exit uses its argument.
        b.switch_to_block(loop_check);
        let retry = b.ins().band_imm(walking, 1);
        b.ins().brif(retry, again, &[], exit, &[word.into()]);
    }
    b.switch_to_block(again);
    b.ins().jump(header, &[word.into()]);

    b.switch_to_block(exit);
    let argument = b.block_params(exit)[0];
    let result = if matches!(control, Control::EscapingWord) {
        b.ins().iadd(argument, word)
    } else {
        argument
    };
    b.ins().return_(&[result]);
    b.switch_to_block(other_exit);
    if matches!(control, Control::EscapingWord) {
        b.ins().jump(header, &[start.into()]);
    } else {
        b.ins().jump(exit, &[start.into()]);
    }
    b.seal_all_blocks();
    b.finalize();
    context
}

type Entry = unsafe extern "C" fn(u64, u64, u64, u64, *mut u64) -> u64;

fn define(module: &mut JITModule, context: &mut Context, name: &str) -> Entry {
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, context).unwrap();
    module.finalize_definitions().unwrap();
    unsafe { std::mem::transmute(module.get_finalized_function(id)) }
}

#[test]
fn forwards_current_stop_and_continue_edges_without_skipping_stores() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut before = fixture(&module, Control::Normal);
    let mut after = fixture(&module, Control::Normal);
    after.verify(module.isa()).unwrap();
    after.compute_cfg();
    after.compute_domtree();
    assert_eq!(optimize(&mut after.func, &after.cfg, &after.domtree), 2);
    after.compute_cfg();
    after.compute_domtree();
    after.verify(module.isa()).unwrap();
    let before = define(&mut module, &mut before, "selected_before");
    let after = define(&mut module, &mut after, "selected_after");
    let mut checked = 0;
    for start in 0..24u64 {
        for limit in 0..24 {
            for stop in 0..24 {
                for salt in [0, 0x47, u64::MAX] {
                    let mut memory = [4, 8, 15];
                    if start != 0 {
                        let mut at = start;
                        while at < limit && at != stop {
                            memory[1] = at;
                            at += 1;
                        }
                        memory[0] = start ^ salt;
                    }
                    for f in [before, after] {
                        let mut actual = [4, 8, 15];
                        let result = unsafe { f(start, limit, stop, salt, actual.as_mut_ptr()) };
                        assert_eq!((result, actual), (start, memory));
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, 82_944);
    println!("{checked} native calls agree with the independent store and result oracle");
}

#[test]
fn refuses_raw_flags_shared_arms_intervening_effects_and_escaping_values() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for control in [
        Control::RawCondition,
        Control::SharedArm,
        Control::InterveningStore,
        Control::EscapingWord,
    ] {
        let mut context = fixture(&module, control);
        context.verify(module.isa()).unwrap();
        context.compute_cfg();
        context.compute_domtree();
        let prior = context.func.display().to_string();
        assert_eq!(
            optimize(&mut context.func, &context.cfg, &context.domtree),
            0,
            "{control:?}"
        );
        assert_eq!(context.func.display().to_string(), prior, "{control:?}");
    }
}
