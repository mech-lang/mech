use super::*;
use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, default_libcall_names};

fn define_leaf(module: &mut JITModule, name: &str, result: i64) -> FuncId {
    let mut context = module.make_context();
    context
        .func
        .signature
        .returns
        .push(AbiParam::new(types::I64));
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let entry = builder.create_block();
    builder.switch_to_block(entry);
    let value = builder.ins().iconst(types::I64, result);
    builder.ins().return_(&[value]);
    builder.seal_all_blocks();
    builder.finalize();
    module.define_function(id, &mut context).unwrap();
    id
}

fn define_phase(
    module: &mut JITModule,
    bodies: &mut PhaseBodies,
    name: &str,
    leaf: FuncId,
) -> (FuncId, Vec<u8>) {
    let mut context = module.make_context();
    context
        .func
        .signature
        .returns
        .push(AbiParam::new(types::I64));
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    let leaf = module.declare_func_in_func(leaf, &mut context.func);
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let entry = builder.create_block();
    builder.switch_to_block(entry);
    let call = builder.ins().call(leaf, &[]);
    let value = builder.inst_results(call)[0];
    builder.ins().return_(&[value]);
    builder.seal_all_blocks();
    builder.finalize();

    let (selected, _) = define_body(module, id, name, &mut context, Some(bodies)).unwrap();
    let buffer = &context.compiled_code().unwrap().buffer;
    assert!(!buffer.relocs().is_empty());
    (selected, buffer.data().to_vec())
}

#[test]
fn identical_machine_bodies_share_only_with_the_same_direct_target() {
    for share_relocations in [false, true] {
        let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let mut bodies = PhaseBodies::new(share_relocations);
        let a = define_leaf(&mut module, "a", 17);
        let b = define_leaf(&mut module, "b", 29);
        let (first, code) = define_phase(&mut module, &mut bodies, "first", a);
        let (same, same_code) = define_phase(&mut module, &mut bodies, "same", a);
        let (different, different_code) = define_phase(&mut module, &mut bodies, "different", b);
        assert_eq!(code, same_code);
        assert_eq!(code, different_code);
        assert_eq!(first == same, share_relocations);
        assert_ne!(first, different);

        module.finalize_definitions().unwrap();
        for (function, expected) in [(first, 17), (same, 17), (different, 29)] {
            let run = unsafe {
                std::mem::transmute::<_, extern "C" fn() -> u64>(
                    module.get_finalized_function(function),
                )
            };
            assert_eq!(run(), expected);
        }
    }
}
