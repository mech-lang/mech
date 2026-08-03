#!/usr/bin/env python3
"""Validate the frozen deterministic Phase 1 bytecode-v1 corpus."""

from __future__ import annotations

import hashlib
import json
import struct
import sys
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "tests/architecture/bytecode-v1/phase1"
MANIFEST = CORPUS / "manifest.json"
EXPECTED_FIXTURE_METADATA = {
    "literal-f64.mecb": {
        "sha256": "dd9596d0a47e87de8b0a6455bbf0058583649c7db16dab4336a5a91694bcac92",
        "source": "42.0",
        "expected_result": 42.0,
        "runtime_functions": [],
    },
    "scalar-add-f64.mecb": {
        "sha256": "732def592c0ba921b7dd8e782163a2803a0ac21adfa5469c133bbeca5fe0f4f2",
        "source": "1.0 + 2.0",
        "expected_result": 3.0,
        "runtime_functions": ["AddSS<f64>"],
    },
    "fixed-matrix-add-f64.mecb": {
        "sha256": "d60ec57aadbc280bca7a3fa577cc10cc2727acb146193eada141f44c80189227",
        "source": "[1.0 2.0; 3.0 4.0] + [5.0 6.0; 7.0 8.0]",
        "expected_result": [[6.0, 8.0], [10.0, 12.0]],
        "runtime_functions": [
            "HorizontalConcatenateS2<f64>",
            "VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>",
            "AddM2M2<f64>",
        ],
    },
    "dynamic-matrix-add-f64.mecb": {
        "sha256": "7355c52b197e169b87bd425a0e147f86dcad744d76f743e716223a23b2890924",
        "source": (
            "[1.0 2.0 3.0 4.0 5.0; 6.0 7.0 8.0 9.0 10.0; "
            "11.0 12.0 13.0 14.0 15.0; 16.0 17.0 18.0 19.0 20.0; "
            "21.0 22.0 23.0 24.0 25.0] + "
            "[25.0 24.0 23.0 22.0 21.0; 20.0 19.0 18.0 17.0 16.0; "
            "15.0 14.0 13.0 12.0 11.0; 10.0 9.0 8.0 7.0 6.0; "
            "5.0 4.0 3.0 2.0 1.0]"
        ),
        "expected_result": [[26.0] * 5 for _ in range(5)],
        "runtime_functions": [
            "HorizontalConcatenateRDN<f64>",
            "VerticalConcatenateNArgs<f64>",
            "AddMDMD<f64>",
        ],
    },
    "variadic-horzcat-f64.mecb": {
        "sha256": "f59cafbed042d33d8ab7ad2e1dc35eb8d25cc556f3d5af3479923c47abd7edaf",
        "source": "[1.0 2.0 3.0 4.0 5.0]",
        "expected_result": [[1.0, 2.0, 3.0, 4.0, 5.0]],
        "runtime_functions": ["HorizontalConcatenateRDN<f64>"],
    },
    "cli-stdout.mecb": {
        "sha256": "295493d90b5b653bfee67763e57a5da60765edfdecd4926ec1227a5b6fc7ae57",
        "source": '+> @out := cli/stdout\n\n@out/line <- "phase1-hosted-ok"\n\n"done"',
        "expected_result": "done",
        "runtime_functions": [],
    },
    "synthetic-live-read.mecb": {
        "sha256": "d4141713973607c9e02cbed6d67b5010022035a95bcd88a7218834606bd35609",
        "source": (
            "+> @clock := test-live/clock\n\n"
            "value := @clock/value\n"
            "doubled := value + value\n\n"
            "doubled"
        ),
        "expected_result": 0.0,
        "runtime_functions": ["AddSS<f64>", "VariableDefineF64"],
    },
}
EXPECTED_FILES = list(EXPECTED_FIXTURE_METADATA)
HEADER = struct.Struct("<4s6H2I2H3Q12s")
SECTION = struct.Struct("<HHIQQQ")
SECTION_NAMES = [
    "Types",
    "ConstantTable",
    "ConstantBlob",
    "Symbols",
    "Instructions",
    "Dictionary",
    "ApplicationRequirements",
]
TYPE_NAMES = {
    1: "u8",
    2: "u16",
    3: "u32",
    4: "u64",
    5: "u128",
    6: "i8",
    7: "i16",
    8: "i32",
    9: "i64",
    10: "i128",
    11: "f32",
    12: "f64",
    13: "c64",
    14: "r64",
    15: "string",
    16: "bool",
    17: "id",
    18: "index",
    19: "empty",
    20: "any",
    21: "none",
}
MATRIX_STORAGE_NAMES = {
    1: "Matrix1",
    2: "Matrix2",
    3: "Matrix3",
    4: "Matrix4",
    5: "Matrix2x3",
    6: "Matrix3x2",
    7: "RowVector2",
    8: "RowVector3",
    9: "RowVector4",
    10: "Vector2",
    11: "Vector3",
    12: "Vector4",
    13: "RowVectorD",
    14: "VectorD",
    15: "MatrixD",
}
INTENT_NAMES = {1: "Read", 2: "Assign", 3: "Send"}
DELIVERY_NAMES = {0: "Snapshot", 1: "Live"}
U64_MASK = (1 << 64) - 1
MECH_HASH_MASK = 0x00FFFFFFFFFFFFFF
SEAHASH_DIFFUSION = 0x6EED0E9DA4D94A4F
SEAHASH_SEEDS = (
    0x16F11FE89B0D677C,
    0xB480A793D8E6C86C,
    0x6FE2E5AAF078EBC9,
    0x14F994A4C5259381,
)


class ContractError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def canonical_json(value: object) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True)


def seahash_diffuse(value: int) -> int:
    value = (value * SEAHASH_DIFFUSION) & U64_MASK
    value ^= (value >> 32) >> (value >> 60)
    return (value * SEAHASH_DIFFUSION) & U64_MASK


def seahash(data: bytes) -> int:
    a, b, c, d = SEAHASH_SEEDS
    for offset in range(0, len(data), 8):
        block = int.from_bytes(data[offset : offset + 8], "little")
        a, b, c, d = b, c, d, seahash_diffuse(a ^ block)
    return seahash_diffuse(a ^ b ^ c ^ d ^ len(data))


def mech_hash(value: str) -> int:
    return seahash(value.encode("utf-8")) & MECH_HASH_MASK


def valid_matrix_dimensions(storage_id: int, rows: int, cols: int) -> bool:
    fixed = {
        1: (1, 1),
        2: (2, 2),
        3: (3, 3),
        4: (4, 4),
        5: (2, 3),
        6: (3, 2),
        7: (1, 2),
        8: (1, 3),
        9: (1, 4),
        10: (2, 1),
        11: (3, 1),
        12: (4, 1),
    }
    if storage_id in fixed:
        return (rows, cols) == fixed[storage_id]
    if storage_id == 13:
        return rows == 1 and cols > 0
    if storage_id == 14:
        return rows > 0 and cols == 1
    if storage_id == 15:
        return rows > 0 and cols > 0
    return False


class Reader:
    def __init__(self, data: bytes, label: str) -> None:
        self.data = data
        self.label = label
        self.offset = 0

    def read(self, size: int) -> bytes:
        end = self.offset + size
        require(end <= len(self.data), f"{self.label}: truncated payload")
        value = self.data[self.offset:end]
        self.offset = end
        return value

    def unpack(self, format_: str) -> tuple[object, ...]:
        layout = struct.Struct("<" + format_)
        return layout.unpack(self.read(layout.size))

    def u8(self) -> int:
        return int(self.unpack("B")[0])

    def u16(self) -> int:
        return int(self.unpack("H")[0])

    def u32(self) -> int:
        return int(self.unpack("I")[0])

    def u64(self) -> int:
        return int(self.unpack("Q")[0])

    def text(self, size: int) -> str:
        try:
            return self.read(size).decode("utf-8")
        except UnicodeDecodeError as error:
            raise ContractError(f"{self.label}: string is not UTF-8") from error

    def string(self) -> str:
        return self.text(self.u32())

    def finish(self) -> None:
        require(self.offset == len(self.data), f"{self.label}: trailing payload bytes")


def decode_types(payload: bytes, count: int, name: str) -> list[dict[str, object]]:
    reader = Reader(payload, f"{name}: types")
    types: list[dict[str, object]] = []
    canonical_order: list[tuple[int, bytes]] = []
    for type_id in range(count):
        tag = reader.u16()
        flags = reader.u16()
        body = Reader(reader.read(reader.u32()), f"{name}: type {type_id}")
        require(flags == 0, f"{name}: type {type_id} has nonzero flags")
        if tag in TYPE_NAMES:
            type_description: dict[str, object] = {"kind": TYPE_NAMES[tag]}
            dependency_depth = 0
            canonical_key = struct.pack("<H", tag)
        elif tag == 22:
            element_id = body.u32()
            storage_id = body.u8()
            rows = body.u32()
            cols = body.u32()
            require(element_id < type_id, f"{name}: matrix child type is not topological")
            require(storage_id in MATRIX_STORAGE_NAMES, f"{name}: unknown matrix storage")
            require(
                valid_matrix_dimensions(storage_id, rows, cols),
                f"{name}: matrix storage and dimensions disagree",
            )
            type_description = {
                "cols": cols,
                "element": types[element_id]["type"],
                "kind": "matrix",
                "rows": rows,
                "storage": MATRIX_STORAGE_NAMES[storage_id],
                "storage_id": storage_id,
            }
            child_depth, child_key = canonical_order[element_id]
            dependency_depth = child_depth + 1
            canonical_key = (
                struct.pack("<HBII", tag, storage_id, rows, cols)
                + struct.pack("<I", len(child_key))
                + child_key
            )
        else:
            raise ContractError(f"{name}: unexpected Phase 1 runtime type tag {tag}")
        body.finish()
        types.append({"id": type_id, "type": type_description})
        canonical_order.append((dependency_depth, canonical_key))
    reader.finish()
    require(
        canonical_order == sorted(set(canonical_order)),
        f"{name}: runtime type IDs are not in canonical deterministic order",
    )
    return types


def decode_constants(
    table: bytes,
    count: int,
    blob: bytes,
    types: list[dict[str, object]],
    name: str,
) -> None:
    require(len(table) == count * 24, f"{name}: constant table length is not exact")
    reader = Reader(table, f"{name}: constant table")
    previous_end = 0
    for index in range(count):
        type_id = reader.u32()
        storage = reader.u8()
        alignment = reader.u8()
        reserved = reader.u16()
        offset = reader.u64()
        length = reader.u64()
        require(type_id < len(types), f"{name}: constant {index} has invalid type ID")
        require(storage == 1, f"{name}: constant {index} is not inline blob storage")
        require(alignment in {1, 2, 4, 8, 16}, f"{name}: invalid constant alignment")
        require(reserved == 0, f"{name}: constant {index} has nonzero reserved field")
        require(offset % alignment == 0, f"{name}: constant {index} is misaligned")
        require(offset >= previous_end, f"{name}: constant payloads are not ordered")
        require(not any(blob[previous_end:offset]), f"{name}: constant padding is nonzero")
        require(offset + length <= len(blob), f"{name}: constant payload is out of bounds")
        value = blob[offset : offset + length]
        runtime_type = types[type_id]["type"]
        require(isinstance(runtime_type, dict), f"{name}: malformed runtime type metadata")
        kind = runtime_type.get("kind")
        if kind == "empty":
            require(not value, f"{name}: Empty constant has payload bytes")
        elif kind == "bool":
            require(value in {b"\x00", b"\x01"}, f"{name}: Bool constant is not canonical")
        elif kind == "string":
            try:
                value.decode("utf-8")
            except UnicodeDecodeError as error:
                raise ContractError(f"{name}: String constant is not UTF-8") from error
        elif kind in {"index", "f64"}:
            require(len(value) == 8, f"{name}: {kind} constant is not eight bytes")
        elif kind == "matrix":
            element = runtime_type.get("element")
            rows = runtime_type.get("rows")
            cols = runtime_type.get("cols")
            require(element == {"kind": "f64"}, f"{name}: matrix constant element is not f64")
            require(
                isinstance(rows, int) and isinstance(cols, int),
                f"{name}: matrix constant dimensions are malformed",
            )
            expected_length = 8 + rows * cols * 8
            require(
                len(value) == expected_length,
                f"{name}: matrix constant payload length disagrees with its type",
            )
            encoded_rows, encoded_cols = struct.unpack_from("<II", value)
            require(
                (encoded_rows, encoded_cols) == (rows, cols),
                f"{name}: matrix constant dimensions disagree with its type",
            )
        else:
            raise ContractError(f"{name}: unsupported Phase 1 constant type {kind!r}")
        previous_end = offset + length
    reader.finish()
    require(previous_end == len(blob), f"{name}: constant blob has trailing bytes")


def decode_symbols(payload: bytes, count: int, registers: int, name: str) -> set[int]:
    require(len(payload) == count * 16, f"{name}: symbol table length is not exact")
    reader = Reader(payload, f"{name}: symbols")
    ids: list[int] = []
    for _ in range(count):
        symbol = reader.u64()
        register = reader.u32()
        mutable = reader.u32()
        require(register < registers, f"{name}: symbol register is out of bounds")
        require(mutable in {0, 1}, f"{name}: symbol mutability is not canonical")
        ids.append(symbol)
    reader.finish()
    require(ids == sorted(set(ids)), f"{name}: symbols are not sorted and unique")
    return set(ids)


def decode_dictionary(payload: bytes, count: int, name: str) -> set[int]:
    reader = Reader(payload, f"{name}: dictionary")
    ids: list[int] = []
    for _ in range(count):
        identifier = reader.u64()
        value = reader.string()
        require(value, f"{name}: dictionary contains an empty value")
        require(
            mech_hash(value) == identifier,
            f"{name}: dictionary value does not hash to its ID",
        )
        ids.append(identifier)
    reader.finish()
    require(ids == sorted(set(ids)), f"{name}: dictionary is not sorted and unique")
    return set(ids)


def validate_resource_identity(base_uri: str, path: str, name: str) -> None:
    require(
        not base_uri.endswith("/"),
        f"{name}: resource base URI has a trailing slash",
    )
    require("://" in base_uri, f"{name}: resource base URI has no scheme separator")
    scheme, rest = base_uri.split("://", 1)
    require(scheme, f"{name}: resource base URI has an empty scheme")
    authority = rest.split("/", 1)[0]
    require(authority, f"{name}: resource base URI has an empty authority")
    require(path == path.strip(), f"{name}: resource path has surrounding whitespace")
    if path:
        segments = path.split("/")
        require(all(segments), f"{name}: resource path has an empty segment")
        require("." not in segments, f"{name}: resource path has a `.` segment")
        require(".." not in segments, f"{name}: resource path has a `..` segment")


def decode_requirements(
    payload: bytes, count: int, name: str
) -> list[dict[str, object]]:
    reader = Reader(payload, f"{name}: requirements")
    requirements: list[dict[str, object]] = []
    ordering_keys: list[tuple[object, ...]] = []
    for _ in range(count):
        kind = reader.u8()
        intent = reader.u8()
        delivery = reader.u8()
        reserved = reader.u8()
        operation_len = reader.u16()
        context_len = reader.u16()
        primary_len = reader.u32()
        secondary_len = reader.u32()
        operation = reader.text(operation_len)
        context = reader.text(context_len)
        primary = reader.text(primary_len)
        secondary = reader.text(secondary_len)
        require(reserved == 0, f"{name}: requirement has nonzero reserved byte")
        require(primary, f"{name}: requirement primary field is empty")
        if kind == 1:
            require(
                intent == delivery == 0 and not operation and not context and not secondary,
                f"{name}: host-function requirement is not canonical",
            )
            requirements.append({"kind": "host-function", "name": primary})
            ordering_keys.append((kind, primary))
        elif kind == 2:
            require(intent in INTENT_NAMES, f"{name}: invalid resource intent")
            require(delivery in DELIVERY_NAMES, f"{name}: invalid resource delivery")
            require(operation and context, f"{name}: resource operation/context is empty")
            validate_resource_identity(primary, secondary, name)
            requirements.append(
                {
                    "base_uri": primary,
                    "context_name": context,
                    "delivery": DELIVERY_NAMES[delivery],
                    "delivery_id": delivery,
                    "intent": INTENT_NAMES[intent],
                    "intent_id": intent,
                    "kind": "resource",
                    "operation": operation,
                    "path": secondary,
                }
            )
            ordering_keys.append(
                (kind, intent, delivery, operation, context, primary, secondary)
            )
        else:
            raise ContractError(f"{name}: invalid application requirement kind {kind}")
    reader.finish()
    require(
        ordering_keys == sorted(set(ordering_keys)),
        f"{name}: requirements are not sorted and unique",
    )
    return requirements


def decode_instructions(
    payload: bytes,
    count: int,
    registers: int,
    constants: int,
    requirements: list[dict[str, object]],
    name: str,
) -> set[int]:
    reader = Reader(payload, f"{name}: instructions")
    runtime_ids: set[int] = set()
    opcodes: list[int] = []

    def register(value: int) -> None:
        require(value < registers, f"{name}: instruction register is out of bounds")

    def requirement(index: int) -> None:
        require(index < len(requirements), f"{name}: requirement index is out of bounds")

    def runtime_function() -> int:
        function = reader.u64()
        require(function != 0, f"{name}: runtime function ID is zero")
        return function

    for _ in range(count):
        opcode = reader.u8()
        opcodes.append(opcode)
        if opcode == 0x01:
            register(reader.u32())
            require(reader.u32() < constants, f"{name}: constant index is out of bounds")
        elif opcode in {0x10, 0x11, 0x12, 0x13, 0x14}:
            runtime_ids.add(runtime_function())
            operand_count = {0x10: 1, 0x11: 2, 0x12: 3, 0x13: 4, 0x14: 5}[opcode]
            for _ in range(operand_count):
                register(reader.u32())
        elif opcode == 0x15:
            runtime_ids.add(runtime_function())
            register(reader.u32())
            for _ in range(reader.u32()):
                register(reader.u32())
        elif opcode == 0x20:
            requirement(reader.u32())
            register(reader.u32())
            for _ in range(reader.u32()):
                register(reader.u32())
        elif opcode in {0x21, 0x22, 0x23}:
            requirement(reader.u32())
            register(reader.u32())
            if opcode in {0x22, 0x23}:
                register(reader.u32())
        elif opcode == 0xFF:
            register(reader.u32())
        else:
            raise ContractError(f"{name}: invalid opcode 0x{opcode:02x}")
    reader.finish()
    require(opcodes and opcodes[-1] == 0xFF, f"{name}: final instruction is not Return")
    require(opcodes.count(0xFF) == 1, f"{name}: fixture must contain one Return")
    return runtime_ids


def validate_fixture(entry: dict[str, object]) -> None:
    name = entry.get("file")
    require(isinstance(name, str), "manifest fixture is missing a file name")
    expected_metadata = EXPECTED_FIXTURE_METADATA.get(name)
    require(expected_metadata is not None, f"{name}: fixture is not in the frozen corpus")
    require(
        entry.get("source") == expected_metadata["source"],
        f"{name}: manifest source is stale",
    )
    require(
        canonical_json(entry.get("expected_result"))
        == canonical_json(expected_metadata["expected_result"]),
        f"{name}: manifest expected result is stale",
    )
    path = CORPUS / name
    data = path.read_bytes()
    require(len(data) >= 292, f"{name}: file is shorter than the v1 envelope")
    observed_sha256 = hashlib.sha256(data).hexdigest()
    require(
        observed_sha256 == expected_metadata["sha256"],
        f"{name}: frozen fixture SHA-256 changed",
    )
    require(
        observed_sha256 == entry.get("sha256"),
        f"{name}: SHA-256 does not match manifest",
    )

    unpacked = HEADER.unpack_from(data)
    (
        magic,
        version,
        header_size,
        mech_major,
        mech_minor,
        mech_patch,
        flags,
        register_count,
        instruction_count,
        section_count,
        reserved0,
        section_table_offset,
        file_len,
        checksum_offset,
        reserved,
    ) = unpacked
    require(magic == b"MECH", f"{name}: wrong magic")
    require(version == 1 and header_size == 64, f"{name}: wrong v1 header")
    require((mech_major, mech_minor, mech_patch) == (0, 3, 5), f"{name}: wrong Mech version")
    require(flags == 0 and reserved0 == 0 and reserved == bytes(12), f"{name}: nonzero reserved header field")
    require(section_count == 7 and section_table_offset == 64, f"{name}: wrong section directory")
    require(file_len == len(data), f"{name}: header length mismatch")
    require(checksum_offset == len(data) - 4, f"{name}: checksum is not the four-byte trailer")
    expected_crc = struct.unpack_from("<I", data, checksum_offset)[0]
    require(zlib.crc32(data[:checksum_offset]) & 0xFFFFFFFF == expected_crc, f"{name}: CRC32 mismatch")

    manifest_header = entry.get("header")
    require(isinstance(manifest_header, dict), f"{name}: manifest header is missing")
    observed_header = {
        "magic": "MECH",
        "version": version,
        "header_size": header_size,
        "mech_major": mech_major,
        "mech_minor": mech_minor,
        "mech_patch": mech_patch,
        "flags": flags,
        "register_count": register_count,
        "instruction_count": instruction_count,
        "section_count": section_count,
        "reserved0": reserved0,
        "section_table_offset": section_table_offset,
        "file_len": file_len,
        "checksum_offset": checksum_offset,
        "reserved": list(reserved),
    }
    require(observed_header == manifest_header, f"{name}: manifest header metadata is stale")

    sections: list[dict[str, object]] = []
    section_payloads: list[bytes] = []
    previous_end = 288
    for index, section_name in enumerate(SECTION_NAMES):
        offset = 64 + index * SECTION.size
        kind, section_flags, item_count, start, length, section_reserved = SECTION.unpack_from(data, offset)
        require(kind == index + 1, f"{name}: section kinds are not exact and ordered")
        require(section_flags == 0 and section_reserved == 0, f"{name}: reserved section field is nonzero")
        require(start % 8 == 0 and start >= previous_end, f"{name}: section is unaligned or overlapping")
        require(start + length <= checksum_offset, f"{name}: section overlaps the CRC trailer")
        require(not any(data[previous_end:start]), f"{name}: section padding is nonzero")
        previous_end = start + length
        sections.append(
            {
                "id": kind,
                "kind": section_name,
                "flags": section_flags,
                "item_count": item_count,
                "offset": start,
                "length": length,
                "reserved": section_reserved,
            }
        )
        section_payloads.append(data[start : start + length])
    require(sections[0]["offset"] == 288, f"{name}: first content offset is not 288")
    require(not any(data[previous_end:checksum_offset]), f"{name}: trailing padding is nonzero")
    require(sections == entry.get("sections"), f"{name}: manifest section metadata is stale")
    require(
        instruction_count == sections[4]["item_count"],
        f"{name}: instruction section count disagrees with header",
    )

    decoded_types = decode_types(
        section_payloads[0], int(sections[0]["item_count"]), name
    )
    require(
        decoded_types == entry.get("runtime_types"),
        f"{name}: decoded runtime types disagree with manifest",
    )
    decode_constants(
        section_payloads[1],
        int(sections[1]["item_count"]),
        section_payloads[2],
        decoded_types,
        name,
    )
    symbol_ids = decode_symbols(
        section_payloads[3],
        int(sections[3]["item_count"]),
        register_count,
        name,
    )
    dictionary_ids = decode_dictionary(
        section_payloads[5], int(sections[5]["item_count"]), name
    )
    require(
        symbol_ids.issubset(dictionary_ids),
        f"{name}: a symbol is missing from the dictionary",
    )
    decoded_requirements = decode_requirements(
        section_payloads[6], int(sections[6]["item_count"]), name
    )
    require(
        decoded_requirements == entry.get("application_requirements"),
        f"{name}: decoded requirements disagree with manifest",
    )
    decoded_runtime_ids = decode_instructions(
        section_payloads[4],
        int(sections[4]["item_count"]),
        register_count,
        int(sections[1]["item_count"]),
        decoded_requirements,
        name,
    )

    runtime_ids = entry.get("runtime_function_ids")
    require(isinstance(runtime_ids, list), f"{name}: runtime function metadata is missing")
    ids: list[int] = []
    runtime_names: list[str] = []
    for item in runtime_ids:
        require(isinstance(item, dict), f"{name}: runtime function metadata is malformed")
        function_id = item.get("id")
        function_name = item.get("name")
        require(
            isinstance(function_id, int) and not isinstance(function_id, bool),
            f"{name}: runtime function ID is malformed",
        )
        require(
            isinstance(function_name, str) and function_name,
            f"{name}: runtime function name is malformed",
        )
        require(function_id != 0, f"{name}: runtime function ID is zero")
        require(
            mech_hash(function_name) == function_id,
            f"{name}: runtime function name does not hash to its ID",
        )
        require(
            item.get("id_hex") == f"{function_id:016x}",
            f"{name}: runtime function hexadecimal ID is stale",
        )
        ids.append(function_id)
        runtime_names.append(function_name)
    require(
        runtime_names == expected_metadata["runtime_functions"],
        f"{name}: runtime function names are stale",
    )
    require(ids == sorted(set(ids)), f"{name}: runtime function IDs are not sorted and unique")
    require(
        decoded_runtime_ids == set(ids),
        f"{name}: instruction runtime IDs disagree with manifest",
    )


def main() -> int:
    try:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        require(isinstance(manifest, dict), "manifest root must be an object")
        require(manifest.get("format") == "mech-bytecode-v1", "manifest format is stale")
        require(
            manifest.get("phase") == 1 and not isinstance(manifest.get("phase"), bool),
            "manifest phase is stale",
        )
        fixtures = manifest.get("fixtures")
        require(isinstance(fixtures, list), "manifest fixtures must be a list")
        require(
            all(isinstance(entry, dict) for entry in fixtures),
            "manifest fixtures must be objects",
        )
        names = [entry.get("file") for entry in fixtures]
        require(names == EXPECTED_FILES, "fixture manifest order or membership changed")
        disk_entries = sorted(CORPUS.iterdir(), key=lambda path: path.name)
        require(
            [path.name for path in disk_entries]
            == sorted([*EXPECTED_FILES, MANIFEST.name]),
            "fixture corpus membership changed",
        )
        require(
            all(path.is_file() and not path.is_symlink() for path in disk_entries),
            "fixture corpus entries must be regular files",
        )
        for entry in fixtures:
            validate_fixture(entry)
    except (ContractError, OSError, TypeError, ValueError, KeyError, struct.error) as error:
        print(f"bytecode v1 format contract failed: {error}", file=sys.stderr)
        return 1
    print("bytecode v1 format contract passed (7 deterministic fixtures)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
