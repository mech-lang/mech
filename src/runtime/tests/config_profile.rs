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

#[test]
fn config_profile_extra_helper_argument_rejected() {
    let msg = err_text(
        r#"make-path(root<string>, child<string>) => <string>
  | join-path(root, child).

config := {serve: {paths: [make-path("docs", "reference", "typo")]}}
"#,
    );
    assert!(msg.contains("wrong arity"));
    assert!(msg.contains("expected 2 got 3"));
}

#[test]
fn config_profile_missing_helper_argument_rejected() {
    let msg = err_text(
        r#"make-path(root<string>, child<string>) => <string>
  | join-path(root, child).

config := {serve: {paths: [make-path("docs")]}}
"#,
    );
    assert!(msg.contains("wrong arity"));
    assert!(msg.contains("expected 2 got 1"));
}

#[test]
fn config_profile_multi_arm_helper_rejected() {
    let msg = err_text(
        r#"choose-path(root<string>) => <string>
  | "docs" => "docs/reference"
  | * => "fallback".

config := {serve: {paths: [choose-path("docs")]}}
"#,
    );
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("pattern-dispatched config helper functions are not supported"));
}

#[test]
fn config_profile_match_expression_rejected() {
    let msg = err_text(
        r#"x := 1? | 1 => 2.
config := {:}
"#,
    );
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("match expressions are not supported"));
}

#[test]
fn config_profile_slice_expression_rejected() {
    let msg = err_text("x := [1 2 3]\nconfig := {serve: {port: x[1]}}\n");
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("slice expressions are not supported"));
}

#[test]
fn config_profile_range_expression_rejected() {
    let msg = err_text("config := {serve: {paths: [1..5]}}\n");
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("range expressions are not supported"));
}

#[test]
fn config_profile_invalid_log_level_rejected() {
    let msg = err_text("config := {runtime: {diagnostics: {log-level: \"verbose\"}}}\n");
    assert!(msg.contains("InvalidConfigField"));
    assert!(msg.contains("log-level"));
}

#[test]
fn config_profile_valid_log_levels_accepted() {
    for level in ["error", "warn", "info", "debug", "trace"] {
        let doc = parse(&format!(
            "config := {{runtime: {{diagnostics: {{log-level: \"{level}\"}}}}}}\n"
        ))
        .unwrap();
        assert_eq!(doc.runtime.diagnostics.log_level.as_deref(), Some(level));
    }
}

#[test]
fn config_profile_single_patterned_helper_rejected() {
    let msg = err_text(
        r#"label(x<u64>) => <string>
  | 0 => "zero".

config := {serve: {paths: [label(1)]}}
"#,
    );
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("pattern-dispatched config helper functions are not supported"));
}

#[test]
fn config_profile_duplicate_let_rejected() {
    let msg = err_text(
        r#"root := "docs"
root := "examples"
config := {serve: {paths: [root]}}
"#,
    );
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("binding `root` is defined more than once"));
}

#[test]
fn config_profile_named_call_argument_rejected() {
    let msg = err_text(
        r#"config := {serve: {paths: [join-path(root: "docs", child: "reference")]}}
"#,
    );
    assert!(msg.contains("ConfigProfileViolation"));
    assert!(msg.contains("named function-call arguments are not supported"));
}

#[test]
fn config_profile_browser_section_accepts_valid_grants() {
    let doc = parse(
        r##"config := {
  browser: {
    dom: [
      {selector: "#mech-output", allow: ["write"]}
    ]
    clipboard: [
      {allow: ["write"]}
    ]
    network: [
      {origin: "https://docs.mech-lang.org", allow: ["read"], methods: ["GET"]}
    ]
    storage: [
      {backend: "opfs", scope: "/workspace", allow: ["read", "write", "list"]}
    ]
  }
}
"##,
    )
    .unwrap();

    assert_eq!(doc.browser.grants().len(), 4);
    assert!(doc
        .browser
        .allows_dom("#mech-output", BrowserOperation::Write)
        .is_ok());
    assert!(doc
        .browser
        .allows_clipboard(BrowserOperation::Write)
        .is_ok());
    assert!(doc
        .browser
        .allows_network(
            "https://docs.mech-lang.org",
            Some("GET"),
            BrowserOperation::Read
        )
        .is_ok());
    assert!(doc
        .browser
        .allows_storage(
            BrowserStorageBackend::Opfs,
            "/workspace",
            BrowserOperation::List
        )
        .is_ok());
}

#[test]
fn config_profile_browser_unknown_field_rejected() {
    let msg = err_text("config := {browser: {geolocation: [{allow: [\"read\"]}]}}\n");
    assert!(msg.contains("InvalidConfigField"));
    assert!(msg.contains("unknown browser field `geolocation`"));
}

#[test]
fn config_profile_browser_unknown_operation_rejected() {
    let msg = err_text("config := {browser: {clipboard: [{allow: [\"read\", \"teleport\"]}]}}\n");
    assert!(msg.contains("InvalidConfigField"));
    assert!(msg.contains("unknown browser operation `teleport`"));
}

#[test]
fn config_profile_browser_absence_is_deny_by_default() {
    let doc = parse("config := {serve: {port: 8081}}\n").unwrap();
    assert!(doc.browser.grants().is_empty());
    assert!(doc
        .browser
        .allows_clipboard(BrowserOperation::Read)
        .is_err());
}

#[test]
fn config_profile_browser_invalid_dom_selector_forms_rejected() {
    for selector in [
        "#mech-output, body",
        "body",
        ".mech-output *",
        "#root + body",
        "#root[data-x]",
        "#root:hover",
        "#",
    ] {
        let msg = err_text(&format!(
            "config := {{browser: {{dom: [{{selector: \"{selector}\", allow: [\"write\"]}}]}}}}\n"
        ));
        assert!(msg.contains("InvalidConfigField"), "{selector}: {msg}");
        assert!(msg.contains("browser.dom[0].selector"), "{selector}: {msg}");
    }
}

#[test]
fn config_profile_browser_invalid_network_origins_rejected() {
    for origin in [
        "https://",
        "https://example.com/path",
        "https://example.com?x=1",
        "https://example.com#frag",
        "https://user@example.com",
        "https://*.example.com",
    ] {
        let msg = err_text(&format!(
            "config := {{browser: {{network: [{{origin: \"{origin}\", allow: [\"read\"], methods: [\"GET\"]}}]}}}}\n"
        ));
        assert!(msg.contains("InvalidConfigField"), "{origin}: {msg}");
        assert!(msg.contains("browser.network[0]"), "{origin}: {msg}");
    }
}

#[test]
fn config_profile_browser_resource_specific_operations_rejected() {
    let cases = [
        (
            "clipboard + list",
            "config := {browser: {clipboard: [{allow: [\"list\"]}]}}\n",
            "browser Clipboard grants do not support operation `list`",
        ),
        (
            "dom + invoke",
            "config := {browser: {dom: [{selector: \"#mech-output\", allow: [\"invoke\"]}]}}\n",
            "browser Dom grants do not support operation `invoke`",
        ),
        (
            "network + write",
            "config := {browser: {network: [{origin: \"https://docs.mech-lang.org\", allow: [\"write\"], methods: [\"GET\"]}]}}\n",
            "browser Network grants do not support operation `write`",
        ),
        (
            "storage + watch",
            "config := {browser: {storage: [{backend: \"opfs\", scope: \"/workspace\", allow: [\"watch\"]}]}}\n",
            "browser Storage grants do not support operation `watch`",
        ),
    ];

    for (label, source, expected) in cases {
        let msg = err_text(source);
        assert!(msg.contains("InvalidConfigField"), "{label}: {msg}");
        assert!(msg.contains(expected), "{label}: {msg}");
    }
}

#[test]
fn config_profile_browser_storage_recursive_scope_is_explicit() {
    let exact = parse(
        r#"config := {
  browser: {
    storage: [
      {backend: "opfs", scope: "/workspace", allow: ["read"]}
    ]
  }
}
"#,
    )
    .unwrap();
    assert!(exact
        .browser
        .allows_storage(
            BrowserStorageBackend::Opfs,
            "/workspace",
            BrowserOperation::Read
        )
        .is_ok());
    assert!(matches!(
        exact.browser.allows_storage(
            BrowserStorageBackend::Opfs,
            "/workspace/main.mec",
            BrowserOperation::Read
        ),
        Err(BrowserCapabilityError::NoMatchingGrant { .. })
    ));

    let recursive = parse(
        r#"config := {
  browser: {
    storage: [
      {backend: "opfs", scope: "/workspace", recursive: true, allow: ["read"]}
    ]
  }
}
"#,
    )
    .unwrap();
    assert!(recursive
        .browser
        .allows_storage(
            BrowserStorageBackend::Opfs,
            "/workspace/main.mec",
            BrowserOperation::Read
        )
        .is_ok());
    assert!(matches!(
        recursive.browser.allows_storage(
            BrowserStorageBackend::Opfs,
            "/workspace2/main.mec",
            BrowserOperation::Read
        ),
        Err(BrowserCapabilityError::NoMatchingGrant { .. })
    ));
}

#[test]
fn config_profile_browser_dom_path_manifest_lowers() {
    let doc = parse(r##"config := {browser: {dom: [{path: "body/header/title", selector: "#title", property: "text", allow: ["read", "write"]}]}}"##).unwrap();
    let entry = &doc.browser.dom_manifest()[0];
    assert_eq!(entry.path.as_str(), "body/header/title");
    assert_eq!(entry.selector.selector, "#title");
    assert_eq!(entry.property, BrowserDomProperty::Text);
}

#[test]
fn config_profile_browser_dom_path_infers_text_property() {
    let doc = parse(r##"config := {browser: {dom: [{path: "body/header/title", selector: "#title", allow: ["read"]}]}}"##).unwrap();
    assert_eq!(doc.browser.dom_manifest()[0].property, BrowserDomProperty::Text);
}

#[test]
fn config_profile_browser_dom_attribute_path_infers_attribute() {
    let doc = parse(r##"config := {browser: {dom: [{path: "body/status/_class", selector: "#status", allow: ["read"]}]}}"##).unwrap();
    assert_eq!(doc.browser.dom_manifest()[0].property, BrowserDomProperty::Attribute("class".to_string()));
}

#[test]
fn config_profile_browser_dom_attribute_property_requires_attribute() {
    let msg = err_text(r##"config := {browser: {dom: [{path: "body/status", selector: "#status", property: "attribute", allow: ["read"]}]}}"##);
    assert!(msg.contains("requires an `attribute` name"), "{msg}");
}

#[test]
fn config_profile_browser_dom_subtree_requires_wildcard_path() {
    let msg = err_text(r##"config := {browser: {dom: [{path: "body/content", selector: "#content", mode: "subtree", allow: ["read"]}]}}"##);
    assert!(msg.contains("must end in `/*`"), "{msg}");
}

#[test]
fn config_profile_browser_dom_wildcard_path_requires_final_star() {
    let msg = err_text(r##"config := {browser: {dom: [{path: "body/*/title", selector: "#content", allow: ["read"]}]}}"##);
    assert!(msg.contains("wildcard `*` must be its own final segment") || msg.contains("only allowed as the final segment"), "{msg}");
}

#[test]
fn config_profile_browser_dom_duplicate_path_last_wins() {
    let doc = parse(r##"config := {browser: {dom: [
      {path: "body/title", selector: "#old", allow: ["read"]}
      {path: "body/title", selector: "#new", allow: ["read"]}
    ]}}"##).unwrap();
    assert_eq!(doc.browser.dom_manifest().len(), 1);
    assert_eq!(doc.browser.dom_manifest()[0].selector.selector, "#new");
}
