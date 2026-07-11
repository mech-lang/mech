use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::{LoadedMechConfig, resolve_config_path};
use clap::{Arg, ArgAction, Command};
use clap::parser::ValueSource;
use mech_core::*;
use mech_runtime::{
    ConfigCapabilityKind, DefaultIdGenerator, FS_IMPORT, FS_LIST, FS_READ, FS_RESOLVE, FS_SERVE,
    FS_WATCH, HostFilesystemAuthority, MECH_TOOL_SUBJECT, SharedCapabilityKernel,
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

pub(crate) fn filesystem_capability_args_present(matches: &clap::ArgMatches) -> bool {
    ["cap_root", "allow_read", "allow_watch", "allow_serve", "no_default_capabilities"]
        .iter()
        .any(|id| matches.value_source(id) == Some(ValueSource::CommandLine))
}


#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FilesystemCapabilityArgs {
    pub cap_roots: Vec<PathBuf>,
    pub allow_read: Vec<PathBuf>,
    pub allow_watch: Vec<PathBuf>,
    pub allow_serve: Vec<PathBuf>,
    pub no_default_capabilities: bool,
}

impl FilesystemCapabilityArgs {
    pub(crate) fn from_matches(matches: &clap::ArgMatches) -> Self {
        Self {
            cap_roots: collect_capability_paths(matches, "cap_root"),
            allow_read: collect_capability_paths(matches, "allow_read"),
            allow_watch: collect_capability_paths(matches, "allow_watch"),
            allow_serve: collect_capability_paths(matches, "allow_serve"),
            no_default_capabilities: matches.get_flag("no_default_capabilities"),
        }
    }
}

fn validation_error(msg: impl Into<String>) -> MechError {
    MechError::new(GenericError { msg: msg.into() }, None).with_compiler_loc()
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesystemCapabilityEvent {
    DefaultGrant {
        path: PathBuf,
        recursive: bool,
        operations: Vec<&'static str>,
    },
    CliGrant {
        source_flag: &'static str,
        path: PathBuf,
        recursive: bool,
        operations: Vec<&'static str>,
    },
    ConfigGrant {
        path: PathBuf,
        recursive: bool,
        operations: Vec<&'static str>,
    },
    NoGrants,
}

pub struct FilesystemAuthorityBuild {
    pub authority: HostFilesystemAuthority,
    pub events: Vec<FilesystemCapabilityEvent>,
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
    path: &Path,
    recursive: bool,
    operations: &[&'static str],
) -> MResult<()> {
    authority.grant_path(id_generator, path, recursive, operations.iter().copied())?;
    Ok(())
}

fn add_config_capability_grant(
    grants: &mut BTreeMap<CapabilityGrantKey, BTreeSet<&'static str>>,
    base_dir: &Path,
    grant: &mech_runtime::ConfigCapabilityGrant,
) -> MResult<()> {
    let path = resolve_config_path(base_dir, &grant.path);
    let path = resolve_capability_path(&path)?;
    match grant.kind {
        ConfigCapabilityKind::CapRoot => {
            if !path.is_dir() {
                return Err(validation_error(format!(
                    "config cap-root path must be an existing directory: {}",
                    path.display()
                )));
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

pub(crate) fn build_mech_filesystem_authority(
    args: &FilesystemCapabilityArgs,
    config: Option<&LoadedMechConfig>,
) -> MResult<FilesystemAuthorityBuild> {
    let mut id_generator = DefaultIdGenerator::new();
    let kernel = SharedCapabilityKernel::new();
    let mut authority = HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, kernel);
    let mut grants = BTreeMap::<CapabilityGrantKey, BTreeSet<&'static str>>::new();
    let mut events = Vec::new();

    let cap_roots = args.cap_roots.clone();
    let allow_read = args.allow_read.clone();
    let allow_watch = args.allow_watch.clone();
    let allow_serve = args.allow_serve.clone();
    let no_default = args.no_default_capabilities;
    let has_config_capabilities =
        config.is_some_and(|loaded| !loaded.document.capabilities.is_empty());
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
            root.clone(),
            true,
            &[FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
        );
        events.push(FilesystemCapabilityEvent::DefaultGrant {
            path: root,
            recursive: true,
            operations: vec![FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
        });
    }

    for path in cap_roots {
        let path = resolve_capability_path(&path)?;
        if !path.is_dir() {
            return Err(validation_error(format!(
                "--cap-root path must be an existing directory: {}",
                path.display()
            )));
        }
        add_capability_grant(
            &mut grants,
            path.clone(),
            true,
            &[FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
        );
        events.push(FilesystemCapabilityEvent::CliGrant {
            source_flag: "--cap-root",
            path,
            recursive: true,
            operations: vec![FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
        });
    }

    for path in allow_read {
        let path = resolve_capability_path(&path)?;
        let recursive = path_is_recursive_capability_target(&path);
        if recursive {
            add_capability_grant(
                &mut grants,
                path.clone(),
                true,
                &[FS_READ, FS_LIST, FS_RESOLVE, FS_IMPORT],
            );
            events.push(FilesystemCapabilityEvent::CliGrant {
                source_flag: "--allow-read",
                path,
                recursive: true,
                operations: vec![FS_READ, FS_LIST, FS_RESOLVE, FS_IMPORT],
            });
        } else {
            add_capability_grant(
                &mut grants,
                path.clone(),
                false,
                &[FS_READ, FS_RESOLVE, FS_IMPORT],
            );
            events.push(FilesystemCapabilityEvent::CliGrant {
                source_flag: "--allow-read",
                path,
                recursive: false,
                operations: vec![FS_READ, FS_RESOLVE, FS_IMPORT],
            });
        }
    }

    for path in allow_watch {
        let path = resolve_capability_path(&path)?;
        let recursive = path_is_recursive_capability_target(&path);
        add_capability_grant(&mut grants, path.clone(), recursive, &[FS_WATCH]);
        events.push(FilesystemCapabilityEvent::CliGrant {
            source_flag: "--allow-watch",
            path,
            recursive,
            operations: vec![FS_WATCH],
        });
    }

    for path in allow_serve {
        let path = resolve_capability_path(&path)?;
        let recursive = path_is_recursive_capability_target(&path);
        add_capability_grant(&mut grants, path.clone(), recursive, &[FS_SERVE]);
        events.push(FilesystemCapabilityEvent::CliGrant {
            source_flag: "--allow-serve",
            path,
            recursive,
            operations: vec![FS_SERVE],
        });
    }

    if let Some(loaded) = config {
        let before = grants.clone();
        for grant in &loaded.document.capabilities {
            add_config_capability_grant(&mut grants, &loaded.base_dir, grant)?;
        }
        for (key, ops) in grants.iter() {
            if before.get(key) != Some(ops) {
                events.push(FilesystemCapabilityEvent::ConfigGrant {
                    path: key.path.clone(),
                    recursive: key.recursive,
                    operations: ops.iter().copied().collect(),
                });
            }
        }
    }

    if grants.is_empty() {
        events.push(FilesystemCapabilityEvent::NoGrants);
    }

    for (key, operations) in grants {
        let operations = operations.into_iter().collect::<Vec<_>>();
        grant_mech_filesystem_path(
            &mut authority,
            &mut id_generator,
            &key.path,
            key.recursive,
            &operations,
        )?;
    }

    Ok(FilesystemAuthorityBuild { authority, events })
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
            let lock = crate::cli::CURRENT_DIR_LOCK.lock().unwrap();
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
        let command = crate::cli::config::add_config_args(add_filesystem_capability_args(
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

    fn parse_config(source: &str) -> LoadedMechConfig {
        LoadedMechConfig {
            path: PathBuf::from("test.mcfg"),
            base_dir: PathBuf::new(),
            document: parse_config_document(
                "test.mcfg".to_string(),
                source,
                ConfigProfileOptions::default(),
            )
            .unwrap(),
            discovered_project_dir: None,
        }
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
        {
            let _guard = CurrentDirGuard::enter(&root);
            let config = parse_config(
                r#"config := {capabilities: [{allow: "read", path: "allowed"} {allow: "watch", path: "allowed"} {allow: "serve", path: "allowed"}]}"#,
            );
            let matches = cli(&["mech", "serve"]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let authority = build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), Some(&config))
                .unwrap()
                .authority;
            assert!(delegate_all(&authority, &allowed).is_ok());
            assert!(delegate_all(&authority, &outside).is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_default_capabilities_does_not_suppress_config_grants() {
        let root = temp_root("no-default-config");
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let config = parse_config(
                r#"config := {capabilities: [{allow: "read", path: "allowed"} {allow: "watch", path: "allowed"} {allow: "serve", path: "allowed"}]}"#,
            );
            let matches = cli(&["mech", "--no-default-capabilities", "serve"]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let authority = build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), Some(&config))
                .unwrap()
                .authority;
            assert!(delegate_all(&authority, &allowed).is_ok());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_config_no_default_capabilities_grants_nothing() {
        let root = temp_root("grants-nothing");
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let matches = cli(&[
                "mech",
                "--no-config",
                "--no-default-capabilities",
                "serve",
                "allowed",
            ]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let authority = build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), None)
                .unwrap()
                .authority;
            assert!(delegate_all(&authority, &allowed).is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_and_cli_capability_grants_aggregate() {
        let root = temp_root("aggregate");
        let allowed = root.join("allowed");
        std::fs::create_dir_all(&allowed).unwrap();
        {
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
            let authority = build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), Some(&config))
                .unwrap()
                .authority;
            assert!(delegate_all(&authority, &allowed).is_ok());
            let mut kernel = authority.kernel().clone();
            assert!(check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, FS_READ, &allowed).is_ok());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_cap_root_requires_existing_directory() {
        let root = temp_root("cap-root-missing");
        {
            let _guard = CurrentDirGuard::enter(&root);
            let config =
                parse_config(r#"config := {capabilities: [{allow: "cap-root", path: "missing"}]}"#);
            let matches = cli(&["mech", "serve"]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            assert!(build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), Some(&config)).is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_capability_paths_resolve_from_config_file_directory() {
        let root = temp_root("relative-capabilities");
        let subdir = root.join("subdir");
        let config_allowed = subdir.join("allowed");
        let cwd_allowed = root.join("allowed");
        let outside = root.join("outside");
        std::fs::create_dir_all(&config_allowed).unwrap();
        std::fs::create_dir_all(&cwd_allowed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(
            subdir.join(mech_runtime::DEFAULT_CONFIG_FILENAME),
            r#"config := {
 capabilities: [
  {allow: "read", path: "allowed"}
  {allow: "watch", path: "allowed"}
  {allow: "serve", path: "allowed"}
 ]
}
"#,
        )
        .unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let matches = cli(&["mech", "--config", "subdir/mech.mcfg", "serve"]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let loaded = crate::cli::config::load_cli_config(serve_matches)
                .unwrap()
                .unwrap();
            let authority = build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), Some(&loaded))
                .unwrap()
                .authority;
            assert!(delegate_all(&authority, &config_allowed).is_ok());
            assert!(delegate_all(&authority, &cwd_allowed).is_err());
            assert!(delegate_all(&authority, &outside).is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_capability_paths_normalize_backslashes() {
        let root = temp_root("normalize-capabilities");
        let subdir = root.join("subdir");
        let allowed = subdir.join("allowed/nested");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::write(
      subdir.join(mech_runtime::DEFAULT_CONFIG_FILENAME),
      r#"config := {capabilities: [{allow: "read", path: "allowed\\nested"} {allow: "watch", path: "allowed\\nested"} {allow: "serve", path: "allowed\\nested"}]}"#,
    )
    .unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let matches = cli(&["mech", "--config", "subdir/mech.mcfg", "serve"]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let loaded = crate::cli::config::load_cli_config(serve_matches)
                .unwrap()
                .unwrap();
            let authority = build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), Some(&loaded))
                .unwrap()
                .authority;
            assert!(delegate_all(&authority, &allowed).is_ok());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cli_capability_paths_remain_current_dir_relative() {
        let root = temp_root("cli-capabilities-cwd-relative");
        let subdir = root.join("subdir");
        let config_allowed = subdir.join("allowed");
        let cli_allowed = root.join("cli-allowed");
        let accidental_subdir_cli_allowed = subdir.join("cli-allowed");
        std::fs::create_dir_all(&config_allowed).unwrap();
        std::fs::create_dir_all(&cli_allowed).unwrap();
        std::fs::create_dir_all(&accidental_subdir_cli_allowed).unwrap();
        std::fs::write(
            subdir.join(mech_runtime::DEFAULT_CONFIG_FILENAME),
            r#"config := {capabilities: [{allow: "read", path: "allowed"}]}"#,
        )
        .unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let matches = cli(&[
                "mech",
                "--config",
                "subdir/mech.mcfg",
                "--allow-watch",
                "cli-allowed",
                "--allow-serve",
                "cli-allowed",
                "serve",
            ]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let loaded = crate::cli::config::load_cli_config(serve_matches)
                .unwrap()
                .unwrap();
            let authority = build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), Some(&loaded))
                .unwrap()
                .authority;

            let mut ids = DefaultIdGenerator::new();
            assert!(
                authority
                    .delegate_path_to(
                        &mut ids,
                        SERVE_HOST_SUBJECT,
                        &config_allowed,
                        true,
                        [FS_READ, FS_LIST, FS_RESOLVE, FS_IMPORT],
                    )
                    .is_ok()
            );
            assert!(
                authority
                    .delegate_path_to(
                        &mut ids,
                        SERVE_HOST_SUBJECT,
                        &cli_allowed,
                        true,
                        [FS_WATCH, FS_SERVE],
                    )
                    .is_ok()
            );
            assert!(
                authority
                    .delegate_path_to(
                        &mut ids,
                        SERVE_HOST_SUBJECT,
                        &accidental_subdir_cli_allowed,
                        true,
                        [FS_WATCH, FS_SERVE],
                    )
                    .is_err()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn file_scoped_allow_watch_stays_scoped_to_file() {
        let root = temp_root("file-watch-parent-grant");
        let target = root.join("main.mec");
        std::fs::write(&target, "x := 1").unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let matches = cli(&[
                "mech",
                "--no-config",
                "--no-default-capabilities",
                "--allow-watch",
                "main.mec",
                "serve",
                "main.mec",
            ]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let authority = build_mech_filesystem_authority(&FilesystemCapabilityArgs::from_matches(serve_matches), None)
                .unwrap()
                .authority;
            let mut ids = DefaultIdGenerator::new();
            assert!(
                authority
                    .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &target, false, [FS_WATCH],)
                    .is_ok()
            );
            assert!(
                authority
                    .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, false, [FS_WATCH],)
                    .is_err()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(feature = "run")]
pub(crate) struct FilesystemRuntimeAccess {
    pub authority: HostFilesystemAuthority,
    pub kernel: SharedCapabilityKernel,
    pub events: Vec<FilesystemCapabilityEvent>,
}

#[cfg(feature = "run")]
pub(crate) fn build_filesystem_runtime_access(
    args: &FilesystemCapabilityArgs,
    loaded_config: Option<&LoadedMechConfig>,
) -> MResult<FilesystemRuntimeAccess> {
    let build = build_mech_filesystem_authority(args, loaded_config)?;
    let kernel = build.authority.kernel().clone();
    Ok(FilesystemRuntimeAccess {
        authority: build.authority,
        kernel,
        events: build.events,
    })
}

#[cfg(feature = "run")]
pub(crate) fn install_file_resolver(
    runtime: &mut mech_runtime::MechRuntime,
    access: &FilesystemRuntimeAccess,
    cwd: &Path,
) -> MResult<()> {
    runtime.set_source_resolver(
        mech_runtime::FileSourceResolver::new(cwd)
            .with_capabilities(access.kernel.clone(), MECH_TOOL_SUBJECT),
    );
    Ok(())
}
