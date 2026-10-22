//! Actual build-pin rejection, including claims runtime metadata cannot prove.
use super::{bounded, hash, pin};
use mech_build::{CargoInvocation, parse_cargo_build_messages};
use mech_gpu::native_artifact::GeneratedLinkedApplication;
use serde_json::{Value, json};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

struct Restore {
    path: PathBuf,
    bytes: Vec<u8>,
    active: bool,
}
impl Restore {
    fn finish(mut self) {
        fs::write(&self.path, &self.bytes).unwrap();
        self.active = false;
    }
}
impl Drop for Restore {
    fn drop(&mut self) {
        if self.active {
            if let Err(error) = fs::write(&self.path, &self.bytes) {
                eprintln!(
                    "failed to restore generated qualification input {}: {error}",
                    self.path.display()
                );
            }
        }
    }
}
#[derive(Clone, Copy)]
enum Mutation {
    Flip(usize),
    AppendWhitespace,
    ManifestPin,
}
fn build(
    project: &GeneratedLinkedApplication,
    binary_name: &str,
    target: &Path,
    release: bool,
) -> Command {
    // Rejection and restored builds must use the same final-link policy as
    // production. Debug remains identical to the ordinary Cargo build.
    let mut command = CargoInvocation::build_linked_native_host(
        project.manifest_path(),
        binary_name,
        None,
        target,
        release,
        true,
    )
    .unwrap()
    .with_current_dir(project.root().ancestors().last().unwrap())
    .command();
    command
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_BUILD_JOBS", "1");
    command
}

pub(super) fn qualify(
    project: &GeneratedLinkedApplication,
    binary_name: &str,
    binary: &Path,
    target: &Path,
    directory: &Path,
    release: bool,
) -> Value {
    let evidence = directory.join("pin-mutations");
    fs::create_dir(&evidence).unwrap();
    let binary_pin = pin(binary);
    let mut rejections = Vec::new();
    for (label, file, mutation, diagnostic) in [
        (
            "identity-stack",
            "object-identity.bin",
            Mutation::Flip(112),
            "changed generated input object-identity.bin",
        ),
        (
            "object",
            "program.o",
            Mutation::Flip(0),
            "changed generated input program.o",
        ),
        (
            "bytecode",
            "program.mecb",
            Mutation::Flip(0),
            "changed generated input program.mecb",
        ),
        (
            "source-provenance",
            "application.mec",
            Mutation::Flip(0),
            "changed generated input application.mec",
        ),
        (
            "manifest-pin",
            "linked-manifest.json",
            Mutation::ManifestPin,
            "manifest generated-input pins changed",
        ),
        (
            "main",
            "src/main.rs",
            Mutation::Flip(0),
            "changed generated input src/main.rs",
        ),
        (
            "build-script",
            "build.rs",
            Mutation::AppendWhitespace,
            "build script changed",
        ),
        (
            "toolchain",
            "rust-toolchain.toml",
            Mutation::AppendWhitespace,
            "changed generated input rust-toolchain.toml",
        ),
        (
            "cargo-manifest",
            "Cargo.toml",
            Mutation::AppendWhitespace,
            "changed generated input Cargo.toml",
        ),
    ] {
        let path = project.root().join(file);
        let original = fs::read(&path).unwrap();
        let original_pin = hash(&original);
        let mut changed = original.clone();
        match mutation {
            Mutation::Flip(offset) => {
                changed[offset] ^= 1;
                if offset == 112 {
                    // Still nonzero: rejection must come from build provenance,
                    // not the runtime's simpler zero-stack-claim check.
                    assert_ne!(u64::from_le_bytes(changed[112..120].try_into().unwrap()), 0);
                }
            }
            Mutation::AppendWhitespace => changed.push(b'\n'),
            Mutation::ManifestPin => {
                let mut manifest: Value = serde_json::from_slice(&changed).unwrap();
                manifest["pins"]["program.o"] = json!("0".repeat(64));
                changed = serde_json::to_vec(&manifest).unwrap();
            }
        }
        let changed_pin = hash(&changed);
        let retained = evidence.join(format!("{label}.input"));
        fs::write(&retained, &changed).unwrap();
        let restore = Restore {
            path,
            bytes: original,
            active: true,
        };
        fs::write(&restore.path, &changed).unwrap();
        let result = bounded(
            &mut build(project, binary_name, target, release),
            &evidence,
            label,
            120,
        );
        restore.finish();
        assert_eq!(pin(&project.root().join(file)), original_pin);
        assert!(!result.status.success(), "{label}: modified input linked");
        assert_eq!(
            parse_cargo_build_messages(Cursor::new(&result.stdout))
                .unwrap()
                .build_finished,
            Some(false)
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(diagnostic),
            "{label}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            pin(binary),
            binary_pin,
            "{label}: failed build replaced prior binary"
        );
        rejections.push(json!({"name":label,"input":file,"original_sha256":original_pin,"mutated_input":retained,"mutated_sha256":changed_pin,"restored_sha256":pin(&project.root().join(file)),"diagnostic":diagnostic,"exit":result.status.code(),"stdout_sha256":hash(&result.stdout),"stderr_sha256":hash(&result.stderr)}));
    }
    let restored = bounded(
        &mut build(project, binary_name, target, release),
        &evidence,
        "restored-build",
        120,
    );
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let restored_messages = parse_cargo_build_messages(Cursor::new(&restored.stdout)).unwrap();
    assert_eq!(restored_messages.build_finished, Some(true));
    let restored_binary = restored_messages
        .select_binary(binary_name)
        .unwrap()
        .executable;
    assert_eq!(
        restored_binary, binary,
        "restored build selected a different executable/profile instead of the qualified binary"
    );
    assert_eq!(
        pin(binary),
        binary_pin,
        "restored build changed the qualified binary"
    );
    json!({"rejections":rejections,"rust_profile":if release {"release"} else {"debug"},"restored_executable":restored_binary,"restored_build_exit":0,"binary_sha256_before_and_after":binary_pin,"restored_build_stdout_sha256":hash(&restored.stdout),"restored_build_stderr_sha256":hash(&restored.stderr)})
}
