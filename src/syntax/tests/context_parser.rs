use mech_core::nodes::*;
use mech_syntax::parser;

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

#[test]
fn parses_context_declaration_with_resource_base() {
  let stmts = statements("@main := db://main{:read(users/*), :write(users/name)}");
  assert_eq!(stmts.len(), 1);
  match &stmts[0] {
    Statement::ContextDeclaration(ctx) => {
      assert_eq!(ctx.name.to_string(), "main");
      match &ctx.base {
        ContextBase::ResourceUri(uri) => assert_eq!(uri.to_string(), "db://main"),
        _ => panic!("expected resource uri base"),
      }
      assert_eq!(ctx.capabilities.len(), 2);
      assert_eq!(ctx.capabilities[0].operation.to_string(), "read");
      match &ctx.capabilities[0].scope {
        ContextCapabilityScope::Path(p) => assert_eq!(p.to_string(), "users/*"),
        _ => panic!("expected path scope"),
      }
      assert_eq!(ctx.capabilities[1].operation.to_string(), "write");
      match &ctx.capabilities[1].scope {
        ContextCapabilityScope::Path(p) => assert_eq!(p.to_string(), "users/name"),
        _ => panic!("expected path scope"),
      }
    }
    _ => panic!("expected context declaration"),
  }
}

#[test]
fn parses_context_declaration_with_context_base() {
  let stmts = statements("@users := @main{:read(users/*)}");
  match &stmts[0] {
    Statement::ContextDeclaration(ctx) => {
      assert_eq!(ctx.name.to_string(), "users");
      match &ctx.base {
        ContextBase::Context(name) => assert_eq!(name.to_string(), "main"),
        _ => panic!("expected context base"),
      }
      assert_eq!(ctx.capabilities[0].operation.to_string(), "read");
      match &ctx.capabilities[0].scope {
        ContextCapabilityScope::Path(p) => assert_eq!(p.to_string(), "users/*"),
        _ => panic!("expected path scope"),
      }
    }
    _ => panic!("expected context declaration"),
  }
}

#[test]
fn parses_context_declaration_with_wildcard_scope() {
  let stmts = statements("@main := db://main{:read(users/*), :write(*)}");
  match &stmts[0] {
    Statement::ContextDeclaration(ctx) => match &ctx.capabilities[1].scope {
      ContextCapabilityScope::Wildcard(t) => assert_eq!(t.to_string(), "*"),
      _ => panic!("expected wildcard scope"),
    },
    _ => panic!("expected context declaration"),
  }
}

#[test]
fn parses_addressed_path_expression() {
  let stmts = statements("name := users/name@main");
  match &stmts[0] {
    Statement::VariableDefine(v) => match &v.expression {
      Expression::Var(var) => {
        assert_eq!(var.name.to_string(), "users/name");
        assert_eq!(var.context.as_ref().unwrap().to_string(), "main");
      }
      _ => panic!("expected addressed var expression"),
    },
    _ => panic!("expected variable define"),
  }
}

#[test]
fn parses_addressed_path_assignment_target() {
  let stmts = statements("users/name@main = 1");
  match &stmts[0] {
    Statement::VariableAssign(assign) => {
      assert_eq!(assign.target.name.to_string(), "users/name");
      assert_eq!(assign.target.context.as_ref().unwrap().to_string(), "main");
      assert!(assign.target.subscript.is_none());
    }
    _ => panic!("expected variable assignment"),
  }
}

#[test]
fn addressed_path_does_not_break_module_path() {
  let stmts = statements("ok := math/tau > 6.0");
  match &stmts[0] {
    Statement::VariableDefine(v) => match &v.expression {
      Expression::Formula(_) => {}
      _ => panic!("expected formula"),
    },
    _ => panic!("expected variable define"),
  }
}

#[test]
fn context_declaration_does_not_break_import_export() {
  let stmts = statements("+> ./math.mec\n<+ tau");
  assert!(matches!(&stmts[0], Statement::ImportDeclaration(_)));
  assert!(matches!(&stmts[1], Statement::ExportDeclaration(_)));
}

#[test]
fn bare_capability_operation_is_rejected() {
  assert!(parser::parse("@main := db://main{:write}").is_err());
}
