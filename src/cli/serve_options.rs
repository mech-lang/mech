use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
  pub project_root: Option<PathBuf>,
  pub uses_configured_paths: bool,
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
  let sole_discovery_selector = config
    .and_then(|loaded| loaded.discovered_project_dir.as_ref().map(|project_dir| (loaded, project_dir)))
    .map(|(_loaded, project_dir)| {
      if cli_paths.len() != 1 {
        return false;
      }
      let selector = Path::new(&cli_paths[0]);
      let selector_abs = if selector.is_absolute() {
        selector.to_path_buf()
      } else {
        std::env::current_dir().unwrap_or_else(|_| Path::new("").to_path_buf()).join(selector)
      };
      let selector_resolved = selector_abs.canonicalize().unwrap_or(selector_abs);
      let project_resolved = project_dir.canonicalize().unwrap_or_else(|_| project_dir.clone());
      selector_resolved == project_resolved
    })
    .unwrap_or(false);

  let project_root = config
    .map(|loaded| {
      loaded
        .discovered_project_dir
        .clone()
        .unwrap_or_else(|| loaded.base_dir.clone())
    })
    .map(|path| path.canonicalize().unwrap_or(path));

  let config_workspace_paths = || {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    if let Some(loaded) = config {
      for path in loaded
        .document
        .run
        .iter()
        .flat_map(|run| &run.paths)
        .chain(
          loaded
            .document
            .serve
            .iter()
            .flat_map(|serve| &serve.paths),
        )
      {
        let path = config_path_to_string(loaded, path);
        if seen.insert(path.clone()) {
          paths.push(path);
        }
      }
    }
    paths
  };

  let configured_paths = config_workspace_paths();
  let (paths, uses_configured_paths) = if !cli_paths.is_empty() && !sole_discovery_selector {
    (cli_paths, false)
  } else if sole_discovery_selector {
    (configured_paths, true)
  } else if config.is_some() {
    (configured_paths, true)
  } else {
    (Vec::new(), false)
  };

  Ok(EffectiveServeOptions {
    address,
    port,
    paths,
    stylesheet_paths,
    shim_path,
    wasm_pkg,
    project_root,
    uses_configured_paths,
  })
}
