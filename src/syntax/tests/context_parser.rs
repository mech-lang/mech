use mech_core::nodes::*;
use mech_syntax::parser;

fn first_statement(src: &str) -> Statement {
  let program = parser::parse(src).expect("parse failed");
  for section in &program.body.sections {
    for element in &section.elements {
      if let SectionElement::MechCode(codes) = element {
        if let Some(stmt) = codes.iter().find_map(|(n, _)| if let MechCode::Statement(stmt) = n { Some(stmt.clone()) } else { None }) {
          return stmt;
        }
      }
    }
  }
  panic!("no statement")
}

#[test]
fn parses_context_declaration_with_resource_base() {
  let stmt = first_statement("@main := db://main{:read(users/*), :write(users/name)}");
  match stmt {
    Statement::ContextDeclaration(ctx) => {
      assert_eq!(ctx.name.to_string(), "main");
      assert!(matches!(ctx.base, ContextBase::ResourceUri(_)));
      assert_eq!(ctx.capabilities.len(), 2);
      assert_eq!(ctx.capabilities[0].operation.to_string(), "read");
      assert_eq!(ctx.capabilities[1].operation.to_string(), "write");
    }
    _ => panic!("expected context declaration"),
  }
}

#[test]
fn parses_addressed_path_expression() {
  let stmt = first_statement("name := users/name@main");
  match stmt {
    Statement::VariableDefine(v) => {
      match v.expression {
        Expression::Var(var) => {
          assert_eq!(var.name.to_string(), "users/name");
          assert_eq!(var.context.unwrap().to_string(), "main");
        }
        _ => panic!("expected addressed var expression"),
      }
    }
    _ => panic!("expected variable define"),
  }
}
