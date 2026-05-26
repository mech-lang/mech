use mech_core::{MResult, MechError, MechErrorKind};

use super::{SourceImportDeclaration, SourceImportKind, SourceRequest};

#[derive(Debug, Clone)]
pub struct InvalidSourceImportDeclaration {
  pub declaration: String,
  pub reason: String,
}

impl MechErrorKind for InvalidSourceImportDeclaration {
  fn name(&self) -> &str {
    "InvalidSourceImportDeclaration"
  }

  fn message(&self) -> String {
    format!(
      "Invalid source import declaration `{}`: {}",
      self.declaration,
      self.reason,
    )
  }
}

pub fn extract_mech_imports(source: &str) -> MResult<Vec<SourceImportDeclaration>> {
  let mut imports = Vec::new();

  for line in source.lines() {
    let trimmed = line.trim();

    if !trimmed.starts_with("++") {
      continue;
    }

    let declaration = trimmed[2..].trim();

    if declaration.is_empty() {
      continue;
    }

    if let Some((lhs_raw, rhs_raw)) = declaration.split_once(":=") {
      let lhs = lhs_raw.trim();
      let rhs = rhs_raw.trim();

      if looks_like_source_specifier(rhs) {
        imports.push(parse_specifier(rhs, Some(lhs.to_string()), declaration)?);
      }

      continue;
    }

    imports.push(parse_specifier(declaration, None, declaration)?);
  }

  Ok(imports)
}

pub fn import_dependencies(imports: &[SourceImportDeclaration]) -> Vec<SourceRequest> {
  imports
    .iter()
    .map(|import| SourceRequest::new(import.specifier.clone()))
    .collect()
}

pub fn strip_mech_import_declarations(source: &str) -> String {
  let mut stripped = String::new();

  for line in source.split_inclusive('\n') {
    if line.trim_start().starts_with("++") {
      continue;
    }

    stripped.push_str(line);
  }

  stripped
}

fn parse_specifier(
  specifier: &str,
  alias: Option<String>,
  declaration: &str,
) -> MResult<SourceImportDeclaration> {
  let specifier = specifier.trim();

  if specifier.contains("/*/") || specifier.starts_with("*/") {
    return Err(MechError::new(
      InvalidSourceImportDeclaration {
        declaration: declaration.to_string(),
        reason: "wildcard must be in the final segment".to_string(),
      },
      None,
    ));
  }

  let kind = if let Some(prefix) = specifier.strip_suffix("/*") {
    if prefix.is_empty() || prefix.ends_with('/') {
      return Err(MechError::new(
        InvalidSourceImportDeclaration {
          declaration: declaration.to_string(),
          reason: "wildcard must be in the final segment".to_string(),
        },
        None,
      ));
    }

    return Ok(SourceImportDeclaration {
      specifier: prefix.to_string(),
      alias,
      kind: SourceImportKind::Wildcard,
    });
  } else if is_remote_or_path_like(specifier) {
    SourceImportKind::DependencyOnly
  } else if let Some((module, name)) = split_single_import(specifier) {
    return Ok(SourceImportDeclaration {
      specifier: module.to_string(),
      alias,
      kind: SourceImportKind::Single {
        name: name.to_string(),
      },
    });
  } else {
    SourceImportKind::Namespace
  };

  Ok(SourceImportDeclaration {
    specifier: specifier.to_string(),
    alias,
    kind,
  })
}

fn split_single_import(specifier: &str) -> Option<(&str, &str)> {
  if specifier.contains("://") || specifier.starts_with("./") || specifier.starts_with("../") || specifier.ends_with(".mec") {
    return None;
  }

  let (module, name) = specifier.rsplit_once('/')?;

  if module.is_empty() || name.is_empty() || !is_bare_module_name(module) || !is_bare_module_name(name) {
    return None;
  }

  Some((module, name))
}

fn is_remote_or_path_like(specifier: &str) -> bool {
  specifier.contains("://")
    || specifier.starts_with("./")
    || specifier.starts_with("../")
    || specifier.ends_with(".mec")
}

fn looks_like_source_specifier(text: &str) -> bool {
  text.contains('/')
    || text.contains("://")
    || text.starts_with("./")
    || text.starts_with("../")
    || text.ends_with(".mec")
    || is_bare_module_name(text)
}

fn is_bare_module_name(text: &str) -> bool {
  if text.is_empty() {
    return false;
  }

  text
    .split('/')
    .all(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extracts_namespace_import() {
    let imports = extract_mech_imports("++ math\nx := 1").unwrap();
    assert_eq!(imports[0].specifier, "math");
    assert_eq!(imports[0].kind, SourceImportKind::Namespace);
  }

  #[test]
  fn extracts_single_import() {
    let imports = extract_mech_imports("++ math/sin\n").unwrap();
    assert_eq!(imports[0].specifier, "math");
    assert_eq!(imports[0].kind, SourceImportKind::Single { name: "sin".to_string() });
  }

  #[test]
  fn extracts_wildcard_import() {
    let imports = extract_mech_imports("++ math/*\n").unwrap();
    assert_eq!(imports[0].specifier, "math");
    assert_eq!(imports[0].kind, SourceImportKind::Wildcard);
  }

  #[test]
  fn strips_import_lines() {
    let stripped = strip_mech_import_declarations("++ ./dep.mec\nx := 42\nx\n");
    assert_eq!(stripped.trim(), "x := 42\nx");
  }
}
