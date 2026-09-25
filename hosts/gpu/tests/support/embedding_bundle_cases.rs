use super::{Backend, Error, Kernel};
use mech_gpu::BatchedExecutionError;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const COUNTER: &str = r#"
Counter @compute
-------------------------------------------------------------------------------
increment := 1f32
~state := 0f32
~turns := 0f32
candidate := state + increment
bounded! := candidate >= 0f32 && candidate <= 100f32
state = candidate
turns = turns + 1f32
(state, turns)
"#;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "mech-bundle-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        // The unique directory was created by this test, never supplied by a caller.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn counter(backend: Backend) -> Kernel {
    Kernel::from_source(COUNTER)
        .input("increment", [1.0, 2.0, 3.0, 4.0])
        .export("state")
        .export("turns")
        .compile(backend)
        .unwrap()
}

fn load_trusted(path: &Path) -> Result<Kernel, Error> {
    // SAFETY: Tests load only their own generated libraries. Tampered metadata
    // and mismatched library cases must be rejected before calling native code.
    unsafe { Kernel::load_bundle(path) }
}

fn assert_bundle_error(path: &Path) {
    assert!(matches!(load_trusted(path), Err(Error::Bundle(_))));
}

#[test]
fn saved_aot_bundles_retain_named_state_checks_and_independent_lifetimes() {
    for backend in [Backend::Aot, Backend::AotSimd] {
        let directory = TestDirectory::new();
        let bundle = directory.join("counter.bundle");
        let original = counter(backend);
        let mut original_session = original.start().unwrap();
        original_session.advance().unwrap();
        original.save_bundle(&bundle).unwrap();
        let loaded = load_trusted(&bundle).unwrap();
        assert_eq!(loaded.instances(), 4);
        assert!(
            loaded
                .library_path()
                .unwrap()
                .starts_with(fs::canonicalize(&bundle).unwrap())
        );
        let mut first = loaded.start().unwrap();
        let mut second = loaded.start().unwrap();
        drop(loaded);
        drop(original);
        assert_eq!(first.state("state").unwrap(), &[0.0; 4]);
        assert_eq!(first.state_width("state").unwrap(), 1);
        assert!(matches!(
            first.state("candidate"),
            Err(Error::UnknownState(_))
        ));
        first.advance().unwrap();
        assert_eq!(first.state("state").unwrap(), &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(first.state("turns").unwrap(), &[1.0; 4]);
        assert_eq!(second.state("state").unwrap(), &[0.0; 4]);

        assert!(matches!(
            first.turn([("increment", [1.0, 1.0, 1.0, 200.0])]),
            Err(Error::Execution(BatchedExecutionError::Integrity(_))),
        ));
        assert_eq!(first.state("state").unwrap(), &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(first.state("turns").unwrap(), &[1.0; 4]);
        assert!(
            first.advance().is_err(),
            "rejected measurement remains bound"
        );
        first.turn([("increment", [2.0])]).unwrap();
        assert_eq!(first.state("state").unwrap(), &[3.0, 4.0, 5.0, 6.0]);
        assert_eq!(first.state("turns").unwrap(), &[2.0; 4]);
        second.advance().unwrap();
        assert_eq!(second.state("state").unwrap(), &[1.0, 2.0, 3.0, 4.0]);
    }
}

#[test]
fn loaded_bundle_rejects_malformed_packets_without_changing_bound_inputs() {
    for backend in [Backend::Aot, Backend::AotSimd] {
        let directory = TestDirectory::new();
        let bundle = directory.join("counter.bundle");
        counter(backend).save_bundle(&bundle).unwrap();
        let loaded = load_trusted(&bundle).unwrap();
        let mut session = loaded.start().unwrap();
        for packet in [
            vec![("increment", vec![3.0; 3])],
            vec![("increment", vec![9.0]), ("unknown", vec![9.0])],
            vec![("increment", vec![9.0]), ("increment", vec![10.0])],
        ] {
            assert!(session.turn(packet).is_err());
            assert_eq!(session.state("state").unwrap(), &[0.0; 4]);
            assert_eq!(session.state("turns").unwrap(), &[0.0; 4]);
        }
        session.advance().unwrap();
        assert_eq!(session.state("state").unwrap(), &[1.0, 2.0, 3.0, 4.0]);
    }
}

#[test]
fn saved_ekf_bundle_matches_compiled_state_and_covariance() {
    for backend in [Backend::Aot, Backend::AotSimd] {
        let directory = TestDirectory::new();
        let bundle = directory.join("ekf.bundle");
        let kernel = Kernel::from_source(include_str!("../../../../examples/embedded_ekf/ekf.mec"))
            .input("bearing", [-0.55; 8])
            .export("state")
            .export("covariance")
            .compile(backend)
            .unwrap();
        kernel.save_bundle(&bundle).unwrap();
        let mut reference = kernel.start().unwrap();
        let mut loaded = load_trusted(&bundle).unwrap().start().unwrap();
        assert_eq!(loaded.state_width("state").unwrap(), 3);
        assert_eq!(loaded.state_width("covariance").unwrap(), 9);
        for turn in 0..6 {
            let bearings = (0..8)
                .map(|lane| -0.55 + turn as f32 * 0.001 + lane as f32 * 0.0002)
                .collect::<Vec<_>>();
            reference.turn([("bearing", &bearings)]).unwrap();
            loaded.turn([("bearing", &bearings)]).unwrap();
            for name in ["state", "covariance"] {
                assert_eq!(loaded.state(name).unwrap(), reference.state(name).unwrap());
            }
        }
        assert!(matches!(
            loaded.turn([("bearing", [f32::NAN])]),
            Err(Error::Execution(BatchedExecutionError::Integrity(_))),
        ));
        for name in ["state", "covariance"] {
            assert_eq!(loaded.state(name).unwrap(), reference.state(name).unwrap());
        }
    }
}

#[test]
fn saved_bundle_can_be_relocated_and_resaved() {
    let directory = TestDirectory::new();
    let original = directory.join("original.bundle");
    let relocated = directory.join("relocated.bundle");
    let resaved = directory.join("resaved.bundle");
    counter(Backend::AotSimd).save_bundle(&original).unwrap();
    fs::rename(&original, &relocated).unwrap();
    let loaded = load_trusted(&relocated).unwrap();
    loaded.save_bundle(&resaved).unwrap();
    drop(loaded);
    let mut session = load_trusted(&resaved).unwrap().start().unwrap();
    session.advance().unwrap();
    assert_eq!(session.state("state").unwrap(), &[1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn bundle_save_rejects_evaluators_jit_and_existing_destinations() {
    let directory = TestDirectory::new();
    for backend in [Backend::Scalar, Backend::Simd, Backend::Jit] {
        let destination = directory.join(&format!("{backend:?}.bundle"));
        let kernel = counter(backend);
        assert!(kernel.library_path().is_none());
        assert!(matches!(
            kernel.save_bundle(&destination),
            Err(Error::Bundle(_))
        ));
        assert!(!destination.exists());
    }
    let destination = directory.join("existing.bundle");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("keep.txt"), b"existing caller data").unwrap();
    assert!(counter(Backend::Aot).save_bundle(&destination).is_err());
    assert_eq!(
        fs::read(destination.join("keep.txt")).unwrap(),
        b"existing caller data"
    );
}

#[test]
fn bundle_load_rejects_missing_malformed_and_modified_manifests() {
    let directory = TestDirectory::new();
    let bundle = directory.join("counter.bundle");
    assert_bundle_error(&bundle);
    counter(Backend::Aot).save_bundle(&bundle).unwrap();
    let manifest_path = bundle.join("manifest.json");
    let original = fs::read(&manifest_path).unwrap();
    fs::write(&manifest_path, b"not json").unwrap();
    assert_bundle_error(&bundle);
    let manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    for (field, value) in [
        ("instances", serde_json::json!(u32::MAX)),
        ("library_file", serde_json::json!("../outside.dylib")),
        ("backend", serde_json::json!("unrecognized")),
    ] {
        let mut modified = manifest.clone();
        modified["metadata"][field] = value;
        fs::write(&manifest_path, serde_json::to_vec(&modified).unwrap()).unwrap();
        assert_bundle_error(&bundle);
    }
    let mut modified = manifest;
    modified["format_version"] = serde_json::json!(999);
    fs::write(&manifest_path, serde_json::to_vec(&modified).unwrap()).unwrap();
    assert_bundle_error(&bundle);
}

#[test]
fn bundle_load_rejects_a_different_native_library_before_execution() {
    let directory = TestDirectory::new();
    let scalar = directory.join("scalar.bundle");
    let simd = directory.join("simd.bundle");
    counter(Backend::Aot).save_bundle(&scalar).unwrap();
    counter(Backend::AotSimd).save_bundle(&simd).unwrap();
    let scalar_path = load_trusted(&scalar)
        .unwrap()
        .library_path()
        .unwrap()
        .to_owned();
    let simd_path = load_trusted(&simd)
        .unwrap()
        .library_path()
        .unwrap()
        .to_owned();
    fs::copy(simd_path, &scalar_path).unwrap();
    assert_bundle_error(&scalar);
    fs::remove_file(&scalar_path).unwrap();
    assert_bundle_error(&scalar);
}

#[test]
fn saved_bundle_loads_in_a_fresh_process_without_source_or_toolchain() {
    let directory = TestDirectory::new();
    let bundle = directory.join("counter.bundle");
    counter(Backend::AotSimd).save_bundle(&bundle).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "bundles::saved_bundle_without_toolchain_child",
            "--nocapture",
        ])
        .env("MECH_EMBEDDING_TEST_BUNDLE", &bundle)
        .env("PATH", directory.join("missing-toolchain"))
        .env("CC", directory.join("missing-cc"))
        .env("MECH_AOT_CACHE_DIR", directory.join("missing-cache"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("loaded saved kernel without toolchain")
    );
}

#[test]
fn saved_bundle_without_toolchain_child() {
    let Some(path) = std::env::var_os("MECH_EMBEDDING_TEST_BUNDLE") else {
        return;
    };
    let kernel = load_trusted(Path::new(&path)).unwrap();
    let mut session = kernel.start().unwrap();
    drop(kernel);
    session.advance().unwrap();
    assert_eq!(session.state("state").unwrap(), &[1.0, 2.0, 3.0, 4.0]);
    println!("loaded saved kernel without toolchain");
}
