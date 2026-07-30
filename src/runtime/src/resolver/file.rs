use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
#[cfg(windows)]
use std::path::Prefix;

use mech_core::{MResult, MechError, MechErrorKind, MechSourceCode};

use crate::{check_fs_capability, SharedCapabilityKernel, FS_IMPORT, FS_READ, FS_RESOLVE};
use crate::resolver::{
  source_request_for_import, ResolvedSource, SourceIndex, SourceRequest, SourceResolver,
};

use super::{
  SourceExtensionDecodeFailed, SourceFileOpenFailed, SourceFileReadFailed,
  SourceIncludeCycle, SourceIncludeReadFailed, SourceKind,
  SourceUnknownFileExtension,
};

impl SourceRequest {
  /// Creates a source request from a filesystem path.
  ///
  /// Unix path bytes are percent-encoded without UTF-8 conversion. The
  /// Windows drive, UNC, verbatim-drive, and verbatim-UNC paths are normalized
  /// to standard local or UNC file URIs. The returned request is suitable for
  /// `FileSourceResolver`.
  pub fn from_filesystem_path(
    path: impl AsRef<Path>,
  ) -> MResult<Self> {
    Ok(Self::new(path_to_file_uri(path.as_ref())?))
  }
}

#[derive(Debug, Clone)]
pub struct SourceFilesystemSpecifierInvalid {
  pub specifier: String,
  pub reason: String,
}

impl MechErrorKind for SourceFilesystemSpecifierInvalid {
  fn name(&self) -> &str { "SourceFilesystemSpecifierInvalid" }

  fn message(&self) -> String {
    format!("Invalid filesystem source specifier `{}`: {}", self.specifier, self.reason)
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FilesystemSourceSpecifier {
  Ordinary(PathBuf),
  RootRelative(PathBuf),
  Absolute(PathBuf),
  OtherScheme,
}

#[derive(Clone, Debug)]
pub struct FileSourceResolver {
  roots: Vec<PathBuf>,
  capability_kernel: Option<SharedCapabilityKernel>,
  capability_subject: Option<String>,
}

impl FileSourceResolver {
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self {
      roots: vec![root.into()],
      capability_kernel: None,
      capability_subject: None,
    }
  }

  pub fn empty() -> Self {
    Self {
      roots: Vec::new(),
      capability_kernel: None,
      capability_subject: None,
    }
  }

  pub fn with_capabilities(mut self, kernel: SharedCapabilityKernel, subject: impl Into<String>) -> Self {
    self.capability_kernel = Some(kernel);
    self.capability_subject = Some(subject.into());
    self
  }

  fn check(&self, operation: &str, path: &Path) -> MResult<()> {
    if let (Some(kernel), Some(subject)) = (&self.capability_kernel, &self.capability_subject) {
      check_fs_capability(&mut kernel.clone(), subject, operation, path)?;
    }
    Ok(())
  }

  pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
    self.roots.push(root.into());
    self
  }

  pub fn add_root(&mut self, root: impl Into<PathBuf>) {
    self.roots.push(root.into());
  }

  pub fn roots(&self) -> &[PathBuf] {
    &self.roots
  }

  fn resolve_path(
    &self,
    request: &SourceRequest,
  ) -> MResult<Option<PathBuf>> {
    match parse_filesystem_source_specifier(&request.specifier)? {
      FilesystemSourceSpecifier::OtherScheme => Ok(None),
      FilesystemSourceSpecifier::Absolute(specifier) => {
        for candidate in absolute_path_candidates(&specifier) {
          if candidate.is_file() {
            return self.authorize_resolved(request, canonicalize_source_path(&candidate)?);
          }
        }
        Ok(None)
      }
      FilesystemSourceSpecifier::RootRelative(specifier) => {
        for root in &self.roots {
          for candidate in path_candidates(root, &specifier) {
            if candidate.is_file() {
              return self.authorize_resolved(request, canonicalize_source_path(&candidate)?);
            }
          }
        }
        Ok(None)
      }
      FilesystemSourceSpecifier::Ordinary(specifier) => {
        if specifier.is_absolute() {
          for candidate in absolute_path_candidates(&specifier) {
            if candidate.is_file() {
              return self.authorize_resolved(request, canonicalize_source_path(&candidate)?);
            }
          }
          return Ok(None);
        }

        if let Some(referrer) = &request.referrer {
          if let Some(referrer_path) = self.normalize_referrer_path(referrer)? {
            let parent = if referrer_path.is_dir() {
              Some(referrer_path.as_path())
            } else {
              referrer_path.parent()
            };

            if let Some(parent) = parent {
              for candidate in path_candidates(parent, &specifier) {
                if candidate.is_file() {
                  return self.authorize_resolved(request, canonicalize_source_path(&candidate)?);
                }
              }
            }
          }
        }

        for root in &self.roots {
          for candidate in path_candidates(root, &specifier) {
            if candidate.is_file() {
              return self.authorize_resolved(request, canonicalize_source_path(&candidate)?);
            }
          }
        }

        Ok(None)
      }
    }
  }

  fn normalize_referrer_path(&self, referrer: &str) -> MResult<Option<PathBuf>> {
    match parse_filesystem_source_specifier(referrer)? {
      FilesystemSourceSpecifier::OtherScheme => Ok(None),
      FilesystemSourceSpecifier::Ordinary(path) | FilesystemSourceSpecifier::Absolute(path) => {
        Ok(Some(path))
      }
      FilesystemSourceSpecifier::RootRelative(path) => {
        for root in &self.roots {
          for candidate in path_candidates(root, &path) {
            if candidate.is_file() {
              return Ok(Some(canonicalize_source_path(&candidate)?));
            }
          }
        }
        Ok(None)
      }
    }
  }

  fn authorize_resolved(&self, request: &SourceRequest, path: PathBuf) -> MResult<Option<PathBuf>> {
    self.check(FS_RESOLVE, &path)?;
    if request.referrer.is_some() { self.check(FS_IMPORT, &path)?; }
    Ok(Some(path))
  }
}

impl SourceResolver for FileSourceResolver {
  fn resolve(
    &self,
    request: &SourceRequest,
  ) -> MResult<Option<ResolvedSource>> {
    request.validate()?;

    let Some(path) = self.resolve_path(request)? else {
      return Ok(None);
    };

    let kind = SourceKind::from_path(&path);
    let source = read_runtime_source_file_with_capabilities_and_import_checks(
      &path,
      self.capability_kernel.as_ref(),
      self.capability_subject.as_deref(),
      request.referrer.is_some(),
    )?;
    let name = path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or("source")
      .to_string();

    let canonical_uri = path_to_file_uri(&path)?;
    let mut resolved = ResolvedSource::new(name, canonical_uri.clone(), source).with_kind(kind);

    if resolved.kind == SourceKind::Mech {
      if let MechSourceCode::String(source_text) = &resolved.source {
        let tree = mech_syntax::parser::parse(source_text.trim())?;
        let referrer = canonical_uri.clone();
        let index = SourceIndex::from_program(&tree);
        index.validate_address_targets()?;
        let imports = index.all_imports();
        let exports = index.all_exports();
        let contexts = index.all_contexts();
        let address_references = index.all_address_references();
        let scopes = index.module_scopes();
        let dependencies = imports
          .iter()
          .map(|import| source_request_for_import(import, Some(&referrer)))
          .collect::<Vec<_>>();

        resolved = resolved
          .with_imports(imports)
          .with_exports(exports)
          .with_contexts(contexts)
          .with_address_references(address_references)
          .with_dependencies(dependencies)
          .with_scopes(scopes);
      }
    }

    Ok(Some(resolved))
  }
}

fn parse_filesystem_source_specifier(specifier: &str) -> MResult<FilesystemSourceSpecifier> {
  let Some((scheme, rest)) = split_uri_scheme(specifier) else {
    return Ok(FilesystemSourceSpecifier::Ordinary(PathBuf::from(specifier)));
  };
  match scheme {
    "fs" => parse_fs_uri(specifier, rest),
    "file" => file_uri_to_path(specifier).map(FilesystemSourceSpecifier::Absolute),
    _ => Ok(FilesystemSourceSpecifier::OtherScheme),
  }
}

fn split_uri_scheme(specifier: &str) -> Option<(&str, &str)> {
  let (prefix, rest) = specifier.split_once("://")?;
  if valid_uri_scheme(prefix) { Some((prefix, rest)) } else { None }
}

fn valid_uri_scheme(prefix: &str) -> bool {
  let mut chars = prefix.chars();
  let Some(first) = chars.next() else { return false; };
  first.is_ascii_alphabetic()
    && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '+' || ch == '.' || ch == '-')
}

fn filesystem_specifier_error(specifier: impl Into<String>, reason: impl Into<String>) -> MechError {
  MechError::new(SourceFilesystemSpecifierInvalid { specifier: specifier.into(), reason: reason.into() }, None)
}

fn parse_fs_uri(specifier: &str, rest: &str) -> MResult<FilesystemSourceSpecifier> {
  reject_query_or_fragment(specifier, rest)?;
  let raw_path = rest.trim_start_matches('/');
  if raw_path.is_empty() {
    return Err(filesystem_specifier_error(specifier, "fs:// path must not be empty"));
  }
  let decoded = percent_decode_to_string(specifier, raw_path)?;
  Ok(FilesystemSourceSpecifier::RootRelative(normalize_root_relative_path(specifier, &decoded)?))
}

fn reject_query_or_fragment(specifier: &str, rest: &str) -> MResult<()> {
  if rest.contains('?') {
    return Err(filesystem_specifier_error(specifier, "query strings are not supported"));
  }
  if rest.contains('#') {
    return Err(filesystem_specifier_error(specifier, "fragments are not supported"));
  }
  Ok(())
}

fn normalize_root_relative_path(specifier: &str, decoded: &str) -> MResult<PathBuf> {
  if decoded.trim().is_empty() {
    return Err(filesystem_specifier_error(specifier, "path must not be empty"));
  }
  if looks_like_windows_drive_path(decoded) {
    return Err(filesystem_specifier_error(specifier, "Windows drive prefixes are not permitted in fs:// paths"));
  }
  let path = Path::new(decoded);
  if path.is_absolute() {
    return Err(filesystem_specifier_error(specifier, "absolute paths are not permitted in fs:// paths"));
  }

  let mut normalized = PathBuf::new();
  for component in path.components() {
    match component {
      Component::Normal(part) => normalized.push(part),
      Component::CurDir => {}
      Component::ParentDir => return Err(filesystem_specifier_error(specifier, "parent traversal is not permitted in fs:// paths")),
      Component::RootDir => return Err(filesystem_specifier_error(specifier, "root components are not permitted in fs:// paths")),
      Component::Prefix(_) => return Err(filesystem_specifier_error(specifier, "path prefixes are not permitted in fs:// paths")),
    }
  }
  if normalized.as_os_str().is_empty() {
    return Err(filesystem_specifier_error(specifier, "path must not be empty"));
  }
  Ok(normalized)
}

fn looks_like_windows_drive_path(text: &str) -> bool {
  let bytes = text.as_bytes();
  bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

pub(crate) fn file_uri_to_path(uri: &str) -> MResult<PathBuf> {
  let Some(rest) = uri.strip_prefix("file://") else {
    return Err(filesystem_specifier_error(uri, "file URI must start with file://"));
  };
  reject_query_or_fragment(uri, rest)?;
  let (authority, raw_path) = if rest.starts_with('/') {
    ("", rest)
  } else if let Some(path_start) = rest.find('/') {
    (&rest[..path_start], &rest[path_start..])
  } else {
    return Err(filesystem_specifier_error(uri, "file URI must include an absolute path"));
  };
  local_file_uri_to_path(uri, authority, raw_path)
}

#[cfg(unix)]
fn local_file_uri_to_path(uri: &str, authority: &str, raw_path: &str) -> MResult<PathBuf> {
  use std::ffi::OsString;
  use std::os::unix::ffi::OsStringExt;

  if !(authority.is_empty() || authority.eq_ignore_ascii_case("localhost")) {
    return Err(filesystem_specifier_error(uri, "non-local file URI authorities are not supported on this platform"));
  }
  let decoded_path = percent_decode_to_bytes(uri, raw_path)?;
  if decoded_path.first() != Some(&b'/') {
    return Err(filesystem_specifier_error(uri, "file URI path must be absolute"));
  }
  Ok(PathBuf::from(OsString::from_vec(decoded_path)))
}

#[cfg(windows)]
fn local_file_uri_to_path(uri: &str, authority: &str, raw_path: &str) -> MResult<PathBuf> {
  let decoded_path = percent_decode_to_string(uri, raw_path)?;
  if authority.is_empty() || authority.eq_ignore_ascii_case("localhost") {
    let local_path = if decoded_path.len() >= 4
      && decoded_path.as_bytes()[0] == b'/'
      && decoded_path.as_bytes()[1].is_ascii_alphabetic()
      && decoded_path.as_bytes()[2] == b':'
      && decoded_path.as_bytes()[3] == b'/'
    {
      decoded_path[1..].replace('/', "\\")
    } else {
      return Err(filesystem_specifier_error(uri, "local Windows file URI path must include a drive prefix"));
    };
    return Ok(PathBuf::from(local_path));
  }
  let path = decoded_path.trim_start_matches('/').replace('/', "\\");
  if path.is_empty() {
    return Err(filesystem_specifier_error(uri, "UNC file URI must include a share path"));
  }
  Ok(PathBuf::from(format!("\\\\{}\\{}", authority, path)))
}

#[cfg(not(any(unix, windows)))]
fn local_file_uri_to_path(uri: &str, authority: &str, raw_path: &str) -> MResult<PathBuf> {
  if !(authority.is_empty() || authority.eq_ignore_ascii_case("localhost")) {
    return Err(filesystem_specifier_error(uri, "non-local file URI authorities are not supported on this platform"));
  }
  let decoded_path = percent_decode_to_string(uri, raw_path)?;
  if !decoded_path.starts_with('/') {
    return Err(filesystem_specifier_error(uri, "file URI path must be absolute"));
  }
  Ok(PathBuf::from(decoded_path))
}

fn percent_decode_to_bytes(specifier: &str, text: &str) -> MResult<Vec<u8>> {
  let bytes = text.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut index = 0;
  while index < bytes.len() {
    if bytes[index] == b'%' {
      if index + 2 >= bytes.len() {
        return Err(filesystem_specifier_error(specifier, "malformed percent escape"));
      }
      let high = hex_value(bytes[index + 1]).ok_or_else(|| filesystem_specifier_error(specifier, "malformed percent escape"))?;
      let low = hex_value(bytes[index + 2]).ok_or_else(|| filesystem_specifier_error(specifier, "malformed percent escape"))?;
      out.push((high << 4) | low);
      index += 3;
    } else {
      out.push(bytes[index]);
      index += 1;
    }
  }
  Ok(out)
}

fn percent_decode_to_string(specifier: &str, text: &str) -> MResult<String> {
  String::from_utf8(percent_decode_to_bytes(specifier, text)?)
    .map_err(|error| filesystem_specifier_error(specifier, format!("percent-decoded path is not valid UTF-8: {}", error)))
}

fn hex_value(byte: u8) -> Option<u8> {
  match byte {
    b'0'..=b'9' => Some(byte - b'0'),
    b'a'..=b'f' => Some(byte - b'a' + 10),
    b'A'..=b'F' => Some(byte - b'A' + 10),
    _ => None,
  }
}

fn path_to_file_uri(path: &Path) -> MResult<String> {
  #[cfg(windows)]
  {
    return windows_path_to_file_uri(path);
  }

  #[cfg(unix)]
  {
    use std::os::unix::ffi::OsStrExt;
    if !path.is_absolute() {
      return Err(filesystem_specifier_error(path.display().to_string(), "file URI source path must be absolute"));
    }
    Ok(format!("file://{}", percent_encode_path(path.as_os_str().as_bytes())))
  }

  #[cfg(not(any(unix, windows)))]
  {
    if !path.is_absolute() {
      return Err(filesystem_specifier_error(path.display().to_string(), "file URI source path must be absolute"));
    }
    let text = path.to_str().ok_or_else(|| {
      filesystem_specifier_error(
        path.display().to_string(),
        "file URI source path must be valid UTF-8 on this target",
      )
    })?;
    Ok(format!("file://{}", percent_encode_path(text.as_bytes())))
  }
}

#[cfg(windows)]
fn windows_path_to_file_uri(path: &Path) -> MResult<String> {
  let path_label = format!("{path:?}");
  let mut components = path.components();
  let prefix = match components.next() {
    Some(Component::Prefix(prefix)) => prefix.kind(),
    _ => {
      return Err(filesystem_specifier_error(
        path_label,
        "Windows file URI source path must include a supported absolute prefix",
      ));
    }
  };

  let mut uri = match prefix {
    Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => {
      if !path.is_absolute() {
        return Err(filesystem_specifier_error(
          path_label,
          "file URI source path must be absolute",
        ));
      }
      format!("file:///{}:", (drive as char).to_ascii_uppercase())
    }
    Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
      if !path.is_absolute() {
        return Err(filesystem_specifier_error(
          path_label,
          "file URI source path must be absolute",
        ));
      }
      let server = windows_file_uri_component(path, server)?;
      let share = windows_file_uri_component(path, share)?;
      if server.is_empty() || share.is_empty() {
        return Err(filesystem_specifier_error(
          path_label,
          "UNC file URI source path must include server and share names",
        ));
      }
      format!("file://{server}/{share}")
    }
    Prefix::DeviceNS(_) => {
      return Err(filesystem_specifier_error(
        path_label,
        "Windows device namespace paths are not supported as file URI sources",
      ));
    }
    Prefix::Verbatim(_) => {
      return Err(filesystem_specifier_error(
        path_label,
        "unsupported Windows verbatim namespace path",
      ));
    }
  };

  for component in components {
    match component {
      Component::Prefix(_) => {
        return Err(filesystem_specifier_error(
          path_label,
          "Windows source path contained more than one prefix",
        ));
      }
      Component::RootDir | Component::CurDir => {}
      Component::ParentDir => uri.push_str("/.."),
      Component::Normal(part) => {
        uri.push('/');
        uri.push_str(&windows_file_uri_component(path, part)?);
      }
    }
  }
  Ok(uri)
}

#[cfg(windows)]
fn windows_file_uri_component(path: &Path, component: &std::ffi::OsStr) -> MResult<String> {
  let text = component.to_str().ok_or_else(|| {
    filesystem_specifier_error(
      format!("{path:?}"),
      "file URI source path must be valid UTF-8 on this target",
    )
  })?;
  Ok(percent_encode_path_segment(text.as_bytes()))
}

fn percent_encode_path(bytes: &[u8]) -> String {
  let mut out = String::new();
  for &byte in bytes {
    if is_file_uri_path_byte_unreserved(byte) {
      out.push(byte as char);
    } else {
      out.push('%');
      out.push(hex_char(byte >> 4));
      out.push(hex_char(byte & 0x0f));
    }
  }
  out
}

#[cfg(windows)]
fn percent_encode_path_segment(bytes: &[u8]) -> String {
  let mut out = String::new();
  for &byte in bytes {
    if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
      out.push(byte as char);
    } else {
      out.push('%');
      out.push(hex_char(byte >> 4));
      out.push(hex_char(byte & 0x0f));
    }
  }
  out
}

fn is_file_uri_path_byte_unreserved(byte: u8) -> bool {
  byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/' | b':')
}

fn hex_char(value: u8) -> char {
  match value {
    0..=9 => (b'0' + value) as char,
    10..=15 => (b'A' + value - 10) as char,
    _ => unreachable!(),
  }
}

fn path_candidates(base: &Path, specifier: &Path) -> Vec<PathBuf> {
  vec![
    base.join(specifier),
    base.join(format!("{}.mec", specifier.to_string_lossy())),
    base.join(specifier).join("index.mec"),
  ]
}

fn absolute_path_candidates(specifier: &Path) -> Vec<PathBuf> {
  vec![
    specifier.to_path_buf(),
    PathBuf::from(format!("{}.mec", specifier.to_string_lossy())),
    specifier.join("index.mec"),
  ]
}

pub fn read_runtime_source_file(path: &Path) -> MResult<MechSourceCode> {
  read_runtime_source_file_with_capabilities(path, None, None)
}

pub fn read_runtime_source_file_with_capabilities(path: &Path, kernel: Option<&SharedCapabilityKernel>, subject: Option<&str>) -> MResult<MechSourceCode> {
  read_runtime_source_file_with_capabilities_and_import_checks(path, kernel, subject, false)
}

fn read_runtime_source_file_with_capabilities_and_import_checks(
  path: &Path,
  kernel: Option<&SharedCapabilityKernel>,
  subject: Option<&str>,
  require_import_for_includes: bool,
) -> MResult<MechSourceCode> {
  check_optional_fs(kernel, subject, FS_READ, path)?;
  let extension = path
    .extension()
    .and_then(|extension| extension.to_str())
    .ok_or_else(|| {
      MechError::new(
        SourceExtensionDecodeFailed {
          path: path.display().to_string(),
        },
        None,
      )
    })?
    .to_ascii_lowercase();

  match extension.as_str() {
    "mec" | "🤖" => {
      let expanded = expand_mechdown_includes_with_capabilities_and_import_checks(
        path,
        kernel,
        subject,
        require_import_for_includes,
      )?;
      Ok(MechSourceCode::String(expanded))
    }

    "mecb" => {
      // Keep this as raw bytecode source for now only if your MechSourceCode
      // supports ByteCode(Vec<u8>). If your current loader needs
      // `load_program_from_file`, wire that in module builder later.
      let bytes = read_file_bytes_with_capabilities(path, kernel, subject)?;
      Ok(MechSourceCode::ByteCode(bytes))
    }

    "html" | "htm" | "md" | "css" => {
      Ok(MechSourceCode::Html(read_file_string_with_capabilities(path, kernel, subject)?))
    }

    "mdoc" | "mpkg" | "m" | "csv" | "js" => {
      Ok(MechSourceCode::String(read_file_string_with_capabilities(path, kernel, subject)?))
    }

    "png" | "jpg" | "jpeg" | "gif" | "svg" => {
      Ok(MechSourceCode::Image(extension, read_file_bytes_with_capabilities(path, kernel, subject)?))
    }

    other => Err(MechError::new(
      SourceUnknownFileExtension {
        path: path.display().to_string(),
        extension: other.to_string(),
      },
      None,
    )),
  }
}

fn check_optional_fs(kernel: Option<&SharedCapabilityKernel>, subject: Option<&str>, operation: &str, path: &Path) -> MResult<()> {
  if let (Some(kernel), Some(subject)) = (kernel, subject) { check_fs_capability(&mut kernel.clone(), subject, operation, path)?; }
  Ok(())
}

pub fn read_file_string_with_capabilities(path: &Path, kernel: Option<&SharedCapabilityKernel>, subject: Option<&str>) -> MResult<String> {
  check_optional_fs(kernel, subject, FS_READ, path)?;
  read_file_string(path)
}

pub fn read_file_string(path: &Path) -> MResult<String> {
  let mut file = File::open(path).map_err(|error| {
    MechError::new(
      SourceFileOpenFailed {
        path: path.display().to_string(),
        source: error.to_string(),
      },
      None,
    )
  })?;

  let mut buffer = String::new();

  file.read_to_string(&mut buffer).map_err(|error| {
    MechError::new(
      SourceFileReadFailed {
        path: path.display().to_string(),
        source: error.to_string(),
      },
      None,
    )
  })?;

  Ok(buffer)
}

pub fn read_file_bytes_with_capabilities(path: &Path, kernel: Option<&SharedCapabilityKernel>, subject: Option<&str>) -> MResult<Vec<u8>> {
  check_optional_fs(kernel, subject, FS_READ, path)?;
  read_file_bytes(path)
}

pub fn read_file_bytes(path: &Path) -> MResult<Vec<u8>> {
  std::fs::read(path).map_err(|error| {
    MechError::new(
      SourceFileReadFailed {
        path: path.display().to_string(),
        source: error.to_string(),
      },
      None,
    )
  })
}

pub fn expand_mechdown_includes(path: &Path) -> MResult<String> { expand_mechdown_includes_with_capabilities(path, None, None) }

pub fn expand_mechdown_includes_with_capabilities(path: &Path, kernel: Option<&SharedCapabilityKernel>, subject: Option<&str>) -> MResult<String> {
  expand_mechdown_includes_with_capabilities_and_import_checks(path, kernel, subject, false)
}

fn expand_mechdown_includes_with_capabilities_and_import_checks(
  path: &Path,
  kernel: Option<&SharedCapabilityKernel>,
  subject: Option<&str>,
  require_import_for_includes: bool,
) -> MResult<String> {
  let canonical = canonicalize_source_path(path)?;
  let mut active = HashSet::new();
  expand_mechdown_includes_inner(
    &canonical,
    &mut active,
    kernel,
    subject,
    require_import_for_includes,
  )
}

fn expand_mechdown_includes_inner(
  path: &Path,
  active: &mut HashSet<PathBuf>,
  kernel: Option<&SharedCapabilityKernel>,
  subject: Option<&str>,
  require_import_for_includes: bool,
) -> MResult<String> {
  let canonical = canonicalize_source_path(path)?;

  if active.contains(&canonical) {
    return Err(MechError::new(
      SourceIncludeCycle {
        path: canonical.display().to_string(),
      },
      None,
    ));
  }

  active.insert(canonical.clone());

  let source = read_file_string_with_capabilities(&canonical, kernel, subject)?;
  let expanded = expand_mechdown_include_tokens(
    &source,
    &canonical,
    active,
    kernel,
    subject,
    require_import_for_includes,
  )?;

  active.remove(&canonical);

  Ok(expanded)
}

fn expand_mechdown_include_tokens(
  source: &str,
  canonical_path: &Path,
  active: &mut HashSet<PathBuf>,
  kernel: Option<&SharedCapabilityKernel>,
  subject: Option<&str>,
  require_import_for_includes: bool,
) -> MResult<String> {
  let mut result = String::new();

  for line in source.split_inclusive('\n') {
    let (line_without_newline, newline) = match line.strip_suffix('\n') {
      Some(prefix) => (prefix, "\n"),
      None => (line, ""),
    };

    if let Some(inner) = standalone_braced_content(line_without_newline) {
      if looks_like_mech_include(inner) {
        let include_raw = inner.trim();
        let parent = canonical_path.parent().unwrap_or(Path::new("."));
        let include_path = parent.join(include_raw);

        let include_canonical = canonicalize_source_path(&include_path)
          .map_err(|error| {
            MechError::new(
              SourceIncludeReadFailed {
                path: canonical_path.display().to_string(),
                include: include_raw.to_string(),
                source: format!("{:?}", error),
              },
              None,
            )
          })?;

        if require_import_for_includes {
          check_optional_fs(kernel, subject, FS_IMPORT, &include_canonical)?;
        }

        let include_source =
          expand_mechdown_includes_inner(
            &include_canonical,
            active,
            kernel,
            subject,
            require_import_for_includes,
          )?;

        result.push_str(&include_source);

        if !include_source.ends_with('\n') {
          result.push('\n');
        }

        result.push_str(newline);

        continue;
      }
    }

    result.push_str(line);
  }

  Ok(result)
}

fn looks_like_mech_include(content: &str) -> bool {
  let trimmed = content.trim();

  trimmed.ends_with(".mec") || trimmed.ends_with(".🤖")
}

fn standalone_braced_content(line_without_newline: &str) -> Option<&str> {
  let trimmed = line_without_newline.trim();

  if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
    return None;
  }

  Some(&trimmed[1..trimmed.len() - 1])
}

fn canonicalize_source_path(path: &Path) -> MResult<PathBuf> {
  path.canonicalize().map_err(|error| {
    MechError::new(
      SourceFileReadFailed {
        path: path.display().to_string(),
        source: error.to_string(),
      },
      None,
    )
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("{}-{}", name, nanos))
  }

  #[test]
  fn source_kind_classifies_mech_files() {
    assert_eq!(
      SourceKind::from_extension("mec"),
      SourceKind::Mech,
    );

    assert_eq!(
      SourceKind::from_extension("mecb"),
      SourceKind::MechBytecode,
    );
  }

  #[test]
  fn file_resolver_resolves_mec_file() {
    let root = temp_root("mech-runtime-file-resolver-test");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let path = root.join("index.mec");
    std::fs::write(&path, "x := 1").unwrap();

    let resolver = FileSourceResolver::new(&root);
    let request = SourceRequest::new("index.mec");

    let resolved = resolver.resolve(&request).unwrap().unwrap();

    assert_eq!(resolved.name, "index.mec");
    assert_eq!(resolved.kind, SourceKind::Mech);
    assert!(resolved.canonical_uri.starts_with("file://"));
    assert!(resolved.is_executable_mech_source());
  }

  #[test]
  fn resolves_math_to_math_mec() {
    let root = temp_root("resolve-math-mec");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("math.mec"), "x := 1").unwrap();
    let resolver = FileSourceResolver::new(&root);
    let resolved = resolver.resolve(&SourceRequest::new("math")).unwrap().unwrap();
    assert_eq!(resolved.name, "math.mec");
  }

  #[test]
  fn resolves_math_to_math_index_mec_when_no_math_mec() {
    let root = temp_root("resolve-math-index");
    std::fs::create_dir_all(root.join("math")).unwrap();
    std::fs::write(root.join("math/index.mec"), "x := 1").unwrap();
    let resolver = FileSourceResolver::new(&root);
    let resolved = resolver.resolve(&SourceRequest::new("math")).unwrap().unwrap();
    assert_eq!(resolved.name, "index.mec");
  }

  #[test]
  fn resolves_math_sin_to_math_sin_mec() {
    let root = temp_root("resolve-math-sin");
    std::fs::create_dir_all(root.join("math")).unwrap();
    std::fs::write(root.join("math/sin.mec"), "x := 1").unwrap();
    let resolver = FileSourceResolver::new(&root);
    let resolved = resolver.resolve(&SourceRequest::new("math/sin")).unwrap().unwrap();
    assert_eq!(resolved.name, "sin.mec");
  }

  #[test]
  fn resolves_relative_import_from_referrer_parent() {
    let root = temp_root("resolve-referrer-parent");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    let referrer = root.join("sub/main.mec");
    std::fs::write(root.join("sub/math.mec"), "x := 1").unwrap();
    std::fs::write(&referrer, "+> ./math.mec").unwrap();
    let resolver = FileSourceResolver::new(&root);
    let request = SourceRequest::new("./math.mec").with_referrer(referrer.to_string_lossy().to_string());
    let resolved = resolver.resolve(&request).unwrap().unwrap();
    assert_eq!(resolved.name, "math.mec");
  }

  #[test]
  fn resolves_fs_uri_beneath_configured_root() {
    let root = temp_root("resolve-fs-root");
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(root.join("lib/dep.mec"), "value := 1").unwrap();
    let resolver = FileSourceResolver::new(&root);
    let resolved = resolver.resolve(&SourceRequest::new("fs://lib/dep.mec")).unwrap().unwrap();
    assert_eq!(resolved.name, "dep.mec");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn resolves_fs_uri_with_empty_authority_beneath_configured_root() {
    let root = temp_root("resolve-fs-root-empty-authority");
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(root.join("lib/dep.mec"), "value := 1").unwrap();
    let resolver = FileSourceResolver::new(&root);
    let resolved = resolver.resolve(&SourceRequest::new("fs:///lib/dep.mec")).unwrap().unwrap();
    assert_eq!(resolved.name, "dep.mec");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn rejects_fs_uri_parent_traversal_before_filesystem_probe() {
    let root = temp_root("reject-fs-parent");
    std::fs::create_dir_all(&root).unwrap();
    let resolver = FileSourceResolver::new(&root);
    let error = resolver.resolve(&SourceRequest::new("fs://../dep.mec")).unwrap_err();
    assert!(error.kind_as::<SourceFilesystemSpecifierInvalid>().is_some());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn resolves_percent_encoded_fs_uri_path() {
    let root = temp_root("resolve-fs-percent");
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(root.join("lib/space dep.mec"), "value := 1").unwrap();
    let resolver = FileSourceResolver::new(&root);
    let resolved = resolver.resolve(&SourceRequest::new("fs://lib/space%20dep.mec")).unwrap().unwrap();
    assert_eq!(resolved.name, "space dep.mec");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn resolves_file_uri_absolute_path() {
    let root = temp_root("resolve-file-absolute");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("main.mec");
    std::fs::write(&path, "value := 1").unwrap();
    let uri = path_to_file_uri(&path.canonicalize().unwrap()).unwrap();
    let resolver = FileSourceResolver::empty();
    let resolved = resolver.resolve(&SourceRequest::new(uri)).unwrap().unwrap();
    assert_eq!(resolved.name, "main.mec");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn resolves_file_uri_localhost_path() {
    let root = temp_root("resolve-file-localhost");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("main.mec");
    std::fs::write(&path, "value := 1").unwrap();
    let uri = path_to_file_uri(&path.canonicalize().unwrap()).unwrap().replacen("file://", "file://localhost", 1);
    let resolver = FileSourceResolver::empty();
    let resolved = resolver.resolve(&SourceRequest::new(uri)).unwrap().unwrap();
    assert_eq!(resolved.name, "main.mec");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn resolves_percent_encoded_file_uri_path() {
    let root = temp_root("resolve-file-percent");
    std::fs::create_dir_all(root.join("space dir")).unwrap();
    let path = root.join("space dir/dep #1.mec");
    std::fs::write(&path, "value := 1").unwrap();
    let uri = path_to_file_uri(&path.canonicalize().unwrap()).unwrap();
    assert!(uri.contains("%20"));
    assert!(uri.contains("%23"));
    let resolver = FileSourceResolver::empty();
    let resolved = resolver.resolve(&SourceRequest::new(uri)).unwrap().unwrap();
    assert_eq!(resolved.name, "dep #1.mec");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn file_uri_referrer_resolves_relative_import_from_parent() {
    let root = temp_root("resolve-file-referrer");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    let referrer = root.join("sub/main.mec");
    let dep = root.join("sub/dep.mec");
    std::fs::write(&referrer, "+> ./dep.mec").unwrap();
    std::fs::write(&dep, "value := 1").unwrap();
    let referrer_uri = path_to_file_uri(&referrer.canonicalize().unwrap()).unwrap();
    let resolver = FileSourceResolver::new(&root);
    let request = SourceRequest::new("./dep.mec").with_referrer(referrer_uri);
    let resolved = resolver.resolve(&request).unwrap().unwrap();
    assert_eq!(resolved.name, "dep.mec");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn other_uri_schemes_are_ignored_by_file_resolver() {
    let root = temp_root("resolve-other-schemes");
    std::fs::create_dir_all(&root).unwrap();
    let resolver = FileSourceResolver::new(&root);
    assert!(resolver.resolve(&SourceRequest::new("https://example.com/main.mec")).unwrap().is_none());
    assert!(resolver.resolve(&SourceRequest::new("memory://main.mec")).unwrap().is_none());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn file_uri_helpers_round_trip_supported_local_path() {
    let root = temp_root("file-uri-round-trip");
    std::fs::create_dir_all(root.join("space dir")).unwrap();
    let path = root.join("space dir/dep #1%.mec");
    std::fs::write(&path, "value := 1").unwrap();
    let canonical = path.canonicalize().unwrap();
    let uri = path_to_file_uri(&canonical).unwrap();
    assert_eq!(
      file_uri_to_path(&uri).unwrap().canonicalize().unwrap(),
      canonical.canonicalize().unwrap(),
    );
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn filesystem_source_request_round_trips_non_utf8_path_bytes() {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let path = temp_root("file-uri-non-utf8-round-trip")
      .join(OsString::from_vec(b"main-\xFF.mec".to_vec()));
    let request = SourceRequest::from_filesystem_path(&path).unwrap();

    assert!(request.specifier.starts_with("file://"));
    assert!(request.specifier.contains("%FF"));
    assert!(!request.specifier.contains('\u{FFFD}'));

    let decoded = file_uri_to_path(&request.specifier).unwrap();
    assert_eq!(
      decoded.as_os_str().as_bytes(),
      path.as_os_str().as_bytes(),
    );
  }

  #[cfg(target_os = "linux")]
  #[test]
  fn file_resolver_reads_non_utf8_source_path() {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let root = temp_root("resolve-file-non-utf8");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join(OsString::from_vec(b"main-\xFF.mec".to_vec()));
    std::fs::write(&path, "answer := 42\n").unwrap();
    let canonical = path.canonicalize().unwrap();

    let resolver = FileSourceResolver::empty();
    let request = SourceRequest::from_filesystem_path(&canonical).unwrap();
    let resolved = resolver.resolve(&request).unwrap().unwrap();

    assert_eq!(resolved.kind, SourceKind::Mech);
    assert!(resolved.canonical_uri.starts_with("file://"));
    let round_trip = file_uri_to_path(&resolved.canonical_uri).unwrap();
    assert_eq!(
      round_trip.as_os_str().as_bytes(),
      canonical.as_os_str().as_bytes(),
    );
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(not(windows))]
  #[test]
  fn rejects_nonlocal_file_uri_authority_on_non_windows() {
    let error = file_uri_to_path("file://server/share/project/main.mec").unwrap_err();
    assert!(error.kind_as::<SourceFilesystemSpecifierInvalid>().is_some());
  }

  #[cfg(windows)]
  #[test]
  fn supports_windows_drive_and_unc_file_uri_forms() {
    assert_eq!(file_uri_to_path("file:///C:/project/main.mec").unwrap(), PathBuf::from("C:\\project\\main.mec"));
    assert_eq!(file_uri_to_path("file://server/share/project/main.mec").unwrap(), PathBuf::from("\\\\server\\share\\project\\main.mec"));
  }

  #[cfg(windows)]
  #[test]
  fn windows_path_codec_normalizes_drive_and_verbatim_drive_prefixes() {
    let disk = path_to_file_uri(Path::new(r"C:\project dir\main.mec")).unwrap();
    let verbatim =
      path_to_file_uri(Path::new(r"\\?\C:\project dir\main.mec")).unwrap();

    assert_eq!(disk, "file:///C:/project%20dir/main.mec");
    assert_eq!(verbatim, disk);
    assert!(disk.starts_with("file:///"));
    assert!(!disk.contains("%3F"));
    assert!(!verbatim.contains("%3F"));
  }

  #[cfg(windows)]
  #[test]
  fn windows_path_codec_normalizes_unc_and_verbatim_unc_prefixes() {
    let unc =
      path_to_file_uri(Path::new(r"\\server\share\project dir\main.mec")).unwrap();
    let verbatim = path_to_file_uri(Path::new(
      r"\\?\UNC\server\share\project dir\main.mec",
    ))
    .unwrap();

    assert_eq!(unc, "file://server/share/project%20dir/main.mec");
    assert_eq!(verbatim, unc);
    assert!(!unc.contains("%3F"));
    assert!(!verbatim.contains("%3F"));
  }

  #[cfg(windows)]
  #[test]
  fn windows_path_codec_rejects_unsupported_namespaces() {
    for path in [Path::new(r"\\.\COM1"), Path::new(r"\\?\pictures")] {
      let error = path_to_file_uri(path).unwrap_err();
      assert_eq!(error.kind_name(), "SourceFilesystemSpecifierInvalid");
    }
  }

  #[cfg(windows)]
  #[test]
  fn windows_canonicalized_source_request_resolves_real_file() {
    let root = temp_root("windows-canonical-source");
    let source_dir = root
      .join("Mech Windows ü source path # % spaces")
      .join("nested source");
    std::fs::create_dir_all(&source_dir).unwrap();
    let source_path = source_dir.join("main # %.mec");
    std::fs::write(&source_path, "answer := 42\n").unwrap();
    let canonical = source_path.canonicalize().unwrap();

    let request = SourceRequest::from_filesystem_path(&canonical).unwrap();
    assert!(!request.specifier.contains("%3F"));
    assert!(request.specifier.contains("%20"));
    assert!(request.specifier.contains("%C3%BC"));
    assert!(request.specifier.contains("%23"));
    assert!(request.specifier.contains("%25"));

    let resolved = FileSourceResolver::empty()
      .resolve(&request)
      .unwrap()
      .unwrap();
    assert_eq!(resolved.kind, SourceKind::Mech);
    assert_eq!(
      file_uri_to_path(&resolved.canonical_uri)
        .unwrap()
        .canonicalize()
        .unwrap(),
      canonical.canonicalize().unwrap(),
    );
    std::fs::remove_dir_all(root).unwrap();
  }


  #[test]
  fn resolves_absolute_file_directory_index_extensionless_and_missing() {
    let root = temp_root("resolve-absolute-candidates");
    std::fs::create_dir_all(root.join("module")).unwrap();
    std::fs::write(root.join("main.mec"), "x := 1").unwrap();
    std::fs::write(root.join("module/index.mec"), "x := 2").unwrap();
    std::fs::write(root.join("other.mec"), "x := 3").unwrap();
    let resolver = FileSourceResolver::new(&root);

    let main = resolver.resolve(&SourceRequest::new(root.join("main.mec").to_string_lossy().to_string())).unwrap().unwrap();
    assert_eq!(main.name, "main.mec");

    let module = resolver.resolve(&SourceRequest::new(root.join("module").to_string_lossy().to_string())).unwrap().unwrap();
    assert_eq!(module.name, "index.mec");

    let other = resolver.resolve(&SourceRequest::new(root.join("other").to_string_lossy().to_string())).unwrap().unwrap();
    assert_eq!(other.name, "other.mec");

    assert!(resolver.resolve(&SourceRequest::new(root.join("missing").to_string_lossy().to_string())).unwrap().is_none());
    std::fs::remove_dir_all(root).unwrap();
  }

}

#[cfg(test)]
mod capability_tests {
  use super::*;
  use crate::{HostFilesystemAuthority, SequentialIdGenerator, MECH_TOOL_SUBJECT, SERVE_HOST_SUBJECT, FS_IMPORT, FS_READ, FS_RESOLVE};

  fn temp_root(label: &str) -> PathBuf { let root = std::env::temp_dir().join(format!("mech-resolver-capability-{}-{}", label, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())); std::fs::create_dir_all(&root).unwrap(); root.canonicalize().unwrap() }
  fn resolver(root: &Path, allowed: &Path) -> FileSourceResolver { let mut ids = SequentialIdGenerator::new(); let mut authority = HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, SharedCapabilityKernel::new()); authority.grant_path(&mut ids, allowed, true, [FS_READ, FS_RESOLVE, FS_IMPORT]).unwrap(); authority.delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, allowed, true, [FS_READ, FS_RESOLVE, FS_IMPORT]).unwrap(); FileSourceResolver::new(root).with_capabilities(authority.kernel().clone(), SERVE_HOST_SUBJECT) }

  #[test]
  fn denies_source_outside_grant() { let root = temp_root("outside"); let allowed = root.join("allowed"); let outside = root.join("outside"); std::fs::create_dir_all(&allowed).unwrap(); std::fs::create_dir_all(&outside).unwrap(); std::fs::write(allowed.join("main.mec"), "x := 1\n").unwrap(); std::fs::write(outside.join("secret.mec"), "x := 2\n").unwrap(); let resolver = resolver(&root, &allowed); assert!(resolver.resolve(&SourceRequest::new("allowed/main.mec")).unwrap().is_some()); assert!(resolver.resolve(&SourceRequest::new("outside/secret.mec")).is_err()); std::fs::remove_dir_all(root).unwrap(); }

  #[test]
  fn denies_include_outside_grant() { let root = temp_root("include"); let allowed = root.join("allowed"); let outside = root.join("outside"); std::fs::create_dir_all(&allowed).unwrap(); std::fs::create_dir_all(&outside).unwrap(); std::fs::write(allowed.join("main.mec"), "{../outside/secret.mec}\n").unwrap(); std::fs::write(outside.join("secret.mec"), "x := 2\n").unwrap(); let resolver = resolver(&root, &allowed); assert!(resolver.resolve(&SourceRequest::new("allowed/main.mec")).is_err()); std::fs::remove_dir_all(root).unwrap(); }

  #[test]
  fn imported_source_requires_import_capability_for_each_include() {
    let root = temp_root("nested-import-include");
    let allowed = root.join("allowed");
    std::fs::create_dir_all(&allowed).unwrap();

    let main = allowed.join("main.mec");
    let child = allowed.join("child.mec");
    let grandchild = allowed.join("grandchild.mec");
    std::fs::write(&main, "{child.mec}\n").unwrap();
    std::fs::write(&child, "{grandchild.mec}\n").unwrap();
    std::fs::write(&grandchild, "x := 1\n").unwrap();

    let mut ids = SequentialIdGenerator::new();
    let mut authority = HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, SharedCapabilityKernel::new());
    authority.grant_path(&mut ids, &allowed, true, [FS_READ, FS_RESOLVE]).unwrap();
    authority.grant_path(&mut ids, &main, false, [FS_IMPORT]).unwrap();
    authority.grant_path(&mut ids, &child, false, [FS_IMPORT]).unwrap();
    authority.delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &allowed, true, [FS_READ, FS_RESOLVE]).unwrap();
    authority.delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &main, false, [FS_IMPORT]).unwrap();
    authority.delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &child, false, [FS_IMPORT]).unwrap();
    let resolver = FileSourceResolver::new(&root)
      .with_capabilities(authority.kernel().clone(), SERVE_HOST_SUBJECT);

    assert!(resolver.resolve(&SourceRequest::new("allowed/main.mec")).is_ok());
    assert!(resolver
      .resolve(&SourceRequest::new("allowed/main.mec").with_referrer("memory://parent.mec"))
      .is_err());

    std::fs::remove_dir_all(root).unwrap();
  }
}
