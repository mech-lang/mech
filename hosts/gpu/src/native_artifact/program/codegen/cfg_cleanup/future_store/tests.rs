use super::*;
use cranelift_codegen::{
    Context,
    ir::{AbiParam, MemFlags, condcodes::IntCC},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy)]
enum Shape {
    Ordinary,
    WideNext,
    ConditionalEdge,
    LoopLocalBase,
}

fn fixture(module: &JITModule, shape: Shape) -> (Context, [Write; 2]) {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let header = b.create_block();
    let work = b.create_block();
    let update = b.create_block();
    let relay = b.create_block();
    let done = b.create_block();
    let failure = b.create_block();
    b.append_block_params_for_function_params(entry);
    for block in [header, relay] {
        for _ in 0..2 {
            b.append_block_param(block, types::I64);
        }
    }
    b.switch_to_block(entry);
    let [base, length, count, seed, reject] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let enough = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, 600);
    b.ins()
        .brif(enough, header, &[seed.into(), count.into()], failure, &[]);

    b.switch_to_block(header);
    let [index, remaining] = b.block_params(header)[..] else {
        unreachable!()
    };
    let more = b.ins().icmp_imm(IntCC::NotEqual, remaining, 0);
    b.ins().brif(more, work, &[], done, &[]);
    b.switch_to_block(done);
    b.ins().return_(&[index]);

    b.switch_to_block(work);
    b.ins().brif(reject, failure, &[], update, &[]);
    b.switch_to_block(failure);
    let error = b.ins().iconst(types::I64, -1);
    b.ins().return_(&[error]);

    b.switch_to_block(update);
    let masked = b.ins().band_imm(index, 255);
    let base = if matches!(shape, Shape::LoopLocalBase) {
        b.ins().iadd_imm(base, 0)
    } else {
        base
    };
    let mut make_write = |stride, bias, site| {
        let scaled = b.ins().imul_imm(masked, stride);
        let one_based = b.ins().iadd_imm(scaled, bias);
        let ordinal = b.ins().iadd_imm(one_based, -1);
        let bytes = b.ins().imul_imm(ordinal, 2);
        let address = b.ins().iadd(base, bytes);
        let incoming = b.ins().ireduce(types::I16, remaining);
        let store = b.ins().store(MemFlags::new(), incoming, address, 0);
        Write {
            store,
            array: 0,
            site,
            width: 2,
        }
    };
    let writes = [make_write(2, 1, 1), make_write(1, 301, 2)];
    let product = b.ins().imul_imm(index, 109);
    let next = if matches!(shape, Shape::WideNext) {
        product
    } else {
        let low = b.ins().band_imm(product, 65535);
        let sixteen = b.ins().iconst(types::I64, 16);
        let eight = b.ins().iconst(types::I64, 8);
        let difference = b.ins().isub(sixteen, eight);
        let shift = b.ins().band_imm(difference, 63);
        b.ins().ushr(low, shift)
    };
    let left = b.ins().iadd_imm(remaining, -1);
    if matches!(shape, Shape::ConditionalEdge) {
        b.ins()
            .brif(more, relay, &[next.into(), left.into()], done, &[]);
    } else {
        b.ins().jump(relay, &[next.into(), left.into()]);
    }
    b.switch_to_block(relay);
    let [next, left] = b.block_params(relay)[..] else {
        unreachable!()
    };
    b.ins().jump(header, &[next.into(), left.into()]);
    b.seal_all_blocks();
    b.finalize();
    (context, writes)
}

type Entry = unsafe extern "C" fn(*mut u16, u64, u64, u64, u64) -> u64;

fn define(module: &mut JITModule, context: &mut Context, name: &str) -> Entry {
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, context).unwrap();
    module.finalize_definitions().unwrap();
    unsafe { std::mem::transmute(module.get_finalized_function(id)) }
}

fn apply(context: &mut Context, writes: &[Write], lengths: &[(usize, u64)]) -> Option<Hint> {
    context.flowgraph();
    insert(
        &mut context.func,
        &context.cfg,
        &context.domtree,
        writes,
        lengths,
    )
}

#[test]
fn future_store_prefers_the_bounded_dense_returning_write() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let (mut before, _) = fixture(&module, Shape::Ordinary);
    let (mut after, writes) = fixture(&module, Shape::Ordinary);
    let hint = apply(&mut after, &writes, &[(0, 600)]).unwrap();
    assert_eq!(
        (
            hint.source_write,
            hint.byte_offset,
            hint.byte_stride,
            hint.selector_mask
        ),
        (2, 600, 2, 255)
    );
    after.verify(module.isa()).unwrap();
    let before = define(&mut module, &mut before, "future_store_before");
    let after = define(&mut module, &mut after, "future_store_after");
    for seed in [0, 1, 2, 127, 255, 256, u64::MAX] {
        for count in [0, 1, 2, 17, 300] {
            let mut expected = [0xface; 600];
            let mut actual = expected;
            for (length, reject) in [(600, 0), (599, 0), (600, 1), (600, 0)] {
                let a = unsafe { before(expected.as_mut_ptr(), length, count, seed, reject) };
                let b = unsafe { after(actual.as_mut_ptr(), length, count, seed, reject) };
                assert_eq!((a, expected), (b, actual));
            }
        }
    }
}

#[test]
fn future_store_declines_unbounded_or_unadmitted_addresses() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for shape in [
        Shape::WideNext,
        Shape::ConditionalEdge,
        Shape::LoopLocalBase,
    ] {
        let (mut context, writes) = fixture(&module, shape);
        assert!(apply(&mut context, &writes, &[(0, 600)]).is_none());
    }
    let (mut context, writes) = fixture(&module, Shape::Ordinary);
    assert!(apply(&mut context, &writes, &[]).is_none());
    assert!(apply(&mut context, &writes, &[(0, 500)]).is_none());
    let wrong_width = Write {
        width: 1,
        ..writes[1]
    };
    assert!(apply(&mut context, &[wrong_width], &[(0, 600)]).is_none());
}
