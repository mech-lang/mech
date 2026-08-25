#![cfg(feature = "full-hosts")]

pub mod support;

use std::process::Command;

use support::*;

#[test]
fn generated_executables_accept_only_once() {
    let result = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Build,
        "literal",
        fixture_path("literal-f64.mecb"),
        "native_generated_arguments",
        false,
    );
    let executable = result.executable.unwrap();

    for arguments in [Vec::<&str>::new(), vec!["--once"]] {
        let output = Command::new(&executable).args(arguments).output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "42");
        assert!(output.stderr.is_empty());
    }

    for arguments in [vec!["--unknown"], vec!["--once", "--unknown"]] {
        let output = Command::new(&executable).args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "usage: generated-app [--once]\n",
        );
    }
}
