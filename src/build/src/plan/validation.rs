use mech_core::MResult;

use crate::error::{NativeBuildErrorKind, native_build_error};

pub(crate) fn validate_binary_name(value: &str) -> MResult<()> {
    if validate_identifier(value, true) {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeBuildBinaryNameInvalid {
                value: value.to_owned(),
            },
            None,
        ))
    }
}

pub(crate) fn validate_target(value: &str) -> MResult<()> {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeBuildTargetInvalid {
                value: value.to_owned(),
            },
            None,
        ))
    }
}

pub(crate) fn validate_installer_path(value: &str) -> MResult<()> {
    let segments: Vec<_> = value.split("::").collect();
    if segments.len() >= 2
        && segments
            .iter()
            .all(|segment| validate_identifier(segment, false))
    {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeBuildInstallerPathInvalid {
                value: value.to_owned(),
            },
            None,
        ))
    }
}

fn validate_identifier(value: &str, allow_hyphen: bool) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(byte) if byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'_' || (allow_hyphen && byte == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_names_and_paths_use_the_frozen_grammars() {
        assert!(validate_binary_name("mech-app_2").is_ok());
        assert!(validate_binary_name("2mech").is_err());
        assert!(validate_target("aarch64-unknown-linux-gnu").is_ok());
        assert!(validate_target("linux;echo").is_err());
        assert!(validate_installer_path("mech_math::__mech_native::install_add_ss_f64").is_ok());
        assert!(validate_installer_path("install").is_err());
    }
}
