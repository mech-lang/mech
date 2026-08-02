use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mech_core::MResult;

use super::{WorkspaceFingerprint, WorkspacePackage, fingerprint_workspace};
use crate::error::{NativeBuildErrorKind, native_build_error};

/// Trusted mapping from package names to locations under a workspace root.
///
/// The registry is populated from native linkage metadata selected by the
/// planner. It intentionally does not search the workspace or accept package
/// declarations from bytecode.
#[derive(Clone, Debug)]
pub struct WorkspacePackageRegistry {
    root: PathBuf,
    packages: BTreeMap<String, WorkspacePackage>,
}

impl WorkspacePackageRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            packages: BTreeMap::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn insert(&mut self, package: WorkspacePackage) -> MResult<()> {
        if self.packages.contains_key(&package.package) {
            return Err(native_build_error(
                NativeBuildErrorKind::NativeWorkspacePackageDuplicate {
                    package: package.package,
                },
                None,
            ));
        }
        self.packages.insert(package.package.clone(), package);
        Ok(())
    }

    pub fn package(&self, package: &str) -> Option<&WorkspacePackage> {
        self.packages.get(package)
    }

    pub fn packages(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&str, &WorkspacePackage)> + ExactSizeIterator {
        self.packages
            .iter()
            .map(|(name, package)| (name.as_str(), package))
    }

    /// Resolve package names in deterministic package-name order.
    pub fn select<I, S>(&self, names: I) -> MResult<Vec<WorkspacePackage>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let names = names
            .into_iter()
            .map(|name| name.as_ref().to_string())
            .collect::<BTreeSet<_>>();
        names
            .into_iter()
            .map(|name| {
                self.packages.get(&name).cloned().ok_or_else(|| {
                    native_build_error(
                        NativeBuildErrorKind::NativeWorkspacePackageUnknown { package: name },
                        None,
                    )
                })
            })
            .collect()
    }

    pub fn fingerprint<I, S>(&self, names: I) -> MResult<WorkspaceFingerprint>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let selected = self.select(names)?;
        fingerprint_workspace(&self.root, &selected)
    }
}

/// Construct the closed, trusted Phase 1 package-to-path mapping.
///
/// Package names collected from a plan must resolve through this table. A
/// runtime string can therefore never introduce a Cargo dependency or choose
/// an arbitrary workspace path.
pub fn standard_workspace_registry(root: impl Into<PathBuf>) -> MResult<WorkspacePackageRegistry> {
    let mut registry = WorkspacePackageRegistry::new(root);
    for (package, crate_name, relative_path) in [
        ("mech-core", "mech_core", "src/core"),
        ("mech-engine", "mech_engine", "src/engine"),
        ("mech-host-cli", "mech_host_cli", "hosts/cli"),
        ("mech-math", "mech_math", "machines/math"),
        ("mech-runtime", "mech_runtime", "src/runtime"),
    ] {
        registry.insert(WorkspacePackage::new(package, crate_name, relative_path)?)?;
    }
    Ok(registry)
}

/// Resolve selected plan package names through the trusted Phase 1 mapping.
pub fn resolve_planned_packages<I, S>(
    root: impl Into<PathBuf>,
    package_names: I,
) -> MResult<Vec<WorkspacePackage>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    standard_workspace_registry(root)?.select(package_names)
}

/// Fingerprint selected plan packages through the trusted Phase 1 mapping.
pub fn fingerprint_planned_packages<I, S>(
    root: impl Into<PathBuf>,
    package_names: I,
) -> MResult<WorkspaceFingerprint>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let registry = standard_workspace_registry(root)?;
    let selected = registry.select(package_names)?;
    fingerprint_workspace(registry.root(), &selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_selection_is_sorted_deduplicated_and_closed() {
        let mut registry = WorkspacePackageRegistry::new("/trusted/workspace");
        registry
            .insert(WorkspacePackage::new("mech-math", "mech_math", "machines/math").unwrap())
            .unwrap();
        registry
            .insert(WorkspacePackage::new("mech-core", "mech_core", "src/core").unwrap())
            .unwrap();

        let selected = registry
            .select(["mech-math", "mech-core", "mech-math"])
            .unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|package| package.package.as_str())
                .collect::<Vec<_>>(),
            ["mech-core", "mech-math"]
        );
        assert!(registry.select(["from-bytecode"]).is_err());
    }

    #[test]
    fn duplicate_package_registration_is_rejected() {
        let mut registry = WorkspacePackageRegistry::new("/trusted/workspace");
        let package = WorkspacePackage::new("mech-core", "mech_core", "src/core").unwrap();
        registry.insert(package.clone()).unwrap();
        assert!(registry.insert(package).is_err());
    }

    #[test]
    fn standard_registry_is_closed_over_phase_one_packages() {
        let registry = standard_workspace_registry("/trusted/workspace").unwrap();
        assert_eq!(
            registry
                .packages()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            [
                "mech-core",
                "mech-engine",
                "mech-host-cli",
                "mech-math",
                "mech-runtime",
            ]
        );
        assert!(registry.select(["mech-stdlib"]).is_err());
    }
}
