"""Author additive V source text only; never build, execute, hash or score.

Root owns selection, native qualification and sealed full manifests. Original
sources/profiles/failed qualification bytes remain immutable.
"""
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
REPAIRED = {'knucleotide', 'revcomp', 'regexredux'}


def write_new(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('xb') as stream:
        stream.write(data)


def read_all_text(source, benchmark, variant):
    assert source.count('import os\n') == 1 and 'import io\n' not in source
    source = source.replace('import os\n', 'import os\nimport io\n', 1)
    lines = source.splitlines(keepends=True)
    found = [i for i, line in enumerate(lines) if 'os.get_raw_stdin()' in line]
    assert len(found) == 1, ('Unique old stdin binding required', benchmark, variant)
    index = found[0]
    original = lines[index]
    indent = original[:len(original) - len(original.lstrip())]
    binding = original.strip().split(' := ', 1)[0]
    assert binding in ('input', 'data')
    reader = 'io.read_all(reader: os.stdin(), read_to_end_of_stream: true) or { panic(err) }'
    if original.strip().endswith('os.get_raw_stdin().bytestr()'):
        lines[index] = indent + 'input_bytes := ' + reader + '\n' + indent + binding + ' := input_bytes.bytestr()\n'
    else:
        assert original.strip() == binding + ' := os.get_raw_stdin()'
        lines[index] = indent + binding + ' := ' + reader + '\n'
    source = ''.join(lines)
    if benchmark == 'regexredux':
        old = "regex.regex_opt('(>.*\\n)|(\\n)')"
        replacement = "regex.regex_opt('(>[^\\n]*\\n)|(\\n)')"
        assert source.count(old) == 1
        source = source.replace(old, replacement, 1)
    return source


REVCOMP_OLD = r'''    for position < data.len {
        b := data[position]
        if at_line_start && b == `;` {
            for position < data.len && data[position] != `\n` {
                position++
            }
            if position < data.len {
                position++
            }
            at_line_start = true
            continue
        } else if at_line_start && b == `>` {
            record(header, mut sequence, table)
            sequence.clear()
            header.clear()
            for position < data.len && data[position] != `\n` {
                if data[position] != `\r` {
                    header << data[position]
                }
                position++
            }
            header << `\n`
            if position < data.len {
                position++
            }
            at_line_start = true
            continue
        } else if b != `\n` && b != `\r` {
            sequence << b
        }
        at_line_start = b == `\n`
        position++
    }
'''
REVCOMP_NEW = r'''    for position < data.len {
        b := data[position]
        if b == `\r` || b == `\n` {
            position++
            if b == `\r` && position < data.len && data[position] == `\n` {
                position++
            }
            at_line_start = true
            continue
        } else if at_line_start && b == `;` {
            for position < data.len && data[position] != `\r` && data[position] != `\n` {
                position++
            }
            at_line_start = true
            continue
        } else if at_line_start && b == `>` {
            record(header, mut sequence, table)
            sequence.clear()
            header.clear()
            for position < data.len && data[position] != `\r` && data[position] != `\n` {
                header << data[position]
                position++
            }
            header << `\n`
            at_line_start = true
            continue
        }
        sequence << b
        at_line_start = false
        position++
    }
'''


def main():
    original_bytes = (ROOT / 'profiles.json').read_bytes()
    parent = json.loads(original_bytes)
    profiles = [r for r in parent['profiles'] if r['variant'] in ('compact', 'optimized')]
    assert len(profiles) == 20 and len({r['id'] for r in profiles}) == 20
    assert {(r['benchmark'], r['variant']) for r in profiles} == {
        (benchmark, variant) for benchmark in (
            'nbody', 'mandelbrot', 'spectralnorm', 'binarytrees', 'fannkuchredux',
            'fasta', 'knucleotide', 'revcomp', 'regexredux', 'pidigits')
        for variant in ('compact', 'optimized')}
    output = []
    source_changes = []
    for original in profiles:
        row = copy.deepcopy(original)
        parent_path = ROOT / original['source']
        before = parent_path.read_bytes()
        repaired = original['benchmark'] in REPAIRED
        changes = []
        if repaired:
            text = read_all_text(before.decode(), original['benchmark'], original['variant'])
            changes.append('Read all stdin bytes with standard io.read_all until EOF; keep existing String/byte representation.')
            if original['benchmark'] == 'regexredux':
                changes.append('Header cleanup explicitly excludes newline instead of relying on dot behavior: (>[^\\n]*\\n)|(\\n).')
            if original['benchmark'] == 'revcomp' and original['variant'] == 'optimized':
                assert text.count(REVCOMP_OLD) == 1, 'Exact old byte-parser block required'
                text = text.replace(REVCOMP_OLD, REVCOMP_NEW, 1)
                changes.append('Byte parser treats CR, LF and CRLF as record/comment boundaries; complement/reversal/wrapping are unchanged.')
            after = text.encode()
            assert after != before and 'os.get_raw_stdin()' not in text
            assert text.count('io.read_all(reader: os.stdin(), read_to_end_of_stream: true)') == 1
        else:
            after = before
            changes.append('Exact unchanged parent source bytes; no source adaptation.')
        destination = HERE / 'sources' / original['benchmark'] / (original['variant'] + '.v')
        write_new(destination, after)
        row.update(id=original['id'] + '-input-v2', historical_id=original['id'],
            source=str(destination.relative_to(ROOT)), source_sha256=None, loc=None,
            status='authored-unbuilt-unqualified-input-v2',
            loc_status='pending canonical normalization under common mutex',
            source_revision='input-v2', parent_source=original['source'],
            parent_source_policy='Original frozen source remains unchanged; seal both exact hashes under common mutex.',
            parent_bytes_unchanged=not repaired, source_changes=changes,
            chart_eligible=False, eligible_for_chart=False, report_only=True, never_chart_eligible=True,
            small_validation=None, required_work_review={'status':'pending native output and work qualification'})
        row['provenance'] = dict(row['provenance'],
            input_revision_parent_id=original['id'], input_revision_parent_source=original['source'])
        output.append(row)
        source_changes.append(dict(id=row['id'], historical_id=original['id'], benchmark=row['benchmark'],
            variant=row['variant'], parent_source=original['source'], source=row['source'],
            source_bytes_changed=repaired, changes=changes))
        assert parent_path.read_bytes() == before, 'Author must not mutate any original source'
    revision = dict(status='additive-input-v2-drafts-unbuilt-unqualified', report_only=True,
        chart_admission=False, full_performance_runs=0, source_revision='input-v2',
        v_release=parent['v_release'], v_source_ref=parent['v_source_ref'],
        canonical_normalization=parent['canonical_normalization'],
        source_path_basis='All source paths are relative to v-report-only/20261012, not this revision directory.',
        parent_profiles='profiles.json', parent_profile_count=len(parent['profiles']),
        selected_profile_count=20, changed_sources=6, exact_parent_copies=14, profiles=output,
        required_pending_gates=['Versioned root-owned selector/preparation', 'Complete canonical and adversarial outputs',
            'Actual all-seven-map and both-pi-division work traces', 'Remaining required work review',
            'Canonical normalized LOC', 'Independent full manifest review and actual full samples'])
    write_new(HERE / 'profiles.json', (json.dumps(revision, indent=2) + '\n').encode())
    write_new(HERE / 'history/parent-profiles.json', original_bytes)
    history = dict(status='historical-failure-preserved-not-reused',
        root_reported_previous_cohort_pid=61707, root_reported_previous_cohort_status='Stopped',
        failed_source_id='v-regexredux-compact', actual_failed_probe='canonical regexredux small input',
        observed_input_length=22, failure_cause='POSIX os.get_raw_stdin uses one C getline, not an entire-input read.',
        original_source_profiles=str((ROOT / 'profiles.json').relative_to(ROOT)),
        preserved_original_profile_inventory='history/parent-profiles.json',
        original_failed_qualification='qualification/production-sdk-v1',
        original_failed_artifacts_mutated=False, original_sources_mutated=False,
        root_reported_prior_full_v_timings=0, old_ids_reused=False, changes=source_changes,
        hash_status='Root seals exact parent/new sources and original failed evidence under the common mutex; no author-side hash sweep.')
    write_new(HERE / 'history.json', (json.dumps(history, indent=2) + '\n').encode())
    assert (ROOT / 'profiles.json').read_bytes() == original_bytes
    print('Authored text only:20 distinct input-v2 profiles,6 standard full-stdin repairs,14 exact parent copies; no compiler/probe/test/hash/score.')


if __name__ == '__main__':
    main()
