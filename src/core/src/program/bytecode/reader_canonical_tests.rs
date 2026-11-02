use super::*;

fn empty_program() -> BytecodeProgram {
    BytecodeProgram {
        register_count: 0,
        constants: Vec::new(),
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: Vec::new(),
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    }
}

// Core admits framing, not semantic JSON. Engine tests separately exercise the
// actual finalized artifact and embedded/outer requirement agreement.
fn artifact_sections(compute: bool) -> BytecodeArtifactSections {
    BytecodeArtifactSections {
        schemas: b"[]".to_vec(),
        constants: b"[]".to_vec(),
        inputs: b"[]".to_vec(),
        slots: b"[]".to_vec(),
        producers: b"[]".to_vec(),
        nodes: b"[]".to_vec(),
        bindings: b"[]".to_vec(),
        outputs: b"[]".to_vec(),
        integrity_constraints: b"[]".to_vec(),
        operations: b"[]".to_vec(),
        operation_contracts: b"[]".to_vec(),
        compute_regions: if compute { b"[]".to_vec() } else { Vec::new() },
    }
}

fn canonical(compute: bool) -> Vec<u8> {
    write_bytecode_with_artifact(&empty_program(), &artifact_sections(compute)).unwrap()
}

fn legacy_program() -> BytecodeProgram {
    BytecodeProgram {
        register_count: 1,
        constants: vec![EncodedConstant {
            runtime_type: RuntimeType::U64,
            alignment: 8,
            bytes: 7u64.to_le_bytes().to_vec(),
        }],
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        ..empty_program()
    }
}

fn entry(kind: BytecodeSectionKind) -> usize {
    BYTECODE_SECTION_TABLE_OFFSET as usize + (kind as usize - 1) * BYTECODE_SECTION_ENTRY_SIZE
}

fn checksum(bytes: &mut [u8]) {
    let offset = bytes.len() - 4;
    let crc = crc32fast::hash(&bytes[..offset]);
    bytes[offset..].copy_from_slice(&crc.to_le_bytes());
}

fn core_rejected(bytes: &[u8]) {
    assert!(matches!(
        CanonicalArtifactBytecode::from_bytes(bytes),
        Err(CanonicalArtifactBytecodeError::CoreBytecode(_))
    ));
    assert!(ParsedProgram::from_bytes(bytes).is_err());
}

#[test]
fn canonical_envelope_matches_general_reader_for_both_section_layouts() {
    for compute in [false, true] {
        let bytes = canonical(compute);
        let general = ParsedProgram::from_bytes(&bytes).unwrap();
        let canonical = CanonicalArtifactBytecode::from_bytes(&bytes).unwrap();
        assert_eq!(canonical.artifact, general.artifact);
        assert_eq!(canonical.requirements, general.requirements);
        assert!(general.types.is_empty());
        assert!(general.constants.is_empty());
        assert!(general.instructions.is_empty());
    }
}

#[test]
fn canonical_envelope_rejects_legacy_only_mixed_and_malformed_legacy() {
    let legacy = write_bytecode(&legacy_program()).unwrap();
    assert!(ParsedProgram::from_bytes(&legacy).is_ok());
    assert!(matches!(
        CanonicalArtifactBytecode::from_bytes(&legacy),
        Err(CanonicalArtifactBytecodeError::MissingArtifactSections)
    ));
    let mixed = write_bytecode_with_artifact(&legacy_program(), &artifact_sections(false)).unwrap();
    assert!(ParsedProgram::from_bytes(&mixed).is_ok());
    let prohibited = |bytes: &[u8]| match CanonicalArtifactBytecode::from_bytes(bytes) {
        Err(CanonicalArtifactBytecodeError::ParallelLegacyExecutionAuthority { sections }) => {
            sections
        }
        result => panic!("parallel authority was not rejected: {result:?}"),
    };
    let names = prohibited(&mixed);
    for name in [
        "registers",
        "instruction header",
        "types",
        "constants",
        "constant blob",
        "instructions",
    ] {
        assert!(names.contains(&name));
    }
    let mut malformed = mixed.clone();
    let instruction_entry = entry(BytecodeSectionKind::Instructions);
    let offset = u64::from_le_bytes(
        malformed[instruction_entry + 8..instruction_entry + 16]
            .try_into()
            .unwrap(),
    ) as usize;
    malformed[offset] = 255;
    checksum(&mut malformed);
    assert!(ParsedProgram::from_bytes(&malformed).is_err());
    assert!(prohibited(&malformed).contains(&"instructions"));

    // A declared zero count does not erase nonempty legacy storage.
    let mut concealed = mixed;
    concealed[16..24].fill(0);
    for kind in [
        BytecodeSectionKind::Types,
        BytecodeSectionKind::ConstantTable,
        BytecodeSectionKind::Instructions,
    ] {
        concealed[entry(kind) + 4..entry(kind) + 8].fill(0);
    }
    checksum(&mut concealed);
    assert!(prohibited(&concealed).contains(&"types"));
    assert!(prohibited(&concealed).contains(&"constants"));
    assert!(prohibited(&concealed).contains(&"instructions"));
}

#[test]
fn canonical_envelope_rejects_each_nonempty_legacy_declaration() {
    for kind in [
        BytecodeSectionKind::Types,
        BytecodeSectionKind::ConstantTable,
        BytecodeSectionKind::ConstantBlob,
        BytecodeSectionKind::Symbols,
        BytecodeSectionKind::Instructions,
        BytecodeSectionKind::Dictionary,
    ] {
        let mut bytes = canonical(false);
        bytes[entry(kind) + 4..entry(kind) + 8].copy_from_slice(&1u32.to_le_bytes());
        checksum(&mut bytes);
        assert!(
            matches!(
                CanonicalArtifactBytecode::from_bytes(&bytes),
                Err(CanonicalArtifactBytecodeError::ParallelLegacyExecutionAuthority { .. })
            ),
            "{kind:?}"
        );
    }
    for offset in [16, 20] {
        let mut bytes = canonical(false);
        bytes[offset..offset + 4].copy_from_slice(&1u32.to_le_bytes());
        checksum(&mut bytes);
        assert!(matches!(
            CanonicalArtifactBytecode::from_bytes(&bytes),
            Err(CanonicalArtifactBytecodeError::ParallelLegacyExecutionAuthority { .. })
        ));
    }
}

#[test]
fn canonical_envelope_keeps_header_directory_checksum_and_presence_checks() {
    for compute in [false, true] {
        let bytes = canonical(compute);
        for length in [
            0,
            1,
            63,
            BYTECODE_CONTENT_OFFSET as usize - 1,
            bytes.len() - 1,
        ] {
            core_rejected(&bytes[..length]);
        }
        // Magic, version, header size, runtime ABI, flags, section count,
        // reserved header, directory offset, file length and checksum offset.
        for offset in [0, 4, 6, 8, 14, 24, 26, 28, 36, 44, 52] {
            let mut changed = bytes.clone();
            changed[offset] ^= 0x80;
            checksum(&mut changed);
            core_rejected(&changed);
        }
        // Unknown section, flags, minimal aligned offset and reserved word.
        for delta in [0, 2, 8, 24] {
            let mut changed = bytes.clone();
            changed[entry(BytecodeSectionKind::Types) + delta] ^= 0x80;
            checksum(&mut changed);
            core_rejected(&changed);
        }
        let mut changed = bytes.clone();
        changed[entry(BytecodeSectionKind::ArtifactSchemas) + 4
            ..entry(BytecodeSectionKind::ArtifactSchemas) + 8]
            .fill(0);
        checksum(&mut changed);
        core_rejected(&changed);
        let mut changed = bytes;
        *changed.last_mut().unwrap() ^= 1;
        core_rejected(&changed);
    }
}

#[test]
fn canonical_envelope_keeps_requirements_and_all_shared_read_limits() {
    let mut program = empty_program();
    program.requirements = vec![ApplicationRequirement::HostFunction(
        crate::ExecutionHostFunctionRequest {
            name: "fixture/operation".into(),
        },
    )];
    let bytes = write_bytecode_with_artifact(&program, &artifact_sections(true)).unwrap();
    assert_eq!(
        CanonicalArtifactBytecode::from_bytes(&bytes)
            .unwrap()
            .requirements,
        program.requirements
    );
    for limits in [
        BytecodeReadLimits {
            max_file_bytes: bytes.len() - 1,
            ..BytecodeReadLimits::default()
        },
        BytecodeReadLimits {
            max_requirements: 0,
            ..BytecodeReadLimits::default()
        },
        BytecodeReadLimits {
            max_artifact_section_bytes: 1,
            ..BytecodeReadLimits::default()
        },
        BytecodeReadLimits {
            max_artifact_bytes: 1,
            ..BytecodeReadLimits::default()
        },
    ] {
        assert!(ParsedProgram::from_bytes_with_limits(&bytes, limits.clone()).is_err());
        assert!(matches!(
            CanonicalArtifactBytecode::from_bytes_with_limits(&bytes, limits),
            Err(CanonicalArtifactBytecodeError::CoreBytecode(_))
        ));
    }
    let mut changed = bytes;
    changed[entry(BytecodeSectionKind::ApplicationRequirements) + 4
        ..entry(BytecodeSectionKind::ApplicationRequirements) + 8]
        .fill(0);
    checksum(&mut changed);
    core_rejected(&changed);
}

#[test]
fn canonical_envelope_keeps_zero_header_limits_before_authority_rejection() {
    for (offset, limits) in [
        (
            16,
            BytecodeReadLimits {
                max_registers: 0,
                ..BytecodeReadLimits::default()
            },
        ),
        (
            20,
            BytecodeReadLimits {
                max_instructions: 0,
                ..BytecodeReadLimits::default()
            },
        ),
    ] {
        let mut bytes = canonical(false);
        assert!(ParsedProgram::from_bytes_with_limits(&bytes, limits.clone()).is_ok());
        assert!(CanonicalArtifactBytecode::from_bytes_with_limits(&bytes, limits.clone()).is_ok());
        bytes[offset..offset + 4].copy_from_slice(&1u32.to_le_bytes());
        checksum(&mut bytes);
        let general = ParsedProgram::from_bytes_with_limits(&bytes, limits.clone()).unwrap_err();
        let Err(CanonicalArtifactBytecodeError::CoreBytecode(canonical)) =
            CanonicalArtifactBytecode::from_bytes_with_limits(&bytes, limits)
        else {
            panic!("header limit must precede parallel-authority rejection");
        };
        for error in [general, canonical] {
            assert!(format!("{error:?}").contains("bytecode header count exceeds read limit"));
        }
    }
}
