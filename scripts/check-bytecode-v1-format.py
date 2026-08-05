#!/usr/bin/env python3
"""Validate the frozen deterministic bytecode-v1 corpus."""

from __future__ import annotations

import hashlib
import json
import struct
import sys
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "tests/architecture/bytecode-v1"
MANIFEST = CORPUS / "manifest.json"
EXPECTED_MANIFEST_SHA256 = "aaa5b3e4217d14d7dd356c54dee38677c582a953bd0cfad63fbd6bc753d95c4c"
EXPECTED_FIXTURE_SHA256 = {
    "canonical-scalars.mecb": "f470d9afe5397a93dc7f52c88a81d9e72810d6789fbdb1588ec8494277effc88",
    "canonical-matrices.mecb": "753c19e2b41ad2b7984bb8adc40dd5c3623c2d12aa238cf954a51ad7e723b193",
    "canonical-composites.mecb": "21b2c1247be183d5b13c5f5c9ed7964a9c7f848cd173824af7185e2240652c02",
    "literal-f64.mecb": "dd9596d0a47e87de8b0a6455bbf0058583649c7db16dab4336a5a91694bcac92",
    "scalar-add-f64.mecb": "732def592c0ba921b7dd8e782163a2803a0ac21adfa5469c133bbeca5fe0f4f2",
    "fixed-matrix-add-f64.mecb": "d60ec57aadbc280bca7a3fa577cc10cc2727acb146193eada141f44c80189227",
    "dynamic-matrix-add-f64.mecb": "7355c52b197e169b87bd425a0e147f86dcad744d76f743e716223a23b2890924",
    "variadic-horzcat-f64.mecb": "f59cafbed042d33d8ab7ad2e1dc35eb8d25cc556f3d5af3479923c47abd7edaf",
    "string.mecb": "54a12c290f5539a774bb3793835416e1ae2d8314141b32f96360163f7b9631f0",
    "unary.mecb": "8bb13ad22de29f59160ec4ec51f7d04e9491069da710836468c16765487298f2",
    "ternary.mecb": "5362544462bdc214915c4ae41df632a4b03a189f79324f9e8af2fcdb14e6c575",
    "quaternary.mecb": "75a23eb4144efcbd6b28fae4ac534cff2dad581bf385cd2114f828f033422dca",
    "named-module-operation.mecb": "d155bc55b645b9ad23879a3a6233a0717f37f67570b5bf36e1e5a09c0b15833a",
    "cli-stdout.mecb": "295493d90b5b653bfee67763e57a5da60765edfdecd4926ec1227a5b6fc7ae57",
    "console.mecb": "9fe9eec36fc808891ff58720b388cdf51611f716a2e7f92e62384a46075cc07e",
    "time.mecb": "e1fad86d1a908f13477a96e5317596842f74a3d70637f38d7bd0e4ff1396638f",
    "timer.mecb": "608a2548ee229b03848a959e00ff4e8c314543d74fab8cdb2336fc4021c9fdb4",
    "scene.mecb": "3d014a27bf8a34a6af74c6a05b14be8656508c2bdbb540c708b1587ef653c30c",
    "robot-arm.mecb": "a68838c61b23cfe9555ae31c0075015c555e846aaf701f31bf6f783a20098385",
    "actor-host-function.mecb": "feaec635e1e33399fb4148ec7ec789373b065ca60161c995cbb61b71b2e7d917",
    "synthetic-live-read.mecb": "d4141713973607c9e02cbed6d67b5010022035a95bcd88a7218834606bd35609",
}
EXPECTED_FILES = [
    "canonical-scalars.mecb",
    "canonical-matrices.mecb",
    "canonical-composites.mecb",
    "literal-f64.mecb",
    "scalar-add-f64.mecb",
    "fixed-matrix-add-f64.mecb",
    "dynamic-matrix-add-f64.mecb",
    "variadic-horzcat-f64.mecb",
    "string.mecb",
    "unary.mecb",
    "ternary.mecb",
    "quaternary.mecb",
    "named-module-operation.mecb",
    "cli-stdout.mecb",
    "console.mecb",
    "time.mecb",
    "timer.mecb",
    "scene.mecb",
    "robot-arm.mecb",
    "actor-host-function.mecb",
    "synthetic-live-read.mecb",
]
SOURCE_DIRECTORY = "sources"
EXPECTED_SOURCE_FILES = [
    "actor-host-function.mec",
    "cli-stdout.mec",
    "console.mec",
    "dynamic-matrix-add-f64.mec",
    "fixed-matrix-add-f64.mec",
    "literal-f64.mec",
    "named-module-operation.mec",
    "quaternary.mec",
    "robot-arm.mec",
    "scalar-add.mec",
    "scene.mec",
    "string.mec",
    "synthetic-live-read.mec",
    "ternary.mec",
    "time.mec",
    "timer.mec",
    "unary.mec",
    "variadic-horzcat-f64.mec",
]
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
    raw_types: list[tuple[int, bytes]] = []
    for type_id in range(count):
        tag = reader.u16()
        flags = reader.u16()
        require(flags == 0, f"{name}: type {type_id} has nonzero flags")
        require(tag in TYPE_NAMES or 22 <= tag <= 32, f"{name}: unexpected runtime type tag {tag}")
        raw_types.append((tag, reader.read(reader.u32())))
    reader.finish()

    descriptions: list[dict[str, object] | None] = [None] * count
    resolving: set[int] = set()

    def child(type_id: int, parent: int) -> dict[str, object]:
        require(type_id < count, f"{name}: runtime type child is out of bounds")
        require(type_id < parent, f"{name}: runtime type child is not topological")
        return resolve(type_id)

    def named(tag: int, body: Reader) -> dict[str, object]:
        identifier = body.u64()
        value = body.string()
        require(value and mech_hash(value) == identifier, f"{name}: named runtime type has invalid identity")
        return {"kind": "enum" if tag == 23 else "atom", "id": identifier, "name": value}

    def fields(body: Reader, parent: int, label: str) -> list[dict[str, object]]:
        count = body.u32()
        require(count <= len(raw_types), f"{name}: {label} has an implausible field count")
        result: list[dict[str, object]] = []
        names: list[str] = []
        for _ in range(count):
            field_name = body.string()
            require(field_name, f"{name}: {label} has an empty field name")
            names.append(field_name)
            result.append({"name": field_name, "type": child(body.u32(), parent)})
        require(len(set(names)) == len(names), f"{name}: {label} has duplicate field names")
        return result

    def resolve(type_id: int) -> dict[str, object]:
        known = descriptions[type_id]
        if known is not None:
            return known
        require(type_id not in resolving, f"{name}: cyclic runtime type graph")
        resolving.add(type_id)
        tag, payload = raw_types[type_id]
        body = Reader(payload, f"{name}: type {type_id}")
        if tag in TYPE_NAMES:
            type_description: dict[str, object] = {"kind": TYPE_NAMES[tag]}
        elif tag == 22:
            element_id = body.u32()
            storage_id = body.u8()
            rows = body.u32()
            cols = body.u32()
            require(storage_id in MATRIX_STORAGE_NAMES, f"{name}: unknown matrix storage")
            require(
                valid_matrix_dimensions(storage_id, rows, cols),
                f"{name}: matrix storage and dimensions disagree",
            )
            type_description = {
                "cols": cols,
                "element": child(element_id, type_id),
                "kind": "matrix",
                "rows": rows,
                "storage": MATRIX_STORAGE_NAMES[storage_id],
                "storage_id": storage_id,
            }
        elif tag in {23, 26}:
            type_description = named(tag, body)
        elif tag == 24:
            type_description = {"kind": "record", "fields": fields(body, type_id, "record")}
        elif tag == 25:
            type_description = {
                "kind": "map",
                "key": child(body.u32(), type_id),
                "value": child(body.u32(), type_id),
            }
        elif tag == 27:
            columns = fields(body, type_id, "table")
            primary_key = body.u32()
            require(
                not columns or primary_key < len(columns),
                f"{name}: table primary key is out of range",
            )
            type_description = {
                "kind": "table",
                "columns": columns,
                "primary_key": primary_key,
            }
        elif tag == 28:
            item_count = body.u32()
            require(item_count <= len(raw_types), f"{name}: tuple has an implausible element count")
            type_description = {
                "kind": "tuple",
                "elements": [child(body.u32(), type_id) for _ in range(item_count)],
            }
        elif tag == 29:
            type_description = {"kind": "reference", "child": child(body.u32(), type_id)}
        elif tag == 30:
            element = child(body.u32(), type_id)
            has_max_len = body.u8()
            require(has_max_len in {0, 1}, f"{name}: set limit presence is invalid")
            type_description = {
                "kind": "set",
                "element": element,
                "max_len": body.u32() if has_max_len else None,
            }
        elif tag == 31:
            type_description = {"kind": "option", "child": child(body.u32(), type_id)}
        elif tag == 32:
            require(payload, f"{name}: Kind runtime type has an empty semantic kind")
            body.read(len(payload))
            type_description = {"kind": "kind"}
        else:
            raise ContractError(f"{name}: unexpected runtime type tag {tag}")
        body.finish()
        resolving.remove(type_id)
        descriptions[type_id] = type_description
        return type_description

    return [{"id": type_id, "type": resolve(type_id)} for type_id in range(count)]


def fixed_scalar_width(kind: str) -> int | None:
    widths = {
        "u8": 1,
        "i8": 1,
        "bool": 1,
        "u16": 2,
        "i16": 2,
        "u32": 4,
        "i32": 4,
        "f32": 4,
        "u64": 8,
        "i64": 8,
        "f64": 8,
        "id": 8,
        "index": 8,
        "u128": 16,
        "i128": 16,
        "c64": 16,
        "r64": 16,
    }
    return widths.get(kind)


def validate_constant_payload(
    runtime_type: dict[str, object], value: bytes, name: str, depth: int = 0
) -> None:
    require(depth <= 256, f"{name}: constant nesting exceeds bytecode v1 limit")
    kind = runtime_type.get("kind")
    require(isinstance(kind, str), f"{name}: malformed runtime type metadata")
    width = fixed_scalar_width(kind)
    if width is not None:
        require(len(value) == width, f"{name}: {kind} constant has an invalid byte length")
        if kind == "bool":
            require(value in {b"\x00", b"\x01"}, f"{name}: Bool constant is not canonical")
        if kind == "r64":
            numerator, denominator = struct.unpack("<qq", value)
            require(denominator > 0, f"{name}: rational denominator is not positive")
            require(
                __import__("math").gcd(abs(numerator), denominator) == 1,
                f"{name}: rational is not reduced",
            )
        return
    if kind == "string":
        try:
            value.decode("utf-8")
        except UnicodeDecodeError as error:
            raise ContractError(f"{name}: String constant is not UTF-8") from error
        return
    if kind in {"empty", "any", "none", "atom", "kind"}:
        require(not value, f"{name}: {kind} constant has payload bytes")
        return

    reader = Reader(value, f"{name}: {kind} constant")

    def typed_child(child_type: object, label: str) -> bytes:
        require(isinstance(child_type, dict), f"{name}: {label} has malformed child type")
        child_bytes = reader.read(reader.u32())
        validate_constant_payload(child_type, child_bytes, name, depth + 1)
        return child_bytes

    if kind == "matrix":
        rows = runtime_type.get("rows")
        cols = runtime_type.get("cols")
        element = runtime_type.get("element")
        require(isinstance(rows, int) and isinstance(cols, int), f"{name}: matrix dimensions are malformed")
        require(isinstance(element, dict), f"{name}: matrix element type is malformed")
        require((reader.u32(), reader.u32()) == (rows, cols), f"{name}: matrix dimensions disagree with its type")
        element_kind = element.get("kind")
        element_width = fixed_scalar_width(element_kind) if isinstance(element_kind, str) else None
        require(element_width is not None or element_kind == "string", f"{name}: unsupported matrix element type")
        for _ in range(rows * cols):
            if element_kind == "string":
                element_bytes = reader.read(reader.u32())
                validate_constant_payload(element, element_bytes, name, depth + 1)
            else:
                validate_constant_payload(element, reader.read(element_width), name, depth + 1)
    elif kind in {"tuple", "record"}:
        children = runtime_type.get("elements") if kind == "tuple" else runtime_type.get("fields")
        require(isinstance(children, list), f"{name}: {kind} schema is malformed")
        require(reader.u32() == len(children), f"{name}: {kind} item count disagrees with its type")
        for child in children:
            child_type = child if kind == "tuple" else child.get("type") if isinstance(child, dict) else None
            typed_child(child_type, kind)
    elif kind == "map":
        key_type = runtime_type.get("key")
        value_type = runtime_type.get("value")
        entries = reader.u32()
        pairs: list[tuple[bytes, bytes]] = []
        for _ in range(entries):
            key = typed_child(key_type, "map key")
            item = typed_child(value_type, "map value")
            pairs.append((key, item))
        require(pairs == sorted(pairs), f"{name}: map entries are not canonical")
        require(len({key for key, _ in pairs}) == len(pairs), f"{name}: map has duplicate keys")
    elif kind == "set":
        element = runtime_type.get("element")
        values = [typed_child(element, "set element") for _ in range(reader.u32())]
        max_len = runtime_type.get("max_len")
        require(max_len is None or len(values) <= max_len, f"{name}: set exceeds its maximum length")
        require(values == sorted(values) and len(set(values)) == len(values), f"{name}: set elements are not canonical")
    elif kind == "table":
        columns = runtime_type.get("columns")
        require(isinstance(columns, list), f"{name}: table schema is malformed")
        rows = reader.u32()
        require(reader.u32() == len(columns), f"{name}: table column count disagrees with its type")
        for _ in range(rows):
            for column in columns:
                child_type = column.get("type") if isinstance(column, dict) else None
                typed_child(child_type, "table cell")
    elif kind == "reference":
        typed_child(runtime_type.get("child"), "reference")
    elif kind == "option":
        present = reader.u8()
        require(present in {0, 1}, f"{name}: option presence tag is invalid")
        if present:
            typed_child(runtime_type.get("child"), "option")
    elif kind == "enum":
        variants = reader.u32()
        previous: tuple[int, str] | None = None
        for _ in range(variants):
            identifier = reader.u64()
            variant = reader.string()
            require(variant and mech_hash(variant) == identifier, f"{name}: enum variant identity is invalid")
            key = (identifier, variant)
            require(previous is None or previous < key, f"{name}: enum variants are not canonical")
            previous = key
            has_payload = reader.u8()
            require(has_payload in {0, 1}, f"{name}: enum payload presence tag is invalid")
            if has_payload:
                require(reader.read(reader.u32()), f"{name}: enum inline type is empty")
                require(reader.read(reader.u32()), f"{name}: enum payload is empty")
    else:
        raise ContractError(f"{name}: unsupported constant type {kind!r}")
    reader.finish()


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
        validate_constant_payload(runtime_type, value, name)
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
    expected_sha256 = EXPECTED_FIXTURE_SHA256.get(name)
    require(expected_sha256 is not None, f"{name}: fixture is not in the frozen corpus")
    origin = entry.get("origin")
    source = entry.get("source")
    source_file = entry.get("source_file")
    construction = entry.get("construction")
    if origin == "source-compiler":
        require(isinstance(source, str) and source, f"{name}: source is missing")
        require(
            isinstance(source_file, str) and source_file in EXPECTED_SOURCE_FILES,
            f"{name}: source file is missing or unexpected",
        )
        require(construction is None, f"{name}: source fixture has construction metadata")
        require(
            (CORPUS / SOURCE_DIRECTORY / source_file).read_text(encoding="utf-8") == source,
            f"{name}: source file disagrees with the manifest",
        )
    elif origin == "constructed-bytecode-program":
        require(source is None and source_file is None, f"{name}: constructed fixture has source metadata")
        require(
            isinstance(construction, str) and construction,
            f"{name}: construction description is missing",
        )
    else:
        raise ContractError(f"{name}: invalid fixture origin {origin!r}")

    require("expected_output" in entry, f"{name}: expected output is missing")
    native_plan_sha256 = entry.get("native_plan_sha256")
    require(
        isinstance(native_plan_sha256, str)
        and len(native_plan_sha256) == 64
        and all(character in "0123456789abcdef" for character in native_plan_sha256),
        f"{name}: native plan SHA-256 is malformed",
    )
    cargo_features = entry.get("cargo_features")
    require(isinstance(cargo_features, dict), f"{name}: Cargo feature metadata is missing")
    for package, features in cargo_features.items():
        require(isinstance(package, str) and package, f"{name}: Cargo feature package is malformed")
        require(
            isinstance(features, list)
            and all(isinstance(feature, str) and feature for feature in features)
            and features == sorted(set(features)),
            f"{name}: Cargo features for {package} are not sorted and unique",
        )
    packages = entry.get("packages")
    require(isinstance(packages, list), f"{name}: native package metadata is missing")
    package_names: list[str] = []
    for package in packages:
        require(isinstance(package, dict), f"{name}: native package metadata is malformed")
        package_name = package.get("package")
        crate_name = package.get("crate_name")
        package_features = package.get("cargo_features")
        package_source = package.get("source")
        require(
            isinstance(package_name, str) and package_name,
            f"{name}: native package name is malformed",
        )
        require(
            isinstance(crate_name, str) and crate_name,
            f"{name}: native crate name is malformed",
        )
        require(
            isinstance(package_features, list)
            and all(isinstance(feature, str) and feature for feature in package_features)
            and package_features == sorted(set(package_features)),
            f"{name}: native package features are not sorted and unique",
        )
        if package_name in cargo_features:
            require(
                package_features == cargo_features[package_name],
                f"{name}: native package features disagree with the feature map",
            )
        require(
            isinstance(package_source, dict)
            and package_source.get("source") in {"workspace", "registry"}
            and (
                isinstance(package_source.get("path"), str)
                or isinstance(package_source.get("version"), str)
            ),
            f"{name}: native package source is malformed",
        )
        package_names.append(package_name)
    require(
        package_names == sorted(set(package_names)),
        f"{name}: native packages are not sorted and unique",
    )

    path = CORPUS / name
    data = path.read_bytes()
    require(len(data) >= 292, f"{name}: file is shorter than the v1 envelope")
    observed_sha256 = hashlib.sha256(data).hexdigest()
    require(
        observed_sha256 == expected_sha256,
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
    require(ids == sorted(set(ids)), f"{name}: runtime function IDs are not sorted and unique")
    require(
        decoded_runtime_ids == set(ids),
        f"{name}: instruction runtime IDs disagree with manifest",
    )


def main() -> int:
    try:
        manifest_bytes = MANIFEST.read_bytes()
        require(
            hashlib.sha256(manifest_bytes).hexdigest() == EXPECTED_MANIFEST_SHA256,
            "frozen manifest SHA-256 changed",
        )
        manifest = json.loads(manifest_bytes)
        require(isinstance(manifest, dict), "manifest root must be an object")
        require(manifest.get("format") == "mech-bytecode-v1", "manifest format is stale")
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
            == sorted([*EXPECTED_FILES, MANIFEST.name, SOURCE_DIRECTORY]),
            "fixture corpus membership changed",
        )
        require(
            all(
                (path.name == SOURCE_DIRECTORY and path.is_dir() and not path.is_symlink())
                or (path.is_file() and not path.is_symlink())
                for path in disk_entries
            ),
            "fixture corpus entries must be regular files or the source directory",
        )
        source_entries = sorted((CORPUS / SOURCE_DIRECTORY).iterdir(), key=lambda path: path.name)
        require(
            [path.name for path in source_entries] == EXPECTED_SOURCE_FILES,
            "source corpus membership changed",
        )
        require(
            all(path.is_file() and not path.is_symlink() for path in source_entries),
            "source corpus entries must be regular files",
        )
        for entry in fixtures:
            validate_fixture(entry)
    except (ContractError, OSError, TypeError, ValueError, KeyError, struct.error) as error:
        print(f"bytecode v1 format contract failed: {error}", file=sys.stderr)
        return 1
    print(f"bytecode v1 format contract passed ({len(EXPECTED_FILES)} deterministic fixtures)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
