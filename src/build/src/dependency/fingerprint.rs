use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use mech_core::MResult;
use sha2::{Digest, Sha256};
use toml_edit::DocumentMut;

use super::{WORKSPACE_RESOLUTION_PATCHES, WorkspacePackage, workspace_path_string};
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
    inputs.extend(resolution_fingerprint_inputs().map(|(_, path)| PathBuf::from(path)));

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
        collect_package_inputs(root, &canonical_root, package, &mut inputs)?;
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

fn resolution_fingerprint_inputs() -> impl Iterator<Item = (&'static str, &'static str)> {
    WORKSPACE_RESOLUTION_PATCHES
        .iter()
        .map(|patch| (patch.package, patch.manifest_relative_path))
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
    canonical_root: &Path,
    package: &WorkspacePackage,
    inputs: &mut BTreeSet<PathBuf>,
) -> MResult<()> {
    let manifest = package.manifest_relative_path();
    inputs.insert(manifest.clone());
    validate_package_build_script(root, canonical_root, package, &manifest)?;

    let source_root = package.source_relative_path();
    let mut rust_inputs = Vec::new();
    collect_rust_files(root, &source_root, &mut rust_inputs)?;

    rust_inputs.sort();
    rust_inputs.dedup();

    for resource in &package.embedded_resources {
        inputs.insert(package.resource_relative_path(resource));
    }

    // `include!` and `#[path] mod` contribute Rust tokens, and those tokens may
    // contain further compile-time inputs. Walk both graphs transitively so
    // every explicitly redirected Rust module Cargo can compile is represented
    // in the workspace fingerprint.
    let mut pending_rust_inputs = rust_inputs;
    let mut scanned_rust_inputs = BTreeSet::new();
    while let Some(rust_input) = pending_rust_inputs.pop() {
        if !scanned_rust_inputs.insert(rust_input.clone()) {
            continue;
        }
        inputs.insert(rust_input.clone());
        let rust_file = resolve_input_file(root, canonical_root, &rust_input)?;
        let source = fs::read_to_string(rust_file).map_err(|error| {
            invalid_input(
                &rust_input,
                format!("Rust input cannot be read as UTF-8: {error}"),
            )
        })?;
        for (macro_name, include) in discover_compile_time_resources(&source, &rust_input, package)?
        {
            inputs.insert(include.clone());
            if macro_name == "include" {
                pending_rust_inputs.push(include);
            }
        }
        pending_rust_inputs.extend(discover_path_modules(&source, &rust_input)?);
    }

    Ok(())
}

/// Cargo permits package manifests to redirect both the library target and the
/// build script. The current workspace registry deliberately fingerprints the
/// complete conventional `src` tree; reject redirects rather than silently
/// hashing files Cargo will not compile or omitting files that it will.
fn validate_package_build_script(
    root: &Path,
    canonical_root: &Path,
    package: &WorkspacePackage,
    manifest: &Path,
) -> MResult<()> {
    let manifest_file = resolve_input_file(root, canonical_root, manifest)?;
    let source = fs::read_to_string(&manifest_file).map_err(|error| {
        invalid_input(
            manifest,
            format!("package manifest cannot be read as UTF-8: {error}"),
        )
    })?;
    let document = source.parse::<DocumentMut>().map_err(|error| {
        invalid_input(
            manifest,
            format!("package manifest is invalid TOML: {error}"),
        )
    })?;

    if let Some(path) = document
        .get("lib")
        .and_then(|item| item.as_table())
        .and_then(|table| table.get("path"))
    {
        let Some(path) = path.as_str() else {
            return Err(invalid_input(manifest, "package lib.path must be a string"));
        };
        if path != "src/lib.rs" {
            return Err(invalid_input(
                manifest,
                format!(
                    "custom package lib.path `{path}` is unsupported by deterministic workspace fingerprinting"
                ),
            ));
        }
    }

    let default = package.build_script_relative_path();
    let build = document
        .get("package")
        .and_then(|item| item.as_table())
        .and_then(|table| table.get("build"));
    match build {
        None if root.join(&default).is_file() => Err(invalid_input(
            manifest,
            "package build scripts are unsupported by deterministic workspace fingerprinting",
        )),
        None => Ok(()),
        Some(item) if item.as_bool() == Some(false) => Ok(()),
        Some(item) if item.as_str() == Some("build.rs") => Err(invalid_input(
            manifest,
            "package build scripts are unsupported by deterministic workspace fingerprinting",
        )),
        Some(item) if item.as_str().is_some() => {
            let path = item.as_str().expect("checked string build path");
            Err(invalid_input(
                manifest,
                format!(
                    "custom package.build path `{path}` is unsupported by deterministic workspace fingerprinting"
                ),
            ))
        }
        Some(_) => Err(invalid_input(
            manifest,
            "package.build must be `false` or omitted without a default build.rs",
        )),
    }
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
        if file_type.is_symlink() {
            return Err(invalid_input(
                &child,
                "symlinked entries are not allowed in selected package source trees",
            ));
        } else if file_type.is_dir() {
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
) -> MResult<Vec<(&'static str, PathBuf)>> {
    let mut resources = Vec::new();
    for (macro_name, argument) in active_include_macros(source) {
        let expression = parse_include_expression(argument).ok_or_else(|| {
            invalid_input(
                source_relative_path,
                format!("unsupported {macro_name}! expression `{}`", argument.trim()),
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
        resources.push((macro_name, relative));
    }
    resources.sort();
    resources.dedup();
    Ok(resources)
}

fn discover_path_modules(source: &str, source_relative_path: &Path) -> MResult<Vec<PathBuf>> {
    let base = source_relative_path
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let mut modules = active_path_attributes(source, source_relative_path)?
        .into_iter()
        .map(|path| normalize_join(base, Path::new(&path)))
        .collect::<MResult<Vec<_>>>()?;
    modules.sort();
    modules.dedup();
    Ok(modules)
}

fn active_path_attributes(source: &str, source_relative_path: &Path) -> MResult<Vec<String>> {
    let bytes = source.as_bytes();
    let mut paths = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if let Some(end) = rust_non_code_end(source, index) {
            index = end;
            continue;
        }
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }

        let mut cursor = skip_rust_trivia(source, index + 1);
        if bytes.get(cursor) == Some(&b'!') {
            cursor = skip_rust_trivia(source, cursor + 1);
        }
        if bytes.get(cursor) != Some(&b'[') {
            index += 1;
            continue;
        }
        cursor = skip_rust_trivia(source, cursor + 1);
        let identifier_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            cursor += 1;
        }
        if &source[identifier_start..cursor] != "path" {
            index += 1;
            continue;
        }
        cursor = skip_rust_trivia(source, cursor);
        if bytes.get(cursor) != Some(&b'=') {
            return Err(invalid_input(
                source_relative_path,
                "unsupported #[path] attribute; expected a string literal",
            ));
        }
        cursor = skip_rust_trivia(source, cursor + 1);
        let literal_end = rust_raw_string_end(source, cursor)
            .or_else(|| rust_quoted_literal_end(source, cursor))
            .ok_or_else(|| {
                invalid_input(
                    source_relative_path,
                    "unsupported #[path] attribute; expected a string literal",
                )
            })?;
        let path = parse_rust_string(&source[cursor..literal_end]).ok_or_else(|| {
            invalid_input(
                source_relative_path,
                "unsupported #[path] attribute; expected a string literal",
            )
        })?;
        if path.is_empty() {
            return Err(invalid_input(
                source_relative_path,
                "unsupported #[path] attribute; module path is empty",
            ));
        }
        let close = skip_rust_trivia(source, literal_end);
        if bytes.get(close) != Some(&b']') {
            return Err(invalid_input(
                source_relative_path,
                "unsupported #[path] attribute; expected closing bracket",
            ));
        }
        paths.push(path);
        index = close + 1;
    }
    Ok(paths)
}

fn active_include_macros(source: &str) -> Vec<(&'static str, &str)> {
    let bytes = source.as_bytes();
    let mut includes = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if let Some(end) = rust_non_code_end(source, index) {
            index = end;
            continue;
        }
        if !bytes[index].is_ascii_alphabetic() && bytes[index] != b'_' {
            index += 1;
            continue;
        }

        let identifier_start = index;
        index += 1;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }
        let macro_name = match &source[identifier_start..index] {
            "include" => "include",
            "include_bytes" => "include_bytes",
            "include_str" => "include_str",
            _ => continue,
        };
        let mut cursor = skip_rust_trivia(source, index);
        if bytes.get(cursor) != Some(&b'!') {
            continue;
        }
        cursor = skip_rust_trivia(source, cursor + 1);
        let Some(argument) = macro_argument(&source[cursor..]) else {
            continue;
        };
        includes.push((macro_name, argument));
        index = cursor + argument.len() + 2;
    }
    includes
}

fn skip_rust_trivia(source: &str, mut index: usize) -> usize {
    let bytes = source.as_bytes();
    loop {
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        let Some(end) = rust_comment_end(source, index) else {
            return index;
        };
        index = end;
    }
}

fn rust_non_code_end(source: &str, index: usize) -> Option<usize> {
    rust_comment_end(source, index)
        .or_else(|| rust_raw_string_end(source, index))
        .or_else(|| rust_quoted_literal_end(source, index))
}

fn rust_comment_end(source: &str, index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(index..index + 2) == Some(b"//") {
        return Some(
            source[index + 2..]
                .find('\n')
                .map_or(bytes.len(), |offset| index + 2 + offset + 1),
        );
    }
    if bytes.get(index..index + 2) != Some(b"/*") {
        return None;
    }
    let mut cursor = index + 2;
    let mut depth = 1usize;
    while cursor < bytes.len() {
        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            depth += 1;
            cursor += 2;
        } else if bytes.get(cursor..cursor + 2) == Some(b"*/") {
            depth -= 1;
            cursor += 2;
            if depth == 0 {
                return Some(cursor);
            }
        } else {
            cursor += 1;
        }
    }
    Some(bytes.len())
}

fn rust_raw_string_end(source: &str, index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let prefix_length = if bytes.get(index) == Some(&b'r') {
        1
    } else if matches!(bytes.get(index..index + 2), Some(b"br" | b"cr")) {
        2
    } else {
        return None;
    };
    let mut cursor = index + prefix_length;
    let hash_start = cursor;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    let hash_count = cursor - hash_start;
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    cursor += 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes.get(cursor + 1..cursor + 1 + hash_count)
                == Some(&bytes[hash_start..hash_start + hash_count])
        {
            return Some(cursor + 1 + hash_count);
        }
        cursor += 1;
    }
    Some(bytes.len())
}

fn rust_quoted_literal_end(source: &str, index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let (quote_index, quote) = match bytes.get(index..index + 2) {
        Some([b'b' | b'c', b'"']) => (index + 1, b'"'),
        Some([b'b', b'\'']) => (index + 1, b'\''),
        _ => match bytes.get(index).copied()? {
            b'"' => (index, b'"'),
            b'\''
                if bytes.get(index + 2) == Some(&b'\'') || bytes.get(index + 1) == Some(&b'\\') =>
            {
                (index, b'\'')
            }
            _ => return None,
        },
    };
    let mut cursor = quote_index + 1;
    let mut escaped = false;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        cursor += 1;
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == quote {
            return Some(cursor);
        }
    }
    Some(bytes.len())
}

fn macro_argument(input: &str) -> Option<&str> {
    let (open, close) = match input.as_bytes().first().copied()? {
        b'(' => (b'(', b')'),
        b'[' => (b'[', b']'),
        b'{' => (b'{', b'}'),
        _ => return None,
    };
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        if index > 0
            && let Some(end) = rust_non_code_end(input, index)
        {
            index = end;
            continue;
        }
        match bytes[index] {
            byte if byte == open => depth += 1,
            byte if byte == close => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&input[1..index]);
                }
            }
            _ => {}
        }
        index += 1;
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
            fs::create_dir_all(root.join("src/syntax/src")).unwrap();
            fs::write(
                root.join("src/syntax/Cargo.toml"),
                "[package]\nname = \"mech-syntax\"\n",
            )
            .unwrap();
            fs::write(
                root.join("src/syntax/src/lib.rs"),
                "pub const SYNTAX: u8 = 1;\n",
            )
            .unwrap();
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
            drop(fs::remove_dir_all(&self.0));
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
            "0f1ec3e5750c3e55613134a92c3686032a3d7cbcb51658de16f617ebbec0368b"
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
    fn resolution_patch_manifest_but_not_source_changes_the_fingerprint() {
        let workspace = TestWorkspace::new("resolution-patch");
        let initial = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();

        fs::write(
            workspace.0.join("src/syntax/src/lib.rs"),
            "pub const SYNTAX: u8 = 2;\n",
        )
        .unwrap();
        let source_changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_eq!(initial, source_changed);

        fs::write(
            workspace.0.join("src/syntax/Cargo.toml"),
            "[package]\nname = \"mech-syntax\"\nversion = \"0.3.5\"\n",
        )
        .unwrap();
        let manifest_changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_ne!(initial, manifest_changed);
    }

    #[test]
    fn missing_resolution_patch_manifest_is_invalid() {
        let workspace = TestWorkspace::new("missing-resolution-patch");
        fs::remove_file(workspace.0.join("src/syntax/Cargo.toml")).unwrap();
        let error = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap_err();
        assert_eq!(error.kind_name(), "NativeWorkspaceInputInvalid");
        assert!(error.kind_message().contains("src/syntax/Cargo.toml"));

        fs::create_dir(workspace.0.join("src/syntax/Cargo.toml")).unwrap();
        let error = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap_err();
        assert_eq!(error.kind_name(), "NativeWorkspaceInputInvalid");
        assert!(error.kind_message().contains("not a file"));
    }

    #[test]
    fn resolution_patch_packages_exactly_match_resolution_fingerprint_packages() {
        let declared = WORKSPACE_RESOLUTION_PATCHES
            .iter()
            .map(|patch| patch.package)
            .collect::<BTreeSet<_>>();
        let fingerprinted = resolution_fingerprint_inputs()
            .map(|(package, _)| package)
            .collect::<BTreeSet<_>>();
        assert_eq!(declared, fingerprinted);
        assert_eq!(declared, BTreeSet::from(["mech-syntax"]));
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
        drop(fingerprint_entries([
            ("duplicate.rs", b"first".as_slice()),
            ("duplicate.rs", b"second".as_slice()),
        ]));
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

    #[test]
    fn include_macro_argument_supports_every_rust_delimiter() {
        for input in [
            "(\"assets/data.bin\")",
            "[\"assets/data.bin\"]",
            "{\"assets/data.bin\"}",
        ] {
            assert_eq!(macro_argument(input), Some("\"assets/data.bin\""));
        }
        assert_eq!(
            macro_argument("{concat![\"a\", \"b\"]}"),
            Some("concat![\"a\", \"b\"]")
        );
    }

    #[test]
    fn bracket_and_brace_includes_change_the_workspace_fingerprint() {
        for (label, source) in [
            (
                "bracket-include",
                "const DATA: &[u8] = include_bytes![\"../assets/data.bin\"];\n",
            ),
            (
                "brace-include",
                "const DATA: &str = include_str!{\"../assets/data.bin\"};\n",
            ),
        ] {
            let workspace = TestWorkspace::new(label);
            fs::write(workspace.0.join("machines/math/src/lib.rs"), source).unwrap();
            let initial = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
            fs::write(
                workspace.0.join("machines/math/assets/data.bin"),
                b"delimiter-resource-v2",
            )
            .unwrap();
            let changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
            assert_ne!(
                initial, changed,
                "{label} resource bytes must be fingerprinted"
            );
        }
    }

    #[test]
    fn included_rust_sources_change_the_workspace_fingerprint() {
        let workspace = TestWorkspace::new("rust-source-include");
        fs::write(
            workspace.0.join("machines/math/src/lib.rs"),
            "include!(\"../generated/catalog.rs\");\n",
        )
        .unwrap();
        fs::create_dir_all(workspace.0.join("machines/math/generated")).unwrap();
        fs::write(
            workspace.0.join("machines/math/generated/catalog.rs"),
            "pub const GENERATED: u8 = 1;\n",
        )
        .unwrap();

        let initial = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        fs::write(
            workspace.0.join("machines/math/generated/catalog.rs"),
            "pub const GENERATED: u8 = 2;\n",
        )
        .unwrap();
        let changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_ne!(initial, changed, "included Rust must be fingerprinted");
    }

    #[test]
    fn path_modules_and_nested_path_modules_change_the_workspace_fingerprint() {
        let workspace = TestWorkspace::new("path-module");
        fs::write(
            workspace.0.join("machines/math/src/lib.rs"),
            "#[path = \"../generated/module.rs\"]\nmod generated;\n",
        )
        .unwrap();
        fs::create_dir_all(workspace.0.join("machines/math/generated/nested")).unwrap();
        fs::write(
            workspace.0.join("machines/math/generated/module.rs"),
            "#[path = \"nested/child.rs\"]\nmod child;\npub const GENERATED: u8 = 1;\n",
        )
        .unwrap();
        fs::write(
            workspace.0.join("machines/math/generated/nested/child.rs"),
            "pub const CHILD: u8 = 1;\n",
        )
        .unwrap();

        let initial = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        fs::write(
            workspace.0.join("machines/math/generated/module.rs"),
            "#[path = \"nested/child.rs\"]\nmod child;\npub const GENERATED: u8 = 2;\n",
        )
        .unwrap();
        let module_changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_ne!(
            initial, module_changed,
            "#[path] module must be fingerprinted"
        );

        fs::write(
            workspace.0.join("machines/math/generated/nested/child.rs"),
            "pub const CHILD: u8 = 2;\n",
        )
        .unwrap();
        let child_changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_ne!(
            module_changed, child_changed,
            "nested #[path] module must be fingerprinted"
        );
    }

    #[test]
    fn path_attribute_text_inside_comments_and_literals_is_ignored() {
        let workspace = TestWorkspace::new("inactive-path-module");
        fs::write(
            workspace.0.join("machines/math/src/lib.rs"),
            r###"
// #[path = "../generated/missing-line.rs"]
/* #[path = "../generated/missing-block.rs"] */
const EXAMPLE: &str = "#[path = \"../generated/missing-string.rs\"]";
const RAW: &str = r#"#[path = "../generated/missing-raw.rs"]"#;
const DATA: &[u8] = include_bytes!("../assets/data.bin");
"###,
        )
        .unwrap();

        fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
    }

    #[test]
    fn custom_cargo_library_and_build_script_paths_are_rejected() {
        let custom_library = TestWorkspace::new("custom-library-path");
        fs::create_dir_all(custom_library.0.join("machines/math/generated")).unwrap();
        fs::write(
            custom_library.0.join("machines/math/generated/lib.rs"),
            "pub const GENERATED: u8 = 1;\n",
        )
        .unwrap();
        fs::write(
            custom_library.0.join("machines/math/Cargo.toml"),
            "[package]\nname = \"mech-math\"\n[lib]\npath = \"generated/lib.rs\"\n",
        )
        .unwrap();

        let error = fingerprint_workspace(&custom_library.0, &[math_package()]).unwrap_err();
        assert_eq!(error.kind_name(), "NativeWorkspaceInputInvalid");
        assert!(error.kind_message().contains("custom package lib.path"));

        let custom_build = TestWorkspace::new("custom-build-path");
        fs::create_dir_all(custom_build.0.join("machines/math/tools")).unwrap();
        fs::write(
            custom_build.0.join("machines/math/tools/build.rs"),
            "fn main() {}\n",
        )
        .unwrap();
        fs::write(
            custom_build.0.join("machines/math/Cargo.toml"),
            "[package]\nname = \"mech-math\"\nbuild = \"tools/build.rs\"\n",
        )
        .unwrap();

        let error = fingerprint_workspace(&custom_build.0, &[math_package()]).unwrap_err();
        assert_eq!(error.kind_name(), "NativeWorkspaceInputInvalid");
        assert!(error.kind_message().contains("custom package.build path"));
    }

    #[test]
    fn disabled_default_build_script_is_not_fingerprinted() {
        let workspace = TestWorkspace::new("disabled-build-script");
        fs::write(
            workspace.0.join("machines/math/Cargo.toml"),
            "[package]\nname = \"mech-math\"\nbuild = false\n",
        )
        .unwrap();
        fs::write(
            workspace.0.join("machines/math/build.rs"),
            "fn main() { println!(\"cargo:rerun-if-changed=first\"); }\n",
        )
        .unwrap();
        let initial = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();

        fs::write(
            workspace.0.join("machines/math/build.rs"),
            "fn main() { println!(\"cargo:rerun-if-changed=second\"); }\n",
        )
        .unwrap();
        let changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();

        assert_eq!(initial, changed);
    }

    #[test]
    fn enabled_default_build_scripts_are_rejected() {
        for manifest in [
            "[package]\nname = \"mech-math\"\n",
            "[package]\nname = \"mech-math\"\nbuild = \"build.rs\"\n",
        ] {
            let workspace = TestWorkspace::new("enabled-build-script");
            fs::write(workspace.0.join("machines/math/Cargo.toml"), manifest).unwrap();
            fs::write(
                workspace.0.join("machines/math/build.rs"),
                "fn main() { println!(\"cargo:rerun-if-env-changed=HOST_INPUT\"); }\n",
            )
            .unwrap();

            let error = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap_err();
            assert_eq!(error.kind_name(), "NativeWorkspaceInputInvalid");
            assert!(
                error
                    .kind_message()
                    .contains("build scripts are unsupported")
            );
        }
    }

    #[test]
    fn nested_resources_from_included_rust_are_fingerprinted() {
        let workspace = TestWorkspace::new("nested-rust-source-include");
        fs::write(
            workspace.0.join("machines/math/src/lib.rs"),
            "include!(\"../generated/catalog.rs\");\n",
        )
        .unwrap();
        fs::create_dir_all(workspace.0.join("machines/math/generated")).unwrap();
        fs::write(
            workspace.0.join("machines/math/generated/catalog.rs"),
            "const GENERATED: &str = include_str!(\"generated.txt\");\n",
        )
        .unwrap();
        fs::write(
            workspace.0.join("machines/math/generated/generated.txt"),
            "generated-v1\n",
        )
        .unwrap();

        let initial = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        fs::write(
            workspace.0.join("machines/math/generated/generated.txt"),
            "generated-v2\n",
        )
        .unwrap();
        let changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_ne!(initial, changed, "nested includes must be fingerprinted");
    }

    #[cfg(unix)]
    #[test]
    fn included_rust_is_validated_before_following_a_symlink_outside_the_workspace() {
        use std::os::unix::fs::symlink;

        let workspace = TestWorkspace::new("included-rust-symlink");
        fs::write(
            workspace.0.join("machines/math/src/lib.rs"),
            "include!(\"../generated.rs\");\n",
        )
        .unwrap();
        let outside = workspace.0.with_extension("outside.rs");
        fs::write(&outside, "pub const OUTSIDE: u8 = 1;\n").unwrap();
        symlink(&outside, workspace.0.join("machines/math/generated.rs")).unwrap();

        let error = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap_err();
        assert_eq!(error.kind_name(), "NativeWorkspaceInputInvalid");
        assert!(
            error
                .kind_message()
                .contains("resolves outside the workspace")
        );

        fs::remove_file(outside).unwrap();
    }

    #[test]
    fn include_text_inside_comments_and_literals_is_ignored() {
        let workspace = TestWorkspace::new("inactive-includes");
        fs::write(
            workspace.0.join("machines/math/src/lib.rs"),
            r###"
// include_bytes!("../assets/missing-line.bin")
/* include_str!["../assets/missing-block.txt"] */
// include!("../generated/missing-line.rs")
const EXAMPLE: &str = "include_bytes!(\"../assets/missing-string.bin\")";
const RAW: &str = r#"include_str!{"../assets/missing-raw.txt"}"#;
const DATA: &[u8] = include_bytes!("../assets/data.bin");
"###,
        )
        .unwrap();

        let initial = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        fs::write(
            workspace.0.join("machines/math/assets/data.bin"),
            b"active-resource-v2",
        )
        .unwrap();
        let changed = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap();
        assert_ne!(initial, changed);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_source_directories_are_rejected_explicitly() {
        use std::os::unix::fs::symlink;

        let workspace = TestWorkspace::new("source-directory-symlink");
        fs::create_dir_all(workspace.0.join("machines/math/shared-source")).unwrap();
        fs::write(
            workspace.0.join("machines/math/shared-source/linked.rs"),
            "pub const LINKED: u8 = 1;\n",
        )
        .unwrap();
        symlink(
            "../shared-source",
            workspace.0.join("machines/math/src/linked"),
        )
        .unwrap();

        let error = fingerprint_workspace(&workspace.0, &[math_package()]).unwrap_err();
        assert_eq!(error.kind_name(), "NativeWorkspaceInputInvalid");
        assert!(
            error
                .kind_message()
                .contains("symlinked entries are not allowed")
        );
        assert!(error.kind_message().contains("machines/math/src/linked"));
    }
}
