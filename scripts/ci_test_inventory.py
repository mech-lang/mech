"""Inventory canonical integration targets against executable CI selections.

This checks target ownership and crate-level feature admission, not executed
test counts, dependency unit tests, platform availability or semantic coverage.
Explicit ignored-case/nonempty-list contracts remain in test_ci_full_contract.
"""

from __future__ import annotations

import fnmatch
import ast
from bisect import bisect_right
import json
import operator
import re
import shlex
import tomllib
from pathlib import Path

from ci_owners import load_owners
from rust_source import rust_code


ROOT = Path(__file__).resolve().parents[1]
CANONICAL_PACKAGES = (
    "src/core", "src/syntax", "src/engine", "src/stdlib",
    "src/runtime", "src/build", "hosts/gpu",
)


def workflow_expression(value: str) -> str:
    value = value.strip()
    # Remove the YAML scalar's quoting before parsing the expression. Quotes
    # inside ${{ ... }} instead belong to GitHub string literals ("false" is
    # a nonempty, truthy string there, not the Boolean false).
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        value = json.loads(value) if value[0] == '"' else value[1:-1].replace("''", "'")
    if value.startswith("${{") and value.endswith("}}"):
        value = value[3:-2].strip()
    return value


def workflow_truth(value: str) -> bool | None:
    """Constant GitHub condition truthiness; dynamic expressions stay unknown."""
    value = workflow_expression(value)
    pattern = re.compile(r"'(?:''|[^'])*'|(?:-?0[xX][0-9a-fA-F]+|-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)|[A-Za-z_][A-Za-z_0-9.]*|&&|\|\||==|!=|<=|>=|[!()<>,]")
    tokens, end = [], 0
    for token in pattern.finditer(value):
        if value[end:token.start()].strip():
            return None
        tokens.append(token[0])
        end = token.end()
    if value[end:].strip():
        return None
    position = 0

    class Unknown:
        # Boolean operators can prove truthiness without proving the value.
        # Keep that distinction for comparisons of parenthesized expressions.
        def __init__(self, truth=None):
            self.truth = truth

    def truth(result):
        return result.truth if isinstance(result, Unknown) else bool(result)

    def atom():
        nonlocal position
        if position == len(tokens):
            raise ValueError("missing condition value")
        token = tokens[position]
        position += 1
        if token == "!":
            result = truth(atom())
            return Unknown() if result is None else not result
        if token == "(":
            result = expression("||")
            if position == len(tokens) or tokens[position] != ")":
                raise ValueError("unclosed condition")
            position += 1
            return result
        if token in {"true", "false", "null"}:
            return None if token == "null" else token == "true"
        if token.startswith("'"):
            return token[1:-1].replace("''", "'")
        if re.match(r"-?(?:\d|\.)", token):
            return int(token, 16) if "x" in token.lower() else float(token)
        if token == ")":
            raise ValueError("unexpected condition close")
        if position < len(tokens) and tokens[position] == "(":
            # Status/context calls are dynamic. Consume the whole invocation
            # so a surrounding `false && ...` remains provably disabled.
            start, depth = position, 0
            while position < len(tokens):
                current = tokens[position]
                position += 1
                depth += (current == "(") - (current == ")")
                if depth == 0:
                    return True if token == "always" and position == start + 2 else Unknown()
            raise ValueError("unclosed condition call")
        return Unknown()

    def number(result):
        if result is None or isinstance(result, bool):
            return float(result or 0)
        if isinstance(result, (int, float)):
            return result
        if not result:
            return 0.0
        try:
            parsed = json.loads(result)
            return parsed if type(parsed) in (int, float) else float("nan")
        except ValueError:
            return float("nan")

    def comparison():
        nonlocal position
        result = atom()
        operations = {"==": operator.eq, "!=": operator.ne, "<": operator.lt,
                      "<=": operator.le, ">": operator.gt, ">=": operator.ge}
        while position < len(tokens) and tokens[position] in operations:
            operation = operations[tokens[position]]
            position += 1
            right = atom()
            if isinstance(result, Unknown) or isinstance(right, Unknown):
                result = Unknown()
            else:
                if isinstance(result, str) and isinstance(right, str):
                    result, right = result.casefold(), right.casefold()
                elif type(result) != type(right) or result is None or isinstance(result, bool):
                    result, right = number(result), number(right)
                result = operation(result, right)
        return result

    def expression(operator):
        nonlocal position
        operand = comparison if operator == "&&" else lambda: expression("&&")
        result = operand()
        while position < len(tokens) and tokens[position] == operator:
            position += 1
            right = operand()
            left_truth, right_truth = truth(result), truth(right)
            if left_truth is None:
                decisive = False if operator == "&&" else True
                result = Unknown(decisive if right_truth is decisive else None)
            elif operator == "&&":
                result = right if left_truth else result
            else:
                result = result if left_truth else right
        return result

    try:
        result = expression("||")
        return truth(result) if position == len(tokens) else None
    except (ValueError, OverflowError):
        return None


def disabled_run(lines: list[str], index: int, indent: int) -> bool:
    def disabled(metadata, expected_indent):
        for offset, line in enumerate(metadata):
            prefix = len(line) - len(line.lstrip())
            text = line.strip()
            if text.startswith("- "):
                prefix, text = prefix + 2, text[2:]
            if prefix != expected_indent:
                continue
            match = re.fullmatch(r"(if|continue-on-error):\s*(.*)", text)
            if match:
                # A '#' inside a YAML/expression string is data, not a comment.
                scalar = re.match(r'''(?:'(?:''|[^'])*'|"(?:\\.|[^"\\])*"|[^'"#])*''', match[2])[0]
                if scalar.strip().startswith(("|", ">")):
                    body = []
                    for following in metadata[offset + 1:]:
                        if following.strip() and len(following) - len(following.lstrip()) <= expected_indent:
                            break
                        body.append(following.strip())
                    scalar = " ".join(body)
                value = workflow_truth(scalar)
                if match[1] == "if" and value is False:
                    return True
                if match[1] == "if" and value is None:
                    # Unsupported constant-only expressions cannot establish
                    # execution. Dynamic impact/status gates remain ownership
                    # candidates, not claims of every platform/trigger running.
                    expression = re.sub(r"'(?:''|[^'])*'", "''", workflow_expression(scalar))
                    if not re.search(r"\b(?:github|inputs|needs|matrix|runner|steps|env|vars|secrets|strategy|job)\.|\b(?:success|failure|cancelled)\s*\(", expression):
                        return True
                if match[1] == "continue-on-error" and value is not False:
                    return True
        return False

    jobs = [offset for offset, line in enumerate(lines[:index]) if re.fullmatch(r"  [a-zA-Z_][a-zA-Z_0-9-]*:\s*", line)]
    if jobs:
        header = []
        for line in lines[jobs[-1] + 1:index]:
            if re.fullmatch(r"    steps:\s*", line):
                break
            header.append(line)
        if disabled(header, 4):
            return True
    steps = [(offset, len(match[1])) for offset, line in enumerate(lines[:index + 1])
             if (match := re.match(r"(\s*)-\s+", line)) and len(match[1]) <= indent]
    if not steps:
        return False
    start, step_indent = steps[-1]
    metadata = []
    for offset in range(start, len(lines)):
        line = lines[offset]
        if offset > start and (match := re.match(r"(\s*)-\s+", line)) and len(match[1]) <= step_indent:
            break
        metadata.append(line)
    return disabled(metadata, step_indent + 2)


def run_blocks(source: str):
    """Read YAML run scalars, not comments, environment values or display names."""
    lines = source.splitlines()
    for index, line in enumerate(lines):
        match = re.fullmatch(r"(\s*)(?:-\s+)?run:\s*(.*)", line)
        if not match:
            continue
        indent, value = len(match[1]), match[2]
        if disabled_run(lines, index, indent):
            continue
        if value.startswith(("|", ">")):
            body = []
            for following in lines[index + 1:]:
                if following.strip() and len(following) - len(following.lstrip()) <= indent:
                    break
                body.append(following.strip())
            yield " ".join(body) if value.startswith(">") else "\n".join(body)
        else:
            yield value


def cargo_test_command(arguments: list[str]):
    if not arguments or arguments[0] != "cargo":
        return None
    arguments = arguments[1:]
    if arguments and arguments[0].startswith("+"):
        arguments = arguments[1:]
    if not arguments or arguments[0] != "test":
        return None
    cargo, harness = arguments[1:], []
    if "--" in cargo:
        index = cargo.index("--")
        cargo, harness = cargo[:index], cargo[index + 1:]
    if "--no-run" in cargo or "--list" in harness:
        return None
    # A successful command that is allowed to fail is not ownership evidence.
    if any(token in {"||", "&&", ";"} for token in cargo + harness):
        return None
    packages, targets, features, filters = [], [], [], []
    index = 0
    while index < len(cargo):
        token = cargo[index]
        if token in {"-p", "--package", "--test", "--features"}:
            index += 1
            if index == len(cargo):
                return None
            destination = packages if token in {"-p", "--package"} else targets if token == "--test" else features
            destination.append(cargo[index])
        elif token.startswith(("--package=", "--test=", "--features=")):
            key, value = token.split("=", 1)
            destination = packages if key == "--package" else targets if key == "--test" else features
            destination.append(value)
        elif token in {"-j", "--jobs", "--manifest-path", "--target-dir", "--target", "--profile", "--color", "--config", "--message-format", "--exclude", "-Z"}:
            index += 1
        elif not token.startswith("-"):
            filters.append(token)
        index += 1
    explicit_kind = any(
        flag in cargo for flag in ("--lib", "--bin", "--bins", "--example", "--examples", "--bench", "--benches", "--doc")
    ) or any(token.startswith(("--bin=", "--example=", "--bench=")) for token in cargo)
    all_targets = "--tests" in cargo or "--all-targets" in cargo or (not targets and not explicit_kind)
    skips = []
    index = 0
    while index < len(harness):
        token = harness[index]
        if token in {"--skip", "--test-threads", "--format", "--logfile"}:
            index += 1
            if index == len(harness):
                return None
            if token == "--skip":
                skips.append(harness[index])
        elif token.startswith("--skip="):
            skips.append(token.split("=", 1)[1])
        elif not token.startswith("-"):
            filters.append(token)
        index += 1
    return {
        "packages": packages, "targets": targets, "all_targets": all_targets,
        "features": set(filter(None, re.split(r"[\s,]+", " ".join(features)))),
        "default": "--no-default-features" not in cargo,
        "all_features": "--all-features" in cargo,
        "filters": filters, "skips": skips, "exact": "--exact" in harness,
        "ignored_mode": "all" if "--include-ignored" in harness else "only" if "--ignored" in harness else "ordinary",
    }


def shell_commands(block: str) -> list[list[str]]:
    """Admit straight-line shell commands, not a general shell interpreter.

    Compound/indirect shell control requires an explicit flat CI selection;
    those blocks supply no ownership. A terminating command also prevents any
    later line from supplying direct, recorded-executable or generator credit.
    Parse the whole supported prefix before returning commands, so a heredoc,
    function body or multiline string cannot masquerade as shell execution.
    """
    commands = []
    block = re.sub(r"\\\n\s*", " ", block)
    block = re.sub(r"\$\{\{.*?\}\}", "__GITHUB_EXPRESSION__", block, flags=re.S)
    for line in block.splitlines():
        lexer = shlex.shlex(line, posix=True, punctuation_chars=";&|<>()")
        lexer.whitespace_split = True
        lexer.commenters = "#"
        try:
            arguments = list(lexer)
        except ValueError:
            return []
        if not arguments:
            continue
        if any(token and all(character in ";&|<>()" for character in token) for token in arguments):
            return []
        while arguments and re.match(r"^[A-Za-z_][A-Za-z_0-9]*=", arguments[0]):
            arguments = arguments[1:]
        if not arguments:
            continue
        if arguments[0] in {
            "if", "then", "elif", "else", "fi", "case", "esac", "for", "while",
            "until", "do", "done", "function", "{", "}", "!", "time", "coproc",
            "eval", "source", ".", "trap", "builtin", "command", "alias", "unalias",
            "enable", "shopt", "break", "continue",
        } or any(marker in arguments[0] for marker in ("$", "`", "__GITHUB_EXPRESSION__")):
            return []
        if arguments[0] == "set" and any(
            token not in {"pipefail", "errexit", "nounset", "xtrace"}
            and not (token.startswith("-") and set(token[1:]) <= set("euox"))
            for token in arguments[1:]
        ):
            return []
        if arguments[0] in {"exit", "return", "exec", "false", "logout"}:
            break
        commands.append(arguments)
    return commands


def workflow_commands(source: str):
    recorded = {}
    for block in run_blocks(source):
        for arguments in shell_commands(block):
            if arguments[0] not in {"cargo", "python3"}:
                continue
            try:
                # This maintained wrapper runs the command after `--` and
                # preserves its exit status; other Python/text mentions do not.
                if arguments[:2] == ["python3", "scripts/run-native-plan-stage.py"] and "--" in arguments:
                    controls, payload = arguments[:arguments.index("--")], arguments[arguments.index("--") + 1:]
                    if "--test-executable-file" in controls:
                        receipt = controls[controls.index("--test-executable-file") + 1]
                        if "--record-test-executable" in controls:
                            target = controls[controls.index("--record-test-executable") + 1]
                            compiled = cargo_test_command([argument for argument in payload if argument != "--no-run"])
                            if compiled is not None and target in compiled["targets"] and "--no-run" in payload:
                                recorded[receipt] = [argument for argument in payload if argument != "--no-run"]
                            continue
                        if receipt not in recorded:
                            continue
                        arguments = [*recorded[receipt], "--", *payload]
                    else:
                        arguments = payload
                command = cargo_test_command(arguments)
            except ValueError:
                continue
            if command is not None:
                yield command


def invoked_generator_commands(root: Path, sources: list[str]):
    """Inspect literal commands passed to the maintained generator runners.

    Dead command arrays, display text, and an uninvoked helper supply no credit.
    These two helpers' execution/exit propagation has separate CI contracts.
    """
    invoked = set()
    for source in sources:
        for block in run_blocks(source):
            for arguments in shell_commands(block):
                if arguments[:2] == ["python3", "scripts/run-native-plan-stage.py"] and "--" in arguments:
                    controls = arguments[:arguments.index("--")]
                    if "--test-executable-file" in controls:
                        continue
                    arguments = arguments[arguments.index("--") + 1:]
                if len(arguments) >= 2 and arguments[0] == "python3":
                    invoked.add(arguments[1])
    for relative in ("scripts/check-generated-project-determinism.py", "scripts/check-native-application-graphs.py"):
        if relative not in invoked:
            continue
        module = ast.parse((root / relative).read_text(encoding="utf-8"))
        yield from reachable_generator_commands(module)


def reachable_generator_commands(module: ast.Module):
    """Follow the script entry point, not every function or syntactic call.

    Credit only straight-line calls and statically selected branches. Unknown
    branches, deferred expressions, exception handlers and loop bodies are not
    evidence of execution; maintained generator invocations need none of them.
    """
    functions = {node.name: node for node in module.body if isinstance(node, ast.FunctionDef)}
    visited = set()

    def constant(node):
        if isinstance(node, ast.Name) and node.id == "__name__":
            return "__main__"
        if isinstance(node, ast.Compare) and len(node.ops) == len(node.comparators) == 1:
            left, right = constant(node.left), constant(node.comparators[0])
            if left is not unknown and right is not unknown:
                if isinstance(node.ops[0], ast.Eq):
                    return left == right
                if isinstance(node.ops[0], ast.NotEq):
                    return left != right
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.Not):
            value = constant(node.operand)
            return unknown if value is unknown else not value
        try:
            return ast.literal_eval(node)
        except (ValueError, TypeError, SyntaxError):
            return unknown

    unknown = object()

    def expression(node, assignments):
        if isinstance(node, (ast.Lambda, ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)):
            return
        if isinstance(node, ast.BoolOp):
            for value in node.values:
                yield from expression(value, assignments)
                result = constant(value)
                if result is unknown or (not result if isinstance(node.op, ast.And) else bool(result)):
                    break
            return
        if isinstance(node, ast.IfExp):
            yield from expression(node.test, assignments)
            condition = constant(node.test)
            if condition is not unknown:
                yield from expression(node.body if condition else node.orelse, assignments)
            return
        for child in ast.iter_child_nodes(node):
            yield from expression(child, assignments)
        if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Name):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and isinstance(node.func.value, ast.Name):
                assignments.pop(node.func.value.id, None)
            return
        if node.func.id in {"run_streamed", "execute_streamed"} and node.args:
            argument = node.args[0]
            if isinstance(argument, ast.Name):
                argument = assignments.get(argument.id)
            try:
                arguments = ast.literal_eval(argument)
            except (ValueError, TypeError, SyntaxError):
                return
            if isinstance(arguments, (list, tuple)) and all(isinstance(value, str) for value in arguments):
                command = cargo_test_command(list(arguments))
                if command is not None:
                    yield command
        elif node.func.id in functions and node.func.id not in visited:
            visited.add(node.func.id)
            yield from statements(functions[node.func.id].body, {})

    def statements(body, assignments):
        for statement in body:
            if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                continue
            if isinstance(statement, ast.If):
                yield from expression(statement.test, assignments)
                condition = constant(statement.test)
                if condition is not unknown:
                    if (yield from statements(statement.body if condition else statement.orelse, assignments)):
                        return True
                else:
                    for child in ast.walk(statement):
                        if isinstance(child, ast.Name) and isinstance(child.ctx, (ast.Store, ast.Del)):
                            assignments.pop(child.id, None)
            elif isinstance(statement, (ast.With, ast.AsyncWith)):
                for item in statement.items:
                    yield from expression(item.context_expr, assignments)
                if (yield from statements(statement.body, assignments)):
                    return True
            elif isinstance(statement, (ast.For, ast.AsyncFor)):
                yield from expression(statement.iter, assignments)
                # The iterable itself is evaluated before the loop, but neither
                # a body invocation nor reaching the next statement is assured.
                return True
            elif isinstance(statement, ast.While):
                yield from expression(statement.test, assignments)
                condition = constant(statement.test)
                if condition is unknown or condition:
                    return True
            elif isinstance(statement, (ast.Try, ast.TryStar)):
                terminated = yield from statements(statement.body, assignments)
                if (yield from statements(statement.finalbody, assignments)) or terminated:
                    return True
                if (yield from statements(statement.orelse, assignments)):
                    return True
            elif isinstance(statement, ast.Match):
                yield from expression(statement.subject, assignments)
                return True
            elif isinstance(statement, (ast.Assign, ast.AnnAssign, ast.AugAssign, ast.Expr, ast.Return, ast.Raise, ast.Assert, ast.Delete, ast.Break, ast.Continue)):
                for child in ast.iter_child_nodes(statement):
                    yield from expression(child, assignments)
                if isinstance(statement, ast.Assign):
                    for target in statement.targets:
                        if isinstance(target, ast.Name):
                            assignments[target.id] = statement.value
                elif isinstance(statement, (ast.AugAssign, ast.AnnAssign)) and isinstance(statement.target, ast.Name):
                    assignments.pop(statement.target.id, None)
                elif isinstance(statement, ast.Delete):
                    for target in statement.targets:
                        if isinstance(target, ast.Name):
                            assignments.pop(target.id, None)
                if isinstance(statement, ast.Assert):
                    condition = constant(statement.test)
                    if condition is not unknown and not condition:
                        return True
                if isinstance(statement, (ast.Return, ast.Raise, ast.Break, ast.Continue)):
                    return True
        return False

    yield from statements(module.body, {})


def feature_closure(manifest: dict, command: dict) -> set[str]:
    declared = manifest.get("features", {})
    enabled = set(command["features"])
    if command["default"]:
        enabled.add("default")
    if command["all_features"]:
        enabled.update(declared)
    pending = list(enabled)
    while pending:
        for dependency in declared.get(pending.pop(), []):
            if "/" in dependency or dependency.startswith("dep:") or dependency in enabled:
                continue
            enabled.add(dependency)
            pending.append(dependency)
    return enabled


def cfg_predicate(expression: str, enabled: set[str]) -> bool | None:
    """Evaluate feature conditions; other target predicates remain unknown."""
    token_pattern = re.compile(r'r(?P<hashes>#{0,255})"[\s\S]*?"(?P=hashes)|"(?:\\.|[^"\\])*"|(?:r#)?[A-Za-z_][A-Za-z_0-9]*|[=(),]')
    tokens, end = [], 0
    for token in token_pattern.finditer(expression):
        if expression[end:token.start()].strip():
            raise ValueError("invalid cfg token")
        tokens.append(token[0])
        end = token.end()
    if expression[end:].strip():
        raise ValueError("invalid trailing cfg token")
    position = 0

    def parse():
        nonlocal position
        if position >= len(tokens):
            raise ValueError("incomplete cfg predicate")
        name = tokens[position].removeprefix("r#")
        position += 1
        if position < len(tokens) and tokens[position] == "=":
            position += 1
            if position == len(tokens):
                raise ValueError("missing cfg value")
            literal = tokens[position]
            raw = re.match(r'r(#{0,255})"', literal)
            value = literal[raw.end():-len(raw[1]) - 1] if raw else json.loads(literal)
            position += 1
            return value in enabled if name == "feature" else None
        if position < len(tokens) and tokens[position] == "(":
            position += 1
            values = []
            while position < len(tokens) and tokens[position] != ")":
                values.append(parse())
                if position < len(tokens) and tokens[position] == ",":
                    position += 1
                elif position >= len(tokens) or tokens[position] != ")":
                    raise ValueError("malformed cfg argument list")
            if position == len(tokens):
                raise ValueError("unclosed cfg predicate")
            position += 1
            if name == "all":
                return False if False in values else None if None in values else True
            if name == "any":
                return True if True in values else None if None in values else False
            if name == "not" and len(values) == 1:
                return None if values[0] is None else not values[0]
            raise ValueError(f"unsupported cfg operator {name}")
        return True if name == "test" else None

    result = parse()
    if position != len(tokens):
        raise ValueError("trailing cfg tokens")
    return result


def attribute_arguments(source: str) -> list[str]:
    masked = rust_code(source)
    parts, start, depth = [], 0, 0
    for index, character in enumerate(masked):
        if character in "([{":
            depth += 1
        elif character in ")]}":
            depth -= 1
        elif character == "," and depth == 0:
            parts.append(source[start:index].strip())
            start = index + 1
    parts.append(source[start:].strip())
    return [part for part in parts if part]


def attribute_cfgs(attribute: str):
    match = re.fullmatch(r"\s*(?:r#)?(cfg|cfg_attr)\s*\((.*)\)\s*", attribute, re.S)
    if match is None:
        return
    if match[1] == "cfg":
        yield match[2]
        return
    parts = attribute_arguments(match[2])
    if len(parts) < 2:
        raise ValueError("cfg_attr requires a condition and an attribute")
    condition, *attributes = parts
    for nested in attributes:
        for predicate in attribute_cfgs(nested):
            yield f"any(not({condition}), {predicate})"


def crate_cfgs(source: str):
    preserved = rust_code(source, preserve_literals=True)
    masked, position = rust_code(source), 0
    # Only leading inner attributes gate the crate. An inner cfg on a later
    # helper module must not be mistaken for the whole integration target.
    while position < len(masked):
        while position < len(masked) and (masked[position].isspace() or masked[position] == "\ufeff"):
            position += 1
        match = re.compile(r"#\s*!\s*\[").match(masked, position)
        if match is None:
            return
        position, start, depth = match.end(), match.end(), 1
        while position < len(masked) and depth:
            if masked[position] == "[":
                depth += 1
            elif masked[position] == "]":
                depth -= 1
            position += 1
        if depth:
            raise ValueError("unclosed crate attribute")
        yield from attribute_cfgs(preserved[start:position - 1])


def attribute_ignores(attribute: str):
    if re.fullmatch(r"\s*(?:r#)?ignore(?:\s*=.*)?\s*", attribute, re.S):
        yield "all()"
        return
    match = re.fullmatch(r"\s*(?:r#)?cfg_attr\s*\((.*)\)\s*", attribute, re.S)
    if match is not None:
        condition, *attributes = attribute_arguments(match[1])
        for nested in attributes:
            for predicate in attribute_ignores(nested):
                yield f"all({condition}, {predicate})"


def registered_cases(source: str):
    masked = rust_code(source)
    preserved = rust_code(source, preserve_literals=True)
    # Match complete adjacent outer-attribute blocks, so ignore-before-test
    # and ignore-after-test have identical harness selection semantics.
    spans = []
    for match in re.finditer(r"#\s*\[", masked):
        if spans and match.start() < spans[-1][1]:
            continue
        position, depth = match.end(), 1
        while position < len(masked) and depth:
            if masked[position] == "[":
                depth += 1
            elif masked[position] == "]":
                depth -= 1
            position += 1
        if depth:
            raise ValueError("unclosed test attribute")
        spans.append((match.start(), position, match.end(), position - 1))
    ends = [span[1] for span in spans]

    def preceding_attributes(position):
        index, before, attributes = bisect_right(ends, position) - 1, position, []
        while index >= 0 and not masked[spans[index][1]:before].strip():
            start, _, content_start, content_end = spans[index]
            attributes.append(preserved[content_start:content_end])
            before, index = start, index - 1
        return attributes

    # Harness names include their module path. Both outer module attributes
    # and leading inner module attributes gate every descendant test case.
    closes, stack = {}, []
    for position, character in enumerate(masked):
        if character == "{":
            stack.append(position)
        elif character == "}" and stack:
            closes[stack.pop()] = position
    modules = []
    pattern = r"(?:pub(?:\([^)]*\))?\s+)?mod\s+((?:r#)?[A-Za-z_][A-Za-z_0-9]*)\s*\{"
    for match in re.finditer(pattern, masked):
        opening = match.end() - 1
        if opening not in closes:
            raise ValueError("unclosed test module")
        cfgs = [predicate for attribute in preceding_attributes(match.start()) for predicate in attribute_cfgs(attribute)]
        cfgs.extend(crate_cfgs(preserved[opening + 1:closes[opening]]))
        modules.append((opening, closes[opening], match[1].removeprefix("r#"), cfgs))

    pattern = r"(?:pub(?:\([^)]*\))?\s+)?fn\s+((?:r#)?[A-Za-z_][A-Za-z_0-9]*)\s*\("
    for match in re.finditer(pattern, masked):
        attributes = preceding_attributes(match.start())
        if not any(re.fullmatch(r"\s*(?:r#)?test\s*", attribute) for attribute in attributes):
            continue
        parents = [module for module in modules if module[0] < match.start() < module[1]]
        yield {
            "name": "::".join([module[2] for module in parents] + [match[1].removeprefix("r#")]),
            "cfgs": [predicate for module in parents for predicate in module[3]]
                    + [predicate for attribute in attributes for predicate in attribute_cfgs(attribute)],
            "ignores": [predicate for attribute in attributes for predicate in attribute_ignores(attribute)],
        }


def integration_targets(directory: Path, manifest: dict):
    targets = {}
    if manifest.get("package", {}).get("autotests", True):
        targets.update((path.stem, {"path": path, "required-features": []}) for path in directory.glob("tests/*.rs"))
        targets.update((path.parent.name, {"path": path, "required-features": []}) for path in directory.glob("tests/*/main.rs"))
    for declaration in manifest.get("test", []):
        name = declaration["name"]
        path = directory / declaration.get("path", f"tests/{name}.rs")
        targets[name] = {"path": path, "required-features": declaration.get("required-features", [])}
    return targets


def target_covered(package: str, name: str, target: dict, manifest: dict, commands: list[dict]) -> bool:
    source = target["path"].read_text(encoding="utf-8")
    predicates = list(crate_cfgs(source))
    cases = list(registered_cases(source))
    for command in commands:
        if package not in command["packages"]:
            continue
        if not command["all_targets"] and not any(fnmatch.fnmatchcase(name, pattern) for pattern in command["targets"]):
            continue
        enabled = feature_closure(manifest, command)
        selected = []
        for case in cases:
            if any(cfg_predicate(predicate, enabled) is False for predicate in case["cfgs"]):
                continue
            ignored = [cfg_predicate(predicate, enabled) for predicate in case["ignores"]]
            if command["ignored_mode"] == "only" and not any(value is not False for value in ignored):
                continue
            if command["ignored_mode"] == "ordinary" and True in ignored:
                continue
            if not any(skip in case["name"] for skip in command["skips"]):
                selected.append(case["name"])
        if (cases or command["ignored_mode"] == "only") and not selected:
            continue
        if command["filters"] and not any(
            any(case == pattern if command["exact"] else pattern in case for pattern in command["filters"])
            for case in selected
        ):
            continue
        if not set(target["required-features"]).issubset(enabled):
            continue
        if all(cfg_predicate(predicate, enabled) is not False for predicate in predicates):
            return True
    return False


def inventory_gaps(root: Path = ROOT) -> list[str]:
    commands = []
    sources = []
    for path in sorted((root / ".github/workflows").glob("*.yml")):
        sources.append(path.read_text(encoding="utf-8"))
    for path in sorted((root / ".github/actions").glob("*/action.yml")):
        sources.append(path.read_text(encoding="utf-8"))
    for source in sources:
        commands.extend(workflow_commands(source))
    commands.extend(invoked_generator_commands(root, sources))
    commands.extend(
        command for owner in load_owners(root / ".github/ci/owners.toml").values()
        if (command := cargo_test_command(owner["command"])) is not None
    )
    gaps = []
    for relative in CANONICAL_PACKAGES:
        directory = root / relative
        manifest = tomllib.loads((directory / "Cargo.toml").read_text(encoding="utf-8"))
        package = manifest["package"]["name"]
        for name, target in sorted(integration_targets(directory, manifest).items()):
            if not target_covered(package, name, target, manifest, commands):
                gaps.append(f"{package}:{name} ({target['path'].relative_to(root)})")
    return gaps
