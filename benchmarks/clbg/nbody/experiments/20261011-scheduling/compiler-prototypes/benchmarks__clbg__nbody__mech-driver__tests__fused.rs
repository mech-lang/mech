use mech_core::ReactiveInstanceId;
use mech_engine::{
    __resident::{ActivationFacts, ResidentValueBorrow, activate},
    SlotRole,
};
use mech_gpu::fused_f64::{FusedProgram, JitKernel};
use mech_runtime::RuntimeBuilder;
use std::collections::BTreeMap;

fn compare(source: &str, checkpoints: &[usize]) {
    compare_with_options(source, checkpoints, false, false);
}

fn compare_scheduling_regions(source: &str) {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .unwrap()
        .compile_source(source)
        .unwrap()
        .into_parts();
    let resident = activate(
        ReactiveInstanceId::new(700, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let initial = artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .map(|slot| {
            let ResidentValueBorrow::F64 { values, .. } = resident.state_borrow(slot.slot).unwrap()
            else {
                panic!("f64 state");
            };
            (slot.slot, values.to_vec())
        })
        .collect::<BTreeMap<_, _>>();
    for (region_size, demand) in [
        (8, false),
        (16, false),
        (32, false),
        (64, false),
        (16, true),
    ] {
        for packed in [false, true] {
            for seeds in [false, true] {
                let mut program = FusedProgram::lower(&artifact, &initial).unwrap();
                program.contract_fma();
                if packed {
                    program.pack_state_tails();
                }
                let baseline = JitKernel::compile_with_output(&program, "result", true).unwrap();
                program.schedule_regions(region_size);
                if demand {
                    program.schedule_dependencies();
                }
                if seeds {
                    program.simplify_square_seeds();
                }
                let scalar = JitKernel::compile_with_output(&program, "result", false).unwrap();
                let simd = JitKernel::compile_with_output(&program, "result", true).unwrap();
                for perturb in [false, true] {
                    let mut expected = program.initial.clone();
                    if perturb {
                        for (index, value) in expected.iter_mut().enumerate() {
                            *value = *value * 0.75 + 0.125 + index as f64 * 0.0001;
                        }
                    }
                    let mut a = expected.clone();
                    let mut b = expected.clone();
                    let mut before = 0;
                    for turns in [0, 1, 7, 31, 1000] {
                        baseline.run(&mut expected, turns - before);
                        scalar.run(&mut a, turns - before);
                        simd.run(&mut b, turns - before);
                        for actual in [&a, &b] {
                            assert_eq!(
                                actual
                                    .iter()
                                    .map(|value| value.to_bits())
                                    .collect::<Vec<_>>(),
                                expected
                                    .iter()
                                    .map(|value| value.to_bits())
                                    .collect::<Vec<_>>(),
                                "region scheduling must preserve complete state: packed={packed} seeds={seeds} perturb={perturb} turns={turns}"
                            );
                        }
                        let output = baseline
                            .observe(&expected)
                            .unwrap()
                            .iter()
                            .map(|value| value.to_bits())
                            .collect::<Vec<_>>();
                        assert_eq!(
                            scalar
                                .observe(&a)
                                .unwrap()
                                .iter()
                                .map(|value| value.to_bits())
                                .collect::<Vec<_>>(),
                            output
                        );
                        assert_eq!(
                            simd.observe(&b)
                                .unwrap()
                                .iter()
                                .map(|value| value.to_bits())
                                .collect::<Vec<_>>(),
                            output
                        );
                        before = turns;
                    }
                }
            }
        }
    }
}

#[test]
fn scheduling_regions_preserves_nbody_and_perturbed_initial_states() {
    compare_scheduling_regions(include_str!("../../nbody-matrix-short.mec"));
    compare_scheduling_regions(include_str!("../../nbody-matrix-pairwise.mec"));
}

#[test]
fn scheduling_regions_generalizes_to_unrelated_recurrences() {
    compare_scheduling_regions(
        "+> stats\n~x := [-0.0 2.0 -4.0; 3.0 -5.0 6.0]\nq := stats/sum/column(x ^ 2)\nroot := q ^ 0.5\nscale := 0.025 / (q * root)\nx += x * scale\nstats/sum/row(q)\n",
    );
    compare_scheduling_regions(
        "~x := [-0.0; 0.0; -1.0; 2.0]\nscaled := x * [-0.5; 0.5; -0.25; 0.25]\nx += scaled * 0.125\nscaled\n",
    );
}

#[test]
fn square_seed_simplification_preserves_signed_zero() {
    compare_scheduling_regions(
        "+> stats\n~x := [-0.0 0.0 1.0; 0.0 -1.0 2.0]\nq := stats/sum/column(x ^ 2)\nx += 0.125\nq\n",
    );
}

#[test]
fn column_gathers_and_repeated_updates_preserve_source_order() {
    compare(
        "~x := [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0]\na := x[:,[3 1 3]]\nx[:,[2 2 1]] += a\nx[:,[1 3 1]] -= a * 0.125\nx\n",
        &[0, 1, 3, 9],
    );
    compare(
        "~x := [1.0 2.0 3.0 4.0 5.0]\na := x[:,[5 1 5]]\nx[:,[2 2 1]] += a\nx\n",
        &[0, 1, 4],
    );
}

#[test]
fn column_oriented_nbody_matches_resident_execution() {
    compare(
        include_str!(
            "../../experiments/20261011-competitor-techniques/sources/nbody-matrix-short-columns.mec"
        ),
        &[0, 1, 7, 31, 1000],
    );
    compare(
        include_str!(
            "../../experiments/20261011-competitor-techniques/sources/nbody-matrix-pairwise-columns.mec"
        ),
        &[0, 1, 7, 31, 1000],
    );
}

fn compare_with_options(source: &str, checkpoints: &[usize], contract: bool, refined: bool) {
    compare_with_compiler_options(source, checkpoints, contract, refined, false);
}

fn compare_with_compiler_options(
    source: &str,
    checkpoints: &[usize],
    contract: bool,
    refined: bool,
    reassociate: bool,
) {
    compare_arithmetic_policies(source, checkpoints, contract, refined, reassociate, false);
}

fn compare_arithmetic_policies(
    source: &str,
    checkpoints: &[usize],
    contract: bool,
    refined: bool,
    reassociate: bool,
    share_products: bool,
) {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .unwrap()
        .compile_source(source)
        .unwrap()
        .into_parts();
    let mut resident = activate(
        ReactiveInstanceId::new(500, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let initial = artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .map(|slot| {
            let ResidentValueBorrow::F64 { values, .. } = resident.state_borrow(slot.slot).unwrap()
            else {
                panic!("f64")
            };
            (slot.slot, values.to_vec())
        })
        .collect::<BTreeMap<_, _>>();
    let mut program = FusedProgram::lower(&artifact, &initial).unwrap();
    if refined {
        program.refine_reciprocal();
    }
    if share_products {
        let before = program.c_kernel().matches(" * ").count();
        program.share_scaled_products();
        let after = program.c_kernel();
        assert!(
            after.matches(" * ").count() < before,
            "sharing must remove arithmetic"
        );
        program.share_scaled_products();
        assert_eq!(program.c_kernel(), after, "sharing must be idempotent");
    }
    if contract {
        program.contract_fma();
    }
    if reassociate {
        program.reassociate_constant_products();
    }
    let output = if source.contains("<+ energy") {
        "energy"
    } else {
        "result"
    };
    let output_index = artifact
        .outputs()
        .iter()
        .position(|decl| decl.name == output)
        .unwrap();
    let unscheduled = JitKernel::compile_with_output(&program, output, true).unwrap();
    if share_products {
        let mut unshared = FusedProgram::lower(&artifact, &initial).unwrap();
        if contract {
            unshared.contract_fma();
        }
        let baseline = JitKernel::compile_with_output(&unshared, output, true).unwrap();
        let mut expected = program
            .initial
            .iter()
            .enumerate()
            .map(|(index, value)| value * 0.75 + 0.125 + index as f64 * 0.0001)
            .collect::<Vec<_>>();
        let mut actual = expected.clone();
        let mut previous = 0;
        for turns in [0, 1, 7, 31, 1000] {
            baseline.run(&mut expected, turns - previous);
            unscheduled.run(&mut actual, turns - previous);
            for (a, b) in actual.iter().zip(&expected) {
                assert!(
                    (a - b).abs() <= 1e-10 * b.abs().max(1.0),
                    "shared-product perturbed state after {turns}: {a} != {b}"
                );
            }
            previous = turns;
        }
    }
    program.schedule_dependencies();
    let jit = JitKernel::compile_with_output(&program, output, false).unwrap();
    let simd = JitKernel::compile_with_output(&program, output, true).unwrap();
    program.pack_state_tails();
    let packed_simd = JitKernel::compile_with_output(&program, output, true).unwrap();
    let mut state = program.initial.clone();
    let mut simd_state = state.clone();
    let mut unscheduled_state = state.clone();
    let mut packed_state = state.clone();
    let mut before = 0;
    for &checkpoint in checkpoints {
        jit.run(&mut state, checkpoint - before);
        simd.run(&mut simd_state, checkpoint - before);
        unscheduled.run(&mut unscheduled_state, checkpoint - before);
        packed_simd.run(&mut packed_state, checkpoint - before);
        assert_eq!(
            state.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            packed_state.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            "packed state tails must preserve state bits at {checkpoint}"
        );
        for (index, (scalar, vector)) in state.iter().zip(&simd_state).enumerate() {
            assert_eq!(
                scalar.to_bits(),
                vector.to_bits(),
                "scalar/SIMD state bits at {checkpoint}, component {index}"
            );
        }
        assert_eq!(
            state.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            unscheduled_state
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            "scheduling must preserve state bits at {checkpoint}"
        );
        for _ in before..checkpoint {
            resident.turn_without_summary(&[]).unwrap();
        }
        let state_before = state.clone();
        let actual_output = jit.observe(&state).unwrap();
        let simd_output = simd.observe(&simd_state).unwrap();
        assert_eq!(state, state_before, "observation must not advance state");
        let expected_output = {
            let candidate = resident.prepare_turn(&[]).unwrap();
            let Some(ResidentValueBorrow::F64 { values, .. }) =
                candidate.output_borrow(output_index)
            else {
                panic!("f64 output")
            };
            values.to_vec()
        };
        assert_eq!(actual_output.len(), expected_output.len());
        for ((actual, vector), expected) in
            actual_output.iter().zip(&simd_output).zip(&expected_output)
        {
            assert_eq!(
                actual.to_bits(),
                vector.to_bits(),
                "scalar/SIMD output at {checkpoint}"
            );
            assert!(
                (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
                "output at {checkpoint}: {actual} != {expected}"
            );
        }
        for (slot, offset, len) in &program.states {
            let ResidentValueBorrow::F64 { values, .. } = resident.state_borrow(*slot).unwrap()
            else {
                panic!("f64")
            };
            for (component, (actual, expected)) in
                state[*offset..offset + len].iter().zip(values).enumerate()
            {
                assert!(
                    (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
                    "turn {checkpoint} slot {slot:?} component {component}: {actual} != {expected}"
                );
            }
        }
        before = checkpoint;
    }
}

#[test]
fn pairwise_nbody_matches_resident() {
    for contract in [false, true] {
        compare_with_options(
            include_str!("../../nbody-pairwise.mec"),
            &[0, 1, 1000, 10_000],
            contract,
            false,
        );
    }
}

#[test]
fn mass_first_pairwise_nbody_matches_resident() {
    for contract in [false, true] {
        compare_with_options(
            include_str!("../../nbody-pairwise-mass.mec"),
            &[0, 1, 1000, 10_000],
            contract,
            false,
        );
    }
}

#[test]
fn compact_matrix_nbody_with_implicit_result_matches_resident() {
    for source in [
        include_str!("../../nbody-matrix-short.mec"),
        include_str!("../../nbody-matrix-pairwise.mec"),
    ] {
        for contract in [false, true] {
            compare_with_options(source, &[0, 1, 1000, 10_000], contract, false);
        }
    }
}

#[test]
fn native_runner_defaults_to_the_final_expression() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../nbody-matrix-short.mec");
    for mode in ["resident", "jit-simd"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech-clbg-nbody-compiled"))
            .arg("1000")
            .arg(&source)
            .arg(mode)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "-0.169075164\n-0.169087605\n"
        );
    }
}

#[test]
fn constant_product_reassociation_matches_resident_and_simd() {
    compare_with_compiler_options(
        include_str!("../../nbody-pairwise.mec"),
        &[0, 1, 1000, 10_000],
        true,
        false,
        true,
    );
    compare_with_compiler_options(
        "+> stats\n~x := [1.0 2.0 3.0; 4.0 5.0 6.0]\nd := x - 0.5\nq := stats/sum/column(d ^ 2)\nlate := 0.1 / (q * q ^ 0.5)\nproduct := d * late\nx += product * 0.01\nresult := stats/sum/row(x)\n",
        &[0, 1, 7, 1000],
        true,
        false,
        true,
    );
}

#[test]
fn square_reduction_fma_preserves_zero_and_output() {
    compare_with_options(
        "+> stats\n~x := [-0.0 0.0; 0.25 -0.25; -0.5 0.5]\nx += x * 0.01\nresult := stats/sum/column(x ^ 2)\n",
        &[0, 1, 7, 23],
        true,
        false,
    );
}

#[test]
fn negative_fma_coefficients_match_resident_and_scheduled_simd() {
    compare_with_options(
        "~x := [-0.0; 0.0; 0.25; -0.25]\ny := x * -2.0 + 0.125\nz := 0.5 - x * -0.25\nx += y * 0.01\nresult := y + z\n",
        &[0, 1, 7, 23],
        true,
        false,
    );
}

#[test]
fn nbody_fma_and_refined_reciprocal_match_resident_tolerance() {
    for source in [
        include_str!("../../nbody.mec"),
        include_str!("../../nbody-power.mec"),
        include_str!("../../nbody-factored.mec"),
    ] {
        for (contract, refined) in [(true, false), (false, true), (true, true)] {
            compare_with_options(source, &[0, 1, 10, 1000, 10_000], contract, refined);
        }
    }
}

#[test]
fn compact_inverse_power_algebraic_lowering_matches_resident() {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .unwrap()
        .compile_source(include_str!("../../nbody-power.mec"))
        .unwrap()
        .into_parts();
    let mut resident = activate(
        ReactiveInstanceId::new(501, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let initial = artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .map(|slot| {
            let ResidentValueBorrow::F64 { values, .. } = resident.state_borrow(slot.slot).unwrap()
            else {
                panic!("f64")
            };
            (slot.slot, values.to_vec())
        })
        .collect::<BTreeMap<_, _>>();
    let mut program = FusedProgram::lower(&artifact, &initial).unwrap();
    program.lower_algebraic_powers();
    program.contract_fma();
    let kernel = JitKernel::compile_with_output(&program, "energy", true).unwrap();
    let mut state = program.initial.clone();
    kernel.run(&mut state, 10_000);
    for _ in 0..10_000 {
        resident.turn_without_summary(&[]).unwrap();
    }
    for (slot, offset, len) in &program.states {
        let ResidentValueBorrow::F64 { values, .. } = resident.state_borrow(*slot).unwrap() else {
            panic!("f64")
        };
        for (actual, expected) in state[*offset..offset + len].iter().zip(values) {
            assert!((actual - expected).abs() <= 1e-10 * expected.abs().max(1.0));
        }
    }
}

#[test]
fn opt_in_transformations_apply_to_unrelated_arithmetic() {
    compare_with_options(
        "~q := [4.0; 9.0; 16.0]\n~v := [0.0; 1.0; 2.0]\nscale := 0.01 / (q * q ^ 0.5)\nv += q * scale\nq += 0.001\nvalue := v * q + 0.125\n",
        &[0, 1, 7, 100],
        true,
        true,
    );
}

#[test]
fn named_constant_output_without_state_is_supported() {
    compare("answer := 42.0\n<+ answer\n", &[0, 1, 7]);
}

#[test]
fn odd_width_matrix_preserves_repeated_scatter_order() {
    compare(
        "~x := [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0; 10.0 11.0 12.0]\nindices := [1; 1; 3]\nupdate := [0.125 0.25 0.5; 0.25 0.5 1.0; 0.5 1.0 2.0]\nx[indices,:] += update\nresult := x\n",
        &[0, 1, 7, 23],
    );
}

#[test]
fn unknown_output_is_rejected_before_native_execution() {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog)
        .build_compiler()
        .unwrap()
        .compile_source("answer := 42.0\n<+ answer\n")
        .unwrap()
        .into_parts();
    let program = FusedProgram::lower(&artifact, &BTreeMap::new()).unwrap();
    assert_eq!(program.output_len("answer").unwrap(), 1);
    assert!(program.output_len("missing").is_err());
    assert!(JitKernel::compile_with_output(&program, "missing", true).is_err());
}

#[test]
fn rectangular_transpose_keeps_live_state_and_output_order() {
    compare(
        "~x := [1.0 2.0 3.0; 4.0 5.0 6.0]\nresult := x'\nx += [0.125 0.25 0.5; 0.75 1.0 1.5]\n",
        &[0, 1, 2, 7, 23],
    );
}

#[test]
fn vector_double_transpose_preserves_live_values() {
    compare(
        "~x := [1.0; 2.0; 3.0]\nrow := x'\nresult := row'\nx += 0.125\n",
        &[0, 1, 2, 7, 23],
    );
}

#[test]
fn transposed_constant_matrix_can_initialize_state() {
    compare(
        "~x := [1.0 2.0 3.0; 4.0 5.0 6.0]'\nx += 0.125\nresult := x\n",
        &[0, 1, 2, 7, 23],
    );
}

#[test]
fn nbody_complete_state_matches_resident_sqrt() {
    compare(
        include_str!("../../nbody.mec"),
        &[0, 1, 2, 10, 100, 1000, 10_000],
    );
}

#[test]
fn nbody_complete_state_matches_resident_power() {
    compare(
        include_str!("../../nbody-power.mec"),
        &[0, 1, 2, 10, 100, 1000, 10_000],
    );
}

#[test]
fn nbody_complete_state_matches_resident_factored() {
    compare(
        include_str!("../../nbody-factored.mec"),
        &[0, 1, 2, 10, 100, 1000, 10_000],
    );
}

#[test]
fn generic_repeated_scatter_and_broadcast_keep_source_order() {
    compare(
        "~x := [1 2; 3 4]\nx[[1;1;2],:] += [0.25 0.5; 0.75 1.5; 2.0 3.0]\nx += 0.125\ny := x\n",
        &[0, 1, 7, 23],
    );
}

#[test]
fn generic_multiple_states_keep_old_and_next_values_distinct() {
    compare(
        "~x := [1; 2; 3]\n~v := [0.5; 1.5; 2.5]\nv += x * 0.01\nx += v * 0.1\ny := x\n",
        &[0, 1, 2, 25],
    );
}

#[test]
fn generic_odd_state_length_keeps_simd_tail() {
    compare("~x := [1; 2; 3]\nx += 0.25\ny := x\n", &[0, 1, 7, 23]);
}

#[test]
fn reductions_keep_summand_order_in_each_lane() {
    compare(
        "+> stats\n~x := [10000000000000000 -10000000000000000 1; -10000000000000000 10000000000000000 2; 1 2 3]\n~y := [0; 0; 0]\n~z := [0 0 0]\ny = stats/sum/column(x)\nz = stats/sum/row(x)\nx += 0.125\n",
        &[0, 1, 2, 7, 23],
    );
}

#[test]
fn simd_preserves_signed_zero() {
    compare(
        "~x := [0.0 -0.0; -0.0 0.0]\nx = -x\ny := x\n",
        &[0, 1, 2, 7],
    );
}

#[test]
fn iros_f32_aot_and_jit_match_scalar_with_v04_remainder() {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog)
        .build_compiler()
        .unwrap()
        .compile_source(
            "~x := [1<f32> 2<f32>; 3<f32> 4<f32>]\nx = (x + 0.25<f32>) % 3.5<f32>\ny := x\n",
        )
        .unwrap()
        .into_parts();
    let program = mech_gpu::ComputeLowerer
        .compile_batched(&artifact, 4)
        .unwrap();
    let inputs = BTreeMap::new();
    let mut scalar = program.prepare_cpu(&inputs).unwrap();
    let mut jit = program.prepare_jit_cpu(&inputs).unwrap();
    let directory = std::env::temp_dir().join(format!("mech-clbg-aot-test-{}", std::process::id()));
    let artifact = program.compile_aot_cpu_to(&directory).unwrap();
    let mut aot = artifact.prepare(&inputs).unwrap();
    scalar.dispatch_turns(7).unwrap();
    jit.dispatch_turns(7).unwrap();
    aot.dispatch_turns(7).unwrap();
    assert_eq!(scalar.state(), jit.state());
    assert_eq!(scalar.state(), aot.state());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn exact_binary_division_rewrite_matches_resident_scalar_and_simd() {
    compare(
        "~x := [-0.0; 0.0; 0.125; -0.5]\nx += 0.03125\nresult := x / 2.0 + x / 0.5 + x / -4.0\n",
        &[0, 1, 7, 23],
    );
}

#[test]
fn shared_scaled_products_match_complete_resident_state() {
    for contract in [false, true] {
        compare_arithmetic_policies(
            include_str!("../../nbody-matrix-pairwise.mec"),
            &[0, 1, 1000, 10_000],
            contract,
            false,
            false,
            true,
        );
        compare_arithmetic_policies(
            "+> stats\n~x := [0.1 -0.2 0.3; -0.4 0.5 -0.6]\n~y := [0.6 -0.5 0.4; -0.3 0.2 -0.1]\na := x - y\nb := 0.01 / (1 + a ^ 2)\nx += (a * 0.25) * b\ny -= b * (a * -0.5)\nresult := stats/sum/row(x + y)\n",
            &[0, 1, 7, 128],
            contract,
            false,
            false,
            true,
        );
    }
}

fn compare_alternative_schedules(source: &str) {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .unwrap()
        .compile_source(source)
        .unwrap()
        .into_parts();
    let resident = activate(
        ReactiveInstanceId::new(700, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let initial = artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .map(|slot| {
            let ResidentValueBorrow::F64 { values, .. } = resident.state_borrow(slot.slot).unwrap()
            else {
                panic!("f64 state");
            };
            (slot.slot, values.to_vec())
        })
        .collect::<BTreeMap<_, _>>();
    for (region_size, policy, decompose) in [
        (8, 1, false),
        (24, 1, true),
        (8, 2, false),
        (24, 2, true),
        (8, 0, true),
        (8, 3, false),
        (24, 3, true),
    ] {
        for packed in [false, true] {
            for seeds in [false, true] {
                let mut program = FusedProgram::lower(&artifact, &initial).unwrap();
                program.lower_algebraic_powers();
                program.contract_fma();
                if packed {
                    program.pack_state_tails();
                }
                let baseline = JitKernel::compile_with_output(&program, "result", true).unwrap();
                program.schedule_regions(region_size);
                program.defer_simd_lanes();
                if decompose {
                    program.decompose_arithmetic();
                    program.decompose_arithmetic();
                }
                match policy {
                    1 => program.schedule_bottom_up(),
                    2 => program.schedule_latency(),
                    3 => program.schedule_retire(),
                    _ => {}
                }
                if seeds {
                    program.simplify_square_seeds();
                }
                let scalar = JitKernel::compile_with_output(&program, "result", false).unwrap();
                let simd = JitKernel::compile_with_output(&program, "result", true).unwrap();
                for perturb in [false, true] {
                    let mut expected = program.initial.clone();
                    if perturb {
                        for (index, value) in expected.iter_mut().enumerate() {
                            *value = *value * 0.75 + 0.125 + index as f64 * 0.0001;
                        }
                    }
                    let mut a = expected.clone();
                    let mut b = expected.clone();
                    let mut before = 0;
                    for turns in [0, 1, 7, 257] {
                        baseline.run(&mut expected, turns - before);
                        scalar.run(&mut a, turns - before);
                        simd.run(&mut b, turns - before);
                        for actual in [&a, &b] {
                            assert_eq!(
                                actual
                                    .iter()
                                    .map(|value| value.to_bits())
                                    .collect::<Vec<_>>(),
                                expected
                                    .iter()
                                    .map(|value| value.to_bits())
                                    .collect::<Vec<_>>(),
                                "alternative schedules must preserve complete state: policy={policy} decompose={decompose} packed={packed} seeds={seeds} perturb={perturb} turns={turns}"
                            );
                        }
                        let output = baseline
                            .observe(&expected)
                            .unwrap()
                            .iter()
                            .map(|value| value.to_bits())
                            .collect::<Vec<_>>();
                        assert_eq!(
                            scalar
                                .observe(&a)
                                .unwrap()
                                .iter()
                                .map(|value| value.to_bits())
                                .collect::<Vec<_>>(),
                            output
                        );
                        assert_eq!(
                            simd.observe(&b)
                                .unwrap()
                                .iter()
                                .map(|value| value.to_bits())
                                .collect::<Vec<_>>(),
                            output
                        );
                        before = turns;
                    }
                }
            }
        }
    }
}

#[test]
fn alternative_schedules_preserve_states_and_observations() {
    compare_alternative_schedules(include_str!("../../nbody-matrix-short.mec"));
    compare_alternative_schedules(include_str!("../../nbody-matrix-pairwise.mec"));
    compare_alternative_schedules(
        "+> stats\n~x := [1.0 2.0 3.0; 4.0 5.0 6.0]\nq := stats/sum/column(x ^ 2)\nw := q ^ -1.5\nx += x * w * 0.025\nstats/sum/row(q)\n",
    );
    compare_alternative_schedules(
        "~x := [-0.0; 0.0; -1.0; 2.0]\nscaled := x * [-0.5; 0.5; -0.25; 0.25]\nx += scaled * 0.125\nscaled\n",
    );
}
