use super::*;
use cranelift_codegen::ir::{AbiParam, condcodes::IntCC};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

fn words(home: Home) -> Words {
    Words {
        variables: BTreeMap::from([(3, (home, MemFlags::new()))]),
        literal_write: Cell::new(false),
        tracked: RefCell::new(Vec::new()),
        promote_tracked: false,
        native_extent_slots: BTreeMap::new(),
        native_extent_values: RefCell::new(BTreeMap::new()),
    }
}

#[inline(never)]
extern "C" fn step(value: u64) -> u64 {
    value.wrapping_add(1)
}

#[test]
fn packet_owner_updates_survive_calls_conditional_clears_and_loop_joins() {
    let mut builder = JITBuilder::new(cranelift_module::default_libcall_names()).unwrap();
    builder.symbol("packet_owner_step", step as *const u8);
    let mut module = JITModule::new(builder);
    let mut step_signature = module.make_signature();
    step_signature.params.push(AbiParam::new(types::I64));
    step_signature.returns.push(AbiParam::new(types::I64));
    let step = module
        .declare_function("packet_owner_step", Linkage::Import, &step_signature)
        .unwrap();
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context
        .func
        .signature
        .returns
        .push(AbiParam::new(types::I64));
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
    let entry = b.create_block();
    let header = b.create_block();
    let body = b.create_block();
    let clear = b.create_block();
    let advance = b.create_block();
    let next = b.create_block();
    let exit = b.create_block();
    b.append_block_params_for_function_params(entry);
    let index = b.append_block_param(header, types::I64);
    b.switch_to_block(entry);
    let [packet, count, clear_at] = b.block_params(entry).try_into().unwrap();
    let zero = b.ins().iconst(types::I64, 0);
    let variable = b.declare_var(types::I64);
    b.def_var(variable, zero);
    let mut words = words(Home::Variable(variable));
    words
        .bind_packet_homes(packet, &BTreeMap::from([(3, 1)]))
        .unwrap();
    b.ins().jump(header, &[zero.into()]);
    b.switch_to_block(header);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, count);
    b.ins().brif(more, body, &[], exit, &[]);
    b.switch_to_block(body);
    let selected = b.ins().icmp(IntCC::Equal, index, clear_at);
    b.ins().brif(selected, clear, &[], advance, &[]);
    b.switch_to_block(clear);
    words.write(&mut b, 3, zero);
    b.ins().jump(next, &[]);
    b.switch_to_block(advance);
    let owner = words.read(&mut b, 3);
    let function = module.declare_func_in_func(step, b.func);
    let call = b.ins().call(function, &[owner]);
    let updated = b.inst_results(call)[0];
    words.write(&mut b, 3, updated);
    b.ins().jump(next, &[]);
    b.switch_to_block(next);
    let index = b.ins().iadd_imm(index, 1);
    b.ins().jump(header, &[index.into()]);
    b.switch_to_block(exit);
    let result = words.read(&mut b, 3);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    let function = module
        .declare_function("packet_owner_loop", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(function, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let run: unsafe extern "C" fn(*mut u64, u64, u64) -> u64 =
        unsafe { std::mem::transmute(module.get_finalized_function(function)) };
    for initial in [0, 37, u64::MAX] {
        for count in [0, 1, 3, 8] {
            for clear_at in [0, 1, 4, u64::MAX] {
                let mut packet = [0x1234_5678, initial, 0x9abc_def0];
                let mut expected = initial;
                for index in 0..count {
                    expected = if index == clear_at {
                        0
                    } else {
                        expected.wrapping_add(1)
                    };
                }
                let result = unsafe { run(packet.as_mut_ptr(), count, clear_at) };
                assert_eq!(result, expected);
                assert_eq!(packet, [0x1234_5678, expected, 0x9abc_def0]);
            }
        }
    }
}

#[test]
fn packet_homes_do_not_replace_literals_or_unknown_source_lanes() {
    let mut function = cranelift_codegen::ir::Function::new();
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut frontend);
    let entry = b.create_block();
    b.switch_to_block(entry);
    let zero = b.ins().iconst(types::I64, 0);
    let mut literal = words(Home::Literal(zero));
    assert!(
        literal
            .bind_packet_homes(zero, &BTreeMap::from([(3, 0)]))
            .is_err()
    );
    assert!(
        literal
            .bind_packet_homes(zero, &BTreeMap::from([(4, 0)]))
            .is_err()
    );
    b.ins().return_(&[]);
    b.seal_all_blocks();
    b.finalize();
}
