//! Build-owned canonical Object application generation, separate from resident apps.
use super::{NativeCompileError, emit_object};
use mech_core::FunctionCatalog;
use mech_engine::native_artifact::lower_pure_native_artifact;
use mech_engine::{ProgramArtifact, encode_program_artifact_bytecode_v1};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[path = "application_value_features.rs"]
mod value_features;

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

/// Cargo ignores a dependency workspace's patch table. Retain the actual
/// workspace's local resolution in the generated standalone workspace too.
fn workspace_resolution_patches(workspace: &Path) -> Result<String, NativeApplicationError> {
    use std::fmt::Write;
    let source = fs::read_to_string(workspace.join("Cargo.toml"))?;
    let manifest = source
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| NativeApplicationError::Encoding(error.to_string()))?;
    let Some(patches) = manifest
        .get("patch")
        .and_then(|patch| patch.get("crates-io"))
    else {
        return Ok(String::new());
    };
    let patches = patches
        .as_table_like()
        .ok_or(NativeApplicationError::Invalid(
            "workspace registry overrides must be a table",
        ))?;
    let mut result = String::from("\n[patch.crates-io]\n");
    for (package, specification) in patches.iter() {
        let table = specification
            .as_table_like()
            .ok_or(NativeApplicationError::Invalid(
                "workspace registry override must be a local path",
            ))?;
        if table.len() != 1 {
            return Err(NativeApplicationError::Invalid(
                "workspace registry override must contain only a local path",
            ));
        }
        let relative = table.get("path").and_then(toml_edit::Item::as_str).ok_or(
            NativeApplicationError::Invalid("workspace registry override has no local path"),
        )?;
        let path = workspace.join(relative).canonicalize()?;
        if !path.starts_with(workspace) || !path.join("Cargo.toml").is_file() {
            return Err(NativeApplicationError::Invalid(
                "workspace registry override is outside the pinned workspace",
            ));
        }
        let path = path.to_str().ok_or(NativeApplicationError::Invalid(
            "workspace registry override path is not Unicode",
        ))?;
        writeln!(
            &mut result,
            "{} = {{ path = {} }}",
            json_string(package),
            json_string(path)
        )
        .expect("String formatting cannot fail");
    }
    Ok(result)
}

/// The exact shared name grammar for every generated linked application.
/// This validates a name only; it creates no project or source/entry authority.
pub fn validate_linked_application_binary_name(
    binary_name: &str,
) -> Result<(), NativeApplicationError> {
    if binary_name.is_empty()
        || binary_name.len() > 64
        || !binary_name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || !binary_name.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err(NativeApplicationError::Invalid("invalid binary name"));
    }
    Ok(())
}

pub(super) fn prepare_destination(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
) -> Result<PathBuf, NativeApplicationError> {
    validate_linked_application_binary_name(binary_name)?;
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
    workspace.to_str().ok_or(NativeApplicationError::Invalid(
        "workspace path is not Unicode",
    ))?;
    Ok(workspace)
}

#[derive(Clone, Copy)]
pub(super) enum LinkedApplicationProfile {
    Scalar,
    #[cfg(feature = "canonical-native-program")]
    ManagedProgram,
    #[cfg(feature = "canonical-native-program-host")]
    HostedManagedProgram {
        benchmark_policy: bool,
    },
}

pub(super) struct LinkedApplicationInputs<'a> {
    pub profile: LinkedApplicationProfile,
    pub schemas: &'a mech_core::SchemaTable,
    pub factory_linkage: mech_engine::linked_factories::NativeFactoryLinkageProfile,
    pub source_provenance: &'a [u8],
    pub bytecode: Vec<u8>,
    pub object: &'a [u8],
    pub identity: &'a [u8],
    pub main: String,
    pub revision: &'a [u8],
    pub typed_ir_sha256: &'a [u8],
    pub object_metadata: Value,
    pub descriptor_image: Option<Vec<u8>>,
    pub resource_image: Option<Vec<u8>>,
    pub manifest_schema: &'static str,
    pub route: &'static str,
}

pub(super) fn render_main(template: &str, symbol: &str, bytecode: &[u8]) -> String {
    template
        .replace("@IDENTITY_SYMBOL@", symbol)
        .replace("@BYTECODE_SHA256@", &digest(bytecode))
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
    let workspace = prepare_destination(workspace, directory, binary_name)?;
    let plan =
        lower_pure_native_artifact(artifact, catalog).map_err(NativeApplicationError::Lowering)?;
    let object = emit_object(&plan).map_err(NativeApplicationError::Compile)?;
    let bytecode = encode_program_artifact_bytecode_v1(artifact)
        .map_err(|error| NativeApplicationError::Encoding(format!("{error:?}")))?;
    let main = render_main(
        include_str!("templates/linked_main.rs.in"),
        object.identity().symbol(),
        &bytecode,
    );
    write_linked_application(
        &workspace,
        directory,
        binary_name,
        LinkedApplicationInputs {
            profile: LinkedApplicationProfile::Scalar,
            schemas: artifact.schemas(),
            factory_linkage: mech_engine::linked_factories::NativeFactoryLinkageProfile::derive(
                artifact, catalog,
            ),
            source_provenance,
            bytecode,
            object: object.bytes(),
            identity: object.identity().bytes(),
            main,
            revision: artifact.revision().as_bytes(),
            typed_ir_sha256: object.descriptor().typed_ir_sha256(),
            object_metadata: object.metadata(),
            descriptor_image: None,
            resource_image: None,
            manifest_schema: "mech.canonical-linked-application.v1",
            route: "canonical-artifact-cranelift-object",
        },
    )
}

pub(super) fn write_linked_application(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
    inputs: LinkedApplicationInputs<'_>,
) -> Result<GeneratedLinkedApplication, NativeApplicationError> {
    let workspace_text = workspace.to_str().ok_or(NativeApplicationError::Invalid(
        "workspace path is not Unicode",
    ))?;
    let selected = inputs.factory_linkage.factories();
    let value_requirements = value_features::required(inputs.schemas);
    let value_feature_names = &value_requirements.features;
    let factory_feature_names = selected
        .into_iter()
        .flatten()
        .flat_map(|factory| factory.required_engine_features().iter().copied())
        .collect::<BTreeSet<_>>();
    let factory_profile = json!({
        "schema": "mech.canonical-linked-factory-profile.v1",
        "mode": if selected.is_some() { "artifact-selected" } else { "complete-catalog" },
        "selection_basis": inputs.factory_linkage.selection_basis(),
        "kernel_operations": inputs.factory_linkage.operations().iter()
            .map(|operation| operation.canonical_name()).collect::<Vec<_>>(),
        "factory_declarations": selected.map(|factories| factories.iter()
            .map(|factory| factory.rust_path()).collect::<Vec<_>>()),
        "required_engine_features": factory_feature_names,
        "required_core_value_features": value_feature_names,
        "selective_conversions": selected.is_some() && !value_requirements.open_types,
        "missing_linkage_operations": inputs.factory_linkage.missing_operations().iter()
            .map(|operation| operation.canonical_name()).collect::<Vec<_>>(),
    });
    // Individual static references, not a runtime dispatcher over the entire
    // catalog. Paths come ONLY from maintained private factory witnesses.
    let installation = match selected {
        Some(factories) => factories.iter().map(|factory| format!(
            r#"{}.install(&mut builder).map_err(|error| format!("{{error:?}}"))?;"#,
            factory.rust_path(),
        )).collect::<Vec<_>>().join("\n"),
        None => r#"mech_engine::install_intrinsic_resident(&mut builder).map_err(|error| format!("{error:?}"))?;"#.into(),
    };
    if !inputs.main.contains("@INSTALL_CANONICAL_FACTORIES@") {
        return Err(NativeApplicationError::Invalid(
            "linked launcher has no catalog installation site",
        ));
    }
    let main = inputs
        .main
        .replace("@INSTALL_CANONICAL_FACTORIES@", &installation);
    let factory_features = factory_feature_names
        .iter()
        .map(|feature| format!(", {}", json_string(feature)))
        .collect::<String>();
    #[cfg(feature = "canonical-native-program-host")]
    let hosted_runtime_feature = "canonical-native-resources";
    let (gpu_feature, core_features, library_features, dependencies) = match inputs.profile {
        LinkedApplicationProfile::Scalar => ("canonical-native-runtime", "", "", String::new()),
        #[cfg(feature = "canonical-native-program")]
        LinkedApplicationProfile::ManagedProgram => (
            "canonical-native-program-runtime",
            ", features = [\"serde\"]",
            ", \"string\", \"pure_text\", \"map_ops\"",
            "serde_json = \"1.0\"\n".to_owned(),
        ),
        #[cfg(feature = "canonical-native-program-host")]
        LinkedApplicationProfile::HostedManagedProgram { benchmark_policy } => (
            if benchmark_policy {
                "canonical-native-benchmark-policy-runtime"
            } else {
                "canonical-native-program-host-runtime"
            },
            ", features = [\"serde\"]",
            ", \"string\", \"pure_text\", \"map_ops\", \"access\", \"set\", \"tuple\", \"enum\", \"sequence_sort\"",
            format!(
                "serde_json = \"1.0\"\nserde = {{ version = \"1.0\", features = [\"derive\"] }}\nmech-runtime = {{ path = {}, default-features = false, features = [\"runtime\", \"string\", \"{hosted_runtime_feature}\"] }}\n",
                json_string(&format!("{workspace_text}/src/runtime")),
            ),
        ),
    };
    let mut core_feature_names = value_requirements.features;
    if selected.is_some() && !value_requirements.open_types {
        core_feature_names.insert("selective-convert");
    }
    if !core_features.is_empty() {
        core_feature_names.insert("serde");
    }
    let core_features = if core_feature_names.is_empty() {
        String::new()
    } else {
        format!(
            ", features = [{}]",
            core_feature_names
                .iter()
                .map(|feature| json_string(feature))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    // The selected declarations are owned by mech-engine. Adding mech-stdlib
    // here would unify its broad operation/type features into those owners,
    // even though this launcher never references the library's installer.
    // Preserve the historical feature bundle only for complete-catalog mode.
    let stdlib_dependency = if selected.is_none() {
        format!(
            "mech-stdlib = {{ path = {}, default-features = false, features = [\"std\", \"native-plan\", \"bool\", \"u64\", \"i64\", \"f64\", \"convert\", \"math_sqrt\", \"math_neg\", \"math_add\", \"math_sub\", \"math_mul\", \"math_div\", \"math_mod\", \"compare_default\", \"logic_default\"{library_features}] }}\n",
            json_string(&format!("{workspace_text}/src/stdlib")),
        )
    } else {
        String::new()
    };
    let mut cargo = format!(
        r#"[package]
name = {name}
version = "0.0.0"
edition = "2024"
publish = false
build = "build.rs"
[workspace]
[dependencies]
mech-core = {{ path = {core}, default-features = false{core_features} }}
mech-engine = {{ path = {engine}, default-features = false, features = ["native-artifact"{factory_features}] }}
mech-gpu = {{ path = {gpu}, default-features = false, features = ["{gpu_feature}"] }}
{stdlib_dependency}sha2 = {{ version = "0.11.0", default-features = false }}
{dependencies}[build-dependencies]
sha2 = {{ version = "0.11.0", default-features = false }}
serde_json = "1.0"
[profile.dev]
debug = 0
incremental = false
[profile.release]
# Remove symbol tables while retaining the normal speed optimization level.
strip = "symbols"
opt-level = 3
# Optimize the Rust host across crate boundaries; the checked Mech Object is
# already native machine code and is linked unchanged, not lowered through LLVM.
lto = "thin"
"#,
        name = json_string(binary_name),
        core = json_string(&format!("{workspace_text}/src/core")),
        engine = json_string(&format!("{workspace_text}/src/engine")),
        gpu = json_string(&format!("{workspace_text}/hosts/gpu")),
    );
    cargo.push_str(&workspace_resolution_patches(workspace)?);
    let mut files = BTreeMap::<String, Vec<u8>>::from([
        ("application.mec".into(), inputs.source_provenance.to_vec()),
        ("program.mecb".into(), inputs.bytecode),
        ("program.o".into(), inputs.object.to_vec()),
        ("object-identity.bin".into(), inputs.identity.to_vec()),
        ("Cargo.toml".into(), cargo.into_bytes()),
        (
            "rust-toolchain.toml".into(),
            b"[toolchain]\nchannel = \"nightly-2026-03-03\"\nprofile = \"minimal\"\n".to_vec(),
        ),
        ("src/main.rs".into(), main.into_bytes()),
        (
            "factory-profile.json".into(),
            serde_json::to_vec_pretty(&factory_profile)
                .map_err(|error| NativeApplicationError::Encoding(error.to_string()))?,
        ),
    ]);
    // Bind both inspectable and embedded copies to the SAME checked Object,
    // bytecode and source. The launcher loads only the smaller packed copy.
    if let Some(image) = inputs.descriptor_image {
        #[cfg(feature = "canonical-native-program")]
        files.insert(
            "native-descriptor.bin".into(),
            super::program_image_packing::pack(&image).map_err(NativeApplicationError::Encoding)?,
        );
        files.insert("native-descriptor.json".into(), image);
    }
    if let Some(image) = inputs.resource_image {
        files.insert("native-resources.json".into(), image);
    }
    let mut pins = files
        .iter()
        .map(|(name, value)| (name.clone(), digest(value)))
        .collect::<BTreeMap<_, _>>();
    // The script pins the independent immutable inputs; the manifest pins the
    // script after it is rendered, avoiding a self-referential digest.
    let build = include_str!("templates/linked_build.rs.in")
        .replace("@MANIFEST_SCHEMA@", inputs.manifest_schema)
        .replace(
            "@PIN_JSON_LITERAL@",
            &json_string(&serde_json::to_string(&pins).unwrap()),
        );
    pins.insert("build.rs".into(), digest(build.as_bytes()));
    files.insert("build.rs".into(), build.into_bytes());
    let mut runtime_features = vec![
        format!("mech-gpu/{gpu_feature}"),
        "mech-engine/native-artifact".to_owned(),
    ];
    if selected.is_none() {
        runtime_features.push("mech-stdlib/native-plan".to_owned());
    }
    runtime_features.extend(
        core_feature_names
            .iter()
            .map(|feature| format!("mech-core/{feature}")),
    );
    #[cfg(feature = "canonical-native-program-host")]
    let runtime_features = {
        let mut runtime_features = runtime_features;
        if matches!(
            inputs.profile,
            LinkedApplicationProfile::HostedManagedProgram { .. }
        ) {
            runtime_features.extend([
                format!("mech-runtime/{hosted_runtime_feature}"),
                "mech-runtime/canonical-native-images".to_owned(),
                "mech-terminal/provider".to_owned(),
            ]);
        }
        runtime_features
    };
    let metadata = json!({
        "schema": inputs.manifest_schema,
        "route": inputs.route,
        "program_revision": super::identity::hex(inputs.revision),
        "typed_ir_sha256": super::identity::hex(inputs.typed_ir_sha256),
        "object": inputs.object_metadata, "pins": pins,
        "source_is_provenance_only": true,
        "runtime_features": runtime_features,
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
