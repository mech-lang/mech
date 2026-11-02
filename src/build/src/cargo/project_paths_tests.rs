//! Shared filesystem/Cargo policy; no application planner or GPU dependency.
use super::tests::{MINIMAL_LOCK_SEED, minimal_project};
use super::*;

#[test]
fn all_build_entrypoints_reject_nonregular_inputs_identically() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let project = minimal_project(&root);
    let paths = crate::CargoProjectPaths::new(&root);
    let legacy = generate_project_lockfile(&project, MINIMAL_LOCK_SEED, true).unwrap_err();
    let shared =
        crate::generate_cargo_project_lockfile(&paths, MINIMAL_LOCK_SEED, true).unwrap_err();
    assert_eq!(legacy.kind_name(), "NativeProjectInvalid");
    assert_eq!(legacy.kind_message(), shared.kind_message());
    project.materialize().unwrap();
    fs::create_dir(paths.lockfile_path()).unwrap();
    let legacy = build_native_project(
        &project,
        "native-cargo-test",
        None,
        temporary.path(),
        false,
        true,
    )
    .unwrap_err();
    let shared = crate::build_cargo_project(
        &paths,
        "native-cargo-test",
        None,
        temporary.path(),
        false,
        true,
    )
    .unwrap_err();
    assert_eq!(legacy.kind_name(), "NativeProjectInvalid");
    assert_eq!(legacy.kind_message(), shared.kind_message());
    assert!(paths.lockfile_path().is_dir());
    let linked = build_linked_native_host(
        &paths,
        "native-cargo-test",
        None,
        temporary.path(),
        true,
        true,
    )
    .unwrap_err();
    assert_eq!(shared.kind_name(), linked.kind_name());
    assert_eq!(shared.kind_message(), linked.kind_message());
}

#[test]
fn invalid_seed_is_rejected_before_either_entrypoint_replaces_the_previous_lock() {
    let temporary = tempfile::tempdir().unwrap();
    let project = minimal_project(&temporary.path().join("project"));
    project.materialize().unwrap();
    let paths = CargoProjectPaths::new(&project.root);
    fs::write(paths.lockfile_path(), b"previous lock").unwrap();
    for error in [
        generate_project_lockfile(&project, b"not valid [toml", true).unwrap_err(),
        generate_cargo_project_lockfile(&paths, b"not valid [toml", true).unwrap_err(),
    ] {
        assert_eq!(error.kind_name(), "NativeCargoFailed");
        assert_eq!(fs::read(paths.lockfile_path()).unwrap(), b"previous lock");
    }
}

#[cfg(unix)]
#[test]
fn shared_helpers_reject_manifest_and_lock_symlinks_before_any_cargo_process() {
    use std::os::unix::fs::symlink;
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir(&root).unwrap();
    let paths = CargoProjectPaths::new(&root);
    let outside = temporary.path().join("outside");
    fs::write(&outside, b"outside").unwrap();
    symlink(&outside, paths.manifest_path()).unwrap();
    let error = generate_cargo_project_lockfile(&paths, MINIMAL_LOCK_SEED, true).unwrap_err();
    assert_eq!(error.kind_name(), "NativeProjectInvalid");
    assert!(!paths.lockfile_path().exists());
    fs::remove_file(paths.manifest_path()).unwrap();
    fs::write(paths.manifest_path(), b"regular manifest").unwrap();
    symlink(&outside, paths.lockfile_path()).unwrap();
    assert_eq!(
        generate_cargo_project_lockfile(&paths, MINIMAL_LOCK_SEED, true)
            .unwrap_err()
            .kind_name(),
        "NativeProjectInvalid"
    );
    assert_eq!(
        build_cargo_project(
            &paths,
            "native-cargo-test",
            None,
            temporary.path(),
            false,
            true
        )
        .unwrap_err()
        .kind_name(),
        "NativeProjectInvalid"
    );
    assert_eq!(fs::read(outside).unwrap(), b"outside");
    assert_eq!(
        build_linked_native_host(
            &paths,
            "native-cargo-test",
            None,
            temporary.path(),
            true,
            true,
        )
        .unwrap_err()
        .kind_name(),
        "NativeProjectInvalid"
    );
}

#[test]
fn shared_lock_resolution_keeps_regular_manifest_then_seed_then_writer_order() {
    use std::cell::Cell;
    let temporary = tempfile::tempdir().unwrap();
    let paths = CargoProjectPaths::new(temporary.path());
    let writes = Cell::new(0);
    let stop = || {
        writes.set(writes.get() + 1);
        Err(cargo_error("stop before Cargo"))
    };
    assert_eq!(
        generate_project_lockfile_with(&paths, MINIMAL_LOCK_SEED, true, stop)
            .unwrap_err()
            .kind_name(),
        "NativeProjectInvalid"
    );
    assert_eq!(writes.get(), 0);
    fs::write(paths.manifest_path(), b"regular manifest").unwrap();
    assert_eq!(
        generate_project_lockfile_with(&paths, b"not valid [toml", true, stop)
            .unwrap_err()
            .kind_name(),
        "NativeCargoFailed"
    );
    assert_eq!(writes.get(), 0);
    let error = generate_project_lockfile_with(&paths, MINIMAL_LOCK_SEED, true, stop).unwrap_err();
    assert!(error.kind_message().contains("stop before Cargo"));
    assert_eq!(writes.get(), 1);
}

#[test]
fn path_view_keeps_isolated_current_host_locked_cargo_arguments() {
    let temporary = tempfile::tempdir().unwrap();
    let project = minimal_project(&temporary.path().join("project"));
    project.materialize().unwrap();
    let paths = CargoProjectPaths::new(&project.root);
    let isolated = isolated_cargo_working_directory(&paths.manifest_path()).unwrap();
    assert_eq!(
        isolated,
        paths
            .manifest_path()
            .canonicalize()
            .unwrap()
            .ancestors()
            .last()
            .unwrap()
    );
    let manifest = paths.manifest_path();
    let target = temporary.path().join("cargo-target");
    let invocation =
        CargoInvocation::build(&manifest, "native-cargo-test", None, &target, true, true)
            .unwrap()
            .with_current_dir(&isolated);
    let args = invocation.arguments().collect::<Vec<_>>();
    assert_eq!(args[0], OsStr::new("+nightly-2026-03-03"));
    assert_eq!(
        args.iter()
            .filter(|arg| **arg == OsStr::new("--locked"))
            .count(),
        1
    );
    assert!(
        !args.contains(&OsStr::new("--target")),
        "current host, not an inferred cross target"
    );
    assert!(args.contains(&OsStr::new("--release")));
    assert!(args.contains(&OsStr::new("--offline")));
    assert_eq!(
        invocation.command().get_current_dir(),
        Some(isolated.as_path())
    );
}

#[test]
fn shared_path_build_and_legacy_wrapper_select_the_same_reported_executable() {
    let temporary = tempfile::tempdir().unwrap();
    let project = minimal_project(&temporary.path().join("project"));
    project.materialize().unwrap();
    let paths = CargoProjectPaths::new(&project.root);
    let target = temporary.path().join("independent-target");
    generate_cargo_project_lockfile(&paths, MINIMAL_LOCK_SEED, true).unwrap();
    let lock = fs::read(paths.lockfile_path()).unwrap();
    generate_project_lockfile(&project, MINIMAL_LOCK_SEED, true).unwrap();
    assert_eq!(fs::read(paths.lockfile_path()).unwrap(), lock);
    let shared =
        build_cargo_project(&paths, "native-cargo-test", None, &target, false, true).unwrap();
    let legacy =
        build_native_project(&project, "native-cargo-test", None, &target, false, true).unwrap();
    assert_eq!(shared, legacy);
    assert!(shared.executable.is_file());
    assert!(shared.executable.starts_with(&target));
    assert_ne!(shared.executable, target.join("guessed-path"));
    assert_eq!(fs::read(paths.lockfile_path()).unwrap(), lock);
}

#[test]
fn linked_release_host_uses_regular_frozen_inputs_and_reported_executable() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("linked-project");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        concat!(
            "[package]\nname = \"linked-host-test\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
            "[workspace]\n[profile.release]\nlto = \"thin\"\nopt-level = 3\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        "fn main() { println!(\"linked-host-ok\"); }\n",
    )
    .unwrap();
    let paths = CargoProjectPaths::new(&root);
    generate_cargo_project_lockfile(&paths, MINIMAL_LOCK_SEED, true).unwrap();
    let lock = fs::read(paths.lockfile_path()).unwrap();
    let target = temporary.path().join("linked-cargo-target");
    let artifact =
        build_linked_native_host(&paths, "linked-host-test", None, &target, true, true).unwrap();
    assert!(artifact.executable.starts_with(&target));
    let run = std::process::Command::new(&artifact.executable)
        .output()
        .unwrap();
    assert!(run.status.success());
    assert_eq!(run.stdout, b"linked-host-ok\n");
    assert!(run.stderr.is_empty());
    assert_eq!(fs::read(paths.lockfile_path()).unwrap(), lock);
}

#[test]
fn materialized_path_only_project_needs_no_resident_plan_or_source_fields() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("independent-project");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        concat!(
            "[package]\nname = \"path-only-generated\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
            "[[bin]]\nname = \"path-only-generated\"\npath = \"src/main.rs\"\n[workspace]\n",
        ),
    )
    .unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    let paths = CargoProjectPaths::new(&root);
    generate_cargo_project_lockfile(&paths, MINIMAL_LOCK_SEED, true).unwrap();
    let target = temporary.path().join("isolated-cargo-target");
    let artifact =
        build_cargo_project(&paths, "path-only-generated", None, &target, false, true).unwrap();
    assert!(artifact.executable.is_file());
    assert!(artifact.executable.starts_with(&target));
    for absent in [
        "build-plan.json",
        "program.mecb",
        "src/catalog.rs",
        "src/runtime.rs",
    ] {
        assert!(
            !root.join(absent).exists(),
            "shared Cargo did not invent {absent}"
        );
    }
}
