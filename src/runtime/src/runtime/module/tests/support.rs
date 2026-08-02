use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use mech_core::MResult;

use crate::runtime::test_support::providers::{TestAfterCommitEffect, test_runtime_builder};
use crate::{
    BasicCapability, CapabilityId, CapabilityRequest, InMemorySourceResolver, MechRuntime,
    ModuleBuildOptions, ResolvedSource, RuntimeBuilder, RuntimeConfig, RuntimeEffectMetadata,
    RuntimeEffectSource, SourceRequest, SourceResolver,
};

#[derive(Debug)]
pub(super) struct CountingSourceResolver {
    pub(super) inner: InMemorySourceResolver,
    pub(super) calls: Arc<AtomicUsize>,
}

impl SourceResolver for CountingSourceResolver {
    fn resolve(&self, request: &SourceRequest) -> MResult<Option<ResolvedSource>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.resolve(request)
    }
}

pub(super) fn counting_after_commit_effect(deliveries: Arc<AtomicUsize>) -> TestAfterCommitEffect {
    TestAfterCommitEffect::new(
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "after-commit-delivery".to_string(),
            },
            "deliver",
        ),
        move || {
            deliveries.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    )
}

pub(super) fn test_module_options() -> ModuleBuildOptions<'static> {
    ModuleBuildOptions::new("test", "v0.3", "native", &[], &[])
}

pub(super) fn runtime_with_sources(sources: &[(&str, &str)]) -> MechRuntime {
    runtime_builder_with_sources(sources).build().unwrap()
}

pub(super) fn runtime_builder_with_sources(sources: &[(&str, &str)]) -> RuntimeBuilder {
    let mut resolver = InMemorySourceResolver::new();
    for (specifier, source) in sources {
        resolver.insert_string(*specifier, *source).unwrap();
    }
    test_runtime_builder()
        .config(RuntimeConfig::default())
        .source_resolver(resolver)
}

pub(super) fn staged_test_capability(
    runtime: &MechRuntime,
    id: CapabilityId,
) -> (Arc<BasicCapability>, CapabilityRequest) {
    let subject = runtime.runtime_context().unwrap().subject;
    (
        Arc::new(BasicCapability::from_keys(
            id,
            &subject,
            "module-transaction://resource",
            [":read"],
        )),
        CapabilityRequest::from_keys(subject, ":read", "module-transaction://resource"),
    )
}
