#!/usr/bin/env python3
"""Enforce the closed standard and full native-host catalogs."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
STANDARD = ROOT / "src/build/src/host/standard.rs"


def capture(*arguments: str) -> str:
    process = subprocess.run(
        arguments,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if process.returncode:
        raise RuntimeError(
            f"{' '.join(arguments)} failed:\n{process.stdout}{process.stderr}"
        )
    return process.stdout


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def main() -> int:
    try:
        source = STANDARD.read_text(encoding="utf-8")
        expected_providers = {
            "cli": ("mech-terminal", "mech_terminal", "provider", "mech_terminal::CliHostFactory::new"),
            "console": ("mech-console", "mech_console", "native", "mech_console::NativeConsoleHostFactory::new"),
            "robot-arm": ("mech-robot-arm", "mech_robot_arm", "provider", "mech_robot_arm::RobotArmHostFactory::new"),
            "scene": ("mech-scene", "mech_scene", "native", "mech_scene::NativeSceneHostFactory::new"),
            "time": ("mech-time", "mech_time", "native", "mech_time::NativeTimeHostFactory::new"),
            "timer": ("mech-timer", "mech_timer", "native", "mech_timer::NativeTimerHostFactory::new"),
        }
        standard_block = re.search(
            r"fn standard_native_host_registrations\(\).*?\{(.*?)\n\}\n\n"
            r"#\[cfg\(feature = \"full-hosts\"\)\]",
            source,
            re.DOTALL,
        )
        require(standard_block is not None, "standard provider registration block is missing")
        standard_providers = set(
            re.findall(r'provider:\s*"([a-z-]+)"', standard_block.group(1))
        )
        require(
            standard_providers == {"cli", "console", "scene", "time", "timer"},
            f"standard provider set drifted: {sorted(standard_providers)}",
        )
        all_providers = set(re.findall(r'provider:\s*"([a-z-]+)"', source))
        require(
            all_providers == set(expected_providers),
            f"full provider set drifted: {sorted(all_providers)}",
        )
        for provider, (package, crate_name, feature, factory) in expected_providers.items():
            require(f'provider: "{provider}"' in source, f"{provider} provider is missing")
            require(f'package: "{package}"' in source, f"{provider} package is wrong")
            require(f'crate_name: "{crate_name}"' in source, f"{provider} crate name is wrong")
            require(f'cargo_features: &["{feature}"]' in source, f"{provider} feature list is not exact")
            require(f'factory_path: "{factory}"' in source, f"{provider} factory path is not exact")

        actor_functions = set(re.findall(r'name:\s*"(actor/[a-z/-]+)"', source))
        require(not actor_functions, f"actor migration functions returned: {sorted(actor_functions)}")
        require("ACTOR_FEATURES" not in source, "actor migration feature closure returned")
        require(
            "actor_host_function_linkages" not in source,
            "actor migration linkage catalog returned",
        )
        require(
            "insert_experimental_actor_functions" not in source,
            "actor migration catalog insertion returned",
        )
        require(
            "assert_eq!(catalog.function_count(), 0);" in source,
            "catalog tests no longer prove that migration functions are absent",
        )
        require(not re.search(r'provider:\s*"browser"', source), "browser provider entered native catalog")

        build_manifest = (ROOT / "src/build/Cargo.toml").read_text(encoding="utf-8")
        standard_hosts = re.search(
            r"standard-hosts\s*=\s*\[(.*?)\]",
            build_manifest,
            re.DOTALL,
        )
        require(standard_hosts is not None, "standard native host feature is missing")
        require(
            re.findall(r'"([^"]+)"', standard_hosts.group(1))
            == [
                "dep:mech-terminal",
                "dep:mech-console",
                "dep:mech-scene",
                "dep:mech-time",
                "dep:mech-timer",
            ],
            "standard native host feature is not exact",
        )
        require(
            'full-hosts = ["standard-hosts", "dep:mech-robot-arm"]' in build_manifest,
            "full native host feature is not standard plus robot-arm",
        )
        require(
            "experimental-actors" not in build_manifest,
            "experimental actor build feature returned",
        )

        expected_runtime_features = {
            "mech-terminal": ["runtime", "string"],
            "mech-console": ["runtime", "string"],
            "mech-time": ["f64", "runtime"],
            "mech-timer": ["f64", "runtime"],
            "mech-scene": ["f64", "matrixd", "record", "runtime", "string", "table"],
            "mech-robot-arm": ["bool", "runtime", "string"],
        }
        metadata = json.loads(
            capture("cargo", "metadata", "--format-version", "1", "--locked", "--offline")
        )
        packages = {package["name"]: package for package in metadata["packages"]}
        forbidden_dependencies = {"mech-bytecode", "mech-syntax"}
        forbidden_features = {"compiler", "source"}
        for package_name, runtime_features in expected_runtime_features.items():
            package = packages[package_name]
            normal_dependencies = [
                dependency
                for dependency in package["dependencies"]
                if dependency["kind"] in (None, "normal")
            ]
            dependency_names = {dependency["name"] for dependency in normal_dependencies}
            require(
                dependency_names.isdisjoint(forbidden_dependencies),
                f"{package_name} has a forbidden production dependency",
            )
            require(
                all(
                    forbidden_features.isdisjoint(dependency["features"])
                    for dependency in normal_dependencies
                ),
                f"{package_name} has a forbidden production feature",
            )
            runtime_dependency = next(
                dependency
                for dependency in normal_dependencies
                if dependency["name"] == "mech-runtime"
            )
            require(
                sorted(runtime_dependency["features"]) == runtime_features,
                f"{package_name} runtime feature closure drifted: "
                f"{sorted(runtime_dependency['features'])}",
            )

    except (OSError, RuntimeError) as error:
        print(f"native host catalog contract failed: {error}", file=sys.stderr)
        return 1
    print(
        "native host catalog contract passed "
        "(five standard providers, six full providers, no actor migration linkages)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
