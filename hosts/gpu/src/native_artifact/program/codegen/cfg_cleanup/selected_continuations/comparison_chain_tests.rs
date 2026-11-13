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
    BranchOnly,
    RawWord,
    Effect,
    EscapingValue,
    OtherEntry,
}

fn fixture(module: &JITModule, shape: Shape) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let start = b.create_block();
    let first = b.create_block();
    let second = b.create_block();
    let third = b.create_block();
    let end = b.create_block();
    let yes = b.create_block();
    let no = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(done, types::I64);
    b.switch_to_block(entry);
    let [input, memory] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let too_large = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, input, 32);
    let excluded = b.ins().icmp_imm(IntCC::Equal, input, 19);
    let too_large = b.ins().uextend(types::I64, too_large);
    let excluded = b.ins().uextend(types::I64, excluded);
    let shared = if matches!(shape, Shape::RawWord) {
        b.ins().bor(input, excluded)
    } else {
        b.ins().bor(too_large, excluded)
    };
    b.ins().jump(start, &[]);
    b.switch_to_block(start);
    let inverted = b.ins().bxor_imm(shared, 1);
    b.ins().jump(first, &[]);
    b.switch_to_block(first);
    let enough = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, input, 7);
    let enough = b.ins().uextend(types::I64, enough);
    let left = b.ins().band(inverted, enough);
    b.ins().jump(second, &[]);
    b.switch_to_block(second);
    let distance = b.ins().iadd_imm(input, -2);
    if matches!(shape, Shape::Effect) {
        b.ins().store(MemFlags::new(), distance, memory, 16);
    }
    let inside = b.ins().icmp_imm(IntCC::UnsignedLessThan, distance, 24);
    let inside = b.ins().uextend(types::I64, inside);
    let left = b.ins().band(left, inside);
    b.ins().jump(third, &[]);
    b.switch_to_block(third);
    let allowed = b.ins().icmp_imm(IntCC::NotEqual, input, 17);
    let allowed = b.ins().uextend(types::I64, allowed);
    let conjunction = b.ins().band(left, allowed);
    b.ins().jump(end, &[]);
    b.switch_to_block(end);
    let condition = b.ins().band_imm(conjunction, 1);
    b.ins().brif(condition, yes, &[], no, &[]);
    b.switch_to_block(yes);
    b.ins().store(MemFlags::new(), input, memory, 0);
    let one = b.ins().iconst(types::I64, 1);
    if matches!(shape, Shape::OtherEntry) {
        b.ins().brif(shared, first, &[], done, &[one.into()]);
    } else {
        b.ins().jump(done, &[one.into()]);
    }
    b.switch_to_block(no);
    if matches!(shape, Shape::EscapingValue) {
        b.ins().store(MemFlags::new(), distance, memory, 16);
    }
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(done, &[zero.into()]);
    b.switch_to_block(done);
    b.ins().store(MemFlags::new(), shared, memory, 8);
    let result = b.block_params(done)[0];
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    context
}

type Entry = unsafe extern "C" fn(u64, *mut u64) -> u64;

fn define(module: &mut JITModule, context: &mut Context, name: &str) -> Entry {
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, context).unwrap();
    module.finalize_definitions().unwrap();
    unsafe { std::mem::transmute(module.get_finalized_function(id)) }
}

#[test]
fn short_circuits_only_pure_comparisons_and_keeps_the_shared_word() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut baseline = fixture(&module, Shape::BranchOnly);
    let mut optimized = fixture(&module, Shape::BranchOnly);
    optimized.flowgraph();
    assert_eq!(
        fold(&mut optimized.func, &optimized.cfg, &optimized.domtree),
        1
    );
    optimized.flowgraph();
    optimized.verify(module.isa()).unwrap();
    let baseline = define(&mut module, &mut baseline, "comparison_chain_baseline");
    let optimized = define(&mut module, &mut optimized, "comparison_chain_optimized");
    let inputs = (0..4096).chain([u64::MAX - 1, u64::MAX]);
    let mut calls = 0;
    for input in inputs {
        let stop = input > 32 || input == 19;
        let take = !stop && input > 7 && input.wrapping_sub(2) < 24 && input != 17;
        let expected = [if take { input } else { u64::MAX }, u64::from(stop)];
        for entry in [baseline, optimized] {
            let mut memory = [u64::MAX; 2];
            let result = unsafe { entry(input, memory.as_mut_ptr()) };
            assert_eq!((result, memory), (u64::from(take), expected));
            calls += 1;
        }
    }
    assert_eq!(calls, 8196);
    println!("{calls} original and transformed native calls match the effect and value oracle");
}

#[test]
fn declines_raw_words_effects_escapes_and_alternate_paths() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for shape in [
        Shape::RawWord,
        Shape::Effect,
        Shape::EscapingValue,
        Shape::OtherEntry,
    ] {
        let mut context = fixture(&module, shape);
        context.flowgraph();
        context.verify(module.isa()).unwrap();
        let previous = context.func.display().to_string();
        assert_eq!(
            fold(&mut context.func, &context.cfg, &context.domtree),
            0,
            "{shape:?}"
        );
        assert_eq!(context.func.display().to_string(), previous);
    }
}
