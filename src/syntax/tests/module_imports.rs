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
    let parsed = imports("+> math\n+> math/sin\n+> math/*");
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].kind, ModuleImportKind::Module);
    assert_eq!(parsed[0].module.to_string(), "math");
    assert!(parsed[0].item.is_none());
    assert_eq!(parsed[1].kind, ModuleImportKind::Item);
    assert_eq!(parsed[1].module.to_string(), "math");
    assert_eq!(parsed[1].item.as_ref().unwrap().to_string(), "sin");
    assert_eq!(parsed[2].kind, ModuleImportKind::Glob);
    assert_eq!(parsed[2].module.to_string(), "math");
    assert!(parsed[2].item.is_none());
}

#[test]
fn rejects_invalid_stdlib_import_paths() {
    assert!(parser::parse("+> ").is_err());
    assert!(parser::parse("+> */x").is_err());
    assert!(parser::parse("+> math/").is_err());
    assert!(parser::parse("+> math/*/x").is_err());
}
