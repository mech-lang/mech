use super::*;
use colored::{ColoredString, Colorize};
use std::path::PathBuf;
#[cfg(feature = "build")]
use crate::cli::commands::build::validate_build_bytecode_inputs;

#[cfg(all(test, feature = "serve"))]
mod filesystem_capability_cli_tests {
    use super::*;
    use mech_runtime::{DefaultIdGenerator, SERVE_HOST_SUBJECT, FS_IMPORT, FS_LIST, FS_READ, FS_RESOLVE, FS_SERVE, FS_WATCH};

    fn capability_matches(arguments: &[&str]) -> clap::ArgMatches {
        capabilities::add_filesystem_capability_args(Command::new("mech").subcommand(
            Command::new("serve").arg(Arg::new("mech_serve_file_paths").action(ArgAction::Append)),
        ))
        .try_get_matches_from(arguments)
        .unwrap()
        .subcommand_matches("serve")
        .unwrap()
        .clone()
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-cli-capability-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn test_badge() -> ColoredString {
        "[Mech Server]".normal()
    }

    #[test]
    fn default_grants_current_directory_when_no_capability_options_are_present() {
        let matches = capability_matches(&["mech", "serve", "."]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(
                &mut ids,
                SERVE_HOST_SUBJECT,
                &std::env::current_dir().unwrap(),
                true,
                [FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
            )
            .unwrap();
    }

    #[test]
    fn cap_root_disables_default_current_directory_authority() {
        let root = temp_root("cap-root");
        let allowed = root.join("allowed");
        let outside = root.join("outside");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let allowed_arg = allowed.to_string_lossy();
        let outside_arg = outside.to_string_lossy();
        let matches =
            capability_matches(&["mech", "--cap-root", &allowed_arg, "serve", &outside_arg]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        assert!(
            authority
                .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &outside, true, [FS_READ])
                .is_err()
        );
        authority
            .delegate_path_to(
                &mut ids,
                SERVE_HOST_SUBJECT,
                &allowed,
                true,
                [FS_READ, FS_SERVE],
            )
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_default_capabilities_grants_nothing() {
        let root = temp_root("none");
        let matches = capability_matches(&[
            "mech",
            "--no-default-capabilities",
            "serve",
            root.to_str().unwrap(),
        ]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        assert!(
            authority
                .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_READ])
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn allow_read_does_not_grant_serve() {
        let root = temp_root("read-only");
        let matches = capability_matches(&[
            "mech",
            "serve",
            root.to_str().unwrap(),
            "--allow-read",
            root.to_str().unwrap(),
        ]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_READ])
            .unwrap();
        assert!(
            authority
                .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_SERVE])
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_granular_grants_combine_for_normal_serve_directory() {
        let root = temp_root("granular-combine");
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let allowed_arg = allowed.to_string_lossy();
        let matches = capability_matches(&[
            "mech",
            "--allow-read",
            &allowed_arg,
            "--allow-watch",
            &allowed_arg,
            "--allow-serve",
            &allowed_arg,
            "serve",
            &allowed_arg,
        ]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        assert_eq!(authority.source_capabilities().len(), 1);
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(
                &mut ids,
                SERVE_HOST_SUBJECT,
                &allowed,
                true,
                [FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
            )
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn allow_serve_grants_serve() {
        let root = temp_root("serve-only");
        let matches = capability_matches(&[
            "mech",
            "serve",
            root.to_str().unwrap(),
            "--allow-serve",
            root.to_str().unwrap(),
        ]);
        let authority = capabilities::build_mech_filesystem_authority(&matches, None, &test_badge()).unwrap();
        let mut ids = DefaultIdGenerator::new();
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_SERVE])
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, feature = "build"))]
mod build_input_tests {
  use super::*;

  fn paths(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
  }

  #[test]
  fn build_rejects_mixed_source_then_bytecode() {
    let error = validate_build_bytecode_inputs(&paths(&["old.mec", "compiled.mecb"]))
      .unwrap_err()
      .full_chain_message();
    assert!(error.contains("Cannot mix bytecode"));
  }

  #[test]
  fn build_rejects_bytecode_then_source() {
    let error = validate_build_bytecode_inputs(&paths(&["compiled.mecb", "next.mec"]))
      .unwrap_err()
      .full_chain_message();
    assert!(error.contains("Cannot mix bytecode"));
  }

  #[test]
  fn build_rejects_multiple_bytecode_inputs() {
    let error = validate_build_bytecode_inputs(&paths(&["a.mecb", "b.mecb"]))
      .unwrap_err()
      .full_chain_message();
    assert!(error.contains("Cannot combine multiple bytecode"));
  }

  #[test]
  fn build_single_bytecode_input_is_allowed_for_clean_copy() {
    assert_eq!(validate_build_bytecode_inputs(&paths(&["compiled.mecb"])).unwrap(), 1);
  }

  #[test]
  fn build_multiple_source_inputs_still_work() {
    assert_eq!(validate_build_bytecode_inputs(&paths(&["a.mec", "b.mec"])).unwrap(), 0);
  }
}

#[cfg(all(test, feature = "run"))]
mod run_collection_tests {
  use super::*;
  use crate::cli::commands::run::collect_run_targets;

  fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-run-collection-{label}-{}",
      std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
  }

  #[test]
  fn collect_run_targets_accepts_explicit_mdoc() {
    let root = temp_root("explicit-mdoc");
    let doc = root.join("doc.mdoc");
    std::fs::write(&doc, "x := 1").unwrap();
    assert_eq!(collect_run_targets(&doc).unwrap(), vec![doc]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_discovers_mdoc_in_directory() {
    let root = temp_root("directory-mdoc");
    std::fs::write(root.join("doc.mdoc"), "x := 1").unwrap();
    std::fs::write(root.join("main.mec"), "y := 2").unwrap();
    let targets = collect_run_targets(&root).unwrap();
    assert!(targets.contains(&root.join("doc.mdoc")));
    assert!(targets.contains(&root.join("main.mec")));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_accepts_explicit_mpkg() {
    let root = temp_root("explicit-mpkg");
    let package = root.join("project.mpkg");
    std::fs::write(&package, "{}").unwrap();
    assert_eq!(collect_run_targets(&package).unwrap(), vec![package]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_discovers_mpkg_in_directory() {
    let root = temp_root("directory-mpkg");
    std::fs::write(root.join("project.mpkg"), "{}").unwrap();
    std::fs::write(root.join("main.mec"), "y := 2").unwrap();
    let targets = collect_run_targets(&root).unwrap();
    assert!(targets.contains(&root.join("project.mpkg")));
    assert!(targets.contains(&root.join("main.mec")));
    std::fs::remove_dir_all(root).unwrap();
  }


  #[test]
  fn collect_run_targets_accepts_explicit_m_source() {
    let root = temp_root("explicit-m");
    let source = root.join("script.m");
    std::fs::write(&source, "x := 1").unwrap();
    assert_eq!(collect_run_targets(&source).unwrap(), vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_accepts_explicit_csv_source() {
    let root = temp_root("explicit-csv");
    let source = root.join("data.csv");
    std::fs::write(&source, "x,y\n1,2\n").unwrap();
    assert_eq!(collect_run_targets(&source).unwrap(), vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_accepts_explicit_js_source() {
    let root = temp_root("explicit-js");
    let source = root.join("script.js");
    std::fs::write(&source, "console.log('mech');").unwrap();
    assert_eq!(collect_run_targets(&source).unwrap(), vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_ignores_loader_supported_text_sources_in_directory() {
    let root = temp_root("directory-loader-text");
    let m = root.join("script.m");
    let csv = root.join("data.csv");
    let js = root.join("script.js");
    std::fs::write(&m, "x := 1").unwrap();
    std::fs::write(&csv, "x,y\n1,2\n").unwrap();
    std::fs::write(&js, "console.log('mech');").unwrap();

    assert!(collect_run_targets(&root).unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_skips_mecb_in_directory() {
    let root = temp_root("directory-skip-mecb");
    let source = root.join("main.mec");
    let bytecode = root.join("output.mecb");
    std::fs::write(&source, "x := 1").unwrap();
    std::fs::write(&bytecode, b"bytecode").unwrap();

    let targets = collect_run_targets(&root).unwrap();

    assert_eq!(targets, vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_allows_explicit_mecb_file() {
    let root = temp_root("explicit-mecb");
    let bytecode = root.join("output.mecb");
    std::fs::write(&bytecode, b"bytecode").unwrap();

    assert_eq!(collect_run_targets(&bytecode).unwrap(), vec![bytecode]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_directory_only_includes_mech_source_document_package_extensions() {
    let root = temp_root("directory-run-supported");
    let files = vec![
      root.join("data.csv"),
      root.join("doc.mdoc"),
      root.join("main.mec"),
      root.join("project.mpkg"),
      root.join("script.js"),
      root.join("script.m"),
    ];
    for path in &files {
      std::fs::write(path, "x := 1").unwrap();
    }
    std::fs::write(root.join("output.mecb"), b"bytecode").unwrap();

    assert_eq!(collect_run_targets(&root).unwrap(), vec![
      root.join("doc.mdoc"),
      root.join("main.mec"),
      root.join("project.mpkg"),
    ]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_run_targets_still_rejects_unsupported_extension() {
    let root = temp_root("unsupported");
    let source = root.join("notes.txt");
    std::fs::write(&source, "not a mech runtime source").unwrap();
    assert!(collect_run_targets(&source).is_err());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn collect_run_targets_includes_symlinked_files_but_not_dirs_or_broken_links() {
    use std::os::unix::fs::symlink;
    let root = temp_root("symlink-file");
    std::fs::write(root.join("main.mec"), "x := 1").unwrap();
    symlink(root.join("main.mec"), root.join("linked.mec")).unwrap();
    symlink(&root, root.join("self")).unwrap();
    symlink(root.join("missing.mec"), root.join("broken.mec")).unwrap();

    let targets = collect_run_targets(&root).unwrap();
    assert_eq!(targets, vec![root.join("linked.mec"), root.join("main.mec")]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn collect_run_targets_skips_symlinked_mecb_in_directory() {
    use std::os::unix::fs::symlink;
    let root = temp_root("symlink-mecb");
    let source = root.join("main.mec");
    let bytecode = root.join("output.mecb");
    std::fs::write(&source, "x := 1").unwrap();
    std::fs::write(&bytecode, b"bytecode").unwrap();
    symlink(&bytecode, root.join("linked.mecb")).unwrap();

    let targets = collect_run_targets(&root).unwrap();

    assert_eq!(targets, vec![source]);
    std::fs::remove_dir_all(root).unwrap();
  }
}

#[test]
fn root_command_parses_available_subcommands() {
  #[cfg(feature = "build")]
  super::build_cli()
    .try_get_matches_from(["mech", "build", "demo.mec", "--out", "target/out"])
    .unwrap();
  #[cfg(feature = "formatter")]
  super::build_cli()
    .try_get_matches_from(["mech", "format", "demo.mec", "--html"])
    .unwrap();
  #[cfg(feature = "run")]
  super::build_cli()
    .try_get_matches_from(["mech", "run", "demo.mec"])
    .unwrap();
  #[cfg(feature = "serve")]
  super::build_cli()
    .try_get_matches_from(["mech", "serve", "demo.mec", "--port", "8082"])
    .unwrap();
  #[cfg(feature = "test")]
  super::build_cli()
    .try_get_matches_from(["mech", "test", "demo.mec", "--verbose"])
    .unwrap();
  #[cfg(feature = "bundle_web")]
  super::build_cli()
    .try_get_matches_from(["mech", "bundle-web", "--help"])
    .unwrap_err();
}
