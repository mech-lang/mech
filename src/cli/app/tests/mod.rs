use super::*;

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

#[cfg(all(test, feature = "formatter"))]
mod format_collection_tests {
  use super::*;

  fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-format-collection-{label}-{}",
      std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
  }

  fn format_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap()
  }

  #[test]
  fn collect_format_targets_preserves_duplicate_basenames_under_directory() {
    let _guard = format_test_lock();
    let root = temp_root("duplicates");
    let docs = root.join("docs");
    std::fs::create_dir_all(docs.join("a")).unwrap();
    std::fs::create_dir_all(docs.join("b")).unwrap();
    std::fs::write(docs.join("a/index.mec"), "x := 1").unwrap();
    std::fs::write(docs.join("b/index.mec"), "x := 2").unwrap();

    let targets = collect_format_targets(&docs, None, false, false).unwrap();
    let relatives = targets.iter().map(|target| target.relative_path.clone()).collect::<Vec<_>>();
    assert_eq!(relatives, vec![PathBuf::from("a/index.mec"), PathBuf::from("b/index.mec")]);
    assert!(targets.iter().all(|target| target.input_root == docs));
    ensure_unique_format_outputs(&targets, &root.join("out"), false, false, true).unwrap();
    ensure_unique_format_outputs(&targets, &root.join("out"), false, false, false).unwrap();
    assert_eq!(root.join("out").join(&targets[0].relative_path).with_extension("html"), root.join("out/a/index.html"));
    assert_eq!(root.join("out").join(&targets[1].relative_path), root.join("out/b/index.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_directory_output_path_selection_preserves_default_and_output_roots() {
    let _guard = format_test_lock();
    let root = temp_root("default-output-paths");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("docs"), None, false, false).unwrap();
    let target = &targets[0];
    assert!(format_writes_in_place(None, Path::new("."), false).unwrap());
    assert!(format_writes_in_place(Some("."), Path::new("."), false).unwrap());
    assert!(format_writes_in_place(Some("./"), Path::new("./"), false).unwrap());
    assert!(format_writes_in_place(Some(root.to_str().unwrap()), &root, false).unwrap());
    assert!(!format_writes_in_place(Some("out"), Path::new("out"), false).unwrap());
    std::env::set_current_dir(old_cwd).unwrap();

    assert_eq!(format_output_file_for_target(target, Path::new("."), false, true, false), PathBuf::from("docs/main.mec"));
    assert_eq!(format_output_file_for_target(target, Path::new("."), false, true, true), PathBuf::from("docs/main.html"));
    assert_eq!(format_output_file_for_target(target, Path::new("."), false, true, false), PathBuf::from("docs/main.mec"));
    assert_eq!(format_output_file_for_target(target, Path::new("out"), false, false, false), PathBuf::from("out/main.mec"));
    assert_eq!(format_output_file_for_target(target, Path::new("out"), false, false, true), PathBuf::from("out/main.html"));
    assert_eq!(format_output_file_for_target(target, Path::new("single.mec"), true, false, false), PathBuf::from("single.mec"));
    assert_eq!(format_output_file_for_target(target, Path::new("page.htm"), true, false, true), PathBuf::from("page.htm"));
    assert_eq!(format_output_file_for_target(target, Path::new("page.html"), true, false, true), PathBuf::from("page.html"));
    assert_eq!(format_output_file_for_target(target, Path::new("page.custom"), true, false, true), PathBuf::from("page.custom"));
    std::fs::remove_dir_all(root).unwrap();
  }


  #[test]
  fn collect_format_targets_preserves_explicit_relative_file_path() {
    let _guard = format_test_lock();
    let root = temp_root("explicit-relative");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("src/main.mec"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert_eq!(targets[0].relative_path, PathBuf::from("src/main.mec"));
    assert_eq!(targets[0].default_output_path, PathBuf::from("src/main.mec"));
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("."), false, true, false), PathBuf::from("src/main.mec"));
    assert_eq!(root.join("out").join(&targets[0].relative_path), root.join("out/src/main.mec"));
    assert_eq!(root.join("out").join(&targets[0].relative_path).with_extension("html"), root.join("out/src/main.html"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn safe_output_relative_path_rejects_parent_components() {
    let _guard = format_test_lock();
    assert_eq!(safe_output_relative_path(Path::new("../docs/main.mec")).unwrap(), PathBuf::from("main.mec"));
  }

  #[test]
  fn format_explicit_parent_relative_input_under_out_uses_filename() {
    let _guard = format_test_lock();
    let root = temp_root("parent-relative-out");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir_all(root.join("examples")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.join("examples")).unwrap();
    let targets = collect_format_targets(Path::new("../docs/main.mec"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    let raw_output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);
    let html_output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, true);
    assert_eq!(raw_output, PathBuf::from("formatted/main.mec"));
    assert_eq!(html_output, PathBuf::from("formatted/main.html"));
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("."), false, true, false), PathBuf::from("../docs/main.mec"));
    assert!(!raw_output.components().any(|component| matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_safe_relative_input_under_out_preserves_subdir() {
    let _guard = format_test_lock();
    let root = temp_root("safe-relative-out");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("src/main.mec"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    let output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);
    assert_eq!(output, PathBuf::from("formatted/src/main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_output_collision_uses_actual_output_paths() {
    let _guard = format_test_lock();
    let root = temp_root("actual-collisions");
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::fs::create_dir_all(root.join("b")).unwrap();
    std::fs::write(root.join("a/main.mec"), "x := 1").unwrap();
    std::fs::write(root.join("b/main.mec"), "y := 2").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let mut targets = Vec::new();
    targets.extend(collect_format_targets(Path::new("a"), None, false, false).unwrap());
    targets.extend(collect_format_targets(Path::new("b"), None, false, false).unwrap());
    std::env::set_current_dir(old_cwd).unwrap();

    ensure_unique_format_outputs(&targets, Path::new("."), false, true, false).unwrap();
    let error = format!("{:?}", ensure_unique_format_outputs(&targets, Path::new("out"), false, false, false).unwrap_err());
    assert!(error.contains("Format output collision"), "got {error}");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn raw_format_preserves_include_directives_without_expanding() {
    let _guard = format_test_lock();
    let root = temp_root("include");
    std::fs::write(root.join("included.mec"), "secret := 42\n").unwrap();
    std::fs::write(root.join("main.mec"), "+> ./included.mec\nvisible := 1\n").unwrap();
    let source = read_format_source(&root.join("main.mec")).unwrap();
    let formatted = match source {
      MechSourceCode::String(text) => {
        let tree = parser::parse(text.trim()).unwrap();
        let mut formatter = Formatter::new();
        formatter.format(&tree)
      }
      other => panic!("expected string source, got {other:?}"),
    };
    assert!(formatted.contains("+> ./included.mec"), "formatted output was {formatted}");
    assert!(formatted.contains("visible"), "formatted output was {formatted}");
    assert!(!formatted.contains("secret := 42"), "formatted output was {formatted}");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn collectors_skip_symlinked_directory_loops() {
    let _guard = format_test_lock();
    use std::os::unix::fs::symlink;
    let root = temp_root("symlink-loop");
    std::fs::write(root.join("main.mec"), "x := 1").unwrap();
    symlink(&root, root.join("self")).unwrap();

    let format_targets = collect_format_targets(&root, None, false, false).unwrap();
    assert_eq!(format_targets.len(), 1);
    assert_eq!(format_targets[0].relative_path, PathBuf::from("main.mec"));

    let run_targets = collect_run_targets(&root).unwrap();
    assert_eq!(run_targets, vec![root.join("main.mec")]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn collect_format_targets_includes_symlinked_files_but_not_dirs_or_broken_links() {
    let _guard = format_test_lock();
    use std::os::unix::fs::symlink;
    let root = temp_root("format-symlink-file");
    std::fs::write(root.join("main.mec"), "x := 1").unwrap();
    symlink(root.join("main.mec"), root.join("linked.mec")).unwrap();
    symlink(&root, root.join("self")).unwrap();
    symlink(root.join("missing.mec"), root.join("broken.mec")).unwrap();

    let targets = collect_format_targets(&root, None, false, false).unwrap();
    let relatives = targets.iter().map(|target| target.relative_path.clone()).collect::<Vec<_>>();
    assert_eq!(relatives, vec![PathBuf::from("linked.mec"), PathBuf::from("main.mec")]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_default_output_has_no_exclusion() {
    let _guard = format_test_lock();
    let exclusion = format_output_exclusion(None, Path::new("."), false).unwrap();
    assert!(exclusion.is_none());
  }

  #[test]
  fn format_explicit_dot_output_has_no_exclusion() {
    let _guard = format_test_lock();
    let exclusion = format_output_exclusion(Some("."), Path::new("."), false).unwrap();
    assert!(exclusion.is_none());
  }

  #[test]
  fn format_docs_default_and_dot_outputs_collect_docs_sources() {
    let _guard = format_test_lock();
    let root = temp_root("default-output");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("main.mec"), "x := 1").unwrap();
    std::fs::write(docs.join("other.mec"), "y := 2").unwrap();

    let default_targets = collect_format_targets(&docs, format_output_exclusion(None, Path::new("."), false).unwrap().as_deref(), false, false).unwrap();
    assert_eq!(default_targets.len(), 2);

    let dot_targets = collect_format_targets(&docs, format_output_exclusion(Some("."), Path::new("."), false).unwrap().as_deref(), false, false).unwrap();
    assert_eq!(dot_targets.len(), 2);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_html_directory_in_place_skips_generated_html_siblings() {
    let _guard = format_test_lock();
    let root = temp_root("html-in-place-skip");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("foo.mec"), "x := 1").unwrap();
    std::fs::write(docs.join("foo.html"), "<html></html>").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("docs"), None, true, true).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, PathBuf::from("docs/foo.mec"));
    ensure_unique_format_outputs(&targets, Path::new("."), false, true, true).unwrap();
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_html_explicit_html_file_still_collected() {
    let _guard = format_test_lock();
    let root = temp_root("explicit-html");
    let html = root.join("foo.html");
    std::fs::write(&html, "<html></html>").unwrap();
    let targets = collect_format_targets(&html, None, true, true).unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, html);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_html_directory_to_separate_out_still_collects_html() {
    let _guard = format_test_lock();
    let root = temp_root("html-separate-out");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("foo.mec"), "x := 1").unwrap();
    std::fs::write(docs.join("foo.html"), "<html></html>").unwrap();
    let targets = collect_format_targets(&docs, None, true, false).unwrap();
    let names = targets.iter().map(|target| target.relative_path.clone()).collect::<Vec<_>>();
    assert!(names.contains(&PathBuf::from("foo.mec")));
    assert!(names.contains(&PathBuf::from("foo.html")));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_directory_dash_o_same_directory_collects_sources_in_place() {
    let _guard = format_test_lock();
    let root = temp_root("same-output-dir");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let mech_paths = vec!["docs".to_string()];
    let output_matches_input_dir = format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    let writes_in_place = format_writes_in_place(Some("docs"), Path::new("docs"), false).unwrap() || output_matches_input_dir;
    let targets = collect_format_targets(Path::new("docs"), None, false, writes_in_place).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert!(writes_in_place);
    assert_eq!(targets.len(), 1);
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("docs"), false, writes_in_place, false), PathBuf::from("docs/main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_directory_dash_o_same_directory_html_skips_generated_siblings() {
    let _guard = format_test_lock();
    let root = temp_root("same-output-html");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    std::fs::write(root.join("docs/main.html"), "<html></html>").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let mech_paths = vec!["docs".to_string()];
    let output_matches_input_dir = format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    let writes_in_place = format_writes_in_place(Some("docs"), Path::new("docs"), false).unwrap() || output_matches_input_dir;
    let targets = collect_format_targets(Path::new("docs"), None, true, writes_in_place).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert!(writes_in_place);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, PathBuf::from("docs/main.mec"));
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("docs"), false, writes_in_place, true), PathBuf::from("docs/main.html"));
    ensure_unique_format_outputs(&targets, Path::new("docs"), false, writes_in_place, true).unwrap();
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_multiple_inputs_dash_o_matching_one_input_errors() {
    let _guard = format_test_lock();
    let root = temp_root("multi-same-output");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir_all(root.join("more")).unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let mech_paths = vec!["docs".to_string(), "more".to_string()];
    let output_matches_input_dir = format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    assert!(output_matches_input_dir);
    let error = format!("{:?}", reject_ambiguous_matching_output_dir(output_matches_input_dir, mech_paths.len(), Path::new("docs")).unwrap_err());
    assert!(error.contains("matches one of multiple format inputs"), "got {error}");
    std::env::set_current_dir(old_cwd).unwrap();
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_directory_dash_o_distinct_directory_still_writes_under_out() {
    let _guard = format_test_lock();
    let root = temp_root("distinct-output-dir");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/main.mec"), "x := 1").unwrap();
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let targets = collect_format_targets(Path::new("docs"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert_eq!(format_output_file_for_target(&targets[0], Path::new("out"), false, false, false), PathBuf::from("out/main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn reject_multi_target_format_to_single_file() {
    let _guard = format_test_lock();
    let root = temp_root("single-file-output");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("a.mec"), "a := 1").unwrap();
    std::fs::write(docs.join("b.mec"), "b := 2").unwrap();
    let targets = collect_format_targets(&docs, None, false, false).unwrap();
    let error = format!("{:?}", reject_multi_target_file_output(targets.len(), &root.join("out.mec"), true).unwrap_err());
    assert!(error.contains("Cannot write 2 formatted sources into single output file"), "got {error}");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn reject_multiple_explicit_inputs_to_single_file() {
    let _guard = format_test_lock();
    let root = temp_root("explicit-single-file-output");
    std::fs::write(root.join("a.mec"), "a := 1").unwrap();
    std::fs::write(root.join("b.mec"), "b := 2").unwrap();
    let mut targets = Vec::new();
    targets.extend(collect_format_targets(&root.join("a.mec"), None, false, false).unwrap());
    targets.extend(collect_format_targets(&root.join("b.mec"), None, false, false).unwrap());
    let error = format!("{:?}", reject_multi_target_file_output(targets.len(), &root.join("out.mec"), true).unwrap_err());
    assert!(error.contains("Cannot write 2 formatted sources into single output file"), "got {error}");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_format_targets_skips_selected_output_directory() {
    let _guard = format_test_lock();
    let root = temp_root("skip-selected-output");
    let docs = root.join("docs");
    let site = docs.join("site");
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(docs.join("main.mec"), "x := 1").unwrap();
    std::fs::write(site.join("main.html"), "<html></html>").unwrap();
    let exclusion = normalize_output_exclusion(&site, false).unwrap();
    let targets = collect_format_targets(&docs, exclusion.as_deref(), false, false).unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].relative_path, PathBuf::from("main.mec"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn collect_format_targets_skips_markdown_in_directories_but_rejects_explicit_markdown() {
    let _guard = format_test_lock();
    let root = temp_root("markdown");
    std::fs::write(root.join("README.md"), "# Raw Markdown").unwrap();
    std::fs::write(root.join("demo.mec"), "x := 1").unwrap();

    let targets = collect_format_targets(&root, None, false, false).unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].relative_path, PathBuf::from("demo.mec"));

    let error = format!("{:?}", collect_format_targets(&root.join("README.md"), None, false, false).unwrap_err());
    assert!(error.contains("Unsupported source extension"), "got {error}");
    std::fs::remove_dir_all(root).unwrap();
  }
  #[test]
  fn format_explicit_absolute_file_default_updates_absolute_path() {
    let root = temp_root("absolute-default");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, false, true).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("."), false, true, false);

    assert_eq!(output, file);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_absolute_file_out_dot_updates_absolute_path() {
    let root = temp_root("absolute-dot");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, false, true).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("."), false, true, false);

    assert_eq!(output, file);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_absolute_file_out_directory_stays_inside_out() {
    let root = temp_root("absolute-out");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, false, false).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);

    assert_eq!(output, PathBuf::from("formatted").join("main.mec"));
    assert!(!output.components().any(|component| matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_absolute_file_html_default_writes_absolute_html_sibling() {
    let root = temp_root("absolute-html-default");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, true, true).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("."), false, true, true);

    assert_eq!(output, root.join("main.html"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn format_explicit_absolute_file_html_out_directory_stays_inside_out() {
    let root = temp_root("absolute-html-out");
    let file = root.join("main.mec");
    std::fs::write(&file, "x := 1").unwrap();
    let targets = collect_format_targets(&file, None, true, false).unwrap();

    let output = format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, true);

    assert_eq!(output, PathBuf::from("formatted").join("main.html"));
    assert!(!output.components().any(|component| matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))));
    std::fs::remove_dir_all(root).unwrap();
  }

}

#[cfg(all(test, feature = "run"))]
mod run_collection_tests {
  use super::*;

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
