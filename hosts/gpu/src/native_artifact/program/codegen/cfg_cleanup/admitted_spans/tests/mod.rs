use super::*;
use cranelift_codegen::ir::{AbiParam, MemFlags};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
mod fixture;
mod window_tests;
type Run = unsafe extern "C" fn(u64, *const u64, *mut u64) -> u64;

fn compile(normalized: bool, word_floor: bool) -> (JITModule, Run, Run) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let (old, loads) = fixture::body(&module, 0);
    let mut fast = module.make_context();
    fast.func = old.func.clone();
    if normalized {
        let view = super::super::composed_reads::View {
            array: 0,
            pointer_word: 3,
            length_word: 2,
            width: 1,
            literal: None,
        };
        assert_eq!(
            super::super::defaulted_loads::fold_immutable(
                &mut module,
                &mut fast,
                &loads,
                &[view],
                false,
            )
            .unwrap(),
            (1, 1)
        );
    }
    let mut spans = fold(&mut fast, &loads, &BTreeSet::from([2]), false);
    let extra = if word_floor {
        let extra = admit_word_length(&mut fast.func, &loads, &mut spans);
        assert!(extra > 0);
        extra
    } else {
        0
    };
    assert_eq!(
        spans,
        vec![Admission {
            begin_word: 0,
            end_word: 1,
            length_word: 2,
            minimum_length: if word_floor { 8 } else { 0 },
            // Readonly conversion also leaves its old comparison for dead-value retirement.
            checks_removed: (if normalized { 2 } else { 1 }) + extra,
        }]
    );
    finish(module, old, fast, spans)
}

fn finish(
    mut module: JITModule,
    mut old: Context,
    mut fast: Context,
    spans: Vec<Admission>,
) -> (JITModule, Run, Run) {
    let mut functions = Vec::new();
    for (name, context) in [("old", &mut old), ("fast", &mut fast)] {
        context.verify(module.isa()).unwrap();
        let id = module
            .declare_function(name, Linkage::Local, &context.func.signature)
            .unwrap();
        module.define_function(id, context).unwrap();
        functions.push(id);
    }
    let mut wrapper = module.make_context();
    wrapper.func.signature = old.func.signature.clone();
    let calls: Vec<_> = functions
        .iter()
        .map(|&id| module.declare_func_in_func(id, &mut wrapper.func))
        .collect();
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut wrapper.func, &mut fc);
    let entry = b.create_block();
    let yes = b.create_block();
    let no = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let [begin, end, length] =
        [0, 8, 16].map(|offset| b.ins().load(types::I64, MemFlags::new(), args[1], offset));
    let ordered = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, begin, end);
    let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, length);
    let mut admitted = b.ins().band(ordered, fits);
    if spans[0].minimum_length != 0 {
        let enough = b.ins().icmp_imm(
            IntCC::UnsignedGreaterThanOrEqual,
            length,
            spans[0].minimum_length as i64,
        );
        admitted = b.ins().band(admitted, enough);
    }
    b.ins().brif(admitted, yes, &[], no, &[]);
    for (block, function) in [(yes, calls[1]), (no, calls[0])] {
        b.switch_to_block(block);
        let call = b.ins().call(function, &args);
        let status = b.inst_results(call)[0];
        b.ins().return_(&[status]);
    }
    b.seal_all_blocks();
    b.finalize();
    let id = module
        .declare_function("guarded", Linkage::Local, &wrapper.func.signature)
        .unwrap();
    module.define_function(id, &mut wrapper).unwrap();
    module.finalize_definitions().unwrap();
    // Both finalized entries have the declared three-word ABI and remain owned by module.
    let old = unsafe {
        core::mem::transmute::<*const u8, Run>(module.get_finalized_function(functions[0]))
    };
    let guarded =
        unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(id)) };
    (module, old, guarded)
}

fn invoke(run: Run, packet: &[u64; 5], output: &mut [u64; 4]) -> u64 {
    // Finalized fixtures use this exact packet/output ABI; their live arrays
    // and any byte storage named by the fixture remain valid for the call.
    unsafe { run(0, packet.as_ptr(), output.as_mut_ptr()) }
}

fn oracle(bytes: &[u8], mut at: u64, end: u64, budget: u64, marker: u64) -> (u64, [u64; 4]) {
    let mut output = [
        0,
        0,
        bytes.len().saturating_sub(8) as u64,
        marker.wrapping_add(1),
    ];
    while at != end {
        if output[1] == budget {
            output[0] = 102;
            return (2, output);
        }
        let byte = bytes.get(at as usize).copied().unwrap_or(0);
        if byte == 255 {
            output[0] = 101;
            return (1, output);
        }
        output[1] += 1;
        output[2] += u64::from(byte);
        at = at.wrapping_add(1);
    }
    (0, output)
}

fn check(normalized: bool, word_floor: bool) {
    let (_module, old, guarded) = compile(normalized, word_floor);
    let mut count = 0;
    for error in [false, true] {
        let mut bytes = [
            11, 22, 33, 44, 55, 66, 77, 88, 99, 110, 121, 132, 143, 154, 165, 176,
        ];
        if error {
            bytes[3] = 255;
        }
        for begin in [0, 1, 7, 8, 9, u64::MAX - 1, u64::MAX] {
            for end in [0, 1, 7, 8, 9, u64::MAX] {
                for length in [0, 1, 7, 8, 16] {
                    for budget in [0, 1, 8] {
                        for marker in [0, 1, u64::MAX] {
                            let expected = oracle(&bytes[..length], begin, end, budget, marker);
                            let packet = [begin, end, length as u64, bytes.as_ptr() as u64, budget];
                            for run in [old, guarded] {
                                let mut output = [77, 88, 99, marker];
                                // The live input extent and exclusive four-word output match the packet.
                                let status = invoke(run, &packet, &mut output);
                                assert_eq!(
                                    (status, output),
                                    expected,
                                    "{begin}/{end}/{length}/{budget}/{marker}/{error}"
                                );
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(count, 7560);
}

#[test]
fn admitted_span_keeps_default_reads_partial_effects_and_original_errors() {
    check(false, false);
}

#[test]
fn admitted_span_composes_with_readonly_default_loads() {
    check(true, false);
}

#[test]
fn admitted_word_floor_preserves_short_input_fallback_and_saturated_windows() {
    check(true, true);
}

#[test]
fn admitted_span_proof_misses_preserve_graph_and_empty_contract() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for mode in 1..=8 {
        let (mut ctx, loads) = fixture::body(&module, mode);
        let before = ctx.func.display().to_string();
        let mut spans = fold(&mut ctx, &loads, &BTreeSet::from([2]), false);
        assert!(spans.is_empty(), "mode {mode}");
        assert_eq!(admit_word_length(&mut ctx.func, &loads, &mut spans), 0);
        assert_eq!(ctx.func.display().to_string(), before, "mode {mode}");
        ctx.verify(module.isa()).unwrap();
    }
}
