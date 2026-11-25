"""Check the tracked Mech tree for alternate native codegen tooling."""

from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
# Character classes keep the search patterns from matching their own source.
BACKEND = r"[l]lvm|[i]nkwell|[m]elior|[m]lir[-_]sys|[c]lang[-_]sys"
FILE_PATTERN = re.compile(BACKEND + r"|(?:^|/)l[l]c(?:$|\.)|\.(?:ll|bc)$", re.IGNORECASE)
SOURCE_PATTERN = BACKEND + r"|(^|[^[:alnum:]_])l[l]c([^[:alnum:]_]|$)"
WINDOWS_TARGET = re.compile(
    r'(?:name = )?"windows_(?:aarch64|i686|x86_64)_gnu[l]lvm(?: [0-9.]+)?",?'
)
UPSTREAM_CRATES = {"cranelift-codegen", "cranelift-codegen-meta", "cranelift-frontend", "regalloc2"}


def upstream_reference(path, line):
    """Allow upstream notices and descriptions of matching a platform ABI."""
    parts = path.split("/", 2)
    if len(parts) != 3 or parts[0] != "vendor" or parts[1] not in UPSTREAM_CRATES:
        return False
    filename = parts[2]
    return (
        filename == "LICENSE"
        or line.startswith(("# ", "//", "* ", ";;"))
        or re.fullmatch(r'(?:license = )?"Apache-2\.0 WITH [L]LVM-exception",?', line)
        or (filename == "README.md" and re.search(r"Apache 2\.0 License with [L]LVM", line))
        or re.fullmatch(r'"cfg\(pulley_assume_[l]lvm_makes_tail_calls\)",', line)
        or re.search(r"enable_[l]lvm_abi_extensions|[l]lvm(?:'s)? (?:ABI|Exception)", line, re.I)
        or (
            filename == "src/shared/settings.rs"
            and re.search(r"(?:follow [l]lvm's$|[l]lvm's (?:behavior|implementation))", line, re.I)
        )
    )


def main():
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
    names = paths.decode(errors="replace").split("\0")
    violations = [
        name for name in names
        if Path(name).suffix.lower() != ".md" and FILE_PATTERN.search(name)
    ]
    matches = subprocess.run(
        ["git", "grep", "--cached", "-IinE", SOURCE_PATTERN, "--"],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    if matches.returncode not in (0, 1):
        sys.exit(matches.stderr.strip())
    for match in matches.stdout.splitlines():
        path, _, location = match.partition(":")
        if Path(path).suffix.lower() == ".md":
            continue
        _, _, line = location.partition(":")
        line = line.strip()
        if path in {"Cargo.lock", "src/build/native-resolution-seed.lock"} and WINDOWS_TARGET.fullmatch(line):
            continue
        # The s390x mnemonic and corporate suffix share a tool name.
        if not re.search(BACKEND, line, re.I) and (
            path.startswith("vendor/cranelift-codegen/src/isa/s390x/inst/")
            or line.startswith("Copyright ")
        ):
            continue
        if upstream_reference(path, line):
            continue
        violations.append(match)
    if violations:
        print("Cranelift is the only Mech native code generator. Remove these entries:", file=sys.stderr)
        print(*violations, sep="\n", file=sys.stderr)
        return 1
    print("Cranelift-only native codegen contract passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
