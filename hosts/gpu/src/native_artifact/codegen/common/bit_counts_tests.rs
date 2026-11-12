//! Native semantic contracts for exact idioms, live intermediates and packet gaps.
use super::*;
use cranelift_codegen::ir::{AbiParam, MemFlags, SourceLoc, Type};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Exact,
    Mask,
    Shift,
    Multiply,
    Packet,
}
fn build(
    module: &mut JITModule,
    bits: u32,
    shape: Shape,
    selected: bool,
    serial: usize,
) -> cranelift_module::FuncId {
    let ty = Type::int(bits as u16).unwrap();
    let mut cx = module.make_context();
    cx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    cx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let id = module
        .declare_function(
            &format!("bit_count_contract_{serial}"),
            Linkage::Local,
            &cx.func.signature,
        )
        .unwrap();
    let mut fc = FunctionBuilderContext::new();
    let root_inst;
    {
        let mut b = FunctionBuilder::new(&mut cx.func, &mut fc);
        let start = b.create_block();
        let body = b.create_block();
        let error = b.create_block();
        b.append_block_params_for_function_params(start);
        b.switch_to_block(start);
        let x = b.block_params(start)[0];
        let out = b.block_params(start)[1];
        let ok = b.block_params(start)[2];
        b.ins().brif(ok, body, &[], error, &[]);
        b.switch_to_block(body);
        let x = if bits == 64 {
            x
        } else {
            b.ins().ireduce(ty, x)
        };
        let shifted = b.ins().ushr_imm(x, 1);
        let sub = b.ins().band_imm(shifted, 0x5555_5555_5555_5555);
        let a = b.ins().isub(x, sub);
        let lo = b.ins().band_imm(a, 0x3333_3333_3333_3333);
        let hi = b.ins().ushr_imm(a, 2);
        let hi = b.ins().band_imm(hi, 0x3333_3333_3333_3333);
        let z = b.ins().iadd(lo, hi);
        let z = if matches!(shape, Shape::Packet) {
            b.ins().store(MemFlags::new(), z, out, 32);
            b.ins().load(ty, MemFlags::new(), out, 32)
        } else {
            z
        };
        let shifted = b.ins().ushr_imm(z, 4);
        let sum = b.ins().iadd(z, shifted);
        let mask = if matches!(shape, Shape::Mask) {
            0x0f0f_0f0f_0f0f_0f0e
        } else {
            0x0f0f_0f0f_0f0f_0f0f
        };
        let c = b.ins().band_imm(sum, mask);
        let multiplier = if matches!(shape, Shape::Multiply) {
            0x0101_0101_0101_0103
        } else {
            0x0101_0101_0101_0101
        };
        let multiply = b.ins().imul_imm(c, multiplier);
        b.set_srcloc(SourceLoc::new(42));
        let root = b.ins().ushr_imm(
            multiply,
            if matches!(shape, Shape::Shift) {
                55
            } else {
                56
            },
        );
        root_inst = match b.func.dfg.value_def(root) {
            ValueDef::Result(inst, _) => inst,
            _ => unreachable!(),
        };
        for (i, v) in [a, z, c].into_iter().enumerate() {
            let v = if bits == 64 {
                v
            } else {
                b.ins().uextend(types::I64, v)
            };
            b.ins().store(MemFlags::new(), v, out, (i * 8) as i32);
        }
        let result = if bits == 64 {
            root
        } else {
            b.ins().uextend(types::I64, root)
        };
        b.ins().return_(&[result]);
        b.switch_to_block(error);
        let stamp = b.ins().iconst(types::I64, 0xbad);
        b.ins().store(MemFlags::new(), stamp, out, 24);
        let status = b.ins().iconst(types::I64, 0xdead);
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        b.finalize();
    }
    let before_loc = cx.func.srcloc(root_inst);
    let count = if selected { optimize(&mut cx.func) } else { 0 };
    assert_eq!(
        count,
        usize::from(selected && bits == 64 && matches!(shape, Shape::Exact)),
        "{bits} {shape:?}"
    );
    assert_eq!(cx.func.srcloc(root_inst), before_loc);
    cx.verify(module.isa()).unwrap();
    module.define_function(id, &mut cx).unwrap();
    id
}
fn oracle(value: u64, bits: u32, shape: Shape) -> [u64; 4] {
    let mask = u64::MAX >> (64 - bits);
    let x = value & mask;
    let shr = |x: u64, n: u32| x >> (n % bits);
    let a = x.wrapping_sub(shr(x, 1) & (0x5555_5555_5555_5555 & mask)) & mask;
    let b = ((a & (0x3333_3333_3333_3333 & mask))
        .wrapping_add(shr(a, 2) & (0x3333_3333_3333_3333 & mask)))
        & mask;
    let c = b.wrapping_add(shr(b, 4))
        & (if matches!(shape, Shape::Mask) {
            0x0f0f_0f0f_0f0f_0f0e
        } else {
            0x0f0f_0f0f_0f0f_0f0f
        } & mask);
    let multiplier = if matches!(shape, Shape::Multiply) {
        0x0101_0101_0101_0103
    } else {
        0x0101_0101_0101_0101
    };
    let root = shr(
        c.wrapping_mul(multiplier) & mask,
        if matches!(shape, Shape::Shift) {
            55
        } else {
            56
        },
    );
    [root, a, b, c]
}
#[test]
fn bit_count_native_exact_neighbors_widths_live_values_and_cold_errors() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut cases = Vec::new();
    for bits in [8, 16, 32, 64] {
        for shape in [
            Shape::Exact,
            Shape::Mask,
            Shape::Shift,
            Shape::Multiply,
            Shape::Packet,
        ] {
            for selected in [false, true] {
                let serial = cases.len();
                cases.push((
                    build(&mut module, bits, shape, selected, serial),
                    bits,
                    shape,
                ));
            }
        }
    }
    module.finalize_definitions().unwrap();
    let mut values = vec![
        0,
        1,
        u64::MAX,
        1 << 63,
        (1 << 63) - 1,
        0x5555_5555_5555_5555,
        0xaaaa_aaaa_aaaa_aaaa,
    ];
    for bit in 0..64 {
        values.extend([1 << bit, !(1 << bit)]);
    }
    let mut seed = 0x4521_8597_abcd_ef01u64;
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        values.push(seed);
    }
    for (id, bits, shape) in cases {
        let call: extern "C" fn(u64, *mut u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for &value in &values {
            let expected = oracle(value, bits, shape);
            let mut out = [0xcafe; 5];
            assert_eq!(
                call(value, out.as_mut_ptr(), 1),
                expected[0],
                "{bits} {shape:?} {value:#x}"
            );
            assert_eq!(&out[..3], &expected[1..]);
            assert_eq!(out[3], 0xcafe);
            if bits == 64 && matches!(shape, Shape::Exact) {
                assert_eq!(expected[0], value.count_ones() as u64);
            }
        }
        let mut out = [0xcafe; 5];
        assert_eq!(call(u64::MAX, out.as_mut_ptr(), 0), 0xdead);
        assert_eq!(&out[..4], &[0xcafe, 0xcafe, 0xcafe, 0xbad]);
    }
}

#[test]
fn compact_bit_count_source_original_decoded_object_errors_and_retry() {
    use crate::native_artifact::{
        compact_runtime::NativeStatus, program::prepare::PreparedNativeProgram,
    };
    use cranelift_module::FuncOrDataId;
    use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    fn artifacts(text: &str) -> [ProgramArtifact; 2] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(915), Revision(1), text).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        [artifact, decoded]
    }
    for indexed in [false, true] {
        for neighboring in [false, true] {
            let input = if indexed {
                "data := values<[u64]>\nat := index<u64>\nvalue := data[at]"
            } else {
                "value := word<u64>\ndelta := delta<u64>"
            };
            let checked = if indexed { "value" } else { "value + delta" };
            let shift = if neighboring { 55 } else { 56 };
            let text = format!(
                "+> bits\n{input}\na := bits/wrapping-sub(value, bits/and(bits/shift-right-wrap(value, 1u64), 6148914691236517205u64))\nb := bits/wrapping-add(bits/and(a, 3689348814741910323u64), bits/and(bits/shift-right-wrap(a, 2u64), 3689348814741910323u64))\nc := bits/and(bits/wrapping-add(b, bits/shift-right-wrap(b, 4u64)), 1085102592571150095u64)\ncount := bits/shift-right-wrap(bits/wrapping-mul(c, 72340172838076673u64), {shift}u64)\nchecked := {checked}\n(count,a,b,c,checked)\n"
            );
            let mut expected_failure = None;
            for artifact in artifacts(&text) {
                let catalog = mech_stdlib::source_catalog();
                let plan = mech_engine::native_artifact::lower_native_program_artifact(
                    &artifact, &catalog,
                )
                .unwrap();
                let prepared = PreparedNativeProgram::prepare(&artifact, &catalog, plan).unwrap();
                let builder = JITBuilder::with_flags(
                    &[
                        ("opt_level", "speed"),
                        ("is_pic", "true"),
                        ("preserve_frame_pointers", "true"),
                        ("unwind_info", "true"),
                        (
                            "log2_min_function_alignment",
                            if cfg!(target_arch = "aarch64") {
                                "7"
                            } else {
                                "0"
                            },
                        ),
                    ],
                    default_libcall_names(),
                )
                .unwrap();
                let mut module = JITModule::new(builder);
                let metadata =
                    crate::native_artifact::program::compact::define(&mut module, &prepared, false)
                        .unwrap();
                assert_eq!(
                    metadata["bit_counts"].as_u64(),
                    Some(u64::from(!neighboring))
                );
                assert_eq!(metadata["concrete_heap_context"], false);
                assert_eq!(metadata["output_words"], 5);
                let (_, object_metadata) =
                    crate::native_artifact::emit_compact_closed_object_for_diagnostic(
                        &artifact, &catalog,
                    )
                    .unwrap();
                assert_eq!(object_metadata["bit_counts"], metadata["bit_counts"]);
                let Some(FuncOrDataId::Func(id)) =
                    module.get_name(metadata["symbol"].as_str().unwrap())
                else {
                    panic!("compact native entry")
                };
                module.finalize_definitions().unwrap();
                let call: unsafe extern "C" fn(*const u64, *mut u64) -> NativeStatus =
                    unsafe { core::mem::transmute(module.get_finalized_function(id)) };
                let values = [
                    0,
                    1,
                    u64::MAX,
                    1 << 63,
                    (1 << 63) - 1,
                    0x5555_5555_5555_5555,
                    0xaaaa_aaaa_aaaa_aaaa,
                ];
                for &value in &values {
                    let data = [value];
                    let mut inputs = vec![0u64; metadata["input_words"].as_u64().unwrap() as usize];
                    for port in metadata["inputs"].as_array().unwrap() {
                        let at = port["input_offset"].as_u64().unwrap() as usize;
                        let name =
                            mech_engine::decode_source_input_name(port["name"].as_str().unwrap())
                                .unwrap();
                        match name.as_str() {
                            "word" => inputs[at] = value,
                            "delta" => inputs[at] = 0,
                            "index" => inputs[at] = 1,
                            "values" => inputs[at..at + 6].copy_from_slice(&[
                                data.as_ptr() as u64,
                                1,
                                1,
                                1,
                                1,
                                0,
                            ]),
                            _ => panic!("unexpected port {name}"),
                        }
                    }
                    let mut output = [0xcafe; 5];
                    let status = unsafe { call(inputs.as_ptr(), output.as_mut_ptr()) };
                    assert_eq!(
                        status.status, 0,
                        "{indexed} {neighboring} {value:#x}: status={} origin={}",
                        status.status, status.origin
                    );
                    let shape = if neighboring {
                        Shape::Shift
                    } else {
                        Shape::Exact
                    };
                    let wanted = oracle(value, 64, shape);
                    assert_eq!(output, [wanted[0], wanted[1], wanted[2], wanted[3], value]);
                }
                let data = [u64::MAX];
                let mut inputs = vec![0u64; metadata["input_words"].as_u64().unwrap() as usize];
                let mut retry_input = None;
                for port in metadata["inputs"].as_array().unwrap() {
                    let at = port["input_offset"].as_u64().unwrap() as usize;
                    let name =
                        mech_engine::decode_source_input_name(port["name"].as_str().unwrap())
                            .unwrap();
                    match name.as_str() {
                        "word" => inputs[at] = u64::MAX,
                        "delta" => {
                            inputs[at] = 1;
                            retry_input = Some((at, 0));
                        }
                        "index" => {
                            inputs[at] = 2;
                            retry_input = Some((at, 1));
                        }
                        "values" => inputs[at..at + 6].copy_from_slice(&[
                            data.as_ptr() as u64,
                            1,
                            1,
                            1,
                            1,
                            0,
                        ]),
                        _ => unreachable!(),
                    }
                }
                let mut output = [0xcafe; 5];
                let failed = unsafe { call(inputs.as_ptr(), output.as_mut_ptr()) };
                assert_ne!(failed.status, 0);
                assert!(failed.origin < metadata["source_blocks"].as_u64().unwrap());
                assert_eq!(
                    (failed.status, failed.origin),
                    *expected_failure.get_or_insert((failed.status, failed.origin))
                );
                assert_eq!(output, [0xcafe; 5]);
                let (at, value) = retry_input.unwrap();
                inputs[at] = value;
                let status = unsafe { call(inputs.as_ptr(), output.as_mut_ptr()) };
                assert_eq!(status.status, 0);
                let wanted = oracle(
                    u64::MAX,
                    64,
                    if neighboring {
                        Shape::Shift
                    } else {
                        Shape::Exact
                    },
                );
                assert_eq!(
                    output,
                    [wanted[0], wanted[1], wanted[2], wanted[3], u64::MAX]
                );
                assert_eq!(data, [u64::MAX]);
            }
        }
    }
}
