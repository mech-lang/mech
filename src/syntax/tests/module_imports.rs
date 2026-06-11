use mech_core::nodes::*;
use mech_syntax::parser;

fn imports(src: &str) -> Vec<ModuleImport> {
    let program = parser::parse(src).expect("parse failed");
    let mut out = vec![];
    for section in &program.body.sections {
        for element in &section.elements {
            if let SectionElement::MechCode(codes) = element {
                for (node, _) in codes {
                    if let MechCode::Import(import) = node {
                        out.push(import.clone());
                    }
                }
            }
        }
    }
    out
}

fn statements(src: &str) -> Vec<Statement> {
    let program = parser::parse(src).expect("parse failed");
    let mut out = vec![];
    for section in &program.body.sections {
        for element in &section.elements {
            if let SectionElement::MechCode(codes) = element {
                for (node, _) in codes {
                    if let MechCode::Statement(stmt) = node {
                        out.push(stmt.clone());
                    }
                }
            }
        }
    }
    out
}

fn item_path(import: &ModuleImport) -> Vec<String> {
    import
        .item
        .as_ref()
        .unwrap()
        .iter()
        .map(|id| id.to_string())
        .collect()
}

#[test]
fn parses_module_item_glob_and_nested_item_imports() {
    let parsed = imports("+> math\n+> math/sin\n+> math/*\n+> stats/sum/column");
    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0].kind, ModuleImportKind::Module);
    assert_eq!(parsed[0].module.to_string(), "math");
    assert!(parsed[0].item.is_none());
    assert_eq!(parsed[1].kind, ModuleImportKind::Item);
    assert_eq!(parsed[1].module.to_string(), "math");
    assert_eq!(item_path(&parsed[1]), vec!["sin"]);
    assert_eq!(parsed[2].kind, ModuleImportKind::Glob);
    assert_eq!(parsed[2].module.to_string(), "math");
    assert!(parsed[2].item.is_none());
    assert_eq!(parsed[3].kind, ModuleImportKind::Item);
    assert_eq!(parsed[3].module.to_string(), "stats");
    assert_eq!(item_path(&parsed[3]), vec!["sum", "column"]);
}

#[test]
fn preserves_source_import_declarations() {
    let stmts = statements(
        "+> ./dep.mec\n+> ../lib/dep.mec\n+> fs://lib/dep.mec\n+> file:///tmp/dep.mec\n+> memory://scratch/dep\n+> https://example.com/dep.mec",
    );
    assert_eq!(stmts.len(), 6);
    assert!(stmts
        .iter()
        .all(|stmt| matches!(stmt, Statement::ImportDeclaration(_))));
}

#[test]
fn unknown_module_roots_fall_back_to_source_imports() {
    let stmts = statements("+> userlib\n+> userlib/tool");
    assert_eq!(stmts.len(), 2);
    assert!(stmts
        .iter()
        .all(|stmt| matches!(stmt, Statement::ImportDeclaration(_))));
}

#[test]
fn rejects_invalid_stdlib_import_paths() {
    assert!(parser::parse("+> ").is_err());
    assert!(parser::parse("+> */x").is_err());
    assert!(parser::parse("+> math/").is_err());
    assert!(parser::parse("+> math/*/x").is_err());
}
