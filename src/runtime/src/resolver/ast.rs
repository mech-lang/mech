use mech_core::{ContextBase, ContextCapabilityScope, MechCode, Program, SectionElement, Statement};

use super::{
  classify_import_specifier, exports_from_fenced_code, imports_from_fenced_code,
  source_request_for_import, SourceContextBase, SourceContextCapability,
  SourceContextCapabilityScope, SourceContextDeclaration, SourceExportDeclaration,
  SourceImportDeclaration, SourceRequest,
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


pub fn contexts_from_program(tree: &Program) -> Vec<SourceContextDeclaration> {
  let mut contexts = Vec::new();
  for section in &tree.body.sections {
    for element in &section.elements {
      if let SectionElement::MechCode(code_items) = element {
        for (code, _) in code_items {
          if let MechCode::Statement(Statement::ContextDeclaration(context)) = code {
            let base = match &context.base {
              ContextBase::ResourceUri(uri) => SourceContextBase::ResourceUri(uri.to_string()),
              ContextBase::Context(name) => SourceContextBase::Context(name.to_string()),
            };
            let capabilities = context.capabilities.iter().map(|capability| {
              let scope = match &capability.scope {
                ContextCapabilityScope::Path(path) => SourceContextCapabilityScope::Path(path.to_string()),
                ContextCapabilityScope::Wildcard(_) => SourceContextCapabilityScope::Wildcard,
              };
              SourceContextCapability {
                operation: capability.operation.to_string(),
                scope,
              }
            }).collect::<Vec<_>>();
            contexts.push(SourceContextDeclaration {
              name: context.name.to_string(),
              base,
              capabilities,
            });
          }
        }
      }
    }
  }
  contexts
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parse_program(source: &str) -> Program {
    mech_syntax::parser::parse(source).unwrap()
  }

  #[test]
  fn resolved_source_extracts_resource_context() {
    let tree = parse_program("@main := db://main{:read(users/*), :write(users/name)}\nx := users/name@main\n");
    let contexts = contexts_from_program(&tree);
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].name, "main");
    assert_eq!(contexts[0].base, SourceContextBase::ResourceUri("db://main".to_string()));
    assert_eq!(contexts[0].capabilities.len(), 2);
    assert_eq!(contexts[0].capabilities[0].operation, "read");
    assert_eq!(contexts[0].capabilities[0].scope, SourceContextCapabilityScope::Path("users/*".to_string()));
    assert_eq!(contexts[0].capabilities[1].operation, "write");
    assert_eq!(contexts[0].capabilities[1].scope, SourceContextCapabilityScope::Path("users/name".to_string()));
  }

  #[test]
  fn resolved_source_extracts_derived_context() {
    let tree = parse_program("@main := db://main{:read(users/*), :write(users/name)}\n@users := @main{:read(users/*)}\n");
    let contexts = contexts_from_program(&tree);
    assert_eq!(contexts.len(), 2);
    assert_eq!(contexts[0].base, SourceContextBase::ResourceUri("db://main".to_string()));
    assert_eq!(contexts[1].name, "users");
    assert_eq!(contexts[1].base, SourceContextBase::Context("main".to_string()));
  }

  #[test]
  fn resolved_source_extracts_wildcard_context_scope() {
    let tree = parse_program("@main := db://main{:write(*)}\n");
    let contexts = contexts_from_program(&tree);
    assert_eq!(contexts[0].capabilities[0].operation, "write");
    assert_eq!(contexts[0].capabilities[0].scope, SourceContextCapabilityScope::Wildcard);
  }

  #[test]
  fn context_extraction_does_not_break_import_export_extraction() {
    let tree = parse_program("@main := db://main{:read(users/*)}\n+> ./math.mec\n<+ tau\n");
    assert_eq!(contexts_from_program(&tree).len(), 1);
    assert_eq!(imports_from_program(&tree).len(), 1);
    assert_eq!(exports_from_program(&tree).len(), 1);
  }
}
