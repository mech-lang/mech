use mech_core::ReactiveInstanceId;
use mech_engine::{
    __resident::{
        ActivationFacts, CapturedSignalInput, ReactiveInstance, ResidentValueBorrow, activate,
    },
    ProgramArtifact, ResidentValueRef, SlotRole,
};
use mech_gpu::fused_f64::{FusedProgram, MapKernel};
use mech_runtime::RuntimeBuilder;
use std::collections::BTreeMap;

fn compile(source: &str) -> (ProgramArtifact, ReactiveInstance, FusedProgram) {
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .unwrap()
        .compile_source(source)
        .unwrap()
        .into_parts();
    let resident = activate(
        ReactiveInstanceId::new(601, 0),
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
            let Some(ResidentValueBorrow::F64 { values, .. }) = resident.state_borrow(slot.slot)
            else {
                panic!("f64 state")
            };
            (slot.slot, values.to_vec())
        })
        .collect::<BTreeMap<_, _>>();
    let program = FusedProgram::lower(&artifact, &initial).unwrap();
    (artifact, resident, program)
}

fn inputs(program: &FusedProgram, named: &BTreeMap<&str, Vec<f64>>) -> Vec<f64> {
    program
        .inputs
        .iter()
        .flat_map(|input| named[input.name.as_str()].clone())
        .collect()
}

fn reference(n: usize, x: usize, y: usize) -> bool {
    let scale = 2.0 / n as f64;
    let cr = x as f64 * scale - 1.5;
    let ci = y as f64 * scale - 1.0;
    let (mut zr, mut zi, mut tr, mut ti) = (0.0, 0.0, 0.0, 0.0);
    for _ in 0..50 {
        zi = 2.0 * zr * zi + ci;
        zr = tr - ti + cr;
        tr = zr * zr;
        ti = zi * zi;
        if tr + ti > 4.0 {
            break;
        }
    }
    tr + ti <= 4.0
}

#[test]
fn scalar_and_simd_mandelbrot_match_complete_official_bitmap() {
    for source in [
        include_str!("../../mandelbrot.mec"),
        include_str!("../../mandelbrot-unrolled.mec"),
        include_str!("../../mandelbrot-unrolled-add.mec"),
        include_str!("../../mandelbrot-unrolled-10.mec"),
    ] {
        for contract in [false, true] {
            let (_, _, mut program) = compile(source);
            if contract {
                program.contract_loop_fma();
            }
            for simd in [false, true] {
                let kernel = MapKernel::compile(&program, "done", "inside", simd).unwrap();
                for n in [8, 16, 24, 200] {
                    let mut bitmap = format!("P4\n{n} {n}\n").into_bytes();
                    for y in 0..n {
                        for x in (0..n).step_by(8) {
                            let named = BTreeMap::from([
                                ("width", vec![n as f64]),
                                ("row", vec![y as f64]),
                                ("pixel", (x..x + 8).map(|i| i as f64).collect()),
                            ]);
                            let output = kernel.evaluate(&inputs(&program, &named)).unwrap();
                            let mut byte = 0;
                            for lane in 0..8 {
                                assert_eq!(
                                    output[lane] != 0.0,
                                    reference(n, x + lane, y),
                                    "N={n}, ({},{y}), SIMD={simd}",
                                    x + lane
                                );
                                if output[lane] != 0.0 {
                                    byte |= 128 >> lane;
                                }
                            }
                            bitmap.push(byte);
                        }
                    }
                    if n == 200 {
                        assert_eq!(bitmap, include_bytes!("../../upstream/mandelbrot-200.pbm"));
                    }
                }
            }
        }
    }
}

#[test]
fn odd_width_map_state_packing_preserves_complete_output() {
    let (_, _, mut program) = compile(
        "~x := [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0]\n~step := 0.0\nx += delta<f64>\nstep += 1.0\ndone := step >= 4.0\nanswer := x\n<+ answer\n<+ done\n",
    );
    let scalar = MapKernel::compile(&program, "done", "answer", false).unwrap();
    program.pack_state_tails();
    let packed = MapKernel::compile(&program, "done", "answer", true).unwrap();
    for delta in [-0.125, 0.0, 0.25] {
        let named = BTreeMap::from([("delta", vec![delta])]);
        let flat = inputs(&program, &named);
        let expected = [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0].map(|x| x + 4.0 * delta);
        for kernel in [&scalar, &packed] {
            let actual = kernel.evaluate(&flat).unwrap();
            assert_eq!(
                actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn loop_fma_preserves_immutable_product_rounding() {
    let (_, _, mut program) = compile(
        "product := x<f64> * y<f64>\n~value := -1.0\nanswer := value\ndone := value != -1.0\nvalue += product\n<+ answer\n<+ done\n",
    );
    program.contract_loop_fma();
    for simd in [false, true] {
        let kernel = MapKernel::compile(&program, "done", "answer", simd).unwrap();
        let epsilon = 2.0_f64.powi(-27);
        let named = BTreeMap::from([("x", vec![1.0 + epsilon]), ("y", vec![1.0 - epsilon])]);
        assert_eq!(
            kernel.evaluate(&inputs(&program, &named)).unwrap(),
            vec![0.0]
        );
    }
}

#[test]
fn map_matches_resident_with_explicit_pixel_inputs() {
    let (artifact, mut resident, program) = compile(include_str!("../../mandelbrot.mec"));
    let kernel = MapKernel::compile(&program, "done", "inside", true).unwrap();
    let named = BTreeMap::from([
        ("width", vec![200.0]),
        ("row", vec![37.0]),
        ("pixel", (96..104).map(f64::from).collect()),
    ]);
    let flat = inputs(&program, &named);
    let captured = artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let slot = resident
                .plan
                .inputs
                .iter()
                .find(|plan| plan.artifact_slot == input.slot)
                .unwrap()
                .slot;
            CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&named[name.as_str()]),
            }
        })
        .collect::<Vec<_>>();
    let output_index = |name: &str| {
        artifact
            .outputs()
            .iter()
            .position(|out| out.name == name)
            .unwrap()
    };
    let mut completed = false;
    for _ in 0..=50 {
        let candidate = resident.prepare_turn(&captured).unwrap();
        let Some(ResidentValueBorrow::Bool { values, .. }) =
            candidate.output_borrow(output_index("done"))
        else {
            panic!("Boolean stop")
        };
        if values[0] != 0 {
            let Some(ResidentValueBorrow::Bool { values, .. }) =
                candidate.output_borrow(output_index("inside"))
            else {
                panic!("Boolean output")
            };
            assert_eq!(
                kernel.evaluate(&flat).unwrap(),
                values.iter().map(|&b| f64::from(b)).collect::<Vec<_>>()
            );
            completed = true;
            break;
        }
        drop(candidate);
        resident.turn_without_summary(&captured).unwrap();
    }
    assert!(
        completed,
        "source must terminate within fifty state commits"
    );
}

#[test]
fn unrelated_map_resets_state_and_obeys_source_stop() {
    let (_, _, program) = compile(
        "~value := 0.0\nanswer := value\ndone := value >= limit<f64>\nvalue += increment<f64>\n<+ answer\n<+ done\n",
    );
    for simd in [false, true] {
        let kernel = MapKernel::compile(&program, "done", "answer", simd).unwrap();
        for (limit, increment, expected) in [
            (5.0, 0.5, 5.0),
            (0.0, 1.0, 0.0),
            (4.0, 1.0, 4.0),
            (5.0, 0.5, 5.0),
        ] {
            let named = BTreeMap::from([("limit", vec![limit]), ("increment", vec![increment])]);
            assert_eq!(
                kernel.evaluate(&inputs(&program, &named)).unwrap(),
                vec![expected]
            );
        }
        assert!(kernel.evaluate(&[]).is_err());
    }
}

#[test]
fn boolean_scalar_access_works_for_linear_and_two_dimensional_indices() {
    for index in ["flags[2]", "flags[2,1]"] {
        let source = format!(
            "flags := values<[f64]:2,2> > 0.0\nanswer := {index}\ndone := true\n<+ answer\n<+ done\n"
        );
        let (artifact, mut resident, program) = compile(&source);
        let kernel = MapKernel::compile(&program, "done", "answer", true).unwrap();
        let input = artifact.inputs().first().unwrap();
        let slot = resident
            .plan
            .inputs
            .iter()
            .find(|plan| plan.artifact_slot == input.slot)
            .unwrap()
            .slot;
        let output = artifact
            .outputs()
            .iter()
            .position(|out| out.name == "answer")
            .unwrap();
        for data in [vec![1.0, -1.0, 0.0, 2.0], vec![-1.0, 1.0, 2.0, 0.0]] {
            let actual = kernel.evaluate(&data).unwrap();
            let candidate = resident
                .prepare_turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::F64(&data),
                }])
                .unwrap();
            let Some(ResidentValueBorrow::Bool { values, .. }) = candidate.output_borrow(output)
            else {
                panic!("Bool")
            };
            assert_eq!(actual, vec![f64::from(values[0])]);
        }
    }
}

#[test]
fn comparisons_keep_ieee_nan_and_boundary_semantics() {
    let (_, _, program) =
        compile("answer := value<f64> <= 4.0\ndone := true\n<+ answer\n<+ done\n");
    for simd in [false, true] {
        let kernel = MapKernel::compile(&program, "done", "answer", simd).unwrap();
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            4.0,
            4.000000000000001,
            -0.0,
        ] {
            assert_eq!(
                kernel.evaluate(&[value]).unwrap(),
                vec![f64::from(value <= 4.0)]
            );
        }
    }
}

#[test]
fn map_rejects_numeric_and_vector_termination_outputs() {
    let (_, _, program) = compile("answer := value<f64>\ndone := 1.0\n<+ answer\n<+ done\n");
    assert!(MapKernel::compile(&program, "done", "answer", true).is_err());
    let (_, _, program) =
        compile("answer := values<[f64]:2,1>\ndone := answer > 0.0\n<+ answer\n<+ done\n");
    assert!(MapKernel::compile(&program, "done", "answer", true).is_err());
}

#[test]
fn boolean_masks_do_not_cache_an_uncomputed_sibling_lane() {
    let (_, _, program) = compile(
        "doubled := values<[f64]:2,1> * 2.0\nanswer := doubled > 0.0\ndone := answer[1,1]\n<+ answer\n<+ done\n",
    );
    for simd in [false, true] {
        let kernel = MapKernel::compile(&program, "done", "answer", simd).unwrap();
        for data in [vec![1.0, 2.0], vec![1.0, -2.0]] {
            assert_eq!(
                kernel.evaluate(&data).unwrap(),
                vec![1.0, f64::from(data[1] > 0.0)]
            );
        }
    }
}

#[test]
fn native_boolean_logic_keeps_canonical_values() {
    let (_, _, program) = compile(
        "flags := values<[f64]:2,1> <= 4.0\nanswer := (flags && true) || !flags\ndone := flags[1,1] == flags[1,1]\n<+ answer\n<+ done\n",
    );
    for simd in [false, true] {
        let kernel = MapKernel::compile(&program, "done", "answer", simd).unwrap();
        for data in [vec![f64::NAN, 4.0], vec![-0.0, f64::INFINITY]] {
            assert_eq!(kernel.evaluate(&data).unwrap(), vec![1.0, 1.0]);
        }
    }
}

#[test]
fn packed_maps_keep_bit_order_boundaries_and_input_validation() {
    for lanes in [1, 2, 7, 8, 9, 63, 64] {
        let (_, _, program) = compile(&format!(
            "answer := values<[f64]:{lanes},1> <= 4.0\ndone := true\n<+ answer\n<+ done\n"
        ));
        let data = (0..lanes)
            .map(|i| match i % 4 {
                0 => f64::NAN,
                1 => 4.0,
                2 => -0.0,
                _ => f64::INFINITY,
            })
            .collect::<Vec<_>>();
        for simd in [false, true] {
            let kernel = MapKernel::compile_bitmap(&program, "done", "answer", simd).unwrap();
            assert_eq!(
                kernel.evaluate(&data).unwrap(),
                data.iter()
                    .map(|&v| f64::from(v <= 4.0))
                    .collect::<Vec<_>>()
            );
            assert!(kernel.evaluate(&[]).is_err());
        }
    }
    for source in [
        "answer := value<f64>\ndone := true\n<+ answer\n<+ done\n",
        "answer := values<[f64]:65,1> <= 4.0\ndone := true\n<+ answer\n<+ done\n",
    ] {
        let (_, _, program) = compile(source);
        assert!(MapKernel::compile_bitmap(&program, "done", "answer", true).is_err());
    }
    let (_, _, mut program) = compile(include_str!("../../mandelbrot-unrolled.mec"));
    program.contract_loop_fma();
    let kernel = MapKernel::compile_bitmap(&program, "done", "inside", true).unwrap();
    let mut bitmap = b"P4\n200 200\n".to_vec();
    for y in 0..200 {
        for x in (0..200).step_by(8) {
            let named = BTreeMap::from([
                ("width", vec![200.0]),
                ("row", vec![y as f64]),
                ("pixel", (x..x + 8).map(|i| i as f64).collect()),
            ]);
            let output = kernel.evaluate(&inputs(&program, &named)).unwrap();
            bitmap.push(output.iter().enumerate().fold(0_u8, |byte, (lane, &flag)| {
                byte | ((flag as u8) << (7 - lane))
            }));
        }
    }
    assert_eq!(bitmap, include_bytes!("../../upstream/mandelbrot-200.pbm"));
}

#[test]
fn source_defined_grid_kernels_use_the_canonical_compiler() {
    for (source, lanes) in [
        (include_str!("../../mandelbrot-compact-program.mec"), 1),
        (include_str!("../../mandelbrot-program.mec"), 8),
    ] {
        let grid =
            mech_engine::native_sequence::grid::compile(source, mech_stdlib::native_grid_catalog())
                .unwrap();
        assert_eq!(grid.lanes, lanes);
        let (_, _, mut program) = compile(&grid.kernel_source);
        program.contract_loop_fma();
        for simd in [false, true] {
            let kernel = MapKernel::compile_bitmap(&program, "done", "inside", simd).unwrap();
            for y in [0, 37, 100, 199] {
                for x in (0..200).step_by(lanes) {
                    let named = BTreeMap::from([
                        ("width", vec![200.0]),
                        ("row", vec![y as f64]),
                        ("pixel", (x..x + lanes).map(|i| i as f64).collect()),
                    ]);
                    let actual = kernel.evaluate(&inputs(&program, &named)).unwrap();
                    for lane in 0..lanes {
                        assert_eq!(actual[lane] != 0.0, reference(200, x + lane, y));
                    }
                }
            }
        }
    }
}

#[test]
fn grid_admission_requires_source_kernel_and_complete_source_orchestration() {
    let valid = "pixels(width<f64>, column<f64>, row<f64>) = (output<[bool]:1,2>) := output := [column + row < width, true].\nsize := io/argument-u64(1)\nimage := grid/map-until(size, size, pixels)\nio/write(image/pbm(image))\n";
    let compile_grid = |source: &str| {
        mech_engine::native_sequence::grid::compile(source, mech_stdlib::native_grid_catalog())
    };
    assert!(compile_grid(valid).is_ok());
    assert!(compile_grid(&format!("-- General grid comment\n{valid}")).is_ok());
    for invalid in [
        valid.replace("size :=", "~size :="),
        valid.replace("pixels)", "host/mandelbrot)"),
        valid.replace("<[bool]:1,2>", "<[f64]:1,2>"),
        valid.replace("io/argument-u64(1)", "io/argument-u64(0)"),
        valid.replace(
            "grid/map-until(size, size, pixels)",
            "image/pbm(grid/map-until(size, size, pixels))",
        ),
        valid.replace(
            "io/write(image/pbm(image))",
            "io/write(image/pbm(image/pbm(image)))",
        ),
        valid.replace("io/write(image/pbm(image))", ""),
        format!("{valid}io/write(image/pbm(image))\n"),
    ] {
        assert!(compile_grid(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn scheduling_regions_and_square_seeds_preserve_complete_bitmap() {
    let (_, _, original) = compile(include_str!("../../mandelbrot.mec"));
    let baseline = MapKernel::compile(&original, "done", "inside", true).unwrap();
    for (count, policy) in [(8, 0), (16, 0), (32, 0), (64, 0), (16, 1), (8, 2), (24, 3)] {
        for seeds in [false, true] {
            let (_, _, mut program) = compile(include_str!("../../mandelbrot.mec"));
            program.share_scaled_products();
            program.schedule_regions(count);
            program.decompose_arithmetic();
            if policy >= 2 {
                program.defer_simd_lanes();
            }
            match policy {
                1 => program.schedule_dependencies(),
                2 => program.schedule_bottom_up(),
                3 => program.schedule_latency(),
                4 => program.schedule_retire(),
                _ => {}
            }
            if seeds {
                program.simplify_square_seeds();
            }
            let scalar = MapKernel::compile(&program, "done", "inside", false).unwrap();
            let vector = MapKernel::compile(&program, "done", "inside", true).unwrap();
            for y in 0..64 {
                for x in (0..64).step_by(8) {
                    let named = BTreeMap::from([
                        ("width", vec![64.0]),
                        ("row", vec![y as f64]),
                        ("pixel", (x..x + 8).map(|p| p as f64).collect()),
                    ]);
                    let input = inputs(&program, &named);
                    let expected = baseline.evaluate(&input).unwrap();
                    assert_eq!(scalar.evaluate(&input).unwrap(), expected);
                    assert_eq!(vector.evaluate(&input).unwrap(), expected);
                }
            }
        }
    }
}
