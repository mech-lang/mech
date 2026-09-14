use crate::{
    MechRuntime, ModuleBuildOptions, ModuleBuilder, ResolvedSource, RuntimeConfig, SourceDocument,
    SourceKind,
};
use mech_core::MechSourceCode;
use mech_syntax::document::{ParseConfig, Revision};

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
fn successful_and_rejected_runtime_replacements_preserve_revision_history() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let options = ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]);
    let first = runtime
        .put_canonical_source_module("main.mec", "memory:main.mec", "value := 1\n", options)
        .unwrap();
    let second = runtime
        .put_canonical_source_module("main.mec", "memory:main.mec", "value := 2\n", options)
        .unwrap();
    let first_document = runtime
        .workspace_module_records(first)
        .unwrap()
        .unwrap()
        .1
        .source_document
        .unwrap();
    let second_document = runtime
        .workspace_module_records(second)
        .unwrap()
        .unwrap()
        .1
        .source_document
        .unwrap();
    assert_eq!(
        first_document.source().document(),
        second_document.source().document()
    );
    assert_eq!(first_document.source().revision(), Revision(0));
    assert_eq!(second_document.source().revision(), Revision(1));
    assert_eq!(
        first_document.source().to_contiguous_string(),
        "value := 1\n"
    );
    assert_eq!(
        second_document.source().to_contiguous_string(),
        "value := 2\n"
    );

    assert!(
        runtime
            .put_canonical_source_module("main.mec", "memory:main.mec", "value := [\n", options)
            .is_err()
    );
    let third = runtime
        .put_canonical_source_module("main.mec", "memory:main.mec", "value := 3\n", options)
        .unwrap();
    let third_document = runtime
        .workspace_module_records(third)
        .unwrap()
        .unwrap()
        .1
        .source_document
        .unwrap();
    assert_eq!(third_document.source().revision(), Revision(2));
}

#[test]
fn resolved_source_admission_seeds_runtime_replacement_revision() {
    for through_resolver in [false, true] {
        let options = ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]);
        let resolved = ResolvedSource::new(
            "main.mec",
            "memory:main.mec",
            MechSourceCode::String("value := 1\n".into()),
        )
        .with_kind(SourceKind::Mech)
        .retain_source_document(Revision(7), ParseConfig::default())
        .unwrap()
        .admit_canonical_document()
        .unwrap();
        #[derive(Debug)]
        struct Resolver(ResolvedSource);
        impl crate::SourceResolver for Resolver {
            fn resolve(
                &self,
                _: &crate::SourceRequest,
            ) -> mech_core::MResult<Option<ResolvedSource>> {
                Ok(Some(self.0.clone()))
            }
        }
        let mut runtime = MechRuntime::builder()
            .source_resolver(Resolver(resolved.clone()))
            .build()
            .unwrap();
        let first = if through_resolver {
            runtime
                .resolve_and_store_module_source("memory:main.mec", options)
                .unwrap()
                .unwrap()
        } else {
            runtime
                .store_resolved_module_source(resolved, options)
                .unwrap()
        };
        let next = runtime
            .put_canonical_source_module("main.mec", "memory:main.mec", "value := 2\n", options)
            .unwrap();
        let old = runtime
            .workspace_module_records(first)
            .unwrap()
            .unwrap()
            .1
            .source_document
            .unwrap();
        let new = runtime
            .workspace_module_records(next)
            .unwrap()
            .unwrap()
            .1
            .source_document
            .unwrap();
        assert_eq!(old.source().document(), new.source().document());
        assert_eq!(old.source().revision(), Revision(7));
        assert_eq!(new.source().revision(), Revision(8));
    }
}

#[test]
fn repeated_and_reverted_source_preserve_distinct_stored_revisions() {
    for canonical in [false, true] {
        let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
        let options = ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]);
        let mut versions = Vec::new();
        for (revision, text) in [
            "value := 1\n",
            "value := 1\n",
            "value := 2\n",
            "value := 1\n",
        ]
        .into_iter()
        .enumerate()
        {
            let version = if canonical {
                runtime.put_canonical_source_module("main.mec", "memory:main.mec", text, options)
            } else {
                runtime.put_source_module("main.mec", "memory:main.mec", text, options)
            }
            .unwrap();
            assert!(!versions.contains(&version));
            versions.push(version);
            let document = runtime
                .workspace_module_records(version)
                .unwrap()
                .unwrap()
                .1
                .source_document
                .unwrap();
            assert_eq!(document.source().revision(), Revision(revision as u64));
            assert_eq!(document.source().to_contiguous_string(), text);
        }
        let first = runtime
            .workspace_module_records(versions[0])
            .unwrap()
            .unwrap()
            .1
            .source_document
            .unwrap();
        assert_eq!(first.source().revision(), Revision(0));
        assert_eq!(first.source().to_contiguous_string(), "value := 1\n");
    }
}

#[test]
fn transaction_revisions_commit_together_and_abort_without_consuming_history() {
    for commit in [false, true] {
        let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
        let options = ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]);
        runtime
            .put_canonical_source_module("main.mec", "memory:main.mec", "value := 0\n", options)
            .unwrap();
        let mut context = runtime.runtime_context().unwrap();
        runtime.begin_transaction(&mut context).unwrap();
        for revision in [1, 2] {
            let version = runtime
                .put_canonical_source_module_with_context(
                    &mut context,
                    "main.mec",
                    "memory:main.mec",
                    &format!("value := {revision}\n"),
                    options,
                )
                .unwrap();
            let document = runtime
                .get_module_version_visible(&context, version)
                .unwrap()
                .unwrap()
                .source_document
                .unwrap();
            assert_eq!(document.source().revision(), Revision(revision));
        }
        if commit {
            runtime.commit_runtime_transaction(&mut context).unwrap();
        } else {
            runtime
                .abort_runtime_transaction(&mut context, "discard candidate")
                .unwrap();
        }
        let version = runtime
            .put_canonical_source_module("main.mec", "memory:main.mec", "value := 3\n", options)
            .unwrap();
        let document = runtime
            .workspace_module_records(version)
            .unwrap()
            .unwrap()
            .1
            .source_document
            .unwrap();
        assert_eq!(
            document.source().revision(),
            Revision(if commit { 3 } else { 1 })
        );
    }
}

#[test]
fn invalid_canonical_authority_never_falls_back_to_an_available_legacy_tree() {
    let source = include_str!("../../../../../../examples/gpu-particles/particles.mec");
    let document = SourceDocument::parse_resolved(
        "memory:main.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(!document.is_strictly_clean());
    let resolved = ResolvedSource::new(
        "main.mec",
        "memory:main.mec",
        MechSourceCode::String(source.to_owned()),
    )
    .with_kind(SourceKind::Mech)
    .with_source_document(document)
    .unwrap()
    .with_syntax_tree(mech_syntax::parser::parse(source.trim()).unwrap());
    assert!(resolved.syntax_tree.is_some());
    assert!(resolved.canonical_document_index().is_err());

    let record = ModuleBuilder::new()
        .build_resolved_source(resolved, "test", "v0.4", "native", &[], &[], &[])
        .unwrap();
    assert!(record.syntax_tree.is_some());
    assert!(record.canonical_document_index().is_err());
}
