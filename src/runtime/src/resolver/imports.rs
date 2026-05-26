use mech_core::{MResult, Program, SectionElement, MechCode, Statement};
use mech_syntax::parser;

use super::{SourceImportDeclaration, SourceImportKind, SourceRequest};

pub fn extract_mech_imports(source: &str) -> MResult<Vec<SourceImportDeclaration>> {
  let program = parser::parse(source)?;
  Ok(extract_imports_from_program(&program))
}

pub fn import_dependencies(imports: &[SourceImportDeclaration]) -> Vec<SourceRequest> {
  imports
    .iter()
    .map(|import| SourceRequest::new(import.specifier.clone()))
    .collect()
}

pub fn strip_mech_import_declarations(source: &str) -> String {
  source.to_string()
}

fn extract_imports_from_program(program: &Program) -> Vec<SourceImportDeclaration> {
  let mut imports = Vec::new();

  for section in &program.body.sections {
    for element in &section.elements {
      let SectionElement::MechCode(lines) = element else {
        continue;
      };

      for (code, _) in lines {
        let MechCode::Statement(statement) = code else {
          continue;
        };

        let Statement::ImportDeclaration(import) = statement else {
          continue;
        };

        let raw = import.module.to_string();
        let specifier = raw.trim_start_matches("+>").trim();

        if let Some(prefix) = specifier.strip_suffix("/*") {
          imports.push(SourceImportDeclaration {
            specifier: prefix.to_string(),
            alias: None,
            kind: SourceImportKind::Wildcard,
          });
        } else if specifier.contains("://")
          || specifier.starts_with("./")
          || specifier.starts_with("../")
          || specifier.ends_with(".mec")
          || specifier.starts_with("fs:")
        {
          imports.push(SourceImportDeclaration {
            specifier: specifier.to_string(),
            alias: None,
            kind: SourceImportKind::DependencyOnly,
          });
        } else if let Some((module, name)) = specifier.rsplit_once('/') {
          imports.push(SourceImportDeclaration {
            specifier: module.to_string(),
            alias: None,
            kind: SourceImportKind::Single {
              name: name.to_string(),
            },
          });
        } else {
          imports.push(SourceImportDeclaration {
            specifier: specifier.to_string(),
            alias: None,
            kind: SourceImportKind::Namespace,
          });
        }
      }
    }
  }

  imports
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extracts_namespace_import() {
    let imports = extract_mech_imports("+> math\nx := 1").unwrap();
    assert_eq!(imports[0].specifier, "math");
  }

  #[test]
  fn extracts_single_import() {
    let imports = extract_mech_imports("+> math/sin\n").unwrap();
    assert_eq!(imports[0].specifier, "math");
  }
}
