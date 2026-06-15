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
  let stmts = statements("name := @main/users/name");
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
  let stmts = statements("@main/users/name = 1");
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
      assert_eq!(assign.target.context.as_ref().unwrap().to_string(), "browser");
    }
    _ => panic!("expected variable assignment"),
  }
}

#[test]
fn program_browser_resource_define_does_not_write() {
  let stmts = statements("@browser/title := \"Hello\"");
  assert!(matches!(&stmts[0], Statement::VariableDefine(_)));
}

#[test]
fn parses_prefix_browser_resource_read_inside_expression() {
  let stmts = statements(
    "@browser := browser://dom/\ngreeting := \"Hello, \" + @browser/body/content/input/_value",
  );
  match &stmts[1] {
    Statement::VariableDefine(v) => match &v.expression {
      Expression::Formula(factor) => match factor {
        Factor::Term(term) => match &term.rhs[0].1 {
          Factor::Expression(expr) => match &**expr {
            Expression::Var(var) => {
              assert_eq!(var.name.to_string(), "body/content/input/_value");
              assert_eq!(var.context.as_ref().unwrap().to_string(), "browser");
            }
            _ => panic!("expected addressed var expression in formula rhs"),
          },
          _ => panic!("expected expression factor"),
        },
        _ => panic!("expected formula term"),
      },
      _ => panic!("expected formula expression"),
    },
    _ => panic!("expected variable define"),
  }
}

#[test]
fn parses_required_context_forms() {
  assert!(parser::parse("@ui := browser://dom").is_ok());
  assert!(parser::parse("@child := @ui").is_ok());
  assert!(parser::parse("x := @ui/counter").is_ok());
  assert!(parser::parse("x := @ui/counter<String>").is_ok());
}

#[test]
fn rejects_legacy_suffix_context_forms() {
  assert!(parser::parse("x := counter@ui").is_err());
  assert!(parser::parse("x := counter@ui<String>").is_err());
  assert!(parser::parse("x := counter @ ui").is_err());
  assert!(parser::parse("x := counter[0]@ui").is_err());
  assert!(parser::parse("x := counter.foo@ui").is_err());
  assert!(parser::parse("x := counter{0}@ui").is_err());
}
