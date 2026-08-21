use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use mech_core::{
    Dictionary, LegacyValue, MechEnum, MechTable, MechTuple, Ref, ValueKind, hash_str,
    matrix::Matrix as ValueMatrix,
};

#[test]
fn resident_repl_starts_without_arguments_and_with_the_compatibility_flag() {
    assert_repl_session(&[]);
    assert_repl_session(&["--repl"]);
}

#[test]
fn resident_repl_multiplies_dynamic_f64_matrices() {
    let output = run_repl(&[], None, "[1 2 3] ** [4 5 6]'\n:quit\n");
    assert!(
        output.contains("32"),
        "missing matrix product from REPL output: {output}"
    );
    assert!(
        !output.contains("ResidentRouteFailure"),
        "matrix multiplication failed resident routing: {output}"
    );
}

#[test]
fn canonical_inline_string_values_round_trip_through_the_mech_parser() {
    let string = LegacyValue::String(Ref::new(
        "quote \" slash \\ line\nreturn\rtab\t α separators\u{2028}\u{2029} nul\0 control\u{1}"
            .to_string(),
    ));
    let matrix = LegacyValue::MatrixString(ValueMatrix::DMatrix(Ref::new(
        nalgebra::DMatrix::from_row_slice(1, 2, &["a\"b".to_string(), "c\\d\nα".to_string()]),
    )));
    let tuple = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![string.clone()])));
    let column_id = hash_str("message");
    let table = LegacyValue::Table(Ref::new(MechTable::from_parts(
        2,
        1,
        vec![(
            column_id,
            ValueKind::String,
            ValueMatrix::from_vec(
                vec![
                    LegacyValue::String(Ref::new("a\"b".to_string())),
                    LegacyValue::String(Ref::new("c\\d\nα".to_string())),
                ],
                2,
                1,
            ),
        )],
        vec![(column_id, "message".to_string())],
    )));
    let nested_column_id = hash_str("nested");
    let nested_table = LegacyValue::Table(Ref::new(MechTable::from_parts(
        1,
        1,
        vec![(
            nested_column_id,
            table.kind(),
            ValueMatrix::from_vec(vec![table.clone()], 1, 1),
        )],
        vec![(nested_column_id, "nested".to_string())],
    )));
    let enum_id = hash_str("status");
    let ready_id = hash_str("status/ready");
    let error_id = hash_str("status/error");
    let mut names = Dictionary::new();
    names.insert(enum_id, "status".to_string());
    names.insert(ready_id, "status/ready".to_string());
    names.insert(error_id, "status/error".to_string());
    let names = Ref::new(names);
    let active_enum = LegacyValue::Enum(Ref::new(MechEnum {
        id: enum_id,
        variants: vec![(error_id, Some(string.clone()))],
        names: names.clone(),
    }));
    let enum_definition = LegacyValue::Enum(Ref::new(MechEnum {
        id: enum_id,
        variants: vec![
            (ready_id, None),
            (error_id, Some(LegacyValue::Kind(ValueKind::String))),
        ],
        names,
    }));

    for value in [
        string,
        matrix,
        tuple,
        table,
        nested_table,
        active_enum,
        enum_definition,
    ] {
        let canonical = value.format_canonical_inline();
        assert!(
            !canonical.chars().any(|character| character.is_control()),
            "canonical formatter leaked a control character: {canonical:?}"
        );
        assert!(
            !canonical.contains(['\u{2028}', '\u{2029}']),
            "canonical formatter leaked a Unicode line separator: {canonical:?}"
        );
        let source = format!("roundtrip := {canonical}");
        mech_syntax::parse(&source)
            .unwrap_or_else(|error| panic!("canonical value did not parse: {source:?}: {error:?}"));
    }
}

#[test]
fn nofun_flag_and_environment_select_a_decoration_free_typed_repl() {
    for (arguments, environment) in [
        (vec!["--nofun"], Vec::new()),
        (Vec::new(), vec![("MECH_NOFUN", "1")]),
        (Vec::new(), vec![("MECH_REPL_STYLE", "plain")]),
        (Vec::new(), vec![("TERM", "dumb")]),
    ] {
        let output =
            run_repl_with_environment(&arguments, &environment, ":help\n[1 2 3; 4 5 6]\n:quit\n");

        assert!(output.starts_with(">: "), "missing REPL prompt: {output:?}");
        assert!(!output.contains("www.mech-lang.org"), "banner: {output}");
        assert!(!output.contains("╭◉╮"), "Mika: {output}");
        assert!(!output.contains("Okay cya!"), "farewell: {output}");
        assert!(!output.contains('\u{1b}'), "ANSI formatting: {output:?}");
        assert!(
            output.contains("|Command<*> Description<*>|"),
            "table was not inline: {output}"
        );
        assert!(
            !output.contains("Host<*>"),
            "help exposed a host column: {output}"
        );
        assert!(
            output.contains("[1 2 3; 4 5 6]"),
            "matrix was not inline: {output}"
        );
        assert!(
            output.contains("[f64]:2,3\n[1 2 3; 4 5 6]\n"),
            "nofun mode dropped the kind/value contract: {output}"
        );
    }
}

#[test]
fn quiet_mode_and_trailing_semicolons_suppress_automatic_values() {
    for (arguments, environment) in [
        (vec!["--nofun", "--quiet"], Vec::new()),
        (vec!["--nofun"], vec![("MECH_REPL_QUIET", "1")]),
        (vec!["--nofun"], vec![("MECH_QUIET", "true")]),
    ] {
        let output = run_repl_with_environment(&arguments, &environment, ":help\n1 + 1\n:quit\n");
        assert!(
            output.contains("|Command<*> Description<*>|"),
            "explicit commands must remain visible in quiet mode: {output}"
        );
        assert!(
            !output.contains("f64\n2\n"),
            "quiet mode leaked an automatic value: {output}"
        );
    }

    let output = run_repl(
        &["--nofun"],
        None,
        "1 + 1;\n1 + 1; -- suppress this value\n1 + 1; // suppress this value too\n2 + 2-- comment semicolon ;\n2 + 3// comment semicolon ;\n1 + 2\n:quit\n",
    );
    assert!(
        !output.contains("f64\n2\n"),
        "semicolon-terminated entry leaked an automatic value: {output}"
    );
    assert!(
        output.contains("f64\n4\n") && output.contains("f64\n5\n") && output.contains("f64\n3\n"),
        "unsuppressed values before adjacent comments did not render: {output}"
    );
}

#[test]
fn repl_commands_are_structured_and_truthful() {
    let output = run_repl(
        &[],
        None,
        concat!(
            "x := 1\n",
            ":help\n",
            ":capabilities\n",
            ":whos\n",
            ":constraints\n",
            ":plan\n",
            ":profile on\n",
            ":step #2 1\n",
            ":quit\n",
        ),
    );

    assert!(
        output.contains("REPL commands"),
        "missing help heading: {output}"
    );
    assert!(
        output.contains("Command") && output.contains("Description"),
        "help lost its structured command index: {output}"
    );
    assert!(
        !output.contains("Host"),
        "help exposed a host column: {output}"
    );
    assert!(
        output.contains("Effective REPL host capabilities"),
        "missing capabilities command: {output}"
    );
    assert!(
        output.contains("cli/env"),
        "missing effective CLI grants: {output}"
    );
    assert!(
        output.contains("Resident values"),
        "whos is not structured: {output}"
    );
    assert!(
        output.contains("Integrity constraints"),
        "constraints is not structured: {output}"
    );
    assert!(
        !output.contains(":symbols"),
        "help still advertises the redundant symbols command: {output}"
    );
    assert!(
        output.contains("Accepted resident turns"),
        "plan summary is missing runtime state: {output}"
    );
    assert!(
        output.contains("unavailable") && output.contains("does not expose a profiling"),
        "profile command is not truthful: {output}"
    );
    assert!(
        output.contains("Step selector #2 is unavailable"),
        "step selector was silently ignored: {output}"
    );
}

#[test]
fn whos_values_are_inline_and_elided_in_rich_and_plain_consoles() {
    let source = concat!(
        "small := [1 2 3; 4 5 6];\n",
        "message := \"a\\\"b\\\\c\\nnext\\u{2028}line\\u{2029}paragraph\";\n",
        "qq := 1..1000;\n",
        "stepped := 1..2..10;\n",
        ":whos message small qq stepped\n",
        ":quit\n",
    );

    for arguments in [Vec::new(), vec!["--nofun"]] {
        let output = run_repl(&arguments, None, &source);
        let (_, values) = output
            .split_once("Resident values")
            .unwrap_or_else(|| panic!("missing :whos response: {output}"));

        assert!(
            values.contains("[1 2 3; 4 5 6]"),
            "matrix preview was not inline: {output}"
        );
        assert!(
            values.contains("\"a\\\"b\\\\c\\nnext\\u{2028}line\\u{2029}paragraph\""),
            "string preview was not canonical inline Mech syntax: {output}"
        );
        assert!(
            values.contains("[1 3 5 7 9]"),
            "exclusive increment range did not use its resident kernel: {output}"
        );
        assert!(
            values.contains("…]"),
            "large preview was not elided with its closing delimiter: {output}"
        );
        assert!(
            !values.contains("ResidentRouteFailure"),
            "exclusive range did not activate in the resident runtime: {output}"
        );
        assert!(
            !values.contains('↩'),
            "preview contained a line-break marker: {output}"
        );
        assert!(
            !values
                .chars()
                .any(|character| "┌┬┐├┼┤└┴┘│─".contains(character)),
            "console table still used box-drawing lines: {output}"
        );
    }
}

#[test]
fn constraints_are_only_exposed_through_their_own_command() {
    let output = run_repl(
        &["--nofun"],
        None,
        "x := 1\nsafe! := x <= 2\n:whos\n:constraints\n:quit\n",
    );
    let (_, after_values_heading) = output
        .split_once("Resident values")
        .unwrap_or_else(|| panic!("missing resident values table: {output}"));
    let (values, constraints) = after_values_heading
        .split_once("Integrity constraints")
        .unwrap_or_else(|| panic!("missing :constraints response: {output}"));

    assert!(
        values.contains(" x f64 1 "),
        "ordinary symbol is missing: {output}"
    );
    assert!(
        !values.contains("safe!"),
        "constraint leaked into resident values: {output}"
    );
    assert!(
        constraints.contains("Constraint<*> Type<*> Value<*>")
            && constraints.contains(" safe! bool true ")
            && constraints.contains("true"),
        "constraint did not receive its own live table: {output}"
    );
}

#[test]
fn filesystem_commands_support_quoted_paths_and_transactional_load_save() {
    let root = unique_temp_directory("mech-repl");
    let source = root.join("source file.mec");
    let subdirectory = root.join("sub directory");
    fs::create_dir_all(&subdirectory).expect("create REPL fixture directory");
    fs::write(&source, "loaded-value := 7\nloaded-value\n").expect("write REPL source fixture");

    let output = run_repl(
        &[],
        Some(&root),
        concat!(
            ":ls\n",
            ":load \"source file.mec\"\n",
            ":save \"saved session.mec\"\n",
            ":cd \"sub directory\"\n",
            ":ls\n",
            ":quit\n",
        ),
    );

    assert!(
        output.contains("Directory:"),
        "ls lost its heading: {output}"
    );
    assert!(
        output.contains("Type") && output.contains("Size"),
        "ls is not tabular: {output}"
    );
    assert!(
        output.contains("Loaded 1 source file(s) transactionally"),
        "load did not report transactional success: {output}"
    );
    assert!(
        output.contains('7'),
        "loaded source was not evaluated: {output}"
    );
    assert!(
        output.contains("Working directory:"),
        "cd did not report the new directory: {output}"
    );
    let saved = fs::read_to_string(root.join("saved session.mec"))
        .expect("saved accepted REPL session source");
    assert!(saved.contains("loaded-value := 7"));

    fs::remove_dir_all(root).expect("remove REPL fixture directory");
}

#[test]
fn code_command_reports_its_own_result_after_loading_a_rich_document() {
    let output = run_repl(
        &["--nofun"],
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))),
        concat!(
            ":load examples/working/fizzbuzz.mec\n",
            ":code 1 + 1\n",
            ":code [1 2 3]\n",
            ":quit\n",
        ),
    );

    assert!(
        output.contains("f64\n2\n>: [f64]:1,3\n[1 2 3]\n>: REPL session terminated."),
        "`:code` selected an older document output instead of the submitted result: {output}"
    );
}

#[test]
fn command_errors_do_not_terminate_the_repl() {
    let (output, errors) = run_repl_streams(
        &[],
        None,
        concat!(
            ":cd definitely-not-a-real-directory\n",
            "x := 1\n",
            ":whos missing\n",
            ":clear\n",
            ":step\n",
            "1 + 1\n",
            ":quit\n",
        ),
    );
    assert!(
        errors.contains("Unable to change directory"),
        "missing command error: {errors}"
    );
    assert!(
        errors.contains("missing") && errors.contains("no resident program is active"),
        "shared dispatcher errors escaped or were not diagnosed: {errors}"
    );
    assert!(
        output.contains("f64") && output.contains('2'),
        "REPL died after command error: {output}"
    );
}

#[test]
fn program_output_diagnostics_and_history_use_distinct_channels() {
    let (output, errors) = run_repl_streams(
        &[],
        None,
        concat!(
            "+> @out := cli/stdout\n",
            "+> @err := cli/stderr\n",
            "@out/line <- \"program-output\"\n",
            "@err/text <- \"program-\"\n",
            "@err/line <- \"error\"\n",
            ":outputs\n",
            ":output output-1\n",
            ":clear output\n",
            ":clear errors\n",
            ":quit\n",
        ),
    );

    assert!(output.contains("program-output"), "stdout: {output}");
    assert!(!output.contains("program-error"), "stdout: {output}");
    assert!(errors.contains("program-error"), "stderr: {errors}");
    assert!(output.contains("Session outputs"), "stdout: {output}");
    assert!(output.contains("output-1"), "stdout: {output}");
    assert!(
        output.contains("Output history cleared"),
        "stdout: {output}"
    );
    assert!(
        output.contains("Diagnostic history cleared"),
        "stdout: {output}"
    );
}

fn assert_repl_session(arguments: &[&str]) {
    let output = run_repl(arguments, None, "1 + 1\n[1 1 2]\n:whos\n:plan\n:quit\n");

    assert!(
        output.contains("www.mech-lang.org"),
        "missing REPL banner: {output}"
    );
    assert!(
        output.contains("Okay cya!"),
        "missing REPL farewell: {output}"
    );
    assert!(output.contains("f64"), "missing scalar type: {output}");
    assert!(
        output.contains("\x1b[38;5;218m"),
        "value kinds lost the REPL's pink ANSI style: {output:?}"
    );
    assert!(
        output.contains("[f64]:1,3"),
        "missing matrix type and shape: {output}"
    );
    assert!(
        output.contains("Resident execution plan"),
        "missing resident plan: {output}"
    );
}

fn run_repl(arguments: &[&str], current_dir: Option<&std::path::Path>, input: &str) -> String {
    let (stdout, stderr) = run_repl_streams(arguments, current_dir, input);
    format!("{stdout}{stderr}")
}

fn run_repl_streams(
    arguments: &[&str],
    current_dir: Option<&std::path::Path>,
    input: &str,
) -> (String, String) {
    run_repl_streams_with_environment(
        arguments,
        current_dir,
        input,
        &[("MECH_REPL_STYLE", "rich")],
    )
}

fn run_repl_with_environment(
    arguments: &[&str],
    environment: &[(&str, &str)],
    input: &str,
) -> String {
    let (stdout, stderr) = run_repl_streams_with_environment(arguments, None, input, environment);
    format!("{stdout}{stderr}")
}

fn run_repl_streams_with_environment(
    arguments: &[&str],
    current_dir: Option<&std::path::Path>,
    input: &str,
    environment: &[(&str, &str)],
) -> (String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
    command
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("MECH_NOFUN")
        .env_remove("MECH_REPL_STYLE")
        .env_remove("MECH_REPL_QUIET")
        .env_remove("MECH_QUIET")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CI")
        .env("TERM", "xterm-256color")
        .envs(environment.iter().copied());
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    let mut child = command.spawn().expect("start the Mech CLI");
    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(input.as_bytes())
        .expect("write REPL transcript");

    let output = child.wait_with_output().expect("wait for Mech CLI");
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    (
        String::from_utf8(output.stdout).expect("UTF-8 REPL stdout"),
        String::from_utf8(output.stderr).expect("UTF-8 REPL stderr"),
    )
}

fn unique_temp_directory(prefix: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()))
}
