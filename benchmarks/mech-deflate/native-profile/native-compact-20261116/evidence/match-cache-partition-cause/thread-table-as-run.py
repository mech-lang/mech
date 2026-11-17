"""Prepare the existing table-slot variant with the table in the Tail carriers."""

from bisect import bisect_right
from difflib import unified_diff
from hashlib import sha256
from pathlib import Path
import re

from control_graph import STUDY


FILES = Path(__file__).resolve().parent
SOURCE = STUDY / "stored-table-slot/source.mec"
PARAMETER = "match-offset-slots"
TABLE = "deflate-table-offset-slots"
CALL_COUNTS = {
    "bt-put-match": 3,
    "bt-search-step": 2,
    "bt-search-loop": 4,
    "bt-short-record": 3,
    "bt-get-matches": 2,
    "bt-advance": 2,
    "optimal-advance-position": 2,
    "optimal-block-loop": 3,
}


def argument_end(source, start):
    nesting = 1
    for at in range(start, len(source)):
        if source[at] == "(":
            nesting += 1
        elif source[at] == ")":
            nesting -= 1
        if nesting == 0:
            return at
    raise ValueError("no closing parenthesis")


def prepare(source):
    expected_sha = "7625c4613caea164d20cb56a507b156d4d9e95c8e045ade490ea459f6bdebb0f"
    assert sha256(source.encode()).hexdigest() == expected_sha
    assert PARAMETER not in source
    definitions = list(re.finditer(r"^([a-z][a-z0-9/-]*)\(", source, flags=re.M))
    starts = [match.start() for match in definitions]
    insertions = []
    seed_callers = []
    for name, expected_count in CALL_COUNTS.items():
        calls = list(re.finditer(rf"(?<![\w/-]){name}\(", source))
        assert len(calls) == expected_count, name
        for call in calls:
            caller = definitions[bisect_right(starts, call.start()) - 1]
            if call.start() == caller.start():
                argument = f"{PARAMETER}<[u8]>"
            elif caller[1] in CALL_COUNTS:
                argument = PARAMETER
            else:
                seed_callers.append((caller[1], name))
                argument = TABLE
            insertions.append((argument_end(source, call.end()), ", " + argument))
    assert sorted(seed_callers) == [
        ("optimal-collect-block", "optimal-block-loop"), ("optimal-collect-position", "bt-get-matches")
    ]
    patched = source
    for at, content in sorted(insertions, reverse=True):
        patched = patched[:at] + content + patched[at:]
    original_read = f"slot := optimal-offset-slot(cached-offset, {TABLE})"
    assert patched.count(original_read) == 1
    return patched.replace(original_read, f"slot := optimal-offset-slot(cached-offset, {PARAMETER})")


if __name__ == "__main__":
    before = SOURCE.read_text()
    after = prepare(before)
    (FILES / "stored-table-in-carrier.mec").write_text(after)
    patch = unified_diff(before.splitlines(keepends=True), after.splitlines(keepends=True),
                         fromfile="stored-table-slot/source.mec", tofile="stored-table-in-carrier.mec", n=1)
    (FILES / "thread-table.patch").write_text("".join(patch))
    print(sha256(after.encode()).hexdigest())
