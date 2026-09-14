//! Browser provider contracts for offline bundle compilation. No host I/O runs.
use mech_core::{MResult, MechError};
use mech_runtime::{
    ConfigValue, HostManifestConfig, MechConfigDocument, RuntimeBuilder, RuntimeConfig,
    RuntimeHostFactory, RuntimeHostInstallation, RuntimeResourceProvider,
};

pub(super) fn compiler_builder(
    document: &MechConfigDocument,
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
    for host in &document.hosts {
        builder = builder.host_instance(host.clone());
    }
    Ok(builder)
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
    super::validation_error("bundle compilation cannot execute browser DOM I/O")
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

pub(super) fn retained_sources(
    paths: &[std::path::PathBuf],
    base: &std::path::Path,
    project: &std::path::Path,
) -> MResult<(
    mech_runtime::InMemorySourceResolver,
    std::collections::HashMap<String, mech_runtime::SourceDocument>,
)> {
    use mech_runtime::resolver::{
        ResolvedSource, import_may_resolve_source_dependency, import_requires_source_dependency,
    };
    let mut resolver = mech_runtime::InMemorySourceResolver::new();
    let mut documents = std::collections::HashMap::new();
    let mut owners = std::collections::HashMap::new();
    for path in paths {
        let relative = super::relative_source_path(path, base, project)?;
        let uri = format!("bundle:///{}", super::bundle_source_specifier(&relative)?);
        let text = std::fs::read_to_string(path)?;
        let document = mech_runtime::SourceDocument::parse_resolved(
            &uri,
            mech_syntax::document::Revision(0),
            std::sync::Arc::<str>::from(text.as_str()),
            mech_syntax::document::ParseConfig::default(),
        )
        .map_err(|error| super::validation_error(format!("invalid bundle source: {error:?}")))?;
        let source = ResolvedSource::new(&uri, &uri, mech_core::MechSourceCode::String(text))
            .with_source_document(document.clone())?
            .admit_canonical_document()?;
        resolver.insert_source(uri.clone(), source)?;
        owners.insert(path.canonicalize()?, uri.clone());
        documents.insert(uri, document);
    }
    for (path, uri) in &owners {
        let index = documents[uri]
            .index()
            .map_err(|error| MechError::new(error, None))?;
        for import in index.root.program_imports() {
            if !import_may_resolve_source_dependency(&import) {
                continue;
            }
            let request = mech_runtime::resolver::source_request_for_import(&import, Some(uri));
            let candidate = path
                .parent()
                .expect("canonical source has a parent")
                .join(&request.specifier);
            let resolved = [candidate.clone(), candidate.with_extension("mec")]
                .into_iter()
                .filter_map(|candidate| candidate.canonicalize().ok())
                .find_map(|candidate| owners.get(&candidate));
            if let Some(target) = resolved {
                resolver.insert_resolution(uri, &request.specifier, target)?;
            } else if import_requires_source_dependency(&import) {
                return Err(super::validation_error(format!(
                    "bundle dependency {} from {uri} is absent from the bundled source set",
                    import.specifier
                )));
            }
        }
    }
    Ok((resolver, documents))
}
