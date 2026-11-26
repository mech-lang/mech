//! Actual map-read emission, fixed guards, machine divisions and axis order.
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use cranelift_codegen::ir::{AbiParam, Function, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const BAD: u64 = 0xbad0_bad0_bad0_bad0;
type Read = unsafe extern "C" fn(u64, u64, u64, u64) -> u64;
fn source(kind: &str, dimensions: &str) -> JitProgramArtifact {
    let source = format!("data := input<[{kind}]{dimensions}>\n[data[i] | i <- 1u64..=6u64]\n");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d415047), Revision(1), source.as_str()).unwrap(),
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
    JitProgramArtifact::compile_with_limits(
        &artifact,
        &catalog,
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap()
}
fn read_function(jit: &JitProgramArtifact) -> Function {
    let metadata = jit.descriptor().metadata();
    let [region] = &metadata.numeric_maps.regions[..] else {
        panic!("one actual map read")
    };
    let [read] = &region.reads[..] else {
        panic!("one actual matrix read")
    };
    assert_eq!(read.indices.len(), 1);
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 4];
    function.signature.returns = vec![AbiParam::new(types::I64)];
    let mut context = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut function, &mut context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let bad = b.create_block();
        // The complete production caller already checks overflow, backing and
        // pointer geometry. This arithmetic-only wrapper supplies tiny bounds.
        let count = b.ins().imul(args[1], args[2]);
        let (count, rows, columns) = guard_fixed_geometry(
            &mut b,
            metadata.plan.values()[region.matrices[0].value as usize]
                .definition()
                .body(),
            count,
            args[1],
            args[2],
            bad,
        );
        let index = b.declare_var(types::I64);
        b.def_var(index, args[0]);
        let words = BTreeMap::from([(read.indices[0], index)]);
        let value = emit_read(
            &mut b,
            read,
            region,
            &metadata.plan,
            &[(args[3], count, rows, columns)],
            &words,
            bad,
        )
        .unwrap();
        b.ins().return_(&[value]);
        b.switch_to_block(bad);
        let sentinel = b.ins().iconst(types::I64, BAD as i64);
        b.ins().return_(&[sentinel]);
        b.seal_all_blocks();
        b.finalize();
    }
    function
}
fn compile(jit: &JitProgramArtifact) -> (cranelift_jit::JITModule, Read, String) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func = read_function(jit);
    context.func.signature.call_conv = module.make_signature().call_conv;
    context.set_disasm(true);
    let id = module
        .declare_function("map_read_geometry", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let assembly = context.compiled_code().unwrap().vcode.clone().unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the retained function has four word inputs and a word result.
    let read =
        unsafe { core::mem::transmute::<*const u8, Read>(module.get_finalized_function(id)) };
    (module, read, assembly)
}
#[test]
fn numeric_map_read_geometry_uses_shared_vector_lowering_and_one_general_division() {
    for kind in ["bool", "u8", "u16", "u32", "u64", "i64", "f64"] {
        for (dimensions, vector) in [(":1,6", true), (":6,1", true), ("", false), (":2,3", false)] {
            let jit = source(kind, dimensions);
            let function = read_function(&jit);
            let divisions = function
                .layout
                .blocks()
                .flat_map(|b| function.layout.block_insts(b))
                .filter(|&i| matches!(function.dfg.insts[i].opcode(), Opcode::Udiv | Opcode::Urem))
                .count();
            assert_eq!(
                divisions,
                usize::from(!vector),
                "{kind} {dimensions}: {}",
                function.display()
            );
            let (_module, _, assembly) = compile(&jit);
            if cfg!(target_arch = "aarch64") {
                let machine = assembly.split(";; REGALLOC_INPUT_BEGIN").next().unwrap();
                let divisions = machine.lines().filter(|s| s.contains("udiv ")).count();
                // A fixed2-row matrix uses a shift; generic matrices need one
                // checked divide, and vectors need no divide at all.
                assert_eq!(
                    divisions,
                    usize::from(dimensions.is_empty()),
                    "{kind} {dimensions}\n{machine}"
                );
            }
        }
    }
}
#[test]
fn numeric_map_read_geometry_keeps_exact_bits_bounds_axes_and_refuses_mismatched_fixed_views() {
    for (kind, width) in [
        ("bool", 1),
        ("u8", 1),
        ("u16", 2),
        ("u32", 4),
        ("u64", 8),
        ("i64", 8),
        ("f64", 8),
    ] {
        let values = if kind == "bool" {
            vec![1, 0, 1, 1, 0, 0]
        } else {
            vec![
                0,
                1,
                0x8000_0000_0000_0000,
                0x7ff8_0000_0000_1234,
                0x7ff0_0000_0000_0000,
                u64::MAX,
            ]
        };
        let mask = if width == 8 {
            u64::MAX
        } else {
            (1u64 << (width * 8)) - 1
        };
        let bytes = values
            .iter()
            .flat_map(|v| u64::to_ne_bytes(*v).into_iter().take(width))
            .collect::<Vec<_>>();
        for dimensions in ["", ":1,6", ":6,1", ":2,3", ":0,6"] {
            let jit = source(kind, dimensions);
            let (_module, read, _) = compile(&jit);
            for (rows, columns) in [(1u64, 6u64), (6, 1), (2, 3), (3, 2), (0, 6), (6, 0)] {
                let fixed = fixed_geometry(
                    jit.descriptor().plan().values()[jit
                        .descriptor()
                        .metadata()
                        .numeric_maps
                        .regions[0]
                        .matrices[0]
                        .value as usize]
                        .definition()
                        .body(),
                );
                for index in 0..=7u64 {
                    let valid = index > 0
                        && index <= rows * columns
                        && fixed.is_none_or(|[_, r, c]| (r, c) == (rows, columns));
                    let expected = if valid {
                        let ordinal = index - 1;
                        let physical = (ordinal % rows) * columns + ordinal / rows;
                        values[physical as usize] & mask
                    } else {
                        BAD
                    };
                    // SAFETY: all accepted views contain exactly six packed
                    // cells. Invalid/empty indexes must branch before a load.
                    let actual = unsafe { read(index, rows, columns, bytes.as_ptr() as u64) };
                    assert_eq!(
                        actual, expected,
                        "{kind} {dimensions} ({rows},{columns}) index={index}"
                    );
                }
            }
        }
    }
}
