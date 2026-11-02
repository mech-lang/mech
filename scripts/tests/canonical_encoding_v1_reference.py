"""Test-only reference encoder for the committed MechSnapshotEncodingV1 vectors."""

from __future__ import annotations

import argparse
import copy
import functools
import hashlib
import json
import math
import struct
import sys
from pathlib import Path
from typing import Any


SCHEMA_TAGS = {
    "Bool": 0x01,
    "UnsignedInteger": 0x02,
    "SignedInteger": 0x03,
    "FloatingPoint": 0x04,
    "Complex": 0x05,
    "Rational": 0x06,
    "String": 0x07,
    "Id": 0x08,
    "Index": 0x09,
    "Atom": 0x0A,
    "Enum": 0x0B,
    "Option": 0x0C,
    "Tuple": 0x0D,
    "Record": 0x0E,
    "Matrix": 0x0F,
    "Table": 0x10,
    "Set": 0x11,
    "Map": 0x12,
    "ReifiedType": 0x13,
}
DIMENSION_TAGS = {
    "Constant": 0x01,
    "Parameter": 0x02,
    "Add": 0x03,
    "Multiply": 0x04,
    "Min": 0x05,
    "Max": 0x06,
}
KIND_TAGS = {
    "Wildcard": 0x01,
    "Never": 0x02,
    "Hole": 0x03,
    "Named": 0x04,
    "Id": 0x05,
    "Index": 0x06,
    "Atom": 0x07,
    "Enum": 0x08,
    "Matrix": 0x09,
    "Option": 0x0A,
    "Tuple": 0x0B,
    "Record": 0x0C,
    "Table": 0x0D,
    "Set": 0x0E,
    "Map": 0x0F,
    "Reference": 0x10,
    "TypeOf": 0x11,
}
LIFETIME_TAGS = {"Activation": 0x01, "Turn": 0x02}
MAX_U64 = (1 << 64) - 1


class EncodingError(ValueError):
    """A named MechSnapshotEncodingV1 canonicalization failure."""


def u8(value: int) -> bytes:
    return struct.pack("<B", value)


def u16(value: int) -> bytes:
    return struct.pack("<H", value)


def u32(value: int) -> bytes:
    return struct.pack("<I", value)


def u64(value: int) -> bytes:
    if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= MAX_U64:
        raise ValueError("U64 value is out of range")
    return struct.pack("<Q", value)


def i64(value: int) -> bytes:
    if not isinstance(value, int) or isinstance(value, bool) or not -(1 << 63) <= value < (1 << 63):
        raise ValueError("I64 value is out of range")
    return struct.pack("<q", value)


def utf8(value: str) -> bytes:
    encoded = value.encode("utf-8")
    return u64(len(encoded)) + encoded


def node(value: bytes) -> bytes:
    return u64(len(value)) + value


def canonical_nominal_path(path: list[str]) -> bytes:
    if not path or any(not part or part in {".", ".."} or "\0" in part for part in path):
        raise ValueError("invalid canonical nominal path")
    return u32(len(path)) + b"".join(utf8(part) for part in path)


def nominal_key(kind: str, path: list[str]) -> bytes:
    kind_tag = {"Atom": 0x01, "Enum": 0x02}[kind]
    return hashlib.sha256(
        b"mech-nominal-v1\0" + u8(kind_tag) + canonical_nominal_path(path)
    ).digest()


def encode_normalized_dimension(expression: dict[str, Any]) -> bytes:
    kind = expression["kind"]
    if kind == "Constant":
        return u8(DIMENSION_TAGS[kind]) + u64(expression["value"])
    if kind == "Parameter":
        return u8(DIMENSION_TAGS[kind]) + u32(expression["ordinal"])
    operands = [encode_normalized_dimension(item) for item in expression["operands"]]
    return u8(DIMENSION_TAGS[kind]) + u32(len(operands)) + b"".join(
        node(operand) for operand in operands
    )


def normalize_dimension(
    expression: dict[str, Any], parameter_count: int
) -> dict[str, Any]:
    kind = expression.get("kind")
    if kind == "Constant":
        value = expression.get("value")
        if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= MAX_U64:
            raise EncodingError("DimensionOverflowV1")
        return {"kind": "Constant", "value": value}
    if kind == "Parameter":
        ordinal = expression.get("ordinal")
        if not isinstance(ordinal, int) or isinstance(ordinal, bool) or not 0 <= ordinal < parameter_count:
            raise EncodingError("UnknownDimensionParameterV1")
        return {"kind": "Parameter", "ordinal": ordinal}
    if kind not in {"Add", "Multiply", "Min", "Max"}:
        raise ValueError(f"unknown dimension expression {kind}")

    operands: list[dict[str, Any]] = []
    for child in expression.get("operands", []):
        normalized = normalize_dimension(child, parameter_count)
        if normalized["kind"] == kind:
            operands.extend(normalized["operands"])
        else:
            operands.append(normalized)

    if kind in {"Add", "Multiply"}:
        identity = 0 if kind == "Add" else 1
        if kind == "Multiply" and any(
            operand["kind"] == "Constant" and operand["value"] == 0
            for operand in operands
        ):
            return {"kind": "Constant", "value": 0}
        constant = identity
        remaining: list[dict[str, Any]] = []
        for operand in operands:
            if operand["kind"] != "Constant":
                remaining.append(operand)
                continue
            value = operand["value"]
            folded = constant + value if kind == "Add" else constant * value
            if folded > MAX_U64:
                raise EncodingError("DimensionOverflowV1")
            constant = folded
        if constant != identity:
            remaining.append({"kind": "Constant", "value": constant})
        remaining.sort(key=encode_normalized_dimension)
        if not remaining:
            return {"kind": "Constant", "value": identity}
        if len(remaining) == 1:
            return remaining[0]
        return {"kind": kind, "operands": remaining}

    unique = {
        encode_normalized_dimension(operand): operand for operand in operands
    }
    ordered = [unique[key] for key in sorted(unique)]
    if not ordered:
        raise EncodingError("EmptyMinMaxV1")
    if len(ordered) == 1:
        return ordered[0]
    return {"kind": kind, "operands": ordered}


def dimension_expression(expression: dict[str, Any], parameter_count: int = 0) -> bytes:
    return encode_normalized_dimension(
        normalize_dimension(expression, parameter_count)
    )


def dimension_references(value: Any) -> list[int]:
    references: list[int] = []
    if isinstance(value, dict):
        if value.get("kind") == "Parameter" and isinstance(value.get("ordinal"), int):
            references.append(value["ordinal"])
        for child in value.values():
            references.extend(dimension_references(child))
    elif isinstance(value, list):
        for child in value:
            references.extend(dimension_references(child))
    return references


def rewrite_dimension_references(value: Any, ordinals: dict[int, int]) -> Any:
    if isinstance(value, dict):
        rewritten = {
            key: rewrite_dimension_references(child, ordinals)
            for key, child in value.items()
        }
        if value.get("kind") == "Parameter":
            old = value["ordinal"]
            if old not in ordinals:
                raise EncodingError("UnknownDimensionParameterV1")
            rewritten["ordinal"] = ordinals[old]
        return rewritten
    if isinstance(value, list):
        return [rewrite_dimension_references(child, ordinals) for child in value]
    return value


def normalize_parameter_environment(
    root: dict[str, Any], parameters: list[dict[str, Any]]
) -> tuple[dict[str, Any], list[dict[str, Any]], list[int]]:
    root_order = dimension_references(root)
    count = len(parameters)
    if any(ordinal < 0 or ordinal >= count for ordinal in root_order):
        raise EncodingError("UnknownDimensionParameterV1")
    dependencies: dict[int, list[int]] = {}
    for ordinal, parameter in enumerate(parameters):
        references = dimension_references(parameter.get("lower_bound", {}))
        references += dimension_references(parameter.get("upper_bound"))
        if any(reference < 0 or reference >= count for reference in references):
            raise EncodingError("UnknownDimensionParameterV1")
        dependencies[ordinal] = references

    state: dict[int, int] = {}
    occurrence_order: list[int] = []

    def visit(ordinal: int) -> None:
        if state.get(ordinal) == 1:
            raise EncodingError("CyclicDimensionParameterBoundsV1")
        if state.get(ordinal) == 2:
            return
        state[ordinal] = 1
        occurrence_order.append(ordinal)
        for dependency in dependencies[ordinal]:
            visit(dependency)
        state[ordinal] = 2

    for ordinal in root_order:
        visit(ordinal)
    reachable = set(occurrence_order)

    explicit = [
        ordinal
        for ordinal, parameter in enumerate(parameters)
        if parameter.get("explicit", True) and ordinal in reachable
    ]
    inferred_occurrence = [
        ordinal
        for ordinal in occurrence_order
        if not parameters[ordinal].get("explicit", True)
    ]
    retained = explicit + inferred_occurrence
    ordinals = {old: new for new, old in enumerate(retained)}
    for old in retained:
        if any(ordinals[dependency] >= ordinals[old] for dependency in dependencies[old]):
            raise EncodingError("ForwardDimensionParameterReferenceV1")
    rewritten_root = rewrite_dimension_references(copy.deepcopy(root), ordinals)
    rewritten_parameters: list[dict[str, Any]] = []
    for old in retained:
        parameter = parameters[old]
        rewritten = {
            "lifetime": parameter["lifetime"],
            "lower_bound": rewrite_dimension_references(
                copy.deepcopy(parameter["lower_bound"]), ordinals
            ),
        }
        if parameter.get("upper_bound") is not None:
            rewritten["upper_bound"] = rewrite_dimension_references(
                copy.deepcopy(parameter["upper_bound"]), ordinals
            )
        rewritten_parameters.append(rewritten)
    return rewritten_root, rewritten_parameters, retained


def evaluate_dimension(expression: dict[str, Any], values: list[int]) -> int:
    """Evaluate a normalized dimension with checked u64 arithmetic."""
    normalized = normalize_dimension(expression, len(values))
    kind = normalized["kind"]
    if kind == "Constant":
        return normalized["value"]
    if kind == "Parameter":
        ordinal = normalized["ordinal"]
        if ordinal >= len(values):
            raise EncodingError("UnknownDimensionParameterV1")
        return values[ordinal]
    operands = [evaluate_dimension(child, values) for child in normalized["operands"]]
    if kind == "Add":
        result = 0
        for operand in operands:
            result += operand
            if result > MAX_U64:
                raise EncodingError("DimensionOverflowV1")
        return result
    if kind == "Multiply":
        result = 1
        for operand in operands:
            result *= operand
            if result > MAX_U64:
                raise EncodingError("DimensionOverflowV1")
        return result
    if not operands:
        raise EncodingError("EmptyMinMaxV1")
    return min(operands) if kind == "Min" else max(operands)


def validate_shape(parameters: list[dict[str, Any]], values: list[int]) -> list[int]:
    """Validate values in canonical retained-parameter order."""
    if len(values) != len(parameters):
        raise EncodingError("ShapeParameterCountMismatchV1")
    resolved: list[int] = []
    for parameter, value in zip(parameters, values):
        if (
            not isinstance(value, int)
            or isinstance(value, bool)
            or not 0 <= value <= MAX_U64
        ):
            raise EncodingError("ShapeBoundViolationV1")
        lower = evaluate_dimension(parameter["lower_bound"], resolved)
        upper_expression = parameter.get("upper_bound")
        upper = (
            None
            if upper_expression is None
            else evaluate_dimension(upper_expression, resolved)
        )
        if value < lower or (upper is not None and value > upper):
            raise EncodingError("ShapeBoundViolationV1")
        resolved.append(value)
    return resolved


def validate_schema(
    schema: dict[str, Any], visiting: set[int] | None = None
) -> None:
    """Reject malformed widths, duplicate names, and recursive object graphs."""
    visiting = set() if visiting is None else visiting
    identity = id(schema)
    if identity in visiting:
        raise EncodingError("RecursiveSchemaUnsupportedV1")
    visiting.add(identity)
    try:
        kind = schema.get("kind")
        if kind in {"UnsignedInteger", "SignedInteger"}:
            if schema.get("bit_width") not in {8, 16, 32, 64, 128}:
                raise EncodingError("InvalidSchemaWidthV1")
        elif kind == "FloatingPoint":
            if schema.get("bit_width") not in {32, 64}:
                raise EncodingError("InvalidSchemaWidthV1")
        elif kind == "Complex":
            if schema.get("component_bit_width") not in {32, 64}:
                raise EncodingError("InvalidSchemaWidthV1")
        elif kind == "Rational":
            if (
                schema.get("numerator_width") != 64
                or schema.get("denominator_width") != 64
            ):
                raise EncodingError("InvalidSchemaWidthV1")

        named_children: list[dict[str, Any]] | None = None
        child_key = "schema"
        if kind == "Record":
            named_children = schema.get("fields", [])
        elif kind == "Table":
            named_children = schema.get("columns", [])
        elif kind == "Enum":
            named_children = schema.get("variants", [])
            child_key = "payload"
        if named_children is not None:
            names = [item.get("name") for item in named_children]
            if len(names) != len(set(names)):
                raise EncodingError("DuplicateSchemaNameV1")
            for item in named_children:
                child = item.get(child_key)
                if child is not None:
                    validate_schema(child, visiting)

        if kind == "Option":
            validate_schema(schema["element"], visiting)
        elif kind == "Tuple":
            for child in schema.get("elements", []):
                validate_schema(child, visiting)
        elif kind == "Matrix":
            validate_schema(schema["element"], visiting)
        elif kind == "Set":
            validate_schema(schema["element"], visiting)
            if not schema_is_keyable(schema["element"]):
                raise EncodingError("SchemaNotKeyableV1")
        elif kind == "Map":
            validate_schema(schema["key"], visiting)
            if not schema_is_keyable(schema["key"]):
                raise EncodingError("SchemaNotKeyableV1")
            validate_schema(schema["value"], visiting)
        elif kind not in SCHEMA_TAGS:
            raise ValueError(f"unknown schema {kind}")
    finally:
        visiting.remove(identity)


def dimension_parameters(parameters: list[dict[str, Any]]) -> bytes:
    encoded = bytearray()
    parameter_count = len(parameters)
    for parameter in parameters:
        encoded += u8(LIFETIME_TAGS[parameter["lifetime"]])
        encoded += node(dimension_expression(parameter["lower_bound"], parameter_count))
        upper = parameter.get("upper_bound")
        encoded += u8(0 if upper is None else 1)
        if upper is not None:
            encoded += node(dimension_expression(upper, parameter_count))
    return bytes(encoded)


def schema_body(schema: dict[str, Any], parameter_count: int = 0) -> bytes:
    kind = schema["kind"]
    tag = u8(SCHEMA_TAGS[kind])
    if kind in {"Bool", "String", "Id", "Index", "ReifiedType"}:
        return tag
    if kind in {"UnsignedInteger", "SignedInteger", "FloatingPoint"}:
        return tag + u16(schema["bit_width"])
    if kind == "Complex":
        return tag + u16(schema["component_bit_width"])
    if kind == "Rational":
        return tag + u16(schema["numerator_width"]) + u16(schema["denominator_width"])
    if kind in {"Atom", "Enum"}:
        encoded = bytearray(tag + nominal_key(kind, schema["nominal_path"]))
        if kind == "Enum":
            variants = schema["variants"]
            encoded += u32(len(variants))
            for variant in variants:
                encoded += utf8(variant["name"])
                payload = variant.get("payload")
                encoded += u8(0 if payload is None else 1)
                if payload is not None:
                    encoded += node(schema_body(payload, parameter_count))
        return bytes(encoded)
    if kind == "Option":
        return tag + node(schema_body(schema["element"], parameter_count))
    if kind == "Tuple":
        elements = schema["elements"]
        return tag + u32(len(elements)) + b"".join(
            node(schema_body(item, parameter_count)) for item in elements
        )
    if kind == "Record":
        fields = schema["fields"]
        return tag + u32(len(fields)) + b"".join(
            utf8(field["name"]) + node(schema_body(field["schema"], parameter_count))
            for field in fields
        )
    if kind == "Matrix":
        dimensions = schema["dimensions"]
        return (
            tag
            + node(schema_body(schema["element"], parameter_count))
            + u32(len(dimensions))
            + b"".join(
                node(dimension_expression(item, parameter_count))
                for item in dimensions
            )
        )
    if kind == "Table":
        columns = schema["columns"]
        return (
            tag
            + u32(len(columns))
            + b"".join(
                utf8(column["name"])
                + node(schema_body(column["schema"], parameter_count))
                for column in columns
            )
            + node(dimension_expression(schema["row_count"], parameter_count))
        )
    if kind == "Set":
        return tag + node(schema_body(schema["element"], parameter_count)) + node(
            dimension_expression(schema["cardinality"], parameter_count)
        )
    if kind == "Map":
        return (
            tag
            + node(schema_body(schema["key"], parameter_count))
            + node(schema_body(schema["value"], parameter_count))
            + node(dimension_expression(schema["cardinality"], parameter_count))
        )
    raise ValueError(f"unknown schema {kind}")


def schema_bytes(schema: dict[str, Any], parameters: list[dict[str, Any]]) -> bytes:
    validate_schema(schema)
    schema, parameters, _retained = normalize_parameter_environment(
        schema, parameters
    )
    return u8(0x01) + u32(len(parameters)) + dimension_parameters(parameters) + node(
        schema_body(schema, len(parameters))
    )


def schema_key(schema: dict[str, Any], parameters: list[dict[str, Any]]) -> bytes:
    return hashlib.sha256(b"mech-schema-v1\0" + schema_bytes(schema, parameters)).digest()


def shape_bytes(values: list[int]) -> bytes:
    return u8(0x01) + u32(len(values)) + b"".join(u64(value) for value in values)


def kind_body(kind_expression: dict[str, Any], parameter_count: int = 0) -> bytes:
    kind = kind_expression["kind"]
    tag = u8(KIND_TAGS[kind])
    if kind in {"Wildcard", "Never", "Hole", "Id", "Index"}:
        return tag
    if kind == "Named":
        return tag + canonical_nominal_path(kind_expression["nominal_path"])
    if kind in {"Atom", "Enum"}:
        return tag + nominal_key(kind, kind_expression["nominal_path"])
    if kind in {"Option", "Reference", "TypeOf"}:
        return tag + node(kind_body(kind_expression["element"], parameter_count))
    if kind == "Tuple":
        elements = kind_expression["elements"]
        return tag + u32(len(elements)) + b"".join(
            node(kind_body(item, parameter_count)) for item in elements
        )
    if kind == "Record":
        fields = kind_expression["fields"]
        return tag + u32(len(fields)) + b"".join(
            utf8(field["name"])
            + node(kind_body(field["kind_expression"], parameter_count))
            for field in fields
        )
    if kind == "Matrix":
        dimensions = kind_expression["dimensions"]
        return (
            tag
            + node(kind_body(kind_expression["element"], parameter_count))
            + u32(len(dimensions))
            + b"".join(
                node(dimension_expression(item, parameter_count))
                for item in dimensions
            )
        )
    if kind == "Table":
        columns = kind_expression["columns"]
        return (
            tag
            + u32(len(columns))
            + b"".join(
                utf8(column["name"])
                + node(kind_body(column["kind_expression"], parameter_count))
                for column in columns
            )
            + node(
                dimension_expression(kind_expression["row_count"], parameter_count)
            )
        )
    if kind == "Set":
        return tag + node(
            kind_body(kind_expression["element"], parameter_count)
        ) + node(
            dimension_expression(
                kind_expression["cardinality"], parameter_count
            )
        )
    if kind == "Map":
        return (
            tag
            + node(kind_body(kind_expression["key"], parameter_count))
            + node(kind_body(kind_expression["value"], parameter_count))
            + node(
                dimension_expression(
                    kind_expression["cardinality"], parameter_count
                )
            )
        )
    raise ValueError(f"unknown kind expression {kind}")


def kind_expression_bytes(
    kind_expression: dict[str, Any], parameters: list[dict[str, Any]]
) -> bytes:
    kind_expression, parameters, _retained = normalize_parameter_environment(
        kind_expression, parameters
    )
    return u8(0x01) + u32(len(parameters)) + dimension_parameters(parameters) + node(
        kind_body(kind_expression, len(parameters))
    )


def reified_type_payload(payload: dict[str, Any]) -> bytes:
    if payload["kind"] == "KindExpr":
        encoded = kind_expression_bytes(
            payload["kind_expression"], payload.get("dimension_parameters", [])
        )
        return u8(0x01) + node(encoded)
    if payload["kind"] == "Schema":
        return u8(0x02) + schema_key(
            payload["schema"], payload.get("dimension_parameters", [])
        )
    raise ValueError("unknown reified type payload")


def float_bits(schema: dict[str, Any], value: Any) -> int:
    width = schema["bit_width"]
    if isinstance(value, dict) and "bits_hex" in value:
        bits = int(value["bits_hex"], 16)
        if bits >= 1 << width:
            raise ValueError("floating-point bits exceed schema width")
        return bits
    packed = struct.pack("<f" if width == 32 else "<d", float(value))
    return int.from_bytes(packed, "little")


def exact_float_payload(schema: dict[str, Any], value: Any) -> bytes:
    width = schema["bit_width"]
    return float_bits(schema, value).to_bytes(width // 8, "little")


def canonical_float_bits(width: int, bits: int) -> int:
    if width == 32:
        sign = 1 << 31
        exponent = 0x7F800000
        fraction = 0x007FFFFF
        canonical_nan = 0x7FC00000
    elif width == 64:
        sign = 1 << 63
        exponent = 0x7FF0000000000000
        fraction = 0x000FFFFFFFFFFFFF
        canonical_nan = 0x7FF8000000000000
    else:
        raise ValueError("unsupported floating-point width")
    if bits & ~sign == 0:
        return 0
    if bits & exponent == exponent and bits & fraction:
        return canonical_nan
    return bits


def rational_parts(value: dict[str, Any]) -> tuple[int, int]:
    numerator = value["numerator"]
    denominator = value["denominator"]
    if (
        not isinstance(numerator, int)
        or isinstance(numerator, bool)
        or not -(1 << 63) <= numerator < (1 << 63)
        or not isinstance(denominator, int)
        or isinstance(denominator, bool)
        or not 0 < denominator <= MAX_U64
        or math.gcd(abs(numerator), denominator) != 1
        or (numerator == 0 and denominator != 1)
    ):
        raise EncodingError("NonCanonicalRationalV1")
    return numerator, denominator


def canonical_payload(
    schema: dict[str, Any], value: Any, shape_values: list[int] | None = None
) -> bytes:
    shape_values = [] if shape_values is None else shape_values
    kind = schema["kind"]
    if kind == "Bool":
        if not isinstance(value, bool):
            raise ValueError("Bool payload must be boolean")
        return u8(1 if value else 0)
    if kind in {"UnsignedInteger", "SignedInteger"}:
        width = schema["bit_width"] // 8
        if not isinstance(value, int) or isinstance(value, bool):
            raise ValueError("integer payload must be integer")
        return value.to_bytes(width, "little", signed=kind == "SignedInteger")
    if kind == "FloatingPoint":
        return exact_float_payload(schema, value)
    if kind == "Complex":
        component = {
            "kind": "FloatingPoint",
            "bit_width": schema["component_bit_width"],
        }
        return exact_float_payload(component, value["real"]) + exact_float_payload(
            component, value["imaginary"]
        )
    if kind == "Rational":
        numerator, denominator = rational_parts(value)
        return i64(numerator) + u64(denominator)
    if kind == "String":
        return utf8(value)
    if kind in {"Id", "Index"}:
        return u64(value)
    if kind == "Atom":
        return b""
    if kind == "Option":
        if value is None:
            return u8(0)
        if not isinstance(value, dict):
            raise EncodingError("PayloadCardinalityMismatchV1")
        if value == {"present": False}:
            return u8(0)
        if set(value) != {"present", "value"} or value.get("present") is not True:
            raise EncodingError("PayloadCardinalityMismatchV1")
        return u8(1) + canonical_payload(
            schema["element"], value["value"], shape_values
        )
    if kind == "Enum":
        if not isinstance(value, dict) or "ordinal" not in value:
            raise EncodingError("EnumOrdinalOutOfRangeV1")
        ordinal = value["ordinal"]
        if (
            not isinstance(ordinal, int)
            or isinstance(ordinal, bool)
            or not 0 <= ordinal < len(schema["variants"])
        ):
            raise EncodingError("EnumOrdinalOutOfRangeV1")
        variant = schema["variants"][ordinal]
        payload = variant.get("payload")
        expected_fields = {"ordinal"} if payload is None else {"ordinal", "payload"}
        if set(value) != expected_fields:
            raise EncodingError("EnumPayloadMismatchV1")
        return u32(ordinal) + (
            b""
            if payload is None
            else canonical_payload(payload, value["payload"], shape_values)
        )
    if kind == "Tuple":
        if not isinstance(value, (list, tuple)) or len(value) != len(
            schema["elements"]
        ):
            raise EncodingError("AggregateArityMismatchV1")
        return b"".join(
            canonical_payload(child, item, shape_values)
            for child, item in zip(schema["elements"], value)
        )
    if kind == "Record":
        if not isinstance(value, dict) or set(value) != {
            field["name"] for field in schema["fields"]
        }:
            raise EncodingError("AggregateFieldMismatchV1")
        return b"".join(
            canonical_payload(
                field["schema"], value[field["name"]], shape_values
            )
            for field in schema["fields"]
        )
    if kind == "Matrix":
        expected = 1
        for expression in schema["dimensions"]:
            expected *= evaluate_dimension(expression, shape_values)
            if expected > MAX_U64:
                raise EncodingError("DimensionOverflowV1")
        if not isinstance(value, list) or len(value) != expected:
            raise EncodingError("PayloadCardinalityMismatchV1")
        return b"".join(
            canonical_payload(schema["element"], item, shape_values)
            for item in value
        )
    if kind == "Table":
        if not isinstance(value, dict) or set(value) != {
            column["name"] for column in schema["columns"]
        }:
            raise EncodingError("AggregateFieldMismatchV1")
        expected = evaluate_dimension(schema["row_count"], shape_values)
        if any(
            not isinstance(value[column["name"]], list)
            or len(value[column["name"]]) != expected
            for column in schema["columns"]
        ):
            raise EncodingError("PayloadCardinalityMismatchV1")
        return b"".join(
            canonical_payload(column["schema"], item, shape_values)
            for column in schema["columns"]
            for item in value[column["name"]]
        )
    if kind == "Set":
        expected = evaluate_dimension(schema["cardinality"], shape_values)
        if not isinstance(value, list) or len(value) != expected:
            raise EncodingError("PayloadCardinalityMismatchV1")
        return encode_set(schema["element"], value)
    if kind == "Map":
        expected = evaluate_dimension(schema["cardinality"], shape_values)
        if not isinstance(value, list) or len(value) != expected:
            raise EncodingError("PayloadCardinalityMismatchV1")
        if any(not isinstance(entry, list) or len(entry) != 2 for entry in value):
            raise EncodingError("MapEntryArityMismatchV1")
        return encode_map(schema["key"], schema["value"], value, shape_values)
    if kind == "ReifiedType":
        return reified_type_payload(value)
    raise ValueError(f"unknown payload schema {kind}")


def schema_is_keyable(schema: dict[str, Any]) -> bool:
    kind = schema["kind"]
    if kind in {
        "Bool",
        "UnsignedInteger",
        "SignedInteger",
        "FloatingPoint",
        "Rational",
        "String",
        "Id",
        "Index",
        "Atom",
    }:
        return True
    if kind == "Enum":
        return all(
            variant.get("payload") is None
            or schema_is_keyable(variant["payload"])
            for variant in schema["variants"]
        )
    if kind == "Option":
        return schema_is_keyable(schema["element"])
    if kind == "Tuple":
        return all(schema_is_keyable(item) for item in schema["elements"])
    if kind == "Record":
        return all(schema_is_keyable(field["schema"]) for field in schema["fields"])
    return False


def canonical_key_payload(schema: dict[str, Any], value: Any) -> bytes:
    if not schema_is_keyable(schema):
        raise EncodingError("SchemaNotKeyableV1")
    kind = schema["kind"]
    if kind == "FloatingPoint":
        width = schema["bit_width"]
        bits = canonical_float_bits(width, float_bits(schema, value))
        return bits.to_bytes(width // 8, "little")
    if kind == "Option":
        if value is None:
            return u8(0)
        if not isinstance(value, dict):
            raise EncodingError("PayloadCardinalityMismatchV1")
        if value == {"present": False}:
            return u8(0)
        if set(value) != {"present", "value"} or value.get("present") is not True:
            raise EncodingError("PayloadCardinalityMismatchV1")
        return u8(1) + canonical_key_payload(schema["element"], value["value"])
    if kind == "Enum":
        if not isinstance(value, dict) or "ordinal" not in value:
            raise EncodingError("EnumOrdinalOutOfRangeV1")
        ordinal = value["ordinal"]
        if (
            not isinstance(ordinal, int)
            or isinstance(ordinal, bool)
            or not 0 <= ordinal < len(schema["variants"])
        ):
            raise EncodingError("EnumOrdinalOutOfRangeV1")
        payload = schema["variants"][ordinal].get("payload")
        expected_fields = {"ordinal"} if payload is None else {"ordinal", "payload"}
        if set(value) != expected_fields:
            raise EncodingError("EnumPayloadMismatchV1")
        return u32(ordinal) + (
            b"" if payload is None else canonical_key_payload(payload, value["payload"])
        )
    if kind == "Tuple":
        if not isinstance(value, (list, tuple)) or len(value) != len(
            schema["elements"]
        ):
            raise EncodingError("AggregateArityMismatchV1")
        return b"".join(
            canonical_key_payload(child, item)
            for child, item in zip(schema["elements"], value)
        )
    if kind == "Record":
        if not isinstance(value, dict) or set(value) != {
            field["name"] for field in schema["fields"]
        }:
            raise EncodingError("AggregateFieldMismatchV1")
        return b"".join(
            canonical_key_payload(field["schema"], value[field["name"]])
            for field in schema["fields"]
        )
    return canonical_payload(schema, value)


def compare_keys(schema: dict[str, Any], left: Any, right: Any) -> int:
    # Validate both aggregates with the same canonical path used for encoding
    # before structural comparison; no suffix or field may disappear via zip.
    canonical_key_payload(schema, left)
    canonical_key_payload(schema, right)
    kind = schema["kind"]
    if kind == "FloatingPoint":
        width = schema["bit_width"]
        sign = 1 << (width - 1)
        mask = (1 << width) - 1

        def ordered(value: Any) -> int:
            bits = canonical_float_bits(width, float_bits(schema, value))
            return (~bits & mask) if bits & sign else bits | sign

        a, b = ordered(left), ordered(right)
    elif kind == "Rational":
        n1, d1 = rational_parts(left)
        n2, d2 = rational_parts(right)
        a, b = n1 * d2, n2 * d1
    elif kind == "Bool":
        a, b = int(left), int(right)
    elif kind in {"UnsignedInteger", "SignedInteger", "Id", "Index"}:
        a, b = left, right
    elif kind == "String":
        a, b = left.encode("utf-8"), right.encode("utf-8")
    elif kind == "Option":
        left_present = left is not None and left.get("present") is not False
        right_present = right is not None and right.get("present") is not False
        if left_present != right_present:
            return 1 if left_present else -1
        if not left_present:
            return 0
        return compare_keys(schema["element"], left["value"], right["value"])
    elif kind == "Enum":
        if left["ordinal"] != right["ordinal"]:
            return -1 if left["ordinal"] < right["ordinal"] else 1
        payload = schema["variants"][left["ordinal"]].get("payload")
        if payload is None:
            return 0
        return compare_keys(payload, left["payload"], right["payload"])
    elif kind == "Tuple":
        for child, left_item, right_item in zip(schema["elements"], left, right):
            order = compare_keys(child, left_item, right_item)
            if order:
                return order
        return 0
    elif kind == "Record":
        for field in schema["fields"]:
            order = compare_keys(
                field["schema"], left[field["name"]], right[field["name"]]
            )
            if order:
                return order
        return 0
    else:
        a, b = (
            canonical_key_payload(schema, left),
            canonical_key_payload(schema, right),
        )
    return (a > b) - (a < b)


def encode_set(element_schema: dict[str, Any], values: list[Any]) -> bytes:
    ordered = sorted(
        values,
        key=functools.cmp_to_key(
            lambda left, right: compare_keys(element_schema, left, right)
        ),
    )
    payloads = [canonical_key_payload(element_schema, value) for value in ordered]
    if len(set(payloads)) != len(payloads):
        raise EncodingError("DuplicateCanonicalKeyV1")
    return u64(len(payloads)) + b"".join(payloads)


def encode_map(
    key_schema: dict[str, Any],
    value_schema: dict[str, Any],
    entries: list[list[Any]],
    shape_values: list[int] | None = None,
) -> bytes:
    shape_values = [] if shape_values is None else shape_values
    ordered = sorted(
        entries,
        key=functools.cmp_to_key(
            lambda left, right: compare_keys(key_schema, left[0], right[0])
        ),
    )
    key_payloads = [canonical_key_payload(key_schema, entry[0]) for entry in ordered]
    if len(set(key_payloads)) != len(key_payloads):
        raise EncodingError("DuplicateCanonicalKeyV1")
    return u64(len(ordered)) + b"".join(
        key_payload + canonical_payload(value_schema, entry[1], shape_values)
        for key_payload, entry in zip(key_payloads, ordered)
    )


def value_hash(
    schema: dict[str, Any],
    parameters: list[dict[str, Any]],
    resolved_shape: list[int],
    payload: bytes,
) -> bytes:
    return hashlib.sha256(
        b"mech-value-v1\0"
        + schema_key(schema, parameters)
        + shape_bytes(resolved_shape)
        + payload
    ).digest()


def key_hash(
    schema: dict[str, Any],
    parameters: list[dict[str, Any]],
    resolved_shape: list[int],
    payload: bytes,
) -> bytes:
    return hashlib.sha256(
        b"mech-key-v1\0"
        + schema_key(schema, parameters)
        + shape_bytes(resolved_shape)
        + payload
    ).digest()


def reproduce_value(vector: dict[str, Any]) -> dict[str, str]:
    source = vector["input"]
    parameters = source.get("dimension_parameters", [])
    schema = source["schema"]
    validate_schema(schema)
    normalized_schema, normalized_parameters, _retained = normalize_parameter_environment(
        schema, parameters
    )
    source_shape = source.get("shape_values", [])
    resolved_shape = validate_shape(normalized_parameters, source_shape)
    payload = canonical_payload(normalized_schema, source["value"], resolved_shape)
    return {
        "schema_hex": schema_bytes(schema, parameters).hex(),
        "schema_key_hex": schema_key(schema, parameters).hex(),
        "shape_hex": shape_bytes(resolved_shape).hex(),
        "payload_hex": payload.hex(),
        "value_hash_hex": value_hash(
            schema, parameters, resolved_shape, payload
        ).hex(),
    }


def reproduce_key(vector: dict[str, Any]) -> dict[str, Any]:
    source = vector["input"]
    schema = source["schema"]
    parameters = source.get("dimension_parameters", [])
    shape = source.get("shape_values", [])
    validate_schema(schema)
    normalized_schema, normalized_parameters, _retained = normalize_parameter_environment(
        schema, parameters
    )
    resolved_shape = validate_shape(normalized_parameters, shape)
    identifier = vector["id"]
    try:
        if identifier in {"f64-signed-zero-equivalence", "f64-nan-equivalence"}:
            payloads = [
                canonical_key_payload(normalized_schema, value)
                for value in source["values"]
            ]
            hashes = [
                key_hash(schema, parameters, resolved_shape, payload)
                for payload in payloads
            ]
            return {
                "canonical_key_payload_hex": payloads[0].hex(),
                "key_hash_hex": hashes[0].hex(),
                "equivalent": payloads[0] == payloads[1] and hashes[0] == hashes[1],
            }
        if identifier == "duplicate-canonical-set-keys":
            encode_set(schema, source["values"])
        elif identifier == "rational64-order":
            order = compare_keys(schema, source["left"], source["right"])
            return {"order": {-1: "Less", 0: "Equal", 1: "Greater"}[order]}
        elif identifier == "complex-not-keyable":
            canonical_key_payload(schema, source["value"])
    except EncodingError as error:
        return {"error": str(error)}
    raise ValueError(f"key vector {identifier} did not produce a result")


def reproduce_dimension(vector: dict[str, Any]) -> dict[str, str]:
    source = vector["input"]
    try:
        encoded = dimension_expression(
            source["expression"], source.get("parameter_count", 0)
        )
        return {"normalized_hex": encoded.hex()}
    except EncodingError as error:
        return {"error": str(error)}


def reproduce_invalid_value(vector: dict[str, Any]) -> dict[str, str]:
    try:
        reproduce_value(vector)
    except EncodingError as error:
        return {"error": str(error)}
    return {"error": "AcceptedInvalidValueV1"}


def reproduce_invalid_schema(vector: dict[str, Any]) -> dict[str, str]:
    source = vector["input"]
    try:
        schema_bytes(source["schema"], source.get("dimension_parameters", []))
    except EncodingError as error:
        return {"error": str(error)}
    return {"error": "AcceptedInvalidSchemaV1"}


def verify_vectors(path: Path) -> list[str]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    failures: list[str] = []
    groups = (
        ("value_vectors", reproduce_value),
        ("key_vectors", reproduce_key),
        ("dimension_vectors", reproduce_dimension),
        ("invalid_value_vectors", reproduce_invalid_value),
        ("invalid_schema_vectors", reproduce_invalid_schema),
    )
    for group, reproduce in groups:
        for vector in payload[group]:
            actual = reproduce(vector)
            expected = vector["expected"]
            if actual != expected:
                failures.append(
                    f"{vector['id']}: expected {expected!r}, reproduced {actual!r}"
                )
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    if not args.check:
        parser.error("--check is required")
    failures = verify_vectors(args.path)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("canonical encoding vectors passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
