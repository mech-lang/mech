use super::*;
use std::fs;

#[test]
fn paths_borrow_only_the_root_and_do_not_materialize_a_project() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("not-materialized");
    let paths = crate::CargoProjectPaths::new(&root);
    assert_eq!(paths.root(), root);
    assert!(std::ptr::eq(paths.root(), root.as_path()));
    assert_eq!(paths.manifest_path(), root.join("Cargo.toml"));
    assert_eq!(paths.lockfile_path(), root.join("Cargo.lock"));
    assert!(!root.exists());
}

#[test]
fn seed_writer_keeps_the_original_descendant_checks_and_atomic_replacement() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("new/project");
    let paths = CargoProjectPaths::new(&root);
    paths.materialize_lockfile_seed(b"first seed").unwrap();
    fs::write(root.join("pinned-object"), b"untouched").unwrap();
    paths.materialize_lockfile_seed(b"second seed").unwrap();
    paths.materialize_lockfile_seed(b"second seed").unwrap();
    assert_eq!(fs::read(paths.lockfile_path()).unwrap(), b"second seed");
    assert_eq!(fs::read(root.join("pinned-object")).unwrap(), b"untouched");
    let mut names = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            std::ffi::OsString::from("Cargo.lock"),
            std::ffi::OsString::from("pinned-object")
        ]
    );
}

#[test]
fn seed_writer_rejects_nonregular_lockfile_without_replacing_it() {
    let temporary = tempfile::tempdir().unwrap();
    let paths = CargoProjectPaths::new(temporary.path());
    fs::create_dir(paths.lockfile_path()).unwrap();
    let error = paths.materialize_lockfile_seed(b"seed").unwrap_err();
    assert_eq!(error.kind_name(), "NativeProjectInvalid");
    assert!(paths.lockfile_path().is_dir());
}

#[cfg(unix)]
#[test]
fn seed_writer_rejects_symlinked_descendants_and_lockfile_without_following_them() {
    use std::os::unix::fs::symlink;
    let temporary = tempfile::tempdir().unwrap();
    let outside = temporary.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("Cargo.lock"), b"outside seed").unwrap();
    let anchor = temporary.path().join("anchor");
    fs::create_dir(&anchor).unwrap();
    let linked_root = anchor.join("project");
    symlink(&outside, &linked_root).unwrap();
    assert!(
        CargoProjectPaths::new(&linked_root)
            .materialize_lockfile_seed(b"changed")
            .is_err()
    );
    let root = temporary.path().join("real-project");
    fs::create_dir(&root).unwrap();
    let paths = CargoProjectPaths::new(&root);
    symlink(outside.join("Cargo.lock"), paths.lockfile_path()).unwrap();
    assert!(paths.materialize_lockfile_seed(b"changed").is_err());
    assert_eq!(
        fs::read(outside.join("Cargo.lock")).unwrap(),
        b"outside seed"
    );
    assert!(
        fs::symlink_metadata(paths.lockfile_path())
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
