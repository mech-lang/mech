//! Shared configured browser planning for static and served compilation. No host I/O runs.
use mech_core::{MResult, MechError};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, HostManifestConfig, RuntimeBuilder, RuntimeConfig,
    RuntimeHostFactory, RuntimeHostInstallation, RuntimeResourceProvider,
};

/// Build the configured planning authority shared by static and served browser products.
pub fn configured_browser_compiler_builder(
    hosts: &[HostInstanceConfig],
    config: RuntimeConfig,
) -> MResult<RuntimeBuilder> {
    let mut builder = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .config(config)
        .host_factory(Box::new(mech_browser::BrowserHostFactory::new(OfflineDom)?))?
        .host_factory(Box::new(ClockFactory(mech_time::time_host_manifest()?)))?
        .host_factory(Box::new(ClockFactory(mech_timer::timer_host_manifest()?)))?
        .host_factory(Box::new(mech_console::ConsoleHostFactory::with_backend(
            mech_console::RecordingConsoleBackend::new(),
        )?))?
        .host_factory(Box::new(mech_scene::SceneHostFactory::with_backend(
            mech_scene::RecordingSceneBackend::new(),
        )?))?;
    #[cfg(feature = "compute_backends_native")]
    {
        builder = builder.host_factory(Box::new(mech_browser::PointerHostFactory::planning()))?;
    }
    for host in hosts {
        if host.provider == "compute" {
            // The compute provider is materialized from the compiled region by
            // the browser. Validate its configuration here without activating it.
            #[cfg(feature = "compute_backends_native")]
            mech_gpu::validate_compute_host_settings(&host.settings)?;
            continue;
        }
        #[cfg(not(feature = "compute_backends_native"))]
        if host.provider == "pointer" {
            continue;
        }
        builder = builder.host_instance(host.clone());
    }
    Ok(builder)
}

/// Compile the browser's admitted coordinator through the canonical root graph.
/// Compute regions use the mixed compiler; both paths retain dependency hashes.
pub(crate) fn compile_browser_document_bundle(
    compiler: &mut mech_runtime::ProgramCompiler,
    uri: &str,
    document: &mech_runtime::SourceDocument,
) -> MResult<mech_runtime::CanonicalProgramBundle> {
    let regions = mech_engine::CanonicalSourceFrontend
        .document_compute_regions(&document.document())
        .map_err(|error| {
            MechError::new(
                mech_core::GenericError {
                    msg: error.to_string(),
                },
                None,
            )
        })?;
    if regions.is_empty() {
        let product =
            compiler.compile_canonical_interactive_root(mech_runtime::SourceRequest::new(uri))?;
        mech_runtime::CanonicalProgramBundle::from_product(uri, document, &product)
    } else {
        #[cfg(feature = "compute_backends_native")]
        {
            let mixed = compiler.compile_canonical_mixed_root(
                mech_runtime::SourceRequest::new(uri),
                mech_runtime::ModuleBuildOptions::new(
                    env!("CARGO_PKG_VERSION"),
                    "v0.4",
                    "browser",
                    &["compute"],
                    &[],
                ),
            )?;
            mech_runtime::CanonicalProgramBundle::from_artifact_product(
                uri,
                document,
                &mixed.coordinator,
                mixed.source_dependencies,
            )
        }
        #[cfg(not(feature = "compute_backends_native"))]
        {
            return Err(MechError::new(
                mech_core::GenericError {
                    msg: "browser compute compilation requires compute_backends_native".into(),
                },
                None,
            ));
        }
    }
}

/// Validate the configured browser program, then transport its retained source
/// and canonical presentation addresses. The browser recompiles that single
/// source authority for its own target instead of decoding a native artifact.
pub(crate) fn compile_browser_document_payload(
    compiler: &mut mech_runtime::ProgramCompiler,
    canonical_uri: &str,
    root_specifier: &str,
    document: &mech_runtime::SourceDocument,
) -> MResult<mech_runtime::BrowserDocumentPayload> {
    compile_browser_document_bundle(compiler, canonical_uri, document)?;
    let presentation_output_ids = mech_runtime::canonical_document_presentation_output_ids(
        &document.document(),
    )
    .map_err(|error| {
        MechError::new(
            mech_core::GenericError {
                msg: error.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    Ok(mech_runtime::BrowserDocumentPayload::new(
        root_specifier,
        document.source().to_contiguous_string(),
    )?
    .with_presentation_output_ids(presentation_output_ids))
}

#[derive(Clone, Debug)]
struct OfflineDom;
impl mech_browser::BrowserDomBackend for OfflineDom {
    fn read_dom_string(
        &self,
        _: &mech_browser::BrowserDomManifestEntry,
        _: &mech_browser::BrowserDomPath,
    ) -> MResult<String> {
        Err(offline_io())
    }
    fn write_dom_string(
        &mut self,
        _: &mech_browser::BrowserDomManifestEntry,
        _: &mech_browser::BrowserDomPath,
        _: &str,
    ) -> MResult<()> {
        Err(offline_io())
    }
}
fn offline_io() -> MechError {
    MechError::new(
        mech_core::GenericError {
            msg: "browser compilation cannot execute DOM I/O".into(),
        },
        None,
    )
    .with_compiler_loc()
}

/// Uses the clock providers' own settings, manifests, and planning snapshots;
/// input drivers belong to the browser that subsequently activates the bundle.
#[derive(Debug)]
struct ClockFactory(HostManifestConfig);
impl RuntimeHostFactory for ClockFactory {
    fn provider_name(&self) -> &str {
        &self.0.provider
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.0
    }
    fn validate_settings(&self, _: &str, settings: &ConfigValue) -> MResult<()> {
        if self.provider_name() == "time" {
            mech_time::time_settings_from_config(settings).map(|_| ())
        } else {
            mech_timer::timer_settings_from_config(settings).map(|_| ())
        }
    }
    fn instantiate(&self, name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(name, settings)?;
        let provider: Box<dyn RuntimeResourceProvider> = if self.provider_name() == "time" {
            Box::new(mech_time::TimeResourceProvider::new(
                name,
                mech_time::new_shared_snapshot(mech_time::TimeSnapshot::default()),
            ))
        } else {
            let settings = mech_timer::timer_settings_from_config(settings)?;
            let initial = mech_timer::TimerSnapshot::new(0, settings.frequency_hz, 0);
            Box::new(
                mech_timer::TimerResourceProvider::new_with_planning_snapshot(
                    name,
                    mech_timer::new_shared_snapshot(initial),
                    initial,
                ),
            )
        };
        Ok(RuntimeHostInstallation {
            interface: mech_runtime::materialize_host_manifest(name, &self.0)?,
            resource_providers: vec![provider],
            input_drivers: Vec::new(),
        })
    }
}

#[cfg(all(test, feature = "compute_backends_native"))]
mod tests {
    use super::*;
    use mech_runtime::{InMemorySourceResolver, SourceDocument};
    use std::collections::BTreeMap;

    #[test]
    fn configured_mixed_browser_bundle_retains_pointer_contracts_and_dependencies() {
        let uri = "bundle:///main.mec";
        let source = "+> ./dep.mec\n@pointer := pointer://mouse/frame{:read(pulse), :read(position)}\n@compute := compute://worker/kernel{:write(input/x), :write(turn)}\n@compute/input/x <- @pointer/position\n@compute/turn <- @pointer/pulse\n\ncalculation @compute\n-------------------\nx := [0f32; 0f32]\nresult := x + dep/value\nresult\n";
        let dependency = "value := 2f32\n<+ value\n";
        let document = SourceDocument::parse_resolved(
            uri,
            mech_syntax::document::Revision(0),
            source,
            Default::default(),
        )
        .unwrap();
        let mut resolver = InMemorySourceResolver::new();
        for (resolved_uri, text) in [(uri, source), ("bundle:///dep.mec", dependency)] {
            let retained = SourceDocument::parse_resolved(
                resolved_uri,
                mech_syntax::document::Revision(0),
                text,
                Default::default(),
            )
            .unwrap();
            resolver
                .insert_source(
                    resolved_uri,
                    mech_runtime::ResolvedSource::new(
                        resolved_uri,
                        resolved_uri,
                        mech_core::MechSourceCode::String(text.into()),
                    )
                    .with_kind(mech_runtime::SourceKind::Mech)
                    .with_source_document(retained)
                    .unwrap()
                    .admit_canonical_document()
                    .unwrap(),
                )
                .unwrap();
        }
        resolver
            .insert_resolution(uri, "./dep.mec", "bundle:///dep.mec")
            .unwrap();
        let hosts = [
            HostInstanceConfig {
                name: "mouse".into(),
                provider: "pointer".into(),
                settings: ConfigValue::Map(Default::default()),
            },
            HostInstanceConfig {
                name: "worker".into(),
                provider: "compute".into(),
                settings: ConfigValue::Map(BTreeMap::from([
                    ("region".into(), ConfigValue::String("calculation".into())),
                    ("backend".into(), ConfigValue::String("auto".into())),
                ])),
            },
        ];
        let mut compiler = configured_browser_compiler_builder(&hosts, RuntimeConfig::default())
            .unwrap()
            .source_resolver(resolver)
            .build_compiler()
            .unwrap();
        let bundle = compile_browser_document_bundle(&mut compiler, uri, &document).unwrap();
        assert_eq!(
            bundle.source_dependencies,
            BTreeMap::from([("bundle:///dep.mec".into(), mech_core::hash_str(dependency))])
        );
        bundle.validate(Some(source)).unwrap();
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bundle.bytecode).unwrap();
        for base in ["pointer://mouse/frame", "compute://worker/kernel"] {
            assert!(artifact.requirements().iter().any(|(_, requirement)| matches!(requirement, mech_core::ApplicationRequirement::Resource(request) if request.base_uri == base)), "{base}");
        }
    }

    #[test]
    fn particle_browser_authority_retains_pointer_and_compute_hosts() {
        let document = mech_runtime::parse_config_document(
            "examples/gpu-particles/mech.mcfg",
            include_str!("../examples/gpu-particles/mech.mcfg"),
            Default::default(),
        )
        .unwrap();
        let authority =
            crate::web_runtime_injection_config_from_document(&document, &RuntimeConfig::default())
                .unwrap();
        for expected in &document.hosts {
            assert!(authority.hosts.contains(expected), "{}", expected.provider);
        }
        for expected in &document.run.as_ref().unwrap().grants {
            assert!(
                authority.run_grants.contains(expected),
                "{}",
                expected.target
            );
        }
    }

    #[test]
    fn invalid_compute_configuration_is_rejected_before_browser_compilation() {
        let host = HostInstanceConfig {
            name: "worker".into(),
            provider: "compute".into(),
            settings: ConfigValue::Map(Default::default()),
        };
        assert!(configured_browser_compiler_builder(&[host], RuntimeConfig::default()).is_err());
    }
}
