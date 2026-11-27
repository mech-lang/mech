use super::*;
use cranelift_codegen::ir::{AbiParam, Endianness};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, default_libcall_names};

type Entry = unsafe extern "C" fn(*const u64, u64, u64) -> u64;

fn fixture(
    module: &JITModule,
    notrap: bool,
    changed_pointer: bool,
) -> (Context, BTreeMap<Value, (Inst, usize)>) {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
    let entry = b.create_block();
    let early = b.create_block();
    let late = b.create_block();
    let short = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(done, types::I64);
    b.switch_to_block(entry);
    let packet = b.block_params(entry)[0];
    let index = b.block_params(entry)[1];
    let enabled = b.block_params(entry)[2];
    let pointer = b.ins().load(types::I64, MemFlags::new(), packet, 0);
    let length = b.ins().load(types::I64, MemFlags::new(), packet, 8);
    let loads = [(pointer, 0), (length, 1)]
        .into_iter()
        .map(|(value, word)| {
            let ValueDef::Result(inst, 0) = b.func.dfg.value_def(value) else {
                unreachable!()
            };
            (value, (inst, word))
        })
        .collect();
    let next = b.ins().iadd_imm(index, 1);
    let enough = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, 4);
    let last = b.ins().iadd_imm(length, -4);
    let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, next, last);
    let valid = b.ins().band(enough, fits);
    b.ins().brif(valid, early, &[], short, &[]);
    b.switch_to_block(short);
    let zero = b.ins().iconst(types::I64, 0x1357);
    b.ins().jump(done, &[zero.into()]);
    b.switch_to_block(early);
    let address = b.ins().iadd(pointer, next);
    let flags = MemFlags::new()
        .with_notrap()
        .with_endianness(Endianness::Little);
    let first = b.ins().uload32(flags, address, 0);
    let use_late = b.ins().icmp_imm(IntCC::NotEqual, enabled, 0);
    let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, index, last);
    let use_late = b.ins().band(use_late, fits);
    b.ins().brif(use_late, late, &[], done, &[first.into()]);
    b.switch_to_block(late);
    let pointer = if changed_pointer {
        b.ins().iadd_imm(pointer, 1)
    } else {
        pointer
    };
    let address = b.ins().iadd(pointer, index);
    let flags = if notrap {
        flags
    } else {
        MemFlags::new().with_endianness(Endianness::Little)
    };
    let second = b.ins().uload32(flags, address, 0);
    let result = b.ins().bxor(first, second);
    b.ins().jump(done, &[result.into()]);
    b.switch_to_block(done);
    let result = b.block_params(done)[0];
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    (ctx, loads)
}

fn view(width: u8) -> View {
    View {
        array: 0,
        pointer_word: 0,
        length_word: 1,
        width,
        literal: None,
    }
}

fn compile(module: &mut JITModule, name: &str, after: bool, delayed: bool) -> Entry {
    let (mut ctx, loads) = fixture(module, true, false);
    if after {
        let moved = if delayed {
            schedule_delayed(module, &mut ctx, &loads, &[view(1)])
        } else {
            schedule(module, &mut ctx, &loads, &[view(1)])
        }
        .unwrap();
        assert_eq!(moved, 1);
        ctx.flowgraph();
        let mut ordinary = None;
        let mut placed = None;
        for block in ctx.func.layout.blocks() {
            for inst in ctx.func.layout.block_insts(block) {
                if let Data::Load {
                    opcode: Opcode::Uload32,
                    flags,
                    ..
                } = ctx.func.dfg.insts[inst]
                {
                    if flags.readonly() {
                        placed = Some(inst);
                    } else {
                        ordinary = Some(inst);
                    }
                }
            }
        }
        let (ordinary, placed) = (ordinary.unwrap(), placed.unwrap());
        assert!(if delayed {
            ctx.domtree.dominates(ordinary, placed, &ctx.func.layout)
        } else {
            ctx.domtree.dominates(placed, ordinary, &ctx.func.layout)
        });
    }
    ctx.verify(module.isa()).unwrap();
    let id = module
        .declare_function(name, Linkage::Local, &ctx.func.signature)
        .unwrap();
    module.define_function(id, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();
    // The module owns this exact packet-pointer, index, enabled ABI throughout use.
    unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) }
}

#[test]
fn immutable_word_placement_preserves_bounds_defaults_and_current_reads() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let before = compile(&mut module, "immutable_word_before", false, false);
    let alternatives = [
        compile(&mut module, "immutable_word_early", true, false),
        compile(&mut module, "immutable_word_delayed", true, true),
    ];
    let bytes = (0..512)
        .map(|i| ((i * 131 + i / 7) & 255) as u8)
        .collect::<Vec<_>>();
    let mut calls = 0;
    for length in (0..32).chain([128, 512]) {
        let packet = [
            if length == 0 {
                0
            } else {
                bytes.as_ptr() as u64
            },
            length,
        ];
        for index in [
            0,
            1,
            2,
            3,
            4,
            7,
            15,
            28,
            29,
            30,
            31,
            32,
            63,
            127,
            128,
            509,
            510,
            511,
            512,
            u64::MAX - 1,
            u64::MAX,
        ] {
            for enabled in [0, 1, 2, u64::MAX] {
                let next = index.wrapping_add(1);
                let expected = if length < 4 || next > length - 4 {
                    0x1357
                } else {
                    let read = |i: u64| {
                        u64::from(u32::from_le_bytes(
                            bytes[i as usize..i as usize + 4].try_into().unwrap(),
                        ))
                    };
                    let first = read(next);
                    if enabled != 0 && index <= length - 4 {
                        first ^ read(index)
                    } else {
                        first
                    }
                };
                // Bounds cover all physical reads; the empty view deliberately has a null pointer.
                for after in alternatives {
                    let (a, b) = unsafe {
                        (
                            before(packet.as_ptr(), index, enabled),
                            after(packet.as_ptr(), index, enabled),
                        )
                    };
                    assert_eq!(
                        (a, b),
                        (expected, expected),
                        "length={length} index={index} enabled={enabled}"
                    );
                    calls += 2;
                }
            }
        }
    }
    assert_eq!(calls, 11424);
}

#[test]
fn immutable_word_placement_requires_the_exact_private_view_and_nontrapping_read() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for (notrap, changed_pointer, views) in [
        (false, false, vec![view(1)]),
        (true, true, vec![view(1)]),
        (true, false, vec![]),
        (true, false, vec![view(2)]),
        (
            true,
            false,
            vec![View {
                pointer_word: 9,
                ..view(1)
            }],
        ),
        (
            true,
            false,
            vec![View {
                length_word: 9,
                ..view(1)
            }],
        ),
    ] {
        let (mut ctx, loads) = fixture(&module, notrap, changed_pointer);
        assert_eq!(schedule(&mut module, &mut ctx, &loads, &views).unwrap(), 0);
        ctx.verify(module.isa()).unwrap();
    }
}
