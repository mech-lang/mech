use std::path::Path;

use mech_core::*;

use crate::{LoadedMechConfig, resolve_config_path, require_config_file, require_config_wasm_package};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServeCliArgs {
  pub paths: Vec<String>,
  pub address: Option<String>,
  pub port: Option<String>,
  pub stylesheet_paths: Vec<String>,
  pub shim: Option<String>,
  pub wasm: Option<String>,
}

impl ServeCliArgs {
  pub(crate) fn from_matches(matches: &clap::ArgMatches) -> Self {
    Self {
      paths: matches
        .get_many::<String>("mech_serve_file_paths")
        .into_iter()
        .flatten()
        .cloned()
        .collect(),
      address: matches.get_one::<String>("address").cloned(),
      port: matches.get_one::<String>("port").cloned(),
      stylesheet_paths: matches
        .get_many::<String>("stylesheet")
        .into_iter()
        .flatten()
        .cloned()
        .collect(),
      shim: matches.get_one::<String>("shim").cloned(),
      wasm: matches.get_one::<String>("wasm").cloned(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EffectiveServeOptions {
  pub address: String,
  pub port: String,
  pub paths: Vec<String>,
  pub stylesheet_paths: Vec<String>,
  pub shim_path: String,
  pub wasm_pkg: String,
}

pub(crate) fn effective_serve_options(
  args: &ServeCliArgs,
  config: Option<&LoadedMechConfig>,
) -> MResult<EffectiveServeOptions> {
  let serve_config = config.and_then(|loaded| loaded.document.serve.as_ref());
  let config_path_to_string = |loaded: &LoadedMechConfig, path: &Path| {
    resolve_config_path(&loaded.base_dir, path)
      .to_string_lossy()
      .to_string()
  };

  let address = args.address.clone()
    .or_else(|| serve_config.and_then(|serve| serve.address.clone()))
    .unwrap_or_else(|| "127.0.0.1".to_string());

  let port = args.port.clone()
    .or_else(|| serve_config.and_then(|serve| serve.port.map(|port| port.to_string())))
    .unwrap_or_else(|| "8081".to_string());

  let cli_shim = args.shim.clone();
  let config_shim = config.and_then(|loaded| {
    loaded.document.serve.as_ref().and_then(|serve| {
      serve
        .shim
        .as_ref()
        .map(|path| config_path_to_string(loaded, path))
    })
  });
  let shim_path = cli_shim
    .clone()
    .or_else(|| config_shim.clone())
    .unwrap_or_default();
  if cli_shim.is_none() {
    if let Some(path) = config_shim.as_ref() {
      require_config_file("serve.shim", Path::new(path))?;
    }
  }

  let cli_wasm = args.wasm.clone();
  let config_wasm = config.and_then(|loaded| {
    loaded.document.serve.as_ref().and_then(|serve| {
      serve
        .wasm
        .as_ref()
        .map(|path| config_path_to_string(loaded, path))
    })
  });
  let wasm_pkg = cli_wasm
    .clone()
    .or_else(|| config_wasm.clone())
    .unwrap_or_default();
  if cli_wasm.is_none() {
    if let Some(path) = config_wasm.as_ref() {
      require_config_wasm_package("serve.wasm", Path::new(path))?;
    }
  }

  let config_stylesheets: Vec<String> = config
    .and_then(|loaded| {
      loaded.document.serve.as_ref().map(|serve| {
        serve
          .stylesheets
          .iter()
          .map(|path| config_path_to_string(loaded, path))
          .collect::<Vec<_>>()
      })
    })
    .unwrap_or_default();
  for path in &config_stylesheets {
    require_config_file("serve.stylesheets", Path::new(path))?;
  }
  let cli_stylesheets = args.stylesheet_paths.clone().into_iter();
  let mut stylesheet_paths = config_stylesheets;
  stylesheet_paths.extend(cli_stylesheets);

  let cli_paths: Vec<String> = args.paths.clone();

  let paths = if !cli_paths.is_empty() {
    cli_paths
  } else {
    config
      .and_then(|loaded| {
        loaded.document.serve.as_ref().map(|serve| {
          serve
            .paths
            .iter()
            .map(|path| config_path_to_string(loaded, path))
            .collect()
        })
      })
      .filter(|paths: &Vec<String>| !paths.is_empty())
      .unwrap_or_default()
  };

  Ok(EffectiveServeOptions {
    address,
    port,
    paths,
    stylesheet_paths,
    shim_path,
    wasm_pkg,
  })
}
