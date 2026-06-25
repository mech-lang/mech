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

fn assert_no_mech_code_errors(program: &Program) {
    for section in &program.body.sections {
        for element in &section.elements {
            match element {
                SectionElement::MechCode(codes) | SectionElement::FencedMechCode(FencedMechCode { code: codes, .. }) => {
                    for (node, _) in codes {
                        if matches!(node, MechCode::Error(..)) {
                            panic!("unexpected MechCode::Error: {node:?}");
                        }
                    }
                }
                _ => {}
            }
        }
    }
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
        "+> ./dep.mec\n+> ../lib/dep.mec\n+> fs://lib/dep.mec\n+> file:///tmp/dep.mec\n+> memory://scratch/dep\n+> https://example.com/dep.mec\n+> http://example.com/dep.mec",
    );
    assert_eq!(stmts.len(), 7);
    assert!(stmts
        .iter()
        .all(|stmt| matches!(stmt, Statement::ImportDeclaration(_))));
}

#[test]
fn arbitrary_module_roots_parse_as_module_imports() {
    let parsed = imports("+> userlib\n+> userlib/tool");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].kind, ModuleImportKind::Module);
    assert_eq!(parsed[1].kind, ModuleImportKind::Item);
}

#[test]
fn rejects_invalid_stdlib_import_paths() {
    assert!(parser::parse("+> ").is_err());
    assert!(parser::parse("+> */x").is_err());
    assert!(parser::parse("+> math/").is_err());
    assert!(parser::parse("+> math/*/x").is_err());
}

#[test]
fn parses_context_and_value_import_aliases() {
    let parsed = imports("+> @ui := browser/dom\n+> s := math/sin");
    assert_eq!(parsed.len(), 2);
    match &parsed[0].alias {
        Some(ModuleImportAlias::Context(name)) => assert_eq!(name.to_string(), "ui"),
        other => panic!("expected context alias, got {other:?}"),
    }
    assert!(matches!(parsed[1].alias, Some(ModuleImportAlias::Value(_))));
}

#[test]
fn parses_combinatorics_module_item_import() {
    let parsed = imports("+> combinatorics/n-choose-k");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].kind, ModuleImportKind::Item);
    assert_eq!(parsed[0].module.to_string(), "combinatorics");
    assert_eq!(item_path(&parsed[0]), vec!["n-choose-k"]);
}

#[test]
fn rejects_invalid_context_import_aliases() {
    assert!(parser::parse("+> @ui/main := browser/dom").is_err());
    assert!(parser::parse("+> @foo/bar := browser/dom").is_err());
    assert!(parser::parse("+> @ui := browser").is_err());
    assert!(parser::parse("+> @ui := browser/*").is_err());
    assert!(parser::parse("+> @ui := browser/{dom, storage}").is_err());
    assert!(parser::parse("+> @ui := fs://workspace").is_err());
}

#[test]
fn whole_documents_parse_module_and_context_imports_without_errors() {
    for src in [
        "+> math/*\nx := 1.23\nsin(x)\n",
        "+> geometry/triangle-area\narea := triangle-area(3, 4, 1.5708)\n<+ area\n",
        "+> @ui := browser/dom\ntitle := @ui/counter/_text\n",
    ] {
        let program = parser::parse(src).expect("whole document should parse");
        assert_no_mech_code_errors(&program);
    }
}

#[test]
fn examples_working_parse_without_mech_code_errors() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/working");
    let mut stack = vec![root];
    while let Some(path) = stack.pop() {
        let entries = std::fs::read_dir(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", path.display()));
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "mec") {
                let src = std::fs::read_to_string(&path)
                    .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
                let program = parser::parse(&src)
                    .unwrap_or_else(|err| panic!("failed to parse {}: {err:?}", path.display()));
                assert_no_mech_code_errors(&program);
            }
        }
    }
}

#[test]
fn dynamic_module_imports_stay_mech_code_imports() {
    let parsed = imports("+> combinatorics/n-choose-k\n+> userlib/tool\n+> math/sin\n+> browser/dom\n");
    assert_eq!(parsed.len(), 4);
    for import in &parsed {
        assert_eq!(import.kind, ModuleImportKind::Item);
    }
    assert_eq!(parsed[0].module.to_string(), "combinatorics");
    assert_eq!(item_path(&parsed[0]), vec!["n-choose-k"]);
    assert_eq!(parsed[1].module.to_string(), "userlib");
    assert_eq!(item_path(&parsed[1]), vec!["tool"]);
    assert_eq!(parsed[2].module.to_string(), "math");
    assert_eq!(item_path(&parsed[2]), vec!["sin"]);
    assert_eq!(parsed[3].module.to_string(), "browser");
    assert_eq!(item_path(&parsed[3]), vec!["dom"]);
}

#[test]
fn context_import_alias_accepts_single_segment_without_underscore() {
    let parsed = imports("+> @ui := browser/dom\n+> @my-ui := browser/dom\n");
    assert_eq!(parsed.len(), 2);
    match &parsed[0].alias {
        Some(ModuleImportAlias::Context(name)) => assert_eq!(name.to_string(), "ui"),
        other => panic!("expected context alias, got {other:?}"),
    }
    match &parsed[1].alias {
        Some(ModuleImportAlias::Context(name)) => assert_eq!(name.to_string(), "my-ui"),
        other => panic!("expected context alias, got {other:?}"),
    }
    assert!(parser::parse("+> @my_ui := browser/dom").is_err());
    assert!(parser::parse("+> @ui/main := browser/dom").is_err());
    assert!(parser::parse("+> @foo/bar := browser/dom").is_err());
}

#[test]
fn source_imports_accept_generic_uris_bare_and_absolute_mec_paths() {
    let stmts = statements(
        "+> dep.mec\n+> lib/dep.mec\n+> ./dep.mec\n+> ../lib/dep.mec\n+> /tmp/lib.mec\n+> /workspace/app/main.mec\n+> fs://lib/dep.mec\n+> file:///tmp/dep.mec\n+> memory://scratch/dep\n+> https://example.com/dep.mec\n+> s3://bucket/app.mec\n+> db://module/main.mec\n",
    );
    assert_eq!(stmts.len(), 12);
    assert!(stmts.iter().all(|stmt| matches!(stmt, Statement::ImportDeclaration(_))));
}

#[test]
fn source_wildcard_import_specifiers_parse() {
    let stmts = statements(
        "+> dep.mec/*\n+> lib/dep.mec/*\n+> ./dep.mec/*\n+> ../lib/dep.mec/*\n+> /tmp/lib.mec/*\n+> fs://lib/dep.mec/*\n+> https://example.com/dep.mec/*\n",
    );

    let specifiers: Vec<String> = stmts
        .iter()
        .map(|stmt| match stmt {
            Statement::ImportDeclaration(import) => import.specifier.to_string(),
            other => panic!("expected source import, got {other:?}"),
        })
        .collect();

    assert_eq!(specifiers, vec![
        "dep.mec/*",
        "lib/dep.mec/*",
        "./dep.mec/*",
        "../lib/dep.mec/*",
        "/tmp/lib.mec/*",
        "fs://lib/dep.mec/*",
        "https://example.com/dep.mec/*",
    ]);
}

#[test]
fn source_wildcard_import_specifiers_reject_invalid_placements() {
    for invalid in [
        "+> *",
        "+> dep.mec*",
        "+> dep.mec/**",
        "+> dep.mec/*/foo",
        "+> ./lib/*/dep.mec",
        "+> fs://lib/dep.mec/**",
        "+> fs://lib*/dep.mec/*",
        "+> https://example.com/dep.mec/*/foo",
        "+> s3://bucket/app.mec*",
    ] {
        assert!(parser::parse(invalid).is_err(), "expected parse failure for {invalid}");
    }
}

#[test]
fn source_and_module_imports_remain_separate() {
    let module_imports = imports("+> math/sin\n+> math/*\n+> combinatorics/n-choose-k\n+> browser/dom\n+> @ui := browser/dom\n");
    assert_eq!(module_imports.len(), 5);
    let source_imports = statements("+> dep.mec\n+> ./dep.mec\n+> ../lib/dep.mec\n+> /tmp/lib.mec\n+> fs://lib/dep.mec\n+> s3://bucket/app.mec\n");
    assert_eq!(source_imports.len(), 6);
    assert!(source_imports.iter().all(|stmt| matches!(stmt, Statement::ImportDeclaration(_))));

    for invalid in [
        "+> @ui/main := browser/dom",
        "+> @foo/bar := browser/dom",
        "+> @ui := browser",
        "+> @ui := browser/*",
        "+> @ui := browser/{dom, storage}",
        "+> @ui := fs://workspace",
        "+> @my_ui := browser/dom",
    ] {
        assert!(parser::parse(invalid).is_err(), "expected parse failure for {invalid}");
    }
}

#[test]
fn source_uri_import_specifiers_trim_trailing_whitespace() {
    let stmts = statements("+> fs://lib/dep.mec   \n+> https://example.com/dep.mec   \n+> memory://scratch/dep   \n");
    let specifiers: Vec<String> = stmts
        .iter()
        .map(|stmt| match stmt {
            Statement::ImportDeclaration(import) => import.specifier.to_string(),
            other => panic!("expected source import, got {other:?}"),
        })
        .collect();
    assert_eq!(specifiers, vec![
        "fs://lib/dep.mec",
        "https://example.com/dep.mec",
        "memory://scratch/dep",
    ]);
}
