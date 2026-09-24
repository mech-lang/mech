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
    for host in hosts
        .iter()
        .filter(|host| browser_document_compiler_host(host))
    {
        builder = builder.host_instance(host.clone());
    }
    Ok(builder)
}

fn browser_document_compiler_host(host: &HostInstanceConfig) -> bool {
    !matches!(host.provider.as_str(), "compute" | "pointer")
}

/// Compile the browser's admitted coordinator through the canonical root graph.
pub(crate) fn compile_browser_document_bundle(
    compiler: &mut mech_runtime::ProgramCompiler,
    uri: &str,
    document: &mech_runtime::SourceDocument,
) -> MResult<mech_runtime::CanonicalProgramBundle> {
    let product =
        compiler.compile_canonical_interactive_root(mech_runtime::SourceRequest::new(uri))?;
    mech_runtime::CanonicalProgramBundle::from_product(uri, document, &product)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_compute_hosts_do_not_enter_the_document_compiler() {
        let hosts = [
            HostInstanceConfig {
                name: "pointer".to_owned(),
                provider: "pointer".to_owned(),
                settings: ConfigValue::Map(Default::default()),
            },
            HostInstanceConfig {
                name: "particles".to_owned(),
                provider: "compute".to_owned(),
                settings: ConfigValue::Map(Default::default()),
            },
            HostInstanceConfig {
                name: "clock".to_owned(),
                provider: "timer".to_owned(),
                settings: ConfigValue::Map(Default::default()),
            },
        ];

        configured_browser_compiler_builder(&hosts, RuntimeConfig::default())
            .unwrap()
            .build_compiler()
            .unwrap();
    }
}
