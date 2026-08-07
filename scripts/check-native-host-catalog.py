#!/usr/bin/env python3
"""Enforce the closed standard native-host and actor-function catalog."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
STANDARD = ROOT / "src/build/src/host/standard.rs"


def run(*arguments: str) -> None:
    process = subprocess.run(
        arguments,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if process.returncode:
        raise RuntimeError(f"{' '.join(arguments)} failed:\n{process.stdout}")


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
            "cli": ("mech-host-cli", "mech_host_cli", "provider", "mech_host_cli::CliHostFactory::new"),
            "console": ("mech-host-console", "mech_host_console", "native", "mech_host_console::NativeConsoleHostFactory::new"),
            "robot-arm": ("mech-host-robot-arm", "mech_host_robot_arm", "provider", "mech_host_robot_arm::RobotArmHostFactory::new"),
            "scene": ("mech-host-scene", "mech_host_scene", "native", "mech_host_scene::NativeSceneHostFactory::new"),
            "time": ("mech-host-time", "mech_host_time", "native", "mech_host_time::NativeTimeHostFactory::new"),
            "timer": ("mech-host-timer", "mech_host_timer", "native", "mech_host_timer::NativeTimerHostFactory::new"),
        }
        providers = set(re.findall(r'provider:\s*"([a-z-]+)"', source))
        require(providers == set(expected_providers), f"provider set drifted: {sorted(providers)}")
        for provider, (package, crate_name, feature, factory) in expected_providers.items():
            require(f'provider: "{provider}"' in source, f"{provider} provider is missing")
            require(f'package: "{package}"' in source, f"{provider} package is wrong")
            require(f'crate_name: "{crate_name}"' in source, f"{provider} crate name is wrong")
            require(f'cargo_features: &["{feature}"]' in source, f"{provider} feature list is not exact")
            require(f'factory_path: "{factory}"' in source, f"{provider} factory path is not exact")

        expected_actor_functions = {
            "actor/message/kind",
            "actor/message/payload",
            "actor/state/get",
            "actor/state/id",
            "actor/state/put",
        }
        actor_functions = set(re.findall(r'name:\s*"(actor/[a-z/-]+)"', source))
        require(actor_functions == expected_actor_functions, f"actor function set drifted: {sorted(actor_functions)}")
        require('cargo_features: ACTOR_FEATURES' in source, "actor feature closure is not shared")
        actor_features = re.search(
            r'const\s+ACTOR_FEATURES:\s*&\[&str\]\s*=\s*&\[(.*?)\];',
            source,
            re.DOTALL,
        )
        require(actor_features is not None, "actor feature closure is missing")
        features = re.findall(r'"([a-z-]+)"', actor_features.group(1))
        require(features == ["native-link", "runtime", "string"], "actor feature closure is not exact")
        require(not re.search(r'provider:\s*"browser"', source), "browser provider entered native catalog")

        expected_runtime_features = {
            "mech-host-cli": ["runtime", "string"],
            "mech-host-console": ["runtime", "string"],
            "mech-host-time": ["f64", "runtime"],
            "mech-host-timer": ["f64", "runtime"],
            "mech-host-scene": ["f64", "runtime", "string"],
            "mech-host-robot-arm": ["bool", "runtime", "string"],
        }
        metadata = json.loads(capture("cargo", "metadata", "--format-version", "1"))
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

        run(
            "cargo",
            "+nightly-2026-03-03",
            "test",
            "-p",
            "mech-build",
            "--features",
            "standard-hosts",
            "--lib",
            "host::",
            "--quiet",
        )
        run(
            "cargo",
            "+nightly-2026-03-03",
            "test",
            "-p",
            "mech-build",
            "--features",
            "standard-hosts",
            "--test",
            "planning",
            "unknown_and_browser_providers_fail_before_generation",
            "--quiet",
        )
    except (OSError, RuntimeError) as error:
        print(f"native host catalog contract failed: {error}", file=sys.stderr)
        return 1
    print("native host catalog contract passed (six providers, five actor functions)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
