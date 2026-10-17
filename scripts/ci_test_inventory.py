"""Inventory canonical integration targets against executable CI selections.

This checks target ownership and crate-level feature admission, not executed
test counts, dependency unit tests, platform availability or semantic coverage.
Explicit ignored-case/nonempty-list contracts remain in test_ci_full_contract.
"""

from __future__ import annotations

import fnmatch
import ast
import json
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


def disabled_run(lines: list[str], index: int, indent: int) -> bool:
    def disabled(metadata):
        for line in metadata:
            match = re.fullmatch(r"\s*(if|continue-on-error):\s*([^#]+?)(?:\s*#.*)?", line)
            if match:
                value = re.sub(r"\s+", "", match[2]).strip("\"'")
                if match[1] == "if" and value in {"false", "${{false}}"}:
                    return True
                if match[1] == "continue-on-error" and value in {"true", "${{true}}"}:
                    return True
        return False

    jobs = [offset for offset, line in enumerate(lines[:index]) if re.fullmatch(r"  [a-zA-Z_][a-zA-Z_0-9-]*:\s*", line)]
    if jobs:
        header = []
        for line in lines[jobs[-1] + 1:index]:
            if re.fullmatch(r"    steps:\s*", line):
                break
            header.append(line)
        if disabled(header):
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
        if len(line) - len(line.lstrip()) == step_indent + 2:
            metadata.append(line)
    return disabled(metadata)


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
        elif not token.startswith("-"):
            filters.append(token)
        index += 1
    return {
        "packages": packages, "targets": targets, "all_targets": all_targets,
        "features": set(filter(None, re.split(r"[\s,]+", " ".join(features)))),
        "default": "--no-default-features" not in cargo,
        "all_features": "--all-features" in cargo,
        "filters": filters, "skips": skips, "exact": "--exact" in harness,
    }


def workflow_commands(source: str):
    recorded = {}
    for block in run_blocks(source):
        block = re.sub(r"\\\n\s*", " ", block)
        for line in block.splitlines():
            if not re.match(r"\s*(?:cargo|python3)\s", line):
                continue
            try:
                arguments = shlex.split(line, comments=True)
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
    for relative in ("scripts/check-generated-project-determinism.py", "scripts/check-native-application-graphs.py"):
        invocation = re.compile(r"(?m)^\s*python3\s+" + re.escape(relative) + r"(?:\s|$)")
        if not any(invocation.search(block) for source in sources for block in run_blocks(source)):
            continue
        module = ast.parse((root / relative).read_text(encoding="utf-8"))
        for function in (node for node in ast.walk(module) if isinstance(node, ast.FunctionDef)):
            assignments = {}
            for statement in function.body:
                if isinstance(statement, ast.Assign) and len(statement.targets) == 1 and isinstance(statement.targets[0], ast.Name):
                    assignments[statement.targets[0].id] = statement.value
            for call in (node for node in ast.walk(function) if isinstance(node, ast.Call)):
                if not isinstance(call.func, ast.Name) or call.func.id not in {"run_streamed", "execute_streamed"} or not call.args:
                    continue
                argument = call.args[0]
                if isinstance(argument, ast.Name):
                    argument = assignments.get(argument.id)
                try:
                    arguments = ast.literal_eval(argument)
                except (ValueError, TypeError, SyntaxError):
                    continue
                if isinstance(arguments, (list, tuple)) and all(isinstance(value, str) for value in arguments):
                    command = cargo_test_command(list(arguments))
                    if command is not None:
                        yield command


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
    tokens = re.findall(r'"(?:\\.|[^"\\])*"|[A-Za-z_][A-Za-z_0-9]*|[=(),]', expression)
    position = 0

    def parse():
        nonlocal position
        if position >= len(tokens):
            raise ValueError("incomplete cfg predicate")
        name = tokens[position]
        position += 1
        if position < len(tokens) and tokens[position] == "=":
            position += 1
            if position == len(tokens):
                raise ValueError("missing cfg value")
            value = json.loads(tokens[position])
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


def crate_cfgs(source: str):
    preserved = rust_code(source, preserve_literals=True)
    source = rust_code(source)
    for match in re.finditer(r"#!\s*\[\s*cfg\s*\(", source):
        position, start, depth = match.end(), match.end(), 1
        while position < len(source) and depth:
            if source[position] == "(":
                depth += 1
            elif source[position] == ")":
                depth -= 1
            position += 1
        if depth:
            raise ValueError("unclosed crate cfg")
        yield preserved[start:position - 1]


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
    code = rust_code(source)
    cases = re.findall(r"#\s*\[\s*test\s*\]\s*(?:#\s*\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)\s*\(", code)
    for command in commands:
        if package not in command["packages"]:
            continue
        if not command["all_targets"] and not any(fnmatch.fnmatchcase(name, pattern) for pattern in command["targets"]):
            continue
        selected = [case for case in cases if not any(skip in case for skip in command["skips"])]
        if cases and not selected:
            continue
        if command["filters"] and not any(
            any(case == pattern if command["exact"] else pattern in case for pattern in command["filters"])
            for case in selected
        ):
            continue
        enabled = feature_closure(manifest, command)
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
