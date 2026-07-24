use mech_interpreter::*;
use mech_syntax::parser;

fn run(src: &str) -> mech_core::MResult<Interpreter> {
  let program = parser::parse(src).unwrap();
  let p = Interpreter::new_with_full_stdlib(0);
  mech_interpreter::program(&program, &p)?;
  Ok(p)
}

fn id(name: &str) -> mech_core::Identifier {
  mech_core::Identifier { name: mech_core::Token { kind: mech_core::TokenKind::Identifier, chars: name.chars().collect(), src_range: mech_core::SourceRange::default() } }
}

#[test]
fn direct_context_binding_binds_base_uri() {
  let p = run("@ui := browser://dom").unwrap();
  assert_eq!(p.context_binding(&id("ui")).unwrap().base_uri, "browser://dom");
}

#[test]
fn direct_context_binding_can_copy_context_base() {
  let p = run("@ui := browser://dom\n@child := @ui").unwrap();
  assert_eq!(p.context_binding(&id("child")).unwrap().base_uri, "browser://dom");
}

#[test]
fn browser_dom_context_import_binds_base_uri() {
  let p = run("+> @ui := browser/dom").unwrap();
  assert_eq!(p.context_binding(&id("ui")).unwrap().base_uri, "browser://dom");
}

#[test]
fn browser_dom_value_alias_errors() {
  let err = match run("+> ui := browser/dom") {
    Ok(_) => panic!("expected browser/dom value alias to fail"),
    Err(err) => err,
  };
  assert!(format!("{}", err.kind_message()).contains("Module export `browser/dom` is a context export; import it with `+> @name := browser/dom`"));
}

