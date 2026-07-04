use clap::ArgMatches;
use colored::*;
use mech_core::*;

use crate::cli::app::Utf8ConversionError;
use crate::cli::{capabilities, config};
use crate::{MechError, MechServer, read_or_download};

pub(crate) struct ServeResources<'a> {
    pub stylesheet_backup_url: &'a str,
    pub shim_backup_url: &'a str,
    pub wasm_backup_url: &'a str,
    pub js_backup_url: &'a str,
    pub shim_html: &'a str,
    pub mech_wasm: &'a [u8],
    pub mech_js: &'a [u8],
}

pub(crate) async fn run(matches: &ArgMatches, resources: ServeResources<'_>) -> MResult<()> {
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
        crate::cli::app::load_stylesheets(&stylesheet_paths, resources.stylesheet_backup_url)
            .await?;

    print!("{badge} Loading HTML shim…");
    let shim = read_or_download(
        shim_path,
        resources.shim_backup_url,
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
        resources.wasm_backup_url,
        Some(resources.mech_wasm),
    )
    .await?;

    print!("{badge} Loading JS…");
    let js = read_or_download(&js_path, resources.js_backup_url, Some(resources.mech_js)).await?;

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
        std::process::exit(1);
    }

    println!("{badge} Sources loaded.");

    server.serve().await?;

    Ok(())
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
