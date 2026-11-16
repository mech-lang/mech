use super::*;
use cranelift_codegen::ir::{AbiParam, InstBuilder};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::default_libcall_names;

#[repr(C)]
struct Backing {
    data: u64,
    owner_or_status: u64,
}

extern "C" fn allocate(fail: u64, capacity: u64, _: u64, _: u64, _: u64) -> Backing {
    if fail != 0 {
        return Backing {
            data: 0,
            owner_or_status: 17,
        };
    }
    Backing {
        data: 1,
        owner_or_status: if capacity == 0 { 0 } else { 11 },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Closed,
    BorrowedEntry,
    BorrowedLatch,
    ZeroCapacity,
    SharedSuccess,
}

type Entry = unsafe extern "C" fn(fail: u64, bypass: u64, borrowed: u64, steps: u64) -> u64;

fn compile(shape: Shape, optimize: bool) -> (JITModule, Entry) {
    let mut jit = JITBuilder::new(default_libcall_names()).unwrap();
    jit.symbol("mech_compact_allocate_backing", allocate as *const u8);
    let mut module = JITModule::new(jit);
    let notional_words = AbiParam::new(types::I64);
    let mut signature = module.make_signature();
    signature.params = vec![notional_words; 5];
    signature.returns = vec![notional_words; 2];
    let runtime = module
        .declare_function("mech_compact_allocate_backing", Linkage::Import, &signature)
        .unwrap();
    let mut context = module.make_context();
    context.func.signature.params = vec![notional_words; 4];
    context.func.signature.returns = vec![notional_words];
    let mut storage = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut storage);
    let entry = b.create_block();
    let allocate = b.create_block();
    let accepted = b.create_block();
    let failed = b.create_block();
    let header = b.create_block();
    let present = b.create_block();
    let absent = b.create_block();
    let latch = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    let accepted_owner = b.append_block_param(accepted, types::I64);
    let owner = b.append_block_param(header, types::I64);
    let remaining = b.append_block_param(header, types::I64);
    b.switch_to_block(entry);
    let &[fail, bypass, borrowed, steps] = b.block_params(entry) else {
        unreachable!()
    };
    match shape {
        Shape::BorrowedEntry => b.ins().brif(
            bypass,
            header,
            &[borrowed.into(), steps.into()],
            allocate,
            &[],
        ),
        Shape::SharedSuccess => b
            .ins()
            .brif(bypass, accepted, &[borrowed.into()], allocate, &[]),
        _ => b.ins().jump(allocate, &[]),
    };
    b.switch_to_block(allocate);
    let capacity = b
        .ins()
        .iconst(types::I64, if shape == Shape::ZeroCapacity { 0 } else { 8 });
    let one = b.ins().iconst(types::I64, 1);
    let runtime = module.declare_func_in_func(runtime, b.func);
    let result = b.ins().call(runtime, &[fail, capacity, one, one, one]);
    let &[data, owner_or_status] = b.inst_results(result) else {
        unreachable!()
    };
    b.ins()
        .brif(data, accepted, &[owner_or_status.into()], failed, &[]);
    b.switch_to_block(accepted);
    b.ins().jump(header, &[accepted_owner.into(), steps.into()]);
    b.switch_to_block(failed);
    b.ins().return_(&[owner_or_status]);
    b.switch_to_block(header);
    let missing = b.ins().icmp_imm(IntCC::Equal, owner, 0);
    b.ins().brif(missing, absent, &[], present, &[]);
    b.switch_to_block(absent);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[zero]);
    b.switch_to_block(present);
    b.ins().brif(remaining, latch, &[], done, &[]);
    b.switch_to_block(latch);
    let next = b.ins().iadd_imm(remaining, -1);
    let next_owner = if shape == Shape::BorrowedLatch {
        borrowed
    } else {
        owner
    };
    b.ins().jump(header, &[next_owner.into(), next.into()]);
    b.switch_to_block(done);
    b.ins().return_(&[owner]);
    b.seal_all_blocks();
    b.finalize();
    if optimize {
        let (backings, checks) = fold(&module, &mut context, Scope::All);
        assert_eq!(
            backings,
            usize::from(!matches!(shape, Shape::ZeroCapacity | Shape::SharedSuccess))
        );
        assert_eq!(checks, usize::from(shape == Shape::Closed));
        context.flowgraph();
        context.eliminate_unreachable_code(module.isa()).unwrap();
    }
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "backing_owner_loop",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry)
}

#[test]
fn owner_checks_need_positive_admission_on_every_incoming_edge() {
    for shape in [
        Shape::Closed,
        Shape::BorrowedEntry,
        Shape::BorrowedLatch,
        Shape::ZeroCapacity,
        Shape::SharedSuccess,
    ] {
        let (_before, original) = compile(shape, false);
        let (_after, optimized) = compile(shape, true);
        for fail in [0, 1] {
            for bypass in [0, 1] {
                for borrowed in [0, 125] {
                    for steps in [0, 3] {
                        let expected = unsafe { original(fail, bypass, borrowed, steps) };
                        let actual = unsafe { optimized(fail, bypass, borrowed, steps) };
                        assert_eq!(actual, expected, "{shape:?}");
                    }
                }
            }
        }
    }
}
