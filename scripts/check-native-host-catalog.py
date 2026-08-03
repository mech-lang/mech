#!/usr/bin/env python3
"""Enforce the closed Phase 1 native-host catalog contract."""

from __future__ import annotations

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


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def main() -> int:
    try:
        source = STANDARD.read_text(encoding="utf-8")
        require(source.count("insert_provider(NativeHostLinkage") == 1, "catalog is not closed to one provider")
        require('provider: "cli"' in source, "CLI provider is missing")
        require('package: "mech-host-cli"' in source, "CLI package is wrong")
        require('crate_name: "mech_host_cli"' in source, "CLI crate name is wrong")
        require('cargo_features: &["provider"]' in source, "CLI feature list is not exact")
        require(
            'factory_path: "mech_host_cli::CliHostFactory::new"' in source,
            "CLI factory path is not exact",
        )
        require("mech_host_cli::cli_host_manifest" in source, "CLI manifest ownership is not delegated to the host")
        require(not re.search(r'provider:\s*"browser"', source), "browser provider entered Phase 1")

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
    print("native host catalog contract passed (CLI only; unknown/browser rejected)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
