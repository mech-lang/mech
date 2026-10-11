"""Create additive v2 preparation metadata; do not mutate the frozen v1 manifest."""
import copy
import datetime
import hashlib
import json
from pathlib import Path

G = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def loc(source):
    return sum(bool(s.strip()) and not s.lstrip().startswith('⍝') and s.strip() != '∇'
               for s in source.splitlines())


def main():
    original = json.loads((G / 'apl-manifest.json').read_text())
    validation_path = G / 'results/apl/regex-v2/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['passed']
    build_path = G / 'results/apl/regex-v2-build.json'
    build = json.loads(build_path.read_text())
    assert build['returncode'] == 0
    source = G / 'support/apl/regex-native.cpp'
    binary = G / '.build/apl-support/libregex-apl.dylib'
    pcre = Path('/opt/homebrew/opt/pcre2/lib/libpcre2-32.0.dylib')
    assert sha(source) == build['source_sha256'] == validation['support_source_sha256']
    assert sha(binary) == build['binary_sha256'] == validation['support_binary_sha256']
    support = dict(name='General GNU APL/PCRE2 literal global substitution binding',
        source=str(source), source_sha256=sha(source), binary=str(binary), binary_sha256=sha(binary),
        build_command=build['command'], build_report=str(build_path), build_report_sha256=sha(build_path),
        source_url='https://www.gnu.org/software/apl/apl.html#Native-Functions',
        interface_template=str(G / '.build/toolchains/apl-2.0/src/native/template.hh'),
        library_documentation='https://www.pcre.org/current/doc/html/pcre2_substitute.html',
        operations=['Literal global regex substitution', 'Optional ordinary PCRE2 JIT compilation'],
        native_benchmark_kernel=False,
        reason='Accepts arbitrary pattern/replacement/subject strings. The adapter invokes standard pcre2_substitute, handles output buffer capacity and converts Unicode strings. It has no benchmark patterns, no count loop, no benchmark sequence, no length-only shortcut, and no I/O.',
        memory_strategy='The large subject is the direct APL right argument. C++ stores Unicode subject/output buffers; it does not allocate whole-length APL integer masks, prefix arrays, destination arrays or a per-match container. Only complete replacement text is returned. Arbitrary expanding replacements retry after PCRE2 reports required capacity.',
        pins={str(source): sha(source), str(binary): sha(binary), str(pcre): sha(pcre)})
    rows = []
    for variant in ('compact', 'optimized'):
        old = next(r for r in original['programs'] if r['benchmark'] == 'regexredux' and r['variant'] == variant)
        row = copy.deepcopy(old)
        path = G / 'sources/apl-v2' / f'regexredux.{variant}.apl'
        small = copy.deepcopy(next(r for r in validation['rows']
                                   if r['variant'] == variant and r['case'] == 'canonical-small'))
        small['reference_match'] = (small['passed'] and small['returncode'] == 0
                                    and small['stdout_sha256'] == small['expected_stdout_sha256'])
        row.update(id=f'regexredux-apl-v2-{variant}', key=f'regexredux:regexredux-apl-v2-{variant}',
            source=str(path), source_sha256=sha(path), loc=loc(path.read_text()),
            normalized_loc=loc(path.read_text()), physical_loc=loc(path.read_text()), loc_expansions=[],
            library_support=[support], supersedes_keys=[old['key']],
            supersedes_source_sha256=old['source_sha256'],
            build_status='interpreted-with-built-general-library',
            small_validation=small,
            adversarial_validation_status='all-new-v2-cases-passed',
            adversarial_validation=dict(report=str(validation_path), sha256=sha(validation_path),
                benchmark_cases=6, general_library_cases=8, jit=variant == 'optimized'),
            source_provenance='New memory-efficient counted APL application, retaining the original canonical orchestration and nine GNU APL regex counts. General PCRE2 substitution binding is provided through the published GNU APL native-library interface.',
            measurement_status='prepared-not-fully-timed', eligible_for_chart=False,
            samples_seconds=[], sample_validation=[], attempts=[])
        row['command'] = [str(G / 'run-apl.sh'), '--cc', '8' if variant == 'optimized' else '1', '-f', str(path)]
        row['runtime']['pins'].update(support['pins'])
        row['adaptations'] = old['adaptations'][:2] + [
            'Avoid packing/disclosing the large subject: the general substitute interface receives it directly as the right argument.',
            'Clear original Input after header/newline removal; retain only its scalar length.',
            'Use actual general PCRE2 global literal substitution in place of per-match concatenation or full-length APL masks/prefix/scatter arrays.',
            'The application still counts all nine case-sensitive regexes over the cleaned subject with GNU APL ⎕RE and performs all five substitutions sequentially on actual strings.',
            'Normal PCRE2 matching.' if variant == 'compact' else 'Ordinary PCRE2 JIT is requested for substitution; matching falls back if JIT is unavailable.']
        row['algorithm'] = dict(strict=True, native_benchmark_kernel=False,
            reason='All nine case-sensitive global regex counts and five sequential complete generic substitutions. Initial, cleaned and final lengths come from actual materialized strings. Application sequencing and all patterns remain counted APL; native support is only general library substitution.')
        row.pop('full_failure', None)
        rows.append(row)
    result = dict(schema_version=1, status='prepared-validated-pending-guarded-full-timing',
        created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(), language='apl',
        native_support=support, programs=rows,
        preserved_v1=dict(manifest=str(G / 'apl-manifest.json'), sha256=sha(G / 'apl-manifest.json'),
            sources=[dict(path=r['source'], sha256=r['source_sha256']) for r in original['programs'] if r['benchmark'] == 'regexredux'],
            failed_evidence=str(G / 'evidence/harness-v1-partial')),
        validation=dict(path=str(validation_path), sha256=sha(validation_path), passed=True),
        capacity_review=dict(runtime_changed=False,
            diagnosis='GNU APL uses /proc/meminfo and falls back to approximately 3 GiB on macOS; WA_scale defaults to 90%. The previous nested subject argument and whole-length array temporaries exceed this logical workspace threshold.',
            code_evidence=['Quad_WA.cc:172–187', 'Value.cc:196–215', 'Workspace.cc:128–141', 'Bif_F12_PARTITION_PICK.cc:44–55,269–318', 'Value.hh:745–769'],
            documented_mem_option='GNU APL --mem requires readable Linux /proc memory/overcommit controls and is unavailable on this macOS runtime.',
            decision='Retain the unchanged runtime and safety guards. Remove application copies and whole-length APL replacement temporaries instead of increasing the memory limit. Full-size acceptance still requires the parent RSS/workspace guards and exact complete output.',
            remaining_general_runtime_gap='A portable macOS available-memory estimator would remove the Linux-only fallback, but is not required or applied by these v2 sources.'))
    out = G / 'apl-regex-v2-manifest.json'
    out.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(manifest=str(out), programs=[dict(key=r['key'], loc=r['loc'], source_sha256=r['source_sha256']) for r in rows],
        support_source_sha256=sha(source), support_binary_sha256=sha(binary)), indent=2))


if __name__ == '__main__':
    main()
