use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
use colored::*;
use mech::*;
use mech_core::*;
use mech_runtime::{
    ConfigCapabilityKind, DefaultIdGenerator, FS_IMPORT, FS_LIST, FS_READ, FS_RESOLVE, FS_SERVE,
    FS_WATCH, HostFilesystemAuthority, MECH_TOOL_SUBJECT, MechConfigDocument,
    SharedCapabilityKernel,
};

pub fn add_filesystem_capability_args(command: Command) -> Command {
    command
        .arg(
            Arg::new("cap_root")
                .long("cap-root")
                .value_name("PATH")
                .num_args(1)
                .action(ArgAction::Append)
                .global(true)
                .help("Grant full recursive filesystem capability authority under PATH."),
        )
        .arg(
            Arg::new("allow_read")
                .long("allow-read")
                .value_name("PATH")
                .num_args(1)
                .action(ArgAction::Append)
                .global(true)
                .help("Grant filesystem read/list/resolve/import authority for PATH."),
        )
        .arg(
            Arg::new("allow_watch")
                .long("allow-watch")
                .value_name("PATH")
                .num_args(1)
                .action(ArgAction::Append)
                .global(true)
                .help("Grant filesystem watch authority for PATH."),
        )
        .arg(
            Arg::new("allow_serve")
                .long("allow-serve")
                .value_name("PATH")
                .num_args(1)
                .action(ArgAction::Append)
                .global(true)
                .help("Grant filesystem serve authority for PATH."),
        )
        .arg(
            Arg::new("no_default_capabilities")
                .long("no-default-capabilities")
                .action(ArgAction::SetTrue)
                .global(true)
                .help(
                    "Disable the default recursive current-directory filesystem capability grant.",
                ),
        )
}

fn collect_capability_paths(matches: &clap::ArgMatches, id: &str) -> Vec<PathBuf> {
    matches
        .get_many::<String>(id)
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .collect()
}

fn resolve_capability_path(path: &Path) -> MResult<PathBuf> {
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if candidate.exists() {
        Ok(candidate.canonicalize()?)
    } else {
        Ok(candidate)
    }
}

fn path_is_recursive_capability_target(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CapabilityGrantKey {
    path: PathBuf,
    recursive: bool,
}

fn add_capability_grant(
    grants: &mut BTreeMap<CapabilityGrantKey, BTreeSet<&'static str>>,
    path: PathBuf,
    recursive: bool,
    operations: &[&'static str],
) {
    grants
        .entry(CapabilityGrantKey { path, recursive })
        .or_default()
        .extend(operations.iter().copied());
}

fn grant_mech_filesystem_path(
    authority: &mut HostFilesystemAuthority,
    id_generator: &mut DefaultIdGenerator,
    badge: &ColoredString,
    path: &Path,
    recursive: bool,
    operations: &[&'static str],
) -> MResult<()> {
    authority.grant_path(id_generator, path, recursive, operations.iter().copied())?;
    println!(
        "{badge} Capability grant: {} {} {} recursive={recursive}",
        MECH_TOOL_SUBJECT,
        operations.join(","),
        mech_runtime::fs_resource_key(path)?,
    );
    Ok(())
}

fn add_config_capability_grant(
    grants: &mut BTreeMap<CapabilityGrantKey, BTreeSet<&'static str>>,
    grant: &mech_runtime::ConfigCapabilityGrant,
) -> MResult<()> {
    let path = resolve_capability_path(&grant.path)?;
    match grant.kind {
        ConfigCapabilityKind::CapRoot => {
            if !path.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "config cap-root path must be an existing directory: {}",
                        path.display()
                    ),
                )
                .into());
            }
            add_capability_grant(
                grants,
                path,
                grant.recursive.unwrap_or(true),
                &[FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
            );
        }
        ConfigCapabilityKind::Read => {
            let recursive = grant
                .recursive
                .unwrap_or_else(|| path_is_recursive_capability_target(&path));
            if recursive {
                add_capability_grant(
                    grants,
                    path,
                    true,
                    &[FS_READ, FS_LIST, FS_RESOLVE, FS_IMPORT],
                );
            } else {
                add_capability_grant(grants, path, false, &[FS_READ, FS_RESOLVE, FS_IMPORT]);
            }
        }
        ConfigCapabilityKind::Watch => {
            let recursive = grant
                .recursive
                .unwrap_or_else(|| path_is_recursive_capability_target(&path));
            add_capability_grant(grants, path, recursive, &[FS_WATCH]);
        }
        ConfigCapabilityKind::Serve => {
            let recursive = grant
                .recursive
                .unwrap_or_else(|| path_is_recursive_capability_target(&path));
            add_capability_grant(grants, path, recursive, &[FS_SERVE]);
        }
    }
    Ok(())
}

pub fn build_mech_filesystem_authority(
    matches: &clap::ArgMatches,
    config: Option<&MechConfigDocument>,
    badge: &ColoredString,
) -> MResult<HostFilesystemAuthority> {
    let mut id_generator = DefaultIdGenerator::new();
    let kernel = SharedCapabilityKernel::new();
    let mut authority = HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, kernel);
    let mut grants = BTreeMap::<CapabilityGrantKey, BTreeSet<&'static str>>::new();

    let cap_roots = collect_capability_paths(matches, "cap_root");
    let allow_read = collect_capability_paths(matches, "allow_read");
    let allow_watch = collect_capability_paths(matches, "allow_watch");
    let allow_serve = collect_capability_paths(matches, "allow_serve");
    let no_default = matches.get_flag("no_default_capabilities");
    let has_config_capabilities = config.is_some_and(|doc| !doc.capabilities.is_empty());
    let explicit = no_default
        || has_config_capabilities
        || !cap_roots.is_empty()
        || !allow_read.is_empty()
        || !allow_watch.is_empty()
        || !allow_serve.is_empty();

    if !explicit {
        let root = std::env::current_dir()?.canonicalize()?;
        add_capability_grant(
            &mut grants,
            root,
            true,
            &[FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
        );
    }

    for path in cap_roots {
        let path = resolve_capability_path(&path)?;
        if !path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "--cap-root path must be an existing directory: {}",
                    path.display()
                ),
            )
            .into());
        }
        add_capability_grant(
            &mut grants,
            path,
            true,
            &[FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
        );
    }

    for path in allow_read {
        let path = resolve_capability_path(&path)?;
        let recursive = path_is_recursive_capability_target(&path);
        if recursive {
            add_capability_grant(
                &mut grants,
                path,
                true,
                &[FS_READ, FS_LIST, FS_RESOLVE, FS_IMPORT],
            );
        } else {
            add_capability_grant(&mut grants, path, false, &[FS_READ, FS_RESOLVE, FS_IMPORT]);
        }
    }

    for path in allow_watch {
        let path = resolve_capability_path(&path)?;
        let recursive = path_is_recursive_capability_target(&path);
        add_capability_grant(&mut grants, path, recursive, &[FS_WATCH]);
    }

    for path in allow_serve {
        let path = resolve_capability_path(&path)?;
        let recursive = path_is_recursive_capability_target(&path);
        add_capability_grant(&mut grants, path, recursive, &[FS_SERVE]);
    }

    if let Some(config) = config {
        for grant in &config.capabilities {
            add_config_capability_grant(&mut grants, grant)?;
        }
    }

    for (key, operations) in grants {
        let operations = operations.into_iter().collect::<Vec<_>>();
        grant_mech_filesystem_path(
            &mut authority,
            &mut id_generator,
            badge,
            &key.path,
            key.recursive,
            &operations,
        )?;
    }

    if authority.source_capabilities().is_empty() {
        println!("{badge} Capability grant: {} <none>", MECH_TOOL_SUBJECT);
    }

    Ok(authority)
}

#[cfg(test)]
mod filesystem_capability_tests {
    use super::*;
    use mech_runtime::{
        ConfigProfileOptions, SERVE_HOST_SUBJECT, check_fs_capability, parse_config_document,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    struct CurrentDirGuard {
        previous: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl CurrentDirGuard {
        fn enter(path: &Path) -> Self {
            let lock = crate::CURRENT_DIR_LOCK.lock().unwrap();
            let previous = std::env::current_dir().unwrap();
            std::env::set_current_dir(path).unwrap();
            Self {
                previous,
                _lock: lock,
            }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.previous).unwrap();
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-cli-capability-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn cli(args: &[&str]) -> clap::ArgMatches {
        let command = crate::config::add_config_args(add_filesystem_capability_args(
            Command::new("mech").subcommand(
                Command::new("serve").arg(
                    Arg::new("mech_serve_file_paths")
                        .required(false)
                        .action(ArgAction::Append),
                ),
            ),
        ));
        command.try_get_matches_from(args).unwrap()
    }

    fn parse_config(source: &str) -> MechConfigDocument {
        parse_config_document(
            "test.mcfg".to_string(),
            source,
            ConfigProfileOptions::default(),
        )
        .unwrap()
    }

    fn delegate_all(authority: &HostFilesystemAuthority, path: &Path) -> MResult<()> {
        let mut ids = DefaultIdGenerator::new();
        authority.delegate_path_to(
            &mut ids,
            SERVE_HOST_SUBJECT,
            path,
            true,
            [FS_READ, FS_LIST, FS_RESOLVE, FS_IMPORT, FS_WATCH, FS_SERVE],
        )?;
        Ok(())
    }

    #[test]
    fn config_capabilities_suppress_default_current_dir_grant() {
        let root = temp_root("suppress-default");
        let allowed = root.join("allowed");
        let outside = root.join("outside");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let _guard = CurrentDirGuard::enter(&root);
        let config = parse_config(
            r#"config := {capabilities: [{allow: "read", path: "allowed"} {allow: "watch", path: "allowed"} {allow: "serve", path: "allowed"}]}"#,
        );
        let matches = cli(&["mech", "serve"]);
        let serve_matches = matches.subcommand_matches("serve").unwrap();
        let badge = "[test]".normal();
        let authority =
            build_mech_filesystem_authority(serve_matches, Some(&config), &badge).unwrap();
        assert!(delegate_all(&authority, &allowed).is_ok());
        assert!(delegate_all(&authority, &outside).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_default_capabilities_does_not_suppress_config_grants() {
        let root = temp_root("no-default-config");
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let _guard = CurrentDirGuard::enter(&root);
        let config = parse_config(
            r#"config := {capabilities: [{allow: "read", path: "allowed"} {allow: "watch", path: "allowed"} {allow: "serve", path: "allowed"}]}"#,
        );
        let matches = cli(&["mech", "--no-default-capabilities", "serve"]);
        let serve_matches = matches.subcommand_matches("serve").unwrap();
        let badge = "[test]".normal();
        let authority =
            build_mech_filesystem_authority(serve_matches, Some(&config), &badge).unwrap();
        assert!(delegate_all(&authority, &allowed).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_config_no_default_capabilities_grants_nothing() {
        let root = temp_root("grants-nothing");
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let _guard = CurrentDirGuard::enter(&root);
        let matches = cli(&[
            "mech",
            "--no-config",
            "--no-default-capabilities",
            "serve",
            "allowed",
        ]);
        let serve_matches = matches.subcommand_matches("serve").unwrap();
        let badge = "[test]".normal();
        let authority = build_mech_filesystem_authority(serve_matches, None, &badge).unwrap();
        assert!(delegate_all(&authority, &allowed).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_and_cli_capability_grants_aggregate() {
        let root = temp_root("aggregate");
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        let _guard = CurrentDirGuard::enter(&root);
        let config =
            parse_config(r#"config := {capabilities: [{allow: "read", path: "allowed"}]}"#);
        let matches = cli(&[
            "mech",
            "--allow-watch",
            "allowed",
            "--allow-serve",
            "allowed",
            "serve",
        ]);
        let serve_matches = matches.subcommand_matches("serve").unwrap();
        let badge = "[test]".normal();
        let authority =
            build_mech_filesystem_authority(serve_matches, Some(&config), &badge).unwrap();
        assert!(delegate_all(&authority, &allowed).is_ok());
        let mut kernel = authority.kernel().clone();
        assert!(check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, FS_READ, &allowed).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_cap_root_requires_existing_directory() {
        let root = temp_root("cap-root-missing");
        let _guard = CurrentDirGuard::enter(&root);
        let config =
            parse_config(r#"config := {capabilities: [{allow: "cap-root", path: "missing"}]}"#);
        let matches = cli(&["mech", "serve"]);
        let serve_matches = matches.subcommand_matches("serve").unwrap();
        let badge = "[test]".normal();
        assert!(build_mech_filesystem_authority(serve_matches, Some(&config), &badge).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
