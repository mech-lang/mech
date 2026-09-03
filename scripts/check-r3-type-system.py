#!/usr/bin/env python3
"""Enforce the permanent R3 Type System v1 architecture."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Pattern


ROOT = Path(__file__).resolve().parents[1]
TYPE_MODULES = (
    "src/core/src/type_system/mod.rs",
    "src/core/src/type_system/builtin.rs",
    "src/core/src/type_system/resolved.rs",
    "src/core/src/type_system/solver.rs",
    "src/core/src/type_system/conversion.rs",
    "src/core/src/type_system/scheme.rs",
    "src/core/src/type_system/diagnostic.rs",
)
REQUIRED = TYPE_MODULES + (
    "src/core/src/lib.rs",
    "src/core/src/kind_scheme.rs",
    "src/core/src/cell_binding.rs",
    "src/core/src/function/catalog.rs",
    "src/core/src/function/specialization.rs",
    "src/engine/src/expressions/formulas.rs",
    "src/engine/src/expressions/errors.rs",
    "src/engine/src/function/resolver.rs",
    "src/engine/src/literals.rs",
    "src/engine/src/resident/conversion.rs",
    "src/runtime/src/runtime/program/tests.rs",
    "src/core/tests/type_system_builtin.rs",
    "src/core/tests/type_system_solver.rs",
    "src/core/tests/type_system_conversion.rs",
    "src/core/tests/type_system_catalog.rs",
    "src/stdlib/tests/type_system_source.rs",
    "docs/design/type-system-v1.md",
    "docs/design/type-memory-boundary.md",
    "docs/design/ROADMAP.mec",
    "docs/design/v0.4-endgame.md",
    "README.md",
    ".github/workflows/ci.yml",
    ".github/workflows/ci-full.yml",
    ".github/ci/owners.toml",
)
BUILTINS = (
    ("U8", 0), ("U16", 1), ("U32", 2), ("U64", 3), ("U128", 4),
    ("I8", 5), ("I16", 6), ("I32", 7), ("I64", 8), ("I128", 9),
    ("F32", 10), ("F64", 11), ("C64", 12), ("R64", 13),
    ("String", 14), ("Bool", 15), ("C32", 16),
)
PREDICATES = (
    "Number", "Real", "Integer", "FloatingPoint", "Ordered",
    "Negatable", "RangeEndpoint", "Equatable", "Keyable",
)
CONFORMANCE = (
    "existing_builtin_ordinals_are_unchanged",
    "c32_and_c64_are_distinct",
    "number_contains_every_numeric_scalar",
    "square_scheme_rejects_independent_equal_bounded_axes",
    "imported_rigid_dimensions_are_never_aliased",
    "activation_dimension_rejects_compound_turn_expression",
    "bounded_turn_rejects_unbounded_turn_expression",
    "dimension_bounds_are_checked_for_compound_expressions",
    "cyclic_kind_binding_is_structured",
    "cyclic_dimension_binding_is_structured",
    "unresolved_outputs_are_structured",
    "normalization_is_idempotent",
    "complete_numeric_promotion_matrix_is_symmetric_and_deterministic",
    "implicit_conversion_is_exactly_the_lossless_table",
    "matrix_and_option_plans_preserve_structure",
    "exact_concrete_beats_generic",
    "different_equal_score_outputs_produce_ambiguity",
    "named_specializers_are_scheme_authoritative",
    "schema_and_direct_kind_predicates_agree_for_structural_products",
    "dimension_inequality_requires_guaranteed_interval_endpoints",
    "dynamic_lower_bound_requires_actual_minimum_above_declared_maximum",
    "matrix_product_preserves_outer_axes_and_rejects_fixed_inner_mismatch",
    "concatenation_templates_accept_more_than_thirty_two_inputs",
    "set_definition_cardinality_and_keyability_are_semantic",
    "source_numeric_promotions_are_semantically_selected",
    "semantic_formula_add_routes_strings_and_numbers",
    "source_explicit_casts_use_checked_conversion_plans",
    "source_variadic_templates_cover_large_concat_and_exact_sets",
    "compiled_conversion_executes_after_bytecode_round_trip",
)
STORAGE_FORBIDDEN = (
    "FunctionValueRepresentation", "FunctionRuntimeType",
    "FunctionMatrixRepresentation", "FunctionMatrixStoragePattern",
    "FunctionMatrixElement", "RuntimeFunctionSignature", "RuntimeFunctionEntry",
    "MechFunctionFactory", "ValueCell", "CanonicalCellId",
    "StorageCapabilityDescriptor", "same_storage",
)
WIRE_ROOTS = (
    "src/core/src/program/bytecode", "src/core/src/schema/encoding.rs",
    "src/core/src/operation_contract/encoding.rs", "src/bytecode",
    "src/engine/src/artifact", "src/abi",
)
R3_WIRE_NAMES = (
    "ResolvedCall", "ConversionPlan", "ConversionStep", "TypeConstraintEnvironment",
    "KindPredicateEvidence", "BuiltinKindPredicate",
)
RAW_LITERAL = re.compile(r'(?:br|rb|r)(?P<hashes>#{0,255})"')


def rust_code(source: str) -> str:
    """Blank Rust comments and literals while preserving offsets and newlines."""
    out = list(source)
    size = len(source)

    def blank(start: int, end: int) -> None:
        for index in range(start, end):
            if out[index] not in "\r\n":
                out[index] = " "

    index = 0
    while index < size:
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = size if end < 0 else end
            blank(index, end)
            index = end
            continue
        if source.startswith("/*", index):
            depth, end = 1, index + 2
            while end < size and depth:
                if source.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif source.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
            blank(index, end)
            index = end
            continue
        raw = RAW_LITERAL.match(source, index)
        if raw:
            delimiter = '"' + raw.group("hashes")
            end = source.find(delimiter, raw.end())
            end = size if end < 0 else end + len(delimiter)
            blank(index, end)
            index = end
            continue
        prefix = 1 if source.startswith(('b"', "b'"), index) else 0
        quote = index + prefix
        if quote < size and source[quote] == '"':
            end, escaped = quote + 1, False
            while end < size:
                char = source[end]
                end += 1
                if char == '"' and not escaped:
                    break
                escaped = char == "\\" and not escaped
                if char != "\\":
                    escaped = False
            blank(index, end)
            index = end
            continue
        if quote < size and source[quote] == "'":
            value, end = quote + 1, quote + 2
            if value < size and source[value] == "\\":
                end = value + (4 if source.startswith("\\x", value) else 2)
            if end < size and source[end] == "'":
                blank(index, end + 1)
                index = end + 1
                continue
        index += 1
    return "".join(out)


def _brace_end(code: str, opening: int) -> int | None:
    depth = 0
    for index in range(opening, len(code)):
        if code[index] == "{":
            depth += 1
        elif code[index] == "}":
            depth -= 1
            if depth == 0:
                return index
    return None


def extract_item_body(source: str, pattern: Pattern[str]) -> str | None:
    code = rust_code(source)
    match = pattern.search(code)
    if match is None:
        return None
    opening = code.find("{", match.end())
    if opening < 0:
        return None
    end = _brace_end(code, opening)
    return None if end is None else code[opening + 1:end]


def extract_item_body_raw(source: str, pattern: Pattern[str]) -> str | None:
    """Extract an item with comments/literals preserved after structural matching."""
    code = rust_code(source)
    match = pattern.search(code)
    if match is None:
        return None
    opening = code.find("{", match.end())
    if opening < 0:
        return None
    end = _brace_end(code, opening)
    return None if end is None else source[opening + 1:end]


def strip_cfg_test_modules(source: str) -> str:
    code = rust_code(source)
    out = list(code)
    cursor = 0
    while True:
        start = code.find("#[", cursor)
        if start < 0:
            break
        close = code.find("]", start + 2)
        if close < 0:
            break
        attribute = code[start:close + 1]
        cursor = close + 1
        if not re.search(r"\bcfg\b", attribute) or not re.search(r"\btest\b", attribute):
            continue
        module = re.match(r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{", code[close + 1:])
        if module is None:
            continue
        opening = code.find("{", close + 1, close + 1 + module.end())
        end = _brace_end(code, opening)
        if end is None:
            break
        for index in range(start, end + 1):
            if out[index] not in "\r\n":
                out[index] = " "
        cursor = end + 1
    return "".join(out)


def _read(root: Path, relative: str, found: list[str]) -> str:
    path = root / relative
    if not path.is_file():
        found.append(f"required file is missing: {relative}")
        return ""
    return path.read_text(encoding="utf-8")


def _rust_files(root: Path, relative: str):
    path = root / relative
    if path.is_file() and path.suffix == ".rs":
        yield path
    elif path.is_dir():
        yield from path.rglob("*.rs")


def _job(source: str, name: str) -> str:
    match = re.search(rf"(?ms)^  {re.escape(name)}:\n(.*?)(?=^  [a-z0-9][a-z0-9-]*:\n|\Z)", source)
    return "" if match is None else match.group(1)


def failures(root: Path) -> list[str]:
    root = root.resolve()
    found: list[str] = []
    sources = {relative: _read(root, relative, found) for relative in REQUIRED}

    lib = rust_code(sources["src/core/src/lib.rs"])
    if not re.search(r"\bpub\s+mod\s+type_system\s*;", lib):
        found.append("type_system is not a public core module")
    if not re.search(r"\bpub\s+use\s+self::type_system::\*\s*;", lib):
        found.append("core does not publicly re-export type_system")
    if re.search(r"\bpub\s+mod\s+type_solver\b", lib):
        found.append("retired public type_solver module remains")

    module = rust_code(sources[TYPE_MODULES[0]])
    for name in ("builtin", "resolved", "solver", "conversion", "scheme", "diagnostic"):
        if not re.search(rf"\bpub\s+mod\s+{name}\s*;", module):
            found.append(f"type_system does not declare {name}")
        if not re.search(rf"\bpub\s+use\s+self::{name}::\*\s*;", module):
            found.append(f"type_system does not re-export {name}")

    target_pattern = re.compile(r"\b(?:Resident\w*|Gpu\w*|GPU\w*|NativeLayout\w*|Allocation\w*)\b")
    for path in _rust_files(root, "src/core/src/type_system"):
        relative = path.relative_to(root).as_posix()
        code = rust_code(path.read_text(encoding="utf-8"))
        for forbidden in STORAGE_FORBIDDEN:
            if re.search(rf"\b{forbidden}\b", code):
                found.append(f"{relative}: storage-bound type-system identifier {forbidden}")
        match = target_pattern.search(code)
        if match:
            found.append(f"{relative}: target-specific type-system identifier {match.group()}")

    builtin = sources["src/core/src/type_system/builtin.rs"]
    builtin_body = extract_item_body(builtin, re.compile(r"\benum\s+BuiltinScalarKind\b")) or ""
    for name, ordinal in BUILTINS:
        if not re.search(rf"\b{name}\s*=\s*{ordinal}(?!\d)\s*,?", builtin_body):
            found.append(f"BuiltinScalarKind lost fixed ordinal {name}={ordinal}")
    if len(re.findall(r"\b(?:U8|U16|U32|U64|U128|I8|I16|I32|I64|I128|F32|F64|C32|C64|R64|String|Bool)\s*=", builtin_body)) != len(BUILTINS):
        found.append("BuiltinScalarKind is not the complete scalar ordinal registry")
    predicate_body = extract_item_body(
        builtin, re.compile(r"\benum\s+BuiltinKindPredicate\b")
    ) or ""
    declared_predicates = tuple(
        re.findall(r"(?m)^\s*([A-Z][A-Za-z0-9_]*)\s*,", predicate_body)
    )
    if declared_predicates != PREDICATES:
        found.append(
            "BuiltinKindPredicate is not exactly the closed nine-predicate vocabulary"
        )
    for name in PREDICATES:
        if not re.search(rf"\b{name}\b", predicate_body):
            found.append(f"BuiltinKindPredicate is missing {name}")

    constraints = extract_item_body(
        sources["src/core/src/kind_scheme.rs"], re.compile(r"\benum\s+KindConstraint\b")
    ) or ""
    for variant in ("Satisfies", "Promotes", "DimensionCompatible"):
        if not re.search(rf"\b{variant}\b", constraints):
            found.append(f"KindConstraint is missing {variant}")
    for retired in ("Keyable", "TableJoin"):
        if re.search(rf"(?m)^\s*{retired}\s*(?:\(|\{{)", constraints):
            found.append(f"KindConstraint retains operation-specific {retired}")

    for relative, declaration in (
        ("src/core/src/type_system/builtin.rs", "BuiltinKindPredicateSet"),
        ("src/core/src/type_system/resolved.rs", "KindPredicateEvidence"),
    ):
        if re.search(
            rf"(?m)^\s*pub\s+(?:struct|enum)\s+{declaration}\b",
            sources[relative],
        ):
            found.append(f"predicate implementation detail {declaration} is public")
    if re.search(r"(?m)^\s*pub\s+fn\s+intrinsic_kind_satisfies_predicate\b", builtin):
        found.append("intrinsic predicate classifier is public")
    if re.search(
        r"(?m)^\s*pub\s+fn\s+predicates_for\b",
        sources["src/core/src/type_system/resolved.rs"],
    ):
        found.append("predicate evidence lookup is public")

    solver = rust_code(sources["src/core/src/type_system/solver.rs"])
    if re.search(r"\b(?:unify_rigid_dimensions|rigid_dimension_alias|rigid_aliases)\b", solver):
        found.append("solver retains a rigid-dimension alias path")
    evolution = extract_item_body(
        sources["src/core/src/type_system/solver.rs"], re.compile(r"\bfn\s+dimension_evolution\b")
    ) or ""
    for expression in ("Add", "Multiply", "Min", "Max"):
        if not re.search(rf"DimensionExpr\s*::\s*{expression}\b", evolution):
            found.append(f"compound dimension evolution omits {expression}")
    if not re.search(r"\.\s*max\s*\(", evolution):
        found.append("compound dimension evolution does not take the maximum child evolution")
    less_equal = extract_item_body(
        sources["src/core/src/type_system/solver.rs"], re.compile(r"\bfn\s+prove_less_equal\b")
    ) or ""
    greater_equal = extract_item_body(
        sources["src/core/src/type_system/solver.rs"], re.compile(r"\bfn\s+prove_greater_equal\b")
    ) or ""
    if not re.search(r"left\s*\.\s*1.*left_max\s*<=\s*right\s*\.\s*0", less_equal, re.DOTALL):
        found.append("dimension <= proof does not compare left maximum to right minimum")
    if not re.search(r"right\s*\.\s*1.*left\s*\.\s*0\s*>=\s*right_max", greater_equal, re.DOTALL):
        found.append("dimension >= proof does not compare actual minimum to lower maximum")

    scheme_source = sources["src/core/src/type_system/scheme.rs"]
    dynamic_product = extract_item_body(
        scheme_source, re.compile(r"\bfn\s+dynamic_matrix_product\b")
    ) or ""
    dynamic_product = re.sub(r"\s+", "", dynamic_product)
    for fragment in (
        "matrix(kind(0),dim(0),dim(1))",
        "matrix(kind(1),dim(2),dim(3))",
        "matrix(kind(2),dim(0),dim(3))",
        "DimensionCompatible(dim(1),dim(2))",
    ):
        if fragment not in dynamic_product:
            found.append(f"dynamic matrix product lost semantic closure: {fragment}")
    for function, fragments in (
        (
            "matrix_product",
            ("KindConstraint::Promotes",),
        ),
        (
            "dynamic_matrix_product",
            ("KindConstraint::Promotes",),
        ),
        (
            "matrix_dot",
            ("KindConstraint::Promotes",),
        ),
        (
            "dynamic_matrix_dot",
            (
                "DimensionCompatible(dim(0),dim(2))",
                "DimensionCompatible(dim(1),dim(3))",
            ),
        ),
        (
            "dynamic_matrix_solve",
            (
                "matrix(kind(0),dim(2),dim(3))",
                "DimensionCompatible(dim(0),dim(1))",
                "DimensionCompatible(dim(0),dim(2))",
            ),
        ),
    ):
        body = extract_item_body(scheme_source, re.compile(rf"\bfn\s+{function}\b")) or ""
        compact = re.sub(r"\s+", "", body)
        if function != "dynamic_matrix_solve" and compact.count(
            "BuiltinKindPredicate::Number"
        ) < 2:
            found.append(f"{function} does not constrain both elements to Number")
        for fragment in fragments:
            if fragment not in compact:
                found.append(f"{function} lost semantic closure: {fragment}")
    if "1..=32" in rust_code(scheme_source):
        found.append("variadic source schemes retain the retired 32-argument expansion")
    for operation, template in (
        ("matrix/horzcat", "HorizontalConcatenation"),
        ("matrix/vertcat", "VerticalConcatenation"),
        ("set/define", "SetDefinition"),
    ):
        if not re.search(
            rf'"{re.escape(operation)}"\s*=>\s*Some\s*\(\s*SourceSchemeTemplate::{template}',
            scheme_source,
        ):
            found.append(f"source arity template is missing {operation} -> {template}")
    source_templates = extract_item_body_raw(
        scheme_source, re.compile(r"\bfn\s+maintained_source_scheme_template\b")
    ) or ""
    for operation, mode in (
        ("table/join", "Inner"),
        ("table/left-outer-join", "LeftOuter"),
        ("table/right-outer-join", "RightOuter"),
        ("table/full-outer-join", "FullOuter"),
        ("table/left-semi-join", "LeftSemi"),
        ("table/left-anti-join", "LeftAnti"),
    ):
        if not re.search(
            rf'"{re.escape(operation)}"\s*=>\s*(?:\{{\s*)?Some\s*\(\s*'
            rf"SourceSchemeTemplate::TableJoin\s*\(\s*TableJoinMode::{mode}",
            source_templates,
        ):
            found.append(f"source table-join template is missing {operation} -> {mode}")
    instantiate_template = extract_item_body(
        scheme_source, re.compile(r"\bfn\s+instantiate_source_scheme_template\b")
    ) or ""
    if "BuiltinKindPredicate::Keyable" not in instantiate_template:
        found.append("set definition template does not enforce Keyable")
    if not re.search(
        r'"set/comprehension"\s*=>\s*set_comprehension_schemes\s*\(',
        scheme_source,
    ):
        found.append("set comprehension does not retain a distinct semantic scheme")

    conversion = rust_code(sources["src/core/src/type_system/conversion.rs"])
    for authority in ("plan_implicit_conversion", "plan_numeric_promotion", "plan_explicit_cast"):
        if not re.search(rf"\bpub\s+fn\s+{authority}\b", conversion):
            found.append(f"conversion authority {authority} is missing")
    permitted = extract_item_body(
        sources["src/core/src/type_system/conversion.rs"], re.compile(r"\bfn\s+permitted_conversion\b")
    ) or ""
    if "plan_implicit_conversion" not in permitted:
        found.append("permitted_conversion does not delegate to ConversionPlan authority")
    numeric_promotion = extract_item_body(
        sources["src/core/src/type_system/conversion.rs"],
        re.compile(r"\bfn\s+plan_numeric_promotion\b"),
    ) or ""
    if re.search(r"\bif\s+exact_type_equal\s*\(", numeric_promotion):
        found.append("numeric promotion admits arbitrary exact-equal kinds")
    literal = rust_code(sources["src/engine/src/literals.rs"])
    integer_target = extract_item_body(
        sources["src/core/src/type_system/conversion.rs"], re.compile(r"\bfn\s+integer_target\b")
    ) or ""
    if re.search(r"\bnumber_to_f64\b", integer_target):
        found.append("integer conversion funnels through f64")
    if "ConversionPlan" not in literal or "execute_conversion_plan" not in literal:
        found.append("production conversion execution does not consume ConversionPlan")
    resident_conversion = rust_code(sources["src/engine/src/resident/conversion.rs"])
    for marker in ("bind_kind_conversion", "plan_explicit_cast"):
        if marker not in resident_conversion:
            found.append(f"resident convert/kind execution is missing {marker}")
    if not re.search(r"execute_conversion_draft\s*\(\s*source\b", resident_conversion):
        found.append("resident convert/kind execution is missing the semantic conversion executor")

    formulas = rust_code(sources["src/engine/src/expressions/formulas.rs"])
    add_route = extract_item_body(
        sources["src/engine/src/expressions/formulas.rs"],
        re.compile(r"\bfn\s+specialize_add_operation\b"),
    ) or ""
    if "operation_semantically_accepts" not in add_route:
        found.append("formula + does not select its operation through semantic schemes")
    if re.search(r"\b(?:representation|runtime_type|storage)\b", add_route, re.IGNORECASE):
        found.append("formula + inspects runtime representation or storage while routing")

    specialization_messages = "\n".join(
        extract_item_body_raw(
            sources["src/core/src/function/specialization.rs"],
            re.compile(rf"\bimpl\s+MechErrorKind\s+for\s+{name}\b"),
        ) or ""
        for name in (
            "SpecializationRuntimeCatalogUnavailable",
            "SpecializationRuntimeFactoryUnavailable",
            "SpecializationRuntimeFactoryAmbiguous",
        )
    )
    if re.search(r"\b(?:representation|prefix|factory name|runtime factory)\b", specialization_messages, re.IGNORECASE):
        found.append("source-facing specialization diagnostics expose physical binding details")

    expression_errors = rust_code(sources["src/engine/src/expressions/errors.rs"])
    if re.search(r"\b(?:FunctionValueRepresentation|FunctionRuntimeType|RuntimeFunctionSignature)\b", expression_errors):
        found.append("source-facing expression diagnostics expose physical binding types")
    for name in (
        "ComprehensionGeneratorError",
        "MatchArmKindMismatchError",
        "InvalidGuardExpressionError",
    ):
        body = extract_item_body(
            sources["src/engine/src/expressions/errors.rs"],
            re.compile(rf"\bimpl\s+MechErrorKind\s+for\s+{name}\b"),
        ) or ""
        if "semantic_name" not in body:
            found.append(f"{name} does not format a semantic type name")

    catalog = sources["src/core/src/function/catalog.rs"]
    runtime_entry = extract_item_body(catalog, re.compile(r"\bstruct\s+RuntimeFunctionEntry\b")) or ""
    if re.search(r"\bKindScheme\b", runtime_entry):
        found.append("RuntimeFunctionEntry carries semantic KindScheme metadata")
    catalog_code = rust_code(catalog)
    named_signature = re.search(
        r"\bfn\s+insert_canonical_specializer\s*\([^)]*FunctionTypeDeclaration[^)]*\)",
        catalog_code,
        re.DOTALL,
    )
    insert_named = extract_item_body(catalog, re.compile(r"\bfn\s+insert_canonical_specializer\b")) or ""
    if named_signature is None or "SourceTypeAuthority::Schemes" not in insert_named:
        found.append("named source specializers do not require FunctionTypeDeclaration")
    insert_intrinsic = extract_item_body(catalog, re.compile(r"\bfn\s+insert_canonical_intrinsic_specializer\b")) or ""
    if "SourceTypeAuthority::SyntaxDirectedIntrinsic" not in insert_intrinsic:
        found.append("parser-only syntax-directed authority is not explicit")

    production: list[tuple[str, str]] = []
    for base in ("src", "machines"):
        for path in _rust_files(root, base):
            relative = path.relative_to(root).as_posix()
            if "tests" not in path.relative_to(root).parts:
                production.append((relative, strip_cfg_test_modules(path.read_text(encoding="utf-8"))))
    for relative, code in production:
        if re.search(r"\bscheme_from_signature\b", code):
            found.append(f"{relative}: runtime signature projection remains semantic authority")
        if re.search(r"\binfer_runtime_output_type\b", code):
            found.append(f"{relative}: runtime representation still infers semantic output")
    for relative in ("src/core/src/type_system/scheme.rs", "src/engine/src/efficacy/ekf/catalog.rs"):
        path = root / relative
        if path.is_file() and re.search(r"KindExpr\s*::\s*Wildcard", rust_code(path.read_text(encoding="utf-8"))):
            found.append(f"{relative}: named semantic declaration contains a wildcard")

    specialization = sources["src/core/src/function/specialization.rs"]
    for method in (
        "bind_runtime_factory", "bind_runtime_factory_derived_output",
        "bind_runtime_factory_existing_output",
    ):
        body = extract_item_body(specialization, re.compile(rf"\bfn\s+{method}\b")) or ""
        if not re.search(r"self\s*\.\s*resolved_output\s*\(", body):
            found.append(f"{method} can bind without an existing ResolvedCall output")
        allocation = body.find("default_for_representation")
        selection = body.find("candidates")
        if allocation >= 0 and (selection < 0 or allocation < selection):
            found.append(f"{method} uses output allocation as overload probing")

    resolver = extract_item_body(
        sources["src/engine/src/function/resolver.rs"], re.compile(r"\bfn\s+specialize_operation_named_with\b")
    ) or sources["src/engine/src/function/resolver.rs"]
    schemes = resolver.find("SourceTypeAuthority::Schemes")
    semantic = resolver.find("resolve_declared_call", schemes)
    physical = resolver.find("specialize_invocation", schemes)
    if semantic < 0 or physical < 0 or semantic > physical:
        found.append("source resolver does not resolve semantics before physical specialization")
    if "validate_resolved_output" not in resolver:
        found.append("source resolver does not close and validate physical output")

    wire_pattern = re.compile(r"\b(?:" + "|".join(R3_WIRE_NAMES) + r")\b")
    for relative in WIRE_ROOTS:
        for path in _rust_files(root, relative):
            code = rust_code(path.read_text(encoding="utf-8"))
            match = wire_pattern.search(code)
            if match:
                found.append(f"{path.relative_to(root).as_posix()}: R3 metadata leaks into a preserved wire surface: {match.group()}")

    conformance = "\n".join(
        sources[relative]
        for relative in REQUIRED
        if "/tests/type_system_" in relative
        or relative == "src/runtime/src/runtime/program/tests.rs"
    )
    for marker in CONFORMANCE:
        if marker not in conformance:
            found.append(f"R3 conformance suite is missing {marker}")

    design = sources["docs/design/type-system-v1.md"]
    for marker in (
        "Status: R3 complete", "Semantic authority order", "Builtin scalar registry",
        "Built-in predicate table", "ResolvedType", "Rigid and bindable dimensions",
        "Implicit conversion table", "Numeric promotion table", "Explicit cast table",
        "Source operation schemes", "Serialization and artifact policy", "R4 handoff",
        "first-order", "expression-local", "shadow-only", "0.3.6",
    ):
        if marker not in design:
            found.append(f"type-system design is missing {marker}")
    for relative in ("README.md", "docs/design/ROADMAP.mec", "docs/design/v0.4-endgame.md"):
        if "0.3.6" not in sources[relative]:
            found.append(f"{relative} lost package version 0.3.6")
        if "R3" not in sources[relative] or "complete" not in sources[relative] or "R4" not in sources[relative]:
            found.append(f"{relative} does not mark R3 complete and R4 next")

    r3 = "python3 scripts/check-r3-type-system.py"
    unit = "scripts/tests/test_check_r3_type_system.py"
    for relative, job in (
        (".github/workflows/ci.yml", "static-contracts"),
        (".github/workflows/ci-full.yml", "architecture-contracts"),
    ):
        block = _job(sources[relative], job)
        if r3 not in block:
            found.append(f"{relative} does not run the R3 architecture checker")
        if unit not in block:
            found.append(f"{relative} does not run the R3 checker tests")
        if "continue-on-error" in block and (r3 in block or unit in block):
            found.append(f"{relative} may waive the R3 architecture gate")
    full = _job(sources[".github/workflows/ci-full.yml"], "architecture-contracts")
    for target in (
        "type_system_builtin", "type_system_solver", "type_system_conversion",
        "type_system_catalog", "type_system_source",
    ):
        if target not in full:
            found.append(f"Full CI does not run R3 conformance target {target}")
    runtime_conversion = re.search(
        r"cargo\s+\+nightly-2026-03-03\s+test\s+--locked\s+-p\s+mech-runtime"
        r"(?:(?!\bcargo\b).)*--features\s+full_compiler,resident-routing-source"
        r"(?:(?!\bcargo\b).)*compiled_conversion_executes_after_bytecode_round_trip",
        full,
        re.DOTALL,
    )
    if runtime_conversion is None:
        found.append("Full CI does not execute resident conversion conformance with source routing")
    owners = sources[".github/ci/owners.toml"]
    for path in ("scripts/check-r3-type-system.py", unit, "docs/design/type-system-v1.md"):
        if path not in owners:
            found.append(f"architecture owner entry is missing {path}")
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    found = failures(args.root)
    if not found:
        print("R3 Type System v1 architecture contract passed")
        return 0
    print("R3 Type System v1 architecture contract failed:", file=sys.stderr)
    for failure in found:
        print(f"  {failure}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
