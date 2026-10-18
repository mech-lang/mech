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
from ci_workflow import step_contexts, workflow_document
from rust_source import rust_code


ROOT = Path(__file__).resolve().parents[1]
CANONICAL_PACKAGES = (
    "src/core", "src/syntax", "src/engine", "src/stdlib",
    "src/runtime", "src/build", "hosts/gpu",
)


def workflow_expression(value: str) -> str:
    # YAML decoding belongs to ci_workflow, not the expression grammar.
    value = value.strip()
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


def disabled_metadata(metadata: dict) -> bool:
    for key in ("if", "continue-on-error"):
        if key not in metadata:
            continue
        scalar = metadata[key]
        if not isinstance(scalar, str):
            return True
        value = workflow_truth(scalar)
        if key == "if" and value is False:
            return True
        if key == "if" and value is None:
            # Dynamic trigger/impact gates remain ownership candidates, not
            # proof that every platform or trigger executes this profile.
            expression = re.sub(r"'(?:''|[^'])*'", "''", workflow_expression(scalar))
            if not re.search(r"\b(?:github|inputs|needs|matrix|runner|steps|env|vars|secrets|strategy|job)\.|\b(?:success|failure|cancelled)\s*\(", expression):
                return True
        if key == "continue-on-error" and value is not False:
            return True
    return False


def unproven_environment_variable(name: str) -> bool:
    """Explicit overrides that can replace a command, compiler or test runner."""
    name = name.upper()
    # Target-specific Cargo configuration controls not just execution runners
    # but also linkers, flags and build-script link overrides. None of that
    # configuration is modeled here. TARGET_DIR is only the shared cache path,
    # not a target-specific execution setting.
    if name.startswith("CARGO_TARGET_") and name != "CARGO_TARGET_DIR":
        return True
    return name in {
        "PATH", "PATHEXT", "COMSPEC", "IFS", "BASH_ENV", "ENV", "SHELLOPTS", "BASHOPTS",
        "BASH_CMDS", "BASH_ALIASES", "CDPATH", "ZDOTDIR", "HOME", "USERPROFILE",
        "CARGO_HOME", "RUSTUP_HOME", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
        "RUSTDOC", "RUSTFLAGS", "RUSTDOCFLAGS", "PYTHONPATH", "PYTHONHOME", "PYTHONSTARTUP",
        "LD_PRELOAD", "DYLD_INSERT_LIBRARIES",
    } or re.fullmatch(r"CARGO_(?:BUILD_(?:RUNNER|RUSTC(?:_WRAPPER|_WORKSPACE_WRAPPER)?|RUSTDOC|RUST(?:DOC)?FLAGS)|ENCODED_RUST(?:DOC)?FLAGS|PROFILE_.+_RUST(?:DOC)?FLAGS)", name) is not None


def admitted_steps(source: str):
    for document, job, step in step_contexts(source):
        if disabled_metadata(job) or disabled_metadata(step):
            continue
        environment, valid = {}, True
        for context in (document, job, step):
            mapping = context.get("env", {})
            if not isinstance(mapping, dict) or any(not isinstance(key, str) or not isinstance(value, str)
                                                   for key, value in mapping.items()):
                valid = False
                break
            environment.update(mapping)
        if not valid or any(unproven_environment_variable(name) for name in environment):
            continue
        defaults, valid = {}, True
        for context in (document, job):
            mapping = context.get("defaults", {})
            if not isinstance(mapping, dict) or not isinstance(mapping.get("run", {}), dict):
                valid = False
                break
            defaults.update(mapping.get("run", {}))
        directory = step.get("working-directory", defaults.get("working-directory", "."))
        if not valid or not isinstance(directory, str) or not directory.strip():
            continue
        directory = re.sub(r"\$\{\{\s*github\.workspace\s*\}\}", ".", directory.strip())
        if directory.startswith("/") or any(part not in {"", "."} for part in directory.split("/")):
            continue
        shell = step.get("shell", defaults.get("shell"))
        if job is document.get("runs") and "run" in step and "shell" not in step:
            continue  # composite run steps have no implicit workflow shell
        if shell is not None and (not isinstance(shell, str) or shell not in {"bash", "sh", "pwsh", "powershell"}):
            continue  # custom shells/noexec templates cannot prove execution
        yield step


def run_blocks(source: str):
    """Read real executable step properties after all inherited job settings."""
    for step in admitted_steps(source):
        if isinstance(step.get("run"), str):
            yield step["run"]


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
    if "--no-run" in cargo or any(token in {"--help", "-h", "--version"} for token in cargo):
        return None
    if any(token in {"--list", "--bench", "--help", "-h", "--version"}
           or token.startswith("--bench=") for token in harness):
        return None
    # A successful command that is allowed to fail is not ownership evidence.
    if any(token in {"||", "&&", ";"} for token in cargo + harness):
        return None
    packages, targets, features, filters, excludes = [], [], [], [], []
    selections = {"-p": packages, "--package": packages, "--test": targets,
                  "-F": features, "--features": features, "--exclude": excludes}
    valued = {*selections, "-j", "--jobs", "--manifest-path", "--target-dir",
              "--target", "--profile", "--color", "--message-format",
              "--bin", "--example", "--bench"}
    flags = {"--locked", "--offline", "--frozen", "--release", "--no-default-features",
             "--all-features", "--tests", "--all-targets", "--workspace", "--all",
             "--lib", "--bins", "--examples", "--benches", "--doc", "--quiet", "-q",
             "--verbose", "-v", "-vv", "--keep-going", "--future-incompat-report", "--timings"}
    index = 0
    while index < len(cargo):
        token = cargo[index]
        key, separator, value = token.partition("=")
        if key in valued:
            if not separator:
                index += 1
                if index == len(cargo):
                    return None
                value = cargo[index]
            if not value or value.startswith("-"):
                return None
            if key == "--manifest-path" and value not in {"Cargo.toml", "./Cargo.toml"}:
                return None  # another manifest is not this reviewed workspace
            if key == "--exclude" and not re.fullmatch(r"[A-Za-z0-9_*?\[\].+-]+", value):
                return None  # unsupported package specs cannot prove selection
            if key in selections:
                selections[key].append(value)
        elif token in flags:
            pass
        elif token.startswith(("-p", "-F", "-j")) and len(token) > 2 and not token.startswith("--"):
            if token[:2] in selections:
                selections[token[:2]].append(token[2:])
        elif token.startswith("-"):
            # Output-only/unstable modes and configuration (including runner
            # overrides) are not ordinary, proven test execution. Fail closed
            # on unknown options instead of discarding their effect.
            return None
        else:
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
        if token in {"--skip", "--test-threads", "--format", "--logfile", "--color"}:
            index += 1
            if index == len(harness):
                return None
            if token == "--skip":
                skips.append(harness[index])
        elif token.startswith("--skip="):
            skips.append(token.split("=", 1)[1])
        elif not token.startswith("-"):
            filters.append(token)
        elif token not in {"--exact", "--ignored", "--include-ignored", "--nocapture", "--show-output", "--test", "--quiet", "-q"} and not token.startswith(("--test-threads=", "--format=", "--logfile=", "--color=")):
            return None  # unknown selection/mode flags cannot establish ownership
        index += 1
    return {
        "packages": packages, "excludes": excludes, "targets": targets, "all_targets": all_targets,
        "features": set(filter(None, re.split(r"[\s,]+", " ".join(features)))),
        "default": "--no-default-features" not in cargo,
        "all_features": "--all-features" in cargo,
        "filters": filters, "skips": skips, "exact": "--exact" in harness,
        "ignored_mode": "all" if "--include-ignored" in harness else "only" if "--ignored" in harness else "ordinary",
    }


def supported_shell_command(arguments: list[str]) -> bool:
    """Bounded command forms; arbitrary preceding programs can replace Cargo."""
    if any(marker in argument for argument in arguments for marker in ("$", "`", "__GITHUB_EXPRESSION__")):
        return False
    command = arguments[0]
    if command in {"set", "true", ":", "echo", "test", "["}:
        return True
    if command == "printf":
        return "-v" not in arguments and not any("%n" in argument for argument in arguments[1:])
    if command == "cargo":
        tail = arguments[1:]
        if tail and tail[0].startswith("+"):
            tail = tail[1:]
        return bool(tail) and (tail[0] in {"test", "check", "fetch", "build"}
                               or (tail[0] == "fmt" and "--check" in tail))
    if command != "python3" or len(arguments) < 2:
        return False
    if arguments[1] in {"scripts/check-generated-project-determinism.py", "scripts/check-native-application-graphs.py"}:
        return True
    if arguments[1] != "scripts/run-native-plan-stage.py" or "--" not in arguments:
        return False
    separator = arguments.index("--")
    controls, payload = arguments[:separator], arguments[separator + 1:]
    if not payload:
        return False
    if "--test-executable-file" in controls and "--record-test-executable" not in controls:
        return cargo_test_command(["cargo", "test", "-p", "recorded", "--test", "recorded", "--", *payload]) is not None
    return supported_shell_command(payload)


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
            if any(marker in arguments[0] for marker in ("$", "`", "__GITHUB_EXPRESSION__")) or unproven_environment_variable(arguments[0].split("=", 1)[0]):
                return []
            arguments = arguments[1:]
        if not arguments:
            continue
        if arguments[0] in {
            "if", "then", "elif", "else", "fi", "case", "esac", "for", "while",
            "until", "do", "done", "function", "{", "}", "!", "time", "coproc",
            "eval", "source", ".", "trap", "builtin", "command", "alias", "unalias",
            "enable", "shopt", "hash", "cd", "export", "unset", "readonly",
            "declare", "typeset", "local", "break", "continue",
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
        if not supported_shell_command(arguments):
            return []
        commands.append(arguments)
    return commands


def workflow_commands(source: str):
    recorded = {}
    for block in run_blocks(source):
        for arguments in shell_commands(block):
            if arguments[0] not in {"cargo", "python3"}:
                continue
            if any(marker in argument for argument in arguments
                   for marker in ("$", "`", "__GITHUB_EXPRESSION__")):
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
                    if any(marker in argument for argument in arguments
                           for marker in ("$", "`", "__GITHUB_EXPRESSION__")):
                        continue
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

    class Scope(dict):
        def __init__(self, aliases=None):
            super().__init__()
            self.aliases = dict(aliases or {})

    module_scope = Scope()

    class ProcessExit(Exception):
        pass

    class CommandValue:
        # Aliases share invalidation, but rebinding one name is not a mutation
        # of its old list. Unknown values are never execution evidence.
        def __init__(self, node):
            self.node, self.valid = node, True

    def call_name(node, assignments):
        if isinstance(node, ast.Name):
            return assignments.aliases.get(node.id, node.id) or "<unknown>"
        if isinstance(node, ast.Attribute):
            return f"{call_name(node.value, assignments)}.{node.attr}"
        return ""

    def invalidate(node, assignments):
        for child in ast.walk(node):
            if isinstance(child, ast.Name) and child.id in assignments:
                assignments[child.id].valid = False

    def assigned_value(node, assignments):
        if isinstance(node, ast.Name):
            return assignments.get(node.id)
        try:
            value = ast.literal_eval(node)
        except (ValueError, TypeError, SyntaxError):
            # Putting a command into an unmodeled container can expose it to a
            # later alias mutation. Do not retain speculative ownership.
            invalidate(node, assignments)
            return None
        return CommandValue(node) if isinstance(value, (list, tuple)) else None

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
        if isinstance(node, ast.NamedExpr):
            # Unmodeled expression-level binding may expose a list alias.
            invalidate(node.value, assignments)
            assignments.pop(node.target.id, None)
            return
        if isinstance(node, ast.Call) and (call_name(node.func, assignments).startswith("<unknown>") or call_name(node.func, assignments) in {
            "exit", "quit", "sys.exit", "os._exit", "builtins.exit", "builtins.quit",
        }):
            # Arguments run first. An exit in a called helper terminates the
            # entry point too, unlike an ordinary return from that helper.
            raise ProcessExit
        if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Name):
            if isinstance(node, ast.Call):
                invalidate(node, assignments)
            return
        if node.func.id in {"run_streamed", "execute_streamed"} and node.args:
            argument = node.args[0]
            if isinstance(argument, ast.Name):
                value = assignments.get(argument.id)
                if value is None or not value.valid:
                    return
                argument = value.node
            try:
                arguments = ast.literal_eval(argument)
            except (ValueError, TypeError, SyntaxError):
                return
            if isinstance(arguments, (list, tuple)) and all(isinstance(value, str) for value in arguments):
                command = cargo_test_command(list(arguments))
                if command is not None:
                    yield command
        elif node.func.id in functions:
            invalidate(functions[node.func.id], assignments)
            if node.func.id not in visited:
                visited.add(node.func.id)
                # Python functions use module globals, never their caller's
                # local imports. A callee's imports stay local to that scope.
                yield from statements(functions[node.func.id].body, Scope(module_scope.aliases))
            for argument in (*node.args, *[keyword.value for keyword in node.keywords]):
                invalidate(argument, assignments)
        else:
            for argument in (*node.args, *[keyword.value for keyword in node.keywords]):
                invalidate(argument, assignments)

    def statements(body, assignments):
        for statement in body:
            if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                continue
            if isinstance(statement, (ast.Import, ast.ImportFrom)):
                for alias in statement.names:
                    name = alias.asname or alias.name.split(".")[0]
                    value = alias.name if isinstance(statement, ast.Import) else f"{statement.module}.{alias.name}"
                    assignments.aliases[name] = value
                    assignments.pop(name, None)
            elif isinstance(statement, ast.If):
                yield from expression(statement.test, assignments)
                condition = constant(statement.test)
                if condition is not unknown:
                    if (yield from statements(statement.body if condition else statement.orelse, assignments)):
                        return True
                else:
                    invalidate(statement, assignments)
                    for child in ast.walk(statement):
                        if isinstance(child, ast.Name) and isinstance(child.ctx, (ast.Store, ast.Del)):
                            assignments.pop(child.id, None)
                            if child.id in assignments.aliases:
                                assignments.aliases[child.id] = None
                        if isinstance(child, (ast.Import, ast.ImportFrom)):
                            for alias in child.names:
                                assignments.aliases[alias.asname or alias.name.split(".")[0]] = None
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
                    value = assigned_value(statement.value, assignments)
                    for target in statement.targets:
                        if isinstance(target, ast.Name):
                            alias = call_name(statement.value, assignments)
                            if target.id in assignments.aliases or target.id in {"sys", "os", "builtins", "exit", "quit"} or alias in {"sys", "os", "builtins", "exit", "quit", "sys.exit", "os._exit", "builtins.exit", "builtins.quit"}:
                                assignments.aliases[target.id] = alias if alias else None
                            if value is None:
                                assignments.pop(target.id, None)
                            else:
                                assignments[target.id] = value
                        else:
                            invalidate(target, assignments)
                            invalidate(statement.value, assignments)
                elif isinstance(statement, ast.AnnAssign):
                    value = assigned_value(statement.value, assignments) if statement.value else None
                    if isinstance(statement.target, ast.Name):
                        if value is None:
                            assignments.pop(statement.target.id, None)
                        else:
                            assignments[statement.target.id] = value
                    else:
                        invalidate(statement.target, assignments)
                        if statement.value:
                            invalidate(statement.value, assignments)
                elif isinstance(statement, ast.AugAssign):
                    invalidate(statement.target, assignments)
                    invalidate(statement.value, assignments)
                    if isinstance(statement.target, ast.Name):
                        assignments.pop(statement.target.id, None)
                elif isinstance(statement, ast.Delete):
                    for target in statement.targets:
                        if isinstance(target, ast.Name):
                            assignments.pop(target.id, None)
                        else:
                            invalidate(target, assignments)
                if isinstance(statement, ast.Assert):
                    condition = constant(statement.test)
                    if condition is not unknown and not condition:
                        return True
                if isinstance(statement, ast.Raise):
                    raise ProcessExit
                if isinstance(statement, (ast.Return, ast.Break, ast.Continue)):
                    return True
        return False

    try:
        yield from statements(module.body, module_scope)
    except ProcessExit:
        return


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
    # Token trees are not parsed items until a macro expands them. Keep their
    # delimiters/prefixes as separators so a preceding attribute cannot attach
    # to an unrelated function after blanking a body.
    pairs, stack = {}, []
    for position, character in enumerate(masked):
        if character in "([{":
            stack.append((character, position))
        elif character in ")]}" and stack:
            opening, start = stack.pop()
            if opening != {")": "(", "]": "[", "}": "{"}[character]:
                raise ValueError("unbalanced Rust token tree")
            pairs[start] = position
    ranges = []
    identifier = r"(?:r#)?[A-Za-z_][A-Za-z_0-9]*"
    for pattern, declaration in (
        (rf"\bmacro_rules\s*!\s*{identifier}\s*([\[({{])", False),
        (rf"\bmacro\s+{identifier}\s*([\[({{])", True),
        (rf"\b{identifier}\s*!\s*([\[({{])", False),
    ):
        for match in re.finditer(pattern, masked):
            opening = match.end() - 1
            if opening not in pairs:
                raise ValueError("unclosed macro token tree")
            closing = pairs[opening]
            ranges.append((opening, closing))
            if declaration and masked[opening] == "(":
                body = closing + 1
                while body < len(masked) and masked[body].isspace():
                    body += 1
                if body < len(masked) and masked[body] == "{":
                    if body not in pairs:
                        raise ValueError("unclosed macro body")
                    ranges.append((body, pairs[body]))
    if ranges:
        code, literals = list(masked), list(preserved)
        last = -1
        for opening, closing in sorted(ranges):
            if opening < last:
                continue
            last = closing
            for position in range(opening + 1, closing):
                if code[position] not in "\r\n":
                    code[position] = literals[position] = " "
        masked, preserved = "".join(code), "".join(literals)
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
    module_openings = {module[0] for module in modules}
    for match in re.finditer(pattern, masked):
        if any(opening < match.start() < closing and opening not in module_openings
               for opening, closing in closes.items()):
            continue  # inner items/methods are not registered free test cases
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
        targets.update((path.stem, {"path": path, "required-features": [], "harness": True}) for path in directory.glob("tests/*.rs"))
        targets.update((path.parent.name, {"path": path, "required-features": [], "harness": True}) for path in directory.glob("tests/*/main.rs"))
    for declaration in manifest.get("test", []):
        name = declaration["name"]
        path = directory / declaration.get("path", f"tests/{name}.rs")
        targets[name] = {"path": path, "required-features": declaration.get("required-features", []),
                         "harness": declaration.get("harness", True)}
    return targets


def target_covered(package: str, name: str, target: dict, manifest: dict, commands: list[dict]) -> bool:
    if not target.get("harness", True):
        return False  # custom mains do not prove execution of source #[test]s
    source = target["path"].read_text(encoding="utf-8")
    predicates = list(crate_cfgs(source))
    cases = list(registered_cases(source))
    for command in commands:
        if package not in command["packages"] or any(
            fnmatch.fnmatchcase(package, pattern) for pattern in command.get("excludes", [])
        ):
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


def literal_action_input(value) -> str | None:
    if not isinstance(value, str) or any(marker in value for marker in ("$", "\x60", "\n", "\r")):
        return None
    return value


def local_action_invocations(source: str):
    for step in admitted_steps(source):
        relative = step.get("uses")
        provided = step.get("with", {})
        if not isinstance(relative, str) or not relative.startswith("./") or not isinstance(provided, dict):
            continue
        if literal_action_input(relative) is None:
            continue
        yield relative, {name: literal_action_input(value) for name, value in provided.items()}


def composite_action_source(source: str, provided: dict[str, str | None]) -> str | None:
    """Expand decoded literal inputs, preserving unknown gates as disabled."""
    document = workflow_document(source)
    definitions = document.get("inputs", {})
    runs = document.get("runs", {})
    if not isinstance(definitions, dict) or not isinstance(runs, dict) or runs.get("using") != "composite":
        return None
    if any(not isinstance(definition, dict) for definition in definitions.values()):
        return None
    defaults = {name: literal_action_input(definition.get("default"))
                for name, definition in definitions.items()}
    required = {name for name, definition in definitions.items()
                if str(definition.get("required", "false")).lower() == "true"}
    values = {**defaults, **provided}
    if any(not values.get(name) for name in required):
        return None

    def expand(value):
        if isinstance(value, str):
            return re.sub(r"\$\{\{\s*inputs\.([A-Za-z_][A-Za-z_0-9-]*)\s*\}\}",
                          lambda match: values.get(match[1]) if values.get(match[1]) is not None else match[0],
                          value)
        if isinstance(value, dict):
            return {key: expand(item) for key, item in value.items()}
        if isinstance(value, list):
            return [expand(item) for item in value]
        return value

    steps = runs.get("steps", [])
    if not isinstance(steps, list):
        return None
    if any(isinstance(step, dict) and "run" in step
           and (not isinstance(step.get("shell"), str)
                or step["shell"] not in {"bash", "sh", "pwsh", "powershell"})
           for step in steps):
        return None  # GitHub rejects the entire invalid composite action
    expanded = []
    for step in steps:
        if not isinstance(step, dict):
            continue
        selected = expand(step)
        if "if" in step:
            if not isinstance(step["if"], str):
                selected["if"] = "false"
            else:
                expression = workflow_expression(step["if"])
                references = re.findall(r"\binputs\.([A-Za-z_][A-Za-z_0-9-]*)", expression)
                if any(values.get(name) is None for name in references):
                    expression = "false"
                else:
                    expression = re.sub(r"\binputs\.([A-Za-z_][A-Za-z_0-9-]*)",
                                        lambda match: "'" + values[match[1]].replace("'", "''") + "'",
                                        expression)
                if re.search(r"\binputs(?:\.|\[)", expression) or workflow_truth("${{ " + expression + " }}") is None:
                    expression = "false"
                selected["if"] = "${{ " + expression + " }}"
        expanded.append(selected)
    return json.dumps({**document, "runs": {**runs, "steps": expanded}})


def workflow_sources(root: Path) -> list[str]:
    """Include composite steps only through admitted local action invocations.

    Literal `with` values override defaults. Unknown input-dependent gates and
    commands cannot own targets. Unsupported YAML forms supply no action credit;
    nested local actions are followed with path confinement and cycle detection.
    """
    sources, visited = [], set()

    def visit(source, ancestors):
        sources.append(source)
        for relative, provided in local_action_invocations(source):
            directory = (root / relative).resolve()
            if not directory.is_relative_to(root.resolve()) or directory in ancestors:
                continue
            path = next((directory / name for name in ("action.yml", "action.yaml")
                         if (directory / name).is_file()), None)
            identity = directory, tuple(sorted(provided.items()))
            if path is None or not path.resolve().is_relative_to(root.resolve()) or identity in visited:
                continue
            visited.add(identity)
            action = composite_action_source(path.read_text(encoding="utf-8"), provided)
            if action is not None:
                visit(action, (*ancestors, directory))

    paths = [*(root / ".github/workflows").glob("*.yml"),
             *(root / ".github/workflows").glob("*.yaml")]
    for path in sorted(paths):
        visit(path.read_text(encoding="utf-8"), ())
    return sources


def inventory_gaps(root: Path = ROOT) -> list[str]:
    commands = []
    sources = workflow_sources(root)
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
