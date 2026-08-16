use crate::{CompilerPlanningConfig, CompilerPlanningProgram};

fn run(src: &str) -> mech_core::MResult<CompilerPlanningProgram> {
    let mut p = CompilerPlanningProgram::new(CompilerPlanningConfig::default());
    p.plan_source_for_test(src)?;
    Ok(p)
}

fn id(name: &str) -> mech_core::Identifier {
    mech_core::Identifier {
        name: mech_core::Token {
            kind: mech_core::TokenKind::Identifier,
            chars: name.chars().collect(),
            src_range: mech_core::SourceRange::default(),
        },
    }
}

#[test]
fn direct_context_binding_binds_base_uri() {
    let p = run("@ui := browser://dom").unwrap();
    assert_eq!(
        p.interpreter.context_binding(&id("ui")).unwrap().base_uri,
        "browser://dom"
    );
}

#[test]
fn direct_context_binding_can_copy_context_base() {
    let p = run("@ui := browser://dom\n@child := @ui").unwrap();
    assert_eq!(
        p.interpreter
            .context_binding(&id("child"))
            .unwrap()
            .base_uri,
        "browser://dom"
    );
}

#[test]
fn browser_dom_context_import_binds_base_uri() {
    let p = run("+> @ui := browser/dom").unwrap();
    assert_eq!(
        p.interpreter.context_binding(&id("ui")).unwrap().base_uri,
        "browser://dom"
    );
}

#[test]
fn browser_dom_value_alias_errors() {
    let err = match run("+> ui := browser/dom") {
        Ok(_) => panic!("expected browser/dom value alias to fail"),
        Err(err) => err,
    };
    assert!(format!("{}", err.kind_message()).contains(
        "Module export `browser/dom` is a context export; import it with `+> @name := browser/dom`"
    ));
}
