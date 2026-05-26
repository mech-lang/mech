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
    .filter(|import| matches!(import.kind, SourceImportKind::DependencyOnly))
    .map(|import| SourceRequest::new(import.specifier.clone()))
    .collect()
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

        let specifier = import.specifier.to_string();

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
  use mech_core::{MechCode, SectionElement, Statement, Value};
  use mech_program::MechProgram;
  use mech_syntax::Formatter;
  use mech_syntax::parser;

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

  #[test]
  fn extracts_uri_dependency_import() {
    let imports = extract_mech_imports("+> fs://foo/bar.mec\n+> memory://foo/bar\n").unwrap();
    assert_eq!(imports[0].kind, SourceImportKind::DependencyOnly);
    assert_eq!(imports[0].specifier, "fs://foo/bar.mec");
    assert_eq!(imports[1].kind, SourceImportKind::DependencyOnly);
    assert_eq!(imports[1].specifier, "memory://foo/bar");
  }

  #[test]
  fn creates_dependency_for_relative_file_import_only() {
    let imports = extract_mech_imports("+> ./dep.mec\n").unwrap();
    let dependencies = import_dependencies(&imports);
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].specifier, "./dep.mec");
  }

  #[test]
  fn non_dependency_imports_do_not_create_dependency_edges() {
    let imports = extract_mech_imports("+> math\n+> math/sin\n+> math/*\n").unwrap();
    let dependencies = import_dependencies(&imports);
    assert_eq!(imports.len(), 3);
    assert_eq!(dependencies.len(), 0);
  }

  #[test]
  fn invalid_wildcard_placements_are_rejected() {
    assert!(extract_mech_imports("+> module/*/x\n").is_err());
    assert!(extract_mech_imports("+> module/s*\n").is_err());
    assert!(extract_mech_imports("+> *\n").is_err());
  }

  #[test]
  fn export_declaration_parses_and_executes_as_noop() {
    let tree = parser::parse("<+ foo\nx := 1\nx\n").unwrap();
    let mut saw_export = false;
    for section in &tree.body.sections {
      for element in &section.elements {
        if let SectionElement::MechCode(lines) = element {
          for (code, _) in lines {
            if let MechCode::Statement(Statement::ExportDeclaration(_)) = code {
              saw_export = true;
            }
          }
        }
      }
    }
    assert!(saw_export);
    let mut program = MechProgram::new(Default::default());
    let value = program.run_string("<+ foo\nx := 1\nx\n").unwrap();
    assert_ne!(value, Value::Empty);
  }

  #[test]
  fn formatter_renders_import_and_export_declarations() {
    let tree = parser::parse("+> math/sin\n<+ foo\n").unwrap();
    let mut formatter = Formatter::new();
    let rendered = formatter.format(&tree);
    assert!(rendered.contains("+> math/sin"));
    assert!(rendered.contains("<+ foo"));
  }
}
