use mech_core::{MResult, ParsedProgram};

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

pub(crate) fn validate_target_index_constants(
    program: &ParsedProgram,
    target: Option<&str>,
) -> MResult<()> {
    let Some(value) = program.maximum_index_constant()? else {
        return Ok(());
    };
    let pointer_width = target_pointer_width(target)?;
    let maximum = match pointer_width {
        16 => u64::from(u16::MAX),
        32 => u64::from(u32::MAX),
        64 => u64::MAX,
        _ => unreachable!("target pointer widths are normalized to 16, 32, or 64"),
    };
    if value > maximum {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeBuildIndexConstantOutOfRange {
                target: target.unwrap_or("current").to_owned(),
                pointer_width,
                value,
            },
            None,
        ));
    }
    Ok(())
}

fn target_pointer_width(target: Option<&str>) -> MResult<u32> {
    let Some(target) = target else {
        return Ok(usize::BITS);
    };
    let architecture = target
        .split('-')
        .next()
        .expect("validated target triple is nonempty");
    if target.contains("_ilp32") || target.ends_with("gnux32") {
        return Ok(32);
    }
    let width = match architecture {
        "avr" | "msp430" => 16,
        "arm64_32" | "arm" | "armeb" | "armebv7r" | "armv4t" | "armv5te" | "armv6" | "armv6k"
        | "armv7" | "armv7a" | "armv7k" | "armv7r" | "armv7s" | "armv8r" | "csky" | "hexagon"
        | "i386" | "i586" | "i686" | "loongarch32" | "m68k" | "mips" | "mipsel" | "mipsisa32r6"
        | "mipsisa32r6el" | "powerpc" | "riscv32" | "riscv32e" | "riscv32em" | "riscv32emc"
        | "riscv32gc" | "riscv32i" | "riscv32im" | "riscv32ima" | "riscv32imac"
        | "riscv32imafc" | "riscv32imc" | "sparc" | "thumbv4t" | "thumbv5te" | "thumbv6"
        | "thumbv6m" | "thumbv7a" | "thumbv7em" | "thumbv7m" | "thumbv7neon" | "thumbv7r"
        | "thumbv8m.base" | "thumbv8m.main" | "thumbv8r" | "wasm32" | "wasm32v1" | "xtensa" => 32,
        "aarch64" | "aarch64_be" | "aarch64v8r" | "amdgcn" | "arm64e" | "arm64ec" | "bpfeb"
        | "bpfel" | "loongarch64" | "mips64" | "mips64el" | "mipsisa64r6" | "mipsisa64r6el"
        | "nvptx64" | "powerpc64" | "powerpc64le" | "riscv64" | "riscv64a23" | "riscv64gc"
        | "riscv64im" | "riscv64imac" | "s390x" | "sparc64" | "sparcv9" | "wasm64" | "x86_64"
        | "x86_64h" => 64,
        _ => {
            return Err(native_build_error(
                NativeBuildErrorKind::NativeBuildTargetPointerWidthUnknown {
                    target: target.to_owned(),
                },
                None,
            ));
        }
    };
    Ok(width)
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
    use std::collections::{BTreeMap, BTreeSet};

    use mech_core::{
        BytecodeInstruction, BytecodeProgram, EncodedConstant, RuntimeType, write_bytecode,
    };

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

    #[test]
    fn target_pointer_width_is_derived_from_the_requested_architecture() {
        assert_eq!(target_pointer_width(None).unwrap(), usize::BITS);
        assert_eq!(
            target_pointer_width(Some("i686-pc-windows-msvc")).unwrap(),
            32
        );
        assert_eq!(target_pointer_width(Some("wasm32-wasip2")).unwrap(), 32);
        assert_eq!(
            target_pointer_width(Some("aarch64-unknown-linux-gnu")).unwrap(),
            64
        );
        assert_eq!(
            target_pointer_width(Some("x86_64-unknown-linux-gnux32")).unwrap(),
            32
        );
        assert!(target_pointer_width(Some("future-unknown-none")).is_err());
    }

    #[test]
    fn cross_target_planning_rejects_index_constants_outside_target_usize() {
        let value = u64::from(u32::MAX) + 1;
        let bytes = write_bytecode(&BytecodeProgram {
            register_count: 1,
            constants: vec![EncodedConstant {
                runtime_type: RuntimeType::Index,
                alignment: 8,
                bytes: value.to_le_bytes().to_vec(),
            }],
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions: vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::Return { src: 0 },
            ],
            dictionary: BTreeMap::new(),
            requirements: Vec::new(),
        })
        .unwrap();
        let program = ParsedProgram::from_bytes(&bytes).unwrap();

        let error =
            validate_target_index_constants(&program, Some("i686-pc-windows-msvc")).unwrap_err();
        assert_eq!(error.kind_name(), "NativeBuildIndexConstantOutOfRange");
        validate_target_index_constants(&program, Some("x86_64-unknown-linux-gnu")).unwrap();
    }
}
