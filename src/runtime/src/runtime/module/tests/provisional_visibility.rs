use crate::{RuntimeEventKind, module_id};

use super::support::{runtime_with_sources, test_module_options};

#[test]
fn explicit_transaction_owns_provisional_graph_visibility() {
    let mut runtime = runtime_with_sources(&[("main.mec", "answer := 42\nanswer\n")]);
    let mut owner = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut owner).unwrap();

    let version = runtime
        .build_module_from_request_with_context(&mut owner, "main.mec", test_module_options())
        .unwrap()
        .unwrap();
    let module = module_id("memory:main.mec");
    let observer = runtime.runtime_context().unwrap();

    assert!(
        runtime
            .get_module_visible(&owner, module)
            .unwrap()
            .is_some(),
    );
    assert!(
        runtime
            .get_module_version_visible(&owner, version)
            .unwrap()
            .is_some(),
    );
    assert!(
        runtime
            .get_module_visible(&observer, module)
            .unwrap()
            .is_none(),
    );
    assert!(
        runtime
            .get_module_version_visible(&observer, version)
            .unwrap()
            .is_none(),
    );
    assert!(runtime.store.get_module(module).unwrap().is_none());
    assert!(runtime.store.get_module_version(version).unwrap().is_none(),);
    runtime.commit_runtime_transaction(&mut owner).unwrap();
    assert!(runtime.store.get_module(module).unwrap().is_some());
    assert!(runtime.store.get_module_version(version).unwrap().is_some(),);
}

#[test]
fn failed_later_build_preserves_earlier_provisional_graph() {
    let mut runtime = runtime_with_sources(&[
        ("earlier.mec", "value := 1\nvalue\n"),
        (
            "later.mec",
            "+> ./later-dependency.mec\n+> ./missing.mec\nvalue := 2\n",
        ),
        ("later-dependency.mec", "value := 3\n<+ value\n"),
    ]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    let earlier = runtime
        .build_module_from_request_with_context(&mut context, "earlier.mec", test_module_options())
        .unwrap()
        .unwrap();

    assert!(
        runtime
            .build_module_from_request_with_context(
                &mut context,
                "later.mec",
                test_module_options(),
            )
            .is_err(),
    );

    assert!(
        runtime
            .get_module_version_visible(&context, earlier)
            .unwrap()
            .is_some(),
    );
    for uri in ["memory:later.mec", "memory:later-dependency.mec"] {
        assert!(
            runtime
                .find_module_by_name_visible(&context, uri)
                .unwrap()
                .is_none(),
            "failed operation retained {uri}",
        );
    }

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert!(runtime.store.get_module_version(earlier).unwrap().is_some(),);
}

#[test]
fn committed_equal_version_emits_no_second_compile_event() {
    let mut runtime = runtime_with_sources(&[("main.mec", "value := 1\nvalue\n")]);
    let first = runtime
        .resolve_and_store_module_source("main.mec", test_module_options())
        .unwrap()
        .unwrap();
    let compiled_before = runtime
        .list_events(None)
        .unwrap()
        .iter()
        .filter(|event| matches!(event.kind, RuntimeEventKind::ModuleCompiled { .. }))
        .count();

    let second = runtime
        .resolve_and_store_module_source("main.mec", test_module_options())
        .unwrap()
        .unwrap();
    let compiled_after = runtime
        .list_events(None)
        .unwrap()
        .iter()
        .filter(|event| matches!(event.kind, RuntimeEventKind::ModuleCompiled { .. }))
        .count();

    assert_eq!(second, first);
    assert_eq!(compiled_after, compiled_before);
}

#[test]
fn equal_publication_from_two_transactions_is_idempotent() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    let mut first = runtime.runtime_context().unwrap();
    let mut second = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut first).unwrap();
    runtime.begin_transaction(&mut second).unwrap();
    let first_version = runtime
        .build_module_from_request_with_context(&mut first, "root.mec", test_module_options())
        .unwrap()
        .unwrap();
    let second_version = runtime
        .build_module_from_request_with_context(&mut second, "root.mec", test_module_options())
        .unwrap()
        .unwrap();
    assert_eq!(second_version, first_version);

    runtime.commit_runtime_transaction(&mut first).unwrap();
    runtime.commit_runtime_transaction(&mut second).unwrap();

    assert!(
        runtime
            .store
            .get_module_version(first_version)
            .unwrap()
            .is_some(),
    );
}
