use mech_core::{LegacyValue, Ref};
use mech_runtime::{
    CapabilityId, FileSourceResolver, InMemoryDocsProvider, ModuleBuildOptions,
    PreparedRuntimeEffect, ResourcePathCapability, RuntimeCapabilityOperation, RuntimeConfigSpec,
    RuntimeInMemoryDocsResourceSpec, RuntimeResourceConfigSpec, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest,
    SourceScope,
};
use std::sync::Arc;

mod support;

fn write_case(root: &std::path::Path, name: &str, source: &str) -> std::path::PathBuf {
    let case_root = root.join(name);
    std::fs::create_dir_all(&case_root).unwrap();
    std::fs::write(case_root.join("main.mec"), source).unwrap();
    case_root
}

fn docs_provider_with(path: &str, value: LegacyValue) -> InMemoryDocsProvider {
    InMemoryDocsProvider::new()
        .with_value("docs://manual", path, value)
        .unwrap()
}

fn run_case(
    root: &std::path::Path,
    name: &str,
    source: &str,
    docs: Option<InMemoryDocsProvider>,
    config_spec: Option<RuntimeConfigSpec>,
    grant_read: bool,
) {
    let case_root = write_case(root, name, source);
    println!("case: {name}");
    println!("root path: {}", case_root.display());

    let mut builder =
        support::source_runtime_builder().source_resolver(FileSourceResolver::new(&case_root));
    if let Some(provider) = docs {
        builder = builder.in_memory_docs(provider);
    }
    if let Some(spec) = config_spec {
        builder = builder.config_spec(spec);
    }
    let mut runtime = builder.build().unwrap();
    if grant_read {
        runtime
            .grant_capability(Arc::new(
                ResourcePathCapability::exact(
                    CapabilityId(1),
                    "task://main",
                    "docs://manual",
                    ["read"],
                    "intro/title",
                )
                .unwrap(),
            ))
            .unwrap();
    }
    let options = ModuleBuildOptions::new("diagnostics", "v0.3", "native", &[], &[]);

    match runtime.resolve_and_store_module_source("main.mec", options) {
        Ok(Some(version)) => {
            println!("main module version: {version}");
            let record = runtime.get_module_version(version).unwrap().unwrap();
            println!("scoped address references:");
            for scope in &record.scopes {
                for reference in &scope.address_references {
                    println!(
                        "  - {:?}: @{}/{}",
                        scope.scope, reference.target, reference.name
                    );
                }
            }
            println!("run result: {:?}", runtime.run_module(version));
            for scope_metadata in &record.scopes {
                if matches!(scope_metadata.scope, SourceScope::Interpreter(_)) {
                    println!(
                        "run {:?} result: {:?}",
                        scope_metadata.scope,
                        runtime.run_module_scope(version, scope_metadata.scope.clone())
                    );
                }
            }
        }
        Ok(None) => {
            println!("main module version: <none>");
            println!("scoped address references: []");
            println!("run result: no module resolved");
        }
        Err(error) => {
            println!("main module version: <resolution failed>");
            println!("scoped address references: <unavailable>");
            println!("run result: resolution error: {:?}", error);
        }
    }
    println!();
}

fn main() {
    let root = std::env::temp_dir().join(format!(
        "mech-address-target-diagnostics-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    println!("root path: {}", root.display());

    let mut provider = InMemoryDocsProvider::new();
    println!("provider write/read:");
    println!("  write docs://manual intro/title = true");
    let effect = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: "docs://manual".to_string(),
            path: "intro/title".to_string(),
            context_name: "manual".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: LegacyValue::Bool(Ref::new(true)),
            intent: RuntimeResourceWriteIntent::Assign,
        })
        .unwrap();
    match effect {
        PreparedRuntimeEffect::Compensatable(mut effect) => effect.apply().unwrap(),
        other => panic!(
            "in-memory docs returned unexpected effect protocol: {:?}",
            other.protocol()
        ),
    }
    let value = provider
        .read(RuntimeResourceReadRequest {
            base_uri: "docs://manual".to_string(),
            path: "intro/title".to_string(),
            context_name: "manual".to_string(),
        })
        .unwrap();
    match value {
        LegacyValue::Bool(value) => println!("  read result: Bool({})", value.borrow()),
        other => println!("  read result: {:?}", other),
    }
    println!();

    run_case(
        &root,
        "@foo/ok works",
        "~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := @foo/ok\n",
        None,
        None,
        false,
    );
    run_case(
        &root,
        "docs://manual intro/title read returns true",
        "@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n",
        Some(docs_provider_with(
            "intro/title",
            LegacyValue::Bool(Ref::new(true)),
        )),
        None,
        true,
    );
    run_case(
        &root,
        "config spec docs://manual intro/title read returns true",
        "@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n",
        None,
        Some(
            RuntimeConfigSpec::new().with_resource(RuntimeResourceConfigSpec::InMemoryDocs(
                RuntimeInMemoryDocsResourceSpec::new("docs://manual")
                    .with_entry("intro/title", LegacyValue::Bool(Ref::new(true))),
            )),
        ),
        true,
    );
    run_case(
        &root,
        "docs read without capability fails CapabilityDenied",
        "@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n",
        Some(docs_provider_with(
            "intro/title",
            LegacyValue::Bool(Ref::new(true)),
        )),
        None,
        false,
    );
    run_case(
        &root,
        "missing docs provider fails",
        "@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n",
        None,
        None,
        true,
    );
    run_case(
        &root,
        "missing docs path fails",
        "@manual := docs://manual{:read(intro/title)}\n\nresult := @manual/intro/title\n",
        Some(InMemoryDocsProvider::new()),
        None,
        true,
    );
    run_case(
        &root,
        "denied docs capability fails",
        "@manual := docs://manual{:read(other/path)}\n\nresult := @manual/intro/title\n",
        Some(docs_provider_with(
            "intro/title",
            LegacyValue::Bool(Ref::new(true)),
        )),
        None,
        true,
    );
    run_case(
        &root,
        "interpreter-scoped docs context works when running interpreter scope",
        "~~~mech:foo\n@manual := docs://manual{:read(intro/title)}\nresult := @manual/intro/title\n~~~\n",
        Some(docs_provider_with(
            "intro/title",
            LegacyValue::Bool(Ref::new(true)),
        )),
        None,
        true,
    );
    run_case(
        &root,
        "interpreter/context conflict fails resolution",
        "~~~mech:foo\nok := true\n<+ ok\n~~~\n\n@foo := docs://foo{:read(ok)}\n",
        None,
        None,
        false,
    );
    run_case(
        &root,
        "unknown target returns UnknownAddressTarget",
        "result := @missing/ok\n",
        None,
        None,
        false,
    );
    run_case(
        &root,
        "string/comment @bar does not execute bar",
        "~~~mech:bar\nbroken := missing\n<+ broken\n~~~\n\ntext := \"@bar\"\n-- @bar\n\nok := true\n",
        None,
        None,
        false,
    );
}
