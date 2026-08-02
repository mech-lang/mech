mod fingerprint;
mod registry;
mod workspace;

use std::path::PathBuf;

use mech_core::MResult;

use crate::error::{NativeBuildErrorKind, native_build_error};

pub use fingerprint::*;
pub use registry::*;
pub use workspace::*;

/// Trusted source used to resolve packages selected by native planning.
///
/// This value is supplied by the build environment. It is never inferred from
/// bytecode or runtime strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeDependencySource {
    Registry { version: String },
    Workspace { root: PathBuf },
}

pub(crate) fn validate_exact_registry_version(version: &str) -> MResult<()> {
    let (without_build, build) = match version.split_once('+') {
        Some((without_build, build)) if !build.is_empty() && !build.contains('+') => {
            (without_build, Some(build))
        }
        Some(_) => return invalid_registry_version(version),
        None => (version, None),
    };
    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, prerelease)) if !prerelease.is_empty() => (core, Some(prerelease)),
        Some(_) => return invalid_registry_version(version),
        None => (without_build, None),
    };
    let components = core.split('.').collect::<Vec<_>>();
    if components.len() != 3 || !components.iter().copied().all(canonical_numeric) {
        return invalid_registry_version(version);
    }
    if prerelease.is_some_and(|value| !valid_identifiers(value, true))
        || build.is_some_and(|value| !valid_identifiers(value, false))
    {
        return invalid_registry_version(version);
    }
    Ok(())
}

fn canonical_numeric(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value.len() == 1 || !value.starts_with('0'))
        && value.parse::<u64>().is_ok()
}

fn valid_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    value.split('.').all(|identifier| {
        !identifier.is_empty()
            && identifier
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && !(reject_numeric_leading_zero
                && identifier.bytes().all(|byte| byte.is_ascii_digit())
                && identifier.len() > 1
                && identifier.starts_with('0'))
    })
}

fn invalid_registry_version<T>(version: &str) -> MResult<T> {
    Err(native_build_error(
        NativeBuildErrorKind::NativeDependencyInvalid {
            reason: format!("registry version `{version}` is not an exact semantic version"),
        },
        None,
    ))
}

#[cfg(test)]
mod version_tests {
    use super::*;

    #[test]
    fn registry_versions_are_exact_semantic_versions() {
        for valid in [
            "0.3.5",
            "1.2.3-alpha.1",
            "1.2.3-alpha-beta",
            "1.2.3+build.7",
        ] {
            validate_exact_registry_version(valid).unwrap();
        }
        for invalid in ["", "*", "^0.3", "0.3", "0.03.5", "1.2.3 || 2"] {
            assert!(
                validate_exact_registry_version(invalid).is_err(),
                "{invalid}"
            );
        }
    }
}
