use super::fold;
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Exhausted,
    DynamicAdmission,
    ChangedStop,
    Effect,
    OtherEntry,
}

fn fixture(module: &JITModule, shape: Shape) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 4];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let admitted = b.create_block();
    let refused = b.create_block();
    let header = b.create_block();
    let body = b.create_block();
    let finish = b.create_block();
    b.append_block_params_for_function_params(entry);
    for ty in [types::I64; 2] {
        b.append_block_param(refused, ty);
        b.append_block_param(header, ty);
    }
    b.append_block_param(finish, types::I64);
    b.switch_to_block(entry);
    let [start, stop, seed, memory] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let more = b.ins().icmp(IntCC::UnsignedLessThan, start, stop);
    let tag = if matches!(shape, Shape::DynamicAdmission) {
        seed
    } else {
        b.ins().iconst(types::I64, 0)
    };
    let active = b.ins().icmp_imm(IntCC::Equal, tag, 0);
    let fits = b.ins().iconst(types::I8, 1);
    let valid = b.ins().band(active, more);
    let valid = b.ins().band(valid, fits);
    b.ins()
        .brif(valid, admitted, &[], refused, &[start.into(), seed.into()]);
    b.switch_to_block(admitted);
    let next = b.ins().iadd_imm(seed, 17);
    if matches!(shape, Shape::OtherEntry) {
        b.ins().brif(
            more,
            finish,
            &[next.into()],
            refused,
            &[start.into(), seed.into()],
        );
    } else {
        b.ins().jump(finish, &[next.into()]);
    }
    b.switch_to_block(refused);
    let [start_again, seed_again] = b.block_params(refused)[..] else {
        unreachable!()
    };
    b.ins()
        .jump(header, &[start_again.into(), seed_again.into()]);
    b.switch_to_block(header);
    let [at, value] = b.block_params(header)[..] else {
        unreachable!()
    };
    if matches!(shape, Shape::Effect) {
        b.ins().store(MemFlags::new(), value, memory, 8);
    }
    let limit = if matches!(shape, Shape::ChangedStop) {
        seed
    } else {
        stop
    };
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, limit);
    let more = b.ins().uextend(types::I64, more);
    b.ins().brif(more, body, &[], finish, &[value.into()]);
    b.switch_to_block(body);
    b.ins().store(MemFlags::new(), at, memory, 8);
    let next = b.ins().iadd_imm(at, 1);
    let updated = b.ins().iadd_imm(value, 3);
    b.ins().jump(header, &[next.into(), updated.into()]);
    b.switch_to_block(finish);
    let value = b.block_params(finish)[0];
    b.ins().store(MemFlags::new(), value, memory, 0);
    b.ins().return_(&[value]);
    b.seal_all_blocks();
    b.finalize();
    context
}

type Entry = unsafe extern "C" fn(u64, u64, u64, *mut u64) -> u64;

fn define(module: &mut JITModule, context: &mut Context, name: &str) -> Entry {
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, context).unwrap();
    module.finalize_definitions().unwrap();
    unsafe { std::mem::transmute(module.get_finalized_function(id)) }
}

#[test]
fn exhausted_edge_forwards_current_values_to_the_original_effect() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut before = fixture(&module, Shape::Exhausted);
    let mut after = fixture(&module, Shape::Exhausted);
    after.flowgraph();
    assert!(fold(&mut after.func, &after.cfg, &after.domtree));
    after.flowgraph();
    after.verify(module.isa()).unwrap();
    let before = define(&mut module, &mut before, "zero_trip_before");
    let after = define(&mut module, &mut after, "zero_trip_after");
    let words: Vec<u64> = (0..16).chain([u64::MAX - 1, u64::MAX]).collect();
    let mut calls = 0;
    for &start in &words {
        for &stop in &words {
            for &seed in &words {
                let expected = if start < stop {
                    seed.wrapping_add(17)
                } else {
                    seed
                };
                for entry in [before, after] {
                    let mut memory = [u64::MAX; 2];
                    let got = unsafe { entry(start, stop, seed, memory.as_mut_ptr()) };
                    assert_eq!((got, memory), (expected, [expected, u64::MAX]));
                    calls += 1;
                }
            }
        }
    }
    assert_eq!(calls, 11664);
    println!("{calls} compiled calls agree on the current typed result and original store");
}

#[test]
fn refuses_another_admission_failure_changed_limit_effect_or_entry() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for shape in [
        Shape::DynamicAdmission,
        Shape::ChangedStop,
        Shape::Effect,
        Shape::OtherEntry,
    ] {
        let mut context = fixture(&module, shape);
        context.flowgraph();
        context.verify(module.isa()).unwrap();
        let previous = context.func.display().to_string();
        assert!(
            !fold(&mut context.func, &context.cfg, &context.domtree),
            "{shape:?}"
        );
        assert_eq!(context.func.display().to_string(), previous);
    }
}
