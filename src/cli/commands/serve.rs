use clap::{Arg, ArgAction, ArgMatches, Command};
use colored::*;
use mech_core::*;

use crate::cli::outcome::CliOutcome;
use crate::cli::resources::{Utf8ConversionError, WebResourceDefaults, load_stylesheets};
use crate::cli::{capabilities, config};
use crate::{MechError, MechServer, read_or_download};

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

pub(crate) struct ServeOptions {
    pub matches: ArgMatches,
    pub resources: WebResourceDefaults,
}

impl ServeOptions {
    pub(crate) fn from_matches(
        matches: &ArgMatches,
        resources: WebResourceDefaults,
    ) -> MResult<Self> {
        Ok(Self {
            matches: matches.clone(),
            resources,
        })
    }
}

pub(crate) async fn run(options: ServeOptions) -> MResult<CliOutcome> {
    let matches = &options.matches;
    let resources = &options.resources;
    let badge = "[Mech Server]".truecolor(34, 204, 187);
    let error_badge = "[Error]".truecolor(246, 98, 78);

    let loaded_config = config::load_cli_config(matches)?;
    let effective = config::effective_serve_options(matches, loaded_config.as_ref())?;

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

    if let Some(loaded) = loaded_config.as_ref() {
        println!(
            "{badge} Loaded host config entries: {}",
            loaded.document.hosts.len()
        );
    }

    let full_address = format!("{}:{}", effective.address, effective.port);

    #[cfg(feature = "host_delegation_signing")]
    let host_config_injection = serve_host_delegation_injection(
        matches,
        loaded_config.as_ref(),
        &runtime_config,
        &full_address,
    )?;

    #[cfg(not(feature = "host_delegation_signing"))]
    let host_config_injection = None;

    let mech_paths = effective.paths;
    let stylesheet_paths = effective.stylesheet_paths;
    let wasm_pkg = effective.wasm_pkg.as_str();
    let shim_path = effective.shim_path.as_str();

    let wasm_path = format!("{wasm_pkg}/mech_wasm_bg.wasm.br");
    let js_path = format!("{wasm_pkg}/mech_wasm.js");

    println!("{badge} Loading resources…");

    print!("{badge} Loading stylesheet…");
    let stylesheet_str =
        load_stylesheets(&stylesheet_paths, &resources.stylesheet_backup_url).await?;

    print!("{badge} Loading HTML shim…");
    let shim = read_or_download(
        shim_path,
        &resources.shim_backup_url,
        Some(resources.shim_html.as_bytes()),
    )
    .await?;

    let shim_str = String::from_utf8(shim).map_err(|e| {
        MechError::new(
            Utf8ConversionError {
                source_error: e.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    })?;

    print!("{badge} Loading WASM…");
    let wasm = read_or_download(
        &wasm_path,
        &resources.wasm_backup_url,
        Some(resources.mech_wasm),
    )
    .await?;

    print!("{badge} Loading JS…");
    let js = read_or_download(&js_path, &resources.js_backup_url, Some(resources.mech_js)).await?;

    let authority =
        capabilities::build_mech_filesystem_authority(matches, loaded_config.as_ref(), &badge)?;

    let mut server = MechServer::new_with_runtime_config_and_host_config(
        "Mech Server".to_string(),
        full_address,
        stylesheet_str,
        shim_str,
        wasm,
        js,
        authority,
        runtime_config,
        host_config,
        host_config_injection,
        config_shim_at_root,
    );

    server.init().await?;

    if let Err(err) = server.load_workspace(&mech_paths) {
        println!("{error_badge} {err:#?}");
        return Ok(CliOutcome::exit(1));
    }

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
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--host-delegation-public-key is required with --host-delegation-key",
            )
        })?;

    let Some(loaded_config) = loaded_config else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "host delegation signing requires a loaded config",
        )
        .into());
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
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "--host-delegation-expires-ms must be an integer",
                )
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
