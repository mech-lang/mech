"""Immutable C2 V2 scanner for legacy value, alias, identity, and journal uses.

This complete module is the scanner implementation freeze.  Its normalized
UTF-8 file bytes, including the final newline, are hashed by the C0 contract.
"""

from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence


SCANNER_VERSION = "c2-legacy-growth-v2"
RAW_STRING_START = re.compile(r'(?:br|r)(?P<hashes>#{0,255})"')
TOKEN_PATTERN = re.compile(
    r"r#[A-Za-z_][A-Za-z0-9_]*|::|->|[A-Za-z_][A-Za-z0-9_]*|"
    r"[.(){}\[\];,=*<>:+&!?]"
)
IDENTIFIER = re.compile(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*\Z")

RESOLUTIONS = {
    "LegacyValue",
    "ValueKind",
    "SemanticKind",
    "SyntaxKind",
    "Ref",
    "Other",
    "Ambiguous",
    "Cycle",
}

LEGACY_ALIAS_SPECS = {
    ("src/core/src/types/mod.rs", "MutableReference"): "Ref<LegacyValue>",
    ("src/core/src/types/mod.rs", "ValRef"): "Ref<LegacyValue>",
}
COMPATIBILITY_ALIAS_SPECS = {
    ("src/core/src/program/symbol_table.rs", "SymbolTableRef"): {
        "target": "Ref<SymbolTable>",
        "route": (
            ("src/core/src/program/mod.rs", "pubuseself::symbol_table::*;"),
            ("src/core/src/lib.rs", "pubuseself::program::*;"),
        ),
    },
    ("src/engine/src/interpreter/mod.rs", "InterpreterRef"): {
        "target": "Ref<Box<Interpreter>>",
        "route": (("src/engine/src/lib.rs", "pubusecrate::interpreter::*;"),),
    },
}
APPROVED_ALIAS_NAMES = {
    name for _path, name in (*LEGACY_ALIAS_SPECS, *COMPATIBILITY_ALIAS_SPECS)
}
APPROVED_REF_TYPE_ALIASES = set(LEGACY_ALIAS_SPECS) | set(
    COMPATIBILITY_ALIAS_SPECS
)
REF_BACKED_ALIAS_NAMES = {
    "MutableReference",
    "ValRef",
    "SymbolTableRef",
    "InterpreterRef",
}

HIGH_RISK_PATTERNS = {
    "value-mutable-reference": "LegacyValue :: MutableReference",
    "value-typed-wrapper": "LegacyValue :: Typed",
    "valref-alias": "ValRef",
    "mutable-reference-alias": "MutableReference",
    "reactive-cell-id": "ReactiveCellId",
    "value-state-journal": "ValueStateJournal",
    "reactive-turn-journal": "ReactiveTurnJournal",
    "transaction-state-values-api": "transaction_state_values",
    "ref-as-ptr-definition": "impl Ref<T> { fn as_ptr }",
    "ref-as-mut-ptr-definition": "impl Ref<T> { fn as_mut_ptr }",
    "ref-addr-definition": "impl Ref<T> { fn addr }",
    "ref-id-definition": "impl Ref<T> { fn id }",
    "ref-as-ptr-ufcs": "Ref identity call as_ptr (UFCS or proven instance)",
    "ref-as-mut-ptr-ufcs": "Ref identity call as_mut_ptr (UFCS or proven instance)",
    "ref-addr-ufcs": "Ref identity call addr (UFCS or proven instance)",
    "ref-id-ufcs": "Ref identity call id (UFCS or proven instance)",
}


@dataclass(frozen=True, order=True)
class Use:
    path: str
    line: int
    column: int
    fingerprint: str


@dataclass(frozen=True)
class Token:
    value: str
    offset: int
    line: int
    column: int


@dataclass(frozen=True)
class UseBinding:
    path: tuple[str, ...]
    local: str
    token: Token
    aliased: bool
    raw: bool
    glob: bool
    grouped: bool


@dataclass(frozen=True)
class TypeAlias:
    token: Token
    raw_name: str
    name: str
    parameters: tuple[str, ...]
    rhs: tuple[Token, ...]
    visibility: str


def canonical_identifier(spelling: str) -> str:
    return spelling[2:] if spelling.startswith("r#") else spelling


def mask_non_code(source: str) -> str:
    """Mask comments and literals while preserving offsets and newlines."""
    masked = list(source)
    offset = 0

    def blank(start: int, end: int) -> None:
        for index in range(start, end):
            if masked[index] != "\n":
                masked[index] = " "

    while offset < len(source):
        if source.startswith("//", offset):
            end = source.find("\n", offset + 2)
            end = len(source) if end < 0 else end
            blank(offset, end)
            offset = end
            continue
        if source.startswith("/*", offset):
            depth = 1
            end = offset + 2
            while end < len(source) and depth:
                if source.startswith("/*", end):
                    depth += 1
                    end += 2
                elif source.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            blank(offset, end)
            offset = end
            continue
        raw_match = RAW_STRING_START.match(source, offset)
        if raw_match is not None:
            delimiter = '"' + raw_match.group("hashes")
            end = source.find(delimiter, raw_match.end())
            end = len(source) if end < 0 else end + len(delimiter)
            blank(offset, end)
            offset = end
            continue
        if source.startswith('b"', offset) or source[offset] == '"':
            end = offset + (2 if source.startswith('b"', offset) else 1)
            while end < len(source):
                if source[end] == "\\":
                    end += 2
                elif source[end] == '"':
                    end += 1
                    break
                else:
                    end += 1
            blank(offset, end)
            offset = end
            continue
        if source[offset] == "'":
            end = offset + 1
            end += 2 if end < len(source) and source[end] == "\\" else 1
            if end < len(source) and source[end] == "'":
                end += 1
                blank(offset, end)
                offset = end
                continue
        offset += 1
    return "".join(masked)


def rust_tokens(source: str, searchable: str | None = None) -> list[Token]:
    searchable = mask_non_code(source) if searchable is None else searchable
    tokens: list[Token] = []
    for match in TOKEN_PATTERN.finditer(searchable):
        last_newline = source.rfind("\n", 0, match.start())
        tokens.append(
            Token(
                match.group(0),
                match.start(),
                source.count("\n", 0, match.start()) + 1,
                match.start() - last_newline,
            )
        )
    return tokens


def balanced_token_end(
    tokens: Sequence[Token], opening: int, opening_value: str, closing_value: str
) -> int | None:
    depth = 0
    for index in range(opening, len(tokens)):
        if tokens[index].value == opening_value:
            depth += 1
        elif tokens[index].value == closing_value:
            depth -= 1
            if depth == 0:
                return index
    return None


def balanced_token_start(
    tokens: Sequence[Token], closing: int, opening_value: str, closing_value: str
) -> int | None:
    depth = 0
    for index in range(closing, -1, -1):
        if tokens[index].value == closing_value:
            depth += 1
        elif tokens[index].value == opening_value:
            depth -= 1
            if depth == 0:
                return index
    return None


def split_top_level_tokens(tokens: Sequence[Token]) -> list[list[Token]]:
    parts: list[list[Token]] = []
    start = 0
    depths = {"(": 0, "[": 0, "{": 0, "<": 0}
    closers = {")": "(", "]": "[", "}": "{", ">": "<"}
    for index, token in enumerate(tokens):
        if token.value in depths:
            depths[token.value] += 1
        elif token.value in closers and depths[closers[token.value]]:
            depths[closers[token.value]] -= 1
        elif token.value == "," and not any(depths.values()):
            parts.append(list(tokens[start:index]))
            start = index + 1
    parts.append(list(tokens[start:]))
    return [part for part in parts if part]


def strip_outer_parentheses(tokens: Sequence[Token]) -> list[Token]:
    result = list(tokens)
    while result and result[0].value == "(":
        closing = balanced_token_end(result, 0, "(", ")")
        if closing != len(result) - 1:
            break
        result = result[1:-1]
    return result


def canonical_token_text(tokens: Sequence[Token]) -> str:
    return "".join(
        canonical_identifier(token.value) if IDENTIFIER.fullmatch(token.value) else token.value
        for token in tokens
    )


def use_bindings(tokens: Sequence[Token]) -> list[UseBinding]:
    def identifiers(items: Sequence[Token]) -> tuple[str, ...]:
        return tuple(
            canonical_identifier(token.value)
            for token in items
            if IDENTIFIER.fullmatch(token.value) or token.value == "*"
        )

    def expand(
        items: Sequence[Token], prefix: tuple[str, ...], grouped: bool = False
    ) -> list[UseBinding]:
        records: list[UseBinding] = []
        for part in split_top_level_tokens(items):
            opening = next(
                (index for index, token in enumerate(part) if token.value == "{"),
                None,
            )
            if opening is not None:
                closing = balanced_token_end(part, opening, "{", "}")
                if closing is None:
                    continue
                head = list(part[:opening])
                while head and head[-1].value == "::":
                    head.pop()
                records.extend(
                    expand(part[opening + 1 : closing], prefix + identifiers(head), True)
                )
                continue
            alias_index = next(
                (index for index, token in enumerate(part) if token.value == "as"),
                None,
            )
            body = part[:alias_index] if alias_index is not None else part
            path_suffix = identifiers(body)
            path = prefix if path_suffix == ("self",) else prefix + path_suffix
            if not path:
                continue
            alias_token = (
                part[alias_index + 1]
                if alias_index is not None and alias_index + 1 < len(part)
                else None
            )
            local = canonical_identifier(
                alias_token.value if alias_token is not None else path[-1]
            )
            records.append(
                UseBinding(
                    path,
                    local,
                    alias_token or body[-1],
                    alias_token is not None,
                    any(token.value.startswith("r#") for token in part),
                    path[-1] == "*",
                    grouped,
                )
            )
        return records

    records: list[UseBinding] = []
    offset = 0
    while offset < len(tokens):
        if canonical_identifier(tokens[offset].value) != "use":
            offset += 1
            continue
        end = offset + 1
        while end < len(tokens) and tokens[end].value != ";":
            end += 1
        records.extend(expand(tokens[offset + 1 : end], ()))
        offset = end + 1
    return records


def type_alias_declarations(tokens: Sequence[Token]) -> list[TypeAlias]:
    declarations: list[TypeAlias] = []
    offset = 0
    while offset < len(tokens):
        if canonical_identifier(tokens[offset].value) != "type" or offset + 1 >= len(tokens):
            offset += 1
            continue
        end = offset + 1
        depths = {"(": 0, "[": 0, "{": 0, "<": 0}
        closers = {")": "(", "]": "[", "}": "{", ">": "<"}
        while end < len(tokens):
            value = tokens[end].value
            if value in depths:
                depths[value] += 1
            elif value in closers and depths[closers[value]]:
                depths[closers[value]] -= 1
            elif value == ";" and not any(depths.values()):
                break
            end += 1
        statement = list(tokens[offset:end])
        equals = next(
            (index for index, token in enumerate(statement) if token.value == "="),
            None,
        )
        if equals is None or len(statement) < 3:
            offset = end + 1
            continue
        name_token = statement[1]
        parameters: list[str] = []
        if statement[2].value == "<":
            generic_end = balanced_token_end(statement, 2, "<", ">")
            if generic_end is not None and generic_end < equals:
                parameters = [
                    canonical_identifier(token.value)
                    for token in statement[3:generic_end]
                    if IDENTIFIER.fullmatch(token.value)
                ]
        visibility = "private"
        before = offset - 1
        if before >= 0 and canonical_identifier(tokens[before].value) == "pub":
            visibility = "pub"
        declarations.append(
            TypeAlias(
                name_token,
                name_token.value,
                canonical_identifier(name_token.value),
                tuple(parameters),
                tuple(statement[equals + 1 :]),
                visibility,
            )
        )
        offset = end + 1
    return declarations


def path_and_arguments(
    tokens: Sequence[Token],
) -> tuple[tuple[str, ...], list[list[Token]] | None] | None:
    items = strip_outer_parentheses(tokens)
    if not items:
        return None
    index = 1 if items[0].value == "::" else 0
    path: list[str] = []
    while index < len(items) and IDENTIFIER.fullmatch(items[index].value):
        path.append(canonical_identifier(items[index].value))
        index += 1
        if index < len(items) and items[index].value == "::":
            if index + 1 < len(items) and items[index + 1].value == "<":
                index += 1
                break
            index += 1
            continue
        break
    if not path:
        return None
    if index == len(items):
        return tuple(path), None
    if items[index].value != "<":
        return None
    closing = balanced_token_end(items, index, "<", ">")
    if closing != len(items) - 1:
        return None
    return tuple(path), split_top_level_tokens(items[index + 1 : closing])


class TransparentTypeResolver:
    """Shared conservative resolver for audited enums and Ref."""

    def __init__(
        self,
        tokens: Sequence[Token],
        *,
        relative: str = "",
        inherited_bindings: Sequence[UseBinding] = (),
    ) -> None:
        self.relative = relative
        self.aliases = {alias.name: alias for alias in type_alias_declarations(tokens)}
        self.bindings = {
            binding.local: binding.path
            for binding in (*use_bindings(tokens), *inherited_bindings)
            if not binding.glob
        }
        self.local_declarations = {
            canonical_identifier(tokens[index + 1].value)
            for index in range(len(tokens) - 1)
            if canonical_identifier(tokens[index].value)
            in {"enum", "struct", "trait", "union"}
            and IDENTIFIER.fullmatch(tokens[index + 1].value)
        }
        self.identity_aliases: dict[str, int] = {}
        for alias in self.aliases.values():
            rhs = strip_outer_parentheses(alias.rhs)
            if len(rhs) == 1 and IDENTIFIER.fullmatch(rhs[0].value):
                parameter = canonical_identifier(rhs[0].value)
                if parameter in alias.parameters:
                    self.identity_aliases[alias.name] = alias.parameters.index(parameter)
        self.memo: dict[str, str] = {}

    @staticmethod
    def direct_path(path: tuple[str, ...]) -> str:
        if not path:
            return "Other"
        terminal = path[-1]
        if terminal == "Kind" and len(path) >= 2 and path[-2] == "nodes":
            return "SyntaxKind"
        if terminal == "Kind" and (
            len(path) == 1
            or (len(path) >= 2 and path[-2] == "kind")
            or path in {("mech_core", "Kind"), ("crate", "Kind")}
        ):
            return "SemanticKind"
        if terminal in {"Value", "LegacyValue"}:
            if terminal == "Value" and "snapshot" in path[:-1]:
                return "Other"
            return (
                "LegacyValue"
                if len(path) == 1
                or path[-2]
                in {"value", "legacy_value", "crate", "self", "super", "mech_core"}
                else "Ambiguous"
            )
        if terminal == "ValueKind":
            return "ValueKind" if len(path) == 1 or path[-2] in {"value", "crate", "self", "super", "mech_core"} else "Ambiguous"
        if terminal == "Ref":
            return "Ref" if len(path) == 1 or path[-2] in {"crate", "self", "super", "mech_core"} else "Ambiguous"
        if terminal in REF_BACKED_ALIAS_NAMES:
            return "Ref"
        return "Other"

    def resolve_alias(self, name: str, stack: tuple[str, ...]) -> str:
        if name in stack:
            return "Cycle"
        if name in self.memo:
            return self.memo[name]
        alias = self.aliases.get(name)
        if alias is None:
            return "Other"
        result = self.resolve(alias.rhs, stack + (name,))
        self.memo[name] = result
        return result

    def resolve(self, tokens: Sequence[Token], stack: tuple[str, ...] = ()) -> str:
        parsed = path_and_arguments(tokens)
        if parsed is None:
            return "Other"
        path, arguments = parsed
        terminal = path[-1]
        if len(path) == 1 and terminal in self.bindings:
            imported = self.direct_path(self.bindings[terminal])
            if imported != "Other":
                return imported
        if len(path) == 1 and terminal in self.aliases:
            selected = self.identity_aliases.get(terminal)
            if selected is not None and arguments is not None:
                if selected >= len(arguments):
                    return "Ambiguous"
                return self.resolve(arguments[selected], stack + (terminal,))
            return self.resolve_alias(terminal, stack)
        direct = self.direct_path(path)
        if direct != "Other":
            return direct
        if len(path) == 1 and terminal in self.local_declarations:
            return "SyntaxKind" if terminal == "Kind" else "Other"
        if terminal in {"Value", "LegacyValue", "ValueKind", "Kind", "Ref"}:
            return "Ambiguous"
        return "Other"

    def alias_resolutions(self) -> dict[str, str]:
        return {name: self.resolve_alias(name, ()) for name in self.aliases}


def transparent_conversion_aliases(tokens: Sequence[Token]) -> dict[str, set[str]]:
    """Resolve only direct or exact-identity aliases used by conversion checks."""
    declarations = {alias.name: alias for alias in type_alias_declarations(tokens)}
    identity_aliases: dict[str, int] = {}
    for alias in declarations.values():
        rhs = strip_outer_parentheses(alias.rhs)
        if len(rhs) == 1 and IDENTIFIER.fullmatch(rhs[0].value):
            parameter = canonical_identifier(rhs[0].value)
            if parameter in alias.parameters:
                identity_aliases[alias.name] = alias.parameters.index(parameter)
    aliases: dict[str, set[str]] = {"LegacyValue": {"legacy"}}

    def resolve(items: Sequence[Token]) -> set[str]:
        parsed = path_and_arguments(items)
        if parsed is None:
            return set()
        path, arguments = parsed
        terminal = path[-1]
        if terminal == "LegacyValue":
            return {"legacy"}
        if terminal == "Value" and "snapshot" in path[:-1]:
            return {"snapshot"}
        if terminal in aliases and arguments is None:
            return set(aliases[terminal])
        selected = identity_aliases.get(terminal)
        if selected is not None and arguments is not None and selected < len(arguments):
            return resolve(arguments[selected])
        return set()

    changed = True
    while changed:
        changed = False
        for binding in use_bindings(tokens):
            category = resolve(
                [
                    Token(part, binding.token.offset, binding.token.line, binding.token.column)
                    for index, part in enumerate(binding.path)
                    for part in (("::", part) if index else (part,))
                ]
            )
            before = set(aliases.get(binding.local, set()))
            aliases.setdefault(binding.local, set()).update(category)
            changed = changed or aliases[binding.local] != before
        for alias in declarations.values():
            before = set(aliases.get(alias.name, set()))
            aliases.setdefault(alias.name, set()).update(resolve(alias.rhs))
            changed = changed or aliases[alias.name] != before
    return aliases


def imported_trait_aliases(
    tokens: Sequence[Token], canonical_traits: set[str]
) -> set[str]:
    """Resolve direct and transitively renamed imported trait bindings."""
    aliases = set(canonical_traits)
    changed = True
    while changed:
        changed = False
        for binding in use_bindings(tokens):
            if binding.path and binding.path[-1] in aliases and binding.local not in aliases:
                aliases.add(binding.local)
                changed = True
    return aliases


def audited_type_alias_violations(
    relative: str,
    tokens: Sequence[Token],
    inherited_bindings: Sequence[UseBinding] = (),
) -> list[dict[str, object]]:
    resolver = TransparentTypeResolver(
        tokens, relative=relative, inherited_bindings=inherited_bindings
    )
    records: list[dict[str, object]] = []
    for alias in resolver.aliases.values():
        resolution = resolver.resolve_alias(alias.name, ())
        if resolution == "Other" or resolution == "SyntaxKind":
            continue
        if resolution == "Ref":
            if (relative, alias.name) in APPROVED_REF_TYPE_ALIASES:
                continue
            enum_name, kind = "Ref", "ref-alias"
        elif resolution == "Cycle":
            enum_name, kind = "Type", "type-alias-cycle"
        elif resolution == "Ambiguous":
            enum_name, kind = "Type", "type-alias-ambiguous"
        else:
            enum_name = {
                "SemanticKind": "Kind",
                "LegacyValue": "LegacyValue",
                "ValueKind": "ValueKind",
            }[resolution]
            raw = any(
                token.value.startswith("r#")
                and canonical_identifier(token.value)
                in {"Value", "LegacyValue", "ValueKind", "Kind"}
                for token in alias.rhs
            )
            kind = "raw-audited-alias" if raw else "semantic-kind-alias" if enum_name == "Kind" else "type-alias"
        records.append(
            {
                "enum": enum_name,
                "kind": kind,
                "path": relative,
                "line": alias.token.line,
                "column": alias.token.column,
            }
        )
    return records


def resolved_ref_type_aliases(tokens: Sequence[Token]) -> dict[str, Token]:
    resolver = TransparentTypeResolver(tokens)
    return {
        name: resolver.aliases[name].token
        for name, resolution in resolver.alias_resolutions().items()
        if resolution == "Ref"
    }


def alias_records(
    corpus: Sequence[tuple[str, str, str, list[Token]]]
) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    sources = {relative: tokens for relative, _source, _searchable, tokens in corpus}
    declarations = [
        (relative, alias)
        for relative, _source, _searchable, tokens in corpus
        for alias in type_alias_declarations(tokens)
        if alias.name in APPROVED_ALIAS_NAMES
    ]

    def base_record(relative: str, alias: TypeAlias) -> dict[str, object]:
        target = canonical_token_text(alias.rhs)
        if alias.name in {"MutableReference", "ValRef"}:
            target = target.replace("Ref<Value>", "Ref<LegacyValue>")
        return {
            "name": alias.name,
            "raw_name": alias.raw_name,
            "target": target,
            "path": relative,
            "line": alias.token.line,
            "column": alias.token.column,
            "visibility": alias.visibility,
        }

    legacy = [
        base_record(path, alias)
        for path, alias in declarations
        if alias.name in {"MutableReference", "ValRef"}
    ]
    compatibility: list[dict[str, object]] = []
    for relative, alias in declarations:
        if alias.name not in {"SymbolTableRef", "InterpreterRef"}:
            continue
        spec = COMPATIBILITY_ALIAS_SPECS.get((relative, alias.name), {"route": ()})
        record = base_record(relative, alias)
        route: list[dict[str, object]] = []
        for path, expected in spec["route"]:
            tokens = sources.get(path, [])
            offset = 0
            while offset < len(tokens):
                if canonical_identifier(tokens[offset].value) != "pub":
                    offset += 1
                    continue
                end = offset
                while end < len(tokens) and tokens[end].value != ";":
                    end += 1
                statement = tokens[offset : min(end + 1, len(tokens))]
                if canonical_token_text(statement) == expected:
                    route.append(
                        {
                            "path": path,
                            "line": tokens[offset].line,
                            "declaration": expected,
                        }
                    )
                    break
                offset = end + 1
        record["public_reexport_route"] = route
        compatibility.append(record)
    order = lambda record: (str(record["name"]), str(record["path"]))
    return sorted(legacy, key=order), sorted(compatibility, key=order)


def raw_approved_aliases(
    legacy: Sequence[dict[str, object]], compatibility: Sequence[dict[str, object]]
) -> list[dict[str, object]]:
    return [
        record
        for record in (*legacy, *compatibility)
        if str(record["raw_name"]).startswith("r#")
    ]


def occurrence_fingerprint(tokens: Sequence[Token], target: int) -> str:
    """Identify an occurrence by its stable clause and identifier context.

    Punctuation and whitespace-sensitive layout are deliberately absent: the C2
    owner rename can make rustfmt wrap an otherwise unchanged arm, and it can
    reorder a grouped import. Cardinality within the resulting clause ID still
    detects additions and substitutions.
    """

    def normalized_identifier(index: int) -> str | None:
        identifier = canonical_identifier(tokens[index].value)
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", identifier):
            return None
        return "LegacyValue" if identifier in {"Value", "LegacyValue"} else identifier

    stack: list[int] = []
    for index, token in enumerate(tokens[:target]):
        if token.value == "{":
            stack.append(index)
        elif token.value == "}" and stack:
            stack.pop()

    scope = ["module"]
    for opening_index in reversed(stack):
        parent = next((item for item in reversed(stack) if item < opening_index), -1)
        header = tokens[parent + 1 : opening_index]
        candidate: tuple[str, str] | None = None
        for index, token in enumerate(header[:-1]):
            keyword = canonical_identifier(token.value)
            if keyword not in {"fn", "type", "struct", "enum", "union", "const", "static"}:
                continue
            name = canonical_identifier(header[index + 1].value)
            if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
                candidate = (keyword, "LegacyValue" if name == "Value" else name)
        if candidate is not None:
            scope = [*candidate]
            break

    target_identifier = normalized_identifier(target) or canonical_identifier(tokens[target].value)
    use_group = any(
        any(token.value == "use" for token in tokens[parent + 1 : opening_index])
        for opening_index in stack
        for parent in [next((item for item in reversed(stack) if item < opening_index), -1)]
    )
    statement_start = max(
        (
            index
            for index in range(target)
            if tokens[index].value in {";", "}"}
        ),
        default=-1,
    )
    module_use = use_group or any(
        token.value == "use" for token in tokens[statement_start + 1 : target]
    )
    if module_use:
        context = ["module-use"]
    else:
        identifiers = [
            (index, identifier)
            for index in range(len(tokens))
            for identifier in [normalized_identifier(index)]
            if identifier is not None
        ]
        target_offset = next(
            offset for offset, (index, _identifier) in enumerate(identifiers) if index == target
        )
        context = [
            identifier
            for index, identifier in identifiers[
                max(0, target_offset - 8) : target_offset + 9
            ]
            if index != target
        ]
    material = [*scope, f"@{target_identifier}", *context]
    return hashlib.sha256("\0".join(material).encode("utf-8")).hexdigest()


def use_at(relative: str, tokens: Sequence[Token], index: int) -> Use:
    token = tokens[index]
    return Use(
        relative,
        token.line,
        token.column,
        occurrence_fingerprint(tokens, index),
    )


def grouped_uses(uses: Iterable[Use]) -> list[dict[str, object]]:
    grouped: dict[str, list[Use]] = {}
    for use in sorted(set(uses)):
        grouped.setdefault(use.path, []).append(use)
    return [
        {
            "path": path,
            "sites": [
                {
                    "line": use.line,
                    "column": use.column,
                    "fingerprint": use.fingerprint,
                }
                for use in path_uses
            ],
            "count": len(path_uses),
        }
        for path, path_uses in sorted(grouped.items())
    ]


def exact_identifier_uses(
    corpus: Sequence[tuple[str, str, str, list[Token]]], identifier: str
) -> list[dict[str, object]]:
    return grouped_uses(
        use_at(relative, tokens, index)
        for relative, _source, _searchable, tokens in corpus
        for index, token in enumerate(tokens)
        if canonical_identifier(token.value) == identifier
    )


def exact_sequence_uses(
    corpus: Sequence[tuple[str, str, str, list[Token]]], sequence: Sequence[str]
) -> list[dict[str, object]]:
    return grouped_uses(
        use_at(relative, tokens, index)
        for relative, _source, _searchable, tokens in corpus
        for index in range(len(tokens) - len(sequence) + 1)
        if [
            canonical_identifier(token.value)
            for token in tokens[index : index + len(sequence)]
        ]
        == list(sequence)
    )


def impl_blocks(tokens: Sequence[Token]) -> list[tuple[list[Token], int, int]]:
    blocks: list[tuple[list[Token], int, int]] = []
    offset = 0
    while offset < len(tokens):
        if canonical_identifier(tokens[offset].value) != "impl":
            offset += 1
            continue
        depths = {"(": 0, "[": 0, "<": 0}
        closers = {")": "(", "]": "[", ">": "<"}
        opening = None
        for index in range(offset + 1, len(tokens)):
            value = tokens[index].value
            if value in depths:
                depths[value] += 1
            elif value in closers and depths[closers[value]]:
                depths[closers[value]] -= 1
            elif value == "{" and not any(depths.values()):
                opening = index
                break
            elif value == ";" and not any(depths.values()):
                break
        if opening is None:
            offset += 1
            continue
        closing = balanced_token_end(tokens, opening, "{", "}")
        if closing is None:
            break
        header = list(tokens[offset + 1 : opening])
        if header and header[0].value == "<":
            generic_end = balanced_token_end(header, 0, "<", ">")
            header = header[generic_end + 1 :] if generic_end is not None else []
        depth = 0
        for_index = None
        where_index = None
        for index, token in enumerate(header):
            if token.value == "<":
                depth += 1
            elif token.value == ">" and depth:
                depth -= 1
            elif depth == 0 and canonical_identifier(token.value) == "for":
                for_index = index
            elif depth == 0 and canonical_identifier(token.value) == "where":
                where_index = index
                break
        self_type = header[(for_index + 1) if for_index is not None else 0 : where_index]
        blocks.append((self_type, opening, closing))
        offset = closing + 1
    return blocks


def ref_method_definition_uses(
    corpus: Sequence[tuple[str, str, str, list[Token]]], method: str
) -> list[dict[str, object]]:
    uses: list[Use] = []
    for relative, _source, _searchable, tokens in corpus:
        resolver = TransparentTypeResolver(tokens, relative=relative)
        for self_type, opening, closing in impl_blocks(tokens):
            if resolver.resolve(self_type) != "Ref":
                continue
            for index in range(opening + 1, closing - 1):
                if (
                    canonical_identifier(tokens[index].value) == "fn"
                    and canonical_identifier(tokens[index + 1].value) == method
                ):
                    uses.append(use_at(relative, tokens, index + 1))
    return grouped_uses(uses)


def ufcs_receiver(tokens: Sequence[Token], separator: int) -> list[Token]:
    end = separator
    if end <= 0:
        return []
    if tokens[end - 1].value == ">":
        opening = balanced_token_start(tokens, end - 1, "<", ">")
        if opening is None:
            return []
        if opening > 0 and tokens[opening - 1].value == "::":
            path_end = opening - 1
            start = path_end - 1
            while start >= 2 and tokens[start - 1].value == "::" and IDENTIFIER.fullmatch(tokens[start - 2].value):
                start -= 2
            return list(tokens[start:path_end]) + list(tokens[opening:end])
        if tokens[opening].value == "<":
            return list(tokens[opening + 1 : end - 1])
    start = end - 1
    while start >= 2 and tokens[start - 1].value == "::" and IDENTIFIER.fullmatch(tokens[start - 2].value):
        start -= 2
    return list(tokens[start:end])


def function_blocks(tokens: Sequence[Token]) -> list[tuple[list[Token], int, int]]:
    """Return top-level parameter tokens and body bounds for functions with bodies."""
    blocks: list[tuple[list[Token], int, int]] = []
    offset = 0
    while offset + 1 < len(tokens):
        if (
            canonical_identifier(tokens[offset].value) != "fn"
            or not IDENTIFIER.fullmatch(tokens[offset + 1].value)
        ):
            offset += 1
            continue
        opening = offset + 2
        while opening < len(tokens) and tokens[opening].value not in {"(", "{", ";"}:
            opening += 1
        if opening >= len(tokens) or tokens[opening].value != "(":
            offset += 1
            continue
        parameter_end = balanced_token_end(tokens, opening, "(", ")")
        if parameter_end is None:
            break
        body_opening = parameter_end + 1
        while body_opening < len(tokens) and tokens[body_opening].value not in {"{", ";"}:
            body_opening += 1
        if body_opening >= len(tokens) or tokens[body_opening].value != "{":
            offset = parameter_end + 1
            continue
        body_closing = balanced_token_end(tokens, body_opening, "{", "}")
        if body_closing is None:
            break
        blocks.append((list(tokens[opening + 1 : parameter_end]), body_opening, body_closing))
        offset = body_closing + 1
    return blocks


def top_level_colon(tokens: Sequence[Token]) -> int | None:
    depths = {"(": 0, "[": 0, "{": 0, "<": 0}
    closers = {")": "(", "]": "[", "}": "{", ">": "<"}
    for index, token in enumerate(tokens):
        if token.value in depths:
            depths[token.value] += 1
        elif token.value in closers and depths[closers[token.value]]:
            depths[closers[token.value]] -= 1
        elif token.value == ":" and not any(depths.values()):
            return index
    return None


def simple_ref_parameters(
    parameters: Sequence[Token], resolver: TransparentTypeResolver
) -> set[str]:
    bindings: set[str] = set()
    for parameter in split_top_level_tokens(parameters):
        colon = top_level_colon(parameter)
        if colon is None:
            continue
        names = [
            canonical_identifier(token.value)
            for token in parameter[:colon]
            if IDENTIFIER.fullmatch(token.value)
            and canonical_identifier(token.value) not in {"mut", "ref"}
        ]
        if len(names) == 1 and resolver.resolve(parameter[colon + 1 :]) == "Ref":
            bindings.add(names[0])
    return bindings


def simply_shadowed(
    tokens: Sequence[Token], opening: int, call: int, receiver: str
) -> bool:
    for index in range(opening + 1, call):
        if canonical_identifier(tokens[index].value) != "let":
            continue
        binding = index + 1
        while binding < call and canonical_identifier(tokens[binding].value) in {
            "mut",
            "ref",
        }:
            binding += 1
        if (
            binding < call
            and IDENTIFIER.fullmatch(tokens[binding].value)
            and canonical_identifier(tokens[binding].value) == receiver
        ):
            return True
    return False


def ref_instance_use_sites(
    tokens: Sequence[Token], resolver: TransparentTypeResolver, method: str
) -> list[int]:
    uses: list[int] = []
    for parameters, opening, closing in function_blocks(tokens):
        bindings = simple_ref_parameters(parameters, resolver)
        if not bindings:
            continue
        for index in range(opening + 1, closing - 2):
            receiver = canonical_identifier(tokens[index].value)
            if (
                receiver not in bindings
                or tokens[index + 1].value != "."
                or canonical_identifier(tokens[index + 2].value) != method
                or (index > opening + 1 and tokens[index - 1].value in {".", "::"})
                or simply_shadowed(tokens, opening, index, receiver)
            ):
                continue
            uses.append(index + 2)
    for self_type, opening, closing in impl_blocks(tokens):
        if resolver.resolve(self_type) != "Ref":
            continue
        for index in range(opening + 1, closing - 2):
            if (
                canonical_identifier(tokens[index].value) == "self"
                and tokens[index + 1].value == "."
                and canonical_identifier(tokens[index + 2].value) == method
            ):
                uses.append(index + 2)
    return uses


def ref_ufcs_uses(
    corpus: Sequence[tuple[str, str, str, list[Token]]], method: str
) -> list[dict[str, object]]:
    uses: list[Use] = []
    for relative, _source, _searchable, tokens in corpus:
        resolver = TransparentTypeResolver(tokens, relative=relative)
        for index in range(2, len(tokens)):
            if (
                canonical_identifier(tokens[index].value) != method
                or tokens[index - 1].value != "::"
            ):
                continue
            receiver = ufcs_receiver(tokens, index - 1)
            if resolver.resolve(receiver) == "Ref":
                token = receiver[0] if receiver else tokens[index]
                use_index = next(
                    position
                    for position, candidate in enumerate(tokens)
                    if candidate.offset == token.offset
                )
                uses.append(use_at(relative, tokens, use_index))
        uses.extend(
            use_at(relative, tokens, index)
            for index in ref_instance_use_sites(tokens, resolver, method)
        )
    return grouped_uses(uses)


def high_risk_uses(
    corpus: Sequence[tuple[str, str, str, list[Token]]], identifier: str
) -> list[dict[str, object]]:
    sequences = {
        "value-mutable-reference": (
            ("Value", "::", "MutableReference"),
            ("LegacyValue", "::", "MutableReference"),
        ),
        "value-typed-wrapper": (
            ("Value", "::", "Typed"),
            ("LegacyValue", "::", "Typed"),
        ),
    }
    identifiers = {
        "valref-alias": "ValRef",
        "mutable-reference-alias": "MutableReference",
        "reactive-cell-id": "ReactiveCellId",
        "value-state-journal": "ValueStateJournal",
        "reactive-turn-journal": "ReactiveTurnJournal",
        "transaction-state-values-api": "transaction_state_values",
    }
    definitions = {
        "ref-as-ptr-definition": "as_ptr",
        "ref-as-mut-ptr-definition": "as_mut_ptr",
        "ref-addr-definition": "addr",
        "ref-id-definition": "id",
    }
    ufcs = {
        "ref-as-ptr-ufcs": "as_ptr",
        "ref-as-mut-ptr-ufcs": "as_mut_ptr",
        "ref-addr-ufcs": "addr",
        "ref-id-ufcs": "id",
    }
    if identifier in sequences:
        accepted = {tuple(sequence) for sequence in sequences[identifier]}
        return grouped_uses(
            use_at(relative, tokens, index)
            for relative, _source, _searchable, tokens in corpus
            for index in range(len(tokens) - 2)
            if tuple(
                canonical_identifier(token.value)
                for token in tokens[index : index + 3]
            )
            in accepted
        )
    if identifier in identifiers:
        return exact_identifier_uses(corpus, identifiers[identifier])
    if identifier in definitions:
        return ref_method_definition_uses(corpus, definitions[identifier])
    if identifier in ufcs:
        return ref_ufcs_uses(corpus, ufcs[identifier])
    raise ValueError(f"unknown high-risk identifier {identifier}")


def mechanism_records(
    corpus: Sequence[tuple[str, str, str, list[Token]]], identifiers: Iterable[str]
) -> list[dict[str, object]]:
    return [
        {
            "id": identifier,
            "pattern": HIGH_RISK_PATTERNS[identifier],
            "uses": high_risk_uses(corpus, identifier),
        }
        for identifier in identifiers
    ]


def scan_corpus(paths: Sequence[Path], root: Path) -> list[tuple[str, str, str, list[Token]]]:
    corpus: list[tuple[str, str, str, list[Token]]] = []
    for path in paths:
        source = path.read_text(encoding="utf-8")
        searchable = mask_non_code(source)
        corpus.append(
            (path.relative_to(root).as_posix(), source, searchable, rust_tokens(source, searchable))
        )
    return corpus


def scanner_module_sha256(path: Path | None = None) -> str:
    scanner_path = Path(__file__) if path is None else path
    normalized = scanner_path.read_text(encoding="utf-8").replace("\r\n", "\n")
    return hashlib.sha256(normalized.encode("utf-8")).hexdigest()
