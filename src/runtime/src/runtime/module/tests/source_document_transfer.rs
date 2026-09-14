use crate::{MechRuntime, ModuleBuildOptions, RuntimeConfig};

#[test]
fn direct_source_build_retains_one_canonical_revision_through_the_store() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let source = "  value := 41\r\n";
    let version = runtime
        .put_source_module(
            "main.mec",
            "memory:main.mec",
            source,
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .unwrap();
    let (_, record) = runtime
        .workspace_module_records(version)
        .unwrap()
        .expect("stored module revision");
    let document = record
        .source_document
        .expect("module store retains the canonical source revision");
    assert_eq!(document.source().to_contiguous_string(), source);
    assert_eq!(
        document.source().document().0,
        mech_core::hash_str("memory:main.mec")
    );
    assert_eq!(document.source().revision().0, 0);
    assert!(document.is_strictly_clean());
}

#[test]
fn retained_revision_does_not_cut_shipping_indexing_over_early() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let source = include_str!("../../../../../../examples/gpu-particles/particles.mec");
    let version = runtime
        .put_source_module(
            "main.mec",
            "memory:main.mec",
            source,
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .expect("A retains canonical ownership without changing the shipping index route");
    let (_, record) = runtime
        .workspace_module_records(version)
        .unwrap()
        .expect("stored module revision");
    let document = record
        .source_document
        .expect("module store retains the incomplete canonical revision");
    assert_eq!(document.source().to_contiguous_string(), source);
    assert!(!document.is_strictly_clean());
    assert_eq!(record.contexts.len(), 2);
    assert_eq!(record.contexts[0].name, "pointer");
}
