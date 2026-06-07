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
