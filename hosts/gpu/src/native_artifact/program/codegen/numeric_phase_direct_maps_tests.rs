//! Actual shared map emission, direct path selection and overlapping old reads.
use super::*;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

type Entry = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64) -> u64;
fn source(text: &str) -> JitProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x444d4150), Revision(1), text).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
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
fn compile(jit: &JitProgramArtifact, enabled: bool) -> (cranelift_jit::JITModule, Entry) {
    let m = jit.descriptor().metadata();
    let maps =
        crate::native_artifact::program::prepare::numeric_phase::matrix_maps::inventory(&m.plan)
            .unwrap();
    let source = maps
        .iter()
        .find(|map| map.extents == [2, 3])
        .expect("actual checked rectangular map");
    assert_eq!(source.region.matrices.len(), 1);
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 6];
    context
        .func
        .signature
        .returns
        .push(AbiParam::new(types::I64));
    let mut frontend = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
        let entry = b.create_block();
        let miss = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [base, input, marker, rows, columns, first] = b.block_params(entry).try_into().unwrap();
        let cells = b.ins().imul(rows, columns);
        let views = vec![[input, cells, rows, columns]];
        let mut captures = BTreeMap::new();
        for &id in &source.region.captures {
            let value = if let Some(c) = m.plan.constants().iter().find(|c| c.value().get() == id) {
                let bits = match c.constant().data() {
                    mech_core::ValueData::U64(n) => *n,
                    mech_core::ValueData::F64(n) => {
                        mech_engine::native_artifact::NativeScalarValue::F64(*n).to_bits()
                    }
                    mech_core::ValueData::Bool(n) => u64::from(*n),
                    other => panic!("fixture literal {other:?}"),
                };
                b.ins().iconst(types::I64, bits as i64)
            } else {
                marker
            };
            captures.insert(id, value);
        }
        // A compiler-local malformed body may not gain direct authority even
        // though the genuine frontend forbids non-exhaustive source cases.
        if enabled {
            if let Some(&reject) = source.region.body.iter().find(|&&site| {
                matches!(
                    m.plan.blocks()[site as usize].terminator(),
                    mech_engine::native_artifact::NativeIterationTerminator::Scalar(
                        mech_engine::native_artifact::NativeMatchTerminator::Reject
                    )
                )
            }) {
                let mut incomplete = source.clone();
                incomplete.region.body = vec![reject];
                assert!(
                    direct::guard(&mut b, &incomplete, &m.plan, base, first, &views, &captures)
                        .is_none()
                );
            }
        }
        let count = b.ins().iconst(types::I64, source.count as i64);
        let direct = emit_map(
            &mut b, source, &m.plan, base, first, count, &views, &captures, enabled, miss,
        )
        .unwrap();
        if enabled && direct.is_none() {
            eprintln!("unselected map: {source:?}");
            for &site in &source.region.body {
                let block = &m.plan.blocks()[site as usize];
                eprintln!("{site}: {:?} / {:?}", block.action(), block.terminator());
            }
        }
        let selected = if let Some(direct) = direct {
            b.ins().uextend(types::I64, direct)
        } else {
            b.ins().iconst(types::I64, 0)
        };
        b.ins().return_(&[selected]);
        b.switch_to_block(miss);
        let failed = b.ins().iconst(types::I64, 2);
        b.ins().return_(&[failed]);
        b.seal_all_blocks();
        b.finalize();
    }
    crate::native_artifact::program::codegen::cfg_cleanup::optimize(
        &CompilationControls::new(OptimizationOptions::default()),
        &mut context,
        module.isa(),
    )
    .unwrap();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "direct_complete_map",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: this retained function takes six word parameters and returns a
    // selection/error status. The tests lend complete, live private banks.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry)
}
fn bytes(values: &[u64], width: usize) -> Vec<u8> {
    values
        .iter()
        .flat_map(|v| v.to_ne_bytes().into_iter().take(width))
        .collect()
}

#[test]
fn numeric_phase_direct_maps_keep_all_scalar_bits_same_cells_and_overlapping_aliases() {
    for (kind, width) in [
        ("bool", 1usize),
        ("u8", 1),
        ("u16", 2),
        ("u32", 4),
        ("u64", 8),
        ("i64", 8),
        ("f64", 8),
    ] {
        let jit = source(&format!(
            "data := input<[{kind}]:2,3>\n[[((i == 1u64)? | true => marker<{kind}> | false => data[i,j]) | i <- 1u64..=2u64]' | j <- 1u64..=3u64]\n"
        ));
        let (_off, old) = compile(&jit, false);
        let (_on, new) = compile(&jit, true);
        let values = if kind == "bool" {
            vec![0, 1, 0, 1, 1, 0, 1]
        } else {
            vec![
                0,
                1,
                u64::MAX,
                1 << 63,
                0x7ff0_0000_0000_0037,
                0x7ff8_0000_0000_0081,
                0xd39a_4c17_805e_12bf,
            ]
        };
        let mask = u64::MAX >> ((8 - width) * 8);
        let marker = if kind == "bool" {
            1
        } else {
            0x7ff0_0000_0000_0091 & mask
        };
        let original = bytes(&values, width);
        let independent = bytes(&values[..6], width);
        for alias in 0..3 {
            let mut a = original.clone();
            let mut z = original.clone();
            let pa = match alias {
                0 => a.as_ptr(),
                1 => unsafe { a.as_ptr().add(width) },
                _ => independent.as_ptr(),
            };
            let pz = match alias {
                0 => z.as_ptr(),
                1 => unsafe { z.as_ptr().add(width) },
                _ => independent.as_ptr(),
            };
            // SAFETY: same-cell/shifted views read at most six initialized cells
            // from a seven-cell allocation. Both destinations own six cells;
            // the shifted source must complete in scratch before any copy.
            let x = unsafe { old(a.as_mut_ptr() as u64, pa as u64, marker, 2, 3, 1) };
            let y = unsafe { new(z.as_mut_ptr() as u64, pz as u64, marker, 2, 3, 1) };
            assert_eq!(x, 0);
            assert_eq!(
                y,
                u64::from(alias != 1),
                "actual direct selection {kind}, alias={alias}"
            );
            assert_eq!(a, z, "exact bits {kind}, alias={alias}");
            assert_eq!(&z[6 * width..], &original[6 * width..]);
            let mut expected = original.clone();
            let marker_bytes = marker.to_ne_bytes();
            for cell in 0..3 {
                expected[cell * width..(cell + 1) * width].copy_from_slice(&marker_bytes[..width]);
            }
            let source = if alias == 1 {
                &original[width..]
            } else {
                &independent[..]
            };
            expected[3 * width..6 * width].copy_from_slice(&source[3 * width..6 * width]);
            assert_eq!(z, expected);
        }
        for (rows, columns, first) in [(3, 2, 1), (2, 3, 0)] {
            let mut a = original.clone();
            let mut z = original.clone();
            // SAFETY: malformed geometry or the first used zero source column
            // must miss before destination publication; source banks are live.
            let x = unsafe {
                old(
                    a.as_mut_ptr() as u64,
                    a.as_ptr() as u64,
                    marker,
                    rows,
                    columns,
                    first,
                )
            };
            let y = unsafe {
                new(
                    z.as_mut_ptr() as u64,
                    z.as_ptr() as u64,
                    marker,
                    rows,
                    columns,
                    first,
                )
            };
            assert_eq!((x, y), (2, 2), "{kind}");
            assert_eq!(a, original);
            assert_eq!(z, original, "no partial direct result");
        }
    }
}

#[test]
fn numeric_phase_direct_maps_preserve_reverse_reads_and_late_arithmetic_failure() {
    let partial = "data := input<[f64]:2,3>\n[[((i == 1u64)? | true => data[i,j]) | i <- 1u64..=2u64]' | j <- 1u64..=3u64]\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x444d4151), Revision(1), partial).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let result = mech_engine::CanonicalSourceFrontend.compile_document_with_catalog(
        &DocumentSyntax::cast(parsed.syntax()).unwrap(),
        mech_stdlib::source_catalog(),
    );
    let diagnostic = match result {
        Err(error) => format!("{error:?}"),
        Ok(_) => panic!("non-exhaustive source must reject before native proof"),
    };
    assert!(
        diagnostic.contains("source-semantics/non-exhaustive-match"),
        "{diagnostic}"
    );

    for (expression, fails) in [
        ("data[3u64 - i,j]", false),
        (
            "data[1u64 / ((i == 2u64)? | true => 0u64 | false => 1u64),j]",
            true,
        ),
    ] {
        let jit = source(&format!(
            "data := input<[f64]:2,3>\n[[{expression} | i <- 1u64..=2u64]' | j <- 1u64..=3u64]\n"
        ));
        let (_off, old) = compile(&jit, false);
        let (_on, new) = compile(&jit, true);
        let original = bytes(
            &[
                0,
                1 << 63,
                0x7ff0_0000_0000_0091,
                0x7ff8_0000_0000_0037,
                u64::MAX,
                0x3ff0_0000_0000_0000,
                0xfeed_face_dead_beef,
            ],
            8,
        );
        let mut a = original.clone();
        let mut z = original.clone();
        // SAFETY: the real shared emitter checks every source selector. Reverse
        // and potentially failing bodies must retain the old complete scratch
        // result and commit only after all source cells succeed.
        let x = unsafe { old(a.as_mut_ptr() as u64, a.as_ptr() as u64, 0, 2, 3, 1) };
        let y = unsafe { new(z.as_mut_ptr() as u64, z.as_ptr() as u64, 0, 2, 3, 1) };
        assert_eq!(x, if fails { 2 } else { 0 });
        assert_eq!(x, y);
        assert_eq!(a, z);
        if fails {
            assert_eq!(z, original, "late failure preserves old destination");
        } else {
            assert_eq!(&z[..24], &original[24..48]);
            assert_eq!(&z[24..48], &original[..24]);
        }
        assert_eq!(&z[48..], &original[48..]);
    }
}
