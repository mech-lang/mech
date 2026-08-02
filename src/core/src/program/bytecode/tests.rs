use std::collections::{BTreeMap, BTreeSet};

use crate::hash_str;

use super::*;

const HEADER_INSTRUCTION_COUNT: usize = 20;
const HEADER_SECTION_COUNT: usize = 24;
const HEADER_FILE_LEN: usize = 36;
const HEADER_CHECKSUM_OFFSET: usize = 44;

fn empty_constant() -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::Empty,
        alignment: 1,
        bytes: Vec::new(),
    }
}

fn program(constants: Vec<EncodedConstant>) -> BytecodeProgram {
    BytecodeProgram {
        register_count: 1,
        constants,
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
    }
}

fn matrix_constant(storage: MatrixStorage, rows: u32, cols: u32) -> EncodedConstant {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&rows.to_le_bytes());
    bytes.extend_from_slice(&cols.to_le_bytes());
    for value in 0..rows * cols {
        bytes.extend_from_slice(&f64::from(value).to_bits().to_le_bytes());
    }
    EncodedConstant {
        runtime_type: RuntimeType::Matrix {
            element: Box::new(RuntimeType::F64),
            storage,
            rows,
            cols,
        },
        alignment: 8,
        bytes,
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn section_entry_offset(index: usize) -> usize {
    BYTECODE_SECTION_TABLE_OFFSET as usize + index * BYTECODE_SECTION_ENTRY_SIZE
}

fn section_offset(bytes: &[u8], index: usize) -> usize {
    read_u64(bytes, section_entry_offset(index) + 8) as usize
}

fn section_length(bytes: &[u8], index: usize) -> usize {
    read_u64(bytes, section_entry_offset(index) + 16) as usize
}

fn refresh_crc(bytes: &mut [u8]) {
    let checksum_offset = read_u64(bytes, HEADER_CHECKSUM_OFFSET) as usize;
    let checksum = crc32fast::hash(&bytes[..checksum_offset]);
    write_u32(bytes, checksum_offset, checksum);
}

fn assert_validation_reason(bytes: &[u8], expected: &str) {
    let error = ParsedProgram::from_bytes(bytes).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
    assert!(
        error.kind_message().contains(expected),
        "expected `{expected}` in `{}`",
        error.kind_message(),
    );
}

fn return_instruction(register: u32) -> Vec<u8> {
    let mut bytes = vec![Opcode::Return as u8];
    bytes.extend_from_slice(&register.to_le_bytes());
    bytes
}

fn const_load_instruction(destination: u32, constant: u32) -> Vec<u8> {
    let mut bytes = vec![Opcode::ConstLoad as u8];
    bytes.extend_from_slice(&destination.to_le_bytes());
    bytes.extend_from_slice(&constant.to_le_bytes());
    bytes
}

fn replace_instruction_section(bytes: &mut Vec<u8>, instructions: &[u8], count: u32) {
    let index = BytecodeSectionKind::Instructions as usize - 1;
    let start = section_offset(bytes, index);
    let old_next = section_offset(bytes, index + 1);
    let new_end = start + instructions.len();
    let new_next = (new_end + 7) / 8 * 8;
    let mut replacement = instructions.to_vec();
    replacement.resize(new_next - start, 0);
    bytes.splice(start..old_next, replacement);

    let delta = new_next as i64 - old_next as i64;
    write_u32(bytes, HEADER_INSTRUCTION_COUNT, count);
    write_u32(bytes, section_entry_offset(index) + 4, count);
    write_u64(
        bytes,
        section_entry_offset(index) + 16,
        instructions.len() as u64,
    );
    for following in index + 1..BYTECODE_SECTION_COUNT {
        let entry = section_entry_offset(following);
        let old = read_u64(bytes, entry + 8) as i64;
        write_u64(bytes, entry + 8, (old + delta) as u64);
    }
    let file_len = read_u64(bytes, HEADER_FILE_LEN) as i64 + delta;
    let checksum_offset = read_u64(bytes, HEADER_CHECKSUM_OFFSET) as i64 + delta;
    write_u64(bytes, HEADER_FILE_LEN, file_len as u64);
    write_u64(bytes, HEADER_CHECKSUM_OFFSET, checksum_offset as u64);
    refresh_crc(bytes);
}

#[test]
fn official_v1_layout_is_deterministic_and_round_trips() {
    let constants = vec![
        empty_constant(),
        EncodedConstant {
            runtime_type: RuntimeType::Bool,
            alignment: 1,
            bytes: vec![1],
        },
        EncodedConstant {
            runtime_type: RuntimeType::String,
            alignment: 1,
            bytes: b"bytecode-v1".to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::Index,
            alignment: 8,
            bytes: 42_u64.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: (-0.0_f64).to_bits().to_le_bytes().to_vec(),
        },
    ];
    let bytes = write_bytecode(&program(constants.clone())).unwrap();
    assert_eq!(bytes, write_bytecode(&program(constants)).unwrap());

    let parsed = ParsedProgram::from_bytes(&bytes).unwrap();
    assert_eq!(parsed.header.magic, BYTECODE_MAGIC);
    assert_eq!(parsed.header.version, BYTECODE_VERSION);
    assert_eq!(parsed.header.header_size, BYTECODE_HEADER_SIZE);
    assert_eq!(parsed.header.section_count as usize, BYTECODE_SECTION_COUNT);
    assert_eq!(
        parsed.header.section_table_offset,
        BYTECODE_SECTION_TABLE_OFFSET
    );
    assert_eq!(parsed.header.file_len, bytes.len() as u64);
    assert_eq!(parsed.header.checksum_offset, bytes.len() as u64 - 4);
    assert_eq!(parsed.sections.len(), BYTECODE_SECTION_COUNT);

    let decoded = parsed.decode_constants().unwrap();
    assert!(matches!(decoded[0], crate::Value::Empty));
    assert!(matches!(&decoded[1], crate::Value::Bool(value) if *value.borrow()));
    assert!(
        matches!(&decoded[2], crate::Value::String(value) if value.borrow().as_str() == "bytecode-v1")
    );
    assert!(matches!(&decoded[3], crate::Value::Index(value) if *value.borrow() == 42));
    assert!(
        matches!(&decoded[4], crate::Value::F64(value) if value.borrow().to_bits() == (-0.0_f64).to_bits())
    );
}

#[test]
fn header_and_section_directory_have_the_exact_v1_bytes() {
    let bytes = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let (mech_major, mech_minor, mech_patch) = MECH_LANGUAGE_RUNTIME_ABI_VERSION;
    let mut expected = Vec::with_capacity(BYTECODE_HEADER_SIZE as usize);
    expected.extend_from_slice(b"MECH");
    expected.extend_from_slice(&1_u16.to_le_bytes());
    expected.extend_from_slice(&64_u16.to_le_bytes());
    expected.extend_from_slice(&mech_major.to_le_bytes());
    expected.extend_from_slice(&mech_minor.to_le_bytes());
    expected.extend_from_slice(&mech_patch.to_le_bytes());
    expected.extend_from_slice(&0_u16.to_le_bytes());
    expected.extend_from_slice(&1_u32.to_le_bytes());
    expected.extend_from_slice(&2_u32.to_le_bytes());
    expected.extend_from_slice(&7_u16.to_le_bytes());
    expected.extend_from_slice(&0_u16.to_le_bytes());
    expected.extend_from_slice(&64_u64.to_le_bytes());
    expected.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    expected.extend_from_slice(&((bytes.len() - 4) as u64).to_le_bytes());
    expected.extend_from_slice(&[0; 12]);
    assert_eq!(&bytes[..64], expected);

    let expected_counts = [1, 1, 0, 0, 2, 0, 0];
    let expected_lengths = [8, 24, 0, 0, 14, 0, 0];
    let mut previous_end = BYTECODE_CONTENT_OFFSET as usize;
    for (index, expected_kind) in BytecodeSectionKind::ALL.into_iter().enumerate() {
        let entry = section_entry_offset(index);
        assert_eq!(read_u16(&bytes, entry), expected_kind as u16);
        assert_eq!(read_u16(&bytes, entry + 2), 0);
        assert_eq!(read_u32(&bytes, entry + 4), expected_counts[index]);
        assert_eq!(read_u64(&bytes, entry + 24), 0);
        let offset = section_offset(&bytes, index);
        let length = section_length(&bytes, index);
        assert_eq!(length, expected_lengths[index]);
        assert_eq!(offset % 8, 0);
        assert!(offset >= previous_end);
        assert!(bytes[previous_end..offset].iter().all(|byte| *byte == 0));
        previous_end = offset + length;
    }
    let checksum_offset = read_u64(&bytes, HEADER_CHECKSUM_OFFSET) as usize;
    assert!(
        bytes[previous_end..checksum_offset]
            .iter()
            .all(|byte| *byte == 0)
    );
}

#[test]
fn every_f64_matrix_storage_tag_is_accepted() {
    let specifications = [
        (MatrixStorage::Matrix1, 1, 1),
        (MatrixStorage::Matrix2, 2, 2),
        (MatrixStorage::Matrix3, 3, 3),
        (MatrixStorage::Matrix4, 4, 4),
        (MatrixStorage::Matrix2x3, 2, 3),
        (MatrixStorage::Matrix3x2, 3, 2),
        (MatrixStorage::RowVector2, 1, 2),
        (MatrixStorage::RowVector3, 1, 3),
        (MatrixStorage::RowVector4, 1, 4),
        (MatrixStorage::Vector2, 2, 1),
        (MatrixStorage::Vector3, 3, 1),
        (MatrixStorage::Vector4, 4, 1),
        (MatrixStorage::RowVectorD, 1, 5),
        (MatrixStorage::VectorD, 5, 1),
        (MatrixStorage::MatrixD, 2, 5),
    ];
    let constants = specifications
        .into_iter()
        .map(|(storage, rows, cols)| matrix_constant(storage, rows, cols))
        .collect();
    let parsed = ParsedProgram::from_bytes(&write_bytecode(&program(constants)).unwrap()).unwrap();
    assert_eq!(parsed.constants.len(), MatrixStorage::MatrixD as usize);
}

#[test]
fn checksum_corruption_is_rejected() {
    let mut bytes = write_bytecode(&program(vec![empty_constant()])).unwrap();
    bytes[BYTECODE_CONTENT_OFFSET as usize] ^= 1;
    assert_validation_reason(&bytes, "CRC32");
}

#[test]
fn rejects_wrong_magic_version_and_mech_version() {
    let original = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let cases = [
        (0, 0_u16, "magic"),
        (4, BYTECODE_VERSION + 1, "version"),
        (8, u16::MAX, "ABI version"),
    ];
    for (offset, value, reason) in cases {
        let mut bytes = original.clone();
        if offset == 0 {
            bytes[0] = b'X';
        } else {
            write_u16(&mut bytes, offset, value);
        }
        refresh_crc(&mut bytes);
        assert_validation_reason(&bytes, reason);
    }
}

#[test]
fn rejects_duplicate_missing_unknown_overlapping_and_oob_sections() {
    let original = write_bytecode(&program(vec![empty_constant()])).unwrap();

    let mut duplicate = original.clone();
    write_u16(&mut duplicate, section_entry_offset(1), 1);
    refresh_crc(&mut duplicate);
    assert_validation_reason(&duplicate, "missing, duplicate, or out-of-order");

    let mut missing = original.clone();
    write_u16(&mut missing, HEADER_SECTION_COUNT, 6);
    refresh_crc(&mut missing);
    assert_validation_reason(&missing, "exact seven-entry");

    let mut unknown = original.clone();
    write_u16(&mut unknown, section_entry_offset(0), u16::MAX);
    refresh_crc(&mut unknown);
    assert_validation_reason(&unknown, "unknown bytecode section kind");

    let mut overlap = original.clone();
    write_u64(
        &mut overlap,
        section_entry_offset(1) + 8,
        section_offset(&original, 0) as u64,
    );
    refresh_crc(&mut overlap);
    assert_validation_reason(&overlap, "overlapping");

    let mut oob = original;
    let checksum_offset = read_u64(&oob, HEADER_CHECKSUM_OFFSET);
    write_u64(
        &mut oob,
        section_entry_offset(BYTECODE_SECTION_COUNT - 1) + 8,
        checksum_offset + 8,
    );
    refresh_crc(&mut oob);
    assert_validation_reason(&oob, "extends into checksum");
}

#[test]
fn rejects_a_first_content_section_after_offset_288() {
    let mut bytes = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let content_offset = usize::try_from(BYTECODE_CONTENT_OFFSET).unwrap();
    bytes.splice(content_offset..content_offset, [0; 8]);
    for index in 0..BYTECODE_SECTION_COUNT {
        let entry = section_entry_offset(index);
        let offset = read_u64(&bytes, entry + 8);
        write_u64(&mut bytes, entry + 8, offset + 8);
    }
    let file_len = read_u64(&bytes, HEADER_FILE_LEN);
    let checksum_offset = read_u64(&bytes, HEADER_CHECKSUM_OFFSET);
    write_u64(&mut bytes, HEADER_FILE_LEN, file_len + 8);
    write_u64(&mut bytes, HEADER_CHECKSUM_OFFSET, checksum_offset + 8);
    refresh_crc(&mut bytes);

    assert_validation_reason(
        &bytes,
        "first bytecode content section must begin at offset 288",
    );
}

#[test]
fn rejects_impossible_declared_counts_before_reserving() {
    let original = write_bytecode(&program(vec![empty_constant()])).unwrap();

    let mut instructions = original.clone();
    write_u32(&mut instructions, HEADER_INSTRUCTION_COUNT, 1_000_000);
    write_u32(
        &mut instructions,
        section_entry_offset(BytecodeSectionKind::Instructions as usize - 1) + 4,
        1_000_000,
    );
    refresh_crc(&mut instructions);
    assert_validation_reason(&instructions, "instruction count exceeds section capacity");

    let mut dictionary = original.clone();
    write_u32(
        &mut dictionary,
        section_entry_offset(BytecodeSectionKind::Dictionary as usize - 1) + 4,
        1_000_000,
    );
    refresh_crc(&mut dictionary);
    assert_validation_reason(
        &dictionary,
        "dictionary entry count exceeds section capacity",
    );

    let mut requirements = original;
    write_u32(
        &mut requirements,
        section_entry_offset(BytecodeSectionKind::ApplicationRequirements as usize - 1) + 4,
        10_000,
    );
    refresh_crc(&mut requirements);
    assert_validation_reason(
        &requirements,
        "application requirement count exceeds section capacity",
    );
}

#[test]
fn rejects_impossible_variable_lengths_before_reserving() {
    let mut input = program(vec![empty_constant()]);
    input.instructions = vec![
        BytecodeInstruction::RuntimeVariadic {
            function: 1,
            dst: 0,
            arguments: vec![0],
        },
        BytecodeInstruction::Return { src: 0 },
    ];
    let mut variadic = write_bytecode(&input).unwrap();
    let instruction_offset =
        section_offset(&variadic, BytecodeSectionKind::Instructions as usize - 1);
    write_u32(&mut variadic, instruction_offset + 13, 65_536);
    refresh_crc(&mut variadic);
    assert_validation_reason(
        &variadic,
        "variadic argument count exceeds remaining instruction bytes",
    );

    let requirement = ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
        name: "host".into(),
    });
    let mut input = program(vec![empty_constant()]);
    input.requirements.push(requirement);
    let mut requirement = write_bytecode(&input).unwrap();
    let requirement_offset = section_offset(
        &requirement,
        BytecodeSectionKind::ApplicationRequirements as usize - 1,
    );
    write_u32(&mut requirement, requirement_offset + 8, u32::MAX);
    refresh_crc(&mut requirement);
    assert_validation_reason(
        &requirement,
        "requirement string bytes exceed remaining section",
    );
}

#[test]
fn rejects_invalid_utf8_and_dictionary_hashes() {
    let mut utf8 = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::String,
        alignment: 1,
        bytes: b"x".to_vec(),
    }]))
    .unwrap();
    let blob = section_offset(&utf8, BytecodeSectionKind::ConstantBlob as usize - 1);
    utf8[blob] = 0xff;
    refresh_crc(&mut utf8);
    assert_validation_reason(&utf8, "UTF-8 String");

    let name = "answer";
    let id = hash_str(name);
    let mut input = program(vec![empty_constant()]);
    input.symbols.insert(id, 0);
    input.dictionary.insert(id, name.into());
    let mut dictionary = write_bytecode(&input).unwrap();
    let dictionary_offset =
        section_offset(&dictionary, BytecodeSectionKind::Dictionary as usize - 1);
    dictionary[dictionary_offset + 12] ^= 1;
    refresh_crc(&mut dictionary);
    assert_validation_reason(&dictionary, "dictionary");
}

#[test]
fn rejects_invalid_register_constant_and_requirement_indexes() {
    let original = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let instruction_offset =
        section_offset(&original, BytecodeSectionKind::Instructions as usize - 1);

    let mut register = original.clone();
    write_u32(&mut register, instruction_offset + 1, 1);
    refresh_crc(&mut register);
    assert_validation_reason(&register, "register is out of range");

    let mut constant = original;
    write_u32(&mut constant, instruction_offset + 5, 1);
    refresh_crc(&mut constant);
    assert_validation_reason(&constant, "constant index is out of range");

    let requirement = ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
        name: "host".into(),
    });
    let mut input = program(vec![empty_constant()]);
    input.requirements.push(requirement);
    input.instructions = vec![
        BytecodeInstruction::HostCall {
            requirement: 0,
            dst: 0,
            arguments: Vec::new(),
        },
        BytecodeInstruction::Return { src: 0 },
    ];
    let mut requirement = write_bytecode(&input).unwrap();
    let offset = section_offset(&requirement, BytecodeSectionKind::Instructions as usize - 1);
    write_u32(&mut requirement, offset + 1, 1);
    refresh_crc(&mut requirement);
    assert_validation_reason(&requirement, "requirement index is out of range");
}

#[test]
fn rejects_missing_duplicate_and_nonfinal_return() {
    let mut missing = write_bytecode(&program(vec![empty_constant()])).unwrap();
    replace_instruction_section(&mut missing, &const_load_instruction(0, 0), 1);
    assert_validation_reason(&missing, "exactly one Return");

    let mut duplicate = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let mut duplicate_payload = return_instruction(0);
    duplicate_payload.extend_from_slice(&return_instruction(0));
    replace_instruction_section(&mut duplicate, &duplicate_payload, 2);
    assert_validation_reason(&duplicate, "Return must be the final instruction");

    let mut nonfinal = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let mut nonfinal_payload = return_instruction(0);
    nonfinal_payload.extend_from_slice(&const_load_instruction(0, 0));
    replace_instruction_section(&mut nonfinal, &nonfinal_payload, 2);
    assert_validation_reason(&nonfinal, "Return must be the final instruction");
}

#[test]
fn representative_read_limits_are_enforced() {
    let bytes = write_bytecode(&program(vec![empty_constant()])).unwrap();

    let mut limits = BytecodeReadLimits::default();
    limits.max_file_bytes = bytes.len() - 1;
    assert!(ParsedProgram::from_bytes_with_limits(&bytes, limits).is_err());

    let mut limits = BytecodeReadLimits::default();
    limits.max_registers = 0;
    assert!(ParsedProgram::from_bytes_with_limits(&bytes, limits).is_err());

    let mut limits = BytecodeReadLimits::default();
    limits.max_constants = 0;
    assert!(ParsedProgram::from_bytes_with_limits(&bytes, limits).is_err());

    let mut input = program(vec![empty_constant()]);
    input.instructions = vec![
        BytecodeInstruction::RuntimeVariadic {
            function: 1,
            dst: 0,
            arguments: vec![0],
        },
        BytecodeInstruction::Return { src: 0 },
    ];
    let variadic = write_bytecode(&input).unwrap();
    let mut limits = BytecodeReadLimits::default();
    limits.max_variadic_arguments = 0;
    assert!(ParsedProgram::from_bytes_with_limits(&variadic, limits).is_err());
}

#[test]
fn runtime_type_ids_are_independent_of_root_traversal_order() {
    let map = RuntimeType::Map {
        key: Box::new(RuntimeType::F64),
        value: Box::new(RuntimeType::Option(Box::new(RuntimeType::String))),
    };
    let matrix = RuntimeType::Matrix {
        element: Box::new(RuntimeType::F64),
        storage: MatrixStorage::Matrix2,
        rows: 2,
        cols: 2,
    };
    let (forward_types, forward_ids) =
        finalize_runtime_types([&map, &matrix, &RuntimeType::F64]).unwrap();
    let (reverse_types, reverse_ids) =
        finalize_runtime_types([&RuntimeType::F64, &matrix, &map]).unwrap();
    assert_eq!(forward_types, reverse_types);
    assert_eq!(forward_ids, reverse_ids);
    assert!(forward_ids[&RuntimeType::F64] < forward_ids[&matrix]);
    assert!(forward_ids[&RuntimeType::String] < forward_ids[&map]);
}

#[test]
fn requirements_use_the_explicit_canonical_order() {
    let mut requirements = vec![
        ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: "mech://z".into(),
            path: "a".into(),
            context_name: "ctx".into(),
            operation: "read".into(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Snapshot,
        }),
        ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: "mech://a".into(),
            path: "z".into(),
            context_name: "ctx".into(),
            operation: "write".into(),
            intent: ResourceIntent::Assign,
            delivery: ResourceDelivery::Live,
        }),
        ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: "host".into(),
        }),
    ];
    requirements.sort_by(compare_application_requirements);
    let mut input = program(vec![empty_constant()]);
    input.requirements = requirements.clone();
    let parsed = ParsedProgram::from_bytes(&write_bytecode(&input).unwrap()).unwrap();
    assert_eq!(parsed.requirements, requirements);
}

fn resource_program(base_uri: &str, path: &str) -> BytecodeProgram {
    let mut input = program(vec![empty_constant()]);
    input
        .requirements
        .push(ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: base_uri.into(),
            path: path.into(),
            context_name: "ctx".into(),
            operation: "read".into(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Snapshot,
        }));
    input
}

fn resource_requirement_field_offsets(bytes: &[u8]) -> (usize, usize) {
    let start = section_offset(
        bytes,
        BytecodeSectionKind::ApplicationRequirements as usize - 1,
    );
    let operation_len = read_u16(bytes, start + 4) as usize;
    let context_len = read_u16(bytes, start + 6) as usize;
    let primary_len = read_u32(bytes, start + 8) as usize;
    let primary = start + 16 + operation_len + context_len;
    (primary, primary + primary_len)
}

#[test]
fn writer_rejects_noncanonical_resource_identities() {
    for base_uri in ["docs://manual/", "docs://manual//"] {
        let error = write_bytecode(&resource_program(base_uri, "chapter/one")).unwrap_err();
        assert!(error.kind_message().contains("base URI must be canonical"));
    }

    for path in [
        "./chapter",
        "chapter/./one",
        "chapter/../one",
        "chapter//one",
        "/chapter/one",
        "chapter/one/",
    ] {
        let error = write_bytecode(&resource_program("docs://manual", path)).unwrap_err();
        assert!(error.kind_message().contains("path must not"));
    }
}

#[test]
fn reader_rejects_noncanonical_resource_identities() {
    let mut trailing_uri =
        write_bytecode(&resource_program("docs://manualx", "chapter/one")).unwrap();
    let (base_uri, _) = resource_requirement_field_offsets(&trailing_uri);
    trailing_uri[base_uri..base_uri + b"docs://manual/".len()].copy_from_slice(b"docs://manual/");
    refresh_crc(&mut trailing_uri);
    assert_validation_reason(&trailing_uri, "base URI must be canonical");

    for (canonical, noncanonical) in [("a/x", "a/."), ("a/xx", "a/.."), ("a/xb", "a//b")] {
        let mut bytes = write_bytecode(&resource_program("docs://manual", canonical)).unwrap();
        let (_, path) = resource_requirement_field_offsets(&bytes);
        bytes[path..path + noncanonical.len()].copy_from_slice(noncanonical.as_bytes());
        refresh_crc(&mut bytes);
        assert_validation_reason(&bytes, "path must not");
    }
}

#[cfg(feature = "compiler")]
mod compiler_tests {
    use crate::program::compiler::{BytecodeCompilerContext, CompileConst, Register};
    use crate::{MResult, Ref, Value, ValueKind};

    use super::*;

    struct RejectingContext;

    impl BytecodeCompilerContext for RejectingContext {
        fn register_for_ptr_with_initialization_status(
            &mut self,
            _pointer: usize,
        ) -> (Register, bool) {
            (0, false)
        }

        fn intern_constant(&mut self, _constant: EncodedConstant) -> MResult<u32> {
            panic!("unsupported constants must fail before interning")
        }

        fn define_symbol(
            &mut self,
            _pointer: usize,
            _register: Register,
            _name: &str,
            _mutable: bool,
        ) -> MResult<()> {
            Ok(())
        }

        fn intern_requirement(&mut self, _requirement: ApplicationRequirement) -> MResult<u32> {
            Ok(0)
        }

        fn emit_const_load(&mut self, _destination: Register, _constant: u32) {}
        fn emit_nullop(&mut self, _function: u64, _destination: Register) {}
        fn emit_unop(&mut self, _function: u64, _destination: Register, _source: Register) {}
        fn emit_binop(
            &mut self,
            _function: u64,
            _destination: Register,
            _lhs: Register,
            _rhs: Register,
        ) {
        }
        fn emit_ternop(
            &mut self,
            _function: u64,
            _destination: Register,
            _a: Register,
            _b: Register,
            _c: Register,
        ) {
        }
        fn emit_quadop(
            &mut self,
            _function: u64,
            _destination: Register,
            _a: Register,
            _b: Register,
            _c: Register,
            _d: Register,
        ) {
        }
        fn emit_varop(
            &mut self,
            _function: u64,
            _destination: Register,
            _arguments: Vec<Register>,
        ) {
        }
        fn emit_host_call(
            &mut self,
            _requirement: u32,
            _destination: Register,
            _arguments: Vec<Register>,
        ) {
        }
        fn emit_resource_read(&mut self, _requirement: u32, _destination: Register) {}
        fn emit_resource_write(
            &mut self,
            _requirement: u32,
            _destination: Register,
            _source: Register,
        ) {
        }
        fn emit_resource_send(
            &mut self,
            _requirement: u32,
            _destination: Register,
            _source: Register,
        ) {
        }
    }

    #[test]
    fn unsupported_phase1_constant_has_the_named_structured_error() {
        let error = 1_u8.compile_const(&mut RejectingContext).unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeConstantUnsupported");
        let detail = error.kind_as::<BytecodeConstantUnsupported>().unwrap();
        assert_eq!(detail.runtime_type, RuntimeType::U8);
        assert_eq!(detail.source_value_kind, ValueKind::U8);
        assert!(!detail.reason.is_empty());
    }

    #[test]
    fn typed_constant_cannot_discard_a_mismatched_declared_type() {
        let value = Value::Typed(Box::new(Value::F64(Ref::new(1.0))), ValueKind::Bool);
        let error = value.compile_const(&mut RejectingContext).unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeConstantUnsupported");
        let detail = error.kind_as::<BytecodeConstantUnsupported>().unwrap();
        assert_eq!(detail.runtime_type, RuntimeType::Bool);
        assert_eq!(detail.source_value_kind, ValueKind::F64);
        assert!(detail.reason.contains("does not match"));
    }
}
