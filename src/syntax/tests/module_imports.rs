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

#[test]
fn parses_module_item_and_glob_imports() {
    let parsed = imports("+> math\n+> math/sin\n+> math/*\n+> stats/sum/column");
    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0].kind, ModuleImportKind::Module);
    assert_eq!(parsed[0].module.to_string(), "math");
    assert!(parsed[0].item.is_none());
    assert_eq!(parsed[1].kind, ModuleImportKind::Item);
    assert_eq!(parsed[1].module.to_string(), "math");
    assert_eq!(parsed[1].item.as_ref().unwrap()[0].to_string(), "sin");
    assert_eq!(parsed[2].kind, ModuleImportKind::Glob);
    assert_eq!(parsed[2].module.to_string(), "math");
    assert!(parsed[2].item.is_none());
    assert_eq!(parsed[3].kind, ModuleImportKind::Item);
    assert_eq!(parsed[3].module.to_string(), "stats");
    let item = parsed[3].item.as_ref().unwrap();
    assert_eq!(item.iter().map(|id| id.to_string()).collect::<Vec<_>>(), vec!["sum", "column"]);
}

#[test]
fn source_import_paths_remain_statement_imports() {
    let src = "+> ./dep.mec\n+> ../lib/dep.mec\n+> fs://lib/dep.mec\n+> file:///tmp/dep.mec\n+> memory://scratch/dep\n+> https://example.com/dep.mec";
    let program = parser::parse(src).expect("parse failed");
    let mut import_declarations = 0;
    for section in &program.body.sections {
        for element in &section.elements {
            if let SectionElement::MechCode(codes) = element {
                for (node, _) in codes {
                    match node {
                        MechCode::Import(import) => panic!("source import parsed as stdlib import: {:?}", import),
                        MechCode::Statement(Statement::ImportDeclaration(_)) => import_declarations += 1,
                        _ => {}
                    }
                }
            }
        }
    }
    assert_eq!(import_declarations, 6);
}

#[test]
fn rejects_invalid_stdlib_import_paths() {
    assert!(parser::parse("+> ").is_err());
    assert!(parser::parse("+> */x").is_err());
    assert!(parser::parse("+> math/").is_err());
    assert!(parser::parse("+> math/*/x").is_err());
}
