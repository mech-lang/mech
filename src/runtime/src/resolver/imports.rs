use mech_core::FencedMechCode;

use super::{SourceExportDeclaration, SourceImportDeclaration, SourceImportKind, SourceRequest};

pub fn import_dependencies(imports: &[SourceImportDeclaration]) -> Vec<SourceRequest> {
  imports
    .iter()
    .filter(|import| matches!(import.kind, SourceImportKind::DependencyOnly))
    .map(|import| SourceRequest::new(import.specifier.clone()))
    .collect()
}

pub fn imports_from_fenced_code(code: &FencedMechCode) -> Vec<SourceImportDeclaration> {
  code.imports.iter().map(|import| {
    let specifier = import.specifier.to_string();
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
  }).collect()
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
  fn dependency_edges_only_for_dependency_only_imports() {
    let fenced = parse_fenced("~~~mech\n+> math\n+> math/sin\n+> math/*\n+> ./dep.mec\n~~~\n");
    let imports = imports_from_fenced_code(&fenced);
    let dependencies = import_dependencies(&imports);
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].specifier, "./dep.mec");
  }
}
