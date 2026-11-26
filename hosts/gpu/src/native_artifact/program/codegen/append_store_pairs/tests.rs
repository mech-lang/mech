use super::*;
use cranelift_codegen::{
    Context,
    ir::{AbiParam, SourceLoc},
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn body(context: &mut Context, control: usize) -> Vec<Inst> {
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [low, high, output] = *b.block_params(entry) else {
        unreachable!()
    };
    let flags = if control == 6 {
        MemFlags::new().with_aligned()
    } else {
        MemFlags::new()
    };
    let first = b.ins().store(flags, low, output, 0);
    let carry = b.ins().ireduce(types::I32, low);
    if control == 1 {
        let _ = b.ins().load(types::I64, MemFlags::new(), output, 16);
    }
    if control == 2 {
        b.ins().store(MemFlags::new(), high, output, 24);
    }
    if control == 3 {
        b.ins().ireduce(types::I16, low);
    }
    let address = if control == 4 {
        b.ins().iadd_imm(output, 8)
    } else {
        output
    };
    let second_flags = if control == 5 {
        MemFlags::new().with_endianness(Endianness::Big)
    } else {
        flags
    };
    let second = b.ins().store(
        second_flags,
        high,
        address,
        if control == 7 { 16 } else { 8 },
    );
    if control == 8 {
        b.func.set_srcloc(second, SourceLoc::new(1));
    }
    let result = b.ins().uextend(types::I64, carry);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    if control == 9 {
        vec![first]
    } else {
        vec![first, second]
    }
}

#[test]
fn pairs_keep_values_flags_and_guards_at_eight_byte_alignment() {
    for level in ["none", "speed"] {
        let mut flags = settings::builder();
        flags.set("opt_level", level).unwrap();
        let isa = cranelift_native::builder()
            .unwrap()
            .finish(settings::Flags::new(flags))
            .unwrap();
        let mut module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));
        let mut functions = Vec::new();
        for paired in [false, true] {
            let mut context = module.make_context();
            let stores = body(&mut context, 0);
            if paired {
                assert_eq!(fold(&mut context.func, &stores), 1);
                let accesses = context
                    .func
                    .layout
                    .blocks()
                    .flat_map(|block| context.func.layout.block_insts(block))
                    .filter_map(|inst| match context.func.dfg.insts[inst] {
                        Data::Store {
                            args: [value, _],
                            flags,
                            ..
                        } => Some((context.func.dfg.value_type(value), flags)),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(accesses, [(types::I128, MemFlags::new())]);
            }
            context.verify(module.isa()).unwrap();
            let id = module
                .declare_function(
                    if paired { "paired" } else { "ordinary" },
                    Linkage::Export,
                    &context.func.signature,
                )
                .unwrap();
            module.define_function(id, &mut context).unwrap();
            functions.push(id);
        }
        module.finalize_definitions().unwrap();
        let functions = functions
            .into_iter()
            .map(|id| unsafe {
                std::mem::transmute::<_, extern "C" fn(u64, u64, *mut u64) -> u64>(
                    module.get_finalized_function(id),
                )
            })
            .collect::<Vec<_>>();
        #[repr(align(16))]
        struct Output([u64; 4]);
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for index in 0..1024 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let low = match index {
                0 => 0,
                1 => u64::MAX,
                _ => state,
            };
            let high = low.rotate_left(29) ^ 0xa5a5_5a5a_ffff_0000;
            for function in &functions {
                let mut output = Output([0x1234_5678_9abc_def0; 4]);
                let pointer = unsafe { output.0.as_mut_ptr().add(1) };
                assert_eq!(pointer as usize % 16, 8);
                assert_eq!(function(low, high, pointer), u64::from(low as u32));
                assert_eq!(
                    output.0,
                    [0x1234_5678_9abc_def0, low, high, 0x1234_5678_9abc_def0]
                );
            }
        }
    }
}

#[test]
fn refuses_unproved_motion_geometry_flags_and_lineage() {
    for control in 1..=9 {
        let mut context = Context::new();
        let stores = body(&mut context, control);
        let before = context.func.display().to_string();
        assert_eq!(fold(&mut context.func, &stores), 0, "control {control}");
        assert_eq!(context.func.display().to_string(), before);
    }
}
