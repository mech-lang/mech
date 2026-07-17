use clap::{Arg, ArgAction, ArgMatches, Command};
use colored::*;
use mech_core::*;

use crate::cli::outcome::CliOutcome;
use crate::cli::resources::{
    LoadedResource, LoadedStylesheets, ResourceEvent, ResourceFallback, ResourceSource,
    Utf8ConversionError, WebResourceDefaults, load_resource, load_stylesheets,
};
use crate::cli::{capabilities, config, serve_options};
use crate::{MechError, MechServer};

fn render_capability_events(badge: &str, events: &[capabilities::FilesystemCapabilityEvent]) {
    for event in events {
        match event {
            capabilities::FilesystemCapabilityEvent::DefaultGrant {
                path, operations, ..
            } => println!(
                "{badge} Default filesystem grant: {} ({})",
                path.display(),
                operations.join(",")
            ),
            capabilities::FilesystemCapabilityEvent::CliGrant {
                source_flag,
                path,
                operations,
                ..
            } => println!(
                "{badge} {source_flag} filesystem grant: {} ({})",
                path.display(),
                operations.join(",")
            ),
            capabilities::FilesystemCapabilityEvent::ConfigGrant {
                path, operations, ..
            } => println!(
                "{badge} Config filesystem grant: {} ({})",
                path.display(),
                operations.join(",")
            ),
            capabilities::FilesystemCapabilityEvent::NoGrants => {
                println!("{badge} No filesystem grants configured.")
            }
        }
    }
}

fn render_config_event(badge: &str, event: &config::ConfigLoadEvent) {
    match event {
        config::ConfigLoadEvent::DisabledByFlag => println!("{badge} Config loading disabled."),
        config::ConfigLoadEvent::LoadedExplicit { path } => {
            println!("{badge} Loading config… {}", path.display())
        }
        config::ConfigLoadEvent::LoadedDiscovered { path } => {
            println!("{badge} Loading config… {}", path.display())
        }
        config::ConfigLoadEvent::NotFound => {}
    }
}

fn render_resource_events(badge: &str, name: &str, events: &[ResourceEvent]) {
    for event in events {
        match event {
            ResourceEvent::LoadedLocal { path } => {
                println!("{badge} Loaded {name}: {}", path.display())
            }
            ResourceEvent::MissingLocalUsedFallback { path, fallback } => match fallback {
                ResourceFallback::EmbeddedDefault => println!(
                    "{badge} {name} not found: {}; using embedded default",
                    path.display()
                ),
                ResourceFallback::RemoteUrl(url) => println!(
                    "{badge} {name} not found: {}; using fallback {url}",
                    path.display()
                ),
            },
            ResourceEvent::LoadedEmbeddedDefault => {
                println!("{badge} Using embedded default {name}")
            }
            ResourceEvent::LoadedRemoteFallback { url } => {
                println!("{badge} Downloaded fallback {name}: {url}")
            }
        }
    }
}


pub(crate) fn command() -> Command {
    Command::new("serve")
        .about("Serve Mech program over an HTTP server.")
        .arg(
            Arg::new("mech_serve_file_paths")
                .help("Source .mec files, .mecb bytecode files, project folders, or directories")
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the port for the server (8081)"),
        )
        .arg(
            Arg::new("stylesheet")
                .short('s')
                .long("stylesheet")
                .value_name("STYLESHEET")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("Sets the stylesheet for the HTML output"),
        )
        .arg(
            Arg::new("shim")
                .short('m')
                .long("shim")
                .value_name("SHIM")
                .help("Sets the shim for the HTML output"),
        )
        .arg(
            Arg::new("wasm")
                .short('w')
                .long("wasm")
                .value_name("WASM")
                .help("Sets the the path to the wasm package"),
        )
        .arg(
            Arg::new("address")
                .short('a')
                .long("address")
                .value_name("ADDRESS")
                .help("Sets the address of the server (127.0.0.1)"),
        )
        .args(host_delegation_args())
}

#[cfg(feature = "host_delegation_signing")]
fn host_delegation_args() -> Vec<Arg> {
    vec![
        Arg::new("host_delegation_key")
            .long("host-delegation-key")
            .value_name("PATH")
            .num_args(1),
        Arg::new("host_delegation_public_key")
            .long("host-delegation-public-key")
            .value_name("PATH")
            .num_args(1),
        Arg::new("host_delegation_key_id")
            .long("host-delegation-key-id")
            .value_name("ID")
            .num_args(1),
        Arg::new("host_delegation_issuer")
            .long("host-delegation-issuer")
            .value_name("ISSUER")
            .num_args(1),
        Arg::new("host_delegation_subject")
            .long("host-delegation-subject")
            .value_name("SUBJECT")
            .num_args(1),
        Arg::new("host_delegation_audience")
            .long("host-delegation-audience")
            .value_name("AUDIENCE")
            .num_args(1),
        Arg::new("host_delegation_expires_ms")
            .long("host-delegation-expires-ms")
            .value_name("MS")
            .num_args(1),
    ]
}

#[cfg(not(feature = "host_delegation_signing"))]
fn host_delegation_args() -> Vec<Arg> {
    Vec::new()
}

pub(crate) struct ServePlan {
    pub paths: Vec<String>,
    pub address: String,
    pub port: String,
    pub stylesheet_paths: Vec<String>,
    pub shim_path: String,
    pub wasm_pkg: String,
    pub loaded_config: Option<crate::LoadedMechConfig>,
    pub runtime_config: mech_runtime::RuntimeConfig,
    pub host_config: Option<mech_host_browser::BrowserRuntimeInjectionConfig>,
    pub host_config_injection: Option<crate::HostAuthorityInjection>,
    pub config_shim_at_root: bool,
    pub authority: mech_runtime::HostFilesystemAuthority,
    pub capability_events: Vec<capabilities::FilesystemCapabilityEvent>,
    pub config_event: config::ConfigLoadEvent,
    pub resources: WebResourceDefaults,
}

pub(crate) fn prepare(
    args: serve_options::ServeCliArgs,
    matches: &ArgMatches,
    resources: WebResourceDefaults,
) -> MResult<ServePlan> {
    let loaded = config::load_cli_config_report_with_inputs(matches, &args.paths)?;
    let loaded_config = loaded.config;
    let effective = serve_options::effective_serve_options(&args, loaded_config.as_ref())?;

    let default_runtime_patch = mech_runtime::RuntimeConfigPatch::default();
    let runtime_config = crate::apply_runtime_config_patch(
        mech_runtime::RuntimeConfig::default(),
        loaded_config
            .as_ref()
            .map(|loaded| &loaded.document.runtime)
            .unwrap_or(&default_runtime_patch),
    )?;

    let host_config = loaded_config
        .as_ref()
        .map(|loaded| {
            crate::web_runtime_injection_config_from_document(&loaded.document, &runtime_config)
        })
        .transpose()?;

    let config_shim_at_root = loaded_config
        .as_ref()
        .and_then(|loaded| loaded.document.serve.as_ref())
        .and_then(|serve| serve.shim.as_ref())
        .is_some()
        && matches.get_one::<String>("shim").is_none();

    #[cfg(feature = "host_delegation_signing")]
    let host_config_injection = {
        let full_address = format!("{}:{}", effective.address, effective.port);
        serve_host_delegation_injection(
            matches,
            loaded_config.as_ref(),
            &runtime_config,
            &full_address,
        )?
    };

    #[cfg(not(feature = "host_delegation_signing"))]
    let host_config_injection = None;

    let capability_args = capabilities::FilesystemCapabilityArgs::from_matches(matches);
    let authority_build =
        capabilities::build_mech_filesystem_authority(&capability_args, loaded_config.as_ref())?;

    Ok(ServePlan {
        paths: effective.paths,
        address: effective.address,
        port: effective.port,
        stylesheet_paths: effective.stylesheet_paths,
        shim_path: effective.shim_path,
        wasm_pkg: effective.wasm_pkg,
        loaded_config,
        runtime_config,
        host_config,
        host_config_injection,
        config_shim_at_root,
        authority: authority_build.authority,
        capability_events: authority_build.events,
        config_event: loaded.event,
        resources,
    })
}


#[derive(Debug)]
struct LoadedBrowserAssets {
    project_js: &'static str,
    wasm: LoadedResource,
    js: LoadedResource,
}

async fn load_browser_assets(
    authority: &mech_runtime::HostFilesystemAuthority,
    wasm_pkg: &str,
    resources: &WebResourceDefaults,
) -> MResult<LoadedBrowserAssets> {
    let project_js = resources.project_js.ok_or_else(|| MechError::new(GenericError { msg: "browser project.js asset is missing; run scripts/build-mech-browser.sh before mech serve".to_string() }, None).with_compiler_loc())?;
    let wasm_path = format!("{}/mech_wasm_bg.wasm", wasm_pkg);
    let js_path = format!("{}/mech_wasm.js", wasm_pkg);
    let wasm = load_resource(authority, &wasm_path, &resources.wasm_backup_url, resources.mech_wasm).await?;
    let js = load_resource(authority, &js_path, &resources.js_backup_url, resources.mech_js).await?;
    Ok(LoadedBrowserAssets { project_js, wasm, js })
}

pub(crate) async fn run(options: ServePlan) -> MResult<CliOutcome> {
    let badge = "[Mech Server]".truecolor(34, 204, 187);
    let resources = &options.resources;

    if let Some(loaded) = options.loaded_config.as_ref() {
        render_config_event(&badge.to_string(), &options.config_event);
        println!(
            "{badge} Loaded host config entries: {}",
            loaded.document.hosts.len()
        );
    } else {
        render_config_event(&badge.to_string(), &options.config_event);
    }

    render_capability_events(&badge.to_string(), &options.capability_events);

    let full_address = format!("{}:{}", options.address, options.port);

    println!("{badge} Loading resources…");

    print!("{badge} Loading stylesheet…");
    let LoadedStylesheets {
        css: stylesheet_str,
        events,
        local_paths: stylesheet_backing_paths,
    } = load_stylesheets(&options.authority, &options.stylesheet_paths, &resources.stylesheet_backup_url).await?;
    render_resource_events(&badge.to_string(), "stylesheet", &events);

    print!("{badge} Loading HTML shim…");
    let shim = load_resource(
        &options.authority,
        &options.shim_path,
        &resources.shim_backup_url,
        Some(resources.shim_html.as_bytes()),
    )
    .await?;
    render_resource_events(&badge.to_string(), "HTML shim", &shim.events);
    let html_shim_backing_paths = match &shim.source {
        ResourceSource::LocalPath(path) => vec![path.clone()],
        _ => Vec::new(),
    };
    let shim_str = String::from_utf8(shim.bytes).map_err(|e| {
        MechError::new(
            Utf8ConversionError {
                source_error: e.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    })?;

    print!("{badge} Loading WASM…");
    let LoadedBrowserAssets { project_js, wasm, js } = load_browser_assets(&options.authority, &options.wasm_pkg, resources).await?;
    render_resource_events(&badge.to_string(), "WASM", &wasm.events);
    let wasm_backing_paths = match &wasm.source {
        ResourceSource::LocalPath(path) => vec![path.clone()],
        _ => Vec::new(),
    };

    print!("{badge} Loading JS…");
    render_resource_events(&badge.to_string(), "JS", &js.events);
    let js_backing_paths = match &js.source {
        ResourceSource::LocalPath(path) => vec![path.clone()],
        _ => Vec::new(),
    };

    let mut server = MechServer::new_with_runtime_config_and_host_config(
        "Mech Server".to_string(),
        full_address,
        stylesheet_str,
        shim_str,
        include_str!("../../../include/project.html").to_string(),
        project_js.to_string(),
        wasm.bytes,
        js.bytes,
        options.authority,
        options.runtime_config,
        options.host_config,
        options.host_config_injection,
        options.config_shim_at_root,
    );

    server.set_resource_backing_paths(
        html_shim_backing_paths,
        stylesheet_backing_paths,
        wasm_backing_paths,
        js_backing_paths,
    );
    server.init().await?;

    server.load_workspace(&options.paths)?;

    println!("{badge} Sources loaded.");

    server.serve().await?;

    Ok(CliOutcome::success())
}

#[cfg(feature = "host_delegation_signing")]
fn serve_host_delegation_injection(
    matches: &clap::ArgMatches,
    loaded_config: Option<&crate::LoadedMechConfig>,
    runtime_config: &mech_runtime::RuntimeConfig,
    full_address: &str,
) -> MResult<Option<crate::HostAuthorityInjection>> {
    let Some(private_key) = matches.get_one::<String>("host_delegation_key") else {
        return Ok(None);
    };

    let public_key = matches
        .get_one::<String>("host_delegation_public_key")
        .ok_or_else(|| {
            MechError::new(GenericError { msg: "--host-delegation-public-key is required with --host-delegation-key".to_string() }, None).with_compiler_loc()
        })?;

    let Some(loaded_config) = loaded_config else {
        return Err(MechError::new(GenericError { msg: "host delegation signing requires a loaded config".to_string() }, None).with_compiler_loc());
    };

    let current_dir = std::env::current_dir()?;

    let options = crate::HostDelegationSigningOptions {
        private_key_path: current_dir.join(private_key),
        public_key_path: current_dir.join(public_key),
        key_id: matches
            .get_one::<String>("host_delegation_key_id")
            .cloned()
            .unwrap_or_else(|| "dev".to_string()),
        issuer: matches
            .get_one::<String>("host_delegation_issuer")
            .cloned()
            .unwrap_or_else(|| "host://mech-cli".to_string()),
        subject: matches
            .get_one::<String>("host_delegation_subject")
            .cloned()
            .unwrap_or_else(|| "wasm://browser".to_string()),
        audience: matches
            .get_one::<String>("host_delegation_audience")
            .cloned()
            .unwrap_or_else(|| format!("browser://serve/{full_address}")),
        expires_ms: matches
            .get_one::<String>("host_delegation_expires_ms")
            .map(|value| value.parse())
            .transpose()
            .map_err(|_| {
                MechError::new(GenericError { msg: "--host-delegation-expires-ms must be an integer".to_string() }, None).with_compiler_loc()
            })?,
    };

    let host_config =
        crate::web_runtime_injection_config_from_document(&loaded_config.document, runtime_config)?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?
        .as_millis() as u64;

    crate::signed_browser_runtime_injection_config(host_config, &options, now_ms).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_runtime::{DefaultIdGenerator, HostFilesystemAuthority, SharedCapabilityKernel, FS_READ, MECH_TOOL_SUBJECT};

    fn authority_for(path: &std::path::Path) -> HostFilesystemAuthority {
        let mut authority = HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, SharedCapabilityKernel::new());
        let mut ids = DefaultIdGenerator::new();
        authority.grant_path(&mut ids, path, true, [FS_READ]).unwrap();
        authority
    }

    fn defaults(project_js: Option<&'static str>, mech_wasm: Option<&'static [u8]>, mech_js: Option<&'static [u8]>) -> WebResourceDefaults {
        WebResourceDefaults {
            stylesheet_backup_url: "http://unused/style.css".to_string(),
            shim_backup_url: "http://unused/shim.html".to_string(),
            wasm_backup_url: "http://unused/mech_wasm_bg.wasm".to_string(),
            js_backup_url: "http://unused/mech_wasm.js".to_string(),
            shim_html: "",
            mech_wasm,
            mech_js,
            project_js,
        }
    }

    #[test]
    fn local_wasm_package_works_without_embedded_assets() {
        let root = tempfile::tempdir().unwrap();
        let pkg = root.path().join("pkg");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(pkg.join("mech_wasm_bg.wasm"), b"local-wasm").unwrap();
        std::fs::write(pkg.join("mech_wasm.js"), b"local-js").unwrap();
        let authority = authority_for(root.path());
        let assets = tokio::runtime::Runtime::new().unwrap().block_on(load_browser_assets(&authority, pkg.to_str().unwrap(), &defaults(Some("project"), None, None))).unwrap();
        assert_eq!(assets.project_js, "project");
        assert_eq!(assets.wasm.bytes, b"local-wasm");
        assert_eq!(assets.js.bytes, b"local-js");
    }

    #[test]
    fn missing_local_and_embedded_wasm_fails() {
        let root = tempfile::tempdir().unwrap();
        let authority = authority_for(root.path());
        let error = format!("{:?}", tokio::runtime::Runtime::new().unwrap().block_on(load_browser_assets(&authority, root.path().join("missing").to_str().unwrap(), &defaults(Some("project"), None, Some(b"js")))).unwrap_err());
        assert!(error.contains("mech_wasm_bg.wasm") || error.contains("unused"), "{error}");
    }

    #[test]
    fn missing_project_js_fails_independently() {
        let root = tempfile::tempdir().unwrap();
        let pkg = root.path().join("pkg");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(pkg.join("mech_wasm_bg.wasm"), b"local-wasm").unwrap();
        std::fs::write(pkg.join("mech_wasm.js"), b"local-js").unwrap();
        let authority = authority_for(root.path());
        let error = format!("{:?}", tokio::runtime::Runtime::new().unwrap().block_on(load_browser_assets(&authority, pkg.to_str().unwrap(), &defaults(None, None, None))).unwrap_err());
        assert!(error.contains("project.js"), "{error}");
    }
}
