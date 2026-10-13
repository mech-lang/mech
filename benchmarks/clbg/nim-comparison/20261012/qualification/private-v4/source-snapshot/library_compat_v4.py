"""Two syntax-only public GMP declaration repairs in a fresh private copy."""
import hashlib
from pathlib import Path

UNIONS = {
    'src/gmp/header.nim': (
        '  INNER_C_UNION_5532179898798000430* {.importc: "no_name", header: "<gmp.h>".} = object  {.\n      union.}',
        '  INNER_C_UNION_5532179898798000430* {.importc: "no_name", header: "<gmp.h>", union.} = object'),
    'src/gmp/pure.nim': (
        '  mm_gmp_randstate_algdata* {.pure.} = object  {.union.}',
        '  mm_gmp_randstate_algdata* {.pure, union.} = object'),
}


def normalize(source, filename):
    before, after = UNIONS[filename]
    assert source.count(before) == 1 and after not in source, 'Exact upstream declaration required; no implicit re-patch'
    return source.replace(before, after, 1)


def patch(preparation, wrapper, private_build, output):
    """Caller owns common mutex; retained archives and old revisions are read-only."""
    wrapper = Path(wrapper).resolve()
    private_build = Path(private_build).resolve()
    assert private_build.name == 'private-v4' and wrapper.is_relative_to(private_build / 'nim-gmp')
    assert not output.exists(), 'Never overwrite a compatibility journal'
    records = []
    changes = []
    for filename in UNIONS:
        target = wrapper / filename
        source = target.read_text()
        transformed = normalize(source, filename)
        original_sha256 = preparation.sha(target)
        # Journal original bytes before the first private-file mutation.
        retained = output.parent / ('original-' + target.name)
        assert not retained.exists()
        retained.parent.mkdir(parents=True, exist_ok=True)
        retained.write_bytes(target.read_bytes())
        records.append(dict(path=str(target), original_copy=str(retained),
            original_sha256=original_sha256,
            changed_sha256=hashlib.sha256(transformed.encode()).hexdigest(),
            old_declaration=UNIONS[filename][0], new_declaration=UNIONS[filename][1],
            arithmetic_changed=False, ffi_symbols_types_argument_order_changed=False))
        changes.append((target, transformed))
    # Validate both exact anchors and retain both originals before mutating
    # either copy. A signal leaves this immutable intended-change ledger.
    preparation.new_json(output.with_name('compatibility-plan.json'), dict(
        status='planned-two-union-pragma-moves', files=records, never_chart_eligible=True))
    for (target, transformed), record in zip(changes, records):
        assert preparation.sha(target) == record['original_sha256']
        target.write_text(transformed)
        assert preparation.sha(target) == record['changed_sha256']
    preparation.new_json(output, dict(status='exact-two-union-pragma-moves', files=records,
        public_package_commit='4e9a6a05bfc274abd02a241f2d7c9162b2300191',
        production_source_changed=False, general_library_compatibility_only=True,
        full_performance_runs=0, never_chart_eligible=True))
    return {str(path): preparation.sha(path) for path in [output, output.with_name('compatibility-plan.json')] +
            [Path(row['original_copy']) for row in records]}
