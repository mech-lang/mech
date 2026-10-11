"""Pin the reviewed explorer sources and prepare the one work-corrected endpoint.

This script does not measure, build, or update the explorer. Evidence refers to
the immutable source bytes embedded in the saved explorer snapshot.
"""
import datetime
import hashlib
import json
import re
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
WEBSITE = CLBG / 'overview/explorer/dist/data.json'
RUNTIME = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


# Each identifier below was reviewed together with its actual arithmetic helper.
# Other execution modes share the same application source and review.
GUARDED = {
    'pidigits-gcc-1', 'pidigits-chapel-2', 'pidigits-gpp-4', 'pidigits-gpp-5',
    'pidigits-go-4', 'pidigits-go-3', 'pidigits-julia-4', 'pidigits-julia-3',
    'mech-pi-final-advance-signed', 'pidigits-rust-4', 'pidigits-rust-3',
    'pidigits-python3-3', 'pidigits-python3-4', 'pidigits-lua-1-luajit',
    'array-bqn-faithful', 'array-k-faithful', 'mojo-pidigits-compact',
    'mojo-pidigits-optimized', 'futhark-pidigits-optimized', 'pidigits-swift-2',
    'pidigits-ghc-6', 'pidigits-ocaml-7', 'pidigits-csharpcore-6-arm64-faithful-jit',
    'pidigits-fsharpcore-6-arm64-faithful-jit', 'pidigits-gnat-2',
    'pidigits-fpascal-3', 'pidigits-fortran-4', 'pidigits-dart-2',
    'pidigits-smalltalk-published-1', 'pidigits-smalltalk-compact', 'd-compact',
    'd-fast',
}
UNCONDITIONAL = {
    'pidigits-graalvmaot-1', 'mech-pi-final-compact-signed', 'pidigits-ruby-1',
    'pidigits-ghc-3', 'pidigits-ocaml-5', 'matlab-compact', 'matlab-optimized',
    'pidigits-erlang-2', 'pidigits-erlang-3', 'pidigits-racket-1',
    'pidigits-sbcl-3-faithful', 'pidigits-racket-2-faithful',
    'pidigits-roc-idiomatic', 'pidigits-odin-gmp', 'pidigits-zig-gmp',
    'pidigits-elm-optimized',
}
NUMERATOR_BASIS = {'pidigits-php-5', 'pidigits-csharpcore-7-jit'}
ALIASES = {
    'mech-pi-final-advance-signed:jit': 'mech-pi-final-advance-signed',
    'mech-pi-final-advance-signed:resident': 'mech-pi-final-advance-signed',
    'mech-pi-final-compact-signed:jit': 'mech-pi-final-compact-signed',
    'mech-pi-final-compact-signed:resident': 'mech-pi-final-compact-signed',
    'pidigits-csharpcore-6-arm64-faithful-aot': 'pidigits-csharpcore-6-arm64-faithful-jit',
    'pidigits-csharpcore-7-aot': 'pidigits-csharpcore-7-jit',
    'pidigits-fsharpcore-6-arm64-faithful-aot': 'pidigits-fsharpcore-6-arm64-faithful-jit',
    'pidigits-dart-jit-2': 'pidigits-dart-2',
}
NOTES = {
    'pidigits-perl-4': 'The q>t guard rejects intervals of width q/t>1, whose endpoint floors cannot agree. Otherwise (3q+r)/t and (4q+r)/t are both actual integer divisions.',
    'pidigits-node-2': 'Computes t=(3q+r)/tden once; m%tden+q<tden substitutes a remainder predicate for the required second quotient. The predicate is numerically equivalent but omits an actual arbitrary-precision division.',
    'pidigits-php-5': 'Two actual arbitrary-precision quotients use endpoint numerators n1=4q+r and n2=3q+r. This is a change of basis for the same sequential LFT spigot, not a different pi algorithm.',
    'pidigits-csharpcore-7-jit': 'Two actual arbitrary-precision quotients use endpoint numerators n1=4q+r and n2=3q+r. This is a change of basis for the same sequential LFT spigot, not a different pi algorithm.',
    'pidigits-dart-2': 'The current counted source is already corrected: t=m~/d; u=(m+n)~/d; equality compares the two actual quotients. The published one-extract form is not the current endpoint.',
    'pidigits-erlang-3': 'The shared 3q+r numerator is reused, but both (3q+r)/t and ((3q+r)+q)/t remain actual arbitrary-precision divisions.',
    'pidigits-ghc-6': 'Lazy evaluation skips both quotients on the permitted n>a guard; when a candidate is tested, both 3$t and 4$t use the actual div helper.',
}


def classify(identifier):
    canonical = ALIASES.get(identifier, identifier)
    if canonical in GUARDED:
        category = 'permitted_guard_two_exact_extracts'
        note = 'A preliminary impossible-candidate guard may skip a test. Every candidate actually tested computes both endpoint extracts using exact arbitrary-precision division.'
    elif canonical in UNCONDITIONAL:
        category = 'two_exact_extracts_for_each_tested_candidate'
        note = 'The canonical sequential LFT stream compares actual exact arbitrary-precision endpoint quotients for 3 and 4; no one-quotient substitute is used.'
    elif canonical in NUMERATOR_BASIS:
        category = 'equivalent_endpoint_numerator_state_two_exact_divisions'
        note = ''
    elif canonical == 'pidigits-perl-4':
        category = 'permitted_interval_width_guard_two_exact_divisions'
        note = ''
    elif canonical == 'pidigits-node-2':
        category = 'nonconformant_one_quotient_remainder_test'
        note = ''
    else:
        raise ValueError(f'Unreviewed endpoint: {identifier}')
    return canonical, category, NOTES.get(canonical, note)


def source_evidence(code):
    pattern = re.compile(r'extract|quotient|divide|DivRem|fdiv|tdiv|\bdiv\b|/|\bflr\b|floor|safe|\b[tu]\s*=|\$z0', re.I)
    return [dict(line=i, code=line) for i, line in enumerate(code.splitlines(), 1)
            if pattern.search(line) and not line.lstrip().startswith(('//', '#', '/*', '*', '--'))]


def main():
    data = json.loads(WEBSITE.read_text())
    programs = {k: r for k, r in data['programs'].items() if r['benchmark'] == 'pidigits'}
    hashes = {h for r in programs.values() for h in r['files']}
    files = {h: data['files'][h] for h in hashes}
    for h, f in files.items():
        assert hashlib.sha256(f['code'].encode()).hexdigest() == h == f['sha256']
    snapshot = dict(schema_version=1, website=str(WEBSITE), website_sha256=sha(WEBSITE),
                    captured_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                    programs=programs, files=files)
    snapshot_path = HERE / 'website-pidigits-snapshot.json'
    snapshot_path.write_text(json.dumps(snapshot, indent=2) + '\n')
    reviewed = []
    for key, r in programs.items():
        f = files[r['files'][0]]
        canonical, category, note = classify(r['id'])
        reviewed.append(dict(key=key, id=r['id'], language=r['language'],
            source=f['path'], source_sha256=f['sha256'],
            support_sources=[dict(path=files[h]['path'], sha256=h) for h in r['files'][1:]],
            classification=category, conforms=category != 'nonconformant_one_quotient_remainder_test',
            reviewed_as=canonical, review_note=note, extraction_evidence=source_evidence(f['code']),
            existing_loc=r['loc'], report=r.get('report'), source_report=r.get('source_report')))
    rejected = [r for r in reviewed if not r['conforms']]
    assert len(rejected) == 1 and rejected[0]['id'] == 'pidigits-node-2'
    old = files[programs[rejected[0]['key']]['files'][0]]
    upstream = HERE / 'upstream/pidigits-node-2.js'
    upstream.write_text(old['code'])
    assert sha(upstream) == old['sha256']
    corrected = old['code'].replace('var res = i.toString(), count;', 'var res = i.toString();\n  var count;')
    corrected = corrected.replace('let i = 0, ns = 0;', 'let i = 0;\n  let ns = 0;')
    corrected = corrected.replace('u = m % d + n;', 'u = (m + n) / d;').replace('if (d > u)', 'if (t === u)')
    assert corrected != old['code'] and 'u = (m + n) / d;' in corrected
    assert 'm % d' not in corrected
    source = HERE / 'sources/pidigits-node-2-two-extracts.js'
    source.write_text(corrected)
    # These sources have one leading block comment and no other comments.
    cleaned = re.sub(r'/\*.*?\*/', '', corrected, flags=re.S)
    loc = sum(bool(line.strip()) for line in cleaned.splitlines())
    assert loc == 49
    row = dict(key='pidigits:pidigits-node-2-two-extracts', id='pidigits-node-2-two-extracts',
        benchmark='pidigits', language='javascript', roles=['compact', 'optimized'],
        replaces_keys=[rejected[0]['key']], source=str(source), source_sha256=sha(source),
        source_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/pidigits-node-2.html',
        upstream_source=str(upstream), upstream_sha256=sha(upstream),
        loc=loc, normalized_loc=loc, physical_loc=loc,
        loc_metric='statement-normalized', loc_expansions=[],
        loc_policy='Nonblank/noncomment source lines. Both independent declarations formerly packed with commas are expanded physically; branch alternatives of a single ternary expression count once.',
        adaptations=['Replace the one-quotient remainder predicate with the actual second exact BigInt division: u=(m+n)/d, then t===u.',
                     'Split the independent var res/count and let i/ns declarations onto separate physical lines. Output and recurrence are otherwise unchanged.'],
        command=[str(RUNTIME), str(source), '{n}'], cwd=str(CLBG), runtime_environment={},
        runtime=dict(path=str(RUNTIME), sha256=sha(RUNTIME),
                     version=subprocess.check_output([str(RUNTIME), '--version'], text=True).strip()),
        build_command=None, full_n=10000,
        algorithm=dict(strict=True, native_benchmark_kernel=False,
            reason='Canonical sequential unbounded LFT spigot, permitted a>n impossible-candidate guard, both true endpoint divisions for every tested candidate, complete output and padding.'),
        validation_status='prepared_not_yet_run', timing_status='unmeasured', samples_seconds=[])
    audit = dict(schema_version=1, created_at=snapshot['captured_at'], benchmark='pidigits',
        status='correction_prepared_pending_validation_and_full_timing',
        timing_policy='No previous sample may be reused for the corrected source. Parent must perform three guarded full process runs; all other accepted endpoint measurements remain unchanged.',
        established_policy=dict(
            local_protocol=str(CLBG / 'work-audit/20261009/README.md'), lines=[34,35],
            local_audit=str(CLBG / 'work-audit/20261009/audit.json'),
            rule='Both actual exact extract(3) and extract(4) divisions for every tested candidate. A preliminary impossible-candidate guard is permitted, including the pinned published Rust #4 q>r guard.',
            clarification='This rule does not require both divisions for every recurrence iteration. Existing Mech advance and the original R early guard are permitted. Sources that perform both divisions on every iteration do additional work and are still valid.',
            official_description='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/pidigits.html',
            official_quote="Do both extract(3) and extract(4). Don't optimize away the work.",
            paper='https://www.cs.ox.ac.uk/jeremy.gibbons/publications/spigot.pdf',
            paper_note='The canonical unbounded stream uses LFT endpoint floors at 3 and 4 to decide safety. The paper also discusses later optimizations and other algorithms; the benchmark requires its specified single-thread stream and both extracts.'),
        snapshot=dict(path=str(snapshot_path), sha256=sha(snapshot_path), website_sha256=snapshot['website_sha256']),
        endpoint_count=len(reviewed), distinct_application_source_count=len({r['source_sha256'] for r in reviewed}),
        classifications=reviewed, rejected_endpoints=[dict(r, retired_samples_seconds=programs[r['key']]['samples'], retired_median_seconds=programs[r['key']]['seconds']) for r in rejected],
        numerator_basis_proof='n1=4q+r, n2=3q+r, d=t, hence q=n1-n2 and r=4n2-3n1. The consume updates n1=(2k-1)n1+2n2, n2=(k-1)n1+(k+2)n2, d=(2k+1)d produce the same q=kq and r=(2q+r)(2k+1). Digit production updates each numerator by 10n-10digit*d. Both endpoint quotients remain real divisions.',
        corrections=[row],
        separate_r_fasta_review=dict(source=str(CLBG / 'r-apl-uiua-comparison/20261011/sources/r/fasta/optimized.R'),
            sha256='1ed7d6a3768b596598d892fc265a6edd25174b65ef9510684e0bd3f03972b458', conforms=True,
            findings='Hillis-Steele modular affine scan composes every output seed from the live incoming seed. Integer products and sums are below 2^35, exactly representable in binary64. All draws use seed/139968 and the original unscaled binary64 CDF. The last seed carries across chunks and from IUB into Human. No RNG period or output cache is used.'))
    (HERE / 'audit.json').write_text(json.dumps(audit, indent=2) + '\n')
    print(json.dumps(dict(endpoints=len(reviewed), distinct_sources=audit['distinct_application_source_count'],
        rejected=[r['key'] for r in rejected], correction=row), indent=2))


if __name__ == '__main__':
    main()
