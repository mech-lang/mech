use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use mech_core::{MResult, MechError};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value, value};

use crate::dependency::{WORKSPACE_RESOLUTION_PATCHES, validate_exact_registry_version};
use crate::error::{NativeBuildErrorKind, NativeProjectRelocationUnsupported, native_build_error};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneratedDependencySource {
    Registry { exact_version: String },
    Workspace { workspace_relative_path: PathBuf },
}

/// One dependency in a generated native application's Cargo manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedDependency {
    pub package: String,
    pub crate_name: String,
    pub source: GeneratedDependencySource,
    pub cargo_features: Vec<String>,
}

impl GeneratedDependency {
    pub fn registry(
        package: impl Into<String>,
        crate_name: impl Into<String>,
        version: impl Into<String>,
        cargo_features: impl IntoIterator<Item = impl Into<String>>,
    ) -> MResult<Self> {
        let version = version.into();
        validate_exact_registry_version(&version)?;
        Self::new(
            package,
            crate_name,
            GeneratedDependencySource::Registry {
                exact_version: format!("={version}"),
            },
            cargo_features,
        )
    }

    pub fn workspace(
        package: impl Into<String>,
        crate_name: impl Into<String>,
        workspace_relative_path: impl AsRef<Path>,
        cargo_features: impl IntoIterator<Item = impl Into<String>>,
    ) -> MResult<Self> {
        Self::new(
            package,
            crate_name,
            GeneratedDependencySource::Workspace {
                workspace_relative_path: validate_generated_relative_path(
                    workspace_relative_path.as_ref(),
                )?,
            },
            cargo_features,
        )
    }

    fn new(
        package: impl Into<String>,
        crate_name: impl Into<String>,
        source: GeneratedDependencySource,
        cargo_features: impl IntoIterator<Item = impl Into<String>>,
    ) -> MResult<Self> {
        let package = package.into();
        let crate_name = crate_name.into();
        validate_cargo_identifier("package name", &package, true)?;
        validate_cargo_identifier("crate name", &crate_name, false)?;
        let mut cargo_features = cargo_features
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>();
        for feature in &cargo_features {
            validate_cargo_feature(feature)?;
            if matches!(feature.as_str(), "source" | "compiler" | "native-plan") {
                return dependency_invalid(format!(
                    "generated dependency `{package}` may not enable `{feature}`"
                ));
            }
        }
        cargo_features.sort();
        cargo_features.dedup();
        Ok(Self {
            package,
            crate_name,
            source,
            cargo_features,
        })
    }
}

/// Deterministic data model used to render a generated `Cargo.toml` in the
/// generation phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeProjectManifest {
    pub package_name: String,
    pub binary_name: String,
    pub dependencies: Vec<GeneratedDependency>,
}

impl NativeProjectManifest {
    pub fn new(
        package_name: impl Into<String>,
        binary_name: impl Into<String>,
        mut dependencies: Vec<GeneratedDependency>,
    ) -> MResult<Self> {
        let package_name = package_name.into();
        let binary_name = binary_name.into();
        validate_cargo_identifier("generated package name", &package_name, true)?;
        validate_project_binary_name(&binary_name)?;

        dependencies.sort_by(|left, right| {
            left.crate_name
                .cmp(&right.crate_name)
                .then_with(|| left.package.cmp(&right.package))
        });
        let mut crates = BTreeSet::new();
        let mut packages = BTreeSet::new();
        for dependency in &dependencies {
            if !crates.insert(dependency.crate_name.as_str()) {
                return dependency_invalid(format!(
                    "duplicate generated crate key `{}`",
                    dependency.crate_name
                ));
            }
            if !packages.insert(dependency.package.as_str()) {
                return dependency_invalid(format!(
                    "duplicate generated package `{}`",
                    dependency.package
                ));
            }
        }

        Ok(Self {
            package_name,
            binary_name,
            dependencies,
        })
    }
}

/// Render a generated Cargo manifest through `toml_edit` in deterministic
/// insertion order. No request-controlled value is interpolated into TOML
/// source directly.
pub fn render_native_project_manifest(
    manifest: &NativeProjectManifest,
    project_root: &Path,
    workspace_root: Option<&Path>,
) -> MResult<String> {
    let mut document = DocumentMut::new();

    let mut package = Table::new();
    package.insert("name", value(&manifest.package_name));
    package.insert("version", value("0.0.0"));
    package.insert("edition", value("2024"));
    package.insert("publish", value(false));
    document.insert("package", Item::Table(package));

    // An explicit empty workspace prevents Cargo from treating this generated
    // package as a member of the source workspace that contains it.
    document.insert("workspace", Item::Table(Table::new()));

    let mut binary = Table::new();
    binary.insert("name", value(&manifest.binary_name));
    binary.insert("path", value("src/main.rs"));
    let mut binaries = ArrayOfTables::new();
    binaries.push(binary);
    document.insert("bin", Item::ArrayOfTables(binaries));

    let mut dependencies = Table::new();
    for dependency in &manifest.dependencies {
        let mut specification = InlineTable::new();
        specification.insert("package", Value::from(dependency.package.clone()));
        match &dependency.source {
            GeneratedDependencySource::Registry { exact_version } => {
                specification.insert("version", Value::from(exact_version.clone()));
            }
            GeneratedDependencySource::Workspace {
                workspace_relative_path,
            } => {
                let workspace_root = workspace_root.ok_or_else(|| {
                    native_build_error(
                        NativeBuildErrorKind::NativeProjectInvalid {
                            reason: "workspace dependency rendering requires a workspace root"
                                .into(),
                        },
                        None,
                    )
                })?;
                let dependency_path = workspace_root.join(workspace_relative_path);
                specification.insert(
                    "path",
                    Value::from(relative_cargo_path(project_root, &dependency_path)?),
                );
            }
        }
        specification.insert("default-features", Value::from(false));
        let mut features = Array::new();
        for feature in &dependency.cargo_features {
            features.push(feature.as_str());
        }
        specification.insert("features", Value::Array(features));
        specification.fmt();
        dependencies.insert(
            &dependency.crate_name,
            Item::Value(Value::InlineTable(specification)),
        );
    }
    document.insert("dependencies", Item::Table(dependencies));

    // Workspace packages depend on one another by version. Patch each selected
    // package to the same relative source used by the generated application's
    // direct dependency so Cargo never resolves a second registry copy of a
    // Mech crate (which would also make shared public types incompatible).
    let mut patches = Table::new();
    for dependency in &manifest.dependencies {
        let GeneratedDependencySource::Workspace {
            workspace_relative_path,
        } = &dependency.source
        else {
            continue;
        };
        let workspace_root = workspace_root.ok_or_else(|| {
            native_build_error(
                NativeBuildErrorKind::NativeProjectInvalid {
                    reason: "workspace dependency rendering requires a workspace root".into(),
                },
                None,
            )
        })?;
        let dependency_path = workspace_root.join(workspace_relative_path);
        let mut specification = InlineTable::new();
        specification.insert(
            "path",
            Value::from(relative_cargo_path(project_root, &dependency_path)?),
        );
        specification.fmt();
        patches.insert(
            &dependency.package,
            Item::Value(Value::InlineTable(specification)),
        );
    }
    if !patches.is_empty() {
        let workspace_root = workspace_root.expect("workspace dependencies require a root");
        for patch in WORKSPACE_RESOLUTION_PATCHES {
            let mut specification = InlineTable::new();
            specification.insert(
                "path",
                Value::from(relative_cargo_path(
                    project_root,
                    &workspace_root.join(patch.package_relative_path),
                )?),
            );
            specification.fmt();
            patches.insert(
                patch.package,
                Item::Value(Value::InlineTable(specification)),
            );
        }
    }
    if !patches.is_empty() {
        let mut crates_io = Table::new();
        crates_io.insert("crates-io", Item::Table(patches));
        document.insert("patch", Item::Table(crates_io));
    }

    let mut rendered = document.to_string();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

/// Validate the exact native-application binary-name grammar.
pub fn validate_project_binary_name(value: &str) -> MResult<()> {
    if valid_ascii_identifier(value, true) {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeBuildBinaryNameInvalid {
                value: value.to_string(),
            },
            None,
        ))
    }
}

/// Validate the exact native-application target-triple argument grammar.
pub fn validate_project_target_triple(value: &str) -> MResult<()> {
    if !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
    {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeBuildTargetInvalid {
                value: value.to_string(),
            },
            None,
        ))
    }
}

/// Validate the exact native-application qualified Rust installer-path grammar.
pub fn validate_project_installer_path(value: &str) -> MResult<()> {
    let segments = value.split("::").collect::<Vec<_>>();
    if segments.len() >= 2
        && segments
            .iter()
            .all(|segment| valid_ascii_identifier(segment, false))
    {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeBuildInstallerPathInvalid {
                value: value.to_string(),
            },
            None,
        ))
    }
}

pub fn relative_cargo_path(from_directory: &Path, to: &Path) -> MResult<String> {
    let from = absolute_normal_components(from_directory)
        .map_err(|reason| relocation_error(from_directory, to, format!("project root {reason}")))?;
    let destination = absolute_normal_components(to)
        .map_err(|reason| relocation_error(from_directory, to, format!("dependency {reason}")))?;

    if !compatible_roots(&from, &destination) {
        return Err(relocation_error(
            from_directory,
            to,
            "paths have incompatible roots or volumes",
        ));
    }

    let common = from
        .components
        .iter()
        .zip(&destination.components)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative =
        Vec::with_capacity(from.components.len() - common + destination.components.len() - common);
    relative.extend((common..from.components.len()).map(|_| "..".to_owned()));
    for component in &destination.components[common..] {
        relative.push(
            component
                .to_str()
                .ok_or_else(|| relocation_error(from_directory, to, "relative path is not UTF-8"))?
                .to_owned(),
        );
    }
    Ok(if relative.is_empty() {
        ".".to_owned()
    } else {
        relative.join("/")
    })
}

pub(crate) fn normalized_absolute_project_root(path: &Path) -> MResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                native_build_error(
                    NativeBuildErrorKind::NativeProjectInvalid {
                        reason: format!("failed to resolve the generated project root: {error}"),
                    },
                    None,
                )
            })?
            .join(path)
    };
    let mut prefix = None;
    let mut rooted = false;
    let mut components = Vec::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(value) => prefix = Some(value.as_os_str().to_os_string()),
            Component::RootDir => rooted = true,
            Component::CurDir => {}
            Component::ParentDir => {
                if components.pop().is_none() {
                    return project_invalid(format!(
                        "generated project root `{}` escapes its filesystem root",
                        path.display()
                    ));
                }
            }
            Component::Normal(value) => components.push(value.to_os_string()),
        }
    }
    if !rooted {
        return project_invalid(format!(
            "generated project root `{}` is not absolute",
            path.display()
        ));
    }
    let mut normalized = PathBuf::new();
    if let Some(prefix) = prefix {
        normalized.push(prefix);
    }
    normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR));
    normalized.extend(components);

    // Resolve the nearest existing parent so platform aliases such as macOS's
    // `/var` -> `/private/var` do not make a lexically relative Cargo path point
    // at the wrong location. Deliberately do not canonicalize the generated
    // project path itself: materialization must still reject a symlink there.
    let project_name = normalized.file_name().ok_or_else(|| {
        native_build_error(
            NativeBuildErrorKind::NativeProjectInvalid {
                reason: format!(
                    "generated project root `{}` has no final path component",
                    path.display()
                ),
            },
            None,
        )
    })?;
    let mut ancestor = normalized
        .parent()
        .expect("a named absolute path has a parent");
    let mut missing_parents = Vec::new();
    while !ancestor.try_exists().map_err(|error| {
        native_build_error(
            NativeBuildErrorKind::NativeProjectInvalid {
                reason: format!(
                    "failed to inspect generated project parent `{}`: {error}",
                    ancestor.display()
                ),
            },
            None,
        )
    })? {
        missing_parents.push(
            ancestor
                .file_name()
                .expect("a missing rooted path above the filesystem root has a name")
                .to_os_string(),
        );
        ancestor = ancestor.parent().ok_or_else(|| {
            native_build_error(
                NativeBuildErrorKind::NativeProjectInvalid {
                    reason: format!(
                        "generated project root `{}` has no existing filesystem ancestor",
                        path.display()
                    ),
                },
                None,
            )
        })?;
    }
    let mut resolved = ancestor.canonicalize().map_err(|error| {
        native_build_error(
            NativeBuildErrorKind::NativeProjectInvalid {
                reason: format!(
                    "failed to resolve generated project parent `{}`: {error}",
                    ancestor.display()
                ),
            },
            None,
        )
    })?;
    resolved.extend(missing_parents.into_iter().rev());
    resolved.push(project_name);
    Ok(resolved)
}

struct AbsoluteNormalPath {
    prefix: Option<OsString>,
    rooted: bool,
    components: Vec<OsString>,
}

fn absolute_normal_components(path: &Path) -> Result<AbsoluteNormalPath, &'static str> {
    if !path.is_absolute() {
        return Err("must be absolute");
    }
    let mut prefix = None;
    let mut rooted = false;
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(value) => prefix = Some(value.as_os_str().to_os_string()),
            Component::RootDir => rooted = true,
            Component::Normal(value) => components.push(value.to_os_string()),
            Component::CurDir | Component::ParentDir => return Err("must be normalized"),
        }
    }
    if !rooted {
        return Err("must have a rooted absolute form");
    }
    Ok(AbsoluteNormalPath {
        prefix,
        rooted,
        components,
    })
}

fn compatible_roots(left: &AbsoluteNormalPath, right: &AbsoluteNormalPath) -> bool {
    left.rooted == right.rooted
        && match (&left.prefix, &right.prefix) {
            (None, None) => true,
            (Some(left), Some(right)) => left
                .to_string_lossy()
                .eq_ignore_ascii_case(&right.to_string_lossy()),
            _ => false,
        }
}

fn relocation_error(
    project_root: &Path,
    dependency: &Path,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        NativeProjectRelocationUnsupported {
            project_root: project_root.to_path_buf(),
            dependency: dependency.to_path_buf(),
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

pub fn generated_project_root(workspace_root: &Path, plan_sha256: &str) -> MResult<PathBuf> {
    if plan_sha256.len() != 64
        || !plan_sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
    {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeProjectInvalid {
                reason: "project digest must be 64 lowercase hexadecimal characters".into(),
            },
            None,
        ));
    }
    Ok(workspace_root
        .join("target/mech-native/projects")
        .join(plan_sha256))
}

pub fn shared_cargo_target_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target/mech-native/cargo-target")
}

fn validate_generated_relative_path(path: &Path) -> MResult<PathBuf> {
    if path.as_os_str().is_empty() || path.to_str().is_none() {
        return project_invalid("generated dependency path must be non-empty UTF-8");
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(component) if component != ".git" && component != "target" => {
                normalized.push(component)
            }
            _ => {
                return project_invalid(format!(
                    "generated dependency path `{}` is not workspace-relative",
                    path.display()
                ));
            }
        }
    }
    Ok(normalized)
}

fn validate_cargo_identifier(kind: &'static str, value: &str, hyphen: bool) -> MResult<()> {
    if valid_ascii_identifier(value, hyphen) {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeIdentifierInvalid {
                kind,
                value: value.to_string(),
            },
            None,
        ))
    }
}

fn validate_cargo_feature(value: &str) -> MResult<()> {
    if !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
    {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeIdentifierInvalid {
                kind: "Cargo feature",
                value: value.to_string(),
            },
            None,
        ))
    }
}

fn valid_ascii_identifier(value: &str, hyphen: bool) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || (hyphen && character == '-')
        })
}

fn dependency_invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(native_build_error(
        NativeBuildErrorKind::NativeDependencyInvalid {
            reason: reason.into(),
        },
        None,
    ))
}

fn project_invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(native_build_error(
        NativeBuildErrorKind::NativeProjectInvalid {
            reason: reason.into(),
        },
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_identifier_grammars_are_enforced() {
        for valid in ["app", "_app", "App_2", "app-name"] {
            validate_project_binary_name(valid).unwrap();
        }
        for invalid in ["", "2app", "app.name", "app name", "app/name"] {
            let error = validate_project_binary_name(invalid).unwrap_err();
            assert_eq!(error.kind_name(), "NativeBuildBinaryNameInvalid");
        }

        for valid in ["x86_64-unknown-linux-gnu", "wasm32-wasip2", "custom.target"] {
            validate_project_target_triple(valid).unwrap();
        }
        for invalid in ["", "x86 64", "--target=x", "x/y"] {
            let error = validate_project_target_triple(invalid).unwrap_err();
            assert_eq!(error.kind_name(), "NativeBuildTargetInvalid");
        }

        validate_project_installer_path("mech_math::__mech_native::install_add_ss_f64").unwrap();
        for invalid in ["install", "::install", "crate::", "crate::bad-name"] {
            let error = validate_project_installer_path(invalid).unwrap_err();
            assert_eq!(error.kind_name(), "NativeBuildInstallerPathInvalid");
        }
    }

    #[test]
    fn generated_dependencies_are_exact_sorted_and_relative() {
        let dependency = GeneratedDependency::workspace(
            "mech-math",
            "mech_math",
            "machines/math",
            ["runtime", "add", "f64", "add"],
        )
        .unwrap();
        assert_eq!(
            dependency.source,
            GeneratedDependencySource::Workspace {
                workspace_relative_path: PathBuf::from("machines/math")
            }
        );
        assert_eq!(dependency.cargo_features, ["add", "f64", "runtime"]);

        let registry =
            GeneratedDependency::registry("mech-core", "mech_core", "0.3.5", ["f64", "program"])
                .unwrap();
        assert_eq!(
            registry.source,
            GeneratedDependencySource::Registry {
                exact_version: "=0.3.5".into()
            }
        );
    }

    #[test]
    fn forbidden_compiler_layers_cannot_enter_generated_dependencies() {
        for feature in ["source", "compiler", "native-plan"] {
            assert!(
                GeneratedDependency::registry("mech-core", "mech_core", "0.3.5", [feature],)
                    .is_err()
            );
        }
    }

    #[test]
    fn manifest_rendering_is_toml_edit_backed_and_deterministic() {
        let manifest = NativeProjectManifest::new(
            "native-app",
            "native-app",
            vec![
                GeneratedDependency::registry(
                    "mech-math",
                    "mech_math",
                    "0.3.5",
                    ["runtime", "add", "f64", "native-link"],
                )
                .unwrap(),
                GeneratedDependency::workspace(
                    "mech-core",
                    "mech_core",
                    "src/core",
                    ["f64", "program"],
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let workspace_root = std::env::temp_dir().join("mech-build-manifest-workspace");
        let project_root = workspace_root.join("target/mech-native/projects/digest");
        let first = render_native_project_manifest(&manifest, &project_root, Some(&workspace_root))
            .unwrap();
        let second =
            render_native_project_manifest(&manifest, &project_root, Some(&workspace_root))
                .unwrap();
        assert_eq!(first, second);
        first.parse::<DocumentMut>().unwrap();
        assert!(first.contains("[workspace]"));
        assert!(first.contains("version = \"=0.3.5\""));
        assert!(first.contains("path = \"../../../../src/core\""));
        assert!(first.contains("[patch.crates-io]"));
        assert!(first.contains("mech-core = { path = \"../../../../src/core\" }"));
        assert!(first.contains("default-features = false"));
        assert!(first.contains("features = [\"add\", \"f64\", \"native-link\", \"runtime\"]"));
    }

    #[test]
    fn cargo_paths_are_relative_to_the_actual_project_location() {
        let workspace = std::env::temp_dir().join("mech-build-relative-workspace");
        assert_eq!(
            relative_cargo_path(
                &workspace.join("target/mech/demo.cargo"),
                &workspace.join("src/core")
            )
            .unwrap(),
            "../../../src/core"
        );
        assert_eq!(
            relative_cargo_path(
                &workspace.join("exports/deeply/nested/demo.cargo"),
                &workspace.join("src/syntax")
            )
            .unwrap(),
            "../../../../src/syntax"
        );
        let error = relative_cargo_path(
            &workspace.join("target/../target/demo.cargo"),
            &workspace.join("src/core"),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectRelocationUnsupported");
    }

    #[cfg(windows)]
    #[test]
    fn windows_cargo_paths_require_the_same_volume() {
        assert_eq!(
            relative_cargo_path(
                Path::new(r"C:\workspace\target\mech\demo.cargo"),
                Path::new(r"C:\workspace\src\core")
            )
            .unwrap(),
            "../../../src/core"
        );
        let error = relative_cargo_path(
            Path::new(r"C:\workspace\target\mech\demo.cargo"),
            Path::new(r"D:\workspace\src\core"),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectRelocationUnsupported");
    }
}
