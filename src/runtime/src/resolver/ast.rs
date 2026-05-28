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
    use crate::resolver::{FileSourceResolver, SourceResolver, SourceScope};

    fn parse_program(source: &str) -> Program {
        mech_syntax::parser::parse(source).unwrap()
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
        assert!(matches!(
            index.declarations[0],
            crate::resolver::SourceDeclaration::Context(_)
        ));
        assert!(matches!(
            index.declarations[1],
            crate::resolver::SourceDeclaration::Import(_)
        ));
        assert!(matches!(
            index.declarations[2],
            crate::resolver::SourceDeclaration::Export(_)
        ));
    }

    #[test]
    fn source_index_collects_fenced_interpreter_declarations() {
        let tree = parse_program(
            "~~~mech:foo\n@main := db://main{:read(users/*)}\n+> math/tau\n<+ result\n~~~\n",
        );
        let index = SourceIndex::from_program(&tree);
        let scopes = index.interpreter_scopes();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].namespace_str, "foo");
        let foo_scope = SourceScope::Interpreter(scopes[0].clone());
        assert_eq!(index.contexts_for_scope(&foo_scope).len(), 1);
        assert_eq!(index.imports_for_scope(&foo_scope).len(), 1);
        assert_eq!(index.exports_for_scope(&foo_scope).len(), 1);
        assert!(index.program_contexts().is_empty());
        assert!(index.program_imports().is_empty());
        assert!(index.program_exports().is_empty());
    }

    #[test]
    fn source_index_keeps_program_and_interpreter_scopes_separate() {
        let tree = parse_program(
            "@doc := db://doc{:read(*)}\n+> ./doc.mec\n\n~~~mech:foo\n@foo-db := db://foo{:read(*)}\n+> ./foo.mec\n<+ foo-result\n~~~\n",
        );
        let index = SourceIndex::from_program(&tree);
        assert_eq!(index.program_contexts()[0].name, "doc");
        assert_eq!(index.program_imports()[0].specifier, "./doc.mec");
        let foo = index
            .interpreter_scopes()
            .into_iter()
            .find(|s| s.namespace_str == "foo")
            .unwrap();
        let foo_scope = SourceScope::Interpreter(foo);
        assert_eq!(index.contexts_for_scope(&foo_scope)[0].name, "foo-db");
        assert_eq!(
            index.imports_for_scope(&foo_scope)[0].specifier,
            "./foo.mec"
        );
        assert_eq!(index.exports_for_scope(&foo_scope)[0].name, "foo-result");
        assert_eq!(index.all_imports().len(), 2);
        assert_eq!(index.all_contexts().len(), 2);
        assert_eq!(index.all_exports().len(), 1);
    }

    #[test]
    fn legacy_helpers_match_source_index_all_views() {
        let tree = parse_program(
            "@doc := db://doc{:read(*)}\n+> ./doc.mec\n~~~mech:foo\n@foo-db := db://foo{:read(*)}\n+> ./foo.mec\n<+ foo-result\n~~~\n",
        );
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
        std::fs::write(&path, "@doc := db://doc{:read(*)}\n+> ./doc.mec\n~~~mech:foo\n@foo-db := db://foo{:read(*)}\n+> ./foo.mec\n<+ foo-result\n~~~\n").unwrap();
        let tree = parse_program(&std::fs::read_to_string(&path).unwrap());
        let index = SourceIndex::from_program(&tree);
        let resolver = FileSourceResolver::new(&tmp);
        let resolved = resolver
            .resolve(&crate::resolver::SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();
        assert_eq!(resolved.imports, index.all_imports());
        assert_eq!(resolved.exports, index.all_exports());
        assert_eq!(resolved.contexts, index.all_contexts());
    }
}
