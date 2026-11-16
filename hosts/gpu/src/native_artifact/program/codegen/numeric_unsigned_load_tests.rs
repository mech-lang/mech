use super::load_unsigned_word;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use cranelift_codegen::ir::{AbiParam, Endianness, InstBuilder, MemFlags, Opcode, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn physical(width: u8) -> cranelift_codegen::ir::Type {
    match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => unreachable!(),
    }
}

fn expected(bytes: &[u8], little: bool) -> u64 {
    if little {
        bytes.iter().enumerate().fold(0, |value, (at, byte)| {
            value | (u64::from(*byte) << (at * 8))
        })
    } else {
        bytes
            .iter()
            .fold(0, |value, byte| (value << 8) | u64::from(*byte))
    }
}

#[test]
fn unsigned_word_load_keeps_physical_width_flags_and_one_i64_definition() {
    for width in [1, 2, 4, 8] {
        for flags in [
            MemFlags::new(),
            MemFlags::new().with_notrap(),
            MemFlags::new().with_endianness(Endianness::Little),
            super::super::numeric_captured_reads::flags(),
        ] {
            let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64)];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            let mut frontend = FunctionBuilderContext::new();
            let value;
            {
                let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
                let entry = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.switch_to_block(entry);
                let pointer = b.block_params(entry)[0];
                value = load_unsigned_word(&mut b, physical(width), flags, pointer, 3);
                b.ins().return_(&[value]);
                b.seal_all_blocks();
                b.finalize();
            }
            assert_eq!(context.func.dfg.value_type(value), types::I64);
            let inst = context.func.dfg.value_def(value).inst().unwrap();
            let cranelift_codegen::ir::InstructionData::Load {
                opcode,
                flags: actual,
                offset,
                ..
            } = context.func.dfg.insts[inst]
            else {
                panic!("expected one physical load")
            };
            assert_eq!(actual, flags);
            assert_eq!(i32::from(offset), 3);
            assert_eq!(
                opcode,
                match width {
                    1 => Opcode::Uload8,
                    2 => Opcode::Uload16,
                    4 => Opcode::Uload32,
                    8 => Opcode::Load,
                    _ => unreachable!(),
                }
            );
            let controls = CompilationControls::new(OptimizationOptions::default());
            super::super::cfg_cleanup::optimize(&controls, &mut context, module.isa()).unwrap();
            context.verify(module.isa()).unwrap();
            let opcodes = context
                .func
                .layout
                .blocks()
                .flat_map(|block| context.func.layout.block_insts(block))
                .map(|inst| context.func.dfg.insts[inst].opcode())
                .collect::<Vec<_>>();
            assert_eq!(opcodes.iter().filter(|&&op| op == opcode).count(), 1);
            assert!(!opcodes.contains(&Opcode::Uextend));
            let id = module
                .declare_function("one_word", Linkage::Local, &context.func.signature)
                .unwrap();
            module.define_function(id, &mut context).unwrap();
        }
    }
}

#[test]
fn unsigned_word_loads_zero_extend_high_bits_unaligned_and_little_endian_cells() {
    type Entry = unsafe extern "C" fn(*const u8) -> u64;
    for width in [1, 2, 4, 8] {
        for little in [false, true] {
            let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64)];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            let mut frontend = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
                let entry = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.switch_to_block(entry);
                let pointer = b.block_params(entry)[0];
                let flags = if little {
                    MemFlags::new().with_endianness(Endianness::Little)
                } else {
                    MemFlags::new()
                };
                let value = load_unsigned_word(&mut b, physical(width), flags, pointer, 0);
                // Consumers can live beyond the source read's block without
                // introducing a second narrow SSA result and later extension.
                for _ in 0..3 {
                    let next = b.create_block();
                    b.ins().jump(next, &[]);
                    b.switch_to_block(next);
                }
                b.ins().return_(&[value]);
                b.seal_all_blocks();
                b.finalize();
            }
            let id = module
                .declare_function("read_word", Linkage::Local, &context.func.signature)
                .unwrap();
            module.define_function(id, &mut context).unwrap();
            module.finalize_definitions().unwrap();
            // SAFETY: the owned function has exactly this pointer-to-word ABI.
            let entry: Entry = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
            for fill in [0, 0x80, 0xff] {
                let mut bytes = [fill; 16];
                for at in 0..width as usize {
                    bytes[1 + at] ^= at as u8;
                }
                // SAFETY: every unaligned physical cell is fully initialized
                // and remains alive through this synchronous call.
                let actual = unsafe { entry(bytes.as_ptr().add(1)) };
                assert_eq!(
                    actual,
                    expected(
                        &bytes[1..1 + width as usize],
                        little || cfg!(target_endian = "little")
                    )
                );
            }
        }
    }
}

#[test]
fn ordinary_unsigned_loads_observe_same_address_writes_without_alias_claims() {
    type Entry = unsafe extern "C" fn(*mut u8, u64) -> u64;
    for width in [1, 2, 4, 8] {
        let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 2];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let mut frontend = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
            let block = b.create_block();
            b.append_block_params_for_function_params(block);
            b.switch_to_block(block);
            let args = b.block_params(block).to_vec();
            let ty = physical(width);
            let old = load_unsigned_word(&mut b, ty, MemFlags::new(), args[0], 0);
            let incoming = if width == 8 {
                args[1]
            } else {
                b.ins().ireduce(ty, args[1])
            };
            b.ins().store(MemFlags::new(), incoming, args[0], 0);
            let new = load_unsigned_word(&mut b, ty, MemFlags::new(), args[0], 0);
            let result = b.ins().bxor(old, new);
            b.ins().return_(&[result]);
            b.seal_all_blocks();
            b.finalize();
        }
        let id = module
            .declare_function("alias_write", Linkage::Local, &context.func.signature)
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: the owned function reads/writes one initialized physical cell.
        let entry: Entry = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let mut bytes = [0xff; 16];
        let mask = u64::MAX >> (64 - width as u32 * 8);
        // SAFETY: the width-byte cell at offset1 and both surrounding bytes live.
        assert_eq!(
            unsafe { entry(bytes.as_mut_ptr().add(1), 0x8040201008040201) },
            mask ^ (0x8040201008040201 & mask)
        );
        assert_eq!(bytes[0], 0xff);
        assert_eq!(
            &bytes[1 + width as usize..],
            &[0xff; 16][1 + width as usize..]
        );
    }
}
