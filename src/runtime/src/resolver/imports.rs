use mech_core::{MResult, Program, MechCode, Statement};
use mech_syntax::parser;

use super::{SourceImportDeclaration, SourceImportKind, SourceRequest};

pub fn extract_mech_imports(source: &str) -> MResult<Vec<SourceImportDeclaration>> {
  let program = parser::parse(source)?;
  Ok(extract_imports_from_program(&program))
}

pub fn import_dependencies(imports: &[SourceImportDeclaration]) -> Vec<SourceRequest> {
  imports.iter().map(|i| SourceRequest::new(i.specifier.clone())).collect()
}

pub fn strip_mech_import_declarations(source: &str) -> String { source.to_string() }

fn extract_imports_from_program(program: &Program) -> Vec<SourceImportDeclaration> {
  let mut out = Vec::new();
  for section in &program.body.sections {
    for element in &section.elements {
      if let mech_core::SectionElement::MechCode(lines) = element {
        for (code, _) in lines {
          if let MechCode::Statement(stmt) = code {
            if let Statement::ImportDeclaration(import) = stmt {
              let raw = import.module.to_string();
              let spec = raw.trim_start_matches("+>").trim();
              if let Some(prefix) = spec.strip_suffix("/*") {
                out.push(SourceImportDeclaration { specifier: prefix.to_string(), alias: None, kind: SourceImportKind::Wildcard });
              } else if spec.contains("://") || spec.starts_with("./") || spec.starts_with("../") || spec.ends_with(".mec") || spec.starts_with("fs:") {
                out.push(SourceImportDeclaration { specifier: spec.to_string(), alias: None, kind: SourceImportKind::DependencyOnly });
              } else if let Some((module, name)) = spec.rsplit_once('/') {
                out.push(SourceImportDeclaration { specifier: module.to_string(), alias: None, kind: SourceImportKind::Single { name: name.to_string() } });
              } else {
                out.push(SourceImportDeclaration { specifier: spec.to_string(), alias: None, kind: SourceImportKind::Namespace });
              }
            }
          }
        }
      }
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extracts_namespace_import() {
    let imports = extract_mech_imports("+> math
x := 1").unwrap();
    assert_eq!(imports[0].specifier, "math");
  }

  #[test]
  fn extracts_single_import() {
    let imports = extract_mech_imports("+> math/sin
").unwrap();
    assert_eq!(imports[0].specifier, "math");
  }
}
