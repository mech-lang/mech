#![cfg(feature = "formatter")]

use mech_core::nodes::*;
use mech_syntax::Formatter;

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
        expression: Expression::Literal(Literal::String(MechString { text: token(TokenKind::String, "hello") })),
    };

    assert_eq!(formatter.variable_assign(&assign), "@browser/body/content/output/_value = \"hello\"");
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
fn formatter_normalizes_old_suffix_context_resource_read_to_prefix_context() {
    let mut formatter = Formatter::new();
    let statement = first_statement("name := body/content/input/_value@browser");

    assert_eq!(formatter.statement(&statement), "name := @browser/body/content/input/_value");
}

#[test]
fn formatter_preserves_new_prefix_context_resource_read() {
    let mut formatter = Formatter::new();
    let statement = first_statement("name := @browser/body/content/input/_value");

    assert_eq!(formatter.statement(&statement), "name := @browser/body/content/input/_value");
}
