use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use mech_core::MResult;
use sha2::{Digest, Sha256};

use super::{WorkspacePackage, workspace_path_string};
use crate::error::{NativeBuildErrorKind, native_build_error};

const WORKSPACE_FINGERPRINT_DOMAIN: &[u8] = b"mech.workspace-fingerprint.v2";
const WORKSPACE_FINGERPRINT_ENTRY_TAG: u8 = 0x01;

/// SHA-256 of the selected, workspace-relative native dependency inputs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceFingerprint(String);

impl WorkspaceFingerprint {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for WorkspaceFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Fingerprint exactly the lockfile and compile inputs belonging to selected
/// packages. Absolute paths and filesystem metadata never enter the digest.
pub fn fingerprint_workspace(
    root: impl AsRef<Path>,
    selected_packages: &[WorkspacePackage],
) -> MResult<WorkspaceFingerprint> {
    let root = root.as_ref();
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        invalid_input(root, format!("workspace root cannot be resolved: {error}"))
    })?;

    let mut inputs = BTreeSet::new();
    inputs.insert(PathBuf::from("Cargo.lock"));

    let mut packages = selected_packages.to_vec();
    packages.sort_by(|left, right| {
        left.package
            .cmp(&right.package)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    for pair in packages.windows(2) {
        if pair[0].package == pair[1].package {
            return Err(native_build_error(
                NativeBuildErrorKind::NativeWorkspacePackageDuplicate {
                    package: pair[0].package.clone(),
                },
                None,
            ));
        }
    }

    for package in &packages {
        collect_package_inputs(root, package, &mut inputs)?;
    }

    let mut entries = Vec::with_capacity(inputs.len());
    for relative_path in inputs {
        let portable_path = workspace_path_string(&relative_path)?;
        let file = resolve_input_file(root, &canonical_root, &relative_path)?;
        let bytes = fs::read(&file).map_err(|error| {
            invalid_input(
                &relative_path,
                format!("workspace input cannot be read: {error}"),
            )
        })?;
        entries.push((portable_path, bytes));
    }

    Ok(fingerprint_entries(entries.iter().map(
        |(path, content)| (path.as_str(), content.as_slice()),
    )))
}

fn fingerprint_entries<'a>(
    entries: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> WorkspaceFingerprint {
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    assert!(
        entries.windows(2).all(|pair| pair[0].0 != pair[1].0),
        "workspace fingerprint entries contain a duplicate path"
    );

    let mut hasher = Sha256::new();
    hasher.update(WORKSPACE_FINGERPRINT_DOMAIN);
    hasher.update(
        u64::try_from(entries.len())
            .expect("workspace fingerprint entry count must fit in u64")
            .to_le_bytes(),
    );
    for (path, content) in entries {
        hasher.update([WORKSPACE_FINGERPRINT_ENTRY_TAG]);
        hasher.update(
            u64::try_from(path.len())
                .expect("workspace fingerprint path length must fit in u64")
                .to_le_bytes(),
        );
        hasher.update(path.as_bytes());
        hasher.update(
            u64::try_from(content.len())
                .expect("workspace fingerprint content length must fit in u64")
                .to_le_bytes(),
        );
        hasher.update(content);
    }

    let mut hexadecimal = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(&mut hexadecimal, "{byte:02x}").expect("writing to String cannot fail");
    }
    WorkspaceFingerprint(hexadecimal)
}

fn collect_package_inputs(
    root: &Path,
    package: &WorkspacePackage,
    inputs: &mut BTreeSet<PathBuf>,
) -> MResult<()> {
    inputs.insert(package.manifest_relative_path());

    let source_root = package.source_relative_path();
    let mut rust_inputs = Vec::new();
    collect_rust_files(root, &source_root, &mut rust_inputs)?;

    let build_script = package.build_script_relative_path();
    if root.join(&build_script).is_file() {
        rust_inputs.push(build_script);
    }
    rust_inputs.sort();
    rust_inputs.dedup();

    for rust_input in &rust_inputs {
        inputs.insert(rust_input.clone());
    }

    for resource in &package.embedded_resources {
        inputs.insert(package.resource_relative_path(resource));
    }

    for rust_input in rust_inputs {
        let source = fs::read_to_string(root.join(&rust_input)).map_err(|error| {
            invalid_input(
                &rust_input,
                format!("Rust input cannot be read as UTF-8: {error}"),
            )
        })?;
        for include in discover_compile_time_resources(&source, &rust_input, package)? {
            inputs.insert(include);
        }
    }

    Ok(())
}

fn collect_rust_files(root: &Path, relative: &Path, files: &mut Vec<PathBuf>) -> MResult<()> {
    let directory = root.join(relative);
    if !directory.exists() {
        return Err(invalid_input(
            relative,
            "selected package has no `src` directory",
        ));
    }
    let mut entries = fs::read_dir(&directory)
        .map_err(|error| invalid_input(relative, format!("directory cannot be read: {error}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| invalid_input(relative, format!("directory entry is invalid: {error}")))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let name = entry.file_name();
        if name == ".git" || name == "target" {
            continue;
        }
        let child = relative.join(name);
        let file_type = entry
            .file_type()
            .map_err(|error| invalid_input(&child, format!("file type is unavailable: {error}")))?;
        if file_type.is_dir() {
            collect_rust_files(root, &child, files)?;
        } else if child.extension().is_some_and(|extension| extension == "rs") {
            files.push(child);
        }
    }
    Ok(())
}

fn resolve_input_file(root: &Path, canonical_root: &Path, relative: &Path) -> MResult<PathBuf> {
    let candidate = root.join(relative);
    let canonical = fs::canonicalize(&candidate).map_err(|error| {
        invalid_input(
            relative,
            format!("selected input cannot be resolved: {error}"),
        )
    })?;
    if !canonical.starts_with(canonical_root) {
        return Err(invalid_input(
            relative,
            "selected input resolves outside the workspace",
        ));
    }
    if !canonical.is_file() {
        return Err(invalid_input(relative, "selected input is not a file"));
    }
    Ok(canonical)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum IncludeExpression {
    SourceRelative(String),
    ManifestRelative(String),
}

fn discover_compile_time_resources(
    source: &str,
    source_relative_path: &Path,
    package: &WorkspacePackage,
) -> MResult<Vec<PathBuf>> {
    let mut resources = Vec::new();
    for macro_name in ["include_bytes!", "include_str!"] {
        let mut remainder = source;
        while let Some(index) = remainder.find(macro_name) {
            remainder = &remainder[index + macro_name.len()..];
            let trimmed = remainder.trim_start();
            let Some(argument) = macro_argument(trimmed) else {
                continue;
            };
            let expression = parse_include_expression(argument).ok_or_else(|| {
                invalid_input(
                    source_relative_path,
                    format!("unsupported {macro_name} expression `{}`", argument.trim()),
                )
            })?;
            let relative = match expression {
                IncludeExpression::SourceRelative(path) => {
                    let base = source_relative_path
                        .parent()
                        .unwrap_or_else(|| Path::new(""));
                    normalize_join(base, Path::new(&path))?
                }
                IncludeExpression::ManifestRelative(path) => normalize_join(
                    &package.relative_path,
                    Path::new(path.trim_start_matches('/')),
                )?,
            };
            resources.push(relative);
            remainder = &trimmed[argument.len() + 2..];
        }
    }
    resources.sort();
    resources.dedup();
    Ok(resources)
}

fn macro_argument(input: &str) -> Option<&str> {
    if !input.starts_with('(') {
        return None;
    }
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'(' => depth += 1,
            b')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&input[1..index]);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_include_expression(argument: &str) -> Option<IncludeExpression> {
    let argument = argument.trim();
    if let Some(path) = parse_rust_string(argument) {
        return Some(IncludeExpression::SourceRelative(path));
    }

    let concat = argument.strip_prefix("concat!")?.trim_start();
    let concat_argument = macro_argument(concat)?;
    let mut output = String::new();
    let mut manifest_relative = false;
    for part in split_top_level(concat_argument) {
        let part = part.trim();
        if part.starts_with("env!") && part.contains("CARGO_MANIFEST_DIR") {
            manifest_relative = true;
        } else {
            output.push_str(&parse_rust_string(part)?);
        }
    }
    manifest_relative.then_some(IncludeExpression::ManifestRelative(output))
}

fn split_top_level(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in input.bytes().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                parts.push(&input[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

fn parse_rust_string(input: &str) -> Option<String> {
    let input = input.trim();
    if let Some(raw) = input.strip_prefix('r') {
        let hashes = raw.bytes().take_while(|byte| *byte == b'#').count();
        let raw = &raw[hashes..];
        let raw = raw.strip_prefix('"')?;
        let suffix = format!("\"{}", "#".repeat(hashes));
        return raw.strip_suffix(&suffix).map(str::to_owned);
    }

    let inner = input.strip_prefix('"')?.strip_suffix('"')?;
    let mut output = String::new();
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match characters.next()? {
            '\\' => output.push('\\'),
            '"' => output.push('"'),
            other => {
                output.push('\\');
                output.push(other);
            }
        }
    }
    Some(output)
}

fn normalize_join(base: &Path, child: &Path) -> MResult<PathBuf> {
    if child.is_absolute() {
        return Err(invalid_input(child, "embedded resource path is absolute"));
    }
    let mut components = base
        .components()
        .filter_map(|component| match component {
            Component::Normal(component) => Some(component.to_os_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for component in child.components() {
        match component {
            Component::Normal(component) => {
                if component == ".git" || component == "target" {
                    return Err(invalid_input(
                        child,
                        "embedded resource enters `.git` or `target`",
                    ));
                }
                components.push(component.to_os_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if components.pop().is_none() {
                    return Err(invalid_input(
                        child,
                        "embedded resource escapes the workspace",
                    ));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_input(child, "embedded resource path is absolute"));
            }
        }
    }
    let normalized = components.into_iter().collect::<PathBuf>();
    if normalized.as_os_str().is_empty() {
        return Err(invalid_input(child, "embedded resource path is empty"));
    }
    Ok(normalized)
}

fn invalid_input(path: impl AsRef<Path>, reason: impl Into<String>) -> mech_core::MechError {
    native_build_error(
        NativeBuildErrorKind::NativeWorkspaceInputInvalid {
            path: path.as_ref().to_path_buf(),
            reason: reason.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);

    struct TestWorkspace(PathBuf);

    impl TestWorkspace {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "mech-build-fingerprint-{}-{label}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("machines/math/src")).unwrap();
            fs::write(root.join("Cargo.lock"), "lock-v1\n").unwrap();
            fs::write(
                root.join("machines/math/Cargo.toml"),
                "[package]\nname = \"mech-math\"\n",
            )
            .unwrap();
            fs::write(
                root.join("machines/math/src/lib.rs"),
                "const DATA: &[u8] = include_bytes!(\"../assets/data.bin\");\n",
            )
            .unwrap();
            fs::create_dir_all(root.join("machines/math/assets")).unwrap();
            fs::write(root.join("machines/math/assets/data.bin"), b"resource-v1").unwrap();
            Self(root)
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn math_package() -> WorkspacePackage {
        WorkspacePackage::new("mech-math", "mech_math", "machines/math").unwrap()
    }

    #[test]
    fn relocation_and_unselected_inputs_do_not_change_fingerprint() {
        let left = TestWorkspace::new("left");
        let right = TestWorkspace::new("right");
        fs::create_dir_all(left.0.join("machines/unselected/src")).unwrap();
        fs::write(
            left.0.join("machines/unselected/src/lib.rs"),
            "pub const UNRELATED: u8 = 1;",
        )
        .unwrap();

        let left_digest = fingerprint_workspace(&left.0, &[math_package()]).unwrap();
        let right_digest = fingerprint_workspace(&right.0, &[math_package()]).unwrap();
        assert_eq!(left_digest, right_digest);
        assert_eq!(
            left_digest.as_str(),
            "97409e5787610f2f7b28345fcee424f103a827da774de7a4156dc2aaf9fd49d0"
        );
        assert_eq!(left_digest.as_str().len(), 64);
        assert!(
            left_digest
                .as_str()
                .chars()
                .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
        );
    }

    #[test]
    fn framed_entries_remove_path_content_boundary_ambiguity() {
        let left = fingerprint_entries([("a.rs", b"x.rsY".as_slice())]);
        let right = fingerprint_entries([("a.rsx.rs", b"Y".as_slice())]);

        assert_ne!(left, right);
        assert_eq!(
            left.as_str(),
            "bf2029ea3efa28de4f46f57bae96f337aab9208fb991dde8ec6080c2af65dc8b"
        );
    }

    #[test]
    fn fingerprint_entries_sorts_and_frames_every_field() {
        let first = ("a.rs", b"alpha".as_slice());
        let second = ("b.rs", b"beta".as_slice());
        assert_eq!(
            fingerprint_entries([first, second]),
            fingerprint_entries([second, first])
        );

        assert_ne!(
            fingerprint_entries([("a.rs", b"same".as_slice())]),
            fingerprint_entries([("b.rs", b"same".as_slice())])
        );
        assert_ne!(
            fingerprint_entries([("a.rs", b"before".as_slice())]),
            fingerprint_entries([("a.rs", b"after".as_slice())])
        );
    }

    #[test]
    #[should_panic(expected = "workspace fingerprint entries contain a duplicate path")]
    fn fingerprint_entries_rejects_duplicate_paths() {
        let _ = fingerprint_entries([
            ("duplicate.rs", b"first".as_slice()),
            ("duplicate.rs", b"second".as_slice()),
        ]);
    }

    #[test]
    fn selected_rust_and_embedded_resource_bytes_change_fingerprint() {
        let workspace = TestWorkspace::new("changes");
        let initial = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();

        fs::write(
            workspace.0.join("machines/math/assets/data.bin"),
            b"resource-v2",
        )
        .unwrap();
        let resource_changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_ne!(initial, resource_changed);

        fs::write(
            workspace.0.join("machines/math/src/lib.rs"),
            "pub const VALUE: u8 = 2;\n",
        )
        .unwrap();
        let rust_changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_ne!(resource_changed, rust_changed);
    }

    #[test]
    fn include_expression_parser_supports_manifest_dir_concat() {
        assert_eq!(
            parse_include_expression("concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/data.bin\")"),
            Some(IncludeExpression::ManifestRelative(
                "/assets/data.bin".into()
            ))
        );
    }
}
