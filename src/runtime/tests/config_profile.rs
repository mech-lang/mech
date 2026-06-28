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
fn shipped_browser_examples_use_hosts_schema() {
    let demo = parse_config_document(
        "examples/browser-dom-demo/demo.mcfg",
        include_str!("../../../examples/browser-dom-demo/demo.mcfg"),
        ConfigProfileOptions::default(),
    )
    .unwrap();
    assert!(demo.hosts.iter().any(|host| host.name == "browser" && host.provider == "browser"));
    assert_eq!(demo.run.as_ref().unwrap().grants.len(), 2);

    let resource = parse_config_document(
        "examples/browser-dom-resource.mcfg",
        include_str!("../../../examples/browser-dom-resource.mcfg"),
        ConfigProfileOptions::default(),
    )
    .unwrap();
    assert!(resource.hosts.iter().any(|host| host.name == "browser" && host.provider == "browser"));
    assert_eq!(resource.run.as_ref().unwrap().grants.len(), 2);
}

#[test]
fn shipped_browser_demo_sources_use_browser_dom_host_import() {
    for (path, source) in [
        (
            "examples/browser-dom-demo/demo.mec",
            include_str!("../../../examples/browser-dom-demo/demo.mec"),
        ),
        (
            "examples/browser-dom-demo/denied.mec",
            include_str!("../../../examples/browser-dom-demo/denied.mec"),
        ),
        (
            "examples/browser-dom-resource.mec",
            include_str!("../../../examples/browser-dom-resource.mec"),
        ),
    ] {
        let first_line = source.lines().find(|line| !line.trim().is_empty()).unwrap_or("");
        assert_eq!(first_line, "+> @browser := browser/dom", "{path} must start with a browser/dom host import");
        assert!(
            !source.contains("@browser := browser://dom/"),
            "{path} must not use a raw browser://dom/ context declaration"
        );
    }
}

#[test]
fn browser_host_crate_does_not_depend_on_robot_arm_host() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let browser_cargo = std::fs::read_to_string(workspace_root.join("hosts/browser/Cargo.toml")).unwrap();
    assert!(
        !browser_cargo.contains("mech-host-robot-arm"),
        "hosts/browser/Cargo.toml must not depend on mech-host-robot-arm"
    );
    assert!(
        !browser_cargo.contains("host-robot-arm"),
        "hosts/browser/Cargo.toml must not define a host-robot-arm feature"
    );

    fn visit(path: &std::path::Path, source: &mut String) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit(&path, source);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                source.push_str(&std::fs::read_to_string(path).unwrap());
            }
        }
    }

    let mut browser_source = String::new();
    visit(&workspace_root.join("hosts/browser/src"), &mut browser_source);
    assert!(
        !browser_source.contains(&["mech", "_host", "_robot", "_arm"].concat()),
        "hosts/browser source must not reference mech-host-robot-arm"
    );
    assert!(
        !browser_source.contains("\"robot-arm\""),
        "hosts/browser source must not know about the robot-arm provider"
    );
    assert!(
        !browser_source.contains("\"cli\""),
        "hosts/browser source must not know about the CLI provider"
    );

    let root_cargo = std::fs::read_to_string(workspace_root.join("Cargo.toml")).unwrap();
    assert!(
        !root_cargo.lines().any(|line| line.trim_start().starts_with("host-robot-arm =")),
        "root Cargo.toml must not define a host-robot-arm feature"
    );

    let wasm_cargo = std::fs::read_to_string(workspace_root.join("src/wasm/Cargo.toml")).unwrap();
    assert!(
        !wasm_cargo.lines().any(|line| line.trim_start().starts_with("host-robot-arm =")),
        "src/wasm/Cargo.toml must not define a host-robot-arm feature"
    );
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
fn config_profile_hosts_browser_settings_preserved() {
    let doc = parse(r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/header/title"
            selector: "#title"
            property: "text"
            operations: ["read", "write"]
          }
        ]
      }
    }
  ]
}
"##).unwrap();

    assert_eq!(doc.hosts.len(), 1);
    assert_eq!(doc.hosts[0].name, "browser");
    assert_eq!(doc.hosts[0].provider, "browser");

    let ConfigValue::Map(settings) = &doc.hosts[0].settings else {
        panic!("expected browser settings map");
    };
    assert!(settings.contains_key("dom"));
}

#[test]
fn config_profile_top_level_browser_is_rejected() {
    let err = parse(r##"
config := {
  browser: {
    dom: [
      { path: "body/header/title" selector: "#title" property: "text" allow: ["read"] }
    ]
  }
}
"##).unwrap_err();

    let error = format!("{err:?}");
    assert!(error.contains("unknown top-level config field `browser`"), "got {error}");
}

#[test]
fn browser_host_manifest_config_parses_and_lowers() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../hosts/browser/host.mcfg");
    let source = std::fs::read_to_string(path).unwrap();

    let doc = parse_config_document(
        "hosts/browser/host.mcfg",
        &source,
        ConfigProfileOptions::default(),
    ).unwrap();

    let host = doc.host.unwrap();
    assert_eq!(host.provider, "browser");
    assert_eq!(host.contexts.len(), 1);
    assert_eq!(host.contexts[0].name, "dom");
    assert_eq!(host.contexts[0].base_uri_template, "browser://{instance}/dom");
    assert_eq!(host.contexts[0].operations, vec!["read".to_string(), "write".to_string()]);
}
