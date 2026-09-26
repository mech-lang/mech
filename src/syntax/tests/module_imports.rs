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
                SectionElement::MechCode(codes)
                | SectionElement::FencedMechCode(FencedMechCode { code: codes, .. }) => {
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
fn parses_comma_separated_module_imports_in_source_order() {
    for src in ["+> math/*, logic/all", "+> math/* ,\tlogic/all"] {
        let parsed = imports(src);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].module.to_string(), "math");
        assert_eq!(parsed[0].kind, ModuleImportKind::Glob);
        assert_eq!(parsed[1].module.to_string(), "logic");
        assert_eq!(item_path(&parsed[1]), vec!["all"]);
        assert_eq!(parsed[0].module.name.src_range.start.row, 1);
        assert_eq!(parsed[1].module.name.src_range.start.row, 1);
        assert!(parsed[0].module.name.src_range.end <= parsed[1].module.name.src_range.start);
    }
}

#[test]
fn comma_imports_accept_groups_namespaces_and_aliases() {
    let parsed =
        imports("+> math/{sin, cos}, stats, s := math/sin, @ui := browser/dom, logic/all\n");
    assert_eq!(parsed.len(), 5);
    assert_eq!(parsed[0].kind, ModuleImportKind::Group);
    let items: Vec<_> = parsed[0]
        .group_items
        .as_ref()
        .unwrap()
        .iter()
        .map(|item| item.item.to_string())
        .collect();
    assert_eq!(items, vec!["sin", "cos"]);
    assert_eq!(parsed[1].kind, ModuleImportKind::Module);
    assert!(matches!(parsed[2].alias, Some(ModuleImportAlias::Value(_))));
    assert!(matches!(
        parsed[3].alias,
        Some(ModuleImportAlias::Context(_))
    ));
    assert_eq!(parsed[4].module.to_string(), "logic");
}

#[test]
fn comma_imports_do_not_consume_following_statements() {
    for src in [
        "+> math/*, logic/all\nx := [1, 2, 3]\n",
        "+> math/*, logic/all; x := [1, 2, 3]\n",
        "+> math/*, logic/all\n+> stats/sum/column\nx := [1, 2, 3]\n",
    ] {
        let program = parser::parse(src).expect("parse import followed by statement");
        assert_no_mech_code_errors(&program);
        assert_eq!(statements(src).len(), 1);
    }
}

#[test]
fn comma_imports_reject_missing_or_invalid_items() {
    for src in [
        "+> math/*,",
        "+> math/*,, logic/all",
        "+> , math/*",
        "+> math/*, logic/",
        "+> math/*, @ui := browser/*",
        "+> math/*, ./dep.mec",
        "+> math/*,\nlogic/all",
        "+> math/*, -- missing import\nx := 1\n",
    ] {
        assert!(
            parser::parse(src).is_err(),
            "expected parse failure for {src:?}"
        );
    }
}

#[test]
fn comma_imports_work_in_fenced_mechdown_and_title_front_matter() {
    let src = "Import Lists\n============\n+> math/*, logic/all -- shared imports\n============\n\n~~~mech:demo\n+> stats/{sum/row, sum/column}, s := math/sin\nx := s(0)\n~~~\n";
    let program = parser::parse(src).expect("parse import lists in Mechdown");
    assert_no_mech_code_errors(&program);
    let front = &program.title.as_ref().unwrap().imports;
    assert_eq!(front.len(), 2);
    assert!(front[0].1.is_none());
    assert!(front[1].1.is_some());
    let fenced = program
        .body
        .sections
        .iter()
        .flat_map(|section| section.elements.iter())
        .find_map(|element| match element {
            SectionElement::FencedMechCode(block) => Some(block),
            _ => None,
        })
        .expect("fenced block");
    assert_eq!(
        fenced
            .code
            .iter()
            .filter(|(code, _)| matches!(code, MechCode::Import(_)))
            .count(),
        2
    );
}

#[cfg(feature = "formatter")]
#[test]
fn formatter_round_trips_comma_imports_and_their_trailing_comment() {
    let src = "+> math/{sin, cos}, stats, s := math/sin, @ui := browser/dom, logic/all -- shared imports\nx := s(0)\n";
    let program = parser::parse(src).unwrap();
    let formatted = mech_syntax::Formatter::new().format(&program);
    assert!(formatted.contains("+> math/{sin, cos}"));
    assert!(formatted.contains("+> stats"));
    assert!(formatted.contains("+> s := math/sin"));
    assert!(formatted.contains("+> @ui := browser/dom"));
    assert!(formatted.contains("+> logic/all -- shared imports"));
    assert_eq!(formatted.matches("shared imports").count(), 1);
    let original = imports(src);
    let reparsed = imports(&formatted);
    assert_eq!(original.len(), reparsed.len());
    for (expected, actual) in original.iter().zip(&reparsed) {
        let mut formatter = mech_syntax::Formatter::new();
        assert_eq!(
            formatter.module_import(expected),
            formatter.module_import(actual)
        );
    }
    assert_eq!(statements(&formatted).len(), 1);
}

#[cfg(feature = "formatter")]
#[test]
fn formatter_emits_styled_import_tokens_without_changing_source_text() {
    for source in [
        "+> stats",
        "+> math/*",
        "+> logic/all",
        "+> stats/{sum/row, sum/column}",
        "+> s := math/sin",
        "+> @ui := browser/dom",
    ] {
        let import = imports(source).remove(0);
        let mut formatter = mech_syntax::Formatter::new();
        assert_eq!(formatter.module_import(&import), source);
        formatter.html = true;
        let html = formatter.module_import(&import);
        assert!(
            html.starts_with(
                "<span class=\"mech-import\"><span class=\"mech-import-sigil\">+&gt;</span> "
            ),
            "{html}"
        );
        assert!(html.contains("class=\"mech-import-module\""), "{html}");
        if source.contains('/') {
            assert!(html.contains("class=\"mech-import-separator\""));
            assert!(html.contains("class=\"mech-import-selector\""));
        }
        if source.contains(":=") {
            assert!(html.contains("class=\"mech-import-alias\""));
            assert!(html.contains("class=\"mech-import-assign-op\">:=</span>"));
        }
        let mut inside_tag = false;
        let visible = html
            .chars()
            .filter(|character| match character {
                '<' => {
                    inside_tag = true;
                    false
                }
                '>' => {
                    inside_tag = false;
                    false
                }
                _ => !inside_tag,
            })
            .collect::<String>()
            .replace("&gt;", ">")
            .replace("&lt;", "<")
            .replace("&amp;", "&");
        assert_eq!(
            visible, source,
            "styling must preserve the import expression"
        );
    }
}

#[cfg(feature = "formatter")]
#[test]
fn formatter_round_trips_comma_imports_in_mechdown() {
    let src = "Import Lists\n============\n+> math/*, logic/all -- shared imports\n============\n\n~~~mech:demo\n+> stats, s := math/sin -- block imports\nx := s(0)\n~~~\n";
    let program = parser::parse(src).unwrap();
    let formatted = mech_syntax::Formatter::new().format(&program);
    let reparsed = parser::parse(&formatted).unwrap();
    assert_no_mech_code_errors(&reparsed);
    assert_eq!(reparsed.title.unwrap().imports.len(), 2);
    assert_eq!(formatted.matches("shared imports").count(), 1);
    assert_eq!(formatted.matches("block imports").count(), 1);
    assert!(formatted.contains("+> stats"));
    assert!(formatted.contains("+> s := math/sin"));
}

#[test]
fn preserves_source_import_declarations() {
    let stmts = statements(
        "+> ./dep.mec\n+> ../lib/dep.mec\n+> fs://lib/dep.mec\n+> file:///tmp/dep.mec\n+> memory://scratch/dep\n+> https://example.com/dep.mec\n+> http://example.com/dep.mec",
    );
    assert_eq!(stmts.len(), 7);
    assert!(
        stmts
            .iter()
            .all(|stmt| matches!(stmt, Statement::ImportDeclaration(_)))
    );
}

#[test]
fn parses_extensionless_and_literal_percent_source_imports() {
    for source in ["+> ./dep", "+> ./package", "+> ./rate%.mec"] {
        parser::parse(source)
            .unwrap_or_else(|error| panic!("failed to parse `{source}`: {error:?}"));
    }
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
            let entry = entry
                .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", path.display()));
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
    let parsed =
        imports("+> combinatorics/n-choose-k\n+> userlib/tool\n+> math/sin\n+> browser/dom\n");
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
    assert!(
        stmts
            .iter()
            .all(|stmt| matches!(stmt, Statement::ImportDeclaration(_)))
    );
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

    assert_eq!(
        specifiers,
        vec![
            "dep.mec/*",
            "lib/dep.mec/*",
            "./dep.mec/*",
            "../lib/dep.mec/*",
            "/tmp/lib.mec/*",
            "fs://lib/dep.mec/*",
            "https://example.com/dep.mec/*",
        ]
    );
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
        assert!(
            parser::parse(invalid).is_err(),
            "expected parse failure for {invalid}"
        );
    }
}

#[test]
fn source_and_module_imports_remain_separate() {
    let module_imports = imports(
        "+> math/sin\n+> math/*\n+> combinatorics/n-choose-k\n+> browser/dom\n+> @ui := browser/dom\n",
    );
    assert_eq!(module_imports.len(), 5);
    let source_imports = statements(
        "+> dep.mec\n+> ./dep.mec\n+> ../lib/dep.mec\n+> /tmp/lib.mec\n+> fs://lib/dep.mec\n+> s3://bucket/app.mec\n",
    );
    assert_eq!(source_imports.len(), 6);
    assert!(
        source_imports
            .iter()
            .all(|stmt| matches!(stmt, Statement::ImportDeclaration(_)))
    );

    for invalid in [
        "+> @ui/main := browser/dom",
        "+> @foo/bar := browser/dom",
        "+> @ui := browser",
        "+> @ui := browser/*",
        "+> @ui := browser/{dom, storage}",
        "+> @ui := fs://workspace",
        "+> @my_ui := browser/dom",
    ] {
        assert!(
            parser::parse(invalid).is_err(),
            "expected parse failure for {invalid}"
        );
    }
}

#[test]
fn source_uri_import_specifiers_trim_trailing_whitespace() {
    let stmts = statements(
        "+> fs://lib/dep.mec   \n+> https://example.com/dep.mec   \n+> memory://scratch/dep   \n",
    );
    let specifiers: Vec<String> = stmts
        .iter()
        .map(|stmt| match stmt {
            Statement::ImportDeclaration(import) => import.specifier.to_string(),
            other => panic!("expected source import, got {other:?}"),
        })
        .collect();
    assert_eq!(
        specifiers,
        vec![
            "fs://lib/dep.mec",
            "https://example.com/dep.mec",
            "memory://scratch/dep",
        ]
    );
}
