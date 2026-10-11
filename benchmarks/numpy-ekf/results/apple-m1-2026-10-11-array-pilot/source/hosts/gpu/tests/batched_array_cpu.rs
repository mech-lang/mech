use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    sync::Arc,
};

use mech_compute::{
    BackendRequest, ComputeBackendRegistry, ComputeDispatchDisposition, ComputeDispatchRequest,
    ComputeInitializerSet, ComputeInputUpdate, ComputeOutputSelection, ComputePlatform,
    ComputeProgram, ComputeValue, TensorLayout,
};
use mech_core::{CellSlotId, ComputePlacement};
use mech_gpu::{
    BatchedExecutionError, ComputeLowerer, CpuScalarBackendFactory, CpuSimdBackendFactory,
    FixedShapeKernel,
};
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue};

const MATRIX_SOURCE: &str = r#"
+> math

rectangular recurrence @compute
-------------------------------------------------------------------------------
input := source-input
scale := source-scale
projection := [1f32 2f32 3f32
               4f32 5f32 6f32]
~result := [0f32; 0f32; 0f32]
~matrix := [0f32 0f32
            0f32 0f32
            0f32 0f32]
product := projection ** input
candidate := result + (projection' ** product) * scale
bounded! := candidate[1] > -1000000f32 && candidate[1] < 1000000f32
result = candidate
matrix = matrix * 0.25<f32> + projection' * scale
(result, matrix)
"#;

const ELEMENTWISE_SOURCE: &str = r#"
+> math

elementwise operations @compute
-------------------------------------------------------------------------------
input := source-input
~result := [0f32; 0f32; 0f32; 0f32; 0f32; 0f32]
candidate := [math/abs(0f32 - input)
              math/sin(input)
              math/cos(input)
              math/sqrt(input * input + 1f32)
              math/ceil(input)
              math/atan2(input, 1f32 + input * input)]
in-range! := (input >= -2f32 && input <= 2f32) || input == 3f32
result = candidate
result
"#;

const COUNTER_SOURCE: &str = r#"
counter @compute
-------------------------------------------------------------------------------
input := source-input
~counter := 0f32
~pair := [0f32; 0f32]
upper-bound! := input < 10f32
lower-bound! := input > 0f32
counter = counter + input
pair = pair + [input; input * 2f32]
(counter, pair)
"#;

fn compile(
    source: &str,
    planning_inputs: BTreeMap<String, RuntimeHostInputValue>,
    input_names: &[&str],
    instances: usize,
) -> FixedShapeKernel {
    let tree = mech_syntax::parse(source).expect("test source must parse");
    let artifact = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .unwrap()
        .compile_tree_artifact_with_inputs(
            &tree,
            &planning_inputs,
            &input_names
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<BTreeSet<_>>(),
        )
        .expect("test source must compile")
        .into_artifact();
    ComputeLowerer
        .compile_batched(&artifact, instances as u32)
        .expect("generic source operations must lower")
}

fn matrix_program(instances: usize) -> FixedShapeKernel {
    compile(
        MATRIX_SOURCE,
        BTreeMap::from([
            (
                "source-input".to_owned(),
                RuntimeHostInputValue::F32Matrix {
                    rows: 3,
                    columns: 1,
                    values: vec![1.0, 10.0, 100.0],
                },
            ),
            ("source-scale".to_owned(), RuntimeHostInputValue::F32(0.5)),
        ]),
        &["input", "scale"],
        instances,
    )
}

fn counter_program(instances: usize) -> FixedShapeKernel {
    compile(
        COUNTER_SOURCE,
        BTreeMap::from([("source-input".to_owned(), RuntimeHostInputValue::F32(1.0))]),
        &["input"],
        instances,
    )
}

fn unchecked_program(program: &FixedShapeKernel) -> FixedShapeKernel {
    let compute = program.compute_program();
    let mut storage = compute.fixed_shape_storage().unwrap().clone();
    storage.constraints = Vec::new().into_boxed_slice();
    let unchecked = ComputeProgram::new(
        compute.interface().clone(),
        compute.plan().clone(),
        compute.kernel().clone(),
    )
    .with_fixed_shape_storage(storage);
    FixedShapeKernel::from_compute_program(&unchecked).unwrap()
}

fn assert_close(expected: &[f32], actual: &[f32]) {
    assert_eq!(expected.len(), actual.len());
    for (component, (&expected, &actual)) in expected.iter().zip(actual).enumerate() {
        let tolerance = 1.0e-4 + 1.0e-5 * expected.abs();
        assert!(
            expected.is_finite() && actual.is_finite() && (expected - actual).abs() <= tolerance,
            "component {component}: expected {expected}, got {actual}, tolerance {tolerance}"
        );
    }
}

fn assert_states_close(
    expected: &BTreeMap<CellSlotId, Vec<f32>>,
    actual: &BTreeMap<CellSlotId, Vec<f32>>,
) {
    assert_eq!(
        expected.keys().collect::<Vec<_>>(),
        actual.keys().collect::<Vec<_>>()
    );
    for (slot, values) in expected {
        assert_close(values, &actual[slot]);
    }
}

#[test]
fn rectangular_matrix_recurrence_matches_existing_cpu_backends() {
    for instances in [1, 17, 256] {
        let checked = matrix_program(instances);
        for program in [checked.clone(), unchecked_program(&checked)] {
            let inputs = BTreeMap::from([
                ("input".to_owned(), vec![1.0, 10.0, 100.0]),
                ("scale".to_owned(), vec![0.5]),
            ]);
            let mut scalar = program.prepare_cpu(&inputs).unwrap();
            let mut simd = program.prepare_simd_cpu(&inputs).unwrap();
            let mut array = program.prepare_array_cpu(&inputs).unwrap();
            #[cfg(feature = "jit")]
            let mut jit = program.prepare_jit_cpu(&inputs).unwrap();
            assert_eq!(scalar.state(), array.state());

            scalar.dispatch_turns(1).unwrap();
            simd.dispatch_turns(1).unwrap();
            array.dispatch_turns(1).unwrap();
            assert_states_close(scalar.state(), simd.state());
            assert_states_close(scalar.state(), array.state());
            #[cfg(feature = "jit")]
            {
                jit.dispatch_turns(1).unwrap();
                assert_states_close(scalar.state(), jit.state());
            }

            let slots = program
                .state_layout()
                .map(|(slot, size)| (size, slot))
                .collect::<BTreeMap<_, _>>();
            assert_close(&[1468.5, 1956.0, 2443.5], &array.state()[&slots[&3]][..3]);
            // Public snapshots retain each matrix's column-major order, even
            // though the executor stores corresponding components together.
            assert_close(
                &[0.5, 1.0, 1.5, 2.0, 2.5, 3.0],
                &array.state()[&slots[&6]][..6],
            );

            let updates = BTreeMap::from([
                (
                    "input".to_owned(),
                    (0..instances)
                        .flat_map(|lane| [2.0 + lane as f32, 3.0, 4.0])
                        .collect(),
                ),
                (
                    "scale".to_owned(),
                    (0..instances)
                        .map(|lane| 0.25 + lane as f32 * 0.001)
                        .collect(),
                ),
            ]);
            scalar.update_inputs(&updates).unwrap();
            simd.update_inputs(&updates).unwrap();
            array.update_inputs(&updates).unwrap();
            scalar.dispatch_turns(3).unwrap();
            simd.dispatch_turns(3).unwrap();
            array.dispatch_turns(3).unwrap();
            assert_states_close(scalar.state(), simd.state());
            assert_states_close(scalar.state(), array.state());
            #[cfg(feature = "jit")]
            {
                jit.update_inputs(&updates).unwrap();
                jit.dispatch_turns(3).unwrap();
                assert_states_close(scalar.state(), jit.state());
            }
            assert_eq!(array.attempted_turns(), 4);
            assert_eq!(array.fault_count(), 0);
        }
    }
}

#[test]
fn elementwise_math_and_boolean_predicates_match_scalar_values() {
    for instances in [1, 17, 256] {
        let program = compile(
            ELEMENTWISE_SOURCE,
            BTreeMap::from([("source-input".to_owned(), RuntimeHostInputValue::F32(0.25))]),
            &["input"],
            instances,
        );
        let values = (0..instances)
            .map(|lane| -1.5 + (lane % 7) as f32 * 0.5)
            .collect::<Vec<_>>();
        let inputs = BTreeMap::from([("input".to_owned(), values.clone())]);
        let mut scalar = program.prepare_cpu(&inputs).unwrap();
        let mut array = program.prepare_array_cpu(&inputs).unwrap();
        scalar.dispatch_turns(2).unwrap();
        array.dispatch_turns(2).unwrap();
        assert_states_close(scalar.state(), array.state());
        let slot = program.state_layout().next().unwrap().0;
        for (lane, input) in values.iter().copied().enumerate() {
            let expected = [
                input.abs(),
                input.sin(),
                input.cos(),
                (input * input + 1.0).sqrt(),
                input.ceil(),
                input.atan2(1.0 + input * input),
            ];
            assert_close(&expected, &array.state()[&slot][lane * 6..(lane + 1) * 6]);
        }
    }
}

#[test]
fn atan2_preserves_quadrants_axes_and_signed_zero() {
    let source = r#"
+> math

quadrant angles @compute
-------------------------------------------------------------------------------
y := source-y
x := source-x
~angle := 0f32
~negation := 0f32
angle = math/atan2(y, x)
negation = -y
(angle, negation)
"#;
    let pairs = [
        (1.0_f32, 1.0_f32),
        (1.0, -1.0),
        (-1.0, -1.0),
        (-1.0, 1.0),
        (0.0, 1.0),
        (-0.0, 1.0),
        (0.0, -1.0),
        (-0.0, -1.0),
        (1.0, 0.0),
        (-1.0, 0.0),
        (0.0, 0.0),
        (-0.0, 0.0),
        (0.0, -0.0),
        (-0.0, -0.0),
        (0.125, -4.0),
        (-4.0, 0.125),
        (3.0, -0.25),
    ];
    let program = compile(
        source,
        BTreeMap::from([
            ("source-y".to_owned(), RuntimeHostInputValue::F32(1.0)),
            ("source-x".to_owned(), RuntimeHostInputValue::F32(1.0)),
        ]),
        &["y", "x"],
        pairs.len(),
    );
    let inputs = BTreeMap::from([
        ("y".to_owned(), pairs.iter().map(|(y, _)| *y).collect()),
        ("x".to_owned(), pairs.iter().map(|(_, x)| *x).collect()),
    ]);
    let mut array = program.prepare_array_cpu(&inputs).unwrap();
    array.dispatch_turns(1).unwrap();
    let outputs = &program.compute_program().interface().outputs;
    let actual = &array.state()[&outputs[0].slot];
    let expected = pairs.iter().map(|(y, x)| y.atan2(*x)).collect::<Vec<_>>();
    assert_close(&expected, actual);
    for (lane, (&expected, &actual)) in expected.iter().zip(actual).enumerate() {
        if expected == 0.0 {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "atan2 signed zero at lane {lane}, inputs {:?}",
                pairs[lane]
            );
        }
    }
    for (lane, &(y, _)) in pairs.iter().enumerate() {
        assert_eq!(
            array.state()[&outputs[1].slot][lane].to_bits(),
            (-y).to_bits()
        );
    }
}

#[test]
fn two_and_four_term_contractions_keep_mixed_sign_accumulation_order() {
    let source = r#"
+> math

mixed contractions @compute
-------------------------------------------------------------------------------
short := source-short
long := source-long
~result := [0f32; 0f32]
two := short · [1f32; -1f32]
four := long · [1f32; -1f32; 2f32; -3f32]
result = [two; four]
result
"#;
    let program = compile(
        source,
        BTreeMap::from([
            (
                "source-short".to_owned(),
                RuntimeHostInputValue::F32Matrix {
                    rows: 2,
                    columns: 1,
                    values: vec![4.0, 1.0],
                },
            ),
            (
                "source-long".to_owned(),
                RuntimeHostInputValue::F32Matrix {
                    rows: 4,
                    columns: 1,
                    values: vec![2.0, 4.0, 6.0, 8.0],
                },
            ),
        ]),
        &["short", "long"],
        17,
    );
    let inputs = BTreeMap::from([
        ("short".to_owned(), vec![4.0, 1.0]),
        ("long".to_owned(), vec![2.0, 4.0, 6.0, 8.0]),
    ]);
    let mut scalar = program.prepare_cpu(&inputs).unwrap();
    let mut array = program.prepare_array_cpu(&inputs).unwrap();
    scalar.dispatch_turns(1).unwrap();
    array.dispatch_turns(1).unwrap();
    assert_states_close(scalar.state(), array.state());
    let slot = program.state_layout().next().unwrap().0;
    assert_eq!(array.state()[&slot], [3.0, -14.0].repeat(17));
}

#[test]
fn tiling_preserves_tail_lanes_and_state_updates() {
    for instances in [1, 17, 256] {
        let program = counter_program(instances);
        let inputs = BTreeMap::from([(
            "input".to_owned(),
            (0..instances).map(|lane| 1.0 + (lane % 5) as f32).collect(),
        )]);
        let mut scalar = program.prepare_cpu(&inputs).unwrap();
        scalar.dispatch_turns(5).unwrap();
        for tile in [1, 4, 17, 128, 257] {
            let mut array = program.prepare_array_cpu_with_tile(&inputs, tile).unwrap();
            array.dispatch_turns(5).unwrap();
            assert_states_close(scalar.state(), array.state());
            assert_eq!(array.attempted_turns(), 5);
        }
        assert!(program.prepare_array_cpu_with_tile(&inputs, 0).is_err());
    }
}

#[test]
fn rejected_late_instance_rolls_back_the_whole_batch_and_can_recover() {
    for instances in [17, 256] {
        let program = counter_program(instances);
        let inputs = BTreeMap::from([("input".to_owned(), vec![1.0])]);
        let mut array = program.prepare_array_cpu_with_tile(&inputs, 17).unwrap();
        array.dispatch_turns(1).unwrap();
        let published = array.state().clone();
        let mut invalid = vec![2.0; instances];
        invalid[instances - 1] = -1.0;
        array
            .update_inputs(&BTreeMap::from([("input".to_owned(), invalid)]))
            .unwrap();
        let BatchedExecutionError::Integrity(fault) = array.dispatch_turns(3).unwrap_err() else {
            panic!("expected an integrity rejection");
        };
        assert_eq!(fault.instance, (instances - 1) as u32);
        assert_eq!(fault.attempted_turn, 2);
        assert_eq!(fault.constraint_name.as_ref(), "lower-bound!");
        assert_eq!(array.state(), &published);
        assert_eq!(array.attempted_turns(), 2);
        assert_eq!(array.fault_count(), 1);
        assert_eq!(array.last_fault(), Some(&fault));

        assert!(matches!(
            array.dispatch_turns(1),
            Err(BatchedExecutionError::Integrity(_))
        ));
        assert_eq!(array.state(), &published);
        assert_eq!(array.attempted_turns(), 3);
        assert_eq!(array.fault_count(), 2);
        array
            .update_inputs(&BTreeMap::from([("input".to_owned(), vec![2.0])]))
            .unwrap();
        array.dispatch_turns(2).unwrap();
        let mut expected = program.prepare_cpu(&inputs).unwrap();
        expected.dispatch_turns(1).unwrap();
        expected
            .update_inputs(&BTreeMap::from([("input".to_owned(), vec![2.0])]))
            .unwrap();
        expected.dispatch_turns(2).unwrap();
        assert_states_close(expected.state(), array.state());
        assert_eq!(array.attempted_turns(), 5);
        assert_eq!(array.fault_count(), 2);
    }
}

#[test]
fn fault_selection_uses_instance_before_constraint_order() {
    let program = counter_program(17);
    let rules = program
        .named_integrity_constraints()
        .map(|(_, name)| name.to_owned())
        .collect::<Vec<_>>();
    assert_eq!(rules.len(), 2);
    let mut values = vec![1.0; 17];
    // The artifact defines stable constraint order. Arrange the values so
    // its second rule fails in the earlier instance, and its first rule in
    // the later instance; instruction-oriented scans must not reverse this.
    let failing_value = |rule: &str| {
        if rule == "upper-bound!" { 11.0 } else { -1.0 }
    };
    values[1] = failing_value(&rules[1]);
    values[2] = failing_value(&rules[0]);
    let inputs = BTreeMap::from([("input".to_owned(), values)]);
    let mut scalar = program.prepare_cpu(&inputs).unwrap();
    let scalar_fault = scalar.dispatch_turns(1).unwrap_err();
    for tile in [1, 4, 17, 128] {
        let mut array = program.prepare_array_cpu_with_tile(&inputs, tile).unwrap();
        assert_eq!(array.dispatch_turns(1).unwrap_err(), scalar_fault);
        let fault = array.last_fault().unwrap();
        assert_eq!(fault.instance, 1);
        assert_eq!(fault.constraint_name.as_ref(), rules[1]);
    }

    let inputs = BTreeMap::from([("input".to_owned(), vec![f32::NAN])]);
    let mut array = program.prepare_array_cpu(&inputs).unwrap();
    let BatchedExecutionError::Integrity(fault) = array.dispatch_turns(1).unwrap_err() else {
        panic!("expected an integrity rejection");
    };
    assert_eq!(fault.instance, 0);
    assert_eq!(fault.constraint_name.as_ref(), rules[0]);
}

#[test]
fn explicitly_unchecked_execution_publishes_values_that_fail_declared_rules() {
    let checked = counter_program(17);
    let inputs = BTreeMap::from([("input".to_owned(), vec![11.0])]);
    let mut rejected = checked.prepare_array_cpu(&inputs).unwrap();
    assert!(matches!(
        rejected.dispatch_turns(1),
        Err(BatchedExecutionError::Integrity(_))
    ));
    let unchecked = unchecked_program(&checked);
    let mut array = unchecked.prepare_array_cpu(&inputs).unwrap();
    array.dispatch_turns(2).unwrap();
    for (slot, elements) in unchecked.state_layout() {
        let expected = if elements == 1 {
            vec![22.0; 17]
        } else {
            [22.0, 44.0].repeat(17)
        };
        assert_eq!(array.state()[&slot], expected);
    }
    assert_eq!(array.attempted_turns(), 2);
    assert_eq!(array.fault_count(), 0);
    assert_eq!(array.last_fault(), None);
}

#[test]
fn malformed_input_updates_and_zero_turns_do_not_change_the_session() {
    let program = matrix_program(17);
    let inputs = BTreeMap::from([
        ("input".to_owned(), vec![1.0, 10.0, 100.0]),
        ("scale".to_owned(), vec![0.5]),
    ]);
    assert!(matches!(
        program.prepare_array_cpu(&BTreeMap::new()),
        Err(BatchedExecutionError::MissingInput(_))
    ));
    let mut malformed = inputs.clone();
    malformed.insert("input".to_owned(), vec![1.0, 2.0]);
    assert!(matches!(
        program.prepare_array_cpu(&malformed),
        Err(BatchedExecutionError::InputLength {
            expected_single: 3,
            expected_batch: 51,
            actual: 2,
            ..
        })
    ));
    let mut array = program.prepare_array_cpu(&inputs).unwrap();
    let initial = array.state().clone();
    assert_eq!(
        array.dispatch_turns(0),
        Err(BatchedExecutionError::ZeroTurns)
    );
    assert_eq!(array.state(), &initial);
    assert_eq!(array.attempted_turns(), 0);
    assert_eq!(array.fault_count(), 0);
    assert_eq!(array.last_fault(), None);

    // BTreeMap order visits the valid input first. A later invalid update
    // must leave that earlier input unchanged as well.
    let updates = BTreeMap::from([
        ("input".to_owned(), vec![2.0, 20.0, 200.0]),
        ("scale".to_owned(), vec![1.0, 2.0]),
    ]);
    assert!(matches!(
        array.update_inputs(&updates),
        Err(BatchedExecutionError::InputLength { .. })
    ));
    assert!(matches!(
        array.update_inputs(&BTreeMap::from([("unknown".to_owned(), vec![1.0])])),
        Err(BatchedExecutionError::MissingInput(_))
    ));
    array.dispatch_turns(1).unwrap();
    let mut scalar = program.prepare_cpu(&inputs).unwrap();
    scalar.dispatch_turns(1).unwrap();
    assert_states_close(scalar.state(), array.state());
}

#[test]
fn a_constant_final_state_needs_no_live_input_or_previous_state() {
    let source = r#"
constant publication @compute
-------------------------------------------------------------------------------
~result := [0f32; 0f32]
result = [2f32; -3f32]
result
"#;
    let program = compile(source, BTreeMap::new(), &[], 17);
    let mut array = program.prepare_array_cpu(&BTreeMap::new()).unwrap();
    array.dispatch_turns(3).unwrap();
    let slot = program.state_layout().next().unwrap().0;
    assert_eq!(array.state()[&slot], [2.0, -3.0].repeat(17));
    assert_eq!(array.attempted_turns(), 3);
}

#[test]
fn registered_simd_factory_preserves_the_compute_session_contract() {
    let instances = 17;
    let program = counter_program(instances);
    let compute = program.compute_program();
    let input = compute.interface().input_named("input").unwrap().id;
    let initializers =
        ComputeInitializerSet::new(BTreeMap::from([(input, ComputeValue::ScalarF32(1.0))]));
    let mut registry = ComputeBackendRegistry::default();
    registry
        .register(Arc::new(CpuScalarBackendFactory::new()))
        .unwrap();
    registry
        .register(Arc::new(CpuSimdBackendFactory::new()))
        .unwrap();
    let rules = program
        .named_integrity_constraints()
        .map(|(_, name)| name.to_owned())
        .collect::<Vec<_>>();
    let mut final_outputs = BTreeMap::new();
    for backend in ["cpu-scalar", "cpu-simd"] {
        let factory = registry
            .resolve(
                &BackendRequest::parse(backend).unwrap(),
                ComputePlatform::Native,
                ComputePlacement::Compute,
                compute,
            )
            .unwrap();
        let executable = factory.compile(compute).unwrap();
        let mut session = executable.create_session(&initializers).unwrap();
        let dispatch = |turns| ComputeDispatchRequest::new(NonZeroU32::new(turns).unwrap());
        let first = session.dispatch(&dispatch(1)).unwrap();
        assert_eq!(first.disposition, ComputeDispatchDisposition::Completed);
        assert_eq!(first.completed_turns, 1);
        session
            .update_inputs(&[ComputeInputUpdate {
                port: input,
                value: ComputeValue::ScalarF32(2.0),
            }])
            .unwrap();
        assert_eq!(session.dispatch(&dispatch(2)).unwrap().completed_turns, 2);
        let published = session.read_outputs(&ComputeOutputSelection::All).unwrap();
        let mut invalid = vec![2.0; instances];
        let failing_value = |rule: &str| if rule == "upper-bound!" { 11.0 } else { -1.0 };
        invalid[1] = failing_value(&rules[1]);
        invalid[2] = failing_value(&rules[0]);
        session
            .update_inputs(&[ComputeInputUpdate {
                port: input,
                value: ComputeValue::TensorF32 {
                    dimensions: vec![instances as u64].into_boxed_slice(),
                    layout: TensorLayout::RowMajor,
                    values: invalid.into(),
                },
            }])
            .unwrap();
        let rejected = session.dispatch(&dispatch(3)).unwrap();
        assert_eq!(rejected.disposition, ComputeDispatchDisposition::Rejected);
        assert_eq!(rejected.completed_turns, 0);
        assert_eq!(rejected.fault_count, 1);
        let fault = rejected.last_fault.unwrap();
        assert_eq!(fault.attempted_turn, 4);
        assert_eq!(fault.constraint.as_ref(), rules[1]);
        assert!(fault.detail.contains("instance 1"));
        assert_eq!(
            session.read_outputs(&ComputeOutputSelection::All).unwrap(),
            published
        );

        session
            .update_inputs(&[ComputeInputUpdate {
                port: input,
                value: ComputeValue::ScalarF32(3.0),
            }])
            .unwrap();
        let recovered = session.dispatch(&dispatch(1)).unwrap();
        assert_eq!(recovered.disposition, ComputeDispatchDisposition::Completed);
        assert_eq!(recovered.completed_turns, 1);
        assert_eq!(recovered.fault_count, 1);
        let outputs = session.read_outputs(&ComputeOutputSelection::All).unwrap();
        let counter = compute
            .interface()
            .outputs
            .iter()
            .find(|port| port.elements().unwrap() == 1)
            .unwrap();
        assert_eq!(
            session
                .read_outputs(&ComputeOutputSelection::Samples {
                    ports: BTreeSet::from([counter.id]),
                    instance: 16
                })
                .unwrap()
                .values[&counter.id],
            ComputeValue::ScalarF32(8.0)
        );
        assert!(
            session
                .read_outputs(&ComputeOutputSelection::Samples {
                    ports: BTreeSet::from([counter.id]),
                    instance: 17
                })
                .is_err()
        );
        final_outputs.insert(backend, outputs);
    }
    assert_eq!(final_outputs["cpu-scalar"], final_outputs["cpu-simd"]);
}

#[test]
fn checked_and_unchecked_ekf_match_after_a_hundred_changing_input_turns() {
    let fixture = include_str!("../fixtures/ekf-kernel.mec");
    let source = format!(
        "+> math\n\n{}",
        &fixture[fixture.find("EKF step @compute").unwrap()..]
    );
    let planning = BTreeMap::from([
        ("lane-dt".to_owned(), RuntimeHostInputValue::F32(0.1)),
        (
            "lane-linear-velocity".to_owned(),
            RuntimeHostInputValue::F32(1.0),
        ),
        (
            "lane-angular-velocity".to_owned(),
            RuntimeHostInputValue::F32(0.015),
        ),
        ("lane-bearing".to_owned(), RuntimeHostInputValue::F32(-0.55)),
        (
            "lane-measurement-noise".to_owned(),
            RuntimeHostInputValue::F32(0.25),
        ),
        (
            "finite-limit".to_owned(),
            RuntimeHostInputValue::F32(f32::MAX),
        ),
        (
            "covariance-symmetry-tolerance".to_owned(),
            RuntimeHostInputValue::F32(0.0001),
        ),
    ]);
    let names = [
        "dt",
        "linear-velocity",
        "angular-velocity",
        "bearing",
        "measurement-noise",
        "finite-limit",
        "covariance-symmetry-tolerance",
    ];
    for instances in [1, 17, 256] {
        let checked = compile(&source, planning.clone(), &names, instances);
        for program in [checked.clone(), unchecked_program(&checked)] {
            let inputs = BTreeMap::from([
                ("dt".to_owned(), vec![0.1]),
                (
                    "linear-velocity".to_owned(),
                    (0..instances)
                        .map(|lane| 1.0 + lane as f32 * 0.0001)
                        .collect(),
                ),
                ("angular-velocity".to_owned(), vec![0.015]),
                ("bearing".to_owned(), vec![-0.55]),
                ("measurement-noise".to_owned(), vec![0.25]),
                ("finite-limit".to_owned(), vec![f32::MAX]),
                ("covariance-symmetry-tolerance".to_owned(), vec![0.0001]),
            ]);
            let mut scalar = program.prepare_cpu(&inputs).unwrap();
            let mut simd = program.prepare_simd_cpu(&inputs).unwrap();
            let mut array = program.prepare_array_cpu(&inputs).unwrap();
            #[cfg(feature = "jit")]
            let mut jit = program.prepare_jit_cpu(&inputs).unwrap();
            for turn in 0..100 {
                let updates = BTreeMap::from([(
                    "bearing".to_owned(),
                    (0..instances)
                        .map(|lane| -0.55 + 0.012 * (turn as f32 * 0.17 + lane as f32 * 0.01).sin())
                        .collect(),
                )]);
                scalar.update_inputs(&updates).unwrap();
                simd.update_inputs(&updates).unwrap();
                array.update_inputs(&updates).unwrap();
                scalar.dispatch_turns(1).unwrap();
                simd.dispatch_turns(1).unwrap();
                array.dispatch_turns(1).unwrap();
                assert_states_close(scalar.state(), simd.state());
                assert_states_close(scalar.state(), array.state());
                #[cfg(feature = "jit")]
                {
                    jit.update_inputs(&updates).unwrap();
                    jit.dispatch_turns(1).unwrap();
                    assert_states_close(scalar.state(), jit.state());
                }
            }
            assert_eq!(array.attempted_turns(), 100);
            assert_eq!(array.fault_count(), 0);
        }
    }
}
