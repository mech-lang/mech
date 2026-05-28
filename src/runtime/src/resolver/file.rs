use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use mech_core::{MResult, MechError, MechSourceCode};

use crate::resolver::{
  contexts_from_program, exports_from_program, imports_from_program, source_request_for_import,
  ResolvedSource, SourceRequest, SourceResolver,
};

use super::{
  SourceExtensionDecodeFailed, SourceFileOpenFailed, SourceFileReadFailed,
  SourceIncludeCycle, SourceIncludeReadFailed, SourceKind,
  SourceUnknownFileExtension,
};

#[derive(Clone, Debug)]
pub struct FileSourceResolver {
  roots: Vec<PathBuf>,
}

impl FileSourceResolver {
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self {
      roots: vec![root.into()],
    }
  }

  pub fn empty() -> Self {
    Self {
      roots: Vec::new(),
    }
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
    let specifier = Path::new(&request.specifier);

    if specifier.is_absolute() && specifier.exists() {
      return Ok(Some(canonicalize_source_path(specifier)?));
    }

    if let Some(referrer) = &request.referrer {
      let referrer_path = Path::new(referrer);

      let parent = if referrer_path.is_dir() {
        Some(referrer_path)
      } else {
        referrer_path.parent()
      };

      if let Some(parent) = parent {
        for candidate in path_candidates(parent, specifier) {
          if candidate.is_file() {
            return Ok(Some(canonicalize_source_path(&candidate)?));
          }
        }
      }
    }

    for root in &self.roots {
      for candidate in path_candidates(root, specifier) {
        if candidate.is_file() {
          return Ok(Some(canonicalize_source_path(&candidate)?));
        }
      }
    }

    Ok(None)
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
    let source = read_runtime_source_file(&path)?;
    let name = path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or("source")
      .to_string();

    let mut resolved = ResolvedSource::new(name, file_uri(&path), source).with_kind(kind);

    if resolved.kind == SourceKind::Mech {
      if let MechSourceCode::String(source_text) = &resolved.source {
        let tree = mech_syntax::parser::parse(source_text.trim())?;
        let referrer = path.to_string_lossy().to_string();
        let imports = imports_from_program(&tree);
        let exports = exports_from_program(&tree);
        let contexts = contexts_from_program(&tree);
        let dependencies = imports
          .iter()
          .map(|import| source_request_for_import(import, Some(&referrer)))
          .collect::<Vec<_>>();

        resolved = resolved
          .with_imports(imports)
          .with_exports(exports)
          .with_contexts(contexts)
          .with_dependencies(dependencies);
      }
    }

    Ok(Some(resolved))
  }
}

fn path_candidates(base: &Path, specifier: &Path) -> Vec<PathBuf> {
  vec![
    base.join(specifier),
    base.join(format!("{}.mec", specifier.to_string_lossy())),
    base.join(specifier).join("index.mec"),
  ]
}

pub fn read_runtime_source_file(path: &Path) -> MResult<MechSourceCode> {
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
      let expanded = expand_mechdown_includes(path)?;
      Ok(MechSourceCode::String(expanded))
    }

    "mecb" => {
      // Keep this as raw bytecode source for now only if your MechSourceCode
      // supports ByteCode(Vec<u8>). If your current loader needs
      // `load_program_from_file`, wire that in module builder later.
      let bytes = read_file_bytes(path)?;
      Ok(MechSourceCode::ByteCode(bytes))
    }

    "html" | "htm" | "md" | "css" => {
      Ok(MechSourceCode::Html(read_file_string(path)?))
    }

    "mdoc" | "mpkg" | "m" | "csv" | "js" => {
      Ok(MechSourceCode::String(read_file_string(path)?))
    }

    "png" | "jpg" | "jpeg" | "gif" | "svg" => {
      Ok(MechSourceCode::Image(extension, read_file_bytes(path)?))
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

pub fn expand_mechdown_includes(path: &Path) -> MResult<String> {
  let canonical = canonicalize_source_path(path)?;
  let mut active = HashSet::new();

  expand_mechdown_includes_inner(&canonical, &mut active)
}

fn expand_mechdown_includes_inner(
  path: &Path,
  active: &mut HashSet<PathBuf>,
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

  let source = read_file_string(&canonical)?;
  let expanded = expand_mechdown_include_tokens(
    &source,
    &canonical,
    active,
  )?;

  active.remove(&canonical);

  Ok(expanded)
}

fn expand_mechdown_include_tokens(
  source: &str,
  canonical_path: &Path,
  active: &mut HashSet<PathBuf>,
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

        let include_source =
          expand_mechdown_includes_inner(&include_canonical, active)?;

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

fn file_uri(path: &Path) -> String {
  let mut text = path.display().to_string();

  if cfg!(windows) {
    text = text.replace('\\', "/");
  }

  format!("file://{}", text)
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

}
