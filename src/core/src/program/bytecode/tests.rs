use std::collections::{BTreeMap, BTreeSet};

use crate::hash_str;

use super::*;

const HEADER_INSTRUCTION_COUNT: usize = 20;
const HEADER_SECTION_COUNT: usize = 24;
const HEADER_FILE_LEN: usize = 36;
const HEADER_CHECKSUM_OFFSET: usize = 44;
const HEADER_REGISTER_COUNT: usize = 16;

fn empty_constant() -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::Empty,
        alignment: 1,
        bytes: Vec::new(),
    }
}

fn u8_constant(value: u8) -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::U8,
        alignment: 1,
        bytes: vec![value],
    }
}

fn string_constant(value: &str) -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::String,
        alignment: 1,
        bytes: value.as_bytes().to_vec(),
    }
}

fn read_requirement() -> ApplicationRequirement {
    ApplicationRequirement::Resource(ExecutionResourceRequest {
        base_uri: "test://provider/root".into(),
        path: "item".into(),
        context_name: "root".into(),
        operation: "read".into(),
        intent: ResourceIntent::Read,
        delivery: ResourceDelivery::Live,
    })
}

fn read_program(
    register_count: u32,
    constants: Vec<EncodedConstant>,
    instructions: Vec<BytecodeInstruction>,
) -> BytecodeProgram {
    BytecodeProgram {
        register_count,
        constants,
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
        dictionary: BTreeMap::new(),
        requirements: vec![read_requirement()],
    }
}

fn u8_tuple_constant(values: &[u8]) -> EncodedConstant {
    let mut bytes = (values.len() as u32).to_le_bytes().to_vec();
    for value in values {
        append_child_payload(&mut bytes, &[*value]);
    }
    EncodedConstant {
        runtime_type: RuntimeType::Tuple(vec![RuntimeType::U8; values.len()]),
        alignment: 4,
        bytes,
    }
}

fn u8_reference_tuple_constant(value: u8) -> EncodedConstant {
    let mut reference = Vec::new();
    append_child_payload(&mut reference, &[value]);
    let mut bytes = 1u32.to_le_bytes().to_vec();
    append_child_payload(&mut bytes, &reference);
    EncodedConstant {
        runtime_type: RuntimeType::Tuple(vec![RuntimeType::Reference(Box::new(RuntimeType::U8))]),
        alignment: 4,
        bytes,
    }
}

fn program(constants: Vec<EncodedConstant>) -> BytecodeProgram {
    let register_count = constants.len().max(1) as u32;
    let mut instructions = constants
        .iter()
        .enumerate()
        .map(|(constant, _)| BytecodeInstruction::ConstLoad {
            dst: constant as u32,
            constant: constant as u32,
        })
        .collect::<Vec<_>>();
    instructions.push(BytecodeInstruction::Return { src: 0 });
    BytecodeProgram {
        register_count,
        constants,
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
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

fn matrixd_constant(element: RuntimeType, element_bytes: Vec<u8>) -> EncodedConstant {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&element_bytes);
    EncodedConstant {
        runtime_type: RuntimeType::Matrix {
            element: Box::new(element),
            storage: MatrixStorage::MatrixD,
            rows: 1,
            cols: 1,
        },
        alignment: 8,
        bytes,
    }
}

fn append_child_payload(bytes: &mut Vec<u8>, child: &[u8]) {
    bytes.extend_from_slice(&(child.len() as u32).to_le_bytes());
    bytes.extend_from_slice(child);
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

fn type_entry_offset(bytes: &[u8], index: usize) -> usize {
    let mut offset = section_offset(bytes, BytecodeSectionKind::Types as usize - 1);
    for _ in 0..index {
        offset += 8 + read_u32(bytes, offset + 4) as usize;
    }
    offset
}

fn type_entry_with_tag(bytes: &[u8], tag: RuntimeTypeTag) -> usize {
    let count = read_u32(
        bytes,
        section_entry_offset(BytecodeSectionKind::Types as usize - 1) + 4,
    ) as usize;
    (0..count)
        .map(|index| type_entry_offset(bytes, index))
        .find(|offset| read_u16(bytes, *offset) == tag as u16)
        .expect("requested runtime type tag must be present")
}

fn type_id_with_tag(bytes: &[u8], tag: RuntimeTypeTag) -> u32 {
    let count = read_u32(
        bytes,
        section_entry_offset(BytecodeSectionKind::Types as usize - 1) + 4,
    ) as usize;
    (0..count)
        .find(|index| read_u16(bytes, type_entry_offset(bytes, *index)) == tag as u16)
        .map(|index| index as u32)
        .expect("requested runtime type tag must be present")
}

fn constant_entry_offset(bytes: &[u8], index: usize) -> usize {
    section_offset(bytes, BytecodeSectionKind::ConstantTable as usize - 1) + index * 24
}

fn constant_payload_offset(bytes: &[u8], index: usize) -> usize {
    let entry = constant_entry_offset(bytes, index);
    section_offset(bytes, BytecodeSectionKind::ConstantBlob as usize - 1)
        + read_u64(bytes, entry + 8) as usize
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

fn assert_named_id_validation(
    bytes: &[u8],
    category: &str,
    supplied: u64,
    expected: u64,
    name: &str,
) {
    let error = ParsedProgram::from_bytes(bytes).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
    let message = error.kind_message();
    for expected_fragment in [
        category.to_owned(),
        format!("0x{supplied:016x}"),
        format!("0x{expected:016x}"),
        name.to_owned(),
    ] {
        assert!(
            message.contains(&expected_fragment),
            "expected `{expected_fragment}` in `{message}`"
        );
    }
}

fn assert_validation_with_limits(bytes: &[u8], limits: BytecodeReadLimits) {
    let error = ParsedProgram::from_bytes_with_limits(bytes, limits).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
}

fn assert_decode_reason(bytes: &[u8], expected: &str) {
    let error = ParsedProgram::from_bytes(bytes)
        .and_then(|program| program.decode_constants().map(|_| program))
        .unwrap_err();
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

fn replace_section_contents(bytes: &mut Vec<u8>, index: usize, contents: &[u8], count: u32) {
    let start = section_offset(bytes, index);
    let has_following_section = index + 1 < BYTECODE_SECTION_COUNT;
    let old_next = if has_following_section {
        section_offset(bytes, index + 1)
    } else {
        read_u64(bytes, HEADER_CHECKSUM_OFFSET) as usize
    };
    let new_end = start + contents.len();
    let new_next = if has_following_section {
        (new_end + 7) / 8 * 8
    } else {
        new_end
    };
    let mut replacement = contents.to_vec();
    replacement.resize(new_next - start, 0);
    bytes.splice(start..old_next, replacement);

    let delta = new_next as i64 - old_next as i64;
    write_u32(bytes, section_entry_offset(index) + 4, count);
    write_u64(
        bytes,
        section_entry_offset(index) + 16,
        contents.len() as u64,
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

fn append_section_item(bytes: &mut Vec<u8>, index: usize, item: &[u8], count: u32) {
    let start = section_offset(bytes, index);
    let length = section_length(bytes, index);
    let mut contents = bytes[start..start + length].to_vec();
    contents.extend_from_slice(item);
    replace_section_contents(bytes, index, &contents, count);
}

fn structural_program(
    register_count: u32,
    instructions: Vec<BytecodeInstruction>,
) -> BytecodeProgram {
    BytecodeProgram {
        register_count,
        constants: vec![empty_constant()],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    }
}

#[test]
fn resource_read_defines_uninitialized_destination() {
    let input = read_program(
        1,
        Vec::new(),
        vec![
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
    );

    let bytes = write_bytecode(&input).unwrap();
    let parsed = ParsedProgram::from_bytes(&bytes).unwrap();
    assert!(parsed.constants.is_empty());
}

#[test]
fn resource_read_destination_counts_as_initialized_after_instruction() {
    let input = read_program(
        2,
        vec![empty_constant()],
        vec![
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 0,
            },
            BytecodeInstruction::RuntimeUnary {
                function: 1,
                dst: 1,
                src: 0,
            },
            BytecodeInstruction::Return { src: 1 },
        ],
    );

    ParsedProgram::from_bytes(&write_bytecode(&input).unwrap()).unwrap();
}

#[test]
fn resource_read_rejects_prior_const_initializer() {
    let input = read_program(
        1,
        vec![empty_constant()],
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
    );

    let error = write_bytecode(&input).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
    assert!(
        error
            .kind_message()
            .contains("register 0 is initialized more than once")
    );
}

#[test]
fn resource_read_rejects_second_static_definition() {
    let input = read_program(
        1,
        Vec::new(),
        vec![
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
    );

    let error = write_bytecode(&input).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
    assert!(
        error
            .kind_message()
            .contains("register 0 is initialized more than once")
    );
}

#[test]
fn resource_read_can_back_a_symbol_without_const_load() {
    let name = "observed";
    let id = hash_str(name);
    let mut input = read_program(
        1,
        Vec::new(),
        vec![
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
    );
    input.symbols.insert(id, 0);
    input.dictionary.insert(id, name.to_string());

    ParsedProgram::from_bytes(&write_bytecode(&input).unwrap()).unwrap();
}

#[test]
fn dynamic_resource_read_can_supply_a_composite_child() {
    let input = read_program(
        2,
        vec![u8_tuple_constant(&[0])],
        vec![
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::CompositePack {
                dst: 1,
                template: 0,
                children: vec![0],
            },
            BytecodeInstruction::Return { src: 1 },
        ],
    );

    ParsedProgram::from_bytes(&write_bytecode(&input).unwrap()).unwrap();
}

#[test]
fn rejects_crc_valid_trailing_unused_registers() {
    let mut bytes = write_bytecode(&program(vec![empty_constant()])).unwrap();
    write_u32(&mut bytes, HEADER_REGISTER_COUNT, 2);
    refresh_crc(&mut bytes);

    assert_validation_reason(
        &bytes,
        "register count 2 does not match highest referenced register count 1",
    );
}

#[test]
fn rejects_crc_valid_unreferenced_type_rows_without_constants() {
    let mut bytes = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::F64,
        alignment: 8,
        bytes: 1.0_f64.to_bits().to_le_bytes().to_vec(),
    }]))
    .unwrap();
    replace_section_contents(
        &mut bytes,
        BytecodeSectionKind::ConstantTable as usize - 1,
        &[],
        0,
    );
    replace_section_contents(
        &mut bytes,
        BytecodeSectionKind::ConstantBlob as usize - 1,
        &[],
        0,
    );

    let error = ParsedProgram::from_bytes(&bytes).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeUnreferencedType");
    assert_eq!(
        error.kind_as::<BytecodeUnreferencedType>().unwrap().type_id,
        0
    );
}

#[test]
fn rejects_crc_valid_unused_scalar_type_row() {
    let mut bytes = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::F64,
        alignment: 8,
        bytes: 1.0_f64.to_bits().to_le_bytes().to_vec(),
    }]))
    .unwrap();
    let type_count = read_u32(
        &bytes,
        section_entry_offset(BytecodeSectionKind::Types as usize - 1) + 4,
    );
    let mut boolean_row = Vec::new();
    boolean_row.extend_from_slice(&(RuntimeTypeTag::Bool as u16).to_le_bytes());
    boolean_row.extend_from_slice(&0_u16.to_le_bytes());
    boolean_row.extend_from_slice(&0_u32.to_le_bytes());
    append_section_item(
        &mut bytes,
        BytecodeSectionKind::Types as usize - 1,
        &boolean_row,
        type_count + 1,
    );

    let error = ParsedProgram::from_bytes(&bytes).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeUnreferencedType");
    assert_eq!(
        error.kind_as::<BytecodeUnreferencedType>().unwrap().type_id,
        type_count
    );
}

#[test]
fn rejects_crc_valid_unused_constant_row() {
    let mut bytes = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::F64,
        alignment: 8,
        bytes: 1.0_f64.to_bits().to_le_bytes().to_vec(),
    }]))
    .unwrap();
    let blob_index = BytecodeSectionKind::ConstantBlob as usize - 1;
    let blob_start = section_offset(&bytes, blob_index);
    let blob_length = section_length(&bytes, blob_index);
    let mut blob = bytes[blob_start..blob_start + blob_length].to_vec();
    blob.extend_from_slice(&2.0_f64.to_bits().to_le_bytes());
    replace_section_contents(&mut bytes, blob_index, &blob, 0);

    let table_index = BytecodeSectionKind::ConstantTable as usize - 1;
    let constant_count = read_u32(&bytes, section_entry_offset(table_index) + 4);
    let mut entry = Vec::new();
    entry.extend_from_slice(&type_id_with_tag(&bytes, RuntimeTypeTag::F64).to_le_bytes());
    entry.push(1);
    entry.push(8);
    entry.extend_from_slice(&0_u16.to_le_bytes());
    entry.extend_from_slice(&(blob_length as u64).to_le_bytes());
    entry.extend_from_slice(&8_u64.to_le_bytes());
    append_section_item(&mut bytes, table_index, &entry, constant_count + 1);

    let error = ParsedProgram::from_bytes(&bytes).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeUnreferencedConstant");
    assert_eq!(
        error
            .kind_as::<BytecodeUnreferencedConstant>()
            .unwrap()
            .constant,
        1
    );
}

#[test]
fn rejects_crc_valid_unused_application_requirement_row() {
    let mut input = program(vec![empty_constant()]);
    input.requirements = vec![
        ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: "host-a".into(),
        }),
        ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: "host-b".into(),
        }),
    ];
    input.instructions.insert(
        1,
        BytecodeInstruction::HostCall {
            requirement: 0,
            dst: 0,
            arguments: Vec::new(),
        },
    );
    let bytes = write_bytecode_without_reader_validation(&input).unwrap();

    let error = ParsedProgram::from_bytes(&bytes).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeUnreferencedRequirement");
    assert_eq!(
        error
            .kind_as::<BytecodeUnreferencedRequirement>()
            .unwrap()
            .requirement,
        1
    );
}

#[test]
fn referenced_runtime_types_include_the_exact_nested_decode_closure() {
    let matrix_type = RuntimeType::Matrix {
        element: Box::new(RuntimeType::U8),
        storage: MatrixStorage::MatrixD,
        rows: 2,
        cols: 2,
    };
    let tuple_type = RuntimeType::Tuple(vec![RuntimeType::F64, matrix_type.clone()]);
    let runtime_type = RuntimeType::Map {
        key: Box::new(RuntimeType::String),
        value: Box::new(tuple_type.clone()),
    };
    let mut matrix = Vec::new();
    matrix.extend_from_slice(&2_u32.to_le_bytes());
    matrix.extend_from_slice(&2_u32.to_le_bytes());
    matrix.extend_from_slice(&[1, 2, 3, 4]);
    let mut tuple = 2_u32.to_le_bytes().to_vec();
    append_child_payload(&mut tuple, &1.0_f64.to_bits().to_le_bytes());
    append_child_payload(&mut tuple, &matrix);
    let mut payload = 1_u32.to_le_bytes().to_vec();
    append_child_payload(&mut payload, b"key");
    append_child_payload(&mut payload, &tuple);
    let bytes = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: runtime_type.clone(),
        alignment: 4,
        bytes: payload,
    }]))
    .unwrap();

    let referenced = ParsedProgram::from_bytes(&bytes)
        .unwrap()
        .referenced_runtime_types()
        .unwrap()
        .into_iter()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        referenced,
        BTreeSet::from([
            RuntimeType::String,
            RuntimeType::F64,
            RuntimeType::U8,
            matrix_type,
            tuple_type,
            runtime_type,
        ])
    );
}

#[test]
fn rejects_uninitialized_instruction_registers_and_duplicate_constant_loads() {
    fn rejects(input: BytecodeProgram, expected: &str) {
        let bytes = write_bytecode_without_reader_validation(&input).unwrap();
        assert_validation_reason(&bytes, expected);
    }

    rejects(
        structural_program(
            2,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::RuntimeUnary {
                    function: 1,
                    dst: 0,
                    src: 1,
                },
                BytecodeInstruction::Return { src: 0 },
            ],
        ),
        "instruction 1 register 1 is uninitialized",
    );

    rejects(
        structural_program(
            2,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: 1,
                    constant: 0,
                },
                BytecodeInstruction::RuntimeUnary {
                    function: 1,
                    dst: 0,
                    src: 1,
                },
                BytecodeInstruction::Return { src: 1 },
            ],
        ),
        "instruction 1 register 0 is uninitialized",
    );

    let mut host = structural_program(
        2,
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::HostCall {
                requirement: 0,
                dst: 0,
                arguments: vec![1],
            },
            BytecodeInstruction::Return { src: 0 },
        ],
    );
    host.requirements.push(ApplicationRequirement::HostFunction(
        ExecutionHostFunctionRequest {
            name: "test/host".into(),
        },
    ));
    rejects(host, "instruction 1 register 1 is uninitialized");

    let mut resource = structural_program(
        2,
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ResourceWrite {
                requirement: 0,
                dst: 0,
                src: 1,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
    );
    resource
        .requirements
        .push(ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: "test://provider".into(),
            path: "output".into(),
            context_name: "test".into(),
            operation: "write".into(),
            intent: ResourceIntent::Assign,
            delivery: ResourceDelivery::Snapshot,
        }));
    rejects(resource, "instruction 1 register 1 is uninitialized");

    rejects(
        structural_program(1, vec![BytecodeInstruction::Return { src: 0 }]),
        "instruction 0 register 0 is uninitialized",
    );

    rejects(
        structural_program(
            1,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::Return { src: 0 },
            ],
        ),
        "instruction 1 register 0 is initialized more than once",
    );
}

#[test]
fn rejects_symbols_bound_to_uninitialized_registers() {
    let name = "uninitialized";
    let id = hash_str(name);
    let mut input = structural_program(
        2,
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
    );
    input.symbols.insert(id, 1);
    input.dictionary.insert(id, name.into());

    let bytes = write_bytecode_without_reader_validation(&input).unwrap();
    assert_validation_reason(&bytes, "symbol register 1 is uninitialized");
}

#[test]
fn rejects_nonzero_constant_blob_item_count() {
    let mut bytes = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let entry = section_entry_offset(BytecodeSectionKind::ConstantBlob as usize - 1);
    write_u32(&mut bytes, entry + 4, 1);
    refresh_crc(&mut bytes);

    assert_validation_reason(&bytes, "ConstantBlob item count must be zero");
}

#[test]
fn rejects_empty_and_duplicate_record_and_table_schema_names() {
    for (runtime_type, expected) in [
        (
            RuntimeType::Record(vec![(String::new(), RuntimeType::F64)]),
            "record field name must not be empty",
        ),
        (
            RuntimeType::Record(vec![
                ("value".into(), RuntimeType::F64),
                ("value".into(), RuntimeType::F64),
            ]),
            "record field schema has duplicate name `value`",
        ),
        (
            RuntimeType::Table {
                columns: vec![(String::new(), RuntimeType::F64)],
                primary_key: 0,
            },
            "table column name must not be empty",
        ),
        (
            RuntimeType::Table {
                columns: vec![
                    ("value".into(), RuntimeType::F64),
                    ("value".into(), RuntimeType::F64),
                ],
                primary_key: 0,
            },
            "table column schema has duplicate name `value`",
        ),
    ] {
        let error = finalize_runtime_types([&runtime_type]).unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeValidation");
        assert!(error.kind_message().contains(expected));
    }
}

#[test]
fn reader_rejects_duplicate_record_and_table_schema_names_with_valid_checksum() {
    let mut record_payload = 2_u32.to_le_bytes().to_vec();
    append_child_payload(&mut record_payload, &[1]);
    append_child_payload(&mut record_payload, &2_i16.to_le_bytes());
    let mut record = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Record(vec![
            ("count".into(), RuntimeType::U8),
            ("delta".into(), RuntimeType::I16),
        ]),
        alignment: 4,
        bytes: record_payload,
    }]))
    .unwrap();
    let entry = type_entry_with_tag(&record, RuntimeTypeTag::Record);
    let payload = entry + 8;
    record[payload + 21..payload + 26].copy_from_slice(b"count");
    refresh_crc(&mut record);
    assert_validation_reason(&record, "record field schema has duplicate name `count`");

    let mut table_payload = 1_u32.to_le_bytes().to_vec();
    table_payload.extend_from_slice(&2_u32.to_le_bytes());
    append_child_payload(&mut table_payload, &[1]);
    append_child_payload(&mut table_payload, b"x");
    let mut table = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Table {
            columns: vec![
                ("left".into(), RuntimeType::U8),
                ("rght".into(), RuntimeType::String),
            ],
            primary_key: 0,
        },
        alignment: 4,
        bytes: table_payload,
    }]))
    .unwrap();
    let entry = type_entry_with_tag(&table, RuntimeTypeTag::Table);
    let payload = entry + 8;
    table[payload + 20..payload + 24].copy_from_slice(b"left");
    refresh_crc(&mut table);
    assert_validation_reason(&table, "table column schema has duplicate name `left`");
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
    assert!(matches!(decoded[0], crate::LegacyValue::Empty));
    assert!(matches!(&decoded[1], crate::LegacyValue::Bool(value) if *value.borrow()));
    assert!(
        matches!(&decoded[2], crate::LegacyValue::String(value) if value.borrow().as_str() == "bytecode-v1")
    );
    assert!(matches!(&decoded[3], crate::LegacyValue::Index(value) if *value.borrow() == 42));
    assert!(
        matches!(&decoded[4], crate::LegacyValue::F64(value) if value.borrow().to_bits() == (-0.0_f64).to_bits())
    );
}

#[cfg(feature = "matrixd")]
#[test]
fn maximum_index_scan_is_pointer_independent_for_scalars_and_matrices() {
    let scalar = u64::from(u32::MAX) + 1;
    let matrix = scalar + 7;
    let constants = vec![
        EncodedConstant {
            runtime_type: RuntimeType::Index,
            alignment: 8,
            bytes: scalar.to_le_bytes().to_vec(),
        },
        matrixd_constant(RuntimeType::Index, matrix.to_le_bytes().to_vec()),
    ];
    let parsed = ParsedProgram::from_bytes(&write_bytecode(&program(constants)).unwrap()).unwrap();

    assert_eq!(parsed.maximum_index_constant().unwrap(), Some(matrix));
}

#[test]
fn every_canonical_scalar_encoding_round_trips_exactly() {
    let f32_bits = 0x7fc0_1234_u32;
    let f64_bits = 0x7ff8_0000_0000_1234_u64;
    let c64_real_bits = (-0.0_f64).to_bits();
    let c64_imaginary_bits = 0x7ff8_0000_0000_0042_u64;
    let constants = vec![
        EncodedConstant {
            runtime_type: RuntimeType::U8,
            alignment: 1,
            bytes: vec![u8::MAX],
        },
        EncodedConstant {
            runtime_type: RuntimeType::U16,
            alignment: 2,
            bytes: u16::MAX.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::U32,
            alignment: 4,
            bytes: u32::MAX.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::U64,
            alignment: 8,
            bytes: u64::MAX.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::U128,
            alignment: 16,
            bytes: u128::MAX.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I8,
            alignment: 1,
            bytes: i8::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I16,
            alignment: 2,
            bytes: i16::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I32,
            alignment: 4,
            bytes: i32::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I64,
            alignment: 8,
            bytes: i64::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I128,
            alignment: 16,
            bytes: i128::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::F32,
            alignment: 4,
            bytes: f32_bits.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: f64_bits.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::C64,
            alignment: 8,
            bytes: [
                c64_real_bits.to_le_bytes(),
                c64_imaginary_bits.to_le_bytes(),
            ]
            .concat(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::R64,
            alignment: 8,
            bytes: [(-3_i64).to_le_bytes(), 7_i64.to_le_bytes()].concat(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::String,
            alignment: 1,
            bytes: "bytecode-v1 🦀".as_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::Bool,
            alignment: 1,
            bytes: vec![1],
        },
        EncodedConstant {
            runtime_type: RuntimeType::Id,
            alignment: 8,
            bytes: 42_u64.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::Index,
            alignment: 8,
            bytes: 7_u64.to_le_bytes().to_vec(),
        },
        empty_constant(),
    ];

    let parsed = ParsedProgram::from_bytes(&write_bytecode(&program(constants)).unwrap()).unwrap();
    let values = parsed.decode_constants().unwrap();
    assert!(matches!(&values[0], crate::LegacyValue::U8(value) if *value.borrow() == u8::MAX));
    assert!(matches!(&values[4], crate::LegacyValue::U128(value) if *value.borrow() == u128::MAX));
    assert!(matches!(&values[5], crate::LegacyValue::I8(value) if *value.borrow() == i8::MIN));
    assert!(matches!(&values[9], crate::LegacyValue::I128(value) if *value.borrow() == i128::MIN));
    assert!(
        matches!(&values[10], crate::LegacyValue::F32(value) if value.borrow().to_bits() == f32_bits)
    );
    assert!(
        matches!(&values[11], crate::LegacyValue::F64(value) if value.borrow().to_bits() == f64_bits)
    );
    assert!(matches!(&values[12], crate::LegacyValue::C64(value)
        if value.borrow().0.re.to_bits() == c64_real_bits
        && value.borrow().0.im.to_bits() == c64_imaginary_bits));
    assert!(matches!(&values[13], crate::LegacyValue::R64(value)
        if *value.borrow().numer() == -3 && *value.borrow().denom() == 7));
    assert!(
        matches!(&values[14], crate::LegacyValue::String(value) if value.borrow().as_str() == "bytecode-v1 🦀")
    );
    assert!(matches!(&values[15], crate::LegacyValue::Bool(value) if *value.borrow()));
    assert!(matches!(&values[16], crate::LegacyValue::Id(42)));
    assert!(matches!(&values[17], crate::LegacyValue::Index(value) if *value.borrow() == 7));
    assert!(matches!(&values[18], crate::LegacyValue::Empty));
}

#[test]
fn scalar_decoder_rejects_noncanonical_boolean_and_rational_bytes() {
    let invalid_boolean = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Bool,
        alignment: 1,
        bytes: vec![2],
    }]))
    .unwrap_err();
    assert!(
        invalid_boolean
            .kind_message()
            .contains("Bool constant must be exactly")
    );

    let unreduced_rational = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::R64,
        alignment: 8,
        bytes: [(2_i64).to_le_bytes(), 4_i64.to_le_bytes()].concat(),
    }]))
    .unwrap_err();
    assert!(
        unreduced_rational
            .kind_message()
            .contains("R64 constant is not reduced")
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
    expected.extend_from_slice(&(BYTECODE_SECTION_COUNT as u16).to_le_bytes());
    expected.extend_from_slice(&0_u16.to_le_bytes());
    expected.extend_from_slice(&64_u64.to_le_bytes());
    expected.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    expected.extend_from_slice(&((bytes.len() - 4) as u64).to_le_bytes());
    expected.extend_from_slice(&[0; 12]);
    assert_eq!(&bytes[..64], expected);

    let expected_counts = [1, 1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let expected_lengths = [8, 24, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
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
        #[cfg(feature = "matrix1")]
        (MatrixStorage::Matrix1, 1, 1),
        #[cfg(feature = "matrix2")]
        (MatrixStorage::Matrix2, 2, 2),
        #[cfg(feature = "matrix3")]
        (MatrixStorage::Matrix3, 3, 3),
        #[cfg(feature = "matrix4")]
        (MatrixStorage::Matrix4, 4, 4),
        #[cfg(feature = "matrix2x3")]
        (MatrixStorage::Matrix2x3, 2, 3),
        #[cfg(feature = "matrix3x2")]
        (MatrixStorage::Matrix3x2, 3, 2),
        #[cfg(feature = "row_vector2")]
        (MatrixStorage::RowVector2, 1, 2),
        #[cfg(feature = "row_vector3")]
        (MatrixStorage::RowVector3, 1, 3),
        #[cfg(feature = "row_vector4")]
        (MatrixStorage::RowVector4, 1, 4),
        #[cfg(feature = "vector2")]
        (MatrixStorage::Vector2, 2, 1),
        #[cfg(feature = "vector3")]
        (MatrixStorage::Vector3, 3, 1),
        #[cfg(feature = "vector4")]
        (MatrixStorage::Vector4, 4, 1),
        #[cfg(feature = "row_vectord")]
        (MatrixStorage::RowVectorD, 1, 5),
        #[cfg(feature = "vectord")]
        (MatrixStorage::VectorD, 5, 1),
        #[cfg(feature = "matrixd")]
        (MatrixStorage::MatrixD, 2, 5),
    ];
    let expected_len = specifications.len();
    let constants = specifications
        .into_iter()
        .map(|(storage, rows, cols)| matrix_constant(storage, rows, cols))
        .collect();
    let parsed = ParsedProgram::from_bytes(&write_bytecode(&program(constants)).unwrap()).unwrap();
    assert_eq!(parsed.constants.len(), expected_len);
    assert_eq!(parsed.decode_constants().unwrap().len(), expected_len);
}

#[test]
fn every_matrix_element_codec_round_trips_a_dynamic_matrix() {
    let constants = vec![
        matrixd_constant(RuntimeType::Index, 5_u64.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::Bool, vec![1]),
        matrixd_constant(RuntimeType::U8, vec![u8::MAX]),
        matrixd_constant(RuntimeType::U16, u16::MAX.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::U32, u32::MAX.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::U64, u64::MAX.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::U128, u128::MAX.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I8, i8::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I16, i16::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I32, i32::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I64, i64::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I128, i128::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::F32, 0x7fc0_1234_u32.to_le_bytes().to_vec()),
        matrixd_constant(
            RuntimeType::F64,
            0x7ff8_0000_0000_1234_u64.to_le_bytes().to_vec(),
        ),
        matrixd_constant(
            RuntimeType::C64,
            [
                (-0.0_f64).to_bits().to_le_bytes(),
                1.5_f64.to_bits().to_le_bytes(),
            ]
            .concat(),
        ),
        matrixd_constant(
            RuntimeType::R64,
            [(-3_i64).to_le_bytes(), 7_i64.to_le_bytes()].concat(),
        ),
        matrixd_constant(
            RuntimeType::String,
            [4_u32.to_le_bytes(), *b"mech"].concat(),
        ),
    ];
    let parsed = ParsedProgram::from_bytes(&write_bytecode(&program(constants)).unwrap()).unwrap();
    let values = parsed.decode_constants().unwrap();
    assert_eq!(values.len(), 17);
    assert!(matches!(&values[0], crate::LegacyValue::MatrixIndex(_)));
    assert!(matches!(&values[1], crate::LegacyValue::MatrixBool(_)));
    assert!(matches!(&values[2], crate::LegacyValue::MatrixU8(_)));
    assert!(matches!(&values[3], crate::LegacyValue::MatrixU16(_)));
    assert!(matches!(&values[4], crate::LegacyValue::MatrixU32(_)));
    assert!(matches!(&values[5], crate::LegacyValue::MatrixU64(_)));
    assert!(matches!(&values[6], crate::LegacyValue::MatrixU128(_)));
    assert!(matches!(&values[7], crate::LegacyValue::MatrixI8(_)));
    assert!(matches!(&values[8], crate::LegacyValue::MatrixI16(_)));
    assert!(matches!(&values[9], crate::LegacyValue::MatrixI32(_)));
    assert!(matches!(&values[10], crate::LegacyValue::MatrixI64(_)));
    assert!(matches!(&values[11], crate::LegacyValue::MatrixI128(_)));
    assert!(matches!(&values[12], crate::LegacyValue::MatrixF32(_)));
    assert!(matches!(&values[13], crate::LegacyValue::MatrixF64(_)));
    assert!(matches!(&values[14], crate::LegacyValue::MatrixC64(_)));
    assert!(matches!(&values[15], crate::LegacyValue::MatrixR64(_)));
    assert!(matches!(&values[16], crate::LegacyValue::MatrixString(_)));
}

#[test]
fn every_composite_constant_codec_round_trips() {
    let mut tuple = 2_u32.to_le_bytes().to_vec();
    append_child_payload(&mut tuple, &[7]);
    append_child_payload(&mut tuple, b"mech");

    let mut record = 2_u32.to_le_bytes().to_vec();
    append_child_payload(&mut record, &[9]);
    append_child_payload(&mut record, &(-4_i16).to_le_bytes());

    let mut map = 1_u32.to_le_bytes().to_vec();
    append_child_payload(&mut map, &[3]);
    append_child_payload(&mut map, b"value");

    let mut set = 1_u32.to_le_bytes().to_vec();
    append_child_payload(&mut set, &[4]);

    let mut table = Vec::new();
    table.extend_from_slice(&1_u32.to_le_bytes());
    table.extend_from_slice(&2_u32.to_le_bytes());
    append_child_payload(&mut table, &[5]);
    append_child_payload(&mut table, b"row");

    let mut reference = Vec::new();
    append_child_payload(&mut reference, &[6]);

    let mut present_option = vec![1];
    append_child_payload(&mut present_option, &[8]);

    let enum_name = "status";
    let variant_name = "ready";
    let enum_id = crate::hash_str(enum_name);
    let variant_id = crate::hash_str(variant_name);
    let inline_u8 = types::canonical_runtime_type_key(&RuntimeType::U8).unwrap();
    let mut enumeration = 1_u32.to_le_bytes().to_vec();
    enumeration.extend_from_slice(&variant_id.to_le_bytes());
    enumeration.extend_from_slice(&(variant_name.len() as u32).to_le_bytes());
    enumeration.extend_from_slice(variant_name.as_bytes());
    enumeration.push(1);
    append_child_payload(&mut enumeration, &inline_u8);
    append_child_payload(&mut enumeration, &[10]);

    let atom_name = "alpha";
    let atom_id = crate::hash_str(atom_name);
    let constants = vec![
        EncodedConstant {
            runtime_type: RuntimeType::Tuple(vec![RuntimeType::U8, RuntimeType::String]),
            alignment: 4,
            bytes: tuple,
        },
        EncodedConstant {
            runtime_type: RuntimeType::Record(vec![
                ("count".to_owned(), RuntimeType::U8),
                ("delta".to_owned(), RuntimeType::I16),
            ]),
            alignment: 4,
            bytes: record,
        },
        EncodedConstant {
            runtime_type: RuntimeType::Map {
                key: Box::new(RuntimeType::U8),
                value: Box::new(RuntimeType::String),
            },
            alignment: 4,
            bytes: map,
        },
        EncodedConstant {
            runtime_type: RuntimeType::Set {
                element: Box::new(RuntimeType::U8),
                max_len: Some(1),
            },
            alignment: 4,
            bytes: set,
        },
        EncodedConstant {
            runtime_type: RuntimeType::Table {
                columns: vec![
                    ("id".to_owned(), RuntimeType::U8),
                    ("name".to_owned(), RuntimeType::String),
                ],
                primary_key: 0,
            },
            alignment: 4,
            bytes: table,
        },
        EncodedConstant {
            runtime_type: RuntimeType::Reference(Box::new(RuntimeType::U8)),
            alignment: 4,
            bytes: reference,
        },
        EncodedConstant {
            runtime_type: RuntimeType::Option(Box::new(RuntimeType::U8)),
            alignment: 1,
            bytes: vec![0],
        },
        EncodedConstant {
            runtime_type: RuntimeType::Option(Box::new(RuntimeType::U8)),
            alignment: 4,
            bytes: present_option,
        },
        EncodedConstant {
            runtime_type: RuntimeType::Atom {
                id: atom_id,
                name: atom_name.to_owned(),
            },
            alignment: 1,
            bytes: Vec::new(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::Enum {
                id: enum_id,
                name: enum_name.to_owned(),
            },
            alignment: 4,
            bytes: enumeration,
        },
        EncodedConstant {
            runtime_type: RuntimeType::Kind(crate::kind::Kind::Scalar(crate::hash_str("u8"))),
            alignment: 1,
            bytes: Vec::new(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::Any,
            alignment: 1,
            bytes: Vec::new(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::None,
            alignment: 1,
            bytes: Vec::new(),
        },
    ];

    let parsed = ParsedProgram::from_bytes(&write_bytecode(&program(constants)).unwrap()).unwrap();
    let values = parsed.decode_constants().unwrap();
    assert_eq!(values.len(), 13);
    assert!(matches!(&values[0], crate::LegacyValue::Tuple(_)));
    assert!(matches!(&values[1], crate::LegacyValue::Record(_)));
    assert!(matches!(&values[2], crate::LegacyValue::Map(_)));
    assert!(matches!(&values[3], crate::LegacyValue::Set(_)));
    assert!(matches!(&values[4], crate::LegacyValue::Table(_)));
    assert!(matches!(
        &values[5],
        crate::LegacyValue::MutableReference(_)
    ));
    assert!(matches!(
        &values[6],
        crate::LegacyValue::EmptyKind(crate::ValueKind::Option(_))
    ));
    assert!(matches!(
        &values[7],
        crate::LegacyValue::Typed(_, crate::ValueKind::Option(_))
    ));
    assert!(matches!(&values[8], crate::LegacyValue::Atom(_)));
    assert!(matches!(&values[9], crate::LegacyValue::Enum(_)));
    assert!(matches!(
        &values[10],
        crate::LegacyValue::Kind(crate::ValueKind::U8)
    ));
    assert!(matches!(
        &values[11],
        crate::LegacyValue::EmptyKind(crate::ValueKind::Any)
    ));
    assert!(matches!(
        &values[12],
        crate::LegacyValue::EmptyKind(crate::ValueKind::None)
    ));
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
fn rejects_noncanonical_header_and_section_fields() {
    let original = write_bytecode(&program(vec![empty_constant()])).unwrap();

    let mut header_size = original.clone();
    write_u16(&mut header_size, 6, BYTECODE_HEADER_SIZE - 1);
    refresh_crc(&mut header_size);
    assert_validation_reason(&header_size, "header size");

    let mut flags = original.clone();
    write_u16(&mut flags, 14, 1);
    refresh_crc(&mut flags);
    assert_validation_reason(&flags, "reserved header fields");

    let mut reserved = original.clone();
    reserved[52] = 1;
    refresh_crc(&mut reserved);
    assert_validation_reason(&reserved, "reserved header fields");

    let mut section_flags = original.clone();
    write_u16(&mut section_flags, section_entry_offset(0) + 2, 1);
    refresh_crc(&mut section_flags);
    assert_validation_reason(&section_flags, "section flags");

    let mut section_reserved = original.clone();
    write_u64(&mut section_reserved, section_entry_offset(0) + 24, 1);
    refresh_crc(&mut section_reserved);
    assert_validation_reason(&section_reserved, "section flags");

    let mut unaligned = original.clone();
    let second_section_offset = section_offset(&unaligned, 1);
    write_u64(
        &mut unaligned,
        section_entry_offset(1) + 8,
        (second_section_offset + 1) as u64,
    );
    refresh_crc(&mut unaligned);
    assert_validation_reason(&unaligned, "minimal aligned offset");

    let mut padding = original;
    let instructions = BytecodeSectionKind::Instructions as usize - 1;
    let dictionary = BytecodeSectionKind::Dictionary as usize - 1;
    let instruction_end =
        section_offset(&padding, instructions) + section_length(&padding, instructions);
    assert!(instruction_end < section_offset(&padding, dictionary));
    padding[instruction_end] = 1;
    refresh_crc(&mut padding);
    assert_validation_reason(&padding, "section padding");

    let mut nonminimal_padding = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let second_section = 1;
    let second_offset = section_offset(&nonminimal_padding, second_section);
    nonminimal_padding.splice(second_offset..second_offset, [0; 8]);
    for following in second_section..BYTECODE_SECTION_COUNT {
        let entry = section_entry_offset(following);
        let offset = read_u64(&nonminimal_padding, entry + 8);
        write_u64(&mut nonminimal_padding, entry + 8, offset + 8);
    }
    let file_len = read_u64(&nonminimal_padding, HEADER_FILE_LEN);
    let checksum_offset = read_u64(&nonminimal_padding, HEADER_CHECKSUM_OFFSET);
    write_u64(&mut nonminimal_padding, HEADER_FILE_LEN, file_len + 8);
    write_u64(
        &mut nonminimal_padding,
        HEADER_CHECKSUM_OFFSET,
        checksum_offset + 8,
    );
    refresh_crc(&mut nonminimal_padding);
    assert_validation_reason(&nonminimal_padding, "minimal aligned offset");
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
    assert_validation_reason(&overlap, "minimal aligned offset");

    let mut oob = original;
    let checksum_offset = read_u64(&oob, HEADER_CHECKSUM_OFFSET);
    let last = BYTECODE_SECTION_COUNT - 1;
    let last_offset = section_offset(&oob, last) as u64;
    write_u64(
        &mut oob,
        section_entry_offset(last) + 16,
        checksum_offset - last_offset + 1,
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
    let mut variadic = write_bytecode_without_reader_validation(&input).unwrap();
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
    input.instructions.insert(
        1,
        BytecodeInstruction::HostCall {
            requirement: 0,
            dst: 0,
            arguments: Vec::new(),
        },
    );
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
fn rejects_unreferenced_dictionary_entries_in_writer_and_reader() {
    let name = "unused-name";
    let id = hash_str(name);
    let mut unused = program(vec![empty_constant()]);
    unused.dictionary.insert(id, name.to_owned());
    let error = write_bytecode(&unused).unwrap_err();
    assert!(error.kind_message().contains("not referenced by a symbol"));

    let mut canonical = program(vec![empty_constant()]);
    canonical.symbols.insert(id, 0);
    canonical.dictionary.insert(id, name.to_owned());
    let mut bytes = write_bytecode(&canonical).unwrap();
    let symbols = BytecodeSectionKind::Symbols as usize - 1;
    let symbols_length = section_length(&bytes, symbols);
    assert_eq!(symbols_length, 16);
    replace_section_contents(&mut bytes, symbols, &[], 0);

    assert_validation_reason(&bytes, "not referenced by a symbol");
}

#[test]
fn rejects_unknown_out_of_range_cyclic_and_invalid_matrix_types() {
    let mut unknown = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let entry = type_entry_offset(&unknown, 0);
    write_u16(&mut unknown, entry, u16::MAX);
    refresh_crc(&mut unknown);
    assert_validation_reason(&unknown, "unknown runtime type tag");

    let matrix = write_bytecode(&program(vec![matrix_constant(
        MatrixStorage::MatrixD,
        2,
        2,
    )]))
    .unwrap();
    let matrix_entry = type_entry_offset(&matrix, 1);

    let mut out_of_range = matrix.clone();
    write_u32(&mut out_of_range, matrix_entry + 8, 99);
    refresh_crc(&mut out_of_range);
    assert_validation_reason(&out_of_range, "out-of-range child");

    let mut cyclic = matrix.clone();
    write_u32(&mut cyclic, matrix_entry + 8, 1);
    refresh_crc(&mut cyclic);
    assert_validation_reason(&cyclic, "cyclic runtime type graph");

    let mut dimensions = matrix;
    dimensions[matrix_entry + 12] = MatrixStorage::Matrix2 as u8;
    write_u32(&mut dimensions, matrix_entry + 13, 3);
    refresh_crc(&mut dimensions);
    assert_validation_reason(&dimensions, "matrix storage and dimensions disagree");

    let mut unsupported_element = write_bytecode_without_reader_validation(&program(vec![
        matrix_constant(MatrixStorage::MatrixD, 2, 2),
        EncodedConstant {
            runtime_type: RuntimeType::Any,
            alignment: 1,
            bytes: Vec::new(),
        },
    ]))
    .unwrap();
    let matrix_entry = type_entry_with_tag(&unsupported_element, RuntimeTypeTag::Matrix);
    let any_type = type_id_with_tag(&unsupported_element, RuntimeTypeTag::Any);
    write_u32(&mut unsupported_element, matrix_entry + 8, any_type);
    refresh_crc(&mut unsupported_element);
    assert_validation_reason(
        &unsupported_element,
        "matrix element type is not supported by bytecode v1",
    );

    let mut deeply_nested = RuntimeType::U8;
    for _ in 0..300 {
        deeply_nested = RuntimeType::Reference(Box::new(deeply_nested));
    }
    let error = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: deeply_nested,
        alignment: 1,
        bytes: Vec::new(),
    }]))
    .unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
    assert!(error.kind_message().contains("recursion"));
}

#[test]
fn rejects_matrix_dimensions_that_exceed_the_feasible_payload_before_allocation() {
    let rows = 1_000_000_000_u32;
    let bytes = [rows.to_le_bytes(), 1_u32.to_le_bytes()].concat();
    let error = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Matrix {
            element: Box::new(RuntimeType::U8),
            storage: MatrixStorage::MatrixD,
            rows,
            cols: 1,
        },
        alignment: 8,
        bytes,
    }]))
    .unwrap_err();

    assert_eq!(error.kind_name(), "BytecodeValidation");
    assert!(
        error
            .kind_message()
            .contains("matrix element count exceeds the feasible remaining payload")
    );
}

#[test]
fn rejects_inline_composite_type_counts_before_allocation() {
    for (tag, label) in [
        (RuntimeTypeTag::Record, "record"),
        (RuntimeTypeTag::Table, "table"),
        (RuntimeTypeTag::Tuple, "tuple"),
    ] {
        let mut canonical = (tag as u16).to_le_bytes().to_vec();
        canonical.extend_from_slice(&u32::MAX.to_le_bytes());
        let error = types::decode_canonical_runtime_type_key(&canonical).unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeValidation");
        assert!(
            error
                .kind_message()
                .contains("exceeds the remaining payload"),
            "expected bounded {label} count error, found {}",
            error.kind_message(),
        );
    }

    let enum_name = "bounded-inline-type";
    let variant_name = "payload";
    let mut malicious_type = (RuntimeTypeTag::Record as u16).to_le_bytes().to_vec();
    malicious_type.extend_from_slice(&u32::MAX.to_le_bytes());
    let mut enumeration = 1_u32.to_le_bytes().to_vec();
    enumeration.extend_from_slice(&hash_str(variant_name).to_le_bytes());
    enumeration.extend_from_slice(&(variant_name.len() as u32).to_le_bytes());
    enumeration.extend_from_slice(variant_name.as_bytes());
    enumeration.push(1);
    append_child_payload(&mut enumeration, &malicious_type);
    append_child_payload(&mut enumeration, &[]);
    let error = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Enum {
            id: hash_str(enum_name),
            name: enum_name.to_owned(),
        },
        alignment: 4,
        bytes: enumeration,
    }]))
    .unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
    assert!(
        error
            .kind_message()
            .contains("canonical record field count exceeds the remaining payload")
    );
}

#[test]
fn rejects_invalid_constant_table_entries_and_scalar_payloads() {
    let boolean = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Bool,
        alignment: 1,
        bytes: vec![1],
    }]))
    .unwrap();
    let entry = constant_entry_offset(&boolean, 0);

    let mut encoding = boolean.clone();
    encoding[entry + 4] = 2;
    refresh_crc(&mut encoding);
    assert_validation_reason(&encoding, "invalid constant table entry");

    let mut flags = boolean.clone();
    write_u16(&mut flags, entry + 6, 1);
    refresh_crc(&mut flags);
    assert_validation_reason(&flags, "invalid constant table entry");

    let mut alignment = boolean.clone();
    alignment[entry + 5] = 3;
    refresh_crc(&mut alignment);
    assert_validation_reason(&alignment, "invalid constant table entry");

    let mut payload = boolean;
    let payload_offset = constant_payload_offset(&payload, 0);
    payload[payload_offset] = 2;
    refresh_crc(&mut payload);
    assert_decode_reason(&payload, "Bool constant must be exactly");

    let mut rational = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::R64,
        alignment: 8,
        bytes: [(-3_i64).to_le_bytes(), 7_i64.to_le_bytes()].concat(),
    }]))
    .unwrap();
    let payload_offset = constant_payload_offset(&rational, 0);
    rational[payload_offset..payload_offset + 8].copy_from_slice(&2_i64.to_le_bytes());
    rational[payload_offset + 8..payload_offset + 16].copy_from_slice(&4_i64.to_le_bytes());
    refresh_crc(&mut rational);
    assert_decode_reason(&rational, "R64 constant is not reduced");
}

#[test]
fn rejects_duplicate_map_and_set_payloads_and_invalid_enum_identity() {
    let mut map_payload = 2_u32.to_le_bytes().to_vec();
    for value in [1_u8, 10, 2, 20] {
        append_child_payload(&mut map_payload, &[value]);
    }
    let mut map = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Map {
            key: Box::new(RuntimeType::U8),
            value: Box::new(RuntimeType::U8),
        },
        alignment: 4,
        bytes: map_payload,
    }]))
    .unwrap();
    let map_offset = constant_payload_offset(&map, 0);
    map[map_offset + 18] = 1;
    refresh_crc(&mut map);
    assert_decode_reason(&map, "map keys are not in strict canonical payload order");

    let mut set_payload = 2_u32.to_le_bytes().to_vec();
    append_child_payload(&mut set_payload, &[1]);
    append_child_payload(&mut set_payload, &[2]);
    let mut set = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Set {
            element: Box::new(RuntimeType::U8),
            max_len: None,
        },
        alignment: 4,
        bytes: set_payload,
    }]))
    .unwrap();
    let set_offset = constant_payload_offset(&set, 0);
    set[set_offset + 13] = 1;
    refresh_crc(&mut set);
    assert_decode_reason(
        &set,
        "set elements are not in strict canonical payload order",
    );

    let enum_name = "status";
    let variant_name = "ready";
    let mut enumeration = 1_u32.to_le_bytes().to_vec();
    enumeration.extend_from_slice(&hash_str(variant_name).to_le_bytes());
    enumeration.extend_from_slice(&(variant_name.len() as u32).to_le_bytes());
    enumeration.extend_from_slice(variant_name.as_bytes());
    enumeration.push(0);
    let mut enumeration = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Enum {
            id: hash_str(enum_name),
            name: enum_name.to_owned(),
        },
        alignment: 4,
        bytes: enumeration,
    }]))
    .unwrap();
    let enum_offset = constant_payload_offset(&enumeration, 0);
    enumeration[enum_offset + 20] ^= 1;
    refresh_crc(&mut enumeration);
    assert_decode_reason(
        &enumeration,
        "enum variant name does not match its stable ID",
    );

    let mut empty_variant = 1_u32.to_le_bytes().to_vec();
    empty_variant.extend_from_slice(&hash_str("").to_le_bytes());
    empty_variant.extend_from_slice(&0_u32.to_le_bytes());
    empty_variant.push(0);
    let constant_entry = constant_entry_offset(&enumeration, 0);
    write_u64(
        &mut enumeration,
        constant_entry + 16,
        empty_variant.len() as u64,
    );
    replace_section_contents(
        &mut enumeration,
        BytecodeSectionKind::ConstantBlob as usize - 1,
        &empty_variant,
        0,
    );
    assert_validation_reason(&enumeration, "enum variant name must not be empty");
}

#[test]
fn rejects_invalid_and_duplicate_symbols() {
    let first_name = "alpha";
    let second_name = "beta";
    let first = hash_str(first_name);
    let second = hash_str(second_name);
    let mut input = program(vec![empty_constant()]);
    input.register_count = 2;
    input.symbols.insert(first, 0);
    input.symbols.insert(second, 1);
    input.dictionary.insert(first, first_name.to_owned());
    input.dictionary.insert(second, second_name.to_owned());
    input.instructions.insert(
        1,
        BytecodeInstruction::ConstLoad {
            dst: 1,
            constant: 0,
        },
    );
    let symbols = write_bytecode(&input).unwrap();
    let symbol_offset = section_offset(&symbols, BytecodeSectionKind::Symbols as usize - 1);

    let mut register = symbols.clone();
    write_u32(&mut register, symbol_offset + 8, 2);
    refresh_crc(&mut register);
    assert_validation_reason(&register, "symbol register is out of range");

    let mut duplicate = symbols;
    let first_id = read_u64(&duplicate, symbol_offset);
    write_u64(&mut duplicate, symbol_offset + 16, first_id);
    refresh_crc(&mut duplicate);
    assert_validation_reason(&duplicate, "symbols are duplicate or unsorted");
}

#[test]
fn rejects_unknown_requirement_fields_utf8_opcode_and_trailing_bytes() {
    let requirement = ApplicationRequirement::Resource(ExecutionResourceRequest {
        base_uri: "test://clock".to_owned(),
        path: "value".to_owned(),
        context_name: "clock".to_owned(),
        operation: "read".to_owned(),
        intent: ResourceIntent::Read,
        delivery: ResourceDelivery::Snapshot,
    });
    let mut input = BytecodeProgram {
        register_count: 1,
        constants: Vec::new(),
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: vec![requirement],
    };
    let requirement = write_bytecode(&input).unwrap();
    let requirement_offset = section_offset(
        &requirement,
        BytecodeSectionKind::ApplicationRequirements as usize - 1,
    );

    for (byte_offset, value, expected) in [
        (0, 3, "unknown application requirement kind"),
        (1, 0, "unknown resource intent"),
        (2, 2, "unknown resource delivery"),
        (3, 1, "requirement flags must be zero"),
    ] {
        let mut malformed = requirement.clone();
        malformed[requirement_offset + byte_offset] = value;
        refresh_crc(&mut malformed);
        assert_validation_reason(&malformed, expected);
    }

    let mut utf8 = requirement;
    utf8[requirement_offset + 16] = 0xff;
    refresh_crc(&mut utf8);
    assert_validation_reason(&utf8, "invalid UTF-8 in requirement operation");

    let mut mismatched_operation = write_bytecode(&input).unwrap();
    let requirement_offset = section_offset(
        &mismatched_operation,
        BytecodeSectionKind::ApplicationRequirements as usize - 1,
    );
    mismatched_operation[requirement_offset + 16..requirement_offset + 20].copy_from_slice(b"writ");
    refresh_crc(&mut mismatched_operation);
    assert_validation_reason(
        &mismatched_operation,
        "read intent requires the canonical `read` operation",
    );

    let mut opcode = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let instruction_offset =
        section_offset(&opcode, BytecodeSectionKind::Instructions as usize - 1);
    opcode[instruction_offset] = 0xfe;
    refresh_crc(&mut opcode);
    assert_validation_reason(&opcode, "unknown bytecode opcode");

    let mut trailing = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let checksum_offset = read_u64(&trailing, HEADER_CHECKSUM_OFFSET) as usize;
    trailing.insert(checksum_offset, 1);
    let trailing_len = trailing.len() as u64;
    write_u64(&mut trailing, HEADER_FILE_LEN, trailing_len);
    write_u64(
        &mut trailing,
        HEADER_CHECKSUM_OFFSET,
        (checksum_offset + 1) as u64,
    );
    refresh_crc(&mut trailing);
    assert_validation_reason(&trailing, "checksum does not immediately follow");
}

#[test]
fn composite_pack_round_trips_and_reconstructs_from_child_registers() {
    let input = BytecodeProgram {
        register_count: 3,
        constants: vec![u8_constant(7), u8_constant(9), u8_tuple_constant(&[0, 0])],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 1,
            },
            BytecodeInstruction::CompositePack {
                dst: 2,
                template: 2,
                children: vec![0, 1],
            },
            BytecodeInstruction::Return { src: 2 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    };
    let parsed = ParsedProgram::from_bytes(&write_bytecode(&input).unwrap()).unwrap();
    assert_eq!(parsed.instructions, input.instructions);
    parsed
        .validate_runtime_contracts(&crate::FunctionCatalog::empty())
        .unwrap();

    let constants = parsed.decode_constants().unwrap();
    let rebuilt = rebuild_bytecode_composite(
        &constants[2],
        vec![constants[0].clone(), constants[1].clone()],
    )
    .unwrap();
    let crate::LegacyValue::Tuple(tuple) = rebuilt else {
        panic!("CompositePack must reconstruct the tuple template");
    };
    assert_eq!(
        tuple.borrow().elements,
        vec![
            Box::new(crate::LegacyValue::U8(crate::Ref::new(7))),
            Box::new(crate::LegacyValue::U8(crate::Ref::new(9))),
        ],
    );
}

#[test]
fn composite_pack_schema_accepts_compiler_unwrapped_reference_children() {
    let input = BytecodeProgram {
        register_count: 2,
        constants: vec![u8_constant(7), u8_reference_tuple_constant(0)],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::CompositePack {
                dst: 1,
                template: 1,
                children: vec![0],
            },
            BytecodeInstruction::Return { src: 1 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    };

    ParsedProgram::from_bytes(&write_bytecode(&input).unwrap()).unwrap();
}

#[test]
fn composite_pack_schema_accepts_nested_compiler_unwrapped_reference_children() {
    let mut actual_record = 1_u32.to_le_bytes().to_vec();
    append_child_payload(&mut actual_record, &7_f64.to_bits().to_le_bytes());
    let actual = EncodedConstant {
        runtime_type: RuntimeType::Record(vec![("rotation".into(), RuntimeType::F64)]),
        alignment: 8,
        bytes: actual_record,
    };

    let mut reference = Vec::new();
    append_child_payload(&mut reference, &7_f64.to_bits().to_le_bytes());
    let mut template_record = 1_u32.to_le_bytes().to_vec();
    append_child_payload(&mut template_record, &reference);
    let mut template_tuple = 1_u32.to_le_bytes().to_vec();
    append_child_payload(&mut template_tuple, &template_record);
    let template = EncodedConstant {
        runtime_type: RuntimeType::Tuple(vec![RuntimeType::Record(vec![(
            "rotation".into(),
            RuntimeType::Reference(Box::new(RuntimeType::F64)),
        )])]),
        alignment: 8,
        bytes: template_tuple,
    };

    let input = BytecodeProgram {
        register_count: 2,
        constants: vec![actual, template],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::CompositePack {
                dst: 1,
                template: 1,
                children: vec![0],
            },
            BytecodeInstruction::Return { src: 1 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    };

    ParsedProgram::from_bytes(&write_bytecode(&input).unwrap()).unwrap();
}

#[test]
fn rejects_duplicate_canonical_constants_even_when_each_id_is_reachable() {
    let bytes =
        write_bytecode_without_reader_validation(&program(vec![u8_constant(7), u8_constant(7)]))
            .unwrap();

    assert_validation_reason(
        &bytes,
        "duplicate canonical runtime type and payload entries",
    );
}

#[test]
fn rejects_constant_ids_outside_canonical_first_reference_order() {
    let input = BytecodeProgram {
        register_count: 2,
        constants: vec![u8_constant(7), u8_constant(9)],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 1,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    };
    let bytes = write_bytecode_without_reader_validation(&input).unwrap();

    assert_validation_reason(&bytes, "expected first-reference ID 0");
}

#[test]
fn composite_pack_rejects_invalid_templates_arities_and_register_flow() {
    let cases = [
        (
            BytecodeProgram {
                register_count: 2,
                constants: vec![u8_tuple_constant(&[0]), string_constant("wrong-kind")],
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions: vec![
                    BytecodeInstruction::ConstLoad {
                        dst: 0,
                        constant: 1,
                    },
                    BytecodeInstruction::CompositePack {
                        dst: 1,
                        template: 0,
                        children: vec![0],
                    },
                    BytecodeInstruction::Return { src: 1 },
                ],
                dictionary: BTreeMap::new(),
                requirements: Vec::new(),
            },
            "expected U8 from the template schema",
        ),
        (
            BytecodeProgram {
                register_count: 2,
                constants: vec![empty_constant()],
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions: vec![
                    BytecodeInstruction::ConstLoad {
                        dst: 0,
                        constant: 0,
                    },
                    BytecodeInstruction::CompositePack {
                        dst: 1,
                        template: 0,
                        children: vec![0],
                    },
                    BytecodeInstruction::Return { src: 1 },
                ],
                dictionary: BTreeMap::new(),
                requirements: Vec::new(),
            },
            "not structurally lowerable",
        ),
        (
            BytecodeProgram {
                register_count: 2,
                constants: vec![u8_tuple_constant(&[0]), u8_constant(1)],
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions: vec![
                    BytecodeInstruction::ConstLoad {
                        dst: 0,
                        constant: 1,
                    },
                    BytecodeInstruction::CompositePack {
                        dst: 1,
                        template: 0,
                        children: vec![],
                    },
                    BytecodeInstruction::Return { src: 1 },
                ],
                dictionary: BTreeMap::new(),
                requirements: Vec::new(),
            },
            "expects 1 children",
        ),
        (
            BytecodeProgram {
                register_count: 2,
                constants: vec![u8_tuple_constant(&[0])],
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions: vec![
                    BytecodeInstruction::CompositePack {
                        dst: 1,
                        template: 0,
                        children: vec![0],
                    },
                    BytecodeInstruction::Return { src: 1 },
                ],
                dictionary: BTreeMap::new(),
                requirements: Vec::new(),
            },
            "register 0 is uninitialized",
        ),
        (
            BytecodeProgram {
                register_count: 1,
                constants: vec![u8_tuple_constant(&[0])],
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions: vec![
                    BytecodeInstruction::ConstLoad {
                        dst: 0,
                        constant: 0,
                    },
                    BytecodeInstruction::CompositePack {
                        dst: 0,
                        template: 0,
                        children: vec![0],
                    },
                    BytecodeInstruction::Return { src: 0 },
                ],
                dictionary: BTreeMap::new(),
                requirements: Vec::new(),
            },
            "initialized more than once",
        ),
    ];

    for (program, reason) in cases {
        let error = write_bytecode(&program).unwrap_err();
        assert!(
            error.kind_message().contains(reason),
            "expected `{reason}` in `{}`",
            error.kind_message(),
        );
    }
}

#[test]
fn writer_rejects_resource_intent_operation_and_delivery_mismatches() {
    let cases = [
        (
            ResourceIntent::Read,
            "write",
            ResourceDelivery::Snapshot,
            "read intent requires",
        ),
        (
            ResourceIntent::Assign,
            "send",
            ResourceDelivery::Snapshot,
            "assign intent requires",
        ),
        (
            ResourceIntent::Assign,
            "write",
            ResourceDelivery::Live,
            "assign intent cannot request live delivery",
        ),
        (
            ResourceIntent::Send,
            "read",
            ResourceDelivery::Snapshot,
            "send intent cannot use the reserved `read` operation",
        ),
        (
            ResourceIntent::Send,
            "write",
            ResourceDelivery::Live,
            "send intent cannot request live delivery",
        ),
    ];
    for (intent, operation, delivery, expected) in cases {
        let mut input = resource_program("test://clock", "value");
        let ApplicationRequirement::Resource(request) = &mut input.requirements[0] else {
            unreachable!()
        };
        request.intent = intent;
        request.operation = operation.to_owned();
        request.delivery = delivery;
        input.instructions[1] = match intent {
            ResourceIntent::Read => BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            ResourceIntent::Assign => BytecodeInstruction::ResourceWrite {
                requirement: 0,
                dst: 0,
                src: 0,
            },
            ResourceIntent::Send => BytecodeInstruction::ResourceSend {
                requirement: 0,
                dst: 0,
                src: 0,
            },
        };
        let error = write_bytecode(&input).unwrap_err();
        assert!(error.kind_message().contains(expected));
    }
}

#[test]
fn rejects_nonminimal_constant_blob_layout() {
    let mut bytes = write_bytecode(&program(vec![
        EncodedConstant {
            runtime_type: RuntimeType::U8,
            alignment: 1,
            bytes: vec![1],
        },
        EncodedConstant {
            runtime_type: RuntimeType::U8,
            alignment: 1,
            bytes: vec![2],
        },
    ]))
    .unwrap();
    let blob_index = BytecodeSectionKind::ConstantBlob as usize - 1;
    replace_section_contents(&mut bytes, blob_index, &[1, 0, 2], 0);
    let second_entry = constant_entry_offset(&bytes, 1);
    write_u64(&mut bytes, second_entry + 8, 2);
    refresh_crc(&mut bytes);
    assert_validation_reason(&bytes, "constant offset is not the minimal aligned offset");

    let mut trailing = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::U8,
        alignment: 1,
        bytes: vec![1],
    }]))
    .unwrap();
    replace_section_contents(&mut trailing, blob_index, &[1, 0], 0);
    assert_validation_reason(&trailing, "noncanonical trailing bytes");
}

#[test]
fn rejects_exponentially_expanding_runtime_type_dag_before_materialization() {
    let mut bytes = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::U8,
        alignment: 1,
        bytes: vec![1],
    }]))
    .unwrap();
    let mut types = Vec::new();
    types.extend_from_slice(&(RuntimeTypeTag::U8 as u16).to_le_bytes());
    types.extend_from_slice(&0_u16.to_le_bytes());
    types.extend_from_slice(&0_u32.to_le_bytes());
    for child in 0..24_u32 {
        types.extend_from_slice(&(RuntimeTypeTag::Tuple as u16).to_le_bytes());
        types.extend_from_slice(&0_u16.to_le_bytes());
        types.extend_from_slice(&12_u32.to_le_bytes());
        types.extend_from_slice(&2_u32.to_le_bytes());
        types.extend_from_slice(&child.to_le_bytes());
        types.extend_from_slice(&child.to_le_bytes());
    }
    replace_section_contents(
        &mut bytes,
        BytecodeSectionKind::Types as usize - 1,
        &types,
        25,
    );
    let constant_entry = constant_entry_offset(&bytes, 0);
    write_u32(&mut bytes, constant_entry, 24);
    refresh_crc(&mut bytes);
    assert_validation_reason(
        &bytes,
        "expanded runtime type graph exceeds bytecode v1 node limit",
    );
}

#[test]
fn rejects_invalid_register_constant_and_requirement_indexes() {
    let original = write_bytecode(&program(vec![empty_constant()])).unwrap();
    let instruction_offset =
        section_offset(&original, BytecodeSectionKind::Instructions as usize - 1);

    let mut register = original.clone();
    write_u32(&mut register, instruction_offset + 1, 1);
    refresh_crc(&mut register);
    assert_validation_reason(&register, "register 1 is out of range");

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
    let mut requirement = write_bytecode_without_reader_validation(&input).unwrap();
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
fn every_read_limit_is_enforced_with_a_structured_validation_error() {
    let bytes = write_bytecode(&program(vec![empty_constant()])).unwrap();

    let mut limits = BytecodeReadLimits::default();
    limits.max_file_bytes = bytes.len() - 1;
    assert_validation_with_limits(&bytes, limits);

    let mut limits = BytecodeReadLimits::default();
    limits.max_registers = 0;
    assert_validation_with_limits(&bytes, limits);

    let mut limits = BytecodeReadLimits::default();
    limits.max_instructions = 0;
    assert_validation_with_limits(&bytes, limits);

    let mut limits = BytecodeReadLimits::default();
    limits.max_types = 0;
    assert_validation_with_limits(&bytes, limits);

    let mut limits = BytecodeReadLimits::default();
    limits.max_constants = 0;
    assert_validation_with_limits(&bytes, limits);

    let mut symbols = program(vec![empty_constant()]);
    let symbol_name = "answer";
    let symbol_id = hash_str(symbol_name);
    symbols.symbols.insert(symbol_id, 0);
    symbols.dictionary.insert(symbol_id, symbol_name.to_owned());
    let symbol_bytes = write_bytecode(&symbols).unwrap();

    let mut limits = BytecodeReadLimits::default();
    limits.max_symbols = 0;
    assert_validation_with_limits(&symbol_bytes, limits);

    let mut limits = BytecodeReadLimits::default();
    limits.max_dictionary_entries = 0;
    assert_validation_with_limits(&symbol_bytes, limits);

    let mut limits = BytecodeReadLimits::default();
    limits.max_dictionary_bytes = 0;
    assert_validation_with_limits(&symbol_bytes, limits);

    let mut requirements = program(vec![empty_constant()]);
    requirements
        .requirements
        .push(ApplicationRequirement::HostFunction(
            ExecutionHostFunctionRequest {
                name: "host".into(),
            },
        ));
    requirements.instructions.insert(
        1,
        BytecodeInstruction::HostCall {
            requirement: 0,
            dst: 0,
            arguments: Vec::new(),
        },
    );
    let requirement_bytes = write_bytecode(&requirements).unwrap();

    let mut limits = BytecodeReadLimits::default();
    limits.max_requirements = 0;
    assert_validation_with_limits(&requirement_bytes, limits);

    let mut input = program(vec![empty_constant()]);
    input.instructions = vec![
        BytecodeInstruction::RuntimeVariadic {
            function: 1,
            dst: 0,
            arguments: vec![0],
        },
        BytecodeInstruction::Return { src: 0 },
    ];
    let variadic = write_bytecode_without_reader_validation(&input).unwrap();
    let mut limits = BytecodeReadLimits::default();
    limits.max_variadic_arguments = 0;
    assert_validation_with_limits(&variadic, limits);
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
            delivery: ResourceDelivery::Snapshot,
        }),
        ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: "host".into(),
        }),
    ];
    requirements.sort_by(compare_application_requirements);
    let mut input = program(vec![empty_constant()]);
    input.register_count = 2;
    input.requirements = requirements.clone();
    input.instructions = vec![BytecodeInstruction::ConstLoad {
        dst: 0,
        constant: 0,
    }];
    for (requirement, entry) in requirements.iter().enumerate() {
        let requirement = requirement as u32;
        input.instructions.push(match entry {
            ApplicationRequirement::HostFunction(_) => BytecodeInstruction::HostCall {
                requirement,
                dst: 0,
                arguments: Vec::new(),
            },
            ApplicationRequirement::Resource(request) => match request.intent {
                ResourceIntent::Read => BytecodeInstruction::ResourceRead {
                    requirement,
                    dst: 1,
                },
                ResourceIntent::Assign => BytecodeInstruction::ResourceWrite {
                    requirement,
                    dst: 0,
                    src: 0,
                },
                ResourceIntent::Send => BytecodeInstruction::ResourceSend {
                    requirement,
                    dst: 0,
                    src: 0,
                },
            },
        });
    }
    input
        .instructions
        .push(BytecodeInstruction::Return { src: 0 });
    let parsed = ParsedProgram::from_bytes(&write_bytecode(&input).unwrap()).unwrap();
    assert_eq!(parsed.requirements, requirements);
}

fn resource_program(base_uri: &str, path: &str) -> BytecodeProgram {
    BytecodeProgram {
        register_count: 1,
        constants: Vec::new(),
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: vec![ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: base_uri.into(),
            path: path.into(),
            context_name: "ctx".into(),
            operation: "read".into(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Snapshot,
        })],
    }
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

    for base_uri in [" docs://manual", "docs://manual "] {
        let error = write_bytecode(&resource_program(base_uri, "chapter/one")).unwrap_err();
        assert!(
            error
                .kind_message()
                .contains("base URI must not have surrounding whitespace")
        );
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

    for (canonical, noncanonical) in [
        ("xdocs://manual", " docs://manual"),
        ("docs://manualx", "docs://manual "),
    ] {
        let mut bytes = write_bytecode(&resource_program(canonical, "chapter/one")).unwrap();
        let (base_uri, _) = resource_requirement_field_offsets(&bytes);
        bytes[base_uri..base_uri + noncanonical.len()].copy_from_slice(noncanonical.as_bytes());
        refresh_crc(&mut bytes);
        assert_validation_reason(&bytes, "base URI must not have surrounding whitespace");
    }

    for (canonical, noncanonical) in [("a/x", "a/."), ("a/xx", "a/.."), ("a/xb", "a//b")] {
        let mut bytes = write_bytecode(&resource_program("docs://manual", canonical)).unwrap();
        let (_, path) = resource_requirement_field_offsets(&bytes);
        bytes[path..path + noncanonical.len()].copy_from_slice(noncanonical.as_bytes());
        refresh_crc(&mut bytes);
        assert_validation_reason(&bytes, "path must not");
    }
}

#[test]
fn rejects_crc_valid_unused_named_runtime_types_with_mismatched_ids() {
    let cases = [
        (
            RuntimeTypeTag::Atom,
            "unused-atom",
            "runtime atom",
            EncodedConstant {
                runtime_type: RuntimeType::Atom {
                    id: hash_str("unused-atom"),
                    name: "unused-atom".to_owned(),
                },
                alignment: 1,
                bytes: Vec::new(),
            },
            empty_constant(),
            RuntimeTypeTag::Empty,
        ),
        (
            RuntimeTypeTag::Enum,
            "unused-enum",
            "runtime enum",
            EncodedConstant {
                runtime_type: RuntimeType::Enum {
                    id: hash_str("unused-enum"),
                    name: "unused-enum".to_owned(),
                },
                alignment: 4,
                bytes: 0_u32.to_le_bytes().to_vec(),
            },
            EncodedConstant {
                runtime_type: RuntimeType::U32,
                alignment: 4,
                bytes: 0_u32.to_le_bytes().to_vec(),
            },
            RuntimeTypeTag::U32,
        ),
    ];

    for (tag, name, category, named, replacement, replacement_tag) in cases {
        let expected = hash_str(name);
        let supplied = expected ^ 1;
        let mut bytes =
            write_bytecode_without_reader_validation(&program(vec![named, replacement])).unwrap();
        let named_entry = type_entry_with_tag(&bytes, tag);
        write_u64(&mut bytes, named_entry + 8, supplied);
        let replacement_type = type_id_with_tag(&bytes, replacement_tag);
        let constant_entry = constant_entry_offset(&bytes, 0);
        write_u32(&mut bytes, constant_entry, replacement_type);
        refresh_crc(&mut bytes);

        assert_named_id_validation(&bytes, category, supplied, expected, name);
    }
}

#[test]
fn rejects_crc_valid_named_ids_nested_in_semantic_kinds() {
    for (name, category, nested) in [
        (
            "nested-atom",
            "kind atom",
            crate::kind::Kind::Option(Box::new(crate::kind::Kind::Atom(
                hash_str("nested-atom"),
                "nested-atom".to_owned(),
            ))),
        ),
        (
            "nested-enum",
            "kind enum",
            crate::kind::Kind::Option(Box::new(crate::kind::Kind::Enum(
                hash_str("nested-enum"),
                "nested-enum".to_owned(),
            ))),
        ),
    ] {
        let expected = hash_str(name);
        let supplied = expected ^ 1;
        let mut bytes = write_bytecode(&program(vec![EncodedConstant {
            runtime_type: RuntimeType::Kind(nested),
            alignment: 1,
            bytes: Vec::new(),
        }]))
        .unwrap();
        let kind_entry = type_entry_with_tag(&bytes, RuntimeTypeTag::Kind);
        // RuntimeType::Kind payload: Option tag, Atom/Enum tag, then the named ID.
        write_u64(&mut bytes, kind_entry + 10, supplied);
        refresh_crc(&mut bytes);

        assert_named_id_validation(&bytes, category, supplied, expected, name);
    }
}

#[test]
fn rejects_table_primary_keys_that_the_runtime_cannot_represent() {
    let empty_table = EncodedConstant {
        runtime_type: RuntimeType::Table {
            columns: Vec::new(),
            primary_key: 0,
        },
        alignment: 4,
        bytes: [0_u32.to_le_bytes(), 0_u32.to_le_bytes()].concat(),
    };
    let mut runtime_table = write_bytecode(&program(vec![empty_table])).unwrap();
    let table_entry = type_entry_with_tag(&runtime_table, RuntimeTypeTag::Table);
    write_u32(&mut runtime_table, table_entry + 12, 1);
    refresh_crc(&mut runtime_table);
    assert_validation_reason(
        &runtime_table,
        "primary keys other than zero are unsupported",
    );

    let empty_kind_table = EncodedConstant {
        runtime_type: RuntimeType::Kind(crate::kind::Kind::Table(Vec::new(), 0)),
        alignment: 1,
        bytes: Vec::new(),
    };
    let mut kind_table = write_bytecode(&program(vec![empty_kind_table])).unwrap();
    let kind_entry = type_entry_with_tag(&kind_table, RuntimeTypeTag::Kind);
    // RuntimeType::Kind payload: table tag, zero column count, primary key.
    write_u32(&mut kind_table, kind_entry + 13, 1);
    refresh_crc(&mut kind_table);
    assert_validation_reason(&kind_table, "primary keys other than zero are unsupported");

    let mut canonical = types::canonical_runtime_type_key(&RuntimeType::Table {
        columns: Vec::new(),
        primary_key: 0,
    })
    .unwrap();
    let primary_key = canonical.len() - 4;
    write_u32(&mut canonical, primary_key, 1);
    let error = types::decode_canonical_runtime_type_key(&canonical).unwrap_err();
    assert!(
        error
            .kind_message()
            .contains("canonical table primary keys other than zero are unsupported")
    );

    let two_columns = EncodedConstant {
        runtime_type: RuntimeType::Table {
            columns: vec![
                ("first".to_owned(), RuntimeType::U8),
                ("second".to_owned(), RuntimeType::U8),
            ],
            primary_key: 0,
        },
        alignment: 4,
        bytes: [0_u32.to_le_bytes(), 2_u32.to_le_bytes()].concat(),
    };
    let mut multi_column = write_bytecode(&program(vec![two_columns])).unwrap();
    let table_entry = type_entry_with_tag(&multi_column, RuntimeTypeTag::Table);
    let payload_len = read_u32(&multi_column, table_entry + 4) as usize;
    write_u32(&mut multi_column, table_entry + 8 + payload_len - 4, 1);
    refresh_crc(&mut multi_column);
    assert_validation_reason(
        &multi_column,
        "table primary keys other than zero are unsupported",
    );
}

#[test]
fn rejects_table_row_counts_before_allocation_or_unbounded_iteration() {
    for columns in [Vec::new(), vec![("value".to_owned(), RuntimeType::U8)]] {
        let column_count = columns.len() as u32;
        let constant = EncodedConstant {
            runtime_type: RuntimeType::Table {
                columns,
                primary_key: 0,
            },
            alignment: 4,
            bytes: [0_u32.to_le_bytes(), column_count.to_le_bytes()].concat(),
        };
        let mut bytes = write_bytecode(&program(vec![constant])).unwrap();
        let payload = constant_payload_offset(&bytes, 0);
        write_u32(&mut bytes, payload, u32::MAX);
        refresh_crc(&mut bytes);
        assert_validation_reason(&bytes, "table row count exceeds bytecode v1 limit");
    }

    let constant = EncodedConstant {
        runtime_type: RuntimeType::Table {
            columns: vec![("value".to_owned(), RuntimeType::U8)],
            primary_key: 0,
        },
        alignment: 4,
        bytes: [0_u32.to_le_bytes(), 1_u32.to_le_bytes()].concat(),
    };
    let mut bytes = write_bytecode(&program(vec![constant])).unwrap();
    let payload = constant_payload_offset(&bytes, 0);
    write_u32(&mut bytes, payload, 1);
    refresh_crc(&mut bytes);
    assert_validation_reason(&bytes, "feasible framed cell payload");
}

#[test]
fn rejects_noncanonical_scalar_ids_nested_in_semantic_kinds() {
    let mut bytes = write_bytecode(&program(vec![EncodedConstant {
        runtime_type: RuntimeType::Kind(crate::kind::Kind::Scalar(hash_str("u8"))),
        alignment: 1,
        bytes: Vec::new(),
    }]))
    .unwrap();
    let kind_entry = type_entry_with_tag(&bytes, RuntimeTypeTag::Kind);
    // RuntimeType::Kind payload: scalar tag followed by the scalar ID.
    write_u64(&mut bytes, kind_entry + 9, hash_str("not-a-runtime-scalar"));
    refresh_crc(&mut bytes);

    assert_validation_reason(
        &bytes,
        "Kind scalar ID does not identify a canonical runtime scalar",
    );
}

#[cfg(feature = "compiler")]
mod compiler_tests {
    #[cfg(all(feature = "table", feature = "vectord", feature = "u8"))]
    use std::collections::HashMap;

    #[cfg(all(feature = "table", feature = "vectord", feature = "u8"))]
    use indexmap::IndexMap;
    #[cfg(all(
        feature = "f64",
        feature = "i64",
        feature = "matrix2",
        feature = "matrix3"
    ))]
    use nalgebra as na;
    #[cfg(all(feature = "table", feature = "vectord", feature = "u8"))]
    use nalgebra::DVector;

    use crate::program::compiler::{BytecodeCompilerContext, CompileConst, Register};
    use crate::{LegacyValue, MResult, Ref, ValueKind};

    #[cfg(any(
        all(feature = "table", feature = "vectord", feature = "u8"),
        all(
            feature = "f64",
            feature = "i64",
            feature = "matrix2",
            feature = "matrix3"
        )
    ))]
    use crate::matrix::Matrix;

    use super::*;

    #[derive(Default)]
    struct ConstantContext {
        constant: Option<EncodedConstant>,
    }

    impl BytecodeCompilerContext for ConstantContext {
        fn register_for_ptr_with_initialization_status(
            &mut self,
            _pointer: usize,
        ) -> (Register, bool) {
            (0, false)
        }

        fn intern_constant(&mut self, constant: EncodedConstant) -> MResult<u32> {
            assert!(
                self.constant.replace(constant).is_none(),
                "a constant encoder must intern exactly one constant"
            );
            Ok(0)
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
        fn emit_composite_pack(
            &mut self,
            _destination: Register,
            _template: u32,
            _children: Vec<Register>,
        ) {
        }
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

    fn encode(value: &LegacyValue) -> EncodedConstant {
        let mut context = ConstantContext::default();
        value.compile_const(&mut context).unwrap();
        context.constant.unwrap()
    }

    #[test]
    fn scalar_constant_is_interned_by_the_v1_codec() {
        assert_eq!(
            1_u8.compile_const(&mut ConstantContext::default()).unwrap(),
            0
        );
    }

    #[test]
    fn typed_constant_cannot_discard_a_mismatched_declared_type() {
        let value = LegacyValue::Typed(Box::new(LegacyValue::F64(Ref::new(1.0))), ValueKind::Bool);
        let error = value
            .compile_const(&mut ConstantContext::default())
            .unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeConstantUnsupported");
        let detail = error.kind_as::<BytecodeConstantUnsupported>().unwrap();
        assert_eq!(detail.runtime_type, RuntimeType::Bool);
        assert_eq!(detail.source_value_kind, ValueKind::F64);
        assert!(detail.reason.contains("does not match"));
    }

    #[cfg(all(
        feature = "f64",
        feature = "i64",
        feature = "matrix2",
        feature = "matrix2x3",
        feature = "matrix3",
        feature = "row_vector2",
        feature = "vector2",
        feature = "matrixd"
    ))]
    fn f64_matrix_storage(value: &LegacyValue) -> MatrixStorage {
        let LegacyValue::MatrixF64(matrix) = value else {
            panic!("expected an f64 matrix, found {value:?}");
        };
        match matrix {
            Matrix::Matrix2(_) => MatrixStorage::Matrix2,
            Matrix::Matrix2x3(_) => MatrixStorage::Matrix2x3,
            Matrix::RowVector2(_) => MatrixStorage::RowVector2,
            Matrix::Vector2(_) => MatrixStorage::Vector2,
            Matrix::DMatrix(_) => MatrixStorage::MatrixD,
            other => panic!("unexpected matrix storage {other:?}"),
        }
    }

    #[cfg(all(
        feature = "f64",
        feature = "i64",
        feature = "matrix2",
        feature = "matrix2x3",
        feature = "matrix3",
        feature = "row_vector2",
        feature = "vector2",
        feature = "matrixd"
    ))]
    #[test]
    fn present_matrix_options_preserve_concrete_storage() {
        let cases = vec![
            (
                LegacyValue::MatrixF64(Matrix::Matrix2(Ref::new(na::Matrix2::from_row_slice(&[
                    1.0, 2.0, 3.0, 4.0,
                ])))),
                MatrixStorage::Matrix2,
                2,
                2,
            ),
            (
                LegacyValue::MatrixF64(Matrix::Matrix2x3(Ref::new(na::Matrix2x3::from_row_slice(
                    &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
                )))),
                MatrixStorage::Matrix2x3,
                2,
                3,
            ),
            (
                LegacyValue::MatrixF64(Matrix::RowVector2(Ref::new(
                    na::RowVector2::from_row_slice(&[1.0, 2.0]),
                ))),
                MatrixStorage::RowVector2,
                1,
                2,
            ),
            (
                LegacyValue::MatrixF64(Matrix::Vector2(Ref::new(na::Vector2::from_column_slice(
                    &[1.0, 2.0],
                )))),
                MatrixStorage::Vector2,
                2,
                1,
            ),
            (
                LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(na::DMatrix::from_row_slice(
                    2,
                    2,
                    &[1.0, 2.0, 3.0, 4.0],
                )))),
                MatrixStorage::MatrixD,
                2,
                2,
            ),
        ];

        for (value, expected_storage, expected_rows, expected_cols) in cases {
            let declared_kind = value.kind();
            let constant = encode(&LegacyValue::Typed(
                Box::new(value.clone()),
                ValueKind::Option(Box::new(declared_kind)),
            ));
            let RuntimeType::Option(inner) = &constant.runtime_type else {
                panic!("expected an option runtime type");
            };
            let RuntimeType::Matrix {
                element,
                storage,
                rows,
                cols,
            } = inner.as_ref()
            else {
                panic!("expected an option matrix child");
            };
            assert_eq!(*element.as_ref(), RuntimeType::F64);
            assert_eq!(*storage, expected_storage);
            assert_eq!((*rows, *cols), (expected_rows, expected_cols));

            let LegacyValue::Typed(decoded, ValueKind::Option(_)) = decode_one(&constant) else {
                panic!("present matrix option did not decode as a typed option");
            };
            assert_eq!(decoded.as_ref(), &value);
            assert_eq!(f64_matrix_storage(&decoded), expected_storage);
        }
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn absent_matrix_option_uses_annotation_derived_dynamic_storage() {
        let constant = encode(&LegacyValue::EmptyKind(ValueKind::Option(Box::new(
            ValueKind::Matrix(Box::new(ValueKind::F64), vec![2, 2]),
        ))));
        assert_eq!(
            constant.runtime_type,
            RuntimeType::Option(Box::new(RuntimeType::Matrix {
                element: Box::new(RuntimeType::F64),
                storage: MatrixStorage::MatrixD,
                rows: 2,
                cols: 2,
            }))
        );
        assert_eq!(constant.bytes, [0]);
        assert!(matches!(
            decode_one(&constant),
            LegacyValue::EmptyKind(ValueKind::Option(_))
        ));
    }

    #[cfg(all(
        feature = "f64",
        feature = "i64",
        feature = "matrix2",
        feature = "matrix3"
    ))]
    #[test]
    fn present_matrix_options_reject_semantic_mismatches() {
        let declared = ValueKind::Matrix(Box::new(ValueKind::F64), vec![2, 2]);
        let mismatches = vec![
            LegacyValue::MatrixI64(Matrix::Matrix2(Ref::new(na::Matrix2::from_row_slice(&[
                1, 2, 3, 4,
            ])))),
            LegacyValue::MatrixF64(Matrix::Matrix3(Ref::new(na::Matrix3::from_row_slice(&[
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
            ])))),
            LegacyValue::F64(Ref::new(1.0)),
        ];

        for actual in mismatches {
            let error = LegacyValue::Typed(
                Box::new(actual),
                ValueKind::Option(Box::new(declared.clone())),
            )
            .compile_const(&mut ConstantContext::default())
            .unwrap_err();
            assert_eq!(error.kind_name(), "BytecodeConstantUnsupported");
            let detail = error.kind_as::<BytecodeConstantUnsupported>().unwrap();
            assert!(detail.reason.contains("does not match"));
        }
    }

    #[cfg(all(
        feature = "f64",
        feature = "i64",
        feature = "map",
        feature = "matrix2",
        feature = "matrix3",
        feature = "set",
        feature = "table",
        feature = "u8",
        feature = "vectord"
    ))]
    #[test]
    fn fixed_matrix_composite_entries_preserve_concrete_storage() {
        let fixed =
            LegacyValue::MatrixI64(Matrix::Matrix2(Ref::new(na::Matrix2::from_row_slice(&[
                1, 2, 3, 4,
            ]))));
        let matrix_type = RuntimeType::Matrix {
            element: Box::new(RuntimeType::I64),
            storage: MatrixStorage::Matrix2,
            rows: 2,
            cols: 2,
        };
        let assert_fixed = |value: &LegacyValue| {
            assert!(matches!(value, LegacyValue::MatrixI64(Matrix::Matrix2(_))));
        };

        let map = crate::MechMap::from_vec(vec![(LegacyValue::U8(Ref::new(1)), fixed.clone())]);
        let map_constant = encode(&LegacyValue::Map(Ref::new(map)));
        assert_eq!(
            map_constant.runtime_type,
            RuntimeType::Map {
                key: Box::new(RuntimeType::U8),
                value: Box::new(matrix_type.clone()),
            }
        );
        let LegacyValue::Map(decoded_map) = decode_one(&map_constant) else {
            panic!("matrix-valued map did not decode as a map");
        };
        assert_fixed(decoded_map.borrow().map.values().next().unwrap());

        let set = crate::MechSet::from_vec(vec![fixed.clone()]);
        let set_constant = encode(&LegacyValue::Set(Ref::new(set)));
        assert_eq!(
            set_constant.runtime_type,
            RuntimeType::Set {
                element: Box::new(matrix_type.clone()),
                max_len: Some(1),
            }
        );
        let LegacyValue::Set(decoded_set) = decode_one(&set_constant) else {
            panic!("matrix-valued set did not decode as a set");
        };
        assert_fixed(decoded_set.borrow().set.iter().next().unwrap());

        let name = "matrix";
        let id = hash_str(name);
        let mut data = IndexMap::new();
        data.insert(
            id,
            (
                fixed.kind(),
                Matrix::DVector(Ref::new(DVector::from_vec(vec![fixed.clone()]))),
            ),
        );
        let table = crate::MechTable::new(1, 1, data, HashMap::from([(id, name.to_owned())]));
        let table_constant = encode(&LegacyValue::Table(Ref::new(table)));
        assert_eq!(
            table_constant.runtime_type,
            RuntimeType::Table {
                columns: vec![(name.to_owned(), matrix_type)],
                primary_key: 0,
            }
        );
        let LegacyValue::Table(decoded_table) = decode_one(&table_constant) else {
            panic!("matrix-valued table did not decode as a table");
        };
        let table = decoded_table.borrow();
        let (_, Matrix::DVector(cells)) = table.data.values().next().unwrap() else {
            panic!("decoded table column did not preserve dynamic cell storage");
        };
        assert_fixed(&cells.borrow()[0]);
    }

    #[cfg(all(feature = "f64", feature = "set"))]
    #[test]
    fn set_constants_preserve_exact_optional_limits() {
        let cases = [
            (Vec::new(), Some(0)),
            (Vec::new(), Some(10)),
            (
                vec![
                    LegacyValue::F64(Ref::new(1.0)),
                    LegacyValue::F64(Ref::new(2.0)),
                ],
                Some(10),
            ),
            (
                vec![
                    LegacyValue::F64(Ref::new(1.0)),
                    LegacyValue::F64(Ref::new(2.0)),
                ],
                None,
            ),
        ];

        for (values, max_elements) in cases {
            let mut set = crate::MechSet::from_vec(values);
            set.kind = ValueKind::F64;
            set.max_elements = max_elements;
            set.num_elements = max_elements.unwrap_or(0);
            let encoded = encode(&LegacyValue::Set(Ref::new(set)));
            assert_eq!(
                encoded.runtime_type,
                RuntimeType::Set {
                    element: Box::new(RuntimeType::F64),
                    max_len: max_elements.map(|limit| limit as u32),
                }
            );

            let decoded = decode_one(&encoded);
            assert_eq!(
                decoded.kind(),
                ValueKind::Set(Box::new(ValueKind::F64), max_elements)
            );
            assert_eq!(encode(&decoded), encoded);
        }
    }

    #[cfg(all(feature = "table", feature = "vectord", feature = "u8"))]
    fn table(rows: usize, columns: &[&str]) -> crate::MechTable {
        let mut data = IndexMap::new();
        let mut col_names = HashMap::new();
        for (column, name) in columns.iter().enumerate() {
            let id = crate::hash_str(name);
            let cells = (0..rows)
                .map(|row| LegacyValue::U8(Ref::new((row + column) as u8)))
                .collect();
            data.insert(
                id,
                (
                    ValueKind::U8,
                    Matrix::DVector(Ref::new(DVector::from_vec(cells))),
                ),
            );
            col_names.insert(id, (*name).to_owned());
        }
        crate::MechTable::new(rows, columns.len(), data, col_names)
    }

    fn decode_one(constant: &EncodedConstant) -> LegacyValue {
        let parsed =
            ParsedProgram::from_bytes(&write_bytecode(&program(vec![constant.clone()])).unwrap())
                .unwrap();
        parsed.decode_constants().unwrap().pop().unwrap()
    }

    fn table_type(runtime_type: &RuntimeType) -> (&[(String, RuntimeType)], u32) {
        let RuntimeType::Table {
            columns,
            primary_key,
        } = runtime_type
        else {
            panic!("expected a table RuntimeType, found {runtime_type:?}");
        };
        (columns, *primary_key)
    }

    fn option_table_type(runtime_type: &RuntimeType) -> (&[(String, RuntimeType)], u32) {
        let RuntimeType::Option(inner) = runtime_type else {
            panic!("expected an option RuntimeType, found {runtime_type:?}");
        };
        table_type(inner)
    }

    #[cfg(all(feature = "table", feature = "vectord", feature = "u8"))]
    #[test]
    fn present_and_absent_table_options_share_the_same_child_type() {
        let table = table(3, &["value"]);
        let option_kind = ValueKind::Option(Box::new(table.kind()));
        let present = encode(&LegacyValue::Typed(
            Box::new(LegacyValue::Table(Ref::new(table))),
            option_kind.clone(),
        ));
        let absent = encode(&LegacyValue::EmptyKind(option_kind));

        assert_eq!(present.runtime_type, absent.runtime_type);
        let (columns, primary_key) = option_table_type(&present.runtime_type);
        assert_eq!(columns, [("value".to_owned(), RuntimeType::U8)]);
        assert_eq!(primary_key, 0);

        let LegacyValue::Typed(present_value, ValueKind::Option(_)) = decode_one(&present) else {
            panic!("present table option did not decode as a typed option");
        };
        let LegacyValue::Table(present_table) = present_value.as_ref() else {
            panic!("present table option did not preserve its table child");
        };
        assert_eq!(present_table.borrow().rows, 3);
        assert!(matches!(
            decode_one(&absent),
            LegacyValue::EmptyKind(ValueKind::Option(_))
        ));

        assert_eq!(present.bytes[0], 1);
        assert_eq!(
            u32::from_le_bytes(present.bytes[5..9].try_into().unwrap()),
            3
        );
        assert_eq!(absent.bytes, [0]);
    }

    #[cfg(all(
        feature = "f64",
        feature = "map",
        feature = "matrix2",
        feature = "matrixd",
        feature = "record",
        feature = "set",
        feature = "string",
        feature = "table",
        feature = "vectord"
    ))]
    mod annotated_option_composites {
        use std::collections::HashMap;

        use indexmap::{IndexMap, IndexSet};
        use nalgebra::{DMatrix, DVector, Matrix2};

        use crate::matrix::Matrix;
        use crate::{MechMap, MechRecord, MechSet, MechTable};

        use super::*;

        fn option(inner: ValueKind) -> ValueKind {
            ValueKind::Option(Box::new(inner))
        }

        fn present(value: LegacyValue, declared_inner: ValueKind) -> LegacyValue {
            LegacyValue::Typed(Box::new(value), option(declared_inner))
        }

        fn map(
            entries: Vec<(LegacyValue, LegacyValue)>,
            key_kind: ValueKind,
            value_kind: ValueKind,
        ) -> LegacyValue {
            let map = entries.into_iter().collect::<IndexMap<_, _>>();
            LegacyValue::Map(Ref::new(MechMap {
                key_kind,
                value_kind,
                num_elements: map.len(),
                map,
            }))
        }

        fn set(elements: Vec<LegacyValue>, kind: ValueKind) -> LegacyValue {
            let set = elements.into_iter().collect::<IndexSet<_>>();
            LegacyValue::Set(Ref::new(MechSet {
                kind,
                max_elements: Some(set.len()),
                num_elements: set.len(),
                set,
            }))
        }

        fn table(kind: ValueKind, cells: Vec<LegacyValue>) -> LegacyValue {
            let name = "value";
            let id = hash_str(name);
            let rows = cells.len();
            LegacyValue::Table(Ref::new(MechTable::new(
                rows,
                1,
                IndexMap::from([(
                    id,
                    (kind, Matrix::DVector(Ref::new(DVector::from_vec(cells)))),
                )]),
                HashMap::from([(id, name.to_owned())]),
            )))
        }

        fn matrix2() -> LegacyValue {
            LegacyValue::MatrixF64(Matrix::Matrix2(Ref::new(Matrix2::from_row_slice(&[
                1.0, 2.0, 3.0, 4.0,
            ]))))
        }

        fn dynamic_matrix2() -> LegacyValue {
            LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::from_row_slice(
                2,
                2,
                &[1.0, 2.0, 3.0, 4.0],
            ))))
        }

        fn declared_matrix() -> ValueKind {
            ValueKind::Matrix(Box::new(ValueKind::F64), vec![2, 2])
        }

        fn option_inner(runtime_type: &RuntimeType) -> &RuntimeType {
            let RuntimeType::Option(inner) = runtime_type else {
                panic!("expected option runtime type, found {runtime_type:?}");
            };
            inner
        }

        #[test]
        fn map_values_finalize_absent_and_present_options_independent_of_iteration_order() {
            let entries = vec![
                (
                    LegacyValue::String(Ref::new("absent".into())),
                    LegacyValue::Empty,
                ),
                (
                    LegacyValue::String(Ref::new("present".into())),
                    present(LegacyValue::F64(Ref::new(1.0)), ValueKind::F64),
                ),
            ];
            let forward = encode(&map(
                entries.clone(),
                ValueKind::String,
                option(ValueKind::F64),
            ));
            let reverse = encode(&map(
                entries.into_iter().rev().collect(),
                ValueKind::String,
                option(ValueKind::F64),
            ));

            assert_eq!(forward, reverse);
            assert_eq!(
                forward.runtime_type,
                RuntimeType::Map {
                    key: Box::new(RuntimeType::String),
                    value: Box::new(RuntimeType::Option(Box::new(RuntimeType::F64))),
                }
            );
            assert_eq!(encode(&decode_one(&forward)), forward);
        }

        #[test]
        fn map_keys_finalize_absent_and_present_options() {
            let encoded = encode(&map(
                vec![
                    (LegacyValue::Empty, LegacyValue::F64(Ref::new(1.0))),
                    (
                        present(
                            LegacyValue::String(Ref::new("present".into())),
                            ValueKind::String,
                        ),
                        LegacyValue::F64(Ref::new(2.0)),
                    ),
                ],
                option(ValueKind::String),
                ValueKind::F64,
            ));

            assert_eq!(
                encoded.runtime_type,
                RuntimeType::Map {
                    key: Box::new(RuntimeType::Option(Box::new(RuntimeType::String))),
                    value: Box::new(RuntimeType::F64),
                }
            );
            assert_eq!(encode(&decode_one(&encoded)), encoded);
        }

        #[test]
        fn set_scalar_options_round_trip_absent_and_present_values() {
            let encoded = encode(&set(
                vec![
                    LegacyValue::Empty,
                    present(LegacyValue::F64(Ref::new(1.0)), ValueKind::F64),
                ],
                option(ValueKind::F64),
            ));

            assert_eq!(
                encoded.runtime_type,
                RuntimeType::Set {
                    element: Box::new(RuntimeType::Option(Box::new(RuntimeType::F64))),
                    max_len: Some(2),
                }
            );
            assert_eq!(encode(&decode_one(&encoded)), encoded);
        }

        #[test]
        fn set_fixed_matrix_options_are_canonical_in_both_iteration_orders() {
            let present = present(matrix2(), declared_matrix());
            let forward = encode(&set(
                vec![LegacyValue::Empty, present.clone()],
                option(declared_matrix()),
            ));
            let reverse = encode(&set(
                vec![present, LegacyValue::Empty],
                option(declared_matrix()),
            ));

            assert_eq!(forward, reverse);
            let RuntimeType::Set { element, .. } = &forward.runtime_type else {
                panic!("expected set runtime type");
            };
            assert!(matches!(
                option_inner(element),
                RuntimeType::Matrix {
                    storage: MatrixStorage::Matrix2,
                    ..
                }
            ));
            assert_eq!(encode(&decode_one(&forward)), forward);
        }

        #[test]
        fn all_absent_matrix_options_use_the_annotation_derived_dynamic_storage() {
            let encoded = encode(&set(vec![LegacyValue::Empty], option(declared_matrix())));
            let RuntimeType::Set { element, .. } = &encoded.runtime_type else {
                panic!("expected set runtime type");
            };
            assert!(matches!(
                option_inner(element),
                RuntimeType::Matrix {
                    storage: MatrixStorage::MatrixD,
                    ..
                }
            ));
        }

        #[test]
        fn heterogeneous_present_matrix_option_storage_is_rejected() {
            let error = set(
                vec![
                    present(matrix2(), declared_matrix()),
                    present(dynamic_matrix2(), declared_matrix()),
                ],
                option(declared_matrix()),
            )
            .compile_const(&mut ConstantContext::default())
            .unwrap_err();

            assert_eq!(error.kind_name(), "BytecodeConstantUnsupported");
            assert!(
                error
                    .kind_as::<BytecodeConstantUnsupported>()
                    .unwrap()
                    .reason
                    .contains("set element type")
            );
        }

        #[test]
        fn bare_empty_under_non_option_annotation_is_rejected() {
            let error = set(vec![LegacyValue::Empty], ValueKind::F64)
                .compile_const(&mut ConstantContext::default())
                .unwrap_err();

            assert_eq!(error.kind_name(), "BytecodeConstantUnsupported");
        }

        #[test]
        fn explicit_absent_option_must_match_the_declared_child_type() {
            let error = set(
                vec![LegacyValue::EmptyKind(option(ValueKind::String))],
                option(ValueKind::F64),
            )
            .compile_const(&mut ConstantContext::default())
            .unwrap_err();

            assert_eq!(error.kind_name(), "BytecodeConstantUnsupported");
        }

        #[test]
        fn table_scalar_option_column_accepts_bare_empty_cells() {
            let encoded = encode(&table(
                option(ValueKind::F64),
                vec![
                    LegacyValue::Empty,
                    present(LegacyValue::F64(Ref::new(1.0)), ValueKind::F64),
                ],
            ));
            let RuntimeType::Table { columns, .. } = &encoded.runtime_type else {
                panic!("expected table runtime type");
            };
            assert_eq!(
                columns,
                &[(
                    "value".into(),
                    RuntimeType::Option(Box::new(RuntimeType::F64))
                )]
            );
            assert_eq!(encode(&decode_one(&encoded)), encoded);
        }

        #[test]
        fn table_fixed_matrix_option_column_uses_the_present_storage() {
            let encoded = encode(&table(
                option(declared_matrix()),
                vec![LegacyValue::Empty, present(matrix2(), declared_matrix())],
            ));
            let RuntimeType::Table { columns, .. } = &encoded.runtime_type else {
                panic!("expected table runtime type");
            };
            assert!(matches!(
                option_inner(&columns[0].1),
                RuntimeType::Matrix {
                    storage: MatrixStorage::Matrix2,
                    ..
                }
            ));
            assert_eq!(encode(&decode_one(&encoded)), encoded);
        }

        #[test]
        fn record_option_field_accepts_a_bare_empty_value() {
            let name = "optional";
            let id = hash_str(name);
            let encoded = encode(&LegacyValue::Record(Ref::new(MechRecord {
                cols: 1,
                kinds: vec![option(ValueKind::String)],
                data: IndexMap::from([(id, LegacyValue::Empty)]),
                field_names: HashMap::from([(id, name.into())]),
            })));

            assert_eq!(
                encoded.runtime_type,
                RuntimeType::Record(vec![(
                    name.into(),
                    RuntimeType::Option(Box::new(RuntimeType::String)),
                )])
            );
            assert_eq!(encode(&decode_one(&encoded)), encoded);
        }
    }

    #[cfg(all(feature = "table", feature = "vectord", feature = "u8"))]
    #[test]
    fn table_row_count_is_payload_data_and_never_primary_key_metadata() {
        let cases = [
            ("zero-row", table(0, &["value"]), 0),
            ("one-row", table(1, &["value"]), 1),
            ("more-rows-than-columns", table(3, &["value"]), 3),
            ("rows-equal-columns", table(2, &["left", "right"]), 2),
            ("multi-column", table(1, &["left", "middle", "right"]), 1),
        ];

        let mut one_column_types = Vec::new();
        for (name, value, expected_rows) in cases {
            let constant = encode(&LegacyValue::Table(Ref::new(value.clone())));
            let (columns, primary_key) = table_type(&constant.runtime_type);
            assert_eq!(primary_key, 0, "{name}");
            assert_eq!(columns.len(), value.cols, "{name}");
            assert_eq!(
                u32::from_le_bytes(constant.bytes[0..4].try_into().unwrap()),
                expected_rows,
                "{name}"
            );

            let LegacyValue::Table(decoded) = decode_one(&constant) else {
                panic!("{name} constant did not decode as a table");
            };
            assert_eq!(decoded.borrow().rows, expected_rows as usize, "{name}");
            assert_eq!(decoded.borrow().cols, value.cols, "{name}");

            if value.cols == 1 {
                one_column_types.push(constant.runtime_type);
            }
        }

        assert!(
            one_column_types
                .windows(2)
                .all(|types| types[0] == types[1])
        );
    }
}
