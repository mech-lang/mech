//! Execute direct native composition against independent scalar references.
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use cranelift_codegen::ir::{AbiParam, Function, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const OK: u64 = 0x600d;
const BAD: u64 = 0xbad0_bad0_bad0_bad0;
type Run = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64, u64) -> u64;

fn compile(kind: &str, reduction: bool) -> (cranelift_jit::JITModule, Run) {
    let expression = "[data[i + shift] | i <- first..=last]";
    let expression = if reduction {
        format!("stats/sum/column({expression})[1u64]")
    } else {
        expression.to_owned()
    };
    let source = format!(
        "+> stats\ndata := input<[{kind}]:2,3>\nfirst := begin<u64>\nlast := end<u64>\nshift := offset<u64>\n{expression}\n"
    );
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5353414d), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{source}");
    let catalog = mech_stdlib::source_catalog();
    let artifact = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let jit = JitProgramArtifact::compile_with_limits(
        &artifact,
        &catalog,
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let metadata = jit.descriptor().metadata();
    let [region] = &metadata.numeric_maps.regions[..] else {
        panic!("one actual source map")
    };
    assert!(region.range);
    assert_eq!(region.reduction.is_some(), reduction);
    assert_eq!(region.captures.len(), 1);
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 9];
    function.signature.returns = vec![AbiParam::new(types::I64)];
    let mut context = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut function, &mut context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let miss = b.create_block();
        let captures = BTreeMap::from([(region.captures[0], args[8])]);
        let output = (!reduction).then_some((args[6], args[7]));
        let value = emit_region(
            &mut b,
            region,
            &metadata.plan,
            args[4],
            args[5],
            &[[args[0], args[1], args[2], args[3]]],
            &captures,
            output,
            miss,
        )
        .unwrap();
        let result = if reduction {
            value
        } else {
            b.ins().iconst(types::I64, OK as i64)
        };
        b.ins().return_(&[result]);
        b.switch_to_block(miss);
        let sentinel = b.ins().iconst(types::I64, BAD as i64);
        b.ins().return_(&[sentinel]);
        b.seal_all_blocks();
        b.finalize();
    }
    assert!(
        function.sized_stack_slots.is_empty(),
        "native composition needs no adapter frame"
    );
    assert!(
        !function
            .layout
            .blocks()
            .flat_map(|block| function.layout.block_insts(block))
            .any(|inst| matches!(
                function.dfg.insts[inst].opcode(),
                Opcode::Call | Opcode::CallIndirect
            ))
    );
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func = function;
    context.func.signature.call_conv = module.make_signature().call_conv;
    let id = module
        .declare_function(
            "map_ssa_composition",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the retained generated function has exactly nine word parameters
    // and one word result; each test below keeps its tiny backing banks alive.
    let run = unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(id)) };
    (module, run)
}

#[test]
fn numeric_map_ssa_keeps_typed_bits_source_axes_capacity_and_checked_failures() {
    for (kind, width) in [
        ("bool", 1usize),
        ("u8", 1),
        ("u16", 2),
        ("u32", 4),
        ("u64", 8),
        ("i64", 8),
        ("f64", 8),
    ] {
        let (_module, run) = compile(kind, false);
        let values: [u64; 6] = if kind == "bool" {
            [0, 1, 1, 0, 0, 1]
        } else {
            [
                0x8000_0000_0000_0000,
                0x7ff8_0000_0000_0037,
                0x7ff0_0000_0000_0001,
                0x7ff0_0000_0000_0000,
                0xfff0_0000_0000_0000,
                u64::MAX,
            ]
        };
        let bytes = values
            .iter()
            .flat_map(|word| word.to_ne_bytes().into_iter().take(width))
            .collect::<Vec<_>>();
        for (first, count, shift) in [
            (1u64, 6u64, 0u64),
            (2, 4, 0),
            (1, 5, 1),
            (0, 1, 1),
            (6, 1, 0),
            (1, 0, 0),
        ] {
            let mut output = vec![0xa5u8; 6 * width + 2];
            // SAFETY: all generated indices are inside six live cells, and the
            // unaligned destination contains the exact checked output span.
            let status = unsafe {
                run(
                    bytes.as_ptr() as u64,
                    6,
                    2,
                    3,
                    first,
                    count,
                    output.as_mut_ptr().add(1) as u64,
                    count * width as u64,
                    shift,
                )
            };
            assert_eq!(status, OK, "{kind} {first} {count} {shift}");
            let expected = (0..count)
                .flat_map(|at| {
                    let ordinal = first + at + shift - 1;
                    let physical = (ordinal % 2) * 3 + ordinal / 2;
                    values[physical as usize]
                        .to_ne_bytes()
                        .into_iter()
                        .take(width)
                })
                .collect::<Vec<_>>();
            assert_eq!(&output[1..1 + expected.len()], expected, "{kind}");
            assert_eq!(output[0], 0xa5);
            assert!(output[1 + expected.len()..].iter().all(|&v| v == 0xa5));
            assert_eq!(
                bytes,
                values
                    .iter()
                    .flat_map(|word| word.to_ne_bytes().into_iter().take(width))
                    .collect::<Vec<_>>()
            );
        }
        for (cells, rows, columns, first, count, shift, capacity) in [
            (6u64, 2u64, 3u64, 0u64, 1u64, 0u64, width as u64),
            (6, 2, 3, 7, 1, 0, width as u64),
            (6, 2, 3, 1, 1, u64::MAX, width as u64),
            (6, 2, 3, u64::MAX, 2, 0, (2 * width) as u64),
            (6, 2, 3, 1, u64::MAX, 0, 0),
            (6, 2, 3, 1, 1, 0, 0),
            (6, 2, 3, 1, 1, 0, width as u64 + 1),
            (5, 2, 3, 1, 1, 0, width as u64),
            (6, 3, 2, 1, 1, 0, width as u64),
            (0, 0, 3, 1, 1, 0, width as u64),
            (u64::MAX, u64::MAX, u64::MAX, 1, 1, 0, width as u64),
        ] {
            let mut output = [0xa5u8; 64];
            // SAFETY: every malformed range/view/capacity or scalar overflow
            // must decline before the first source read or destination store.
            let status = unsafe {
                run(
                    bytes.as_ptr() as u64,
                    cells,
                    rows,
                    columns,
                    first,
                    count,
                    output.as_mut_ptr() as u64,
                    capacity,
                    shift,
                )
            };
            assert_eq!(
                status, BAD,
                "{kind}: ({cells},{rows},{columns}) {first} {count} {shift} {capacity}"
            );
            assert_eq!(output, [0xa5; 64]);
        }
        if kind == "bool" {
            let input = [2u8, 0, 0, 0, 0, 0];
            let mut output = [0xa5u8; 1];
            // SAFETY: live banks have their complete admitted geometry; the
            // noncanonical Bool must decline before it can be stored.
            assert_eq!(
                unsafe {
                    run(
                        input.as_ptr() as u64,
                        6,
                        2,
                        3,
                        1,
                        1,
                        output.as_mut_ptr() as u64,
                        1,
                        0,
                    )
                },
                BAD
            );
            assert_eq!(output, [0xa5]);
        }
    }
}

#[test]
fn numeric_map_ssa_returns_ordered_fold_and_keeps_overflow_and_shape_misses() {
    let (_module, run) = compile("f64", true);
    for values in [
        [1e16, -1e16, 1.0, 2.0, -2.0, -0.0],
        [-0.0; 6],
        [f64::INFINITY, f64::NEG_INFINITY, 0.0, 1.0, 2.0, 3.0],
    ] {
        for (first, count, shift) in [
            (1u64, 6u64, 0u64),
            (2, 4, 0),
            (1, 5, 1),
            (6, 1, 0),
            (1, 0, 0),
        ] {
            let mut expected = 0.0f64;
            for at in 0..count {
                let ordinal = first + at + shift - 1;
                expected += values[((ordinal % 2) * 3 + ordinal / 2) as usize];
            }
            // SAFETY: the source is a live two-by-three F64 matrix and every
            // selected index is in range. A fold needs no destination bank.
            let actual = f64::from_bits(unsafe {
                run(values.as_ptr() as u64, 6, 2, 3, first, count, 0, 0, shift)
            });
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
        // SAFETY: each invalid case must decline before dereferencing a bad
        // index; input backing remains valid for its full claimed view.
        assert_eq!(
            unsafe { run(values.as_ptr() as u64, 6, 2, 3, 1, 1, 0, 0, u64::MAX) },
            BAD
        );
        assert_eq!(
            unsafe { run(values.as_ptr() as u64, 6, 3, 2, 1, 6, 0, 0, 0) },
            BAD
        );
        assert_eq!(
            unsafe { run(values.as_ptr() as u64, 6, 2, 3, 7, 1, 0, 0, 0) },
            BAD
        );
    }
}
