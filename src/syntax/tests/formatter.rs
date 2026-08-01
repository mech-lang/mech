#![cfg(feature = "formatter")]

use mech_core::{hash_str, nodes::*};
use mech_syntax::{Formatter, HtmlShimExtraSlots};

fn token(kind: TokenKind, text: &str) -> Token {
    Token::new(kind, SourceRange::default(), text.chars().collect())
}

fn ident(name: &str) -> Identifier {
    Identifier {
        name: token(TokenKind::Identifier, name),
    }
}

fn atom_expr(name: &str) -> Expression {
    Expression::Literal(Literal::Atom(Atom { name: ident(name) }))
}

fn fsm_declare_fixture() -> FsmDeclare {
    let arg = (Some(ident("input")), atom_expr("start"));
    FsmDeclare {
        fsm: Fsm {
            name: ident("machine"),
            args: Some(vec![arg.clone()]),
            kind: Some(KindAnnotation {
                kind: Kind::Scalar(ident("State")),
            }),
        },
        pipe: FsmPipe {
            start: FsmInstance {
                name: ident("other"),
                args: Some(vec![arg]),
            },
            transitions: vec![Transition::Next(Pattern::Expression(atom_expr("ready")))],
        },
    }
}

#[test]
fn fsm_declare_statement_formats_plain_without_panicking() {
    let mut formatter = Formatter::new();
    let statement = Statement::FsmDeclare(fsm_declare_fixture());

    assert_eq!(
        formatter.statement(&statement),
        "#machine(input: :start)⟨State⟩ := #other(input: :start) -> :ready"
    );
}

#[test]
fn split_table_statement_formats_plain_operator() {
    let mut formatter = Formatter::new();

    assert_eq!(formatter.statement(&Statement::SplitTable), ">-");
}

#[test]
fn flatten_table_statement_formats_plain_operator() {
    let mut formatter = Formatter::new();

    assert_eq!(formatter.statement(&Statement::FlattenTable), "-<");
}

#[test]
fn mech_code_error_formats_without_panicking() {
    let mut formatter = Formatter::new();
    let code: Vec<(MechCode, Option<Comment>)> = vec![(
        MechCode::Error(token(TokenKind::Error, "bad"), SourceRange::default()),
        None,
    )];

    assert!(formatter.mech_code(&code).contains("ERROR"));
}

#[test]
fn fsm_declare_statement_formats_html_class() {
    let mut formatter = Formatter::new();
    formatter.html = true;
    let statement = Statement::FsmDeclare(fsm_declare_fixture());

    assert!(formatter.statement(&statement).contains("mech-fsm-declare"));
}

#[test]
fn formatter_renders_context_qualified_var_with_prefix_context() {
    let mut formatter = Formatter::new();
    let var = Var {
        name: ident("body/content/input/_value"),
        context: Some(ident("browser")),
        kind: None,
    };

    assert_eq!(formatter.var(&var), "@browser/body/content/input/_value");
}

#[test]
fn formatter_renders_context_qualified_assignment_target_with_prefix_context() {
    let mut formatter = Formatter::new();
    let assign = VariableAssign {
        target: SliceRef {
            name: ident("body/content/output/_value"),
            context: Some(ident("browser")),
            subscript: None,
        },
        expression: Expression::Literal(Literal::String(MechString {
            text: token(TokenKind::String, "hello"),
        })),
    };

    assert_eq!(
        formatter.variable_assign(&assign),
        "@browser/body/content/output/_value = \"hello\""
    );
}

#[test]
fn formatter_uses_the_stable_root_namespace_for_inline_output_addresses() {
    let tree =
        mech_syntax::parser::parse("The document evaluates {answer + 1} inline.\n\nanswer := 41")
            .unwrap();
    let html = Formatter::new().format_html(&tree, String::new(), "{{INTRO}}".to_string());
    let expected = format!(
        "id=\"{}:0\" class=\"mech-inline-mech-code\"",
        hash_str("inline-eval:0:0"),
    );

    assert!(
        html.contains(&expected),
        "missing formatter inline address: {html}"
    );
}

fn first_statement(src: &str) -> Statement {
    let program = mech_syntax::parser::parse(src).expect("parse failed");
    for section in &program.body.sections {
        for element in &section.elements {
            if let SectionElement::MechCode(codes) = element {
                for (node, _) in codes {
                    if let MechCode::Statement(statement) = node {
                        return statement.clone();
                    }
                }
            }
        }
    }
    panic!("expected statement")
}

#[test]
fn formatter_preserves_new_prefix_context_resource_read() {
    let mut formatter = Formatter::new();
    let statement = first_statement("name := @browser/body/content/input/_value");

    assert_eq!(
        formatter.statement(&statement),
        "name := @browser/body/content/input/_value"
    );
}

fn plain_paragraph(text: &str) -> Paragraph {
    Paragraph {
        elements: vec![ParagraphElement::Text(token(TokenKind::Text, text))],
        error_range: None,
    }
}

fn html_fixture(sections: &[(&str, &str)]) -> Program {
    Program {
        title: Some(Title {
            text: token(TokenKind::Title, "Slot Fixture"),
            author: Some(plain_paragraph("Fixture Author")),
            date: Some(plain_paragraph("Fixture Date")),
            hero: None,
            kicker: Some(plain_paragraph("Fixture Kicker")),
            section: Some(plain_paragraph("Fixture Section")),
            summary: Some(plain_paragraph("Fixture Summary")),
            next: Some(plain_paragraph("Fixture Next")),
            previous: Some(plain_paragraph("Fixture Previous")),
        }),
        body: Body {
            sections: sections
                .iter()
                .map(|(heading, content)| Section {
                    subtitle: Some(Subtitle {
                        text: plain_paragraph(heading),
                        level: 2,
                    }),
                    elements: vec![SectionElement::Paragraph(plain_paragraph(content))],
                })
                .collect(),
        },
    }
}

#[test]
fn html_shim_static_slots_render_once() {
    let tree = html_fixture(&[("Fixture section", "Fixture content")]);
    let mut extra_slots = HtmlShimExtraSlots::default();
    extra_slots.insert("TITLE", "Extra title");
    extra_slots.insert("CUSTOM", "{{TITLE}}");

    assert_eq!(extra_slots.get("CUSTOM"), Some("{{TITLE}}"));

    let mut formatter = Formatter::new();
    let render = formatter.format_html_with_slots(
        &tree,
        "/* {{TITLE}} */".to_string(),
        "{{STYLESHEET}}|{{TITLE}}|{{CUSTOM}}|{{REPL}}".to_string(),
        &extra_slots,
    );

    assert!(
        render
            .html
            .starts_with("/* {{TITLE}} */|Extra title|{{TITLE}}|")
    );
    assert!(
        render
            .html
            .contains("class=\"console-scroll mech-repl hidden\"")
    );
    assert!(render.html.contains("id=\"mech-output\""));
    assert!(render.html.contains("data-mech-repl-mount"));
    assert!(render.html.contains("aria-live=\"polite\""));
    assert!(render.consumed_slots.contains("STYLESHEET"));
    assert!(render.consumed_slots.contains("TITLE"));
    assert!(render.consumed_slots.contains("CUSTOM"));
    assert!(render.consumed_slots.contains("REPL"));
    assert!(render.unresolved_mech_slots.is_empty());

    let mut wrapper_formatter = Formatter::new();
    assert_eq!(
        wrapper_formatter.format_html(&tree, String::new(), "{{TITLE}}".to_string()),
        "Slot Fixture"
    );
}

#[test]
fn html_shim_preserves_literal_placeholders_in_document_content() {
    let tree = html_fixture(&[("Literal section", "{{TITLE}} {{AUTHOR}} {{SECTION1}}")]);
    let mut formatter = Formatter::new();
    let render = formatter.format_html_with_slots(
        &tree,
        String::new(),
        "<main>{{CONTENT}}</main>".to_string(),
        &HtmlShimExtraSlots::default(),
    );

    for placeholder in ["{{TITLE}}", "{{AUTHOR}}", "{{SECTION1}}"] {
        assert!(
            render.html.contains(placeholder),
            "rendered content lost literal {placeholder}"
        );
    }
    assert!(render.consumed_slots.contains("CONTENT"));
    assert!(render.unresolved_mech_slots.is_empty());
}

#[test]
fn html_shim_preserves_dynamic_variable_placeholders() {
    let tree = html_fixture(&[("Fixture section", "Fixture content")]);
    let mut extra_slots = HtmlShimExtraSlots::default();
    extra_slots.insert("VAR:answer", "must not replace a dynamic binding");
    let mut formatter = Formatter::new();
    let render = formatter.format_html_with_slots(
        &tree,
        String::new(),
        "before {{VAR:answer}} after {{TITLE}}".to_string(),
        &extra_slots,
    );

    assert_eq!(render.html, "before {{VAR:answer}} after Slot Fixture");
    assert!(!render.consumed_slots.contains("VAR:answer"));
    assert!(render.consumed_slots.contains("TITLE"));
    assert!(render.unresolved_mech_slots.is_empty());
}

#[test]
fn html_shim_reports_unknown_uppercase_slots() {
    let tree = html_fixture(&[("Fixture section", "Fixture content")]);
    let mut formatter = Formatter::new();
    let render = formatter.format_html_with_slots(
        &tree,
        String::new(),
        "{{UNKNOWN_SLOT}} {{mixed_case}} {{TITLE}}".to_string(),
        &HtmlShimExtraSlots::default(),
    );

    assert!(render.html.contains("{{UNKNOWN_SLOT}}"));
    assert!(render.html.contains("{{mixed_case}}"));
    assert!(render.unresolved_mech_slots.contains("UNKNOWN_SLOT"));
    assert_eq!(render.unresolved_mech_slots.len(), 1);
    assert!(render.consumed_slots.contains("TITLE"));
}

#[test]
fn html_shim_section_slots_render_by_index() {
    let tree = html_fixture(&[
        ("First heading", "first-section-marker"),
        ("Second heading", "second-section-marker"),
    ]);
    let mut formatter = Formatter::new();
    let render = formatter.format_html_with_slots(
        &tree,
        String::new(),
        "{{SECTION2}}|{{SECTION1}}".to_string(),
        &HtmlShimExtraSlots::default(),
    );

    let second = render
        .html
        .find("second-section-marker")
        .expect("second section should render");
    let first = render
        .html
        .find("first-section-marker")
        .expect("first section should render");
    assert!(
        second < first,
        "section placeholders should preserve their indices"
    );
    assert!(render.consumed_slots.contains("SECTION1"));
    assert!(render.consumed_slots.contains("SECTION2"));
    assert!(render.unresolved_mech_slots.is_empty());
}

fn shipped_shim(name: &str) -> String {
    let path = format!("{}/../../include/{name}.html", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read shipped shim {path}: {error}");
    })
}

#[test]
fn shipped_document_shims_consume_required_slots() {
    let tree = html_fixture(&[("Fixture section", "Fixture content")]);

    for shim_name in ["index", "blog", "docs"] {
        let mut extra_slots = HtmlShimExtraSlots::default();
        extra_slots.insert("DOCUMENT_SCRIPT", "window.mechDocumentController = true;");
        extra_slots.insert("DOCUMENT_SOURCES", "eyJ2ZXJzaW9uIjoxLCJzb3VyY2VzIjpbXX0=");
        extra_slots.insert("WASM_MODULE_URL", "./_mech/pkg/mech_wasm.js");
        extra_slots.insert("SOURCE_URL_KEY", "encoded-source-key");
        let mut formatter = Formatter::new();
        let render = formatter.format_html_with_slots(
            &tree,
            "body { color: black; }".to_string(),
            shipped_shim(shim_name),
            &extra_slots,
        );

        assert!(
            render.unresolved_mech_slots.is_empty(),
            "{shim_name} has unresolved slots: {:?}",
            render.unresolved_mech_slots
        );
        for slot in [
            "DOCUMENT_SCRIPT",
            "DOCUMENT_SOURCES",
            "WASM_MODULE_URL",
            "SOURCE_URL_KEY",
            "TITLE",
            "CODE",
            "REPL",
        ] {
            assert!(
                render.consumed_slots.contains(slot),
                "{shim_name} did not consume {slot}"
            );
        }
        assert!(
            render.consumed_slots.contains("CONTENT") || render.consumed_slots.contains("CONTENTS"),
            "{shim_name} did not consume a document-content slot"
        );
        assert!(render.html.contains("encoded-source-key"));
    }
}
