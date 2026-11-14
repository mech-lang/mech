use super::rotate;
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Current,
    ChangingStop,
    EffectBeforeTest,
    ComputedExit,
}

fn fixture(module: &JITModule, shape: Shape) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let outer = b.create_block();
    let seed_visit = b.create_block();
    let header = b.create_block();
    let test = b.create_block();
    let visit = b.create_block();
    let finish = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(outer, types::I64);
    b.append_block_param(header, types::I64);
    b.append_block_param(header, types::I32);
    b.append_block_param(header, types::I64);
    b.switch_to_block(entry);
    let [visits, start, stop, seed, memory] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let seed = b.ins().ireduce(types::I32, seed);
    b.ins().jump(outer, &[visits.into()]);
    b.switch_to_block(outer);
    let remaining = b.block_params(outer)[0];
    b.ins().brif(remaining, seed_visit, &[], done, &[]);
    b.switch_to_block(seed_visit);
    b.ins()
        .jump(header, &[start.into(), seed.into(), stop.into()]);
    b.switch_to_block(header);
    let [at, score, dynamic_stop] = b.block_params(header)[..] else {
        unreachable!()
    };
    let next_score = b.ins().iadd_imm(score, 3);
    if matches!(shape, Shape::EffectBeforeTest) {
        b.ins().store(MemFlags::new(), at, memory, 0);
    }
    b.ins().jump(test, &[]);
    b.switch_to_block(test);
    let bound = if matches!(shape, Shape::ChangingStop) {
        dynamic_stop
    } else {
        stop
    };
    let more = b.ins().icmp(IntCC::NotEqual, at, bound);
    let more = b.ins().uextend(types::I64, more);
    b.ins().brif(more, visit, &[], finish, &[]);
    b.switch_to_block(visit);
    let count = b.ins().load(types::I64, MemFlags::new(), memory, 0);
    let count = b.ins().iadd_imm(count, 1);
    b.ins().store(MemFlags::new(), count, memory, 0);
    let next = b.ins().iadd_imm(at, 1);
    b.ins().jump(
        header,
        &[next.into(), next_score.into(), dynamic_stop.into()],
    );
    b.switch_to_block(finish);
    let result = if matches!(shape, Shape::ComputedExit) {
        next_score
    } else {
        score
    };
    let result = b.ins().uextend(types::I64, result);
    b.ins().store(MemFlags::new(), result, memory, 8);
    b.ins().store(MemFlags::new(), at, memory, 16);
    let remaining = b.ins().iadd_imm(remaining, -1);
    b.ins().jump(outer, &[remaining.into()]);
    b.switch_to_block(done);
    let result = b.ins().load(types::I64, MemFlags::new(), memory, 8);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    context
}

type Entry = unsafe extern "C" fn(u64, u64, u64, u64, *mut u64) -> u64;

fn compile(module: &mut JITModule, context: &mut Context, name: &str) -> Entry {
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the function's declared signature is the Entry signature.
    // The module stays alive through all synchronous calls.
    unsafe { std::mem::transmute(module.get_finalized_function(id)) }
}

#[test]
fn preserves_the_initial_test_typed_results_and_every_reached_visit() {
    let builder = JITBuilder::with_flags(
        &[("opt_level", "speed"), ("enable_verifier", "true")],
        default_libcall_names(),
    )
    .unwrap();
    let mut module = JITModule::new(builder);
    let mut baseline = fixture(&module, Shape::Current);
    let mut candidate = fixture(&module, Shape::Current);
    candidate.flowgraph();
    assert!(rotate(
        &mut candidate.func,
        &candidate.cfg,
        &candidate.domtree
    ));
    candidate.flowgraph();
    candidate.verify(module.isa()).unwrap();
    let baseline = compile(&mut module, &mut baseline, "nested_before");
    let candidate = compile(&mut module, &mut candidate, "nested_after");
    let mut calls = 0;
    for trips in [0, 1, 3, 17] {
        for start in [0u64, 7, u64::MAX - 2] {
            for span in [0u64, 1, 2, 8] {
                for seed in [0u64, 1, u64::from(u32::MAX)] {
                    let stop = start.wrapping_add(span);
                    let expected = if trips == 0 {
                        [0, u64::MAX, 17]
                    } else {
                        [
                            trips * span,
                            (seed as u32).wrapping_add(3 * span as u32).into(),
                            stop,
                        ]
                    };
                    for call in [baseline, candidate] {
                        let mut words = [0, u64::MAX, 17];
                        // SAFETY: the caller owns three valid exclusive words.
                        let result = unsafe { call(trips, start, stop, seed, words.as_mut_ptr()) };
                        assert_eq!((result, words), (expected[1], expected));
                        calls += 1;
                    }
                }
            }
        }
    }
    assert_eq!(calls, 288);
    println!(
        "{calls} native calls preserve zero trips, wrapping induction, current lanes and stores"
    );
}

#[test]
fn refuses_a_changing_stop_observable_corridor_and_computed_exit() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for shape in [
        Shape::ChangingStop,
        Shape::EffectBeforeTest,
        Shape::ComputedExit,
    ] {
        let mut context = fixture(&module, shape);
        context.flowgraph();
        context.verify(module.isa()).unwrap();
        let previous = context.func.display().to_string();
        assert!(
            !rotate(&mut context.func, &context.cfg, &context.domtree),
            "{shape:?}"
        );
        assert_eq!(context.func.display().to_string(), previous);
    }
}
