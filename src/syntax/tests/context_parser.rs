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
fn context_capability_paths_accept_slashes_and_underscores() {
  assert!(parser::parse("@main := db://main{:read(users/*)}").is_ok());
  assert!(parser::parse("@main := db://main{:read(users/name), :write(users/*)}").is_ok());
  assert!(
    parser::parse("@browser := browser://dom/{:read(body/search/_value), :write(body/header/title)}")
      .is_ok()
  );

  let stmts = statements("@browser := browser://dom/{:read(body/search/_value), :write(body/header/title)}");
  match &stmts[0] {
    Statement::ContextDeclaration(ctx) => {
      assert_eq!(ctx.capabilities.len(), 2);
      match &ctx.capabilities[0].scope {
        ContextCapabilityScope::Path(path) => assert_eq!(path.to_string(), "body/search/_value"),
        _ => panic!("expected read path capability"),
      }
      match &ctx.capabilities[1].scope {
        ContextCapabilityScope::Path(path) => assert_eq!(path.to_string(), "body/header/title"),
        _ => panic!("expected write path capability"),
      }
    }
    _ => panic!("expected context declaration"),
  }
}

#[test]
fn context_capability_paths_extract_nested_wildcard_scopes() {
  let stmts = statements("@main := db://main{:read(users/*), :write(users/name)}");
  match &stmts[0] {
    Statement::ContextDeclaration(ctx) => {
      assert_eq!(ctx.capabilities.len(), 2);
      match &ctx.capabilities[0].scope {
        ContextCapabilityScope::Path(path) => assert_eq!(path.to_string(), "users/*"),
        _ => panic!("expected read path capability"),
      }
      match &ctx.capabilities[1].scope {
        ContextCapabilityScope::Path(path) => assert_eq!(path.to_string(), "users/name"),
        _ => panic!("expected write path capability"),
      }
    }
    _ => panic!("expected context declaration"),
  }
}

#[test]
fn context_capability_paths_reject_invalid_wildcard_placement() {
  assert!(parser::parse("@main := db://main{:read(users*)}").is_err());
  assert!(parser::parse("@main := db://main{:read(users/**)}").is_err());
  assert!(parser::parse("@main := db://main{:read(users/*/name)}").is_err());
  assert!(parser::parse("@main := db://main{:read(*users)}").is_err());
  assert!(parser::parse("@main := db://main{:read(users/*foo)}").is_err());
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
fn program_browser_resource_define_is_rejected() {
  assert!(parser::parse("@browser/title := \"Hello\"").is_err());
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
fn rejects_underscore_in_context_identifier() {
  assert!(parser::parse("@my_ui := browser://dom").is_err());
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


#[test]
fn parses_context_send_statements() {
  let stmts = statements("@out/line <- \"hello\"\n@err/text <- \"warning\"");
  assert_eq!(stmts.len(), 2);
  match &stmts[0] {
    Statement::ContextSend(send) => {
      assert_eq!(send.target.context.as_ref().unwrap().to_string(), "out");
      assert_eq!(send.target.name.to_string(), "line");
    }
    other => panic!("expected context send, got {other:?}"),
  }
  match &stmts[1] {
    Statement::ContextSend(send) => {
      assert_eq!(send.target.context.as_ref().unwrap().to_string(), "err");
      assert_eq!(send.target.name.to_string(), "text");
    }
    other => panic!("expected context send, got {other:?}"),
  }
}

#[test]
fn top_level_context_send_still_parses_after_fsm_rejection() {
  assert!(parser::parse("@out/line <- \"hello\"").is_ok());
}

#[test]
fn rejects_context_send_inside_fsm_statement_transition() {
  assert!(
    parser::parse(
      "#machine(x) -> :start\n:start -> @out/line <- \"hello\"\n."
    )
    .is_err()
  );
}

#[test]
fn rejects_context_send_inside_fsm_block_transition() {
  assert!(
    parser::parse(
      "#machine(x) -> :start\n:start -> { @out/line <- \"hello\" }\n."
    )
    .is_err()
  );
}


#[test]
fn rejects_context_send_inside_function_body() {
  assert!(parser::parse("emit() = result<string> := @out/line <- \"hello\".").is_err());
}

#[test]
fn function_body_without_context_send_still_parses() {
  assert!(parser::parse("emit() = result<string> := result := \"hello\".").is_ok());
}

#[test]
fn rejects_local_send_and_context_definitions() {
  assert!(parser::parse("x <- 5").is_err());
  assert!(parser::parse("@out/line := \"hello\"").is_err());
  assert!(parser::parse("@ui/counter/_text := \"hello\"").is_err());
}

#[test]
fn context_assignment_still_parses() {
  let stmts = statements("@ui/counter/_text = \"hello\"");
  assert!(matches!(stmts.as_slice(), [Statement::VariableAssign(_)]));
}
