use mech_core::Program;

use super::{
  source_request_for_import, SourceContextDeclaration, SourceExportDeclaration,
  SourceImportDeclaration, SourceIndex, SourceRequest,
};

pub fn imports_from_program(tree: &Program) -> Vec<SourceImportDeclaration> {
  SourceIndex::from_program(tree).all_imports()
}

pub fn exports_from_program(tree: &Program) -> Vec<SourceExportDeclaration> {
  SourceIndex::from_program(tree).all_exports()
}

pub fn dependencies_from_program(tree: &Program, referrer: Option<&str>) -> Vec<SourceRequest> {
  SourceIndex::from_program(tree)
    .all_imports()
    .iter()
    .map(|import| source_request_for_import(import, referrer))
    .collect()
}


pub fn contexts_from_program(tree: &Program) -> Vec<SourceContextDeclaration> {
  SourceIndex::from_program(tree).all_contexts()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::resolver::{
    FileSourceResolver, SourceContextBase, SourceContextCapabilityScope, SourceDeclaration,
    SourceRequest, SourceResolver, SourceScope,
  };

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

  #[test]
  fn source_index_collects_program_declarations() {
    let tree = parse_program("@main := db://main{:read(users/*)}\n+> ./math.mec\n<+ tau\n");
    let index = SourceIndex::from_program(&tree);
    assert_eq!(index.contexts.len(), 1);
    assert_eq!(index.imports.len(), 1);
    assert_eq!(index.exports.len(), 1);
    assert_eq!(index.contexts[0].occurrence.scope, SourceScope::Program);
    assert_eq!(index.imports[0].occurrence.scope, SourceScope::Program);
    assert_eq!(index.exports[0].occurrence.scope, SourceScope::Program);
    assert!(matches!(index.declarations[0], SourceDeclaration::Context(_)));
    assert!(matches!(index.declarations[1], SourceDeclaration::Import(_)));
    assert!(matches!(index.declarations[2], SourceDeclaration::Export(_)));
  }

  #[test]
  fn legacy_helpers_match_source_index_all_views() {
    let tree = parse_program("@doc := db://doc{:read(*)}\n+> ./doc.mec\n\n~~~mech:foo\n@foo-db := db://foo{:read(*)}\n+> ./foo.mec\n<+ foo-result\n~~~\n");
    let index = SourceIndex::from_program(&tree);
    assert_eq!(imports_from_program(&tree), index.all_imports());
    assert_eq!(exports_from_program(&tree), index.all_exports());
    assert_eq!(contexts_from_program(&tree), index.all_contexts());
  }

  #[test]
  fn file_resolver_uses_index_without_behavior_change() {
    let tmp = std::env::temp_dir().join(format!("mech-source-index-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("main.mec");
    std::fs::write(&path, "@doc := db://doc{:read(*)}\n+> ./doc.mec\n\n~~~mech:foo\n@foo-db := db://foo{:read(*)}\n+> ./foo.mec\n<+ foo-result\n~~~\n").unwrap();
    let tree = parse_program(&std::fs::read_to_string(&path).unwrap());
    let index = SourceIndex::from_program(&tree);
    let resolver = FileSourceResolver::new(&tmp);
    let resolved = resolver.resolve(&SourceRequest::new("main.mec")).unwrap().unwrap();
    assert_eq!(resolved.imports, index.all_imports());
    assert_eq!(resolved.exports, index.all_exports());
    assert_eq!(resolved.contexts, index.all_contexts());
  }
}
