use super::{pack_used_input_words, qualify_input_packet_loads, sink_exit_packet_words};
use cranelift_codegen::ir::types::{I32, I64};
use cranelift_codegen::ir::{AbiParam, Block, Function, Inst, InstBuilder, MemFlags};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    ExitCopy,
    RepeatedCopy,
    DifferentBlock,
    UnusedInput,
    ExtraUse,
    InputEscape,
    InputWrite,
    InputOutside,
    OutputOutside,
    OtherWidth,
    Nonterminal,
}

#[test]
fn input_packet_flags_require_the_complete_nonescaping_load_contract() {
    use cranelift_codegen::ir::InstructionData;
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for shape in [
        Shape::ExitCopy,
        Shape::RepeatedCopy,
        Shape::ExtraUse,
        Shape::UnusedInput,
    ] {
        let mut context = module.make_context();
        build(&mut context.func, shape);
        let expected = if shape == Shape::UnusedInput { 2 } else { 1 };
        assert_eq!(qualify_input_packet_loads(&mut context.func, 2), expected);
        for block in context.func.layout.blocks() {
            for inst in context.func.layout.block_insts(block) {
                if let InstructionData::Load { flags, .. } = context.func.dfg.insts[inst] {
                    assert!(flags.notrap() && flags.readonly() && flags.aligned());
                }
            }
        }
        context.verify(module.isa()).unwrap();
    }
    for shape in [
        Shape::InputEscape,
        Shape::InputWrite,
        Shape::InputOutside,
        Shape::OtherWidth,
    ] {
        let mut context = module.make_context();
        build(&mut context.func, shape);
        let before = context.func.display().to_string();
        assert_eq!(
            qualify_input_packet_loads(&mut context.func, 2),
            0,
            "{shape:?}"
        );
        assert_eq!(context.func.display().to_string(), before);
    }
}

fn build(function: &mut Function, shape: Shape) -> (Inst, Inst, Block) {
    function.signature.params = vec![AbiParam::new(I64); 3];
    function.signature.returns = vec![AbiParam::new(I64)];
    let mut context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(function, &mut context);
    let start = b.create_block();
    let failure = b.create_block();
    let exit = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(start);
    b.switch_to_block(start);
    let [ok, input, output] = b.block_params(start).try_into().unwrap();
    let offset = if shape == Shape::InputOutside { 16 } else { 8 };
    let ty = if shape == Shape::OtherWidth { I32 } else { I64 };
    if shape == Shape::UnusedInput {
        b.ins().load(I64, MemFlags::new(), input, 0);
    }
    let word = b.ins().load(ty, MemFlags::new(), input, offset);
    if shape == Shape::InputEscape {
        b.ins().store(MemFlags::new(), input, output, 0);
    }
    if shape == Shape::InputWrite {
        b.ins().store(MemFlags::new(), word, input, 0);
    }
    b.ins().brif(ok, exit, &[], failure, &[]);
    b.switch_to_block(failure);
    let status = b.ins().iconst(I64, 1);
    b.ins().return_(&[status]);
    b.switch_to_block(exit);
    let offset = if shape == Shape::OutputOutside { 8 } else { 0 };
    let store = b.ins().store(MemFlags::new(), word, output, offset);
    if shape == Shape::RepeatedCopy {
        b.ins().store(MemFlags::new(), word, output, 8);
    }
    if shape == Shape::Nonterminal {
        b.ins().brif(ok, done, &[], failure, &[]);
    } else {
        b.ins().jump(done, &[]);
    }
    b.switch_to_block(done);
    if shape == Shape::DifferentBlock {
        b.ins().store(MemFlags::new(), word, output, 8);
    }
    let status = if shape == Shape::ExtraUse {
        word
    } else {
        b.ins().iconst(I64, 0)
    };
    b.ins().return_(&[status]);
    b.seal_all_blocks();
    b.finalize();
    let cranelift_codegen::ir::ValueDef::Result(load, _) = function.dfg.value_def(word) else {
        unreachable!();
    };
    (load, store, exit)
}

#[test]
fn compact_input_offsets_keep_live_words_and_refuse_escapes() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut escaped = module.make_context();
    build(&mut escaped.func, Shape::InputEscape);
    assert!(pack_used_input_words(&mut escaped.func, 2).is_none());

    let mut context = module.make_context();
    build(&mut context.func, Shape::UnusedInput);
    assert_eq!(pack_used_input_words(&mut context.func, 2), Some(vec![1]));
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "read_compact_input",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    type Call = unsafe extern "C" fn(u64, *const u64, *mut u64) -> u64;
    let call =
        unsafe { core::mem::transmute::<*const u8, Call>(module.get_finalized_function(id)) };
    let input = [u64::MAX - 9];
    let mut output = 0;
    assert_eq!(unsafe { call(0, input.as_ptr(), &mut output) }, 1);
    assert_eq!(output, 0);
    assert_eq!(unsafe { call(1, input.as_ptr(), &mut output) }, 0);
    assert_eq!(output, input[0]);
}

#[test]
fn exit_packet_words_keep_selected_results_and_failure_order() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut single = module.make_context();
    build(&mut single.func, Shape::ExitCopy);
    assert_eq!(sink_exit_packet_words(&mut single.func, 2, 1, false), 1);
    let mut context = module.make_context();
    let (load, store, exit) = build(&mut context.func, Shape::RepeatedCopy);
    assert_eq!(sink_exit_packet_words(&mut context.func, 2, 2, false), 0);
    assert_eq!(sink_exit_packet_words(&mut context.func, 2, 2, true), 1);
    assert_eq!(context.func.layout.inst_block(load), Some(exit));
    assert_eq!(context.func.layout.next_inst(load), Some(store));
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("copy_exit_packet", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    type Call = unsafe extern "C" fn(u64, *const u64, *mut u64) -> u64;
    // The verified function takes two nonoverlapping live packets with these sizes.
    let call =
        unsafe { core::mem::transmute::<*const u8, Call>(module.get_finalized_function(id)) };
    let input = [77u64, u64::MAX - 123];
    let mut output = [0u64; 2];
    assert_eq!(unsafe { call(0, input.as_ptr(), output.as_mut_ptr()) }, 1);
    assert_eq!(output, [0, 0]);
    assert_eq!(unsafe { call(1, input.as_ptr(), output.as_mut_ptr()) }, 0);
    assert_eq!(output, [input[1]; 2]);
    assert_eq!(input, [77u64, u64::MAX - 123]);
}

#[test]
fn exit_packet_words_refuse_escape_extent_width_and_path_changes() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for shape in [
        Shape::ExtraUse,
        Shape::InputEscape,
        Shape::InputWrite,
        Shape::InputOutside,
        Shape::OutputOutside,
        Shape::OtherWidth,
        Shape::Nonterminal,
    ] {
        let mut context = module.make_context();
        let (load, _, _) = build(&mut context.func, shape);
        let original = context.func.layout.inst_block(load);
        assert_eq!(
            sink_exit_packet_words(&mut context.func, 2, 1, true),
            0,
            "{shape:?}"
        );
        assert_eq!(context.func.layout.inst_block(load), original);
        context.verify(module.isa()).unwrap();
    }
    let mut context = module.make_context();
    let (load, _, _) = build(&mut context.func, Shape::DifferentBlock);
    let original = context.func.layout.inst_block(load);
    assert_eq!(sink_exit_packet_words(&mut context.func, 2, 2, true), 0);
    assert_eq!(context.func.layout.inst_block(load), original);
    context.verify(module.isa()).unwrap();
}
