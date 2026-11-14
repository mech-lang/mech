use super::optimize;
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Shape {
    SharedState,
    DifferentState,
    SharedComparison,
    RawWord,
}

fn fixture(module: &JITModule, shape: Shape) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 4];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let header = b.create_block();
    let stop = b.create_block();
    let step = b.create_block();
    let finish = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(header, types::I64);
    b.append_block_param(finish, types::I64);
    b.switch_to_block(entry);
    let [start, limit, stop_at, memory] = b.block_params(entry)[..] else {
        unreachable!()
    };
    b.ins().jump(header, &[start.into()]);
    b.switch_to_block(header);
    let at = b.block_params(header)[0];
    let next = b.ins().iadd_imm(at, 1);
    let a = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, at, limit);
    let c = b.ins().icmp(IntCC::Equal, at, stop_at);
    let a = b.ins().uextend(types::I64, a);
    let c = b.ins().uextend(types::I64, c);
    let c = if matches!(shape, Shape::RawWord) {
        start
    } else {
        c
    };
    let combined = b.ins().bor(a, c);
    let flag = b.ins().band_imm(combined, 1);
    b.ins().brif(flag, stop, &[], step, &[]);
    b.switch_to_block(step);
    b.ins().store(MemFlags::new(), at, memory, 0);
    b.ins().jump(header, &[next.into()]);
    b.switch_to_block(stop);
    let result = if matches!(shape, Shape::DifferentState) {
        at
    } else {
        next
    };
    b.ins().jump(finish, &[result.into()]);
    b.switch_to_block(finish);
    if matches!(shape, Shape::SharedComparison) {
        b.ins().store(MemFlags::new(), a, memory, 8);
    }
    let result = b.block_params(finish)[0];
    b.ins().return_(&[result]);
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
fn complements_branch_only_integer_comparisons_with_current_loop_state() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut before = fixture(&module, Shape::SharedState);
    let mut after = fixture(&module, Shape::SharedState);
    after.compute_cfg();
    after.compute_domtree();
    assert_eq!(optimize(&mut after.func, &after.domtree), 1);
    after.compute_cfg();
    after.compute_domtree();
    after.verify(module.isa()).unwrap();
    let before = define(&mut module, &mut before, "branch_only_before");
    let after = define(&mut module, &mut after, "branch_only_after");
    let mut checked = 0;
    let words: Vec<u64> = (0..20)
        .chain([u64::MAX - 2, u64::MAX - 1, u64::MAX])
        .collect();
    for &start in &words {
        for &limit in &words {
            for &stop in &words {
                if start < 20 && limit >= u64::MAX - 2 {
                    continue;
                }
                let mut at = start;
                let mut expected = [77, 88];
                while at < limit && at != stop {
                    expected[0] = at;
                    at += 1;
                }
                for f in [before, after] {
                    let mut actual = [77, 88];
                    let result = unsafe { f(start, limit, stop, actual.as_mut_ptr()) };
                    assert_eq!((result, actual), (at.wrapping_add(1), expected));
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 21_574);
    println!("{checked} native calls agree with the independent result and store oracle");
}

#[test]
fn retains_raw_shared_or_unrelated_predicates() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for shape in [
        Shape::DifferentState,
        Shape::SharedComparison,
        Shape::RawWord,
    ] {
        let mut context = fixture(&module, shape);
        context.compute_cfg();
        context.compute_domtree();
        context.verify(module.isa()).unwrap();
        let old = context.func.display().to_string();
        assert_eq!(
            optimize(&mut context.func, &context.domtree),
            0,
            "{shape:?}"
        );
        assert_eq!(context.func.display().to_string(), old);
    }
}
