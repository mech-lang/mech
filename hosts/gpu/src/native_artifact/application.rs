//! Build-owned canonical Object application generation, separate from resident apps.
use super::{NativeCompileError, emit_object};
use mech_core::FunctionCatalog;
use mech_engine::native_artifact::lower_pure_native_artifact;
use mech_engine::{ProgramArtifact, encode_program_artifact_bytecode_v1};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum NativeApplicationError {
    Invalid(&'static str),
    Io(std::io::Error),
    Lowering(mech_engine::native_artifact::NativeArtifactLoweringError),
    Compile(NativeCompileError),
    Encoding(String),
}
impl std::fmt::Display for NativeApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native application generation: {self:?}")
    }
}
impl std::error::Error for NativeApplicationError {}
impl From<std::io::Error> for NativeApplicationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Generated inputs only; this value makes no claim of linking or execution.
pub struct GeneratedLinkedApplication {
    root: PathBuf,
    pins: BTreeMap<String, String>,
    metadata: Value,
}
impl GeneratedLinkedApplication {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("Cargo.toml")
    }
    /// SHA256 pins of all generated immutable inputs except the manifest that
    /// contains these pins. Cargo.lock is generated later by the owned builder.
    pub fn pins(&self) -> &BTreeMap<String, String> {
        &self.pins
    }
    pub fn metadata(&self) -> &Value {
        &self.metadata
    }
}
fn digest(bytes: &[u8]) -> String {
    super::identity::hex(&Sha256::digest(bytes))
}
fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization cannot fail")
}

/// Derives a real private-witness plan from `artifact` and the actual catalog,
/// emits its Object, and creates a Rust runtime-only linked application.
/// No source interpreter, resident executor, JIT or compiler runs in that app.
/// `source_provenance` is retained and pinned; it is never parsed or used as a
/// semantic substitute for the canonical artifact. Callers that claim source
/// pairing must establish that compilation separately. The destination must
/// be fresh. Generation does not run Cargo, a linker, or an application.
pub fn generate_linked_application(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    source_provenance: &[u8],
) -> Result<GeneratedLinkedApplication, NativeApplicationError> {
    if binary_name.is_empty()
        || binary_name.len() > 64
        || !binary_name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || !binary_name.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err(NativeApplicationError::Invalid("invalid binary name"));
    }
    let workspace = workspace.canonicalize()?;
    if !workspace.join("hosts/gpu/Cargo.toml").is_file()
        || !workspace.join("src/stdlib/Cargo.toml").is_file()
    {
        return Err(NativeApplicationError::Invalid(
            "workspace dependencies are absent",
        ));
    }
    if !directory.is_absolute() || directory.exists() {
        return Err(NativeApplicationError::Invalid(
            "destination must be new and absolute",
        ));
    }
    let workspace_text = workspace.to_str().ok_or(NativeApplicationError::Invalid(
        "workspace path is not Unicode",
    ))?;
    let plan =
        lower_pure_native_artifact(artifact, catalog).map_err(NativeApplicationError::Lowering)?;
    let object = emit_object(&plan).map_err(NativeApplicationError::Compile)?;
    let bytecode = encode_program_artifact_bytecode_v1(artifact)
        .map_err(|error| NativeApplicationError::Encoding(format!("{error:?}")))?;
    let main = include_str!("templates/linked_main.rs.in")
        .replace("@IDENTITY_SYMBOL@", object.identity().symbol())
        .replace("@BYTECODE_SHA256@", &digest(&bytecode));
    let cargo = format!(
        r#"[package]
name = {name}
version = "0.0.0"
edition = "2024"
publish = false
build = "build.rs"
[workspace]
[dependencies]
mech-core = {{ path = {core}, default-features = false }}
mech-engine = {{ path = {engine}, default-features = false, features = ["native-artifact"] }}
mech-gpu = {{ path = {gpu}, default-features = false, features = ["canonical-native-runtime"] }}
mech-stdlib = {{ path = {stdlib}, default-features = false, features = ["std", "native-plan", "bool", "u64", "math_add", "math_sub", "math_mul", "math_div", "math_mod", "compare_default", "logic_default"] }}
sha2 = {{ version = "0.11.0", default-features = false }}
[build-dependencies]
sha2 = {{ version = "0.11.0", default-features = false }}
serde_json = "1.0"
[profile.dev]
debug = 0
incremental = false
"#,
        name = json_string(binary_name),
        core = json_string(&format!("{workspace_text}/src/core")),
        engine = json_string(&format!("{workspace_text}/src/engine")),
        gpu = json_string(&format!("{workspace_text}/hosts/gpu")),
        stdlib = json_string(&format!("{workspace_text}/src/stdlib")),
    );
    let mut files = BTreeMap::<String, Vec<u8>>::from([
        ("application.mec".into(), source_provenance.to_vec()),
        ("program.mecb".into(), bytecode),
        ("program.o".into(), object.bytes().to_vec()),
        (
            "object-identity.bin".into(),
            object.identity().bytes().to_vec(),
        ),
        ("Cargo.toml".into(), cargo.into_bytes()),
        (
            "rust-toolchain.toml".into(),
            b"[toolchain]\nchannel = \"nightly-2026-03-03\"\nprofile = \"minimal\"\n".to_vec(),
        ),
        ("src/main.rs".into(), main.into_bytes()),
    ]);
    let mut pins = files
        .iter()
        .map(|(name, value)| (name.clone(), digest(value)))
        .collect::<BTreeMap<_, _>>();
    // The script pins the independent immutable inputs; the manifest pins the
    // script after it is rendered, avoiding a self-referential digest.
    let build = include_str!("templates/linked_build.rs.in").replace(
        "@PIN_JSON_LITERAL@",
        &json_string(&serde_json::to_string(&pins).unwrap()),
    );
    pins.insert("build.rs".into(), digest(build.as_bytes()));
    files.insert("build.rs".into(), build.into_bytes());
    let metadata = json!({
        "schema": "mech.canonical-linked-application.v1",
        "route": "canonical-artifact-cranelift-object",
        "program_revision": super::identity::hex(artifact.revision().as_bytes()),
        "typed_ir_sha256": super::identity::hex(object.descriptor().typed_ir_sha256()),
        "object": object.metadata(), "pins": pins,
        "source_is_provenance_only": true,
        "runtime_features": ["mech-gpu/canonical-native-runtime", "mech-engine/native-artifact", "mech-stdlib/native-plan"],
        "linked": false, "executed": false, "timing_samples": 0,
    });
    files.insert(
        "linked-manifest.json".into(),
        serde_json::to_vec_pretty(&metadata)
            .map_err(|error| NativeApplicationError::Encoding(error.to_string()))?,
    );
    fs::create_dir(directory)?;
    fs::create_dir(directory.join("src"))?;
    for (name, value) in files {
        fs::write(directory.join(name), value)?;
    }
    Ok(GeneratedLinkedApplication {
        root: directory.to_path_buf(),
        pins,
        metadata,
    })
}
