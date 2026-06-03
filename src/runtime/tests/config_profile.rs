use std::path::PathBuf;

use mech_runtime::*;

fn parse(source: &str) -> mech_core::MResult<MechConfigDocument> {
    parse_config_document("mech.mcfg", source, ConfigProfileOptions::default())
}

fn err_text(source: &str) -> String {
    let err = parse(source).expect_err("expected config parse to fail");
    format!("{} {} {:?}", err.kind_name(), err.kind_message(), err)
}

#[test]
fn config_profile_rich_mechdown_config_parses_and_lowers() {
    let doc = parse(
        r#"Project config

Some docs.

config := {
  serve: {
    paths: ["docs/reference"]
    port: 8081
  }

  capabilities: [
    {allow: "read", path: "docs/reference"}
    {allow: "watch", path: "docs/reference"}
    {allow: "serve", path: "docs/reference"}
  ]
}
"#,
    )
    .unwrap();

    let serve = doc.serve.unwrap();
    assert_eq!(serve.paths, vec![PathBuf::from("docs/reference")]);
    assert_eq!(serve.port, Some(8081));
    assert_eq!(doc.capabilities.len(), 3);
    assert_eq!(doc.capabilities[0].kind, ConfigCapabilityKind::Read);
    assert_eq!(doc.capabilities[1].kind, ConfigCapabilityKind::Watch);
    assert_eq!(doc.capabilities[2].kind, ConfigCapabilityKind::Serve);
}

#[test]
fn config_profile_markdown_code_block_faux_config_fence_is_ignored() {
    let doc = parse(
        r#"```text
~~~mech:config
config := {serve: {port: "bad"}}
~~~
```

~~~mech:config
config := {serve: {port: 8081}}
~~~
"#,
    )
    .unwrap();
    assert_eq!(doc.serve.unwrap().port, Some(8081));
}

#[test]
fn config_profile_mutable_binding_rejected() {
    let msg = err_text("~config := {:}\n");
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("mutable bindings are not allowed"));
}

#[test]
fn config_profile_import_rejected() {
    let msg = err_text("+> math/*\n");
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("imports are not allowed"));
}

#[test]
fn config_profile_context_declaration_rejected() {
    let msg = err_text("@manual := docs://manual{:read(intro)}\n");
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("context declarations are not allowed"));
}

#[test]
fn config_profile_state_machine_rejected() {
    let msg = err_text(
        "#Counter(n<u64>) => <u64>\n  ├ :Count(n<u64>)\n  └ :Done(n<u64>).\n\nconfig := {:}\n",
    );
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("state machines are not allowed"));
}

#[test]
fn config_profile_assignment_rejected() {
    let msg = err_text("config := {:}\nconfig = {:}\n");
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("assignment is not allowed"));
}

#[test]
fn config_profile_op_assignment_rejected() {
    let msg = err_text("x := 1\nx += 1\nconfig := {:}\n");
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("op assignment is not allowed"));
}

#[test]
fn config_profile_pure_function_allowed() {
    let doc = parse(
        r#"make-path(root<string>, child<string>) => <string>
  | join-path(root, child).

config := {serve: {paths: [make-path("docs", "reference")]}}
"#,
    )
    .unwrap();
    assert_eq!(
        doc.serve.unwrap().paths,
        vec![PathBuf::from("docs/reference")]
    );
}

#[test]
fn config_profile_direct_recursion_rejected() {
    let msg = err_text(
        r#"loop(x<u64>) => <u64>
  | loop(x).

config := {serve: {port: 8081}}
"#,
    );
    assert!(msg.contains("ConfigRecursionNotAllowed"));
}

#[test]
fn config_profile_indirect_recursion_rejected() {
    let msg = err_text(
        r#"a(x<u64>) => <u64>
  | b(x).

b(x<u64>) => <u64>
  | a(x).

config := {serve: {port: 8081}}
"#,
    );
    assert!(msg.contains("ConfigRecursionNotAllowed"));
}

#[test]
fn config_profile_wrong_field_type_rejected() {
    let msg = err_text("config := {serve: {port: \"8081\"}}\n");
    assert!(msg.contains("InvalidConfigField"));
    assert!(msg.contains("serve.port"));
}

#[test]
fn config_profile_unknown_field_rejected() {
    let msg = err_text("config := {serve: {banana: true}}\n");
    assert!(msg.contains("InvalidConfigField"));
    assert!(msg.contains("unknown serve field `banana`"));
}

#[test]
fn config_profile_missing_config_binding_rejected() {
    let msg = err_text("x := 1\n");
    assert!(msg.contains("MissingConfigBinding"));
}
