//! Reusable source-to-application Cranelift build path, shared with `mech build`.
//! Arithmetic and traversal execute in emitted objects. Application shells only
//! initialize generic library services, link those objects, and perform IO.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationKind {
    Sequence,
    Grid,
    Control,
}

/// Emit native source objects and the generic application project/link inputs.
/// The workspace supplies the exact library implementation used for admission.
pub fn emit_application(
    source: &Path,
    directory: &Path,
    workspace: &Path,
    name: &str,
    simd: bool,
    loop_fma: bool,
) -> Result<ApplicationKind, String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("invalid native application name".into());
    }
    let source = source.canonicalize().map_err(|e| e.to_string())?;
    let text = fs::read_to_string(&source).map_err(|e| e.to_string())?;
    let library_features = if cfg!(feature = "system-gmp") {
        "[\"sequence-cli\",\"system-gmp\"]"
    } else {
        "[\"sequence-cli\"]"
    };
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    fs::write(directory.join("source.mec"), &text).map_err(|e| e.to_string())?;
    // Admission is semantic in both frontends. A failed collection compilation
    // does not execute anything; the grid path then reports both diagnostics.
    match crate::sequence::compile(&text, 4096) {
        Ok(_) => {
            crate::sequence::emit_program_with_array_simd(
                &text, directory, 4096, true, false, true, true, simd,
            )?;
            fs::create_dir_all(directory.join("src")).map_err(|e| e.to_string())?;
            fs::write(
                directory.join("src/main.rs"),
                include_str!("sequence/application.rs.in"),
            )
            .map_err(|e| e.to_string())?;
            fs::write(directory.join("build.rs"), "fn main() { let root = std::env::var(\"CARGO_MANIFEST_DIR\").unwrap(); println!(\"cargo:rustc-link-arg={root}/mech-sequence.o\"); println!(\"cargo:rerun-if-changed=mech-sequence.o\"); println!(\"cargo:rerun-if-changed=constants.json\"); }\n").map_err(|e| e.to_string())?;
            let quoted = |path: &Path| {
                serde_json::to_string(&path.display().to_string()).map_err(|e| e.to_string())
            };
            let mut manifest = format!(
                "[package]\nname = {}\nversion = \"0.0.0\"\nedition = \"2024\"\n[dependencies]\nmech-gpu = {{ path = {}, default-features = false, features = {library_features} }}\nserde_json = \"1\"\n[patch.crates-io]\n",
                serde_json::to_string(name).unwrap(),
                quoted(&workspace.join("hosts/gpu"))?
            );
            for (component, path) in [
                ("core", "src/core"),
                ("engine", "src/engine"),
                ("syntax", "src/syntax"),
                ("stdlib", "src/stdlib"),
            ] {
                manifest.push_str(&format!(
                    "mech-{component} = {{ path = {} }}\n",
                    quoted(&workspace.join(path))?
                ));
            }
            manifest.push_str("[workspace]\n");
            fs::write(directory.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;
            Ok(ApplicationKind::Sequence)
        }
        Err(sequence_error) => {
            if mech_engine::native_sequence::control::compile(
                &text,
                mech_stdlib::native_control_catalog(),
            )
            .is_ok()
            {
                crate::control::emit(&text, directory, simd)?;
                fs::create_dir_all(directory.join("src")).map_err(|e| e.to_string())?;
                fs::write(
                    directory.join("src/main.rs"),
                    include_str!("control/application.rs.in"),
                )
                .map_err(|e| e.to_string())?;
                fs::write(directory.join("build.rs"),"fn main(){let root=std::env::var(\"CARGO_MANIFEST_DIR\").unwrap();println!(\"cargo:rustc-link-arg={root}/mech-control.o\");println!(\"cargo:rerun-if-changed=mech-control.o\");println!(\"cargo:rerun-if-changed=control.json\");}").map_err(|e|e.to_string())?;
                let quoted = |p: &Path| serde_json::to_string(&p.display().to_string()).unwrap();
                let mut manifest = format!(
                    "[package]\nname={}\nversion=\"0.0.0\"\nedition=\"2024\"\n[dependencies]\nmech-gpu={{path={},default-features=false,features={library_features}}}\n[patch.crates-io]\n",
                    serde_json::to_string(name).unwrap(),
                    quoted(&workspace.join("hosts/gpu"))
                );
                for (component, path) in [
                    ("core", "src/core"),
                    ("engine", "src/engine"),
                    ("syntax", "src/syntax"),
                    ("stdlib", "src/stdlib"),
                ] {
                    manifest.push_str(&format!(
                        "mech-{component}={{path={}}}\n",
                        quoted(&workspace.join(path))
                    ));
                }
                manifest.push_str("[workspace]\n");
                fs::write(directory.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;
                return Ok(ApplicationKind::Control);
            }
            crate::fused_f64::map_runner::emit_source_program(
                &source,
                &directory.join("grid.c"),
                &directory.join("grid.o"),
                simd,
                loop_fma,
            )
            .map_err(|e| format!("sequence admission: {sequence_error}; control admission: {:?}; grid admission: {e}", mech_engine::native_sequence::control::compile(&text,mech_stdlib::native_control_catalog()).err()))?;
            Ok(ApplicationKind::Grid)
        }
    }
}
fn command(mut command: Command) -> Result<(), String> {
    let rendered = format!("{command:?}");
    let status = command.status().map_err(|e| format!("{rendered}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{rendered}: {status}"))
    }
}

fn cargo_application_build(
    directory: &Path,
    name: &str,
    release: bool,
    offline: bool,
) -> Result<(Command, PathBuf), String> {
    // The emitted object uses this host ISA. An explicit Cargo target also
    // overrides both CARGO_BUILD_TARGET and a configured build.target.
    let host = cranelift_native::builder()
        .map_err(str::to_owned)?
        .triple()
        .to_string();
    let mut cargo = Command::new("cargo");
    cargo
        .args(["build", "--manifest-path"])
        .arg(directory.join("Cargo.toml"))
        .args(["--target", &host])
        .env_remove("CARGO_BUILD_TARGET");
    if release {
        cargo.arg("--release");
    }
    if offline {
        cargo.arg("--offline");
    }
    let flags = std::env::var("RUSTFLAGS").unwrap_or_default();
    cargo.env("RUSTFLAGS", format!("{flags} -C target-cpu=native"));
    // A stable workspace cache shares library builds between programs.
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| directory.join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(target)
    };
    cargo.env("CARGO_TARGET_DIR", &target);
    let executable = target
        .join(host)
        .join(if release { "release" } else { "debug" })
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    Ok((cargo, executable))
}

/// Link/build already-emitted application inputs on the current host.
/// Cross compilation requires a separate target-aware object emission path.
pub fn build_application(
    kind: ApplicationKind,
    directory: &Path,
    output: &Path,
    name: &str,
    release: bool,
    offline: bool,
) -> Result<(), String> {
    let output = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(output)
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    match kind {
        ApplicationKind::Sequence | ApplicationKind::Control => {
            let (cargo, executable) = cargo_application_build(directory, name, release, offline)?;
            command(cargo)?;
            fs::copy(executable, output).map_err(|e| e.to_string())?;
        }
        ApplicationKind::Grid => {
            let mut cc = Command::new(std::env::var_os("CC").unwrap_or_else(|| "cc".into()));
            cc.arg(if release { "-O3" } else { "-O0" })
                .arg(if cfg!(target_arch = "aarch64") {
                    "-mcpu=native"
                } else {
                    "-march=native"
                });
            cc.arg("-ffp-contract=off")
                .arg(directory.join("grid.c"))
                .arg(directory.join("grid.o"))
                .args(["-lm", "-pthread", "-o"])
                .arg(output);
            if cfg!(target_os = "macos") {
                let sdk = Command::new("xcrun")
                    .arg("--show-sdk-path")
                    .output()
                    .map_err(|e| e.to_string())?;
                if !sdk.status.success() {
                    return Err("xcrun could not locate the installed SDK".into());
                }
                cc.arg("-isysroot").arg(
                    String::from_utf8(sdk.stdout)
                        .map_err(|e| e.to_string())?
                        .trim(),
                );
            }
            command(cc)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn cargo_target_and_artifact_path_use_the_emitted_host_triple() {
        let host = cranelift_native::builder().unwrap().triple().to_string();
        for release in [false, true] {
            let (cargo, executable) =
                cargo_application_build(Path::new("generated"), "demo", release, true).unwrap();
            let arguments = cargo.get_args().collect::<Vec<_>>();
            let targets = arguments
                .windows(2)
                .filter(|pair| pair[0] == OsStr::new("--target"))
                .map(|pair| pair[1])
                .collect::<Vec<_>>();
            assert_eq!(targets, vec![OsStr::new(&host)]);
            assert!(cargo.get_envs().any(|(key, value)| {
                key == OsStr::new("CARGO_BUILD_TARGET") && value.is_none()
            }));
            let target_dir = cargo
                .get_envs()
                .find_map(|(key, value)| (key == OsStr::new("CARGO_TARGET_DIR")).then_some(value))
                .flatten()
                .unwrap();
            assert_eq!(
                executable,
                Path::new(target_dir)
                    .join(&host)
                    .join(if release { "release" } else { "debug" })
                    .join(format!("demo{}", std::env::consts::EXE_SUFFIX))
            );
            assert_eq!(arguments.contains(&OsStr::new("--release")), release);
            assert!(arguments.contains(&OsStr::new("--offline")));
        }
    }
}
