use super::*;
use mech_gpu::native_artifact::GeneratedLinkedApplication;

pub(super) struct Built {
    pub(super) executable: PathBuf,
    pub(super) units: Vec<Value>,
    pub(super) compiler_profiles: Vec<Value>,
}

pub(super) fn project(
    project: &GeneratedLinkedApplication,
    name: &str,
    target: &Path,
    directory: &Path,
    profile: profile::Profile,
) -> Built {
    fs::copy(
        workspace().join("Cargo.lock"),
        project.root().join("Cargo.lock"),
    )
    .unwrap();
    let isolated = project.root().ancestors().last().unwrap();
    let metadata = bounded(
        &mut CargoInvocation::resolve_lockfile(project.manifest_path(), true)
            .with_current_dir(isolated)
            .command(),
        directory,
        "metadata",
        120,
    );
    assert!(
        metadata.status.success(),
        "{}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    let metadata: Value = serde_json::from_slice(&metadata.stdout).unwrap();
    let mut command = profile
        .invocation(&project.manifest_path(), name, target)
        .with_current_dir(isolated)
        .command();
    command
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_BUILD_JOBS", "1");
    let built = bounded(&mut command, directory, "build", 900);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let messages = parse_cargo_build_messages(Cursor::new(&built.stdout)).unwrap();
    assert_eq!(messages.build_finished, Some(true));
    let units = closure(&metadata, &built.stdout);
    for unit in &units {
        if unit["name"] == "mech-runtime" {
            for feature in unit["features"].as_array().unwrap() {
                assert!(
                    ![
                        "source",
                        "semantic-compiler",
                        "compiler",
                        "full_compiler",
                        "resident-routing-source"
                    ]
                    .contains(&feature.as_str().unwrap())
                );
            }
        }
    }
    assert!(units.iter().any(|unit| unit["name"] == "mech-runtime"));
    assert!(units.iter().any(|unit| unit["name"] == "mech-terminal"));
    let executable = messages.select_binary(name).unwrap().executable;
    let compiler_profiles = profile.audit(&built.stdout, &executable);
    Built {
        executable,
        units,
        compiler_profiles,
    }
}
