use super::{Backend, Error, Kernel, KernelBuilder, Session, aot_simd, backends};
use mech_gpu::BatchedExecutionError;

const COUNTER: &str = r#"
Counter @compute
-------------------------------------------------------------------------------
a := 1f32
z := 2f32
~state := 0f32
~turns := 0f32
candidate := state + a + z
bounded! := candidate >= 0f32 && candidate <= 1000f32
state = candidate
turns = turns + 1f32
(state, turns)
"#;

const MATRIX: &str = r#"
Matrix accumulator @compute
-------------------------------------------------------------------------------
matrix := [1f32 2f32; 3f32 4f32]
~state := [0f32 0f32; 0f32 0f32]
state = state + matrix
state
"#;

const EKF: &str = include_str!("../../../../examples/embedded_ekf/ekf.mec");

#[test]
fn source_requires_one_compute_section_without_separate_driver_code() {
    for source in [
        COUNTER.replace("Counter @compute", "Counter"),
        format!("Driver\n------\n~driver := 0f32\ndriver = driver + 1f32\n{COUNTER}"),
        format!("{COUNTER}\nOther @compute\n----------------\nx := 1f32\nx"),
        COUNTER.replace("@compute", "@gpu"),
    ] {
        assert!(matches!(
            Kernel::from_source(source)
                .input("a", [1.0])
                .export("state")
                .compile(Backend::Scalar),
            Err(Error::Source(_)),
        ));
    }
}

fn counter_builder() -> KernelBuilder {
    Kernel::from_source(COUNTER)
        .input("a", vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0])
        .input("z", [2.0])
        .export("state")
        .export("turns")
}

fn snapshot(session: &Session) -> (Vec<f32>, Vec<f32>) {
    (
        session.state("state").unwrap().to_vec(),
        session.state("turns").unwrap().to_vec(),
    )
}

fn assert_close(expected: &[f32], actual: &[f32], label: &str) {
    assert_eq!(expected.len(), actual.len(), "{label}");
    for (index, (&expected, &actual)) in expected.iter().zip(actual).enumerate() {
        let tolerance = 1.0e-4 + expected.abs() * 1.0e-5;
        assert!(
            expected.is_finite() && actual.is_finite() && (expected - actual).abs() <= tolerance,
            "{label}[{index}]: expected {expected}, actual {actual}, tolerance {tolerance}",
        );
    }
}

#[test]
fn backends_share_named_state_and_checked_turn_semantics() {
    for backend in backends() {
        let kernel = counter_builder().compile(backend).unwrap();
        assert_eq!(kernel.instances(), 8, "{backend:?}");
        if matches!(backend, Backend::Scalar | Backend::Simd) {
            assert!(kernel.library_path().is_none());
        } else {
            assert!(kernel.library_path().unwrap().is_file());
        }
        let mut session = kernel.start().unwrap();
        assert_eq!(session.instances(), 8);
        assert_eq!(session.state_width("state").unwrap(), 1);
        assert_eq!(session.state_width("turns").unwrap(), 1);
        assert_eq!(session.state("state").unwrap(), &[0.0; 8]);
        session.advance().unwrap();
        assert_eq!(
            session.state("state").unwrap(),
            &[3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
            "{backend:?}",
        );
        session.turn([("z", [3.0])]).unwrap();
        assert_eq!(
            session.state("state").unwrap(),
            &[7.0, 9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0],
            "{backend:?}",
        );
        assert_eq!(session.state("turns").unwrap(), &[2.0; 8]);
    }
}

#[test]
fn sessions_are_independent_and_outlive_the_compiled_kernel() {
    for backend in backends() {
        let kernel = counter_builder().compile(backend).unwrap();
        let mut first = kernel.start().unwrap();
        let mut second = kernel.start().unwrap();
        drop(kernel);
        first.turn([("z", [9.0])]).unwrap();
        assert_eq!(second.state("state").unwrap(), &[0.0; 8]);
        second.advance().unwrap();
        assert_eq!(second.state("state").unwrap()[0], 3.0);
        assert_eq!(first.state("state").unwrap()[0], 10.0);
        first.advance().unwrap();
        assert_eq!(first.state("state").unwrap()[0], 20.0);
        assert_eq!(second.state("turns").unwrap(), &[1.0; 8]);
    }
}

#[test]
fn state_access_requires_an_explicit_persistent_export() {
    let kernel = Kernel::from_source(COUNTER)
        .input("a", [1.0])
        .input("z", [2.0])
        .export("state")
        .compile(Backend::Scalar)
        .unwrap();
    let session = kernel.start().unwrap();
    for name in ["turns", "candidate", "missing"] {
        assert!(matches!(session.state(name), Err(Error::UnknownState(_))));
        assert!(matches!(
            session.state_width(name),
            Err(Error::UnknownState(_))
        ));
    }
    assert!(
        counter_builder()
            .export("candidate")
            .compile(Backend::Scalar)
            .is_err(),
        "a derived expression must not be exposed as persistent state",
    );
    assert!(
        counter_builder()
            .export("missing")
            .compile(Backend::Scalar)
            .is_err(),
    );
    assert!(matches!(
        counter_builder().export("state").compile(Backend::Scalar),
        Err(Error::DuplicateExport(name)) if name == "state",
    ));
}

#[test]
fn builder_rejects_duplicate_empty_unknown_and_mismatched_inputs() {
    assert!(matches!(
        counter_builder().input("a", [2.0]).compile(Backend::Scalar),
        Err(Error::DuplicateInput(name)) if name == "a",
    ));
    assert!(matches!(
        Kernel::from_source(COUNTER)
            .input("a", Vec::<f32>::new())
            .compile(Backend::Scalar),
        Err(Error::EmptyInput(name)) if name == "a",
    ));
    assert!(
        counter_builder()
            .input("typo", [1.0])
            .compile(Backend::Scalar)
            .is_err(),
    );
    assert!(
        Kernel::from_source(COUNTER)
            .input("a", [1.0; 8])
            .input("z", [2.0; 3])
            .export("state")
            .compile(Backend::Scalar)
            .is_err(),
    );
    assert!(matches!(
        counter_builder()
            .artifact_directory("unused-scalar-artifact-directory")
            .compile(Backend::Scalar),
        Err(Error::ArtifactDirectoryRequiresAot),
    ));
}

#[test]
fn malformed_packets_do_not_modify_inputs_state_or_turn_count() {
    for backend in backends() {
        let kernel = counter_builder().compile(backend).unwrap();
        let mut session = kernel.start().unwrap();
        let mut control = kernel.start().unwrap();
        session.advance().unwrap();
        control.advance().unwrap();

        let packets = [
            vec![("a", vec![100.0]), ("z", vec![3.0; 3])],
            vec![("a", vec![100.0]), ("missing", vec![3.0])],
            vec![("a", vec![100.0]), ("a", vec![200.0])],
            vec![("a", vec![100.0]), ("z", Vec::new())],
        ];
        for packet in packets {
            let before = snapshot(&session);
            assert!(session.turn(packet).is_err(), "{backend:?}");
            assert_eq!(snapshot(&session), before);
            session.advance().unwrap();
            control.advance().unwrap();
            assert_eq!(snapshot(&session), snapshot(&control), "{backend:?}");
        }
    }
}

#[test]
fn failed_integrity_retains_all_published_state_and_keeps_valid_inputs_bound() {
    for backend in backends() {
        let kernel = counter_builder().compile(backend).unwrap();
        let mut session = kernel.start().unwrap();
        let mut control = kernel.start().unwrap();
        session.advance().unwrap();
        control.advance().unwrap();
        let before = snapshot(&session);
        let mut invalid = vec![1.0; 8];
        invalid[7] = 2000.0;
        assert!(matches!(
            session.turn([("a", invalid)]),
            Err(Error::Execution(BatchedExecutionError::Integrity(_))),
        ));
        assert_eq!(snapshot(&session), before, "{backend:?}");
        assert!(matches!(
            session.advance(),
            Err(Error::Execution(BatchedExecutionError::Integrity(_))),
        ));
        assert_eq!(snapshot(&session), before, "{backend:?}");
        session.turn([("a", [1.0])]).unwrap();
        control.turn([("a", [1.0])]).unwrap();
        assert_eq!(snapshot(&session), snapshot(&control), "{backend:?}");
    }
}

#[test]
fn matrix_inputs_and_state_use_column_major_components_per_instance() {
    let matrices = vec![
        1.0, 3.0, 2.0, 4.0, // [1 2; 3 4]
        5.0, 7.0, 6.0, 8.0, // [5 6; 7 8]
        9.0, 11.0, 10.0, 12.0, 13.0, 15.0, 14.0, 16.0,
    ];
    let broadcast = [2.0, 4.0, 3.0, 5.0];
    let expected = matrices
        .iter()
        .enumerate()
        .map(|(index, value)| value + broadcast[index % 4])
        .collect::<Vec<_>>();
    for backend in backends() {
        let kernel = Kernel::from_source(MATRIX)
            .input("matrix", matrices.clone())
            .export("state")
            .compile(backend)
            .unwrap();
        let mut session = kernel.start().unwrap();
        assert_eq!(session.instances(), 4);
        assert_eq!(session.state_width("state").unwrap(), 4);
        session.advance().unwrap();
        assert_eq!(session.state("state").unwrap(), &matrices, "{backend:?}");
        session.turn([("matrix", broadcast)]).unwrap();
        assert_eq!(session.state("state").unwrap(), &expected, "{backend:?}");
    }
}

fn ekf_kernel(backend: Backend) -> Kernel {
    Kernel::from_source(EKF)
        .input("bearing", vec![-0.55; 8])
        .export("state")
        .export("covariance")
        .compile(backend)
        .unwrap()
}

#[test]
fn real_ekf_matches_across_scalar_simd_and_aot_backends() {
    let reference_kernel = ekf_kernel(Backend::Scalar);
    let mut reference = reference_kernel.start().unwrap();
    let mut sessions = backends()
        .into_iter()
        .map(|backend| (backend, ekf_kernel(backend).start().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(reference.state_width("state").unwrap(), 3);
    assert_eq!(reference.state_width("covariance").unwrap(), 9);
    for turn in 0..6 {
        let bearings = (0..8)
            .map(|lane| -0.55 + lane as f32 * 0.001 + turn as f32 * 0.0002)
            .collect::<Vec<_>>();
        reference.turn([("bearing", &bearings)]).unwrap();
        for (backend, session) in &mut sessions {
            session.turn([("bearing", &bearings)]).unwrap();
            for name in ["state", "covariance"] {
                assert_close(
                    reference.state(name).unwrap(),
                    session.state(name).unwrap(),
                    &format!("{backend:?} turn {turn} {name}"),
                );
            }
        }
    }
}

#[test]
fn real_ekf_rejects_nan_without_publishing_any_lane_and_recovers() {
    for backend in backends() {
        let kernel = ekf_kernel(backend);
        let mut session = kernel.start().unwrap();
        let mut control = kernel.start().unwrap();
        session.advance().unwrap();
        control.advance().unwrap();
        let state = session.state("state").unwrap().to_vec();
        let covariance = session.state("covariance").unwrap().to_vec();
        let mut bearings = vec![-0.55; 8];
        bearings[7] = f32::NAN;
        assert!(matches!(
            session.turn([("bearing", bearings)]),
            Err(Error::Execution(BatchedExecutionError::Integrity(_))),
        ));
        assert_eq!(session.state("state").unwrap(), &state);
        assert_eq!(session.state("covariance").unwrap(), &covariance);
        assert!(session.advance().is_err(), "NaN input remains bound");
        assert_eq!(session.state("state").unwrap(), &state);
        assert_eq!(session.state("covariance").unwrap(), &covariance);
        session.turn([("bearing", [-0.55])]).unwrap();
        control.advance().unwrap();
        assert_eq!(
            session.state("state").unwrap(),
            control.state("state").unwrap()
        );
        assert_eq!(
            session.state("covariance").unwrap(),
            control.state("covariance").unwrap(),
            "{backend:?}",
        );
    }
}

#[test]
fn aot_simd_rejects_unsupported_instance_count() {
    let Some(backend) = aot_simd() else { return };
    assert!(matches!(
        Kernel::from_source(COUNTER)
            .input("a", [1.0; 3])
            .input("z", [2.0])
            .export("state")
            .compile(backend),
        Err(Error::Execution(BatchedExecutionError::Native(_))),
    ));
}

#[test]
fn concurrent_aot_compilation_reuses_one_complete_library() {
    use std::sync::{Arc, Barrier};
    use std::time::{SystemTime, UNIX_EPOCH};

    for backend in backends()
        .into_iter()
        .filter(|backend| !matches!(backend, Backend::Scalar | Backend::Simd))
    {
        let directory = std::env::temp_dir().join(format!(
            "mech-embedding-concurrency-{}-{}-{backend:?}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&directory).unwrap();
        let barrier = Arc::new(Barrier::new(4));
        let handles = (0..4)
            .map(|_| {
                let directory = directory.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let kernel = counter_builder()
                        .artifact_directory(directory)
                        .compile(backend)
                        .unwrap();
                    let path = kernel.library_path().unwrap().to_owned();
                    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
                    let mut session = kernel.start().unwrap();
                    session.advance().unwrap();
                    assert_eq!(session.state("state").unwrap()[0], 3.0);
                    assert_eq!(kernel.start().unwrap().state("state").unwrap(), &[0.0; 8]);
                    assert_eq!(
                        std::fs::metadata(&path).unwrap().modified().unwrap(),
                        modified
                    );
                    path
                })
            })
            .collect::<Vec<_>>();
        let paths = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        assert!(paths.iter().all(|path| path == &paths[0] && path.is_file()));
        // Only the unique directory created above is removed, after every
        // thread has dropped its sessions and loaded library handles.
        std::fs::remove_dir_all(directory).unwrap();
    }
}
