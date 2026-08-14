use super::*;
#[cfg(feature = "build")]
use crate::cli::commands::build::{
    BuildEmit, BuildOptions, BuildProfile, run as run_build, validate_build_bytecode_inputs,
};
use colored::{ColoredString, Colorize};
use std::path::PathBuf;

#[cfg(all(test, feature = "serve"))]
mod filesystem_capability_cli_tests {
    use super::*;
    use mech_runtime::{
        DefaultIdGenerator, FS_IMPORT, FS_LIST, FS_READ, FS_RESOLVE, FS_SERVE, FS_WATCH,
        SERVE_HOST_SUBJECT,
    };

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
        let authority = capabilities::build_mech_filesystem_authority(
            &capabilities::FilesystemCapabilityArgs::from_matches(&matches),
            None,
        )
        .unwrap()
        .authority;
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
        let authority = capabilities::build_mech_filesystem_authority(
            &capabilities::FilesystemCapabilityArgs::from_matches(&matches),
            None,
        )
        .unwrap()
        .authority;
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
        let authority = capabilities::build_mech_filesystem_authority(
            &capabilities::FilesystemCapabilityArgs::from_matches(&matches),
            None,
        )
        .unwrap()
        .authority;
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
        let authority = capabilities::build_mech_filesystem_authority(
            &capabilities::FilesystemCapabilityArgs::from_matches(&matches),
            None,
        )
        .unwrap()
        .authority;
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
        let authority = capabilities::build_mech_filesystem_authority(
            &capabilities::FilesystemCapabilityArgs::from_matches(&matches),
            None,
        )
        .unwrap()
        .authority;
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
        let authority = capabilities::build_mech_filesystem_authority(
            &capabilities::FilesystemCapabilityArgs::from_matches(&matches),
            None,
        )
        .unwrap()
        .authority;
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

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-build-module-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn build_options(paths: Vec<PathBuf>, output_path: PathBuf) -> BuildOptions {
        BuildOptions {
            paths: paths
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            emit: BuildEmit::Bytecode,
            name: Some("output".to_string()),
            output_path: Some(output_path.join("output.mecb")),
            target: None,
            profile: BuildProfile::Release,
            config_path: None,
            no_config: true,
            workspace_root: None,
            keep_project: false,
            offline: true,
            debug: false,
            trace: false,
            time: false,
            rounds_per_step: 10_000,
        }
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
    fn build_rejects_empty_inputs() {
        let error = validate_build_bytecode_inputs(&[])
            .unwrap_err()
            .full_chain_message();
        assert!(error.contains("no build inputs supplied"));
    }

    #[test]
    fn build_single_bytecode_input_is_allowed_for_clean_copy() {
        assert_eq!(
            validate_build_bytecode_inputs(&paths(&["compiled.mecb"])).unwrap(),
            1
        );
    }

    #[test]
    fn build_multiple_source_inputs_still_work() {
        assert_eq!(
            validate_build_bytecode_inputs(&paths(&["a.mec", "b.mec"])).unwrap(),
            0
        );
    }

    #[test]
    fn build_resolves_sibling_dependency_before_compiling() {
        let root = temp_root("sibling");
        let main = root.join("main.mec");
        let output = root.join("out");
        std::fs::write(&main, "+> ./dep.mec\nanswer := dep/value + 1\n").unwrap();
        std::fs::write(root.join("dep.mec"), "value := 41\n<+ value\n").unwrap();

        run_build(build_options(vec![main], output.clone())).unwrap();

        assert!(output.join("output.mecb").metadata().unwrap().len() > 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn build_resolves_nested_dependency_before_compiling() {
        let root = temp_root("nested");
        let main = root.join("main.mec");
        let lib = root.join("lib");
        let output = root.join("out");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(&main, "+> ./lib/first.mec\nanswer := first/value + 1\n").unwrap();
        std::fs::write(
            lib.join("first.mec"),
            "+> ./second.mec\nvalue := second/value + 1\n<+ value\n",
        )
        .unwrap();
        std::fs::write(lib.join("second.mec"), "value := 40\n<+ value\n").unwrap();

        run_build(build_options(vec![main], output.clone())).unwrap();

        assert!(output.join("output.mecb").metadata().unwrap().len() > 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(feature = "full_compiler")]
    #[test]
    fn build_materializes_linked_function_module_imports_before_compiling() {
        let root = temp_root("linked-function-module");
        let main = root.join("main.mec");
        let output = root.join("out");
        std::fs::write(&main, "+> math\nresult := math/sin(0)\n").unwrap();

        run_build(build_options(vec![main], output.clone())).unwrap();

        assert!(output.join("output.mecb").metadata().unwrap().len() > 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn build_missing_dependency_preserves_error_and_creates_no_output() {
        let root = temp_root("missing");
        let main = root.join("main.mec");
        let output = root.join("out");
        std::fs::write(&main, "+> ./missing.mec\nanswer := 1\n").unwrap();

        let error = match run_build(build_options(vec![main], output.clone())) {
            Ok(_) => panic!("build unexpectedly succeeded"),
            Err(error) => error,
        };

        let chain = error.full_chain_message();
        assert!(chain.contains("missing.mec"));
        assert!(chain.contains("main.mec"));
        assert!(!output.join("output.mecb").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn build_multiple_source_roots_in_caller_order() {
        let root = temp_root("multiple-roots");
        let first = root.join("first.mec");
        let second = root.join("second.mec");
        let output = root.join("out");
        std::fs::write(&first, "marker := 1\n").unwrap();
        std::fs::write(&second, "answer := marker + 1\n").unwrap();

        run_build(build_options(vec![first, second], output.clone())).unwrap();

        assert!(output.join("output.mecb").metadata().unwrap().len() > 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, feature = "build"))]
mod build_argument_tests {
    use super::*;

    fn options(arguments: &[&str]) -> MResult<BuildOptions> {
        let matches = super::super::build_cli()
            .try_get_matches_from(arguments)
            .unwrap();
        let build_matches = matches.subcommand_matches("build").unwrap();
        BuildOptions::from_matches(super::super::root_flags(&matches), &matches, build_matches)
    }

    #[test]
    fn build_defaults_to_native_release() {
        let options = options(&["mech", "build"]).unwrap();
        assert_eq!(options.emit, BuildEmit::Native);
        assert_eq!(options.profile, BuildProfile::Release);
        assert!(options.target.is_none());
        assert!(options.workspace_root.is_none());
    }

    #[test]
    fn build_accepts_every_emit_mode() {
        for (value, expected) in [
            ("native", BuildEmit::Native),
            ("bytecode", BuildEmit::Bytecode),
            ("cargo-project", BuildEmit::CargoProject),
            ("plan", BuildEmit::Plan),
        ] {
            assert_eq!(
                options(&["mech", "build", "demo.mec", "--emit", value])
                    .unwrap()
                    .emit,
                expected,
                "{value}"
            );
        }
    }

    #[test]
    fn build_rejects_invalid_emit_and_profile() {
        for arguments in [
            ["mech", "build", "demo.mec", "--emit", "zip"].as_slice(),
            ["mech", "build", "demo.mec", "--profile", "fast"].as_slice(),
        ] {
            assert!(
                super::super::build_cli()
                    .try_get_matches_from(arguments)
                    .is_err()
            );
        }
    }

    #[test]
    fn build_rejects_invalid_name_target_and_conflicting_project_flags() {
        assert!(options(&["mech", "build", "demo.mec", "--name", "not valid"]).is_err());
        assert!(options(&["mech", "build", "demo.mec", "--target", "bad target"]).is_err());
        assert!(
            options(&[
                "mech",
                "build",
                "demo.mec",
                "--emit",
                "cargo-project",
                "--keep-project",
            ])
            .is_err()
        );
    }

    #[test]
    fn bytecode_only_build_skips_unused_native_identity_validation() {
        let bytecode = options(&[
            "mech",
            "build",
            "2026-demo.mec",
            "--emit",
            "bytecode",
            "--name",
            "2026-demo",
            "--target",
            "unused target",
        ])
        .unwrap();
        assert_eq!(bytecode.emit, BuildEmit::Bytecode);
        assert!(!bytecode.keep_project);

        assert!(
            options(&[
                "mech",
                "build",
                "2026-demo.mec",
                "--emit",
                "bytecode",
                "--name",
                "2026-demo",
                "--keep-project",
            ])
            .is_err()
        );
    }

    #[test]
    fn build_records_workspace_root_and_offline_without_changing_emit_defaults() {
        let options = options(&[
            "mech",
            "build",
            "demo.mec",
            "--workspace-root",
            "workspace",
            "--offline",
        ])
        .unwrap();
        assert_eq!(options.workspace_root, Some(PathBuf::from("workspace")));
        assert!(options.offline);
        assert_eq!(options.emit, BuildEmit::Native);
    }

    #[test]
    fn build_records_exact_output_and_keep_project() {
        let options = options(&[
            "mech",
            "build",
            "demo.mec",
            "--emit",
            "plan",
            "--out",
            "dist/exact.json",
            "--keep-project",
        ])
        .unwrap();
        assert_eq!(options.output_path, Some(PathBuf::from("dist/exact.json")));
        assert!(options.keep_project);
    }

    #[test]
    fn build_config_flags_remain_mutually_exclusive() {
        assert!(
            super::super::build_cli()
                .try_get_matches_from([
                    "mech",
                    "--config",
                    "mech.mcfg",
                    "--no-config",
                    "build",
                    "demo.mec",
                ])
                .is_err()
        );
    }
}

#[cfg(all(test, feature = "run"))]
mod run_collection_tests {
    use super::*;
    use crate::cli::commands::run::collect_run_targets;

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-run-collection-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
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

        assert_eq!(
            collect_run_targets(&root).unwrap(),
            vec![
                root.join("doc.mdoc"),
                root.join("main.mec"),
                root.join("project.mpkg"),
            ]
        );
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
        assert_eq!(
            targets,
            vec![root.join("linked.mec"), root.join("main.mec")]
        );
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
    #[cfg(feature = "bundle_web")]
    super::build_cli()
        .try_get_matches_from(["mech", "bundle-web", "--help"])
        .unwrap_err();
}

#[test]
fn root_help_does_not_advertise_parse_tree() {
    let mut command = super::build_cli();
    let mut help = Vec::new();
    command.write_long_help(&mut help).unwrap();
    let help = String::from_utf8(help).unwrap();

    assert!(!help.contains("--tree"));
    assert!(!help.contains("Print parse tree"));
}

#[test]
fn root_rejects_removed_parse_tree_options() {
    for option in ["--tree", "-e"] {
        let error = super::build_cli()
            .try_get_matches_from(["mech", option, "demo.mec"])
            .unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }
}

#[test]
fn root_rounds_per_step_rejects_invalid_values() {
    for value in ["typo", "0", "-1", "18446744073709551616"] {
        assert!(
            super::build_cli()
                .try_get_matches_from(["mech", "--rounds-per-step", value])
                .is_err()
        );
    }
}

#[test]
fn root_rounds_per_step_is_stored_as_usize() {
    let matches = super::build_cli()
        .try_get_matches_from(["mech", "--rounds-per-step", "10"])
        .unwrap();

    assert_eq!(super::root_flags(&matches).rounds_per_step, Some(10));
}

#[cfg(feature = "run")]
#[test]
fn run_rounds_per_step_rejects_invalid_values() {
    for value in ["typo", "0", "-1", "18446744073709551616"] {
        assert!(
            super::build_cli()
                .try_get_matches_from(["mech", "run", "--rounds-per-step", value, "demo.mec"])
                .is_err()
        );
    }
}

#[cfg(feature = "run")]
#[test]
fn run_rounds_per_step_is_stored_as_usize() {
    let matches = super::build_cli()
        .try_get_matches_from(["mech", "run", "--rounds-per-step", "20", "demo.mec"])
        .unwrap();
    let args = crate::cli::run_options::RunCliArgs::from_matches(
        super::root_flags(&matches),
        &matches,
        matches.subcommand_matches("run"),
    )
    .unwrap();

    assert_eq!(args.rounds_per_step, Some(20));
}

#[cfg(feature = "run")]
#[test]
fn run_rounds_per_step_overrides_root_value() {
    let matches = super::build_cli()
        .try_get_matches_from([
            "mech",
            "--rounds-per-step",
            "10",
            "run",
            "--rounds-per-step",
            "20",
            "demo.mec",
        ])
        .unwrap();
    let args = crate::cli::run_options::RunCliArgs::from_matches(
        super::root_flags(&matches),
        &matches,
        matches.subcommand_matches("run"),
    )
    .unwrap();

    assert_eq!(args.rounds_per_step, Some(20));
}

#[cfg(all(
    test,
    feature = "build",
    feature = "formatter",
    feature = "bundle_web",
    feature = "run",
    feature = "serve"
))]
mod filesystem_flag_dispatch_tests {
    const MESSAGE: &str = "filesystem capability flags are only supported by `mech run`, bare run inputs, and `mech serve`";

    fn dispatch_error(args: &[&str]) -> String {
        let matches = super::build_cli().try_get_matches_from(args).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let error = match runtime.block_on(super::dispatch(matches)) {
            Ok(_) => panic!("dispatch unexpectedly succeeded"),
            Err(error) => error,
        };
        error.full_chain_message()
    }

    #[test]
    fn filesystem_flags_rejected_for_build() {
        assert!(dispatch_error(&["mech", "build", "--allow-read", "."]).contains(MESSAGE));
    }

    #[test]
    fn filesystem_flags_rejected_for_format() {
        assert!(dispatch_error(&["mech", "format", "--allow-read", "."]).contains(MESSAGE));
    }

    #[test]
    fn filesystem_flags_rejected_for_bundle_web() {
        assert!(
            dispatch_error(&[
                "mech",
                "bundle-web",
                "--allow-read",
                ".",
                ".",
                "--out",
                "out"
            ])
            .contains(MESSAGE)
        );
    }

    #[test]
    fn filesystem_flags_accepted_for_run() {
        super::build_cli()
            .try_get_matches_from(["mech", "run", "--allow-read", "."])
            .unwrap();
    }

    #[test]
    fn filesystem_flags_accepted_for_bare_run() {
        super::build_cli()
            .try_get_matches_from(["mech", "--allow-read", "."])
            .unwrap();
    }

    #[test]
    fn filesystem_flags_accepted_for_serve() {
        super::build_cli()
            .try_get_matches_from(["mech", "serve", "--allow-read", "."])
            .unwrap();
    }
}
