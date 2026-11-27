//! Execute the actual shared fold emitter against its former word-home form.
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

type Entry = unsafe extern "C" fn(*mut Frame) -> u64;
fn source(text: &str) -> JitProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x46464f4c), Revision(1), text).unwrap(),
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
fn compile<const FLOATING: bool>(jit: &JitProgramArtifact) -> (cranelift_jit::JITModule, Entry) {
    let m = jit.descriptor().metadata();
    let region = m
        .numeric_maps
        .regions
        .iter()
        .find(|region| region.reduction.is_some())
        .expect("actual admitted F64 fold");
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context
        .func
        .signature
        .params
        .push(AbiParam::new(types::I64));
    context
        .func
        .signature
        .returns
        .push(AbiParam::new(types::I64));
    let mut frontend = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
        let entry = b.create_block();
        let complete = b.create_block();
        let miss = b.create_block();
        let malformed = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(miss, types::I64);
        b.switch_to_block(entry);
        let frame = b.block_params(entry)[0];
        let mut captures = BTreeMap::new();
        for &id in &region.captures {
            let constant = m
                .plan
                .constants()
                .iter()
                .find(|constant| constant.value().get() == id)
                .expect("fixture capture is a literal");
            let bits = match constant.constant().data() {
                mech_core::ValueData::F64(value) => {
                    mech_engine::native_artifact::NativeScalarValue::F64(*value).to_bits()
                }
                mech_core::ValueData::U64(value) => *value,
                mech_core::ValueData::Bool(value) => u64::from(*value),
                other => panic!("unexpected literal {other:?}"),
            };
            captures.insert(id, b.ins().iconst(types::I64, bits as i64));
        }
        emit_with_fold_home::<FLOATING>(
            &mut b, frame, region, &m.plan, &captures, complete, miss, malformed,
        )
        .unwrap();
        for (block, status) in [(complete, 0), (miss, 1), (malformed, 2)] {
            b.switch_to_block(block);
            let status = b.ins().iconst(types::I64, status);
            b.ins().return_(&[status]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    let floating_joins = context
        .func
        .layout
        .blocks()
        .flat_map(|block| context.func.dfg.block_params(block))
        .filter(|&&value| context.func.dfg.value_type(value) == types::F64)
        .count();
    assert_eq!(
        floating_joins > 0,
        FLOATING,
        "actual floating fold loop parameters"
    );
    crate::native_artifact::program::codegen::cfg_cleanup::optimize(
        &CompilationControls::new(OptimizationOptions::default()),
        &mut context,
        module.isa(),
    )
    .unwrap();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "numeric_f64_fold_home",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the owned native function takes exactly one live mutable Frame.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry)
}
fn frame(input: &[u64]) -> Frame {
    Frame {
        input: input.as_ptr() as u64,
        input_bytes: (input.len() * 8) as u64,
        count: input.len() as u64,
        source_count: input.len() as u64,
        input_width: 8,
        output_width: 8,
        epoch: 7,
        issued: 7,
        ..Default::default()
    }
}

#[test]
fn numeric_map_f64_fold_homes_keep_ordered_bits_empty_odd_counts_and_branch_joins() {
    let mut cases = vec![
        vec![],
        vec![0],
        vec![1 << 63],
        vec![1 << 63, 1 << 63],
        vec![1],
        vec![0x7ff0_0000_0000_0037],
        vec![0x7ff8_0000_0000_0081, 0xfff0_0000_0000_0049],
        vec![f64::INFINITY.to_bits(), f64::NEG_INFINITY.to_bits()],
        vec![1e16f64.to_bits(), 1f64.to_bits(), (-1e16f64).to_bits()],
        vec![1, 0x8000_0000_0000_0001, 0],
    ];
    let mut state = 0x1234_5678_9abc_def0u64;
    let mut random = Vec::new();
    for _ in 0..65 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        random.push(state);
    }
    for n in [1, 2, 3, 17, 64, 65] {
        cases.push(random[..n].to_vec());
    }
    for expression in [
        "value",
        "((value < 0.0)? | true => value | false => value * 1.0)",
    ] {
        let jit = source(&format!(
            "+> stats\nstats/sum/column([{expression} | value <- input<[f64]>])[1u64]\n"
        ));
        let (_old_module, old) = compile::<false>(&jit);
        let (_new_module, new) = compile::<true>(&jit);
        for cells in &cases {
            let before = cells.clone();
            let initial = frame(cells);
            let mut word = initial;
            let mut floating = initial;
            // SAFETY: both frames describe the same live, fully initialized input;
            // no output bank is needed by the genuine admitted reduction.
            assert_eq!(unsafe { old(&mut word) }, 0);
            assert_eq!(unsafe { new(&mut floating) }, 0);
            assert_eq!(
                floating, word,
                "all bits and frame fields: {expression}, {cells:x?}"
            );
            assert_eq!(floating.next, cells.len() as u64);
            if cells.is_empty() {
                assert_eq!(floating.fold, 0, "positive-zero seed");
            }
            if cells == &vec![1e16f64.to_bits(), 1f64.to_bits(), (-1e16f64).to_bits()] {
                assert_eq!(floating.fold, 0, "no reassociation");
            }
            assert_eq!(*cells, before);
        }
    }
}

#[test]
fn numeric_map_f64_fold_homes_keep_malformed_guards_and_uncommitted_late_misses() {
    let jit = source(
        "+> stats\ndata := input<[f64]>\nstats/sum/column([data[index] | index <- indices<[u64]>])[1u64]\n",
    );
    let (_old_module, old) = compile::<false>(&jit);
    let (_new_module, new) = compile::<true>(&jit);
    let data = [1.0f64.to_bits(), 2.0f64.to_bits(), 4.0f64.to_bits()];
    for indices in [
        vec![],
        vec![1],
        vec![1, 2, 3],
        vec![1, 2, 0, 3],
        vec![1, 2, 4],
    ] {
        let mut initial = frame(&indices);
        initial.matrices[0] = MatrixView {
            input: data.as_ptr() as u64,
            count: 3,
            rows: 1,
            columns: 3,
        };
        let mut word = initial;
        let mut floating = initial;
        // SAFETY: admitted read indices are checked before access to the live
        // three-cell source; zero/out-of-range cells branch to the miss block.
        let a = unsafe { old(&mut word) };
        let b = unsafe { new(&mut floating) };
        assert_eq!(a, b);
        assert_eq!(word, floating);
        if indices.iter().any(|&index| index == 0 || index > 3) {
            assert_eq!(a, 1);
            assert_eq!(floating, initial, "late miss publishes no partial fold");
        } else {
            assert_eq!(a, 0);
        }
    }
    let indices = [1u64, 2, 3];
    for bad in 0..10 {
        let mut initial = frame(&indices);
        initial.matrices[0] = MatrixView {
            input: data.as_ptr() as u64,
            count: 3,
            rows: 1,
            columns: 3,
        };
        match bad {
            0 => initial.issued = 8,
            1 => {
                initial.epoch = 0;
                initial.issued = 0;
            }
            2 => initial.next = 1,
            3 => initial.fold = 1 << 63,
            4 => initial.input_bytes -= 1,
            5 => initial.source_count += 1,
            6 => initial.matrices[0].columns = 2,
            7 => initial.input = u64::MAX - 1,
            8 => initial.matrices[0].input = u64::MAX - 1,
            9 => initial.output_count = 1,
            _ => unreachable!(),
        }
        let mut word = initial;
        let mut floating = initial;
        // SAFETY: each malformed field must reject before any access through
        // its pointer. Every otherwise admissible bank remains initialized.
        assert_eq!(unsafe { old(&mut word) }, 2, "bad={bad}");
        assert_eq!(unsafe { new(&mut floating) }, 2, "bad={bad}");
        assert_eq!(word, initial);
        assert_eq!(floating, initial, "all guard failures remain uncommitted");
    }
}
