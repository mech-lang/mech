use super::*;

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mech-format-collection-{label}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
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
    let relatives = targets
        .iter()
        .map(|target| target.relative_path.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        relatives,
        vec![PathBuf::from("a/index.mec"), PathBuf::from("b/index.mec")]
    );
    assert!(targets.iter().all(|target| target.input_root == docs));
    ensure_unique_format_outputs(&targets, &root.join("out"), false, false, true).unwrap();
    ensure_unique_format_outputs(&targets, &root.join("out"), false, false, false).unwrap();
    assert_eq!(
        root.join("out")
            .join(&targets[0].relative_path)
            .with_extension("html"),
        root.join("out/a/index.html")
    );
    assert_eq!(
        root.join("out").join(&targets[1].relative_path),
        root.join("out/b/index.mec")
    );
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

    assert_eq!(
        format_output_file_for_target(target, Path::new("."), false, true, false),
        PathBuf::from("docs/main.mec")
    );
    assert_eq!(
        format_output_file_for_target(target, Path::new("."), false, true, true),
        PathBuf::from("docs/main.html")
    );
    assert_eq!(
        format_output_file_for_target(target, Path::new("."), false, true, false),
        PathBuf::from("docs/main.mec")
    );
    assert_eq!(
        format_output_file_for_target(target, Path::new("out"), false, false, false),
        PathBuf::from("out/main.mec")
    );
    assert_eq!(
        format_output_file_for_target(target, Path::new("out"), false, false, true),
        PathBuf::from("out/main.html")
    );
    assert_eq!(
        format_output_file_for_target(target, Path::new("single.mec"), true, false, false),
        PathBuf::from("single.mec")
    );
    assert_eq!(
        format_output_file_for_target(target, Path::new("page.htm"), true, false, true),
        PathBuf::from("page.htm")
    );
    assert_eq!(
        format_output_file_for_target(target, Path::new("page.html"), true, false, true),
        PathBuf::from("page.html")
    );
    assert_eq!(
        format_output_file_for_target(target, Path::new("page.custom"), true, false, true),
        PathBuf::from("page.custom")
    );
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
    assert_eq!(
        targets[0].default_output_path,
        PathBuf::from("src/main.mec")
    );
    assert_eq!(
        format_output_file_for_target(&targets[0], Path::new("."), false, true, false),
        PathBuf::from("src/main.mec")
    );
    assert_eq!(
        root.join("out").join(&targets[0].relative_path),
        root.join("out/src/main.mec")
    );
    assert_eq!(
        root.join("out")
            .join(&targets[0].relative_path)
            .with_extension("html"),
        root.join("out/src/main.html")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn safe_output_relative_path_rejects_parent_components() {
    let _guard = format_test_lock();
    assert_eq!(
        safe_output_relative_path(Path::new("../docs/main.mec")).unwrap(),
        PathBuf::from("main.mec")
    );
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
    let targets =
        collect_format_targets(Path::new("../docs/main.mec"), None, false, false).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    let raw_output =
        format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);
    let html_output =
        format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, true);
    assert_eq!(raw_output, PathBuf::from("formatted/main.mec"));
    assert_eq!(html_output, PathBuf::from("formatted/main.html"));
    assert_eq!(
        format_output_file_for_target(&targets[0], Path::new("."), false, true, false),
        PathBuf::from("../docs/main.mec")
    );
    assert!(!raw_output.components().any(|component| matches!(
        component,
        std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_)
    )));
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
    let output =
        format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);
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
    let error = format!(
        "{:?}",
        ensure_unique_format_outputs(&targets, Path::new("out"), false, false, false).unwrap_err()
    );
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
    assert!(
        formatted.contains("+> ./included.mec"),
        "formatted output was {formatted}"
    );
    assert!(
        formatted.contains("visible"),
        "formatted output was {formatted}"
    );
    assert!(
        !formatted.contains("secret := 42"),
        "formatted output was {formatted}"
    );
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

    #[cfg(feature = "run")]
    {
        let run_targets = crate::cli::commands::run::collect_run_targets(&root).unwrap();
        assert_eq!(run_targets, vec![root.join("main.mec")]);
    }
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
    let relatives = targets
        .iter()
        .map(|target| target.relative_path.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        relatives,
        vec![PathBuf::from("linked.mec"), PathBuf::from("main.mec")]
    );
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

    let default_targets = collect_format_targets(
        &docs,
        format_output_exclusion(None, Path::new("."), false)
            .unwrap()
            .as_deref(),
        false,
        false,
    )
    .unwrap();
    assert_eq!(default_targets.len(), 2);

    let dot_targets = collect_format_targets(
        &docs,
        format_output_exclusion(Some("."), Path::new("."), false)
            .unwrap()
            .as_deref(),
        false,
        false,
    )
    .unwrap();
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
    let names = targets
        .iter()
        .map(|target| target.relative_path.clone())
        .collect::<Vec<_>>();
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
    let output_matches_input_dir =
        format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    let writes_in_place = format_writes_in_place(Some("docs"), Path::new("docs"), false).unwrap()
        || output_matches_input_dir;
    let targets = collect_format_targets(Path::new("docs"), None, false, writes_in_place).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert!(writes_in_place);
    assert_eq!(targets.len(), 1);
    assert_eq!(
        format_output_file_for_target(
            &targets[0],
            Path::new("docs"),
            false,
            writes_in_place,
            false
        ),
        PathBuf::from("docs/main.mec")
    );
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
    let output_matches_input_dir =
        format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    let writes_in_place = format_writes_in_place(Some("docs"), Path::new("docs"), false).unwrap()
        || output_matches_input_dir;
    let targets = collect_format_targets(Path::new("docs"), None, true, writes_in_place).unwrap();
    std::env::set_current_dir(old_cwd).unwrap();
    assert!(writes_in_place);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, PathBuf::from("docs/main.mec"));
    assert_eq!(
        format_output_file_for_target(&targets[0], Path::new("docs"), false, writes_in_place, true),
        PathBuf::from("docs/main.html")
    );
    ensure_unique_format_outputs(&targets, Path::new("docs"), false, writes_in_place, true)
        .unwrap();
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
    let output_matches_input_dir =
        format_output_matches_input_dir(&mech_paths, Path::new("docs"), false).unwrap();
    assert!(output_matches_input_dir);
    let error = format!(
        "{:?}",
        reject_ambiguous_matching_output_dir(
            output_matches_input_dir,
            mech_paths.len(),
            Path::new("docs")
        )
        .unwrap_err()
    );
    assert!(
        error.contains("matches one of multiple format inputs"),
        "got {error}"
    );
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
    assert_eq!(
        format_output_file_for_target(&targets[0], Path::new("out"), false, false, false),
        PathBuf::from("out/main.mec")
    );
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
    let error = format!(
        "{:?}",
        reject_multi_target_file_output(targets.len(), &root.join("out.mec"), true).unwrap_err()
    );
    assert!(
        error.contains("Cannot write 2 formatted sources into single output file"),
        "got {error}"
    );
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
    let error = format!(
        "{:?}",
        reject_multi_target_file_output(targets.len(), &root.join("out.mec"), true).unwrap_err()
    );
    assert!(
        error.contains("Cannot write 2 formatted sources into single output file"),
        "got {error}"
    );
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

    let error = format!(
        "{:?}",
        collect_format_targets(&root.join("README.md"), None, false, false).unwrap_err()
    );
    assert!(
        error.contains("Unsupported source extension"),
        "got {error}"
    );
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

    let output =
        format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, false);

    assert_eq!(output, PathBuf::from("formatted").join("main.mec"));
    assert!(!output.components().any(|component| matches!(
        component,
        std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_)
    )));
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

    let output =
        format_output_file_for_target(&targets[0], Path::new("formatted"), false, false, true);

    assert_eq!(output, PathBuf::from("formatted").join("main.html"));
    assert!(!output.components().any(|component| matches!(
        component,
        std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_)
    )));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn format_resource_authority_is_limited_to_configured_paths() {
    let root = temp_root("resource-authority");
    let configured_dir = root.join("configured");
    let unrelated_dir = root.join("unrelated");
    std::fs::create_dir_all(&configured_dir).unwrap();
    std::fs::create_dir_all(&unrelated_dir).unwrap();
    let configured = configured_dir.join("style.css");
    let unrelated = unrelated_dir.join("style.css");
    std::fs::write(&configured, "body{}").unwrap();
    std::fs::write(&unrelated, "body{}").unwrap();
    let authority = build_format_resource_authority(&[configured.to_string_lossy().to_string()], "").unwrap();
    let mut kernel = authority.kernel().clone();
    mech_runtime::check_fs_capability(&mut kernel, authority.subject(), FS_READ, &configured.canonicalize().unwrap()).unwrap();
    let mut kernel = authority.kernel().clone();
    assert!(mech_runtime::check_fs_capability(&mut kernel, authority.subject(), FS_READ, &unrelated.canonicalize().unwrap()).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
