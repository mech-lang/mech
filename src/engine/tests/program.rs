use mech_core::{GenericError, MechError, NoMechExecutionServices, nodes::*};
use mech_engine::{MechProgram, MechProgramConfig, ProgramTurnFinalization};

fn statements(src: &str) -> Vec<Statement> {
    let program = mech_syntax::parser::parse(src).expect("parse failed");
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

#[test]
fn program_browser_resource_binding_declaration() {
    let stmts = statements("@browser := browser://dom/");
    match &stmts[0] {
        Statement::ContextDeclaration(ctx) => {
            assert_eq!(ctx.name.to_string(), "browser");
            assert!(ctx.capabilities.is_empty());
            match &ctx.base {
                ContextBase::ResourceUri(uri) => assert_eq!(uri.to_string(), "browser://dom/"),
                _ => panic!("expected resource uri"),
            }
        }
        _ => panic!("expected context declaration"),
    }
}

#[test]
fn program_browser_resource_read() {
    let stmts = statements("x := @browser/body/content/input/_value");
    match &stmts[0] {
        Statement::VariableDefine(v) => match &v.expression {
            Expression::Var(var) => {
                assert_eq!(var.name.to_string(), "body/content/input/_value");
                assert_eq!(var.context.as_ref().unwrap().to_string(), "browser");
            }
            _ => panic!("expected addressed var expression"),
        },
        _ => panic!("expected variable define"),
    }
}

#[test]
fn program_browser_resource_write() {
    let stmts = statements("@browser/body/content/output/_value = \"Hello\"");
    match &stmts[0] {
        Statement::VariableAssign(assign) => {
            assert_eq!(assign.target.name.to_string(), "body/content/output/_value");
            assert_eq!(
                assign.target.context.as_ref().unwrap().to_string(),
                "browser"
            );
        }
        _ => panic!("expected variable assignment"),
    }
}

#[test]
fn program_browser_resource_define_syntax_is_rejected() {
    assert!(mech_syntax::parser::parse("@browser/title := \"Hello\"").is_err());
}

#[test]
fn coordinated_turn_rollback_is_safe_and_returns_the_finalizer_error() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let mut services = NoMechExecutionServices;

    let error = program
        .update_inputs_and_advance_turn_coordinated(&[], &mut services, |_| {
            ProgramTurnFinalization::Rollback(MechError::new(
                GenericError {
                    msg: "deliberate finalizer rollback".into(),
                },
                None,
            ))
        })
        .unwrap_err();

    assert_eq!(error.kind_name(), "GenericError");
    assert!(
        error
            .kind_message()
            .contains("deliberate finalizer rollback")
    );
}
