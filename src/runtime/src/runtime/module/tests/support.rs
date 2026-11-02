use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{
    InMemorySourceResolver, MechRuntime, ModuleBuildOptions, RuntimeBuilder, RuntimeConfig,
};

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
