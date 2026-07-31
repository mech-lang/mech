use mech_core::Program;

use super::{
  import_requires_source_dependency, source_request_for_import, SourceContextDeclaration, SourceExportDeclaration,
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
    .filter(|import| import_requires_source_dependency(import))
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
    let tree = parse_program("@main := db://main{:read(users/*), :write(users/name)}\nx := @main/users/name\n");
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
    assert_eq!(index.all_address_references().len(), 0);
  }


  #[test]
  fn source_index_collects_program_address_reference() {
    let tree = parse_program("result := @foo/ok\n");
    let index = SourceIndex::from_program(&tree);
    let refs = index.program_address_references();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "ok");
    assert_eq!(refs[0].target, "foo");
  }

  #[test]
  fn source_index_collects_fenced_address_reference() {
    let tree = parse_program("~~~mech:foo\nresult := @bar/ok\n~~~\n");
    let index = SourceIndex::from_program(&tree);
    assert_eq!(index.program_address_references().len(), 0);
    let foo_scope = index.module_scopes().into_iter().find(|metadata| match &metadata.scope {
      SourceScope::Interpreter(interpreter) => interpreter.namespace_str == "foo",
      SourceScope::Program => false,
    }).unwrap();
    assert_eq!(foo_scope.address_references.len(), 1);
    assert_eq!(foo_scope.address_references[0].name, "ok");
    assert_eq!(foo_scope.address_references[0].target, "bar");
  }

  #[test]
  fn source_index_does_not_collect_string_address_text() {
    let tree = parse_program("text := \"@foo\"\n");
    let index = SourceIndex::from_program(&tree);
    assert_eq!(index.all_address_references().len(), 0);
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
    assert_eq!(resolved.address_references, index.all_address_references());
  }
  #[test]
  fn source_index_unions_repeated_fenced_interpreter_namespaces() {
    let tree = parse_program("~~~mech:bayes\nprior := 0.01\n~~~\n\n~~~mech:bayes\nposterior := prior\n~~~\n");
    let index = SourceIndex::from_program(&tree);
    assert!(index.validate_address_targets().is_ok());
    assert_eq!(index.address_target_interpreters.len(), 1);
    assert_eq!(index.address_target_interpreters[0].namespace_str, "bayes");
    assert_eq!(index.interpreter_scopes().len(), 1);
  }

  #[test]
  fn source_index_keeps_different_fenced_interpreter_namespaces_separate() {
    let tree = parse_program("~~~mech:foo\nx := 1\n~~~\n\n~~~mech:bar\nx := 2\n~~~\n");
    let index = SourceIndex::from_program(&tree);
    assert!(index.validate_address_targets().is_ok());
    let namespaces = index.interpreter_scopes().into_iter().map(|scope| scope.namespace_str).collect::<Vec<_>>();
    assert_eq!(namespaces, vec!["foo", "bar"]);
  }

}
