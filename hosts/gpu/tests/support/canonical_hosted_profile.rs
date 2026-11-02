//! Explicit Rust build profile; this does not change the checked Object's flags.
use mech_build::CargoInvocation;
use serde_json::{Value, json};
use std::{ffi::OsStr, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Profile {
    Debug,
    Release,
}
impl Profile {
    pub(super) fn parse(value: Option<&OsStr>) -> Result<Self, &'static str> {
        match value {
            None => Ok(Self::Debug),
            Some(value) if value == "debug" => Ok(Self::Debug),
            Some(value) if value == "release" => Ok(Self::Release),
            Some(_) => Err("MECH_CANONICAL_HOSTED_PROFILE must be debug or release"),
        }
    }
    pub(super) fn release(self) -> bool {
        self == Self::Release
    }
    pub(super) fn name(self) -> &'static str {
        if self.release() { "release" } else { "debug" }
    }
    pub(super) fn invocation(
        self,
        manifest: &Path,
        binary: &str,
        target: &Path,
    ) -> CargoInvocation {
        CargoInvocation::build_linked_native_host(
            manifest,
            binary,
            None,
            target,
            self.release(),
            true,
        )
        .unwrap()
    }
    /// Keep actual compiler-reported profiles, not inferred target-directory names.
    pub(super) fn audit(self, stdout: &[u8], executable: &Path) -> Vec<Value> {
        let mut root = 0;
        let mut profiles = Vec::new();
        for line in stdout
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let message: Value = serde_json::from_slice(line).unwrap();
            if message["reason"] != "compiler-artifact" {
                continue;
            }
            if message["executable"].as_str().map(Path::new) == Some(executable) {
                root += 1;
                assert_eq!(
                    message["profile"]["opt_level"],
                    if self.release() { "3" } else { "0" }
                );
                assert_eq!(message["profile"]["debug_assertions"], !self.release());
                assert_eq!(message["profile"]["test"], false);
            }
            profiles.push(json!({"package_id":message["package_id"],
                "target":message["target"], "features":message["features"],
                "profile":message["profile"], "executable":message["executable"]}));
        }
        assert_eq!(
            root, 1,
            "expected exactly one actual selected executable profile"
        );
        profiles
    }
}

#[test]
fn explicit_profile_is_strict_and_changes_the_actual_cargo_invocation() {
    assert_eq!(Profile::parse(None).unwrap(), Profile::Debug);
    for (name, profile) in [("debug", Profile::Debug), ("release", Profile::Release)] {
        assert_eq!(Profile::parse(Some(OsStr::new(name))).unwrap(), profile);
        assert_eq!(profile.name(), name);
        let invocation = profile.invocation(
            Path::new("/project/Cargo.toml"),
            "hosted",
            Path::new("/target"),
        );
        assert_eq!(
            invocation.arguments().any(|arg| arg == "--release"),
            profile.release()
        );
        assert!(invocation.arguments().any(|arg| arg == "--locked"));
        assert!(invocation.arguments().any(|arg| arg == "--offline"));
        let args = invocation.arguments().collect::<Vec<_>>();
        assert_eq!(args[1], if profile.release() { "rustc" } else { "build" });
        if profile.release() {
            assert_eq!(
                &args[args.len() - 5..],
                ["--", "-C", "lto=fat", "-C", "codegen-units=1"].map(OsStr::new)
            );
        } else {
            assert!(!args.contains(&OsStr::new("--")));
        }
    }
    for value in ["", "Release", "true", "release --config x", "debug "] {
        assert!(Profile::parse(Some(OsStr::new(value))).is_err());
    }
}

#[test]
fn actual_selected_cargo_profile_is_recorded_and_checked() {
    for profile in [Profile::Debug, Profile::Release] {
        let message = json!({"reason":"compiler-artifact", "package_id":"root", "target":{"name":"hosted"},
            "features":[], "executable":"/actual/hosted", "profile":{"opt_level":if profile.release() {"3"} else {"0"},
                "debug_assertions":!profile.release(), "test":false}});
        let profiles = profile.audit(
            &serde_json::to_vec(&message).unwrap(),
            Path::new("/actual/hosted"),
        );
        assert_eq!(profiles[0]["profile"], message["profile"]);
        assert!(
            std::panic::catch_unwind(|| {
                (if profile.release() {
                    Profile::Debug
                } else {
                    Profile::Release
                })
                .audit(
                    &serde_json::to_vec(&message).unwrap(),
                    Path::new("/actual/hosted"),
                )
            })
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(|| profile.audit(
                &serde_json::to_vec(&message).unwrap(),
                Path::new("/wrong/hosted")
            ))
            .is_err()
        );
    }
}
