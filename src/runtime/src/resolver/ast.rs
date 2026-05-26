use mech_core::{MechCode, Program, SectionElement, Statement};

use super::{
  classify_import_specifier, exports_from_fenced_code, imports_from_fenced_code,
  source_request_for_import, SourceExportDeclaration, SourceImportDeclaration,
  SourceRequest,
};

pub fn imports_from_program(tree: &Program) -> Vec<SourceImportDeclaration> {
  let mut imports = Vec::new();
  for section in &tree.body.sections {
    for element in &section.elements {
      match element {
        SectionElement::FencedMechCode(code) => imports.extend(imports_from_fenced_code(code)),
        SectionElement::MechCode(code_items) => {
          for (code, _) in code_items {
            if let MechCode::Statement(Statement::ImportDeclaration(import)) = code {
              imports.push(classify_import_specifier(import.specifier.to_string()));
            }
          }
        }
        _ => {}
      }
    }
  }
  imports
}

pub fn exports_from_program(tree: &Program) -> Vec<SourceExportDeclaration> {
  let mut exports = Vec::new();
  for section in &tree.body.sections {
    for element in &section.elements {
      match element {
        SectionElement::FencedMechCode(code) => exports.extend(exports_from_fenced_code(code)),
        SectionElement::MechCode(code_items) => {
          for (code, _) in code_items {
            if let MechCode::Statement(Statement::ExportDeclaration(export)) = code {
              exports.push(SourceExportDeclaration {
                name: export.name.to_string(),
              });
            }
          }
        }
        _ => {}
      }
    }
  }
  exports
}

pub fn dependencies_from_program(tree: &Program, referrer: Option<&str>) -> Vec<SourceRequest> {
  imports_from_program(tree)
    .iter()
    .map(|import| source_request_for_import(import, referrer))
    .collect()
}
