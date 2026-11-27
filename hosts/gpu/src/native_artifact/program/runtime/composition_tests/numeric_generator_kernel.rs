//! Real Cranelift generator code over explicit test-owned private storage.
//! These functional tests grant no ordinary entry or benchmark qualification.
use super::*;
use crate::native_artifact::{
    codegen::control::plan::CodePlan,
    program::{
        codegen::numeric_generator,
        prepare::{
            PreparedNativeProgram,
            numeric_generators::{
                self,
                banks::{self, Frame, MatrixView},
            },
        },
    },
};
use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use mech_core::snapshot::{F64Bits, SequenceView};
use std::mem::size_of;

fn source(end: &str, reverse: bool, binary: bool) -> String {
    let text =
        super::numeric_generators::source(end).replace("]'\n(answer,image)", "]\n(answer,image)");
    let text = if reverse {
        text.replace("1u64,0.0)", "3u64,0.0)")
            .replace("at < end", "at >= end")
            .replace("at + 1u64", "at - 1u64")
    } else {
        text
    };
    if binary {
        text.replace(
            "(#Scan([image[row,column] | column <- 1u64..=3u64]',a<f64>,",
            "(#Scan([image[row,column] | column <- 1u64..=3u64]',a<f64>,",
        )
        .replace(
            ")) | row",
            ")) + (#Scan([image[row,column] | column <- 1u64..=3u64]',a<f64>,4u64)) | row",
        )
    } else {
        text
    }
}
struct Kernel {
    module: JITModule,
    entry: unsafe extern "C" fn(*mut Frame, *const u64) -> u64,
}
fn compile(
    artifact: &ProgramArtifact,
    region: usize,
) -> (
    Kernel,
    PreparedNativeProgram,
    numeric_generators::Region,
    banks::Layout,
) {
    let catalog = mech_stdlib::source_catalog();
    let jit = JitProgramArtifact::compile(artifact, &catalog).unwrap();
    let prepared = PreparedNativeProgram::prepare(
        artifact,
        &catalog,
        jit.descriptor().metadata().plan.clone(),
    )
    .unwrap();
    let m = prepared.metadata();
    let inventory = numeric_generators::Layout::derive(
        &m.plan,
        &m.buffer_frames,
        &m.scalar_aggregates,
        &m.closed_tails,
    )
    .unwrap();
    let region = inventory.regions[region].clone();
    let banks = banks::Layout::derive(&m.plan, &region, &m.closed_tails).unwrap();
    let mut module =
        JITModule::new(JITBuilder::new(cranelift_module::default_libcall_names()).unwrap());
    let mut context = module.make_context();
    let mut signature = module.make_signature();
    signature
        .params
        .extend([AbiParam::new(types::I64), AbiParam::new(types::I64)]);
    signature.returns.push(AbiParam::new(types::I64));
    context.func.signature = signature.clone();
    let function = module
        .declare_function("checked_generator_test", Linkage::Local, &signature)
        .unwrap();
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let frame = b.block_params(entry)[0];
        let locals = b.block_params(entry)[1];
        let done = b.create_block();
        let miss = b.create_block();
        b.append_block_param(miss, types::I64);
        let malformed = b.create_block();
        numeric_generator::emit(
            &mut b,
            CodePlan::Program(&prepared),
            &region,
            &banks,
            frame,
            locals,
            done,
            miss,
            malformed,
        )
        .unwrap();
        for (block, status) in [(done, 0), (miss, 1), (malformed, 2)] {
            b.switch_to_block(block);
            let result = b.ins().iconst(types::I64, status);
            b.ins().return_(&[result]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    module.define_function(function, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let address = module.get_finalized_function(function);
    // SAFETY: same retained module and exact two-pointer/status C signature.
    let entry = unsafe {
        std::mem::transmute::<*const u8, unsafe extern "C" fn(*mut Frame, *const u64) -> u64>(
            address,
        )
    };
    (Kernel { module, entry }, prepared, region, banks)
}

#[test]
fn numeric_generator_kernel_executes_source_frames_forward_reverse_and_broadcast_arithmetic() {
    let base = source("4u64", false, false);
    let cases = [
        (base.clone(), false, false),
        (source("1u64", true, false), true, false),
        (source("4u64", false, true), false, true),
        (base.replace(")) | row", ")) + a<f64> | row"), false, true),
        (
            base.replace(")) | row", ")) + bias<[f64]:1,3> | row"),
            false,
            true,
        ),
        (
            super::numeric_generators::conditional_source()
                .replace("]'\n(answer,image)", "]\n(answer,image)"),
            false,
            false,
        ),
    ];
    for (text, reverse, binary) in cases {
        for artifact in artifacts(&text) {
            let (kernel, prepared, region, banks) = compile(&artifact, 0);
            let _keep_module = &kernel.module;
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &mech_stdlib::source_catalog(),
                crate::native_artifact::codegen::options::OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let native =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(
                native
                    .descriptor()
                    .metadata()
                    .numeric_generators
                    .regions
                    .len(),
                1,
                "ordinary funding: {:?}",
                native.descriptor().metadata().numeric_generators
            );
            for (gate, values) in [
                (false, [1., 2., 3., 4., 5., 6.]),
                (true, [1., 2., 3., 4., 5., 6.]),
                (true, [-0., 0., f64::MAX, -f64::MAX, f64::INFINITY, 1.]),
                (
                    true,
                    [
                        f64::from_bits(0x7ff8_0000_0000_0042),
                        f64::from_bits(0x7ff0_0000_0000_0031),
                        -0.,
                        0.,
                        1.,
                        -1.,
                    ],
                ),
            ] {
                let cells = values.map(|v| Data::F64(F64Bits::from_f64(v))).to_vec();
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("input", Data::Matrix(cells.into()), &[2, 3]),
                        ("a", Data::F64(F64Bits::from_f64(2.)), &[]),
                        ("gate", Data::Bool(gate), &[]),
                        (
                            "bias",
                            Data::Matrix(
                                [3., 7., 11.]
                                    .map(|v| Data::F64(F64Bits::from_f64(v)))
                                    .into(),
                            ),
                            &[1, 3],
                        ),
                    ],
                );
                let before = contents(&args);
                let mut output = vec![None; artifact.outputs().len()];
                ordinary
                    .evaluate_into(&args, &mut output, mode.execution(), &memory)
                    .unwrap();
                let mut actual = vec![None; artifact.outputs().len()];
                let (result, counts) =
                    observe::run(&native, &args, &mut actual, mode.execution(), &memory);
                result.unwrap();
                let drafts = |xs: &[Option<Value>]| {
                    xs.iter()
                        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    drafts(&actual),
                    drafts(&output),
                    "ordinary generator output"
                );
                let finish =
                    crate::native_artifact::program::protocol::GlobalHelper::NumericGeneratorFinish
                        .raw();
                assert_eq!(
                    counts.helpers.get(&(region.header as u64, finish)),
                    Some(&1),
                    "ordinary entry declined: {counts:?}"
                );
                if values.iter().all(|v| v.is_finite()) {
                    for &site in &region.sites {
                        if site != region.header {
                            assert!(
                                !counts.admits.contains_key(&(site as u64)),
                                "body source callback {site}: {counts:?}"
                            );
                        }
                    }
                }
                drop(actual);
                let mut data = vec![0u64; (region.extents[0] * region.extents[1]) as usize];
                let mut scratch = vec![0u64; banks.elements as usize];
                let _credit = memory
                    .reserve_capacity(
                        ((data.capacity() + scratch.capacity()) * 8 + size_of::<Frame>()) as u64,
                    )
                    .unwrap();
                let mut locals = vec![0u64; prepared.metadata().local_words().unwrap()];
                let mut frame = Frame {
                    output: data.as_mut_ptr() as u64,
                    output_elements: data.len() as u64,
                    scratch: scratch.as_mut_ptr() as u64,
                    scratch_elements: scratch.len() as u64,
                    epoch: 7,
                    issued: 7,
                    source_count: region.count,
                    ..Default::default()
                };
                for input in prepared.metadata().plan.inputs() {
                    let argument = &args[input.input().get() as usize];
                    match argument.data() {
                        ValueData::F64(value) => {
                            locals[input.value().get() as usize] = value.bits()
                        }
                        ValueData::Bool(value) => {
                            locals[input.value().get() as usize] = u64::from(*value)
                        }
                        _ => {}
                    }
                }
                for (at, input) in banks.inputs.iter().enumerate() {
                    let slot = prepared
                        .metadata()
                        .plan
                        .inputs()
                        .iter()
                        .find(|v| v.value().get() == input.value)
                        .unwrap();
                    let ValueData::Matrix(matrix) = args[slot.input().get() as usize].data() else {
                        panic!("matrix input")
                    };
                    let SequenceView::F64(values) = matrix.elements() else {
                        panic!("F64 input")
                    };
                    frame.inputs[at] = MatrixView {
                        input: values.as_ptr() as u64,
                        count: values.len() as u64,
                        rows: input.extents[0],
                        columns: input.extents[1],
                    };
                }
                // SAFETY: retained JIT, pinned typed inputs, exact initialized
                // frame/local banks and disjoint test-owned admitted storage.
                let status = unsafe { (kernel.entry)(&mut frame, locals.as_ptr()) };
                if status == 0 {
                    let expected = output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap();
                    let Data::Tuple(fields) = expected else {
                        panic!("tuple output")
                    };
                    let Data::Matrix(cells) = &fields[0] else {
                        panic!("matrix output")
                    };
                    let expected = cells
                        .iter()
                        .map(|v| {
                            let Data::F64(v) = v else {
                                panic!("F64 output")
                            };
                            v.bits()
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(data, expected, "reverse {reverse}, binary {binary}");
                    assert_eq!(frame.completed, region.count);
                    assert_eq!(frame.traversals[0], region.count + 1);
                } else {
                    assert!(
                        binary && status == 1,
                        "unexpected private miss {status}, reverse {reverse}, binary {binary}"
                    );
                }
                assert_eq!(contents(&args), before);
            }
        }
    }
}

#[test]
fn numeric_generator_kernel_compiles_both_complete_optimization_fixture_roots() {
    for artifact in artifacts(include_str!(
        "../../../../../tests/fixtures/polybench/deriche.mec"
    )) {
        for root in 0..2 {
            let (_kernel, _prepared, region, banks) = compile(&artifact, root);
            assert_eq!(region.extents, [64, 64]);
            assert!(banks.elements > 0);
        }
    }
}

#[test]
fn numeric_generator_kernel_rejects_malformed_geometry_epochs_and_overlapping_spans_before_writes()
{
    for artifact in artifacts(&source("4u64", false, false)) {
        let (kernel, prepared, region, banks) = compile(&artifact, 0);
        let _keep_module = &kernel.module;
        let mut locals = vec![0; prepared.metadata().local_words().unwrap()];
        for input in prepared.metadata().plan.inputs() {
            if prepared.metadata().plan.values()[input.value().get() as usize]
                .scalar_kind()
                .is_some()
            {
                locals[input.value().get() as usize] = 2.0f64.to_bits();
            }
        }
        let input = [1., 2., 3., 4., 5., 6.].map(f64::to_bits);
        for case in 0..19 {
            let mut cells =
                vec![0xfeed_f00d_dead_beefu64; (region.extents[0] * region.extents[1]) as usize];
            let mut scratch = vec![0xcafe_babe_dead_beefu64; banks.elements as usize];
            let mut frame = Frame {
                output: cells.as_mut_ptr() as u64,
                output_elements: cells.len() as u64,
                scratch: scratch.as_mut_ptr() as u64,
                scratch_elements: scratch.len() as u64,
                epoch: 3,
                issued: 3,
                source_count: region.count,
                ..Default::default()
            };
            for (at, source) in banks.inputs.iter().enumerate() {
                frame.inputs[at] = MatrixView {
                    input: input.as_ptr() as u64,
                    count: input.len() as u64,
                    rows: source.extents[0],
                    columns: source.extents[1],
                };
            }
            match case {
                0 => {
                    frame.epoch = 0;
                    frame.issued = 0;
                }
                1 => frame.issued += 1,
                2 => frame.completed = 1,
                3 => frame.source_count += 1,
                4 => frame.output_elements += 1,
                5 => frame.scratch_elements += 1,
                6 => frame.inputs[0].rows += 1,
                7 => frame.inputs[0].columns += 1,
                8 => frame.inputs[0].count += 1,
                9 => frame.traversals[0] = 1,
                10 => frame.output = input.as_ptr() as u64,
                11 => frame.scratch = input.as_ptr() as u64,
                12 => frame.output = 0,
                13 => frame.scratch = 0,
                14 => frame.inputs[0].input = 0,
                15 => frame.inputs[0].input = u64::MAX - 7,
                16 => frame.output = 1,
                17 => frame.scratch = frame.output,
                18 => frame.inputs[0].input = frame.output,
                _ => unreachable!(),
            }
            let expected = frame;
            // Every malformed address is rejected before any attempted access;
            // all genuine backing and the JIT module remain alive throughout.
            let status = unsafe { (kernel.entry)(&mut frame, locals.as_ptr()) };
            assert_eq!(status, 2, "case {case}");
            assert_eq!(frame, expected, "case {case}");
            assert!(cells.iter().all(|&v| v == 0xfeed_f00d_dead_beef));
            assert!(scratch.iter().all(|&v| v == 0xcafe_babe_dead_beef));
            assert_eq!(input, [1., 2., 3., 4., 5., 6.].map(f64::to_bits));
        }
    }
}
