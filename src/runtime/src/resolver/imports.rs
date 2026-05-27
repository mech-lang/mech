use mech_core::FencedMechCode;

use super::{SourceExportDeclaration, SourceImportDeclaration, SourceImportKind, SourceRequest};

pub fn classify_import_specifier(specifier: impl Into<String>) -> SourceImportDeclaration {
  let specifier = specifier.into();
  if let Some(prefix) = specifier.strip_suffix("/*") {
    SourceImportDeclaration { specifier: prefix.to_string(), alias: None, kind: SourceImportKind::Wildcard }
  } else if specifier.contains("://")
    || specifier.starts_with("./")
    || specifier.starts_with("../")
    || specifier.ends_with(".mec")
  {
    SourceImportDeclaration { specifier, alias: None, kind: SourceImportKind::DependencyOnly }
  } else if let Some((module, name)) = specifier.rsplit_once('/') {
    SourceImportDeclaration { specifier: module.to_string(), alias: None, kind: SourceImportKind::Single { name: name.to_string() } }
  } else {
    SourceImportDeclaration { specifier, alias: None, kind: SourceImportKind::Namespace }
  }
}

pub fn module_namespace_for_import(import: &SourceImportDeclaration) -> Option<String> {
  fn stem_from_specifier(specifier: &str) -> Option<String> {
    let trimmed = specifier.trim_end_matches('/');
    let candidate = trimmed.rsplit('/').next().unwrap_or(trimmed);
    let candidate = candidate.strip_suffix(".mec").unwrap_or(candidate);
    if candidate.is_empty() { None } else { Some(candidate.to_string()) }
  }

  if import.specifier.trim().is_empty() {
    return None;
  }

  match &import.kind {
    SourceImportKind::Single { .. } | SourceImportKind::Wildcard | SourceImportKind::Namespace => {
      stem_from_specifier(&import.specifier)
    }
    SourceImportKind::DependencyOnly => {
      let spec = import.specifier.trim();
      if let Some((_, path_part)) = spec.rsplit_once("://") {
        stem_from_specifier(path_part)
      } else {
        stem_from_specifier(spec)
      }
    }
  }
}

pub fn normalize_import_specifier(raw: &str) -> String {
  raw.trim().strip_suffix("/*").unwrap_or(raw.trim()).to_string()
}

pub fn source_request_for_import(
  import: &SourceImportDeclaration,
  referrer: Option<&str>,
) -> SourceRequest {
  let mut request = SourceRequest::new(normalize_import_specifier(&import.specifier));
  if let Some(referrer) = referrer {
    request = request.with_referrer(referrer.to_string());
  }
  request
}

pub fn import_dependencies(imports: &[SourceImportDeclaration]) -> Vec<SourceRequest> {
  imports
    .iter()
    .map(|import| source_request_for_import(import, None))
    .collect()
}

pub fn imports_from_fenced_code(code: &FencedMechCode) -> Vec<SourceImportDeclaration> {
  code.imports.iter().map(|import| classify_import_specifier(import.specifier.to_string())).collect()
}

pub fn exports_from_fenced_code(code: &FencedMechCode) -> Vec<SourceExportDeclaration> {
  code.exports.iter().map(|export| SourceExportDeclaration {
    name: export.name.to_string(),
  }).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use mech_syntax::parser;

  fn parse_fenced(source: &str) -> FencedMechCode {
    let tree = parser::parse(source).unwrap();
    for section in &tree.body.sections {
      for element in &section.elements {
        if let mech_core::SectionElement::FencedMechCode(code) = element {
          return code.clone();
        }
      }
    }
    panic!("expected fenced code block");
  }

  #[test]
  fn classifies_single_import() {
    let fenced = parse_fenced("~~~mech\n+> math/sin\n~~~\n");
    let imports = imports_from_fenced_code(&fenced);
    assert_eq!(imports[0].specifier, "math");
    assert_eq!(imports[0].kind, SourceImportKind::Single { name: "sin".to_string() });
  }

  #[test]
  fn classifies_wildcard_import() {
    let fenced = parse_fenced("~~~mech\n+> math/*\n~~~\n");
    let imports = imports_from_fenced_code(&fenced);
    assert_eq!(imports[0].specifier, "math");
    assert_eq!(imports[0].kind, SourceImportKind::Wildcard);
  }

  #[test]
  fn classifies_dependency_only_imports() {
    let fenced = parse_fenced("~~~mech\n+> ./dep.mec\n+> fs://lib/dep.mec\n+> file:///tmp/dep.mec\n+> memory://scratch/dep\n+> https://example.com/dep.mec\n~~~\n");
    let imports = imports_from_fenced_code(&fenced);
    assert!(imports.iter().all(|imp| imp.kind == SourceImportKind::DependencyOnly));
  }

  #[test]
  fn exports_are_extracted() {
    let fenced = parse_fenced("~~~mech\n<+ area\n~~~\n");
    let exports = exports_from_fenced_code(&fenced);
    assert_eq!(exports[0].name, "area");
  }

  #[test]
  fn all_imports_create_dependency_edges() {
    let fenced = parse_fenced("~~~mech\n+> math\n+> math/sin\n+> math/*\n+> ./dep.mec\n~~~\n");
    let imports = imports_from_fenced_code(&fenced);
    let dependencies = import_dependencies(&imports);
    assert_eq!(dependencies.len(), 4);
    assert_eq!(dependencies[0].specifier, "math");
    assert_eq!(dependencies[1].specifier, "math");
    assert_eq!(dependencies[2].specifier, "math");
    assert_eq!(dependencies[3].specifier, "./dep.mec");
  }

  #[test]
  fn namespace_for_relative_file_import() {
    let import = classify_import_specifier("./math.mec");
    assert_eq!(module_namespace_for_import(&import), Some("math".to_string()));
  }

  #[test]
  fn namespace_for_parent_relative_file_import() {
    let import = classify_import_specifier("../lib/math.mec");
    assert_eq!(module_namespace_for_import(&import), Some("math".to_string()));
  }

  #[test]
  fn namespace_for_namespace_import() {
    let import = classify_import_specifier("math");
    assert_eq!(module_namespace_for_import(&import), Some("math".to_string()));
  }

  #[test]
  fn namespace_for_single_import() {
    let import = classify_import_specifier("math/tau");
    assert_eq!(module_namespace_for_import(&import), Some("math".to_string()));
  }

  #[test]
  fn namespace_for_wildcard_import() {
    let import = classify_import_specifier("math/*");
    assert_eq!(module_namespace_for_import(&import), Some("math".to_string()));
  }
}
